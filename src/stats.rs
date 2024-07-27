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
pub struct Stats {
    card_stats: HashMap<CardId, CardStats>,
}

const DEFAULT_HOME_STATS_PATH: &str = ".config/flashr/stats.json";

impl Stats {
    pub fn new() -> Self {
        Self {
            card_stats: HashMap::new(),
        }
    }

    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, StatsError> {
        let path = path.into();

        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_file() {
                let json = std::fs::read_to_string(&path)
                    .map_err(|err| StatsError::IoError(path.clone(), err))?;
                let stats =
                    serde_json::from_str(&json).map_err(|err| StatsError::SerdeError(path, err))?;

                Ok(stats)
            } else {
                Err(StatsError::ConfigIsDir(path))
            }
        } else {
            Ok(Self::new())
        }
    }

    pub fn load_from_user_home() -> Result<Self, StatsError> {
        let path = get_home_folder()?;
        Self::load_from_file(path)
    }

    pub fn save_to_file(&self, path: impl Into<PathBuf>) -> Result<(), StatsError> {
        let path: PathBuf = path.into();

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| StatsError::IoError(path.clone(), err))?;
            }
        }

        std::fs::write(
            &path,
            serde_json::to_string(&self)
                .map_err(|err| StatsError::SerdeError(path.clone(), err))?,
        )
        .map_err(|err| StatsError::IoError(path.clone(), err))?;

        Ok(())
    }

    pub fn save_to_user_home(&self) -> Result<(), StatsError> {
        let path = get_home_folder()?;
        self.save_to_file(path)
    }

    pub fn for_card(&mut self, id: impl Into<CardId>) -> &CardStats {
        let id = id.into();
        if self.card_stats.contains_key(&id) {
            self.card_stats.get(&id)
        } else {
            let stats = Default::default();
            self.card_stats.insert(id.clone(), stats);
            self.card_stats.get(&id)
        }
        .expect("Unable to find stats for card")
    }

    pub fn for_card_mut(&mut self, id: impl Into<CardId>) -> &mut CardStats {
        let id = id.into();
        if self.card_stats.contains_key(&id) {
            self.card_stats.get_mut(&id)
        } else {
            let stats = Default::default();
            self.card_stats.insert(id.clone(), stats);
            self.card_stats.get_mut(&id)
        }
        .expect("Unable to find stats for card")
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

fn get_home_folder() -> Result<PathBuf, StatsError> {
    let path = dirs::home_dir();
    if let Some(mut path) = path {
        path.push(DEFAULT_HOME_STATS_PATH);
        Ok(path)
    } else {
        Err(StatsError::NoHomeDirError())
    }
}

#[derive(Serialize, Deserialize)]
pub struct CardStats {
    pub correct: usize,
    pub incorrect: usize,
}

impl CardStats {
    fn new() -> Self {
        Self {
            correct: 0,
            incorrect: 0,
        }
    }

    pub fn weight(&self) -> f64 {
        1.0 / (self.correct + 1) as f64
    }
}

impl Default for CardStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::deck::{Card, Deck};

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
        let deck_card = (&deck, &card);

        {
            let mut stats = Stats::default();
            let card_stats = stats.for_card_mut(&deck_card);
            card_stats.correct += 1;
            assert!(stats.save_to_file(TEST_STATS_FILE_PATH).is_ok());
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
        let deck_card = (&deck, &card);

        {
            let mut stats = Stats::default();
            let card_stats = stats.for_card_mut(&deck_card);
            card_stats.correct += 1;
            assert!(stats.save_to_file(TEST_STATS_FILE_PATH_NESTED).is_ok());
        }

        {
            let mut stats = Stats::load_from_file(TEST_STATS_FILE_PATH_NESTED)
                .expect("Unable to load deck from nesed test stats file");
            assert!(stats.for_card(&deck_card).correct == 1);
        }
    }
}
