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

use std::{fmt::Display, path::PathBuf};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::deck::CardId;

#[derive(Debug)]
pub enum StatsError {
    NoHomeDirError(),
    ConfigIsDir(PathBuf),
    IoError(PathBuf, std::io::Error),
    SerdeError(PathBuf, serde_json::Error),
}

impl Display for StatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoHomeDirError() => f.write_str("Unable to find user home directory"),
            Self::ConfigIsDir(path) => f.write_fmt(format_args!(
                "Config file is directory: {}",
                path.to_str().unwrap_or("unknown")
            )),
            Self::IoError(path, err) => f.write_fmt(format_args!(
                "IoError: {err}, path: {}",
                path.to_str().unwrap_or("unknown")
            )),
            Self::SerdeError(path, err) => f.write_fmt(format_args!(
                "SerdeError: {err}, path: {}",
                path.to_str().unwrap_or("unknown")
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StatsJson {
    card_stats: HashMap<CardId, CardStats>,
}

impl From<Stats> for StatsJson {
    fn from(value: Stats) -> Self {
        StatsJson {
            card_stats: value.card_stats,
        }
    }
}

pub struct Stats {
    path: PathBuf,
    card_stats: HashMap<CardId, CardStats>,
}

const DEFAULT_HOME_STATS_PATH: &str = ".config/flashr/stats.json";

impl Stats {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            card_stats: HashMap::new(),
        }
    }

    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, StatsError> {
        let path: PathBuf = path.into();
        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_file() {
                let json = std::fs::read_to_string(&path)
                    .map_err(|err| StatsError::IoError(path.clone(), err))?;

                serde_json::from_str(&json)
                    .map(|StatsJson { card_stats }| Self {
                        path: path.clone(),
                        card_stats,
                    })
                    .map_err(|err| StatsError::SerdeError(path, err))
            } else {
                Err(StatsError::ConfigIsDir(path))
            }
        } else {
            Ok(Self::new(path))
        }
    }

    pub fn load_from_user_home() -> Result<Self, StatsError> {
        let path = get_home_config_file()?;
        Self::load_from_file(path)
    }

    pub fn save_to_file(self) -> Result<(), StatsError> {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| StatsError::IoError(self.path.clone(), err))?;
            }
        }

        let path = self.path.clone();
        let json: StatsJson = self.into();

        std::fs::write(
            &path,
            serde_json::to_string(&json)
                .map_err(|err| StatsError::SerdeError(path.clone(), err))?,
        )
        .map_err(|err| StatsError::IoError(path.clone(), err))?;

        Ok(())
    }

    pub fn for_card(&mut self, id: impl Into<CardId>) -> &CardStats {
        self.for_card_mut(id)
    }

    pub fn for_card_mut(&mut self, id: impl Into<CardId>) -> &mut CardStats {
        let id = id.into();
        //SAFETY: This is safe because either it exists or we add it here
        unsafe {
            if self.card_stats.contains_key(&id) {
                self.card_stats.get_mut(&id)
            } else {
                let stats = Default::default();
                self.card_stats.insert(id.clone(), stats);
                self.card_stats.get_mut(&id)
            }
            .unwrap_unchecked()
        }
    }
}

fn get_home_config_file() -> Result<PathBuf, StatsError> {
    let path = dirs::home_dir();
    if let Some(mut path) = path {
        path.push(DEFAULT_HOME_STATS_PATH);
        Ok(path)
    } else {
        Err(StatsError::NoHomeDirError())
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct CardStats {
    pub correct: usize,
    pub incorrect: usize,
}

impl CardStats {
    pub fn weight(&self) -> f64 {
        (1.0 / (self.correct.saturating_sub(self.incorrect) + 1) as f64)
            + self.incorrect.saturating_sub(self.correct) as f64
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        deck::{Card, Deck},
        DeckCard,
    };

    use super::Stats;

    const TEST_STATS_FILE_PATH: &str = "./tests/stats.json";

    #[test]
    fn save_load_file() {
        let _ = std::fs::remove_file(TEST_STATS_FILE_PATH);

        let deck = Deck {
            name: "test".to_owned(),
            faces: vec![],
            cards: vec![],
        };
        let card = Card::new(vec![Some("Front"), Some("Back")]);
        let deck_card = DeckCard::new(&deck, &card);

        {
            let mut stats = Stats::new(TEST_STATS_FILE_PATH);
            let card_stats = stats.for_card_mut(&deck_card);
            card_stats.correct += 1;
            assert!(stats.save_to_file().is_ok());
        }

        {
            let mut stats = Stats::load_from_file(TEST_STATS_FILE_PATH)
                .expect("Unable to load from test stats file");
            assert!(stats.for_card(&deck_card).correct == 1);
        }
    }

    const TEST_STATS_FOLDER: &str = "./tests/stats/";
    const TEST_STATS_FILE_PATH_NESTED: &str = "./tests/stats/stats.json";

    #[test]
    fn save_load_file_nested() {
        let _ = std::fs::remove_dir_all(TEST_STATS_FOLDER);

        let deck = Deck {
            name: "test".to_owned(),
            faces: vec![],
            cards: vec![],
        };
        let card = Card::new(vec![Some("Front"), Some("Back")]);
        let deck_card = DeckCard::new(&deck, &card);

        {
            let mut stats = Stats::new(TEST_STATS_FILE_PATH_NESTED);
            let card_stats = stats.for_card_mut(&deck_card);
            card_stats.correct += 1;
            assert!(stats.save_to_file().is_ok());
        }

        {
            let mut stats = Stats::load_from_file(TEST_STATS_FILE_PATH_NESTED)
                .expect("Unable to load deck from nesed test stats file");
            assert!(stats.for_card(&deck_card).correct == 1);
        }
    }
}
