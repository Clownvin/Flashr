# Flashr
Extremely simple and lightweight [TUI (Text/terminal-based user interface)](https://en.wikipedia.org/wiki/Text-based_user_interface) based flashcard application, written in [Rust](https://www.rust-lang.org/) and using [Ratatui](https://ratatui.rs/) for rendering. Decks are stored as JSONs, and many can be loaded at once. Currently only supports one mode, "match", which shows a "question" face and prompts for the user for a multiple choice answer.

## Installation
Simply clone the repository, and then run:
```sh
cargo build --release
```
This will create an executable in ./target/release/, which you can then link/copy/use as needed.

## Usage
Example deck (`example.json`):
```json
{
    "name": "Example",
    "faces": ["Front", "Middle", "Back"],
    "cards": [
        ["Front 1", "Middle 1", "Back 1"],
        [["Front 2, 1", "Front 2, 3"], ["Middle 2, 1", "Middle 2, 2"], "Back 2"],
        [null, "Middle 3", ["Back 3, 1", "Back 3, 2", "Back 3, 3"]],
        [["Front 4"], null, "Back 4"],
        ["Front 5", "Middle 5", "Back 5"]
    ]
}
```
Note that:
- You can have any number of deck.faces
- Each card must have at the same number of faces as the deck
- Cards may have nulls to represent missing faces, as long as they have at least two non-null faces they will be valid
- Each card's face may be subdivided, and the subdivisions will be joined randomly when shown as problems/questions. The idea is to reduce memorization of sentence structures/order of definitions.
- Decks may have NO cards present

To run the program using the `example.json` deck:
```sh
flashr example.json
```

Note that you can provide any number of paths to files/directories with decks. See `flashr -h` for more usage information.
