use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event};
use deck::{load_decks, DeckError};
use modes::match_faces::match_cards;
use terminal::TerminalWrapper;

pub mod deck;
mod modes;
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
    let mut term = initialize_terminal()?;
    let decks = load_decks(cli.paths)?;
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

enum UserInput {
    Answer(usize),
    Resize,
    Quit,
}

fn clear_and_match_event<T>(match_fn: fn(event: Event) -> Option<T>) -> Result<T, FlashrError> {
    clear_event_loop()?;
    match_user_input(match_fn)
}

fn clear_event_loop() -> Result<(), FlashrError> {
    loop {
        if event::poll(Duration::from_millis(0)).map_err(UiError::IoError)? {
            event::read().map_err(UiError::IoError)?;
            continue;
        }
        break Ok(());
    }
}

fn match_user_input<T>(match_fn: fn(event: Event) -> Option<T>) -> Result<T, FlashrError> {
    loop {
        if event::poll(Duration::from_secs(1)).map_err(UiError::IoError)? {
            let event = event::read().map_err(UiError::IoError)?;
            if let Some(value) = match_fn(event) {
                return Ok(value);
            }
        }
    }
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
