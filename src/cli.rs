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
