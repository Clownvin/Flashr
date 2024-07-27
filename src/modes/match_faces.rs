use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use rand::{rngs::ThreadRng, seq::SliceRandom};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    symbols::{border, line},
    widgets::{Block, Borders, Gauge, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    deck::Deck,
    event::{clear_and_match_event, UserInput},
    random::{GetRandom, IntoIterShuffled, WeightedList},
    stats::Stats,
    terminal::TerminalWrapper,
    CorrectIncorrect, DeckCard, FaceCardIndex, FlashrError, ModeArguments, ModeResult, OptionTuple,
    ProblemResult,
};

pub fn match_faces(
    mut term: TerminalWrapper,
    args: ModeArguments,
) -> Result<ModeResult, FlashrError> {
    let term = &mut term;
    let rng = &mut rand::thread_rng();
    let mut stats = args.stats;
    let mut problems = MatchProblemIterator::new(args.deck_cards, &mut stats, args.faces, rng);

    let mut total_correct = 0;

    if let Some(count) = args.problem_count {
        for _ in 0..count {
            if let Some(problem) = problems.next() {
                let problem = &problem?;
                let result = show_match_problem(term, problem, (total_correct, count))?;

                if result.is_quit() {
                    break;
                } else {
                    let stats = stats.for_card_mut((problem.deck, problem.question.1));

                    if result.is_correct() {
                        stats.correct += 1;
                        total_correct += 1;
                    } else {
                        stats.incorrect += 1;
                    }

                    problems.change_weight(problem.question.2, stats.weight());
                }
            } else {
                break;
            }
        }

        Ok(((total_correct, count), stats))
    } else {
        let mut total = 0;

        for i in 0.. {
            if let Some(problem) = problems.next() {
                let problem = &problem?;
                let result = show_match_problem(term, problem, (total_correct, i))?;

                total += 1;

                if result.is_quit() {
                    break;
                } else {
                    let stats = stats.for_card_mut((problem.deck, problem.question.1));

                    if result.is_correct() {
                        stats.correct += 1;
                        total_correct += 1;
                    } else {
                        stats.incorrect += 1;
                    }

                    problems.change_weight(problem.question.2, stats.weight());
                }
            } else {
                break;
            }
        }

        Ok(((total_correct, total), stats))
    }
}

struct MatchProblemIterator<'a> {
    rng: &'a mut ThreadRng,
    weighted_deck_cards: WeightedList<DeckCard<'a>>,
    faces: Option<Vec<String>>,
}

impl<'a> MatchProblemIterator<'a> {
    fn new(
        deck_cards: Vec<DeckCard<'a>>,
        stats: &mut Stats,
        faces: Option<Vec<String>>,
        rng: &'a mut ThreadRng,
    ) -> Self {
        let cards = deck_cards
            .into_iter()
            .map(|deck_card| (deck_card, stats.for_card(deck_card).weight()))
            .collect();
        Self {
            rng,
            weighted_deck_cards: cards,
            faces,
        }
    }

    fn change_weight(&mut self, index: usize, weight: f64) {
        self.weighted_deck_cards.change_weight(index, weight)
    }
}

impl<'a> Iterator for MatchProblemIterator<'a> {
    type Item = Result<MatchProblem<'a>, FlashrError>;

    fn next(&mut self) -> Option<Self::Item> {
        let ((deck, card), i) = self.weighted_deck_cards.get_random(self.rng)?;

        let mut possible_faces = Vec::with_capacity(deck.faces.len());
        deck.faces
            .iter()
            .enumerate()
            .filter(|(i, _)| card[*i].is_some())
            .for_each(|face| possible_faces.push(face));

        let ((question_index, question_face), (answer_index, answer_face)) =
            match self.faces.as_ref() {
                Some(faces) => {
                    let question = possible_faces
                        .clone()
                        .into_iter_shuffled(self.rng)
                        .find(|(_, face)| faces.iter().any(|specified| face == &specified))
                        .expect("Unable to find a valid question face");
                    let (question_index, _) = question;

                    let answer = possible_faces
                        .into_iter_shuffled(self.rng)
                        .find(|(i, _)| *i != question_index)
                        .expect("Unable to find a valid answer face");

                    (question, answer)
                }
                None => possible_faces
                    .into_iter_shuffled(self.rng)
                    .collect::<OptionTuple<_>>()
                    .expect("Unable to find valid question and answer faces"),
            };

        let face_question = card[question_index]
            .as_ref()
            .expect("Unable to find question face on card");
        let face_answer = card[answer_index]
            .as_ref()
            .expect("Unable to find answer face on card");

        let mut seen_faces = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        seen_faces.push(face_answer);

        let mut answer_cards = Vec::with_capacity(ANSWERS_PER_PROBLEM);

        answer_cards.push(((face_answer, *card, i), true));

        self.weighted_deck_cards
            .clone()
            .into_iter_shuffled(self.rng)
            .filter_map(|(((deck, card), _), card_index)| {
                deck.faces.iter().enumerate().find_map(|(i, face)| {
                    if face != answer_face {
                        None
                    } else {
                        card[i].as_ref().and_then(|face| {
                            if seen_faces.contains(&face) {
                                None
                            } else {
                                seen_faces.push(face);
                                Some(((face, card, card_index), false))
                            }
                        })
                    }
                })
            })
            .take(ANSWERS_PER_PROBLEM - 1)
            .for_each(|answer_card| answer_cards.push(answer_card));

        if answer_cards.len() < ANSWERS_PER_PROBLEM {
            let deck = &deck.name;
            return Some(Err(FlashrError::DeckMismatch(format!("Cannot find enough answers for question {face_question}, which is a \"{question_face}\" face, from deck {deck}, given answer face \"{answer_face}\""))));
        }

        answer_cards.shuffle(self.rng);

        let answer_index = answer_cards
            .iter()
            .enumerate()
            .find(|(_, (_, correct))| *correct)
            .map(|(i, _)| i)
            .expect("Unable to find answer index after shuffling");

        Some(Ok(MatchProblem {
            deck,
            question: (
                face_question.join_random(face_question.infer_separator(), self.rng),
                card,
                i,
            ),
            answers: answer_cards
                .into_iter()
                .map(|((face, card, i), correct)| {
                    (
                        (face.join_random(face.infer_separator(), self.rng), card, i),
                        correct,
                    )
                })
                .collect(),
            answer_index,
        }))
    }
}

struct MatchProblem<'a> {
    deck: &'a Deck,
    question: FaceCardIndex<'a>,
    answers: Vec<(FaceCardIndex<'a>, bool)>,
    answer_index: usize,
}

//NB 'suite lifetime technically not required, but I think it's more accurate
struct MatchProblemWidget<'a> {
    problem: &'a MatchProblem<'a>,
    progress: CorrectIncorrect,
    answer: Option<(usize, bool)>,
}

impl<'a> MatchProblemWidget<'a> {
    fn new(problem: &'a MatchProblem<'a>, progress: CorrectIncorrect) -> Self {
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

const ANSWERS_PER_PROBLEM: usize = 4;

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

        let (question_area, answer_area, progress_area) = (layout[0], layout[1], layout[2]);

        let layout =
            Layout::new(Direction::Vertical, [Constraint::Ratio(1, 2); 2]).split(answer_area);

        let (answer_top, answer_bot) = (layout[0], layout[1]);
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
                    |(answer_index, ((answer, _answer_card, _), _))| {
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
                    |(answer_index, ((_, card_answer, _), is_correct))| {
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
    progress: CorrectIncorrect,
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
    progress: CorrectIncorrect,
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
    use crate::{deck::load_decks, stats::Stats, ModeArguments};

    use super::MatchProblemIterator;

    #[test]
    fn ensure_unique_question_answers() {
        let decks = load_decks(vec!["./tests/deck1.json"]).unwrap();
        let mut args = ModeArguments::new(&decks, Stats::new(), None, None);
        let rng = &mut rand::thread_rng();
        let problems = MatchProblemIterator::new(args.deck_cards, &mut args.stats, args.faces, rng);

        for problem in problems.take(100) {
            let problem = problem.unwrap();
            assert!(problem
                .answers
                .iter()
                //Assert that each problem question is not present in the answers
                .all(|((answer, _, _), _)| answer != &problem.question.0));
            assert!(problem
                .answers
                .iter()
                .enumerate()
                .all(|(ref i, ((answer, _, _), _))| problem
                    .answers
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| i != j)
                    .all(|(_, ((other_answer, _, _), _))| other_answer != answer)))
        }
    }

    #[test]
    fn fails_if_not_enough_unique_answers() {
        let decks = load_decks(vec!["./tests/duplicate_cards"]).unwrap();
        let mut args = ModeArguments::new(&decks, Stats::new(), None, None);
        let rng = &mut rand::thread_rng();
        let mut problems =
            MatchProblemIterator::new(args.deck_cards, &mut args.stats, args.faces, rng);

        assert!(problems
            .next()
            .is_some_and(|problem| problem
                .is_err_and(|err| matches!(err, crate::FlashrError::DeckMismatch(_)))));
    }
}
