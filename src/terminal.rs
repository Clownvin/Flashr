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

use std::sync::Mutex;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{StatefulWidget, Widget},
    Frame, Terminal,
};

use crate::{FlashrError, UiError};

pub struct TerminalWrapper {
    #[allow(unused)]
    mouse_capture: MouseCapture,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl TerminalWrapper {
    pub fn new() -> Result<TerminalWrapper, std::io::Error> {
        let raw_mode = RawMode::enable()?;
        let alt_screen = AltScreen::enter(raw_mode)?;
        let mouse_capture = MouseCapture::enable(alt_screen)?;
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        Ok(TerminalWrapper {
            mouse_capture,
            terminal,
        })
    }

    pub fn draw(&mut self, draw_fn: impl FnOnce(&mut Frame)) -> Result<(), FlashrError> {
        self.terminal.draw(draw_fn).map_err(UiError::IoError)?;
        Ok(())
    }

    #[allow(unused)]
    pub fn render_widget(&mut self, widget: impl Widget) -> Result<(), FlashrError> {
        self.draw(|frame| frame.render_widget(widget, frame.area()))
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        state: &mut W::State,
    ) -> Result<(), FlashrError> {
        self.draw(|frame| frame.render_stateful_widget(widget, frame.area(), state))
    }
}

static LOCKED: Mutex<bool> = Mutex::new(false);

struct Lock;

impl Lock {
    fn acquire() -> Lock {
        let mut locked = LOCKED.lock().expect("Unable to acquire lock mutex");
        assert!(!*locked, "Terminal is already being used, cannot lock");
        *locked = true;
        Lock
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let mut locked = LOCKED.lock().expect("Unable to acquire lock mutex");
        assert!(*locked, "Terminal is not being used, cannot unlock");
        *locked = false;
    }
}

#[repr(transparent)]
struct RawMode(Lock);

impl RawMode {
    fn enable() -> Result<RawMode, std::io::Error> {
        let lock = Lock::acquire();
        enable_raw_mode()?;
        Ok(RawMode(lock))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[repr(transparent)]
struct AltScreen(RawMode);

impl AltScreen {
    fn enter(raw_mode: RawMode) -> Result<Self, std::io::Error> {
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        Ok(Self(raw_mode))
    }
}

impl Drop for AltScreen {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

#[repr(transparent)]
struct MouseCapture(AltScreen);

impl MouseCapture {
    fn enable(alt_screen: AltScreen) -> Result<Self, std::io::Error> {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        Ok(Self(alt_screen))
    }
}

impl Drop for MouseCapture {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
    }
}
