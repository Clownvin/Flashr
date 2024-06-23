use flashr::{
    deck::{CardError, DeckError},
    FlashrError,
};

fn main() {
    let result = flashr::run();
    if let Err(err) = result {
        match err {
            FlashrError::DeckMismatchError(reason) => eprintln!("DeckMismatch: {reason}"),
            FlashrError::DeckError(err) => match err {
                DeckError::NotEnoughCards(deck) => eprintln!(
                    "NotEnoughCards: Deck \"{}\" does not have enough cards.",
                    deck.name
                ),
                DeckError::NotEnoughFaces(deck) => eprintln!(
                    "NotEnoughFaces: Deck \"{}\" does not have enough faces. Requires two, has {}",
                    deck.name,
                    deck.faces.len()
                ),
                DeckError::DuplicateFace(deck, face) => eprintln!(
                    "DuplicateFace: Deck \"{}\" has at least two \"{}\" faces",
                    deck.name, face
                ),
                DeckError::InvalidCard(deck, card_err) => match card_err {
                    CardError::NotEnoughFaces(card) => {
                        let front = card.first().cloned().unwrap_or("MISSING FRONT".to_owned());
                        eprintln!("InvalidCard: NotEnoughFaces: Card with front \"{}\" does not have enough faces. Has {}, needs {}", front, card.len(), deck.faces.len())
                    }
                    CardError::TooManyFaces(card) => {
                        let front = card.first().cloned().unwrap_or("MISSING FRONT".to_owned());
                        eprintln!("InvalidCard: TooManyFaces: Card with front \"{}\" has too many faces. Has {}, needs {}", front, card.len(), deck.faces.len())
                    }
                },
                DeckError::IoError(err) => {
                    eprintln!("IoError: {err}")
                }
                DeckError::SerdeError(err) => {
                    eprintln!("SerdeError: {err}")
                }
            },
            _ => eprintln!("Todo"),
        }
    }
}
