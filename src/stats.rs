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

const DEFAULT_DIRECTORY: &str = ".config/flashr/stats.json";

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
        let path = dirs::home_dir();
        if let Some(mut path) = path {
            path.push(DEFAULT_DIRECTORY);
            Self::load_from_file(path)
        } else {
            Err(StatsError::NoHomeDirError())
        }
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
        .unwrap()
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
        .unwrap()
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
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
