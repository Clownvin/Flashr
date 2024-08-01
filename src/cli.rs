use clap::Parser;

use crate::Mode;

#[derive(Parser, Debug)]
#[command(name = "flashr", version = env!("CARGO_PKG_VERSION"))]
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
    #[arg(long = "line", help = "Toggle the weight line", long_help = LINE_HELP, default_value_t = false)]
    pub line: bool,
    #[arg(short = 'm', long = "mode", default_value_t = Mode::Match, value_name = "MODE", help = "Program mode", long_help = MODE_HELP)]
    pub mode: Mode,
    #[arg(help = "Deck JSON file/dir paths", long_help = PATHS_HELP)]
    pub paths: Vec<String>,
}

const COUNT_HELP: &str = r#"Number of problems to show. If omitted, will continue indefinitely."#;
const FACES_HELP: &str = r#"Faces to show problems for.
Example Usage: flashr -f Front -f Back ./decks"#;
const LINE_HELP: &str = r#"Toggle the weight line. This will render a bar chart at the top which represents the weights of the backing weighted list."#;
const MODE_HELP: &str = r#"Program mode. Possible values:
    match   - Multiple choice matching problems
    flash   - Typical flashcards
    type    - Shown a face, and asked to type the answer"#;
const PATHS_HELP: &str = r#"Paths to load decks from. Can be individual files or directories."#;

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::cli;

    #[test]
    fn verify_cli() {
        cli::FlashrCli::command().debug_assert();
    }
}
