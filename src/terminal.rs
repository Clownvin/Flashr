use alt_screen::AltScreen;
use mouse_capture::MouseCapture;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{StatefulWidget, Widget},
    Frame, Terminal,
};
use raw_mode::RawMode;

use crate::{FlashrError, UiError};

pub struct TerminalWrapper {
    _mouse_capture: MouseCapture,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl TerminalWrapper {
    pub fn new() -> Result<TerminalWrapper, std::io::Error> {
        let raw_mode = RawMode::enable()?;
        let alt_screen = AltScreen::enter(raw_mode)?;
        let mouse_capture = MouseCapture::enable(alt_screen)?;
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        Ok(TerminalWrapper {
            _mouse_capture: mouse_capture,
            terminal,
        })
    }

    pub fn draw(&mut self, draw_fn: impl FnOnce(&mut Frame)) -> Result<(), FlashrError> {
        self.terminal.draw(draw_fn).map_err(UiError::IoError)?;
        Ok(())
    }

    pub fn _render_widget(&mut self, widget: impl Widget) -> Result<(), FlashrError> {
        self.draw(|frame| frame.render_widget(widget, frame.size()))
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        state: &mut W::State,
    ) -> Result<(), FlashrError> {
        self.draw(|frame| frame.render_stateful_widget(widget, frame.size(), state))
    }
}

mod raw_mode {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    use lock::Lock;

    pub struct RawMode(Lock);

    impl RawMode {
        pub fn enable() -> Result<RawMode, std::io::Error> {
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

    mod lock {
        use std::sync::Mutex;

        static LOCKED: Mutex<bool> = Mutex::new(false);

        pub struct Lock(());

        impl Lock {
            pub fn acquire() -> Lock {
                let mut locked = LOCKED.lock().unwrap();
                assert!(!*locked);
                *locked = true;
                Lock(())
            }
        }

        impl Drop for Lock {
            fn drop(&mut self) {
                let mut locked = LOCKED.lock().unwrap();
                assert!(*locked);
                *locked = false;
            }
        }
    }
}

mod alt_screen {
    use crossterm::{
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };

    use super::raw_mode::RawMode;

    pub struct AltScreen(RawMode);

    impl AltScreen {
        pub fn enter(raw_mode: RawMode) -> Result<Self, std::io::Error> {
            std::io::stdout().execute(EnterAlternateScreen)?;
            Ok(Self(raw_mode))
        }
    }

    impl Drop for AltScreen {
        fn drop(&mut self) {
            let _ = std::io::stdout().execute(LeaveAlternateScreen);
        }
    }
}

mod mouse_capture {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        ExecutableCommand,
    };

    use super::alt_screen::AltScreen;

    pub struct MouseCapture(AltScreen);

    impl MouseCapture {
        pub fn enable(alt_screen: AltScreen) -> Result<Self, std::io::Error> {
            std::io::stdout().execute(EnableMouseCapture)?;
            Ok(Self(alt_screen))
        }
    }

    impl Drop for MouseCapture {
        fn drop(&mut self) {
            let _ = std::io::stdout().execute(DisableMouseCapture);
        }
    }
}
