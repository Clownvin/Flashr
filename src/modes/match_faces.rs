use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use rand::{prelude::SliceRandom, rngs::ThreadRng, Rng};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    symbols::{border, line},
    widgets::{Block, Borders, Gauge, Paragraph, StatefulWidget, Widget, Wrap},
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
    let rng = &mut rand::thread_rng();
    let suite = get_match_problem_suite(rng, &decks)?;

    let total_problems = suite.problems.remaining();
    let mut total_correct = 0;

    for (i, ref problem) in suite.problems.enumerate() {
        let result = show_match_problem(term, problem, i as f64 / total_problems as f64)?;

        match result {
            ProblemResult::Correct => total_correct += 1,
            ProblemResult::Quit => return Ok((total_correct, i)),
            ProblemResult::Incorrect => {}
        }
    }

    Ok((total_correct, total_problems))
}

struct MatchProblemSuite<'rng, 'decks> {
    problems: ShuffleIter<'rng, MatchProblem<'decks>>,
}

struct MatchProblem<'suite> {
    question: FaceAndCard<'suite>,
    answers: Vec<(FaceAndCard<'suite>, bool)>,
    index_answer_correct: usize,
}

type FaceAndCard<'suite> = (&'suite String, &'suite Card);

trait IterShuffled<'rng>
where
    Self: IntoIterator,
{
    fn iter_shuffled(self, rng: &'rng mut ThreadRng) -> ShuffleIter<'rng, Self::Item>;
}

struct ShuffleIter<'rng, T> {
    values: Vec<T>,
    rng: &'rng mut ThreadRng,
}

impl<T> Iterator for ShuffleIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.values.len() {
            0 => None,
            1 => Some(self.values.swap_remove(0)),
            r => Some(self.values.swap_remove(self.rng.gen_range(0..r))),
        }
    }
}

impl<'rng, T> ShuffleIter<'rng, T> {
    fn remaining(&self) -> usize {
        self.values.len()
    }
}

impl<'rng, T> IterShuffled<'rng> for Vec<T> {
    fn iter_shuffled(self, rng: &'rng mut ThreadRng) -> ShuffleIter<'rng, Self::Item> {
        ShuffleIter { values: self, rng }
    }
}

trait GetRandom {
    type Item;

    fn get_random(&self, rng: &mut ThreadRng) -> &'_ Self::Item;
}

impl<T> GetRandom for Vec<T> {
    type Item = T;

    fn get_random(&self, rng: &mut ThreadRng) -> &'_ Self::Item {
        &self[rng.gen_range(0..self.len())]
    }
}

fn get_match_problem_suite<'rng, 'decks>(
    rng: &'rng mut ThreadRng,
    decks: &'decks [Deck],
) -> Result<MatchProblemSuite<'rng, 'decks>, FlashrError> {
    if decks.is_empty() {
        return Err(FlashrError::DeckMismatchError("No decks provided".into()));
    }

    let problem_count = decks
        .iter()
        .map(|deck| deck.cards.len() * deck.faces.len())
        .sum();

    let problems = decks
        .iter()
        .try_fold(
            Vec::with_capacity(problem_count),
            |mut problems, deck| -> Result<_, FlashrError> {
                let deck_problems = get_match_problems_for_deck(deck, decks, rng)?;
                problems.extend(deck_problems);
                Ok(problems)
            },
        )?
        .iter_shuffled(rng);

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

    let answers_faces_possible = faces_possible
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

    if answers_faces_possible.is_empty() {
        let deck_name = &deck.name;
        return Err(FlashrError::DeckMismatchError(
            format!("Unable to find enough possible answers for the \"{problem_face}\" face of the \"{deck_name}\" deck"),
        ));
    }

    //NB: Converting to refs to ideally make the Vec::clone faster
    //TODO: Needs test/benchmark to prove faster.
    let answers_faces_possible = answers_faces_possible
        .iter()
        .map(|(answer_face_index, cards)| (*answer_face_index, cards.iter().collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let problems_for_face = deck.cards.iter().try_fold(
        Vec::with_capacity(deck.cards.len()),
        |mut problems, problem_card| {
            let (answer_face_index, answer_cards) = answers_faces_possible.get_random(rng);

            let mut seen = vec![&problem_card[*answer_face_index]];
            let mut answers = answer_cards
                .clone()
                //NOTE: Shuffling here as well so that the filter isn't deterministic
                //Otherwise, it would always filter out answers that appear later
                .iter_shuffled(rng)
                .filter(|(answer, _answer_card)| {
                    if seen.contains(answer)
                        //TODO: Needs test case to prove works
                        //NB: Two+ decks with different face count can error, still no proof that
                        //this solves the issue
                        // || answer_card[problem_face_index] == problem_card[problem_face_index]
                    {
                        false
                    } else {
                        seen.push(answer);
                        true
                    }
                })
                .take(3)
                .map(|answer_and_card| (*answer_and_card, false))
                .chain(std::iter::once((
                    (&problem_card[*answer_face_index], problem_card),
                    true,
                )))
                .collect::<Vec<_>>();

            if answers.len() < 4 {
                let card_face_problem = &problem_card[problem_face_index];
                let deck_face_answer = &deck.faces[*answer_face_index];

                return Err(FlashrError::DeckMismatchError(format!(
                    "Not enough answers for card face \"{card_face_problem}\", using answer face \"{deck_face_answer}\""
                )));
            }

            answers.shuffle(rng);

            let index_answer_correct = answers
                .iter()
                .enumerate()
                .find(|(_, (_, correct))| *correct)
                .map(|(i, _)| i)
                .unwrap();

            problems.push(MatchProblem {
                question: (&problem_card[problem_face_index], problem_card),
                answers,
                index_answer_correct,
            });

            Ok(problems)
        },
    )?;

    Ok(problems_for_face)
}

//NB 'suite lifetime technically not required, but I think it's more accurate
struct MatchProblemWidget<'suite> {
    problem: &'suite MatchProblem<'suite>,
    progress: f64,
    answer: Option<(usize, bool)>,
}

impl<'suite> MatchProblemWidget<'suite> {
    fn new(problem: &'suite MatchProblem<'suite>, progress: f64) -> Self {
        Self {
            problem,
            progress,
            answer: None,
        }
    }

    fn answered(mut self, answer: (usize, bool)) -> Self {
        self.answer = Some(answer);
        self
    }
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

impl StatefulWidget for MatchProblemWidget<'_> {
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

        match self.answer {
            None => {
                Paragraph::new(question.clone())
                    .wrap(Wrap { trim: false })
                    .centered()
                    .render(question_area, buf);

                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, ((answer, _answer_card), _))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        MatchAnswerWidget::new(answer.to_string(), answer_index)
                            .render(answer_area, buf)
                    },
                );
            }
            Some((answered_index, correct)) => {
                Paragraph::new(question.clone())
                    .wrap(Wrap { trim: false })
                    .centered()
                    .fg(if correct { Color::Green } else { Color::Red })
                    .render(question_area, buf);

                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, ((_, card_answer), is_correct))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        let is_answered = answer_index == answered_index;
                        MatchAnswerWidget::new(card_answer.join("\n"), answer_index)
                            .answered((*is_correct, is_answered))
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

struct MatchAnswerWidget {
    answer: String,
    answer_index: usize,
    outcome: Option<(bool, bool)>,
}

impl MatchAnswerWidget {
    fn new(answer: String, answer_index: usize) -> Self {
        Self {
            answer,
            answer_index,
            outcome: None,
        }
    }

    fn answered(mut self, outcome: (bool, bool)) -> Self {
        self.outcome = Some(outcome);
        self
    }
}

impl Widget for MatchAnswerWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let top_row = self.answer_index < 2;
        let left_side = self.answer_index % 2 == 0;

        Paragraph::new(format!("{}: {}", self.answer_index + 1, self.answer))
            .wrap(Wrap { trim: false })
            .centered()
            .block(
                Block::bordered()
                    .borders({
                        let mut borders = Borders::TOP;
                        if left_side {
                            borders |= Borders::RIGHT;
                        }
                        borders
                    })
                    .border_set(border::Set {
                        bottom_right: if !left_side {
                            line::DOUBLE_BOTTOM_RIGHT
                        } else {
                            line::DOUBLE_HORIZONTAL_UP
                        },
                        bottom_left: line::DOUBLE_BOTTOM_LEFT,
                        top_left: line::DOUBLE_VERTICAL_RIGHT,
                        top_right: if top_row && left_side {
                            line::DOUBLE_HORIZONTAL_DOWN
                        } else if !left_side {
                            line::DOUBLE_VERTICAL_LEFT
                        } else {
                            line::DOUBLE_CROSS
                        },
                        vertical_left: line::DOUBLE_VERTICAL,
                        vertical_right: line::DOUBLE_VERTICAL,
                        horizontal_top: line::DOUBLE_HORIZONTAL,
                        horizontal_bottom: line::DOUBLE_HORIZONTAL,
                    }),
            )
            .fg(match self.outcome {
                None | Some((false, false)) => Color::default(),
                Some((is_correct, _)) => {
                    if is_correct {
                        Color::Green
                    } else {
                        Color::Red
                    }
                }
            })
            .render(area, buf)
    }
}

fn show_match_problem(
    term: &mut TerminalWrapper,
    problem: &MatchProblem,
    progress: f64,
) -> Result<ProblemResult, FlashrError> {
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(MatchProblemWidget::new(problem, progress), widget_state)?;

        match clear_and_match_event(|event| match_match_input(event, widget_state))? {
            UserInput::Answer(index_answered) => {
                return show_match_problem_result(term, problem, progress, index_answered)
            }
            UserInput::Resize => continue,
            UserInput::Quit => return Ok(ProblemResult::Quit),
        }
    }
}

fn show_match_problem_result(
    term: &mut TerminalWrapper,
    problem: &MatchProblem,
    progress: f64,
    index_answered: usize,
) -> Result<ProblemResult, FlashrError> {
    let correct = index_answered == problem.index_answer_correct;
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(
            MatchProblemWidget::new(problem, progress).answered((index_answered, correct)),
            widget_state,
        )?;

        match clear_and_match_event(|event| match_match_input(event, widget_state))? {
            UserInput::Answer(answer) if answer == problem.index_answer_correct => {
                return Ok(if correct {
                    ProblemResult::Correct
                } else {
                    ProblemResult::Incorrect
                })
            }
            UserInput::Answer(_) | UserInput::Resize => continue,
            UserInput::Quit => return Ok(ProblemResult::Quit),
        }
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
