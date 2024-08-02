/*
 * Copyright (C) 2024 Clownvin <123clownvin@gmail.com>
 *
 * This file is part of Flashr.
 *
 * Flashr is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Flashr is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Flashr.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::{
    deck::Face,
    render_utils::{horizontally_centered_area_for_string, BoxOffsets},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

#[derive(Default)]
pub(super) struct FlashcardWidgetState {
    pub left: Rect,
    pub right: Rect,
}

pub(super) struct FlashcardWidget<'a> {
    prev: String,
    face: (&'a String, &'a Face),
    next: String,
}

impl<'a> FlashcardWidget<'a> {
    pub fn new(face: (&'a String, &'a Face), prev: String, next: String) -> Self {
        Self { face, prev, next }
    }
}

impl<'a> StatefulWidget for FlashcardWidget<'a> {
    type State = FlashcardWidgetState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        let face_string = self.face.1.join(self.face.1.infer_separator());

        let (title_area, face_area, sides) = {
            let (left, middle, right) = {
                let layout = Layout::new(
                    Direction::Horizontal,
                    [
                        Constraint::Ratio(1, 6),
                        Constraint::Ratio(2, 3),
                        Constraint::Ratio(1, 6),
                    ],
                );
                let split = layout.split(area);

                (split[0], split[1], split[2])
            };

            let title_area = {
                let layout = Layout::new(
                    Direction::Vertical,
                    [Constraint::Length(3), Constraint::Fill(1)],
                );
                layout.split(middle)[0]
            };

            let face_area =
                horizontally_centered_area_for_string(middle, &face_string, BoxOffsets::default());

            (title_area, face_area, (left, right))
        };

        {
            state.left = sides.0;
            Block::default()
                .borders(Borders::RIGHT)
                .border_type(BorderType::Plain)
                .render(state.left, buf);

            let area = horizontally_centered_area_for_string(
                state.left,
                &self.prev,
                BoxOffsets::default().right(),
            );
            Paragraph::new(self.prev)
                .wrap(Wrap { trim: false })
                .centered()
                .render(area, buf);
        }

        {
            let string = format!("{}:", self.face.0);
            let area =
                horizontally_centered_area_for_string(title_area, &string, BoxOffsets::default());
            Paragraph::new(string)
                .wrap(Wrap { trim: false })
                .centered()
                .render(area, buf);
        }

        {
            Paragraph::new(face_string)
                .wrap(Wrap { trim: false })
                .centered()
                .render(face_area, buf);
        }

        {
            state.right = sides.1;
            Block::default()
                .borders(Borders::LEFT)
                .border_type(BorderType::Plain)
                .render(state.right, buf);

            let area = horizontally_centered_area_for_string(
                state.right,
                &self.next,
                BoxOffsets::default().left(),
            );
            Paragraph::new(self.next)
                .wrap(Wrap { trim: false })
                .centered()
                .render(area, buf);
        }
    }
}
