use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use deck::{load_decks, Card, Deck, DeckError};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Gauge, Paragraph, Widget, Wrap},
};
use terminal::TerminalWrapper;

pub mod deck;
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

pub fn run() -> Result<(usize, usize), FlashrError> {
    let cli = FlashrCli::parse();
    let mut term = initialize_terminal()?;
    let decks = load_decks(cli.paths)?;
    match_cards(&mut term, decks)
}

fn initialize_terminal() -> Result<TerminalWrapper, FlashrError> {
    Ok(TerminalWrapper::new().map_err(UiError::IoError)?)
}

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

enum ProblemResult {
    Correct,
    Incorrect,
    Quit,
}

//NB 'suite lifetime technically not required, but I think it's more accurate
struct MatchProblemWidget<'decks, 'suite> {
    question: &'decks String,
    problem: &'suite MatchProblem<'decks>,
    progress: f64,
    answer: Option<(usize, bool)>,
}

impl Widget for MatchProblemWidget<'_, '_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
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

        match self.answer {
            None => {
                Paragraph::new(self.question.clone())
                    .wrap(Wrap { trim: false })
                    .alignment(Alignment::Center)
                    .block(Block::bordered().border_type(BorderType::Double))
                    .render(question_area, buf);
                self.problem
                    .answers
                    .iter()
                    .enumerate()
                    .for_each(|(i, (answer, _answer_card))| {
                        Paragraph::new(format!("{}: {}", i + 1, answer))
                            .wrap(Wrap { trim: false })
                            .alignment(Alignment::Center)
                            .block(Block::bordered().border_type(BorderType::Double))
                            .render(answer_areas[i], buf)
                    });
            }
            Some((answered, correct)) => {
                Paragraph::new(self.question.clone())
                    .wrap(Wrap { trim: false })
                    .alignment(Alignment::Center)
                    .block(Block::bordered().border_type(BorderType::Double))
                    .fg(if correct { Color::Green } else { Color::Red })
                    .render(question_area, buf);
                self.problem
                    .answers
                    .iter()
                    .enumerate()
                    .for_each(|(i, (_answer, answer_card))| {
                        let is_answer = i == self.problem.correct_answer_index;
                        let is_answered = i == answered;

                        let color = if is_answer {
                            Color::Green
                        } else if is_answered {
                            Color::Red
                        } else {
                            Color::Gray
                        };
                        Paragraph::new(format!("{}: {}", i + 1, answer_card.join("\n"),))
                            .wrap(Wrap { trim: false })
                            .alignment(Alignment::Center)
                            .block(Block::bordered().border_type(BorderType::Double))
                            .fg(color)
                            .render(answer_areas[i], buf)
                    });
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
    let (question, _question_card) = problem.problem;

    loop {
        term.draw(|frame| {
            frame.render_widget(
                MatchProblemWidget {
                    problem: &problem,
                    question,
                    progress,
                    answer: None,
                },
                frame.size(),
            )
        })
        .map_err(UiError::IoError)?;

        match get_user_input()? {
            UserInput::Answer(answered) => {
                let correct = answered == problem.correct_answer_index;

                loop {
                    term.draw(|frame| {
                        frame.render_widget(
                            MatchProblemWidget {
                                problem: &problem,
                                question,
                                progress,
                                answer: Some((answered, correct)),
                            },
                            frame.size(),
                        )
                    })
                    .map_err(UiError::IoError)?;

                    let answer = get_user_input()?;
                    if let UserInput::Answer(answer) = answer {
                        if answer == problem.correct_answer_index {
                            break;
                        }
                    }
                    if matches!(answer, UserInput::Quit) {
                        return Ok(ProblemResult::Quit);
                    }
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

enum UserInput {
    Answer(usize),
    Resize,
    Quit,
}

fn get_user_input() -> Result<UserInput, FlashrError> {
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
            let event = event::read().map_err(UiError::IoError)?;
            match event {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        let input = match key.code {
                            KeyCode::Char('1') => Some(UserInput::Answer(0)),
                            KeyCode::Char('2') => Some(UserInput::Answer(1)),
                            KeyCode::Char('3') => Some(UserInput::Answer(2)),
                            KeyCode::Char('4') => Some(UserInput::Answer(3)),
                            KeyCode::Esc | KeyCode::Char('q') => Some(UserInput::Quit),
                            _ => None,
                        };

                        if let Some(answer) = input {
                            return Ok(answer);
                        }
                    }
                }
                Event::Resize(_, _) => return Ok(UserInput::Resize),
                _ => {}
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
