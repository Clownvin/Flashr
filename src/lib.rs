#![feature(iter_intersperse)]
use clap::Parser;
use stats::{Stats, StatsError};
use std::{fmt::Display, ops::Deref, str::FromStr};

use deck::{load_decks, Card, Deck, DeckError};
use modes::{match_faces::match_faces, type_faces::type_faces};
use terminal::TerminalWrapper;

mod cli;
pub mod deck;
mod event;
mod modes;
mod random;
mod stats;
mod terminal;

pub fn run() -> Result<ModeResult, FlashrError> {
    let cli = cli::FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let stats = Stats::load_from_user_home()?;
    let term = TerminalWrapper::new().map_err(UiError::IoError)?;
    let args = ModeArguments::new(&decks, stats, cli.problem_count, cli.faces);
    args.validate()?;

    match cli.mode {
        Mode::Match => match_faces(term, args),
        Mode::Type => type_faces(term, args),
    }
}

type Faces = Option<Vec<String>>;
type ProblemCount = Option<usize>;
type CorrectIncorrect = (usize, usize);
type DeckCard<'a> = (&'a Deck, &'a Card);
type ModeResult = (CorrectIncorrect, Stats);
type FaceCardIndex<'a> = (String, &'a Card, usize);

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
    deck_cards: Vec<(&'a Deck, &'a Card)>,
    stats: Stats,
}

impl<'a> ModeArguments<'a> {
    fn new(decks: &'a [Deck], stats: Stats, problem_count: ProblemCount, faces: Faces) -> Self {
        let mut deck_cards = Vec::with_capacity(decks.iter().fold(0, |total, deck| {
            total + (deck.cards.len() * deck.faces.len())
        }));

        if let Some(faces) = faces.as_ref() {
            for deck in decks.iter() {
                let mut deck_faces = Vec::with_capacity(deck.faces.len());
                deck.faces
                    .iter()
                    .enumerate()
                    .filter(|(_, deck_face)| faces.iter().any(|face| face == *deck_face))
                    .for_each(|(i, _)| deck_faces.push(i));

                if deck_faces.is_empty() {
                    continue;
                } else {
                    for card in deck.cards.iter() {
                        if deck_faces.iter().any(|i| card[*i].is_some()) {
                            deck_cards.push((deck, card));
                        } else {
                            // Don't push, no matching faces
                        }
                    }
                }
            }
        } else {
            for deck in decks.iter() {
                for card in deck.cards.iter() {
                    deck_cards.push((deck, card));
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

#[derive(PartialEq, Eq)]
enum ProblemResult {
    Correct,
    Incorrect,
    Quit,
}

impl ProblemResult {
    fn is_correct(&self) -> bool {
        *self == ProblemResult::Correct
    }

    fn is_quit(&self) -> bool {
        *self == ProblemResult::Quit
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
