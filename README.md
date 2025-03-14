# Flashr
Extremely simple and lightweight [TUI (Text/terminal-based user interface)](https://en.wikipedia.org/wiki/Text-based_user_interface) based flashcard application, written in [Rust](https://www.rust-lang.org/) and using [Ratatui](https://ratatui.rs/) for rendering. Decks are stored as [JSONs](https://en.wikipedia.org/wiki/JSON), and many can be loaded at once. I have found it rather nice to create a "deck tree" file structure, with decks arranged based on the type of content. That way it's very trivial to target the entire tree to go over everything, but also makes it easy to specify specific decks for more directed study.

Currently supports two modes, "match", which shows a "question" face and prompts for the user for a multiple choice answer, and "flash", which provides the typical flashcard experience.

## Installation
Simply clone the repository, and then run:
```sh
cargo build --release
```
This will create an executable in `./target/release/`, which you can then link/copy/use as needed.

You can also download one of the pre-built Linux releases.

## Usage
Example deck (`example.json`):
```json
{
    "name": "Example",
    "faces": ["Front", "Middle", "Back"],
    "cards": [
        ["Front 1", "Middle 1", "Back 1"],
        [["Front 2, 1", "Front 2, 2"], ["Middle 2, 1", "Middle 2, 2"], "Back 2"],
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

Note that you can provide any number of paths to files/directories with decks. See `flashr -h` or `flashr --help` for more usage information.
