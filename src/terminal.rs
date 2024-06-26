use alt_screen::AltScreen;
use ratatui::{backend::CrosstermBackend, widgets::Widget, Frame, Terminal};
use raw_mode::RawMode;

use crate::{FlashrError, UiError};

pub struct TerminalWrapper {
    _alt_screen: AltScreen,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl TerminalWrapper {
    pub fn new() -> Result<TerminalWrapper, std::io::Error> {
        let raw_mode = RawMode::enable()?;
        let alt_screen = AltScreen::enter(raw_mode)?;
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

        Ok(TerminalWrapper {
            _alt_screen: alt_screen,
            terminal,
        })
    }

    pub fn _draw(&mut self, draw_fn: impl FnOnce(&mut Frame)) -> Result<(), FlashrError> {
        self.terminal.draw(draw_fn).map_err(UiError::IoError)?;
        Ok(())
    }

    pub fn render_widget(&mut self, widget: impl Widget) -> Result<(), FlashrError> {
        self.terminal
            .draw(|frame| frame.render_widget(widget, frame.size()))
            .map_err(UiError::IoError)?;
        Ok(())
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
            disable_raw_mode().unwrap();
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
        pub fn enter(raw_mode: RawMode) -> Result<AltScreen, std::io::Error> {
            std::io::stdout().execute(EnterAlternateScreen)?;
            Ok(AltScreen(raw_mode))
        }
    }

    impl Drop for AltScreen {
        fn drop(&mut self) {
            std::io::stdout().execute(LeaveAlternateScreen).unwrap();
        }
    }
}
