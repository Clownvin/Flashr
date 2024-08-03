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

use std::ops::{Deref, Index};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
};
use widget::{FlashcardWidget, FlashcardWidgetState};

use crate::{event::clear_and_match_event, terminal::TerminalWrapper, DeckCard, FlashrError};

mod widget;

enum Action {
    Prev,
    Next,
    Quit,
}

struct WrappingIndex<'a, T> {
    backing: &'a T,
    index: usize,
}

trait Length {
    fn length(&self) -> usize;
}

impl<T> Length for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<'a, T> WrappingIndex<'a, T>
where
    T: Index<usize> + Length,
{
    fn new(backing: &'a T) -> Self {
        Self { backing, index: 0 }
    }

    fn max_index(&self) -> usize {
        self.backing.length() - 1
    }

    fn next_index(&self) -> usize {
        let max_index = self.max_index();
        if self.index == max_index {
            0
        } else {
            self.index + 1
        }
    }

    fn prev_index(&self) -> usize {
        if self.index == 0 {
            self.max_index()
        } else {
            self.index - 1
        }
    }

    fn increment(&mut self) {
        self.index = self.next_index();
    }

    fn decrement(&mut self) {
        self.index = self.prev_index();
    }

    fn set(&mut self, index: usize) {
        self.index = index.min(self.max_index());
    }
}

impl<'a, T> Deref for WrappingIndex<'a, T> {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.index
    }
}

pub fn show_flashcards(
    term: &mut TerminalWrapper,
    deck_cards: Vec<DeckCard>,
) -> Result<(), FlashrError> {
    if deck_cards.is_empty() {
        return Ok(());
    }

    let mut index = WrappingIndex::new(&deck_cards);

    loop {
        let problem = FlashcardProblem {
            prev: deck_cards[index.prev_index()],
            current: deck_cards[*index],
            next: deck_cards[index.next_index()],
        };

        let action = show_flashcard(term, problem)?;

        match action {
            Action::Prev => index.decrement(),
            Action::Next => index.increment(),
            Action::Quit => break,
        };
    }

    Ok(())
}

struct FlashcardProblem<'a> {
    prev: DeckCard<'a>,
    current: DeckCard<'a>,
    next: DeckCard<'a>,
}

fn show_flashcard(
    term: &mut TerminalWrapper,
    problem: FlashcardProblem,
) -> Result<Action, FlashrError> {
    let faces = problem.current.possible_faces();
    let mut index = WrappingIndex::new(&faces);
    let state = &mut FlashcardWidgetState::default();

    loop {
        let (_, deck_face, card_face) = faces[*index];
        term.render_stateful_widget(
            FlashcardWidget::new(
                (deck_face, card_face),
                problem.prev.front_string(),
                problem.next.front_string(),
            ),
            state,
        )?;

        let input = clear_and_match_event(|event| match_user_input(event, state))?;

        match input {
            UserInput::NextFace => index.increment(),
            UserInput::PrevFace => index.decrement(),
            UserInput::ExactFace(exact_index) => index.set(exact_index),
            UserInput::NextCard => return Ok(Action::Next),
            UserInput::PrevCard => return Ok(Action::Prev),
            UserInput::Quit => return Ok(Action::Quit),
            UserInput::Resize => continue,
        };
    }
}

enum UserInput {
    NextFace,
    PrevFace,
    ExactFace(usize),
    NextCard,
    PrevCard,
    Resize,
    Quit,
}

fn match_user_input(event: Event, state: &FlashcardWidgetState) -> Option<UserInput> {
    match event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            ..
        }) => match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') => Some(UserInput::PrevCard),
            KeyCode::Down | KeyCode::Enter | KeyCode::Char('j') | KeyCode::Char('s') => {
                Some(UserInput::NextCard)
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('a') => Some(UserInput::PrevFace),
            KeyCode::Right | KeyCode::Char(' ') | KeyCode::Char('l') | KeyCode::Char('d') => {
                Some(UserInput::NextFace)
            }
            KeyCode::Esc | KeyCode::Char('q') => Some(UserInput::Quit),
            KeyCode::Char(char) => char.to_digit(10).map(|index| {
                UserInput::ExactFace(
                    (index as usize)
                        //NOTE: Subbing one so that '1' is actually index 0
                        //More logical than expecting the user to understand
                        //zero-indexing
                        .saturating_sub(1),
                )
            }),
            _ => None,
        },
        Event::Resize(_, _) => Some(UserInput::Resize),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(button),
            column,
            row,
            ..
        }) => Some({
            if state.left.contains((column, row).into()) {
                UserInput::PrevCard
            } else if state.right.contains((column, row).into()) {
                UserInput::NextCard
            } else {
                match button {
                    MouseButton::Left => UserInput::NextFace,
                    MouseButton::Right => UserInput::PrevFace,
                    MouseButton::Middle => UserInput::NextCard,
                }
            }
        }),
        _ => None,
    }
}
