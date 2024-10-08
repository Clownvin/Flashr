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

use clap::Parser;
use stats::StatsError;
use std::{
    fmt::Display,
    ops::{Deref, Not},
    str::FromStr,
};

use deck::{load_decks, Card, CardId, Deck, DeckError, Face};
use modes::{flashcards::show_flashcards, match_faces::match_faces};
use terminal::TerminalWrapper;

mod cli;
mod color;
pub mod deck;
mod event;
mod modes;
mod random;
mod render_utils;
mod stats;
mod terminal;
mod weighted_list;

pub fn run() -> Result<Option<Progress>, FlashrError> {
    let cli = cli::FlashrCli::parse();
    let decks = load_decks(cli.paths)?;
    let args = ModeArguments::new(&decks, cli.problem_count, cli.faces, cli.line);

    std::panic::catch_unwind(|| {
        //NOTE: From this point, stdout/stderr will not be usable, hence we
        //need to catch any panics, since they are not loggable.
        let term = &mut TerminalWrapper::new().map_err(UiError::IoError)?;

        let correct_incorrect = match cli.mode {
            Mode::Match => match_faces(term, args).map(Some),
            Mode::Flash => show_flashcards(term, args.deck_cards).map(|_| None),
            Mode::Type => todo!("Type mode not yet implemented"),
        }?;

        Ok(correct_incorrect)
    })
    .map_err(|err| {
        FlashrError::Panic({
            if let Some(msg) = err.downcast_ref::<&str>() {
                (*msg).to_owned()
            } else if let Some(msg) = err.downcast_ref::<String>() {
                msg.clone()
            } else {
                "Unknown panic occurred".to_owned()
            }
        })
    })?
}

type Faces = Option<Vec<String>>;
type ProblemCount = Option<usize>;

#[derive(Clone, Copy)]
struct DeckCard<'a> {
    deck: &'a Deck,
    card: &'a Card,
}

impl<'a> DeckCard<'a> {
    fn new(deck: &'a Deck, card: &'a Card) -> Self {
        Self { deck, card }
    }

    fn possible_faces(&self) -> Vec<(usize, &String, &Face)> {
        let mut possible_faces = Vec::with_capacity(self.deck.faces.len());
        for (index, deck_face) in self.deck.faces.iter().enumerate() {
            if let Some(card_face) = self.card[index].as_ref() {
                possible_faces.push((index, deck_face, card_face));
            }
        }
        possible_faces
    }
}

impl<'a> Deref for DeckCard<'a> {
    type Target = Card;

    fn deref(&self) -> &'a Self::Target {
        self.card
    }
}

struct PromptCard<'a> {
    prompt: String,
    deck_card: DeckCard<'a>,
    index: usize,
}

impl<'a> From<&PromptCard<'a>> for CardId {
    fn from(card: &PromptCard<'a>) -> Self {
        (&card.deck_card).into()
    }
}

#[derive(Clone, Debug)]
enum Mode {
    Match,
    Type,
    Flash,
}

impl FromStr for Mode {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if s == "match" {
            Ok(Self::Match)
        } else if s == "flash" {
            Ok(Self::Flash)
        } else if s == "type" {
            Ok(Self::Type)
        } else {
            Err(format!("Mode argument not recognized: {s}"))
        }
    }

    type Err = String;
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Match => "match",
            Mode::Type => "type",
            Mode::Flash => "flash",
        })
    }
}

struct ModeArguments<'a> {
    problem_count: ProblemCount,
    faces: Faces,
    deck_cards: Vec<DeckCard<'a>>,
    line: bool,
}

impl<'a> ModeArguments<'a> {
    fn new(decks: &'a [Deck], problem_count: ProblemCount, faces: Faces, line: bool) -> Self {
        let mut deck_cards = {
            let max_num_problems = decks.iter().fold(0, |total, deck| {
                total + (deck.cards.len() * deck.faces.len())
            });
            Vec::with_capacity(max_num_problems)
        };

        if let Some(faces) = faces.as_ref() {
            for deck in decks {
                let deck_faces = {
                    let mut buf = Vec::with_capacity(deck.faces.len());
                    deck.faces
                        .iter()
                        .enumerate()
                        .filter(|(_, deck_face)| faces.iter().any(|face| face == *deck_face))
                        .for_each(|(i, _)| buf.push(i));
                    buf
                };

                deck_faces.is_empty().not().then(|| {
                    for card in deck.cards.iter() {
                        if deck_faces.iter().any(|i| card[*i].is_some()) {
                            deck_cards.push(DeckCard::new(deck, card));
                        }
                    }
                });
            }
        } else {
            for deck in decks {
                for card in deck.cards.iter() {
                    deck_cards.push(DeckCard::new(deck, card));
                }
            }
        }
        Self {
            problem_count,
            faces,
            deck_cards,
            line,
        }
    }
}

#[repr(transparent)]
struct OptionTuple<T>(Option<(T, T)>);

impl<T> Deref for OptionTuple<T> {
    type Target = Option<(T, T)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromIterator<T> for OptionTuple<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        Self(
            iter.next()
                .and_then(|first| iter.next().map(|second| (first, second))),
        )
    }
}

trait AndThen {
    fn and_then<T>(&self, f: impl FnOnce() -> Option<T>) -> Option<T>;
}

impl AndThen for bool {
    fn and_then<T>(&self, f: impl FnOnce() -> Option<T>) -> Option<T> {
        if *self {
            f()
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Progress {
    pub correct: usize,
    pub total: usize,
}

impl Progress {
    pub fn ratio_percent(&self) -> (f64, f64) {
        let ratio = if self.total == 0 {
            //NOTE: Starting at ratio 1.0 so that
            //colors are "correct"
            1.0
        } else {
            self.correct as f64 / self.total as f64
        };

        (ratio, ratio * 100.0)
    }

    fn add_correct(&mut self) {
        self.correct += 1;
        self.total += 1;
    }

    fn add_incorrect(&mut self) {
        self.total += 1;
    }
}

#[derive(Debug)]
pub enum FlashrError {
    Deck(Box<DeckError>),
    Ui(UiError),
    DeckMismatch(String),
    Arg(ArgError),
    Stats(StatsError),
    Panic(String),
}

impl Display for FlashrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deck(err) => f.write_fmt(format_args!("Deck: {err}")),
            Self::DeckMismatch(err) => f.write_fmt(format_args!("DeckMismatch: {err}")),
            Self::Arg(err) => f.write_fmt(format_args!("Arg: {err}")),
            Self::Ui(err) => f.write_fmt(format_args!("Ui: {err}")),
            Self::Stats(err) => f.write_fmt(format_args!("Stats: {err}")),
            Self::Panic(err) => f.write_fmt(format_args!("Panicked: {err}")),
        }
    }
}

impl From<DeckError> for FlashrError {
    fn from(err: DeckError) -> Self {
        Self::Deck(Box::new(err))
    }
}

impl From<UiError> for FlashrError {
    fn from(err: UiError) -> Self {
        Self::Ui(err)
    }
}

impl From<ArgError> for FlashrError {
    fn from(err: ArgError) -> Self {
        Self::Arg(err)
    }
}

impl From<StatsError> for FlashrError {
    fn from(err: StatsError) -> Self {
        Self::Stats(err)
    }
}

#[derive(Debug)]
pub enum UiError {
    IoError(std::io::Error),
}

impl Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => f.write_fmt(format_args!("IoError: {err}")),
        }
    }
}

impl From<std::io::Error> for UiError {
    fn from(err: std::io::Error) -> Self {
        UiError::IoError(err)
    }
}

#[derive(Debug)]
pub enum ArgError {
    DeckNotEnoughFaces(Vec<String>, String),
}

impl Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeckNotEnoughFaces(faces, deck) => {
                let faces = faces.join(", ");
                f.write_fmt(format_args!("Deck \"{deck}\" does not have enough faces for arguments:\nNeeds at least one of: {faces}"))
            }
        }
    }
}
