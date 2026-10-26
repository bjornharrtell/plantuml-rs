//! Choice between several tiles, drawn top to bottom (`ETileAlternation`).

use super::corner_curved::CornerCurved;
use super::etile::{draw_hline, draw_hline_directed, draw_vline, ETile};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::StringBounder;

/// Horizontal space between the vertical trunk and a branch.
const MARGIN_X: f64 = 12.0;
/// Vertical space between two branches.
const MARGIN_Y: f64 = 10.0;

/// Tiles in alternation, one per branch (`ETileAlternation`).
///
/// Java also has an `inZeroOrMore` mode for this tile, but its only setter is deprecated and never
/// called, so the port keeps the normal drawing only.
#[derive(Default)]
pub struct ETileAlternation {
    tiles: Vec<Box<dyn ETile>>,
}

impl ETileAlternation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a branch, like Java's `push` (the last token of the rule comes first).
    pub fn push(&mut self, tile: Box<dyn ETile>) {
        self.tiles.insert(0, tile);
    }
}

impl ETile for ETileAlternation {
    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles.first().map_or(0.0, |t| t.get_h1(sb))
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        let mut height = self.tiles.first().map_or(0.0, |t| t.get_h2(sb));
        for tile in self.tiles.iter().skip(1) {
            height += tile.get_h1(sb) + tile.get_h2(sb) + MARGIN_Y;
        }
        height
    }

    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        let width = self
            .tiles
            .iter()
            .map(|t| t.calculate_dimension(sb).width)
            .fold(0.0, f64::max);
        width + 4.0 * MARGIN_X
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let mut y = 0.0;
        let mut last_line_pos = 0.0;
        let a = 0.0;
        let b = a + MARGIN_X;
        let c = b + MARGIN_X;
        let full_dim = self.calculate_dimension(sb);
        let r = full_dim.width;
        let q = r - MARGIN_X;
        let p = q - MARGIN_X;
        let line_pos = self.get_h1(sb);

        for (i, tile) in self.tiles.iter().enumerate() {
            let dim = tile.calculate_dimension(sb);
            last_line_pos = y + tile.get_h1(sb);
            tile.draw_u(&ug.apply_translate(UTranslate::new(c, y)));
            if i == 0 {
                draw_hline(ug, last_line_pos, a, c);
                draw_hline(ug, last_line_pos, c + dim.width, r);
            } else if i < self.tiles.len() - 1 {
                CornerCurved::sw(MARGIN_X)
                    .draw(&ug.apply_translate(UTranslate::new(b, last_line_pos)));
                draw_hline_directed(ug, last_line_pos, c + dim.width, p, 0.5, 25.0);
                CornerCurved::se(MARGIN_X)
                    .draw(&ug.apply_translate(UTranslate::new(q, last_line_pos)));
            } else {
                draw_hline_directed(ug, last_line_pos, c + dim.width, p, 0.5, 25.0);
            }
            y += dim.height + MARGIN_Y;
        }

        let height42 = last_line_pos - line_pos;
        let ug_b = ug.apply_translate(UTranslate::new(b, line_pos));
        let ug_q = ug.apply_translate(UTranslate::new(q, line_pos));
        CornerCurved::sw(MARGIN_X).draw(&ug_b.apply_translate(UTranslate::dy(height42)));
        draw_vline(&ug_b, 0.0, MARGIN_X, height42 - MARGIN_X);
        CornerCurved::ne(MARGIN_X).draw(&ug_b);
        CornerCurved::se(MARGIN_X).draw(&ug_q.apply_translate(UTranslate::dy(height42)));
        draw_vline(&ug_q, 0.0, MARGIN_X, height42 - MARGIN_X);
        CornerCurved::nw(MARGIN_X).draw(&ug_q);
    }

    fn as_alternation(&mut self) -> Option<&mut ETileAlternation> {
        Some(self)
    }
}
