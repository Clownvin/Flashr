use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Gauge, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    clear_and_match_event,
    deck::{Card, Deck},
    terminal::TerminalWrapper,
    FlashrError, ProblemResult, UserInput,
};

pub fn match_cards(
    term: &mut TerminalWrapper,
    decks: Vec<Deck>,
) -> Result<(usize, usize), FlashrError> {
    let suite = get_match_problem_suite(&decks)?;

    let total_problems = suite.problems.len();
    let mut total_correct = 0;

    for (i, problem) in suite.problems.into_iter().enumerate() {
        let result = show_match_problem(term, problem, i as f64 / total_problems as f64)?;

        match result {
            ProblemResult::Correct => total_correct += 1,
            ProblemResult::Quit => return Ok((total_correct, i)),
            ProblemResult::Incorrect => {}
        }
    }

    Ok((total_correct, total_problems))
}

struct MatchProblemSuite<'suite> {
    problems: Vec<MatchProblem<'suite>>,
}

struct MatchProblem<'suite> {
    question: FaceAndCard<'suite>,
    answers: Vec<(FaceAndCard<'suite>, bool)>,
    index_answer_correct: usize,
}

type FaceAndCard<'suite> = (&'suite String, &'suite Card);

fn get_match_problem_suite(decks: &[Deck]) -> Result<MatchProblemSuite, FlashrError> {
    if decks.is_empty() {
        return Err(FlashrError::DeckMismatchError("No decks provided".into()));
    }

    let rng = &mut rand::thread_rng();

    let mut problems = decks.iter().try_fold(
        Vec::with_capacity(decks.iter().fold(0, |total, deck| {
            total + (deck.cards.len() * deck.faces.len())
        })),
        |mut problems, deck| -> Result<_, FlashrError> {
            let deck_problems = get_match_problems_for_deck(deck, decks, rng)?;
            problems.extend(deck_problems);
            Ok(problems)
        },
    )?;

    problems.shuffle(rng);

    Ok(MatchProblemSuite { problems })
}

fn get_match_problems_for_deck<'decks>(
    deck: &'decks Deck,
    decks: &'decks [Deck],
    rng: &mut ThreadRng,
) -> Result<Vec<MatchProblem<'decks>>, FlashrError> {
    let deck_problems = deck.faces.iter().enumerate().try_fold(
        Vec::with_capacity(deck.faces.len() * deck.cards.len()),
        |mut problems,
         (problem_face_index_original, problem_face)|
         -> Result<Vec<_>, FlashrError> {
            let problems_for_face = get_match_problems_for_deck_face(
                deck,
                decks,
                problem_face_index_original,
                problem_face,
                rng,
            )?;
            problems.extend(problems_for_face);
            Ok(problems)
        },
    )?;
    Ok(deck_problems)
}

fn get_match_problems_for_deck_face<'decks>(
    deck: &'decks Deck,
    decks: &'decks [Deck],
    problem_face_index: usize,
    problem_face: &'decks String,
    rng: &mut ThreadRng,
) -> Result<Vec<MatchProblem<'decks>>, FlashrError> {
    let faces_possible = deck
        .faces
        .iter()
        .enumerate()
        .filter(|(answer_face_index, _answer_face)| *answer_face_index != problem_face_index);

    let answers_possible = faces_possible
        .map(|(answer_face_index, answer_face)| {
            let decks_with_face = decks.iter().filter_map(|deck| {
                deck.faces
                    .iter()
                    .enumerate()
                    .find(|(_deck_face_index, deck_face)| *deck_face == answer_face)
                    .map(|(deck_face_index, _deck_face)| (deck, deck_face_index))
            });

            let answers_for_face = decks_with_face
                .flat_map(|(deck, deck_face_index)| {
                    deck.cards
                        .iter()
                        .map(move |card| (&card[deck_face_index], card))
                })
                .collect::<Vec<_>>();

            (answer_face_index, answers_for_face)
        })
        .filter(|(_answer_face_index, cards)| cards.len() > 3)
        .collect::<Vec<_>>();

    if answers_possible.is_empty() {
        let deck_name = &deck.name;
        return Err(FlashrError::DeckMismatchError(
            format!("Unable to find enough possible answers for the \"{problem_face}\" face of the \"{deck_name}\" deck"),
        ));
    }

    let problems_for_face = deck.cards.iter().try_fold(
        Vec::with_capacity(deck.cards.len()),
        |mut problems, card| {
            let (answer_face_index, answer_cards) =
                &answers_possible[rng.gen_range(0..answers_possible.len())];

            //NOTE: Shuffling here as well so that the filter isn't deterministic
            //Otherwise, it would always filter out answers that appear later
            let mut answer_cards = answer_cards.clone();
            answer_cards.shuffle(rng);

            let mut seen = vec![&card[*answer_face_index]];
            let mut answers = answer_cards
                .into_iter()
                .filter(|(answer, _answer_card)| {
                    if seen.contains(answer) {
                        false
                    } else {
                        seen.push(answer);
                        true
                    }
                })
                .take(3)
                .map(|answer_and_card| (answer_and_card, false))
                .collect::<Vec<_>>();

            if answers.len() < 3 {
                return Err(FlashrError::DeckMismatchError(format!(
                    "Not enough answers for card face \"{}\", using answer face \"{}\"",
                    card[problem_face_index], deck.faces[*answer_face_index]
                )));
            }

            let correct_answer = &card[*answer_face_index];

            answers.push(((correct_answer, card), true));
            answers.shuffle(rng);

            let index_answer_correct = answers
                .iter()
                .enumerate()
                .find(|(_, (_, correct))| *correct)
                .map(|(i, _)| i)
                .unwrap();

            problems.push(MatchProblem {
                question: (&card[problem_face_index], card),
                answers,
                index_answer_correct,
            });

            Ok(problems)
        },
    )?;

    Ok(problems_for_face)
}

//NB 'suite lifetime technically not required, but I think it's more accurate
struct MatchProblemWidget<'decks, 'suite> {
    problem: &'suite MatchProblem<'decks>,
    progress: f64,
    answer: Option<(usize, bool)>,
}

struct MatchProblemWidgetState {
    answer_areas: Vec<Rect>,
}

impl Default for MatchProblemWidgetState {
    fn default() -> Self {
        Self {
            answer_areas: [Rect::default()].repeat(4),
        }
    }
}

impl StatefulWidget for MatchProblemWidget<'_, '_> {
    type State = MatchProblemWidgetState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        let layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(2, 3),
                Constraint::Min(1),
            ],
        )
        .split(area);

        let question_area = layout[0];
        let answer_area = layout[1];
        let progress_area = layout[2];

        let layout =
            Layout::new(Direction::Vertical, [Constraint::Ratio(1, 2); 2]).split(answer_area);
        let answer_top = layout[0];
        let answer_bot = layout[1];
        let layout = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 2); 2]);
        let answer_areas = [layout.split(answer_top), layout.split(answer_bot)].concat();

        let question = self.problem.question.0;

        //TODO: Need refactor to DRY. Had to debug silly issue already
        match self.answer {
            None => {
                Paragraph::new(question.clone())
                    .wrap(Wrap { trim: false })
                    .alignment(Alignment::Center)
                    .block(Block::bordered().border_type(BorderType::Double))
                    .render(question_area, buf);
                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, ((answer, _answer_card), _))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        Paragraph::new(format!("{}: {}", answer_index + 1, answer))
                            .wrap(Wrap { trim: false })
                            .alignment(Alignment::Center)
                            .block(Block::bordered().border_type(BorderType::Double))
                            .render(answer_area, buf)
                    },
                );
            }
            Some((answered_index, correct)) => {
                Paragraph::new(question.clone())
                    .wrap(Wrap { trim: false })
                    .alignment(Alignment::Center)
                    .block(Block::bordered().border_type(BorderType::Double))
                    .fg(if correct { Color::Green } else { Color::Red })
                    .render(question_area, buf);

                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, ((_, card_answer), is_correct))| {
                        let is_answered = answer_index == answered_index;
                        let answer_area = answer_areas[answer_index];

                        state.answer_areas[answer_index] = answer_area;

                        Paragraph::new(format!("{}: {}", answer_index + 1, card_answer.join("\n"),))
                            .wrap(Wrap { trim: false })
                            .alignment(Alignment::Center)
                            .block(Block::bordered().border_type(BorderType::Double))
                            .fg(if *is_correct {
                                Color::Green
                            } else if is_answered {
                                Color::Red
                            } else {
                                Color::Gray
                            })
                            .render(answer_area, buf)
                    },
                );
            }
        }

        Gauge::default()
            .percent((self.progress * 100.0) as u16)
            .render(progress_area, buf);
    }
}

fn show_match_problem(
    term: &mut TerminalWrapper,
    problem: MatchProblem,
    progress: f64,
) -> Result<ProblemResult, FlashrError> {
    let problem = &problem;
    let mut widget_state = MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(
            MatchProblemWidget {
                problem,
                progress,
                answer: None,
            },
            &mut widget_state,
        )?;

        match clear_and_match_event(|event| match_match_input(event, &widget_state))? {
            UserInput::Answer(answered) => {
                let correct = answered == problem.index_answer_correct;

                loop {
                    term.render_stateful_widget(
                        MatchProblemWidget {
                            problem,
                            progress,
                            answer: Some((answered, correct)),
                        },
                        &mut widget_state,
                    )?;

                    match clear_and_match_event(|event| match_match_input(event, &widget_state))? {
                        UserInput::Answer(answer) => {
                            if answer == problem.index_answer_correct {
                                break;
                            }
                        }
                        UserInput::Resize => continue,
                        UserInput::Quit => return Ok(ProblemResult::Quit),
                    };
                }

                return Ok(if correct {
                    ProblemResult::Correct
                } else {
                    ProblemResult::Incorrect
                });
            }
            UserInput::Resize => continue,
            UserInput::Quit => return Ok(ProblemResult::Quit),
        };
    }
}

fn match_match_input(event: Event, state: &MatchProblemWidgetState) -> Option<UserInput> {
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