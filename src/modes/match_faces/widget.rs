use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::{border, line},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, Gauge, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

use crate::color::LinearGradient;

use super::{MatchProblem, ANSWERS_PER_PROBLEM};

pub(super) struct MatchProblemWidget<'a> {
    problem: &'a MatchProblem<'a>,
    progress: (usize, usize),
    answer: Option<(usize, bool)>,
}

impl<'a> MatchProblemWidget<'a> {
    pub(super) fn new(problem: &'a MatchProblem<'a>, progress: (usize, usize)) -> Self {
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

const COLOR_CORRECT: Color = Color::Green;
const COLOR_INCORRECT: Color = Color::Red;

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
                if let Some(weights) = self.problem.weights.as_ref() {
                    let layout = Layout::new(
                        Direction::Vertical,
                        [
                            Constraint::Ratio(1, 12),
                            Constraint::Ratio(3, 12),
                            Constraint::Ratio(8, 12),
                            Constraint::Min(1),
                        ],
                    );
                    let split = layout.split(area);
                    let line_area = split[0];

                    //NOTE: Rendering weightline here to save passing around line_area/checking
                    //Options
                    WeightLineWidget::new(
                        weights,
                        self.answer.map(|(answered, _)| {
                            (
                                self.problem.question.index,
                                self.problem.answers[answered].0.index,
                            )
                        }),
                        line_area.width as usize,
                    )
                    .render(line_area, buf);

                    (split[1], split[2], split[3])
                } else {
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
                }
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

                for (answer_index, (answer, _)) in self.problem.answers.iter().enumerate() {
                    let answer_area = answer_areas[answer_index];
                    state.answer_areas[answer_index] = answer_area;

                    MatchAnswerWidget::new(answer.prompt.to_owned(), answer_index)
                        .render(answer_area, buf)
                }

                divider_top.render(divider_top_area, buf);
                divider_bot.render(divider_bot_area, buf);
            }
            Some((answered_index, correct)) => {
                {
                    let color = if correct {
                        COLOR_CORRECT
                    } else {
                        COLOR_INCORRECT
                    };
                    question.fg(color).render(question_area, buf);
                }

                for (answer_index, (answer, is_correct)) in self.problem.answers.iter().enumerate()
                {
                    let is_answered = answer_index == answered_index;

                    let answer_area = answer_areas[answer_index];
                    state.answer_areas[answer_index] = answer_area;

                    MatchAnswerWidget::new(answer.deck_card.join("\n"), answer_index)
                        .answered((*is_correct, is_answered))
                        .render(answer_area, buf)
                }

                let color_for_divider = |index_test: fn(usize) -> bool| -> Color {
                    if index_test(answered_index) {
                        if correct {
                            COLOR_CORRECT
                        } else {
                            COLOR_INCORRECT
                        }
                    } else if index_test(self.problem.answer_index) {
                        COLOR_CORRECT
                    } else {
                        Color::default()
                    }
                };

                divider_top
                    .fg(color_for_divider(|index| index < 2))
                    .render(divider_top_area, buf);
                divider_bot
                    .fg(color_for_divider(|index| index >= 2))
                    .render(divider_bot_area, buf);
            }
        }

        {
            let (completed, total) = self.progress;
            let (ratio, percent) = if total == 0 {
                //NOTE: Starting at ratio 1.0 so that
                //colors are "correct"
                (1.0, 0.0)
            } else {
                let ratio = completed as f64 / total as f64;
                (ratio, ratio * 100.0)
            };

            Gauge::default()
                .ratio(ratio)
                .label(format!("{percent:05.2}% ({completed}/{total})"))
                .gauge_style(Style::default().fg(COLOR_CORRECT).bg(COLOR_INCORRECT))
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
                        COLOR_CORRECT
                    } else {
                        COLOR_INCORRECT
                    }
                }
            })
            .render(area, buf)
    }
}

struct WeightLineWidget {
    weights: Vec<(f64, Option<bool>)>,
}

impl WeightLineWidget {
    fn new(weights: &[f64], answered: Option<(usize, usize)>, width: usize) -> Self {
        let num_weights = weights.len();

        let (weights, (min, max)) = if num_weights > width {
            let mut data = Vec::with_capacity(width);

            let ((big_window_size, small_window_size), (num_big, num_small)) = {
                let weights_per_width = num_weights as f64 / width as f64;

                let big_window_size = weights_per_width.ceil() as usize;
                let small_window_size = big_window_size - 1;

                let floor = weights_per_width.floor();
                let num_big = ((weights_per_width - floor) * width as f64).round() as usize;
                let num_small = width - num_big;

                ((big_window_size, small_window_size), (num_big, num_small))
            };

            let mut iter = weights.iter().enumerate();
            let (mut min, mut max) = (f64::MAX, f64::MIN);

            fn fold_next_window<'a, 'b>(
                size: usize,
                iter: &'b mut impl Iterator<Item = (usize, &'a f64)>,
                answered: Option<&(usize, usize)>,
            ) -> (f64, Option<bool>) {
                (0..size).fold((0.0, None), |(total, selected), _| {
                    let (i, w) = iter.next().expect("Unable to get next weight");

                    let total = total + w;
                    let selected = selected
                        .filter(|s| *s)
                        .or_else(|| answered.map(|(i_q, i_a)| *i_q == i || *i_a == i));

                    (total / size as f64, selected)
                })
            }

            for _ in 0..num_small {
                let (avg, selected) =
                    fold_next_window(small_window_size, &mut iter, answered.as_ref());

                min = min.min(avg);
                max = max.max(avg);
                data.push((avg, selected));
            }

            for _ in 0..num_big {
                let (avg, selected) =
                    fold_next_window(big_window_size, &mut iter, answered.as_ref());

                min = min.min(avg);
                max = max.max(avg);
                data.push((avg, selected));
            }

            let count = iter.count();
            assert!(count == 0, "Weights remaining after folding: {count}",);

            (data, (min, max))
        } else {
            //NOTE: Allocating new array just so if-arms match
            let mut data = Vec::with_capacity(num_weights);
            let (mut min, mut max) = (f64::MAX, f64::MIN);

            for (i, w) in weights.iter().enumerate() {
                let weight = *w;
                min = min.min(weight);
                max = max.max(weight);
                data.push((
                    weight,
                    answered.as_ref().map(|(i_q, i_a)| *i_q == i || *i_a == i),
                ));
            }

            (data, (min, max))
        };

        Self {
            weights: {
                let diff = max - min;
                let mut buf = Vec::with_capacity(weights.len());
                for (weight, percent) in weights {
                    buf.push(((1.0 - ((weight - min) / diff)), percent))
                }
                buf
            },
        }
    }
}

impl Widget for WeightLineWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut chart = BarChart::default();

        const PERCENT_SELECTED: f64 = 1.0;
        const PERCENT_HIDDEN: f64 = 0.25;

        for (w, selected) in self.weights.into_iter() {
            let color = {
                let color = LinearGradient::rainbow().sample(w);

                selected
                    .map(|selected| {
                        color
                            .percent(if selected {
                                PERCENT_SELECTED
                            } else {
                                PERCENT_HIDDEN
                            })
                            .into()
                    })
                    .unwrap_or_else(|| color.into())
            };

            let style = Style::default().fg(color);

            chart = chart.data(
                BarGroup::default().bars(&[Bar::default()
                    .value((w * u8::MAX as f64) as u64)
                    .style(style)]),
            );
        }

        chart.bar_gap(0).reversed().render(area, buf)
    }
}
