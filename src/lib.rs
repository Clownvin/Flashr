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

///Test documentation
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
    let suite = get_tests(decks)?;

    let mut total_correct = 0;
    let mut total_completed = 0;

    let total_tests = suite.tests.len() as f64;

    let mut rng = rand::thread_rng();

    suite
        .tests
        .into_iter()
        .enumerate()
        .try_for_each(|(i, test)| {
            let correct = show_test(
                term,
                &mut rng,
                &suite.cards,
                test,
                suite.face_count,
                i as f64 / total_tests,
            )?;

            total_completed += 1;
            if correct {
                total_correct += 1;
            }

            Ok::<_, FlashrError>(())
        })?;

    Ok(())
}

fn show_test(
    term: &mut TerminalWrapper,
    rng: &mut ThreadRng,
    cards: &[Card],
    test: TestCase,
    total_faces: usize,
    progress: f64,
) -> Result<bool, FlashrError> {
    let question = cards[test.index][test.face].to_owned();
    let answer_face = get_other_test_face(test.face, total_faces, rng);
    let mut answer_indices = get_other_test_card_indices(test.index, cards.len(), 3, rng);
    answer_indices.extend(std::iter::once(test.index));
    answer_indices.shuffle(rng);

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
            Paragraph::new(question.clone())
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Center)
                .block(Block::bordered().border_type(BorderType::Double)),
            question_area,
        );

        answer_indices.iter().enumerate().for_each(|(i, index)| {
            frame.render_widget(
                Paragraph::new(format!(
                    "{}: {}",
                    i + 1,
                    cards[*index][answer_face].to_owned(),
                ))
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
    let correct = answer_indices[answered] == test.index;

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
            Paragraph::new(question)
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Center)
                .block(Block::bordered().border_type(BorderType::Double).fg(color)),
            question_area,
        );

        answer_indices.iter().enumerate().for_each(|(i, index)| {
            let is_answer = *index == test.index;
            let is_answered = i == answered;

            let color = if is_answer {
                Color::Green
            } else if is_answered {
                Color::Red
            } else {
                Color::Gray
            };

            frame.render_widget(
                Paragraph::new(format!("{}: {}", i + 1, cards[*index].join("\n"),))
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
        if answer_indices[answer] == test.index {
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

struct TestSuite {
    face_count: usize,
    cards: Vec<Card>,
    tests: Vec<TestCase>,
}

struct TestCase {
    index: usize,
    face: usize,
}

fn get_tests(decks: Vec<Deck>) -> Result<TestSuite, FlashrError> {
    if decks.is_empty() {
        return Err(FlashrError::DeckMismatchError("No decks provided".into()));
    }

    let expected_face_count = decks[0].faces.len();
    if let Some(deck) = decks
        .iter()
        .find(|deck| deck.faces.len() != expected_face_count)
    {
        let name = deck.name.to_string();
        let face_count = deck.faces.len();
        return Err(FlashrError::DeckMismatchError(format!("At least one deck, {name}, has an incorrect amount of cards. Expected {expected_face_count}, but has {face_count}")));
    }

    let total_cards = decks.iter().fold(0, |total, deck| total + deck.len());

    if total_cards < 4 {
        return Err(FlashrError::DeckMismatchError(
            "Requires at least 4 cards to run".into(),
        ));
    }

    let cards = decks
        .into_iter()
        .fold(Vec::with_capacity(total_cards), |mut cards, deck| {
            cards.extend(deck.cards);
            cards
        });

    let mut tests = (0..expected_face_count)
        .flat_map(|face| (0..total_cards).map(move |index| TestCase { index, face }))
        .collect::<Vec<_>>();

    tests.shuffle(&mut rand::thread_rng());

    Ok(TestSuite {
        face_count: expected_face_count,
        cards,
        tests,
    })
}

//TODO: Needs to hande "out of" case
//Could probably be smarter too
fn get_other_test_card_indices(
    this_index: usize,
    total_cards: usize,
    count: usize,
    rng: &mut ThreadRng,
) -> Vec<usize> {
    let mut seen = Vec::with_capacity(count + 1);
    seen.push(this_index);

    (0..count)
        .map(|_| loop {
            let index = rng.gen_range(0..total_cards);

            if seen.contains(&index) {
                continue;
            } else {
                seen.push(index);
                return index;
            }
        })
        .collect()
}

//TODO: Needs to hande "out of" case
//Could probably be smarter too
fn get_other_test_face(this_face: usize, total_faces: usize, rng: &mut ThreadRng) -> usize {
    loop {
        let face = rng.gen_range(0..total_faces);

        if face == this_face {
            continue;
        } else {
            return face;
        }
    }
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
