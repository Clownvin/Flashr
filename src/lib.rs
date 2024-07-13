#![feature(iter_intersperse)]
use std::fmt::Display;

use clap::Parser;
use deck::{load_decks, DeckError};
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
    #[arg(short = 'c', long = "count", value_name = "PROBLEM_COUNT")]
    problem_count: Option<usize>,
    paths: Vec<String>,
}

#[derive(Debug)]
pub enum FlashrError {
    DeckError(Box<DeckError>),
    UiError(UiError),
    DeckMismatchError(String),
}

impl Display for FlashrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlashrError::DeckMismatchError(reason) => {
                f.write_fmt(format_args!("DeckMismatch: {reason}"))
            }
            FlashrError::DeckError(err) => f.write_fmt(format_args!("DeckError: {err}")),
            FlashrError::UiError(err) => f.write_fmt(format_args!("UiError: {err}")),
        }
    }
}

impl From<DeckError> for FlashrError {
    fn from(err: DeckError) -> Self {
        FlashrError::DeckError(Box::new(err))
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

pub enum ProblemResult {
    Correct,
    Incorrect,
    Quit,
}

pub fn run() -> Result<(usize, usize), FlashrError> {
    let FlashrCli {
        paths,
        problem_count,
    } = FlashrCli::parse();
    let decks = load_decks(paths)?;
    let mut term = initialize_terminal()?;
    match_cards(&mut term, decks, problem_count)
}

fn initialize_terminal() -> Result<TerminalWrapper, FlashrError> {
    Ok(TerminalWrapper::new().map_err(UiError::IoError)?)
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
