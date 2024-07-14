#![feature(iter_intersperse)]
use std::{fmt::Display, str::FromStr};

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
    #[arg(short = 'c', long = "count", value_name = "PROBLEM_COUNT", help = "Number of problems to show.", long_help = COUNT_HELP)]
    problem_count: Option<usize>,
    #[arg(
        short = 'f',
        long = "faces",
        value_name = "[...FACE_N]",
        help = "Faces to show problems for.",
        long_help = FACES_HELP
    )]
    faces: Option<Vec<String>>,
    #[arg(short = 'm', long = "mode", default_value_t = Mode::Match, value_name = "MODE", help = "Program mode", long_help = MODE_HELP)]
    mode: Mode,
    #[arg(last = true, help = "Deck JSON file/dir paths", long_help = PATHS_HELP)]
    paths: Vec<String>,
}

const COUNT_HELP: &str = r#"Number of problems to show. If omitted, will continue indefinitely."#;
const FACES_HELP: &str = r#"Faces to show problems for.
Example Usage: flashr -f Front -f Back ./decks"#;
const MODE_HELP: &str = r#"Possible values:
    match   - Multiple choice matching problems
    type    - Shown a face, and asked to type the answer"#;
const PATHS_HELP: &str = r#"Paths to load decks from. Can be individual files or directories."#;

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

    match cli.mode {
        Mode::Match => match_cards(&mut term, args),
        Mode::Type => todo!(),
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
