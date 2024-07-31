use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::deck::Face;

pub(super) struct FlashcardWidget<'a> {
    face: (&'a String, &'a Face),
}

impl<'a> FlashcardWidget<'a> {
    pub fn new(face: (&'a String, &'a Face)) -> Self {
        Self { face }
    }
}

impl<'a> Widget for FlashcardWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let (face_name_area, face_area) = {
            let layout = Layout::new(
                Direction::Vertical,
                [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)],
            );

            let split = layout.split(area);
            (split[0], split[1])
        };

        Paragraph::new(format!("{}:", self.face.0).to_owned())
            .wrap(Wrap { trim: false })
            .centered()
            .render(face_name_area, buf);

        Paragraph::new(self.face.1.join(self.face.1.infer_separator()))
            .wrap(Wrap { trim: false })
            .centered()
            .render(face_area, buf)
    }
}
