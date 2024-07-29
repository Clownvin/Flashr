#![feature(iter_intersperse)]
use clap::Parser;
use stats::{Stats, StatsError};
use std::{fmt::Display, ops::Deref, str::FromStr};

use deck::{load_decks, Card, CardId, Deck, DeckError};
use modes::match_faces::match_faces;
use terminal::TerminalWrapper;

mod cli;
pub mod deck;
mod event;
mod modes;
mod random;
mod stats;
mod terminal;

pub fn run() -> Result<CorrectIncorrect, FlashrError> {
    let cli = cli::FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let stats = Stats::load_from_user_home()?;
    let term = TerminalWrapper::new().map_err(UiError::IoError)?;
    let args = ModeArguments::new(&decks, stats, cli.problem_count, cli.faces);
    args.validate()?;

    let (correct_incorrect, stats) = match cli.mode {
        Mode::Match => match_faces(term, args),
        Mode::Type => todo!("Type mode not yet implemented"),
    }?;

    stats.save_to_user_home()?;

    Ok(correct_incorrect)
}

type Faces = Option<Vec<String>>;
type ProblemCount = Option<usize>;
type CorrectIncorrect = (usize, usize);
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

    fn possible_faces(&self) -> Vec<(usize, &String)> {
        let mut possible_faces = Vec::with_capacity(self.deck.faces.len());
        self.deck
            .faces
            .iter()
            .enumerate()
            .filter(|(i, _)| self.card[*i].is_some())
            .for_each(|face| possible_faces.push(face));
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
}

impl FromStr for Mode {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if s == "match" {
            Ok(Self::Match)
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
        })
    }
}

struct ModeArguments<'a> {
    problem_count: ProblemCount,
    faces: Faces,
    deck_cards: Vec<DeckCard<'a>>,
    stats: Stats,
}

impl<'a> ModeArguments<'a> {
    fn new(decks: &'a [Deck], stats: Stats, problem_count: ProblemCount, faces: Faces) -> Self {
        let mut deck_cards = Vec::with_capacity(decks.iter().fold(0, |total, deck| {
            total + (deck.cards.len() * deck.faces.len())
        }));

        if let Some(faces) = faces.as_ref() {
            for deck in decks.iter() {
                let deck_faces = {
                    let mut buf = Vec::with_capacity(deck.faces.len());
                    deck.faces
                        .iter()
                        .enumerate()
                        .filter(|(_, deck_face)| faces.iter().any(|face| face == *deck_face))
                        .for_each(|(i, _)| buf.push(i));
                    buf
                };

                if deck_faces.is_empty() {
                    continue;
                } else {
                    for card in deck.cards.iter() {
                        if deck_faces.iter().any(|i| card[*i].is_some()) {
                            deck_cards.push(DeckCard::new(deck, card));
                        } else {
                            // Don't push, no matching faces
                        }
                    }
                }
            }
        } else {
            for deck in decks.iter() {
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
        }
    }

    //TODO add and test logic to make sure that each face asked for appears in some deck
    //TODO add and test logic to make sure that each face has at least one problem?
    fn validate(&self) -> Result<(), ArgError> {
        Ok(())
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
        if let Some(first) = iter.next() {
            if let Some(second) = iter.next() {
                return Self(Some((first, second)));
            }
        }
        Self(None)
    }
}

#[derive(Debug)]
pub enum FlashrError {
    Deck(Box<DeckError>),
    Ui(UiError),
    DeckMismatch(String),
    Arg(ArgError),
    Stats(StatsError),
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
