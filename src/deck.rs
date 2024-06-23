use std::{fmt::Debug, fs, ops::Deref, path::PathBuf};

use serde::{Deserialize, Serialize};

///Example JSON:
///```JSON
///{
///  "name": "Kanji Words",
///  "faces": ["Kanji", "Hiragana", "Definition"],
///  "cards": [
///    [
///      "日本",
///      "にほん",
///      "Japan"
///    ]
///  ]
///}
///```
#[derive(Serialize, Deserialize)]
pub struct Deck {
    pub name: String,
    pub faces: Vec<String>,
    pub cards: Vec<Card>,
}

impl Debug for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deck")
            .field("name", &self.name)
            .field("faces", &self.faces)
            .field("cards", &self.cards.len())
            .finish()
    }
}

impl Deref for Deck {
    type Target = Vec<Card>;

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Card(Vec<String>);

impl Deref for Card {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum DeckError {
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    NotEnoughFaces(Deck),
    NotEnoughCards(Deck),
    InvalidCard(Deck, CardError),
}

#[derive(Debug)]
pub enum CardError {
    NotEnoughFaces(Card),
    TooManyFaces(Card),
}

impl From<std::io::Error> for DeckError {
    fn from(err: std::io::Error) -> Self {
        DeckError::IoError(err)
    }
}

impl From<serde_json::Error> for DeckError {
    fn from(err: serde_json::Error) -> Self {
        DeckError::SerdeError(err)
    }
}

pub fn load_decks<P: Into<PathBuf> + Clone>(paths: Vec<P>) -> Result<Vec<Deck>, DeckError> {
    let len = paths.len();

    paths
        .into_iter()
        .try_fold(Vec::with_capacity(len), |mut decks, path| {
            decks.extend(load_decks_from_path(path)?);
            Ok(decks)
        })
}

fn load_decks_from_path(path: impl Into<PathBuf> + Clone) -> Result<Vec<Deck>, DeckError> {
    let metadata = std::fs::metadata(path.clone().into())?;

    if metadata.is_dir() {
        load_decks_from_dir(path)
    }
    //NOTE: Making a possibly bold assumption that the only alternative is a file
    else {
        load_deck_from_file(path).map(|deck| vec![deck])
    }
}

fn load_decks_from_dir(path: impl Into<PathBuf>) -> Result<Vec<Deck>, DeckError> {
    let path = path.into();
    let files = fs::read_dir(path)?
        .filter_map(|file| file.ok())
        .collect::<Vec<_>>();
    let len = files.len();

    files
        .into_iter()
        .try_fold(Vec::with_capacity(len), |mut decks, file| {
            decks.extend(load_decks_from_path(file.path())?);
            Ok(decks)
        })
}

fn load_deck_from_file(path: impl Into<PathBuf>) -> Result<Deck, DeckError> {
    let path = path.into();
    let json = std::fs::read_to_string(path)?;
    let deck = serde_json::from_str(&json)?;

    validate_deck(deck)
}

///Card within a deck must have at least two faces: a front and back
const MIN_FACE_COUNT: usize = 2;

fn validate_deck(deck: Deck) -> Result<Deck, DeckError> {
    if deck.cards.is_empty() {
        return Err(DeckError::NotEnoughCards(deck));
    }

    let expected_face_count = deck.faces.len();

    if expected_face_count < MIN_FACE_COUNT {
        return Err(DeckError::NotEnoughFaces(deck));
    }

    if let Some(card) = deck.iter().find(|card| card.len() != expected_face_count) {
        let card = card.clone();
        return Err(DeckError::InvalidCard(
            deck,
            if card.len() > expected_face_count {
                CardError::TooManyFaces(card)
            } else {
                CardError::NotEnoughFaces(card)
            },
        ));
    }

    Ok(deck)
}

#[cfg(test)]
mod tests {
    use crate::deck::{Deck, DeckError};

    use super::load_decks;

    #[test]
    fn deserialize_deck() -> serde_json::Result<()> {
        let deck_json = r#"
        {
            "name": "Kanji Words",
            "faces": ["Kanji", "Hiragana", "Definition"],
            "cards": [
                [
                    "日本",
                    "にほん",
                    "Japan"
                ]
            ]
        }"#;

        let deck: Deck = serde_json::from_str(deck_json)?;
        assert_eq!(deck.len(), 1);
        assert_eq!(deck.name, "Kanji Words");
        assert_eq!(deck.faces.len(), 3);
        assert_eq!(deck[0][2], "Japan");
        Ok(())
    }

    #[test]
    fn load_decks_from_files() {
        let decks = load_decks(vec!["./tests/deck1.json", "./tests/dir"]).unwrap();
        assert_eq!(decks.len(), 2);
    }

    #[test]
    fn load_invalid_decks_from_files() {
        //TODO: Add more tests for other invalid deck types
        assert!(load_decks(vec!["./tests/invalid_deck1.json"])
            .is_err_and(|err| matches!(err, DeckError::InvalidCard(_, _))))
    }
}
