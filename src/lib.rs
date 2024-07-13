#![feature(iter_intersperse)]
use std::fmt::Display;

use clap::Parser;
use deck::{load_decks, CardError, DeckError};
use modes::match_faces::match_cards;
use terminal::TerminalWrapper;

pub mod deck;
mod event;
mod modes;
mod random;
mod terminal;

#[derive(Parser, Debug)]
#[command(name = "flashr")]
struct FlashrCli {
    paths: Vec<String>,
}

#[derive(Debug)]
pub enum FlashrError {
    DeckError(DeckError),
    UiError(UiError),
    DeckMismatchError(String),
}

impl Display for FlashrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            FlashrError::DeckMismatchError(reason) => format!("DeckMismatch: {reason}"),
            FlashrError::DeckError(err) => match err {
                DeckError::NotEnoughCards(deck) => format!(
                    "NotEnoughCards: Deck \"{}\" does not have enough cards.",
                    deck.name
                ),
                DeckError::NotEnoughFaces(deck) => format!(
                    "NotEnoughFaces: Deck \"{}\" does not have enough faces. Requires two, has {}",
                    deck.name,
                    deck.faces.len()
                ),
                DeckError::DuplicateFace(deck, face) => format!(
                    "DuplicateFace: Deck \"{}\" has at least two \"{}\" faces",
                    deck.name, face
                ),
                DeckError::InvalidCard(deck, card_err) => match card_err {
                    CardError::NotEnoughFaces(card) => {
                        let front = card.front_string();
                        let face_count = card.len();
                        let required = deck.faces.len();
                        format!("InvalidCard: NotEnoughFaces: Card with front \"{front}\" does not have enough faces. Has {face_count}, needs {required}")
                    }
                    CardError::TooManyFaces(card) => {
                        let front = card.front_string();
                        let face_count = card.len();
                        let required = deck.faces.len();
                        format!("InvalidCard: TooManyFaces: Card with front \"{front}\" has too many faces. Has {face_count}, needs {required}")
                    }
                },
                DeckError::IoError(path, err) => {
                    format!(
                        "IoError: {err}, path: {}",
                        path.to_str().unwrap_or("unknown")
                    )
                }
                DeckError::SerdeError(path, err) => {
                    format!(
                        "SerdeError: {err}, path: {}",
                        path.to_str().unwrap_or("unknown")
                    )
                }
            },
            FlashrError::UiError(err) => match err {
                UiError::IoError(err) => format!("UiError: IoError: {err}"),
            },
        };

        f.write_str(&string)
    }
}

impl From<DeckError> for FlashrError {
    fn from(err: DeckError) -> Self {
        FlashrError::DeckError(err)
    }
}

impl From<UiError> for FlashrError {
    fn from(err: UiError) -> Self {
        FlashrError::UiError(err)
    }
}

#[derive(Debug)]
pub enum UiError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for UiError {
    fn from(err: std::io::Error) -> Self {
        UiError::IoError(err)
    }
}

pub fn run() -> Result<(usize, usize), FlashrError> {
    let cli = FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let mut term = initialize_terminal()?;
    match_cards(&mut term, decks)
}

fn initialize_terminal() -> Result<TerminalWrapper, FlashrError> {
    Ok(TerminalWrapper::new().map_err(UiError::IoError)?)
}

pub enum ProblemResult {
    Correct,
    Incorrect,
    Quit,
}

#[cfg(test)]
mod tests {
    use crate::FlashrCli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        FlashrCli::command().debug_assert();
    }
}
