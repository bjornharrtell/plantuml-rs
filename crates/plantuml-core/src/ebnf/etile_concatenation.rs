//! Sequence of tiles drawn left to right (`ETileConcatenation`).

use super::etile::{draw_hline, draw_hline_directed, ETile};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::StringBounder;

/// Space between two tiles of the sequence.
const MARGIN_X: f64 = 20.0;

/// Tiles in sequence, separated by an arrow (`ETileConcatenation`).
#[derive(Default)]
pub struct ETileConcatenation {
    tiles: Vec<Box<dyn ETile>>,
}

impl ETileConcatenation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a tile at the beginning, like Java's `push`.
    pub fn push(&mut self, tile: Box<dyn ETile>) {
        self.tiles.insert(0, tile);
    }

    /// Adds a tile at the end (`pushLast`).
    pub fn push_last(&mut self, tile: Box<dyn ETile>) {
        self.tiles.push(tile);
    }
}

impl ETile for ETileConcatenation {
    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles.iter().map(|t| t.get_h1(sb)).fold(0.0, f64::max)
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles.iter().map(|t| t.get_h2(sb)).fold(0.0, f64::max)
    }

    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        let mut width = 0.0;
        for (i, tile) in self.tiles.iter().enumerate() {
            width += tile.get_width(sb);
            if i != self.tiles.len() - 1 {
                width += MARGIN_X;
            }
        }
        width
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let full_line_pos = self.get_h1(sb);
        let mut x = 0.0;
        draw_hline(ug, full_line_pos, 0.0, x);
        for (i, tile) in self.tiles.iter().enumerate() {
            let line_pos = tile.get_h1(sb);
            tile.draw_u(&ug.apply_translate(UTranslate::new(x, full_line_pos - line_pos)));
            x += tile.calculate_dimension(sb).width;
            if i != self.tiles.len() - 1 {
                draw_hline_directed(ug, full_line_pos, x, x + MARGIN_X, 0.5, 25.0);
                x += MARGIN_X;
            }
        }
    }

    fn as_concatenation(&mut self) -> Option<&mut ETileConcatenation> {
        Some(self)
    }
}
