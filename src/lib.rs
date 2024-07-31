use clap::Parser;
use stats::{Stats, StatsError};
use std::{
    fmt::Display,
    ops::{Deref, Not},
    str::FromStr,
};

use deck::{load_decks, Card, CardId, Deck, DeckError, Face};
use modes::{flashcards::show_flashcards, match_faces::match_faces};
use terminal::TerminalWrapper;

mod cli;
mod color;
pub mod deck;
mod event;
mod modes;
mod random;
mod stats;
mod terminal;
mod weighted_list;

pub fn run() -> Result<CorrectIncorrect, FlashrError> {
    let cli = cli::FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let stats = Stats::load_from_user_home()?;
    let args = ModeArguments::new(&decks, stats, cli.problem_count, cli.faces, cli.line);

    std::panic::catch_unwind(|| -> Result<CorrectIncorrect, FlashrError> {
        //NOTE: From this point, stdout/stderr will not be usable, hence we
        //need to catch any panics, since they are not loggable. Mapping to
        //FlashrError allows us to gracefully exit and log the panic.
        let term = &mut TerminalWrapper::new().map_err(UiError::IoError)?;

        let (correct_incorrect, stats) = match cli.mode {
            Mode::Match => match_faces(term, args),
            Mode::Flash => show_flashcards(term, args.deck_cards).map(|_| (None, args.stats)),
            Mode::Type => todo!("Type mode not yet implemented"),
        }?;

        stats.save_to_user_home()?;

        Ok(correct_incorrect)
    })
    .map_err(|err| {
        FlashrError::Panic({
            // Attempt to extract the panic message
            let message = if let Some(msg) = err.downcast_ref::<String>() {
                msg.clone()
            } else if let Some(msg) = err.downcast_ref::<&str>() {
                (*msg).to_owned()
            } else {
                "Unknown panic occurred".to_owned()
            };

            // Get the location of the panic
            let location = std::panic::Location::caller();
            let file_name = location.file();
            let line_number = location.line();

            // Create the formatted string
            format!("{}:{}: {}", file_name, line_number, message)
        })
    })?
}

type Faces = Option<Vec<String>>;
type ProblemCount = Option<usize>;
type CorrectIncorrect = Option<(usize, usize)>;
type ModeResult = (CorrectIncorrect, Stats);

#[derive(Clone, Copy)]
struct DeckCard<'a> {
    deck: &'a Deck,
    card: &'a Card,
}

impl<'a> DeckCard<'a> {
    fn new(deck: &'a Deck, card: &'a Card) -> Self {
        Self { deck, card }
    }

    fn possible_faces(&self) -> Vec<(usize, &String, &Face)> {
        let mut possible_faces = Vec::with_capacity(self.deck.faces.len());
        for (index, face_str) in self.deck.faces.iter().enumerate() {
            if let Some(face) = self.card[index].as_ref() {
                possible_faces.push((index, face_str, face));
            }
        }
        possible_faces
    }
}

impl<'a> Deref for DeckCard<'a> {
    type Target = Card;

    fn deref(&self) -> &'a Self::Target {
        self.card
    }
}

struct PromptCard<'a> {
    prompt: String,
    deck_card: DeckCard<'a>,
    index: usize,
}

impl<'a> From<&PromptCard<'a>> for CardId {
    fn from(card: &PromptCard<'a>) -> Self {
        (&card.deck_card).into()
    }
}

#[derive(Clone, Debug)]
enum Mode {
    Match,
    Type,
    Flash,
}

impl FromStr for Mode {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if s == "match" {
            Ok(Self::Match)
        } else if s == "flash" {
            Ok(Self::Flash)
        } else if s == "type" {
            Ok(Self::Type)
        } else {
            Err(format!("Mode argument not recognized: {s}"))
        }
    }

    type Err = String;
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Match => "match",
            Mode::Type => "type",
            Mode::Flash => "flash",
        })
    }
}

struct ModeArguments<'a> {
    problem_count: ProblemCount,
    faces: Faces,
    deck_cards: Vec<DeckCard<'a>>,
    stats: Stats,
    line: bool,
}

impl<'a> ModeArguments<'a> {
    fn new(
        decks: &'a [Deck],
        stats: Stats,
        problem_count: ProblemCount,
        faces: Faces,
        line: bool,
    ) -> Self {
        let mut deck_cards = {
            let max_num_problems = decks.iter().fold(0, |total, deck| {
                total + (deck.cards.len() * deck.faces.len())
            });
            Vec::with_capacity(max_num_problems)
        };

        if let Some(faces) = faces.as_ref() {
            for deck in decks {
                let deck_faces = {
                    let mut buf = Vec::with_capacity(deck.faces.len());
                    deck.faces
                        .iter()
                        .enumerate()
                        .filter(|(_, deck_face)| faces.iter().any(|face| face == *deck_face))
                        .for_each(|(i, _)| buf.push(i));
                    buf
                };

                deck_faces.is_empty().not().then(|| {
                    for card in deck.cards.iter() {
                        if deck_faces.iter().any(|i| card[*i].is_some()) {
                            deck_cards.push(DeckCard::new(deck, card));
                        }
                    }
                });
            }
        } else {
            for deck in decks {
                for card in deck.cards.iter() {
                    deck_cards.push(DeckCard::new(deck, card));
                }
            }
        }
        Self {
            problem_count,
            faces,
            deck_cards,
            stats,
            line,
        }
    }
}

struct OptionTuple<T>(Option<(T, T)>);

impl<T> Deref for OptionTuple<T> {
    type Target = Option<(T, T)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromIterator<T> for OptionTuple<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        Self(
            iter.next()
                .and_then(|first| iter.next().map(|second| (first, second))),
        )
    }
}

trait AndThen {
    fn and_then<T>(&self, f: impl FnOnce() -> Option<T>) -> Option<T>;
}

impl AndThen for bool {
    fn and_then<T>(&self, f: impl FnOnce() -> Option<T>) -> Option<T> {
        if *self {
            f()
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum FlashrError {
    Deck(Box<DeckError>),
    Ui(UiError),
    DeckMismatch(String),
    Arg(ArgError),
    Stats(StatsError),
    Panic(String),
}

impl Display for FlashrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            //TODO: Refactor this type
            Self::DeckMismatch(reason) => f.write_fmt(format_args!("DeckMismatch: {reason}")),
            Self::Deck(err) => f.write_fmt(format_args!("Deck: {err}")),
            Self::Ui(err) => f.write_fmt(format_args!("Ui: {err}")),
            Self::Arg(err) => f.write_fmt(format_args!("Arg: {err}")),
            Self::Stats(err) => f.write_fmt(format_args!("Stats: {err}")),
            Self::Panic(err) => f.write_fmt(format_args!("Panicked: {err}")),
        }
    }
}

impl From<DeckError> for FlashrError {
    fn from(err: DeckError) -> Self {
        Self::Deck(Box::new(err))
    }
}

impl From<UiError> for FlashrError {
    fn from(err: UiError) -> Self {
        Self::Ui(err)
    }
}

impl From<ArgError> for FlashrError {
    fn from(err: ArgError) -> Self {
        Self::Arg(err)
    }
}

impl From<StatsError> for FlashrError {
    fn from(err: StatsError) -> Self {
        Self::Stats(err)
    }
}

#[derive(Debug)]
pub enum UiError {
    IoError(std::io::Error),
}

impl Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => f.write_fmt(format_args!("IoError: {err}")),
        }
    }
}

impl From<std::io::Error> for UiError {
    fn from(err: std::io::Error) -> Self {
        UiError::IoError(err)
    }
}

#[derive(Debug)]
pub enum ArgError {
    DeckNotEnoughFaces(Vec<String>, String),
}

impl Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeckNotEnoughFaces(faces, deck) => {
                let faces = faces.join(", ");
                f.write_fmt(format_args!("Deck \"{deck}\" does not have enough faces for arguments:\nNeeds at least one of: {faces}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::cli;

    #[test]
    fn verify_cli() {
        cli::FlashrCli::command().debug_assert();
    }
}
