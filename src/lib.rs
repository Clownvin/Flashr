use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use deck::{load_decks, Card, Deck, DeckError};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Gauge, Paragraph, Wrap},
};
use terminal::TerminalWrapper;

mod deck;
mod terminal;

#[derive(Parser, Debug)]
#[command(name = "flashr")]
struct FlashrCli {
    paths: Vec<String>,
}

#[derive(Debug)]
pub enum FlashrError {
    DeckError(DeckError),
    UiError(UiError),
    DeckMismatchError(String),
}

impl From<DeckError> for FlashrError {
    fn from(err: DeckError) -> Self {
        FlashrError::DeckError(err)
    }
}

impl From<UiError> for FlashrError {
    fn from(err: UiError) -> Self {
        FlashrError::UiError(err)
    }
}

#[derive(Debug)]
pub enum UiError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for UiError {
    fn from(err: std::io::Error) -> Self {
        UiError::IoError(err)
    }
}

pub fn run() -> Result<(), FlashrError> {
    let cli = FlashrCli::parse();
    let mut term = initialize()?;
    let decks = load_decks(cli.paths)?;
    flash_cards(&mut term, decks)
}

fn initialize() -> Result<TerminalWrapper, FlashrError> {
    Ok(TerminalWrapper::new().map_err(UiError::IoError)?)
}

pub fn flash_cards(term: &mut TerminalWrapper, decks: Vec<Deck>) -> Result<(), FlashrError> {
    let suite = get_match_problem_suite(&decks)?;

    let mut total_correct = 0;
    let mut total_completed = 0;

    let total_problems = suite.problems.len() as f64;

    suite.problems.into_iter().enumerate().try_for_each(
        |(i, problem)| -> Result<_, FlashrError> {
            let correct = show_match_problem(term, problem, i as f64 / total_problems)?;

            total_completed += 1;
            if correct {
                total_correct += 1;
            }

            Ok(())
        },
    )?;

    Ok(())
}

fn show_match_problem(
    term: &mut TerminalWrapper,
    problem: MatchProblem,
    progress: f64,
) -> Result<bool, FlashrError> {
    let (question, _question_card) = problem.problem;

    term.draw(|frame| {
        let layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(2, 3),
                Constraint::Min(1),
            ],
        )
        .split(frame.size());

        let question_area = layout[0];
        let answer_area = layout[1];
        let progress_area = layout[2];

        let layout =
            Layout::new(Direction::Vertical, [Constraint::Ratio(1, 2); 2]).split(answer_area);
        let answer_top = layout[0];
        let answer_bot = layout[1];
        let layout = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 2); 2]);
        let answer_areas = [layout.split(answer_top), layout.split(answer_bot)].concat();

        frame.render_widget(
            Paragraph::new(question.to_owned())
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Center)
                .block(Block::bordered().border_type(BorderType::Double)),
            question_area,
        );

        problem
            .answers
            .iter()
            .enumerate()
            .for_each(|(i, (answer, _answer_card))| {
                frame.render_widget(
                    Paragraph::new(format!("{}: {}", i + 1, answer))
                        .wrap(Wrap { trim: false })
                        .alignment(Alignment::Center)
                        .block(Block::bordered().border_type(BorderType::Double)),
                    answer_areas[i],
                )
            });

        frame.render_widget(
            Gauge::default().percent((progress * 100.0) as u16),
            progress_area,
        );
    })
    .map_err(UiError::IoError)?;

    let answered = get_answer()?;
    let correct = answered == problem.correct_answer_index;

    term.draw(|frame| {
        let layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(2, 3),
                Constraint::Min(1),
            ],
        )
        .split(frame.size());

        let question_area = layout[0];
        let answer_area = layout[1];
        let progress_area = layout[2];

        let layout =
            Layout::new(Direction::Vertical, [Constraint::Ratio(1, 2); 2]).split(answer_area);
        let answer_top = layout[0];
        let answer_bot = layout[1];
        let layout = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 2); 2]);
        let answer_areas = [layout.split(answer_top), layout.split(answer_bot)].concat();

        let color = if correct { Color::Green } else { Color::Red };

        frame.render_widget(
            Paragraph::new(question.to_owned())
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Center)
                .block(Block::bordered().border_type(BorderType::Double).fg(color)),
            question_area,
        );

        problem
            .answers
            .iter()
            .enumerate()
            .for_each(|(i, (_answer, answer_card))| {
                let is_answer = i == problem.correct_answer_index;
                let is_answered = i == answered;

                let color = if is_answer {
                    Color::Green
                } else if is_answered {
                    Color::Red
                } else {
                    Color::Gray
                };

                frame.render_widget(
                    Paragraph::new(format!("{}: {}", i + 1, answer_card.join("\n"),))
                        .wrap(Wrap { trim: false })
                        .alignment(Alignment::Center)
                        .block(Block::bordered().border_type(BorderType::Double))
                        .fg(color),
                    answer_areas[i],
                )
            });

        frame.render_widget(
            Gauge::default().percent((progress * 100.0) as u16),
            progress_area,
        );
    })
    .map_err(UiError::IoError)?;

    // thread::sleep(Duration::from_secs(if correct { 0 } else { 5 }));

    loop {
        let answer = get_answer()?;
        if answer == problem.correct_answer_index {
            break;
        }
    }

    Ok(correct)
}

fn get_answer() -> Result<usize, FlashrError> {
    //Clear the loop
    loop {
        if event::poll(Duration::from_millis(0)).map_err(UiError::IoError)? {
            event::read().map_err(UiError::IoError)?;
            continue;
        }
        break;
    }
    //Get answer
    loop {
        if event::poll(Duration::from_secs(1)).map_err(UiError::IoError)? {
            if let Event::Key(key) = event::read().map_err(UiError::IoError)? {
                if key.kind == event::KeyEventKind::Press {
                    let answer = match key.code {
                        KeyCode::Char('1') => Some(0),
                        KeyCode::Char('2') => Some(1),
                        KeyCode::Char('3') => Some(2),
                        KeyCode::Char('4') => Some(3),
                        _ => None,
                    };

                    if let Some(answer) = answer {
                        return Ok(answer);
                    }
                }
            }
        }
    }
}

struct MatchProblemSuite<'suite> {
    problems: Vec<MatchProblem<'suite>>,
}

type FaceAndCard<'suite> = (&'suite String, &'suite Card);

struct MatchProblem<'suite> {
    problem: FaceAndCard<'suite>,
    answers: Vec<FaceAndCard<'suite>>,
    correct_answer_index: usize,
}

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
    let answers_possible = deck
        .faces
        .iter()
        .enumerate()
        .filter(|(answer_face_index, _answer_face)| *answer_face_index != problem_face_index)
        .map(|(answer_face_index, answer_face)| {
            (
                answer_face_index,
                decks
                    .iter()
                    .filter_map(|deck| {
                        deck.faces
                            .iter()
                            .enumerate()
                            .find(|(_deck_face_index, deck_face)| *deck_face == answer_face)
                            .map(|(deck_face_index, _deck_face)| (deck, deck_face_index))
                    })
                    .flat_map(|(deck, deck_face_index)| {
                        deck.cards
                            .iter()
                            .map(move |card| (&card[deck_face_index], card))
                    })
                    .collect::<Vec<_>>(),
            )
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
                .collect::<Vec<_>>();

            if answers.len() < 3 {
                return Err(FlashrError::DeckMismatchError(format!(
                    "Not enough answers for card face \"{}\", using answer face \"{}\"",
                    card[problem_face_index], deck.faces[*answer_face_index]
                )));
            }

            let correct_answer = &card[*answer_face_index];

            answers.push((correct_answer, card));
            answers.shuffle(rng);

            let correct_answer_index = answers
                .iter()
                .enumerate()
                .find(|(_i, (answer, _answer_card))| *answer == correct_answer)
                .map(|(i, _)| i)
                .unwrap();

            problems.push(MatchProblem {
                problem: (&card[problem_face_index], card),
                answers,
                correct_answer_index,
            });

            Ok(problems)
        },
    )?;

    Ok(problems_for_face)
}

#[cfg(test)]
mod tests {
    use crate::FlashrCli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        FlashrCli::command().debug_assert();
    }
}
