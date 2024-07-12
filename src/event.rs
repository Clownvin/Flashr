use std::time::Duration;

use crossterm::event::{self, Event};

use crate::{FlashrError, UiError};

pub enum UserInput {
    Answer(usize),
    Resize,
    Quit,
}

pub fn clear_and_match_event<T>(match_fn: impl Fn(Event) -> Option<T>) -> Result<T, FlashrError> {
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

fn match_user_input<T>(match_fn: impl Fn(Event) -> Option<T>) -> Result<T, FlashrError> {
    loop {
        if event::poll(Duration::MAX).map_err(UiError::IoError)? {
            let event = event::read().map_err(UiError::IoError)?;
            if let Some(value) = match_fn(event) {
                return Ok(value);
            }
        }
    }
}
