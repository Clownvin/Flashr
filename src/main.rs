/*
 * Copyright (C) 2025 Clownvin <123clownvin@gmail.com>
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

use std::process::exit;

fn main() {
    let result = flashr::run();
    match result {
        Ok(progress) => {
            if let Some(progress) = progress {
                println!("{progress}");
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            exit(1);
        }
    }
}
