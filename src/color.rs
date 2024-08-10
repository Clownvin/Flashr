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

use std::ops::Deref;

use ratatui::style::Color as RatColor;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[repr(transparent)]
pub(crate) struct Percent(f64);

impl From<f64> for Percent {
    fn from(value: f64) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&value),
            "Value must be in the range [0, 1]"
        );
        Self(value)
    }
}

impl Deref for Percent {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Color {
    pub const RED: Color = Color::new(0xFF, 0x00, 0x00);
    pub const YELLOW: Color = Color::new(0xFF, 0xFF, 0x00);
    pub const GREEN: Color = Color::new(0x00, 0xFF, 0x00);
    pub const CYAN: Color = Color::new(0x00, 0xFF, 0xFF);
    pub const BLUE: Color = Color::new(0x00, 0x00, 0xFF);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn blend_with(self, other: Color, pct_other: impl Into<Percent>) -> Self {
        let pct_other = *pct_other.into();
        let pct_self = 1.0 - pct_other;
        Self::new(
            ((self.r as f64 * pct_self) + (other.r as f64 * pct_other)) as u8,
            ((self.g as f64 * pct_self) + (other.g as f64 * pct_other)) as u8,
            ((self.b as f64 * pct_self) + (other.b as f64 * pct_other)) as u8,
        )
    }

    pub fn percent(self, percent: impl Into<Percent>) -> Self {
        let percent = *percent.into();
        Self::new(
            (self.r as f64 * percent) as u8,
            (self.g as f64 * percent) as u8,
            (self.b as f64 * percent) as u8,
        )
    }
}

impl From<Color> for RatColor {
    fn from(value: Color) -> Self {
        RatColor::Rgb(value.r, value.g, value.b)
    }
}

#[repr(transparent)]
pub struct LinearGradient {
    colors: Vec<Color>,
}

const RAINBOW: [Color; 5] = [
    Color::RED,
    Color::YELLOW,
    Color::GREEN,
    Color::CYAN,
    Color::BLUE,
];

impl LinearGradient {
    pub fn new(colors: impl IntoIterator<Item = Color>) -> Self {
        let colors = colors.into_iter().collect::<Vec<_>>();

        Self { colors }
    }

    pub fn rainbow() -> Self {
        Self::new(RAINBOW)
    }

    pub fn sample(&self, progress: f64) -> Color {
        if progress >= 1.0 {
            return *self.colors.last().expect("No last color");
        }

        let scaled = (self.colors.len() - 1) as f64 * progress;
        let floor = scaled.floor();

        let left = floor as usize;
        let right = left + 1;

        debug_assert!(
            right < self.colors.len(),
            "{} greater than {}: calculated from ({} * {}).floor() = {}",
            right,
            self.colors.len(),
            self.colors.len(),
            progress,
            left
        );

        let diff = scaled - floor;

        self.colors[left].blend_with(self.colors[right], diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real gradient that was displaying grayscale 180 almost 100%
    /// while early into the step (0 in fact). Point is, it was broken,
    /// and is now fixed.
    #[test]
    fn test_linear_gradient_1() {
        let gradient = LinearGradient::new([
            Color::new(244, 230, 139),
            Color::new(17, 167, 17),
            Color::new(5, 61, 5),
            Color::new(70, 70, 70),
            Color::new(180, 180, 180),
            Color::new(230, 230, 230),
        ]);

        let progress = (431 - 300) as f64 / (520 - 300) as f64;
        let color1 = gradient.sample(progress);
        assert!(color1 == Color::new(68, 69, 68));

        let progress = (432 - 300) as f64 / (520 - 300) as f64;
        let color2 = gradient.sample(progress);
        assert!(color2 == Color::new(70, 70, 70));

        let progress = (433 - 300) as f64 / (520 - 300) as f64;
        let color3 = gradient.sample(progress);
        assert!(color3 == Color::new(72, 72, 72));
    }
}
