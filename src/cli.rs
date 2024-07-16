use clap::Parser;

use crate::Mode;

#[derive(Parser, Debug)]
#[command(name = "flashr")]
pub struct FlashrCli {
    #[arg(short = 'c', long = "count", value_name = "PROBLEM_COUNT", help = "Number of problems to show.", long_help = COUNT_HELP)]
    pub problem_count: Option<usize>,
    #[arg(
        short = 'f',
        long = "faces",
        value_name = "[...FACE_N]",
        help = "Faces to show problems for.",
        long_help = FACES_HELP
    )]
    pub faces: Option<Vec<String>>,
    #[arg(short = 'm', long = "mode", default_value_t = Mode::Match, value_name = "MODE", help = "Program mode", long_help = MODE_HELP)]
    pub mode: Mode,
    #[arg(help = "Deck JSON file/dir paths", long_help = PATHS_HELP)]
    pub paths: Vec<String>,
}

const COUNT_HELP: &str = r#"Number of problems to show. If omitted, will continue indefinitely."#;
const FACES_HELP: &str = r#"Faces to show problems for.
Example Usage: flashr -f Front -f Back ./decks"#;
const MODE_HELP: &str = r#"Possible values:
    match   - Multiple choice matching problems
    type    - Shown a face, and asked to type the answer"#;
const PATHS_HELP: &str = r#"Paths to load decks from. Can be individual files or directories."#;