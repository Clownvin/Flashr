use std::ops::AddAssign;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};

use iter::MatchProblemIterator;
use widget::{MatchProblemWidget, MatchProblemWidgetState};

use crate::{
    event::clear_and_match_event, stats::Stats, terminal::TerminalWrapper, CorrectIncorrect,
    FlashrError, ModeArguments, PromptCard,
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

type MatchProblemResult<'a, 'b> = Result<MatchResult<'a, 'b>, Quit>;

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
) -> Result<CorrectIncorrect, FlashrError> {
    let rng = &mut rand::thread_rng();
    let mut stats = Stats::load_from_user_home()?;
    let mut problems =
        MatchProblemIterator::new(args.deck_cards, &mut stats, args.faces, args.line, rng);

    let mut total_correct = 0;

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

    fn match_result(
        result: MatchResult,
        total_correct: &mut usize,
        stats: &mut Stats,
        problems: &mut MatchProblemIterator,
    ) {
        match result {
            MatchResult::Correct(card) => {
                total_correct.add_assign(1);
                update_correct(card, stats, problems);
            }
            MatchResult::Incorrect { q, a } => {
                update_incorrect(q, stats, problems);
                update_incorrect(a, stats, problems);
            }
        }
    }

    let (total_correct, total) = if let Some(count) = args.problem_count {
        for _ in 0..count {
            if let Some(problem) = problems.next() {
                let progress = (total_correct, count);
                match show_match_problem(term, &problem?, progress)? {
                    Ok(result) => {
                        match_result(result, &mut total_correct, &mut stats, &mut problems)
                    }
                    Err(Quit) => break,
                }
            } else {
                break;
            }
        }

        (total_correct, count)
    } else {
        let mut total = 0;

        for _ in 0.. {
            if let Some(problem) = problems.next() {
                let progress = (total_correct, total);
                match show_match_problem(term, &problem?, progress)? {
                    Ok(result) => {
                        total += 1;

                        match_result(result, &mut total_correct, &mut stats, &mut problems)
                    }
                    Err(Quit) => break,
                }
            } else {
                break;
            }
        }

        (total_correct, total)
    };

    stats.save_to_file()?;
    Ok((total_correct, total))
}

fn show_match_problem<'a, 'b>(
    term: &mut TerminalWrapper,
    problem: &'b MatchProblem<'a>,
    progress: (usize, usize),
) -> Result<MatchProblemResult<'a, 'b>, FlashrError> {
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(MatchProblemWidget::new(problem, progress), widget_state)?;

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
    progress: (usize, usize),
    index_answered: usize,
) -> Result<MatchProblemResult<'a, 'b>, FlashrError> {
    let correct = index_answered == problem.answer_index;
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(
            MatchProblemWidget::new(problem, progress).answered((index_answered, correct)),
            widget_state,
        )?;

        let input = clear_and_match_event(|event| match_user_input(event, widget_state))?;
        match input {
            UserInput::Answer(answer) if answer == problem.answer_index => {
                return Ok(if correct {
                    Ok(MatchResult::Correct(&problem.question))
                } else {
                    Ok(MatchResult::Incorrect {
                        q: &problem.question,
                        a: problem
                            .answers
                            .iter()
                            .enumerate()
                            .find_map(|(i, (card, _))| (i == index_answered).then_some(card))
                            .expect("Unable to find selected answer in problem answers"),
                    })
                })
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
