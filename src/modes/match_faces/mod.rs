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

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};

use iter::MatchProblemIterator;
use widget::{MatchProblemWidget, MatchProblemWidgetState};

use crate::{
    event::clear_and_match_event, stats::Stats, terminal::TerminalWrapper, FlashrError,
    ModeArguments, Progress, PromptCard,
};

use super::flashcards::show_flashcards;

mod iter;
mod widget;

const ANSWERS_PER_PROBLEM: usize = 4;

struct MatchProblem<'a> {
    question: PromptCard<'a>,
    answers: Vec<(PromptCard<'a>, bool)>,
    answer_index: usize,
    weights: Option<Vec<f64>>,
}

struct Quit;

enum MatchResult<'a, 'b> {
    Correct(&'b PromptCard<'a>),
    Incorrect {
        q: &'b PromptCard<'a>,
        a: &'b PromptCard<'a>,
    },
}

pub fn match_faces(
    term: &mut TerminalWrapper,
    args: ModeArguments,
) -> Result<Progress, FlashrError> {
    let rng = &mut rand::thread_rng();
    let mut stats = Stats::load_from_user_home()?;
    let mut problems =
        MatchProblemIterator::new(args.deck_cards, &mut stats, args.faces, args.line, rng);

    fn update_correct(card: &PromptCard, stats: &mut Stats, problems: &mut MatchProblemIterator) {
        let stats = stats.for_card_mut(card);
        stats.correct += 1;
        problems.change_weight(card.index, stats.weight());
    }

    fn update_incorrect(card: &PromptCard, stats: &mut Stats, problems: &mut MatchProblemIterator) {
        let stats = stats.for_card_mut(card);
        stats.incorrect += 1;
        problems.change_weight(card.index, stats.weight());
    }

    let mut progress = Progress::default();
    let range = args.problem_count.map_or(0..usize::MAX, |count| 0..count);

    for _ in range {
        if let Some(problem) = problems.next() {
            let problem = &problem?;
            let result = show_match_problem(term, problem, progress.clone())?;

            match result {
                Ok(result) => match result {
                    MatchResult::Correct(card) => {
                        update_correct(card, &mut stats, &mut problems);
                        progress.add_correct();
                    }
                    MatchResult::Incorrect { q, a } => {
                        update_incorrect(q, &mut stats, &mut problems);
                        update_incorrect(a, &mut stats, &mut problems);
                        progress.add_incorrect();
                    }
                },
                Err(Quit) => break,
            }
        } else {
            break;
        }
    }

    stats.save_to_file()?;

    Ok(progress)
}

type MatchProblemResult<'a, 'b> = Result<MatchResult<'a, 'b>, Quit>;

fn show_match_problem<'a, 'b>(
    term: &mut TerminalWrapper,
    problem: &'b MatchProblem<'a>,
    progress: Progress,
) -> Result<MatchProblemResult<'a, 'b>, FlashrError> {
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(MatchProblemWidget::new(problem, &progress), widget_state)?;

        let input = clear_and_match_event(|event| match_user_input(event, widget_state))?;
        match input {
            UserInput::Answer(index_answered) => {
                return show_match_problem_result(term, problem, progress, index_answered)
            }
            UserInput::Resize | UserInput::EnterFlashcard(_) => continue,
            UserInput::Quit => return Ok(Err(Quit)),
        }
    }
}

fn show_match_problem_result<'a, 'b>(
    term: &mut TerminalWrapper,
    problem: &'b MatchProblem<'a>,
    progress: Progress,
    index_answered: usize,
) -> Result<MatchProblemResult<'a, 'b>, FlashrError> {
    let correct = index_answered == problem.answer_index;
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(
            MatchProblemWidget::new(problem, &progress).answered((index_answered, correct)),
            widget_state,
        )?;

        let input = clear_and_match_event(|event| match_user_input(event, widget_state))?;
        match input {
            UserInput::Answer(answer) if answer == problem.answer_index => {
                return Ok(Ok(if correct {
                    MatchResult::Correct(&problem.question)
                } else {
                    MatchResult::Incorrect {
                        q: &problem.question,
                        a: problem
                            .answers
                            .iter()
                            .enumerate()
                            .find_map(|(i, (card, _))| (i == index_answered).then_some(card))
                            .expect("Unable to find selected answer in problem answers"),
                    }
                }))
            }
            UserInput::EnterFlashcard(specific) => match specific {
                None => {
                    show_flashcards(
                        term,
                        problem
                            .answers
                            .iter()
                            .map(|(card, _)| card.deck_card)
                            .collect(),
                    )?;
                }
                Some(specific_index) => {
                    show_flashcards(
                        term,
                        problem
                            .answers
                            .iter()
                            .enumerate()
                            .filter_map(|(i, (card, _))| {
                                (specific_index == i).then_some(card.deck_card)
                            })
                            .collect(),
                    )?;
                }
            },
            UserInput::Answer(_) | UserInput::Resize => continue,
            UserInput::Quit => return Ok(Err(Quit)),
        }
    }
}

enum UserInput {
    Answer(usize),
    EnterFlashcard(Option<usize>),
    Resize,
    Quit,
}

fn match_user_input(event: Event, state: &MatchProblemWidgetState) -> Option<UserInput> {
    match event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            ..
        }) => match code {
            KeyCode::Char('1') => Some(UserInput::Answer(0)),
            KeyCode::Char('2') => Some(UserInput::Answer(1)),
            KeyCode::Char('3') => Some(UserInput::Answer(2)),
            KeyCode::Char('4') => Some(UserInput::Answer(3)),
            KeyCode::Enter => Some(UserInput::EnterFlashcard(None)),
            KeyCode::Char('!') => Some(UserInput::EnterFlashcard(Some(0))),
            KeyCode::Char('@') => Some(UserInput::EnterFlashcard(Some(1))),
            KeyCode::Char('#') => Some(UserInput::EnterFlashcard(Some(2))),
            KeyCode::Char('$') => Some(UserInput::EnterFlashcard(Some(3))),
            KeyCode::Esc | KeyCode::Char('q') => Some(UserInput::Quit),
            _ => None,
        },
        Event::Resize(_, _) => Some(UserInput::Resize),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(_),
            column,
            row,
            ..
        }) => state
            .answer_areas
            .iter()
            .enumerate()
            .find(|(_, area)| area.contains((column, row).into()))
            .map(|(index, _)| UserInput::Answer(index)),
        _ => None,
    }
}
