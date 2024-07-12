use flashr::{
    deck::{CardError, DeckError, Face},
    FlashrError,
};

fn main() {
    let result = flashr::run();
    match result {
        Ok((total_correct, total)) => {
            println!(
                "You got {total_correct} correct out of {total} ({:.2}%)",
                if total == 0 {
                    0.0
                } else {
                    (total_correct as f64 / total as f64) * 100.0
                }
            );
            if total_correct == total && total > 0 {
                println!("Well done!");
            }
        }
        Err(err) => match err {
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
                        let front = card
                            .first()
                            .map(Face::to_string)
                            .unwrap_or("MISSING FRONT".to_owned());
                        eprintln!("InvalidCard: NotEnoughFaces: Card with front \"{}\" does not have enough faces. Has {}, needs {}", front, card.len(), deck.faces.len())
                    }
                    CardError::TooManyFaces(card) => {
                        let front = card
                            .first()
                            .map(Face::to_string)
                            .unwrap_or("MISSING FRONT".to_owned());
                        eprintln!("InvalidCard: TooManyFaces: Card with front \"{}\" has too many faces. Has {}, needs {}", front, card.len(), deck.faces.len())
                    }
                },
                DeckError::IoError(path, err) => {
                    eprintln!(
                        "IoError: {err}, path: {}",
                        path.to_str().unwrap_or("unknown")
                    )
                }
                DeckError::SerdeError(path, err) => {
                    eprintln!(
                        "SerdeError: {err}, path: {}",
                        path.to_str().unwrap_or("unknown")
                    )
                }
            },
            FlashrError::UiError(err) => match err {
                flashr::UiError::IoError(err) => eprintln!("UiError: IoError: {err}"),
            },
        },
    }
}
