use std::ops::AddAssign;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};

use iter::MatchProblemIterator;
use widget::{MatchProblemWidget, MatchProblemWidgetState};

use crate::{
    event::{clear_and_match_event, UserInput},
    stats::Stats,
    terminal::TerminalWrapper,
    CorrectIncorrect, FlashrError, ModeArguments, ModeResult, PromptCard,
};

mod iter;
mod widget;

const ANSWERS_PER_PROBLEM: usize = 4;

struct MatchProblem<'a> {
    question: PromptCard<'a>,
    answers: Vec<(PromptCard<'a>, bool)>,
    answer_index: usize,
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
    mut term: TerminalWrapper,
    args: ModeArguments,
) -> Result<ModeResult, FlashrError> {
    let term = &mut term;
    let rng = &mut rand::thread_rng();
    let mut stats = args.stats;
    let mut problems = MatchProblemIterator::new(args.deck_cards, &mut stats, args.faces, rng);

    let mut total_correct = 0;

    #[inline(always)]
    fn update_correct(card: &PromptCard, stats: &mut Stats, problems: &mut MatchProblemIterator) {
        let stats = stats.for_card_mut(card);
        stats.correct += 1;
        problems.change_weight(card.index, stats.weight());
    }

    #[inline(always)]
    fn update_incorrect(card: &PromptCard, stats: &mut Stats, problems: &mut MatchProblemIterator) {
        let stats = stats.for_card_mut(card);
        stats.incorrect += 1;
        problems.change_weight(card.index, stats.weight());
    }

    #[inline(always)]
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

    if let Some(count) = args.problem_count {
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

        Ok(((total_correct, count), stats))
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

        Ok(((total_correct, total), stats))
    }
}

fn show_match_problem<'a, 'b>(
    term: &mut TerminalWrapper,
    problem: &'b MatchProblem<'a>,
    progress: CorrectIncorrect,
) -> Result<MatchProblemResult<'a, 'b>, FlashrError> {
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(MatchProblemWidget::new(problem, progress), widget_state)?;

        let input = clear_and_match_event(|event| match_user_input(event, widget_state))?;
        match input {
            UserInput::Answer(index_answered) => {
                return show_match_problem_result(term, problem, progress, index_answered)
            }
            UserInput::Resize => continue,
            UserInput::Quit => return Ok(Err(Quit)),
        }
    }
}

fn show_match_problem_result<'a, 'b>(
    term: &mut TerminalWrapper,
    problem: &'b MatchProblem<'a>,
    progress: CorrectIncorrect,
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
                            .find_map(|(i, (card, _))| if i == answer { Some(card) } else { None })
                            .expect("Unable to find selected answer in problem answers"),
                    })
                })
            }
            UserInput::Answer(_) | UserInput::Resize => continue,
            UserInput::Quit => return Ok(Err(Quit)),
        }
    }
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
