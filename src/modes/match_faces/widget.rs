use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    symbols::{border, line},
    widgets::{Block, Borders, Gauge, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::CorrectIncorrect;

use super::{MatchProblem, ANSWERS_PER_PROBLEM};

pub(super) struct MatchProblemWidget<'a> {
    problem: &'a MatchProblem<'a>,
    progress: CorrectIncorrect,
    answer: Option<(usize, bool)>,
}

impl<'a> MatchProblemWidget<'a> {
    pub(super) fn new(problem: &'a MatchProblem<'a>, progress: CorrectIncorrect) -> Self {
        Self {
            problem,
            progress,
            answer: None,
        }
    }

    pub(super) fn answered(mut self, answer: (usize, bool)) -> Self {
        self.answer = Some(answer);
        self
    }
}

pub(super) struct MatchProblemWidgetState {
    pub(super) answer_areas: Vec<Rect>,
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
        let (question_area, answer_areas, progress_area, (divider_top_area, divider_bot_area)) = {
            let (question_area, answer_area, progress_area) = {
                let layout = Layout::new(
                    Direction::Vertical,
                    [
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(2, 3),
                        Constraint::Min(1),
                    ],
                );
                let split = layout.split(area);
                (split[0], split[1], split[2])
            };

            let (answer_top, answer_bot) = {
                let layout = Layout::new(Direction::Vertical, [Constraint::Ratio(1, 2); 2]);
                let split = layout.split(answer_area);
                (split[0], split[1])
            };

            let layout = Layout::new(
                Direction::Horizontal,
                [
                    Constraint::Ratio(1, 2),
                    Constraint::Min(1),
                    Constraint::Ratio(1, 2),
                ],
            );

            let (top_left, divider_top, top_right) = {
                let split = layout.split(answer_top);
                (split[0], split[1], split[2])
            };

            let (bot_left, divider_bot, bot_right) = {
                let split = layout.split(answer_bot);
                (split[0], split[1], split[2])
            };

            (
                question_area,
                [top_left, top_right, bot_left, bot_right],
                progress_area,
                (divider_top, divider_bot),
            )
        };

        let question = Paragraph::new(self.problem.question.prompt.to_owned())
            .wrap(Wrap { trim: false })
            .centered();

        let divider_top = Block::new()
            .borders(Borders::RIGHT | Borders::TOP)
            .border_set(border::Set {
                top_right: line::DOUBLE_HORIZONTAL_DOWN,
                ..border::DOUBLE
            });
        let divider_bot = Block::new()
            .borders(Borders::RIGHT | Borders::TOP)
            .border_set(border::Set {
                top_right: line::DOUBLE_CROSS,
                ..border::DOUBLE
            });

        match self.answer {
            None => {
                question.render(question_area, buf);

                self.problem
                    .answers
                    .iter()
                    .enumerate()
                    .for_each(|(answer_index, (answer, _))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        MatchAnswerWidget::new(answer.prompt.to_owned(), answer_index)
                            .render(answer_area, buf)
                    });

                divider_top.render(divider_top_area, buf);
                divider_bot.render(divider_bot_area, buf);
            }
            Some((answered_index, correct)) => {
                question
                    .fg(if correct { Color::Green } else { Color::Red })
                    .render(question_area, buf);

                self.problem.answers.iter().enumerate().for_each(
                    |(answer_index, (answer, is_correct))| {
                        let answer_area = answer_areas[answer_index];
                        state.answer_areas[answer_index] = answer_area;

                        let is_answered = answer_index == answered_index;
                        MatchAnswerWidget::new(answer.deck_card.join("\n"), answer_index)
                            .answered((*is_correct, is_answered))
                            .render(answer_area, buf)
                    },
                );

                divider_top
                    .fg(if answered_index < 2 {
                        if correct {
                            Color::Green
                        } else {
                            Color::Red
                        }
                    } else if self.problem.answer_index < 2 {
                        Color::Green
                    } else {
                        Color::default()
                    })
                    .render(divider_top_area, buf);
                divider_bot
                    .fg(if answered_index >= 2 {
                        if correct {
                            Color::Green
                        } else {
                            Color::Red
                        }
                    } else if self.problem.answer_index >= 2 {
                        Color::Green
                    } else {
                        Color::default()
                    })
                    .render(divider_bot_area, buf);
            }
        }

        {
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
        Paragraph::new(format!("{}: {}", self.answer_index + 1, self.answer))
            .wrap(Wrap { trim: false })
            .centered()
            .block(
                Block::bordered()
                    .borders(Borders::TOP)
                    .border_set(border::DOUBLE),
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
