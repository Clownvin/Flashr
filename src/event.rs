/*
 * Copyright (C) 2024 Clownvin <123clownvin@gmail.com>
 *
 * This file is part of Flashr.
 *
 * Flashr is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Flashr is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Flashr.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::time::Duration;

use crossterm::event::{self, Event};

use crate::{FlashrError, UiError};

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
