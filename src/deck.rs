use std::{
    ffi::OsStr,
    fmt::{Debug, Display},
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

use rand::{rngs::ThreadRng, seq::SliceRandom};
use serde::{de::Visitor, ser::SerializeSeq, Deserialize, Serialize};

///Represents a deck of flashcards. Each card must have the same number of faces as
///the deck's own faces array, though any number of those faces may optionally be null/None
///as long as at least two are non-nullish/Some. Faces may also be subdivided into subfaces
///which will be randomized when shown as questions/answers.
///
///Example:
///```
///# use flashr::deck::Deck;
///let json = r#"{
///  "name": "Kanji Words",
///  "faces": ["Kanji", "Hiragana", "Definition"],
///  "cards": [
///    ["日本", "にほん", "Japan"],
///    [null, "いいえ", ["No", "Don't mention it (eg in reply to apology/praise)"]]
///  ]
///}"#;
///assert!(serde_json::from_str::<Deck>(json)
///  .is_ok_and(|deck| {
///    deck.name == "Kanji Words" && deck.cards.len() == 2
///  }));
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

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
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

impl Face {
    pub fn join(&self, sep: &str) -> String {
        match self {
            Self::Single(face) => face.clone(),
            Self::Multi(faces) => faces.join(sep),
        }
    }

    pub fn join_random(&self, sep: &str, rng: &mut ThreadRng) -> String {
        match self {
            Self::Single(face) => face.clone(),
            Self::Multi(faces) => {
                let mut faces = faces.clone();
                faces.shuffle(rng);
                faces.join(sep)
            }
        }
    }

    pub fn is_multi_and<F>(&self, func: F) -> bool
    where
        F: FnOnce(&[String]) -> bool,
    {
        match self {
            Self::Multi(vec) => func(vec),
            Self::Single(_) => false,
        }
    }
}

impl Display for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.join(", "))
    }
}

impl Serialize for Face {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Single(face) => serializer.serialize_str(face),
            Self::Multi(faces) => {
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

#[derive(Debug)]
pub enum DeckError {
    IoError(PathBuf, std::io::Error),
    SerdeError(PathBuf, serde_json::Error),
    NotEnoughFaces(Deck),
    DuplicateFace(Deck, String),
    DuplicateDeckNames(String),
    InvalidCard(Deck, CardError),
}

impl Display for DeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(path, err) => f.write_fmt(format_args!(
                "IoError: {err}, path: {}",
                path.to_str().unwrap_or("unknown")
            )),
            Self::SerdeError(path, err) => f.write_fmt(format_args!(
                "SerdeError: {err}, path: {}",
                path.to_str().unwrap_or("unknown")
            )),
            Self::NotEnoughFaces(deck) => f.write_fmt(format_args!(
                "NotEnoughFaces: Deck \"{}\" does not have enough faces. Requires two, has {}",
                deck.name,
                deck.faces.len()
            )),
            Self::DuplicateFace(deck, face) => f.write_fmt(format_args!(
                "DuplicateFaces: Deck \"{}\" has more than one \"{face}\" face",
                deck.name
            )),
            Self::DuplicateDeckNames(name) => f.write_fmt(format_args!(
                "DuplicateDecks: At least two decks loaded have the same name, {name}"
            )),
            Self::InvalidCard(deck, err) => f.write_fmt(format_args!(
                "InvalidCard: Deck \"{}\" contains an invalid card: {err}",
                deck.name
            )),
        }
    }
}

#[derive(Debug)]
pub enum CardError {
    DuplicateFront(Box<(Face, Card, Card)>),
    EmptyFace(Card),
    NotEnoughFaces(Card, usize),
    NotEnoughUsableFaces(Card),
    TooManyFaces(Card, usize),
}

impl Display for CardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateFront(card_box) => {
                let (front, card_a, card_b) = card_box.as_ref();
                f.write_fmt(format_args!(
                    "\"{card_a}\" and \"{card_b}\" both have the same front, {front}"
                ))
            }
            Self::EmptyFace(card) => {
                f.write_fmt(format_args!("\"{card}\" has at least one empty face"))
            }
            Self::NotEnoughFaces(card, expected) => {
                let front = card.front_string();
                let face_count = card.len();
                f.write_fmt(format_args!("Card with front \"{front}\" does not have enough faces. Has {face_count}, needs {expected}"))
            }
            Self::NotEnoughUsableFaces(card) => {
                let front = card.front_string();
                let face_count = card.len();
                f.write_fmt(format_args!("Card with front \"{front}\" does not have enough usable (non-null) faces. Has {face_count}, needs {}", MIN_FACE_COUNT))
            }
            Self::TooManyFaces(card, expected) => {
                let front = card.front_string();
                let face_count = card.len();
                f.write_fmt(format_args!("Card with front \"{front}\" has too many faces. Has {face_count}, needs {expected}"))
            }
        }
    }
}

pub fn load_decks<P: Into<PathBuf>>(paths: Vec<P>) -> Result<Vec<Deck>, DeckError> {
    let len = paths.len();

    let decks = paths
        .into_iter()
        .try_fold(Vec::with_capacity(len), |mut decks, path| {
            decks.extend(load_decks_from_path(path.into())?.into_iter().flatten());
            Ok(decks)
        })?;

    validate_decks(&decks)?;

    Ok(decks)
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
                CardError::TooManyFaces(card, expected_face_count)
            } else {
                CardError::NotEnoughFaces(card, expected_face_count)
            },
        ));
    }

    if let Some(card) = deck
        .iter()
        .find(|card| card.iter().flatten().count() < MIN_FACE_COUNT)
    {
        let card = card.clone();
        return Err(DeckError::InvalidCard(
            deck,
            CardError::NotEnoughUsableFaces(card),
        ));
    }

    if let Some(card) = deck.iter().find(|card| {
        card.iter()
            .flatten()
            .any(|face| face.is_multi_and(|faces| faces.is_empty()))
    }) {
        let card = card.clone();
        return Err(DeckError::InvalidCard(deck, CardError::EmptyFace(card)));
    }

    if let Some(card_box) = deck.iter().enumerate().find_map(|(i, card_a)| {
        card_a.front().and_then(|front_a| {
            deck.iter().enumerate().find_map(|(j, card_b)| {
                if i != j {
                    card_b.front().and_then(|front_b| {
                        if front_a == front_b {
                            Some(Box::new((front_a.clone(), card_a.clone(), card_b.clone())))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
        })
    }) {
        return Err(DeckError::InvalidCard(
            deck,
            CardError::DuplicateFront(card_box),
        ));
    }

    Ok(deck)
}

fn validate_decks(decks: &[Deck]) -> Result<(), DeckError> {
    let deck_names = decks.iter().map(|deck| &deck.name).collect::<Vec<_>>();

    if let Some(name) = deck_names.iter().enumerate().find_map(|(i, deck_a)| {
        if deck_names
            .iter()
            .enumerate()
            .any(|(j, deck_b)| i != j && deck_a == deck_b)
        {
            Some(deck_a)
        } else {
            None
        }
    }) {
        return Err(DeckError::DuplicateDeckNames((*name).clone()));
    }

    Ok(())
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
        let file = File::create("./tests/test_serialize.json").unwrap();
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
        let decks = load_decks(vec![
            "./tests/deck1.json",
            "./tests/dir",
            "./tests/empty_dir",
        ])
        .unwrap();
        assert_eq!(decks.len(), 3);
    }

    #[test]
    fn load_decks_duplicate_deck_names() {
        assert!(load_decks(vec!["./tests/duplicate_deck_names"])
            .is_err_and(|err| matches!(err, DeckError::DuplicateDeckNames(_))))
    }

    #[test]
    fn load_decks_from_file() {
        let decks = load_decks(vec!["./tests/example.json"]).unwrap();
        assert_eq!(decks.len(), 1);
    }

    #[test]
    fn load_decks_from_non_deck_file() {
        let decks = load_decks(vec!["./tests/dir/another_random_file.txt"]).unwrap();
        assert_eq!(decks.len(), 0);
    }

    #[test]
    fn load_decks_from_empty_folder() {
        let decks = load_decks(vec!["./tests/empty_dir"]).unwrap();
        assert_eq!(decks.len(), 0);
    }

    #[test]
    fn load_decks_from_folder() {
        let decks = load_decks(vec!["./tests/dir"]).unwrap();
        assert_eq!(decks.len(), 2);
    }

    #[test]
    fn load_decks_with_subfaces() {
        let decks = load_decks(vec!["./tests/deck_subfaces.json"]).unwrap();
        assert!(decks.iter().any(|deck| {
            deck.cards.iter().any(|card| {
                card.iter()
                    .flatten()
                    .any(|face| matches!(face, Face::Multi(_)))
            })
        }));
    }

    #[test]
    fn load_deck_with_no_cards() {
        let decks = load_decks(vec!["./tests/not_enough_cards.json"]).unwrap();
        assert!(decks.first().is_some_and(|deck| deck.cards.is_empty()));
    }

    #[test]
    fn load_deck_not_enough_faces() {
        assert!(load_decks(vec!["./tests/not_enough_faces.json"])
            .is_err_and(|err| matches!(err, DeckError::NotEnoughFaces(_))));
    }

    #[test]
    fn load_deck_duplicate_faces() {
        assert!(load_decks(vec!["./tests/duplicate_faces.json"])
            .is_err_and(|err| matches!(err, DeckError::DuplicateFace(_, _))));
    }

    #[test]
    fn load_deck_not_enough_card_faces() {
        assert!(
            load_decks(vec!["./tests/not_enough_card_faces.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::NotEnoughFaces(_, _))
            ))
        );
    }

    #[test]
    fn load_deck_too_many_card_faces() {
        assert!(
            load_decks(vec!["./tests/too_many_card_faces.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::TooManyFaces(_, _))
            ))
        );
    }

    #[test]
    fn load_deck_not_enough_usable_card_faces() {
        assert!(
            load_decks(vec!["./tests/not_enough_usable_card_faces.json"]).is_err_and(
                |err| matches!(
                    err,
                    DeckError::InvalidCard(_, CardError::NotEnoughUsableFaces(_))
                )
            )
        );
    }

    #[test]
    fn load_deck_duplicate_card_front() {
        assert!(
            load_decks(vec!["./tests/duplicate_card_front.json"]).is_err_and(|err| matches!(
                err,
                DeckError::InvalidCard(_, CardError::DuplicateFront(_))
            ))
        );
    }

    #[test]
    fn load_deck_duplicate_card_front_subfaced() {
        assert!(
            load_decks(vec!["./tests/duplicate_card_front_subfaced.json"]).is_err_and(
                |err| matches!(err, DeckError::InvalidCard(_, CardError::DuplicateFront(_)))
            )
        );
    }
}
