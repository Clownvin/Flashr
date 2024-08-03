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

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

/// Offsets to account for boxes.
/// Eg, if Text is boxed on all sides, would use 2 for both
#[derive(Default)]
pub(crate) struct BoxOffsets {
    flags: u8,
}

const TOP: u8 = 0x1;
const BOT: u8 = 0x2;
const LEFT: u8 = 0x4;
const RIGHT: u8 = 0x8;

impl BoxOffsets {
    pub const fn top(self) -> Self {
        Self {
            flags: self.flags | TOP,
        }
    }

    pub const fn bot(self) -> Self {
        Self {
            flags: self.flags | BOT,
        }
    }

    pub const fn left(self) -> Self {
        Self {
            flags: self.flags | LEFT,
        }
    }

    pub const fn right(self) -> Self {
        Self {
            flags: self.flags | RIGHT,
        }
    }

    pub const fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub const fn vertical(&self) -> u16 {
        self.has(TOP) as u16 + self.has(BOT) as u16
    }

    pub const fn horizontal(&self) -> u16 {
        self.has(LEFT) as u16 + self.has(RIGHT) as u16
    }
}

const fn crop_box(mut area: Rect, box_offsets: BoxOffsets) -> Rect {
    let true_area_width = area.width.saturating_sub(box_offsets.horizontal());

    if true_area_width == 0 || area.height == 0 {
        return area;
    }

    let true_area_height = area.height.saturating_sub(box_offsets.vertical());

    area.x += box_offsets.has(LEFT) as u16;
    area.y += box_offsets.has(TOP) as u16;
    area.width = true_area_width;
    area.height = true_area_height;

    area
}

pub(crate) fn horizontally_centered_area_for_string(
    area: Rect,
    string: &str,
    box_offsets: BoxOffsets,
) -> Rect {
    let mut area = crop_box(area, box_offsets);

    if area.height <= 1 {
        return area;
    }

    let lines = string.lines().fold(0.0, |total, s| {
        total + (s.width() as f64 / area.width as f64).ceil()
    });

    //SAFETY: Convert to usize first, which is guaranteed to be able to hold
    //line count
    if area.height as usize > lines as usize {
        let lines = lines as u16;

        area.y += (area.height - lines) / 2;
        area.height = lines;
    }

    area
}
