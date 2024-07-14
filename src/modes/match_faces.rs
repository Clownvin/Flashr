use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use rand::{rngs::ThreadRng, seq::SliceRandom};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    symbols::{border, line},
    widgets::{Block, Borders, Gauge, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    deck::{Card, Deck},
    event::{clear_and_match_event, UserInput},
    random::{GetRandom, IterShuffled},
    terminal::TerminalWrapper,
    FlashrError, ModeArguments, ProblemResult,
};

const ANSWERS_PER_PROBLEM: usize = 4;

pub fn match_cards(
    term: &mut TerminalWrapper,
    args: ModeArguments,
) -> Result<(usize, usize), FlashrError> {
    let rng = &mut rand::thread_rng();
    let problems = MatchProblemIterator::new(&args.decks, args.faces, rng);

    let mut total_correct = 0;

    if let Some(count) = args.problem_count {
        for problem in problems.take(count) {
            let result = show_match_problem(term, &problem?, (total_correct, count))?;

            match result {
                ProblemResult::Correct => total_correct += 1,
                ProblemResult::Quit => return Ok((total_correct, count)),
                ProblemResult::Incorrect => {}
            }
        }

        Ok((total_correct, count))
    } else {
        let mut total = 0;

        for (i, problem) in problems.enumerate() {
            let result = show_match_problem(term, &problem?, (total_correct, i))?;

            total += 1;
            match result {
                ProblemResult::Correct => total_correct += 1,
                ProblemResult::Quit => return Ok((total_correct, total)),
                ProblemResult::Incorrect => {}
            }
        }

        Ok((total_correct, total))
    }
}

struct MatchProblemIterator<'rng, 'decks> {
    rng: &'rng mut ThreadRng,
    deck_cards: Vec<(&'decks Deck, &'decks Card)>,
    faces: Option<Vec<String>>,
}

impl<'rng, 'decks> MatchProblemIterator<'rng, 'decks> {
    fn new(decks: &'decks [Deck], faces: Option<Vec<String>>, rng: &'rng mut ThreadRng) -> Self {
        let mut deck_cards = Vec::with_capacity(decks.iter().fold(0, |total, deck| {
            total + (deck.cards.len() * deck.faces.len())
        }));

        if let Some(faces) = faces.as_ref() {
            for deck in decks {
                let mut deck_faces = Vec::with_capacity(deck.faces.len());
                deck.faces
                    .iter()
                    .enumerate()
                    .filter(|(_, deck_face)| faces.iter().any(|face| face == *deck_face))
                    .for_each(|(i, _)| deck_faces.push(i));

                if deck_faces.is_empty() {
                    continue;
                } else {
                    for card in deck.cards.iter() {
                        if deck_faces.iter().any(|i| card[*i].is_some()) {
                            deck_cards.push((deck, card));
                        } else {
                            // Don't push, no matching faces
                        }
                    }
                }
            }
        } else {
            for deck in decks {
                for card in deck.cards.iter() {
                    deck_cards.push((deck, card));
                }
            }
        }

        Self {
            rng,
            deck_cards,
            faces,
        }
    }
}

impl<'rng, 'decks> Iterator for MatchProblemIterator<'rng, 'decks> {
    type Item = Result<MatchProblem<'decks>, FlashrError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (deck, card) = self.deck_cards.get_random(self.rng)?;
        let possible_faces = deck
            .faces
            .iter()
            .enumerate()
            .filter(|(i, _)| card[*i].is_some())
            .collect::<Vec<_>>();

        let ((question_index, question_face), (answer_index, answer_face)) =
            match self.faces.as_ref() {
                Some(faces) => {
                    let mut buffer = Vec::with_capacity(possible_faces.len());
                    possible_faces
                        .iter()
                        .filter(|(_, face)| faces.iter().any(|specified| face == &specified))
                        .for_each(|face| buffer.push(face));

                    let question = buffer.get_random(self.rng).unwrap();
                    let (question_index, _) = question;

                    let mut buffer = Vec::with_capacity(possible_faces.len() - 1);
                    possible_faces
                        .iter()
                        .filter(|(i, _)| i != question_index)
                        .for_each(|face| buffer.push(face));

                    let answer = buffer.get_random(self.rng).unwrap();

                    (**question, **answer)
                }
                None => {
                    //TODO: Benchmark against just looping get_random to see which is faster.
                    //Cloning the faces could be more expensive
                    let faces = possible_faces
                        .iter_shuffled(self.rng)
                        .take(2)
                        .collect::<Vec<_>>();
                    (faces[0], faces[1])
                }
            };

        let problem_question = card[question_index].clone().unwrap();
        let problem_answer = card[answer_index].clone().unwrap();

        let mut seen_faces = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        seen_faces.push(&problem_answer);

        let mut answer_cards = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        answer_cards.push(((problem_answer.clone(), *card), true));

        self.deck_cards
            .clone()
            .iter_shuffled(self.rng)
            .filter_map(|(deck, card)| {
                deck.faces.iter().enumerate().find_map(|(i, face)| {
                    if face != answer_face {
                        None
                    } else {
                        card[i].as_ref().and_then(|face| {
                            if seen_faces.contains(&face) {
                                None
                            } else {
                                seen_faces.push(face);
                                Some(((face.clone(), card), false))
                            }
                        })
                    }
                })
            })
            .take(ANSWERS_PER_PROBLEM - 1)
            .for_each(|answer_card| answer_cards.push(answer_card));

        if answer_cards.len() < ANSWERS_PER_PROBLEM {
            let deck = &deck.name;
            return Some(Err(FlashrError::DeckMismatch(format!("Cannot find enough answers for question {problem_question}, which is a \"{question_face}\" face, from deck {deck}, given answer face \"{answer_face}\""))));
        }

        answer_cards.shuffle(self.rng);

        let answer_index = answer_cards
            .iter()
            .enumerate()
            .find(|(_, (_, correct))| *correct)
            .map(|(i, _)| i)
            .unwrap();

        Some(Ok(MatchProblem {
            question: (problem_question.join_random(", ", self.rng), card),
            answers: answer_cards
                .into_iter()
                .map(|((face, card), correct)| ((face.join_random(", ", self.rng), card), correct))
                .collect(),
            answer_index,
        }))
    }
}

struct MatchProblem<'suite> {
    question: FaceAndCard<'suite>,
    answers: Vec<(FaceAndCard<'suite>, bool)>,
    answer_index: usize,
}

type FaceAndCard<'suite> = (String, &'suite Card);

//NB 'suite lifetime technically not required, but I think it's more accurate
struct MatchProblemWidget<'suite> {
    problem: &'suite MatchProblem<'suite>,
    progress: (usize, usize),
    answer: Option<(usize, bool)>,
}

impl<'suite> MatchProblemWidget<'suite> {
    fn new(problem: &'suite MatchProblem<'suite>, progress: (usize, usize)) -> Self {
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
            answer_areas: [Rect::default()].repeat(ANSWERS_PER_PROBLEM),
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

        let question = &self.problem.question.0;

        match self.answer {
            None => {
                Paragraph::new(question.to_owned())
                    .wrap(Wrap { trim: false })
                    .centered()
                    .render(question_area, buf);

                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, ((answer, _answer_card), _))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        MatchAnswerWidget::new(answer.to_owned(), answer_index)
                            .render(answer_area, buf)
                    },
                );
            }
            Some((answered_index, correct)) => {
                Paragraph::new(question.to_owned())
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

        let (completed, total) = self.progress;
        let ratio = if total == 0 {
            0.0
        } else {
            completed as f64 / total as f64
        };
        let percent = ratio * 100.0;

        Gauge::default()
            .ratio(ratio)
            .label(format!("{percent:05.2}% ({completed}/{total})"))
            .use_unicode(true)
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
    progress: (usize, usize),
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
    progress: (usize, usize),
    index_answered: usize,
) -> Result<ProblemResult, FlashrError> {
    let correct = index_answered == problem.answer_index;
    let widget_state = &mut MatchProblemWidgetState::default();

    loop {
        term.render_stateful_widget(
            MatchProblemWidget::new(problem, progress).answered((index_answered, correct)),
            widget_state,
        )?;

        match clear_and_match_event(|event| match_match_input(event, widget_state))? {
            UserInput::Answer(answer) if answer == problem.answer_index => {
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

#[cfg(test)]
mod test {
    use crate::deck::load_decks;

    use super::MatchProblemIterator;

    #[test]
    fn ensure_unique_question_answers() {
        let decks = load_decks(vec!["./tests/deck1.json"]).unwrap();
        let rng = &mut rand::thread_rng();
        let problems = MatchProblemIterator::new(&decks, None, rng);

        for problem in problems.take(100) {
            let problem = problem.unwrap();
            assert!(problem
                .answers
                .iter()
                //Assert that each problem question is not present in the answers
                .all(|((answer, _), _)| answer != &problem.question.0));
            assert!(problem
                .answers
                .iter()
                .enumerate()
                .all(|(ref i, ((answer, _), _))| problem
                    .answers
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| i != j)
                    .all(|(_, ((other_answer, _), _))| other_answer != answer)))
        }
    }

    #[test]
    fn fails_if_not_enough_unique_answers() {
        let decks = load_decks(vec!["./tests/duplicate_cards"]).unwrap();
        let rng = &mut rand::thread_rng();
        let mut problems = MatchProblemIterator::new(&decks, None, rng);

        assert!(problems
            .next()
            .is_some_and(|problem| problem
                .is_err_and(|err| matches!(err, crate::FlashrError::DeckMismatch(_)))));
    }
}
