use std::{
    ffi::OsStr,
    fmt::{Debug, Display},
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

use rand::{rngs::ThreadRng, seq::SliceRandom};
use serde::{de::Visitor, ser::SerializeSeq, Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Card(Vec<Option<Face>>);

impl Card {
    pub fn join(&self, sep: &str) -> String {
        self.iter()
            .flatten()
            .map(Face::to_string)
            .intersperse(sep.to_owned())
            .collect::<String>()
    }

    pub fn front(&self) -> Option<&Face> {
        self.iter().flatten().next()
    }

    pub fn front_string(&self) -> String {
        self.front()
            .map(Face::to_string)
            .unwrap_or("MISSING_FRONT".to_owned())
    }
}

impl Deref for Card {
    type Target = Vec<Option<Face>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Face {
    Single(String),
    Multi(Vec<String>),
}

impl Serialize for Face {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Face::Single(face) => serializer.serialize_str(face),
            Face::Multi(faces) => {
                let mut seq = serializer.serialize_seq(Some(faces.len()))?;
                for face in faces {
                    seq.serialize_element(face)?;
                }
                seq.end()
            }
        }
    }
}

struct FaceVisitor;

impl<'de> Visitor<'de> for FaceVisitor {
    type Value = Face;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string or a sequence of strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut faces = match seq.size_hint() {
            Some(size) => Vec::with_capacity(size),
            None => vec![],
        };

        while let Some(next) = seq.next_element()? {
            faces.push(next);
        }

        Ok(Face::Multi(faces))
    }

    fn visit_str<E>(self, face: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Face::Single(face.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Face {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FaceVisitor)
    }
}

impl Face {
    pub fn join(&self) -> String {
        match self {
            Face::Single(face) => face.clone(),
            Face::Multi(faces) => faces.join(", "),
        }
    }

    pub fn join_random(&self, rng: &mut ThreadRng) -> String {
        match self {
            Face::Single(face) => face.clone(),
            Face::Multi(faces) => {
                let mut faces = faces.clone();
                faces.shuffle(rng);
                faces.join(", ")
            }
        }
    }
}

impl Display for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.join()))
    }
}

#[derive(Debug)]
pub enum DeckError {
    IoError(PathBuf, std::io::Error),
    SerdeError(PathBuf, serde_json::Error),
    NotEnoughFaces(Deck),
    NotEnoughCards(Deck),
    DuplicateFace(Deck, String),
    InvalidCard(Deck, CardError),
}

#[derive(Debug)]
pub enum CardError {
    NotEnoughFaces(Card),
    TooManyFaces(Card),
}

pub fn load_decks<P: Into<PathBuf> + Clone>(paths: Vec<P>) -> Result<Vec<Deck>, DeckError> {
    let len = paths.len();

    paths
        .into_iter()
        .try_fold(Vec::with_capacity(len), |mut decks, path| {
            decks.extend(load_decks_from_path(path.into())?.into_iter().flatten());
            Ok(decks)
        })
}

fn load_decks_from_path(path: PathBuf) -> Result<Option<Vec<Deck>>, DeckError> {
    let metadata = std::fs::metadata(&path).map_err(|err| DeckError::IoError(path.clone(), err))?;

    if metadata.is_dir() {
        load_decks_from_dir(path).map(Some)
    } else if file_extension(&path).is_some_and(|ext| ext.to_lowercase() == "json") {
        load_deck_from_file(path).map(|deck| Some(vec![deck]))
    } else {
        Ok(None)
    }
}

fn file_extension(path: &PathBuf) -> Option<&str> {
    let path = Path::new(path);
    path.extension().and_then(OsStr::to_str)
}

fn load_decks_from_dir(path: PathBuf) -> Result<Vec<Deck>, DeckError> {
    let files = fs::read_dir(&path)
        .map_err(|err| DeckError::IoError(path, err))?
        .filter_map(|file| file.ok())
        .collect::<Vec<_>>();
    let len = files.len();

    files
        .into_iter()
        .try_fold(Vec::with_capacity(len), |mut decks, file| {
            decks.extend(load_decks_from_path(file.path())?.into_iter().flatten());
            Ok(decks)
        })
}

fn load_deck_from_file(path: PathBuf) -> Result<Deck, DeckError> {
    let json =
        std::fs::read_to_string(&path).map_err(|err| DeckError::IoError(path.clone(), err))?;
    let deck = serde_json::from_str(&json).map_err(|err| DeckError::SerdeError(path, err))?;

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

    let mut seen_faces = Vec::with_capacity(expected_face_count);

    if let Some(face) = deck.faces.iter().find(|face| {
        if seen_faces.contains(face) {
            true
        } else {
            seen_faces.push(face);
            false
        }
    }) {
        let face = face.clone();
        return Err(DeckError::DuplicateFace(deck, face));
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

    if let Some(card) = deck
        .iter()
        .find(|card| card.iter().filter(|face| !face.is_none()).count() <= 1)
    {
        let card = card.clone();
        return Err(DeckError::InvalidCard(
            deck,
            CardError::NotEnoughFaces(card),
        ));
    }

    Ok(deck)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter};

    use crate::deck::{CardError, Deck, DeckError, Face};

    use super::{load_decks, Card};

    #[test]
    fn serialize_deck() {
        let deck: Deck = Deck {
            name: "Test".to_owned(),
            faces: vec!["Face 1".to_owned(), "Face 2".to_owned()],
            cards: vec![Card(vec![
                Some(Face::Single("Front".to_owned())),
                Some(Face::Multi(vec!["Back".to_owned(), "With many".to_owned()])),
                None,
            ])],
        };
        let file = File::create("test_ser.json").unwrap();
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &deck).unwrap();
    }

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
        assert_eq!(deck[0][2], Some(Face::Single("Japan".into())));
        Ok(())
    }

    #[test]
    fn load_decks_from_files() {
        let decks = load_decks(vec!["./tests/deck1.json", "./tests/dir"]).unwrap();
        assert_eq!(decks.len(), 3);
    }

    #[test]
    fn load_invalid_decks_from_files() {
        assert!(
            load_decks(vec!["./tests/not_enough_card_faces.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::NotEnoughFaces(_))
            ))
        );
        assert!(
            load_decks(vec!["./tests/too_many_card_faces.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::TooManyFaces(_))
            ))
        );
        assert!(load_decks(vec!["./tests/not_enough_faces.json"])
            .is_err_and(|err| matches!(err, DeckError::NotEnoughFaces(_))));
        assert!(load_decks(vec!["./tests/not_enough_cards.json"])
            .is_err_and(|err| matches!(err, DeckError::NotEnoughCards(_))));
        assert!(load_decks(vec!["./tests/duplicate_face.json"])
            .is_err_and(|err| matches!(err, DeckError::DuplicateFace(_, _))));
        assert!(
            load_decks(vec!["./tests/not_enough_non_null_faces.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::NotEnoughFaces(_))
            ))
        )
    }
}
