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

pub(crate) fn horizontally_centered_area_for_string(
    area: Rect,
    string: &String,
    box_offsets: BoxOffsets,
) -> Rect {
    let horizontal_box_offset = box_offsets.horizontal();
    let true_area_width = area.width.saturating_sub(horizontal_box_offset);
    if true_area_width == 0 {
        return area;
    }

    let vertical_box_offset = box_offsets.vertical();
    let true_area_height = area.height.saturating_sub(vertical_box_offset);
    if true_area_height <= 1 {
        return area;
    }

    let lines = ((string.width() as f64 / true_area_width as f64).ceil() as usize).max(1);

    if lines >= true_area_height as usize {
        area
    } else {
        //SAFETY: Converting to u16 now rather than earlier
        //as lines could potentially have been larger than a u16.
        let lines = lines as u16;
        let box_top = box_offsets.has(TOP) as u16;
        let box_left = box_offsets.has(LEFT) as u16;

        let top = area.y + box_top + ((true_area_height / 2) - (lines / 2));

        Rect {
            x: area.x + box_left,
            y: top,
            width: area.width - box_offsets.horizontal(),
            height: lines,
        }
    }
}
