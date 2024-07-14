#![feature(iter_intersperse)]
use std::fmt::Display;

use clap::Parser;
use deck::{load_decks, Deck, DeckError};
use modes::match_faces::match_cards;
use terminal::TerminalWrapper;

pub mod deck;
mod event;
mod modes;
mod random;
mod terminal;

type Decks = Vec<Deck>;
type Faces = Option<Vec<String>>;
type ProblemCount = Option<usize>;

#[derive(Parser, Debug)]
#[command(name = "flashr")]
struct FlashrCli {
    #[arg(short = 'c', long = "count", value_name = "PROBLEM_COUNT")]
    problem_count: Option<usize>,
    #[arg(short = 'f', long = "faces", value_name = "[FACE_1, [...FACE_N]]")]
    faces: Option<Vec<String>>,
    paths: Vec<String>,
}

#[derive(Debug)]
pub enum FlashrError {
    Deck(Box<DeckError>),
    Ui(UiError),
    DeckMismatch(String),
    Arg(ArgError),
}

impl Display for FlashrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            //TODO: Refactor this type
            Self::DeckMismatch(reason) => f.write_fmt(format_args!("DeckMismatch: {reason}")),
            Self::Deck(err) => f.write_fmt(format_args!("Deck: {err}")),
            Self::Ui(err) => f.write_fmt(format_args!("Ui: {err}")),
            Self::Arg(err) => f.write_fmt(format_args!("Arg: {err}")),
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

struct ModeArguments {
    decks: Decks,
    problem_count: ProblemCount,
    faces: Faces,
}

impl ModeArguments {
    fn new(decks: Decks, problem_count: ProblemCount, faces: Faces) -> Self {
        Self {
            decks,
            problem_count,
            faces,
        }
    }

    //TODO add and test logic to make sure that each face asked for appears in some deck
    //TODO add and test logic to make sure that each face has at least one problem?
    fn validate(&self) -> Result<(), ArgError> {
        if let Some(faces) = self.faces.as_ref() {
            if let Some(deck) = self.decks.iter().find(|deck| {
                deck.faces
                    .iter()
                    .filter(|deck_face| faces.iter().any(|face| face == *deck_face))
                    .count()
                    == 0
            }) {
                return Err(ArgError::DeckNotEnoughFaces(
                    faces.clone(),
                    deck.name.clone(),
                ));
            }
        }

        Ok(())
    }
}

enum ProblemResult {
    Correct,
    Incorrect,
    Quit,
}

pub fn run() -> Result<(usize, usize), FlashrError> {
    let cli = FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let mut term = TerminalWrapper::new().map_err(UiError::IoError)?;
    let args = ModeArguments::new(decks, cli.problem_count, cli.faces);
    args.validate()?;
    match_cards(&mut term, args)
}

#[cfg(test)]
mod tests {
    use crate::{deck::load_decks, ArgError, FlashrCli, ModeArguments};

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        FlashrCli::command().debug_assert();
    }

    #[test]
    fn verify_enough_faces() {
        let decks = load_decks(vec!["./tests/example.json", "./tests/deck1.json"]).unwrap();
        let args = ModeArguments::new(decks, None, None);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn verify_enough_faces_specified() {
        let decks = load_decks(vec!["./tests/example.json", "./tests/deck_subfaces.json"]).unwrap();
        let args = ModeArguments::new(decks, None, Some(vec!["Front".to_owned()]));
        assert!(args.validate().is_ok());
    }

    #[test]
    fn verify_not_enough_faces_specified() {
        let decks = load_decks(vec!["./tests/example.json", "./tests/deck1.json"]).unwrap();
        let args = ModeArguments::new(decks, None, Some(vec!["Front".to_owned()]));
        assert!(args
            .validate()
            .is_err_and(|err| matches!(err, ArgError::DeckNotEnoughFaces(_, _))));
    }
}
