//! Base trait of the tiles of an EBNF diagram (`ETile`).

use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPath, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::style::{queries, StyleQuery};
use std::rc::Rc;

/// The style of every tile of an EBNF diagram (`ETile.getStyleQuery`).
pub fn get_style_query() -> StyleQuery {
    queries::EBNF.clone()
}

/// A piece of a syntax diagram.
///
/// A tile is laid out around its center line: [`ETile::get_h1`] is the height above it and
/// [`ETile::get_h2`] the height below it.
pub trait ETile {
    fn get_width(&self, sb: &dyn StringBounder) -> f64;
    fn get_h1(&self, sb: &dyn StringBounder) -> f64;
    fn get_h2(&self, sb: &dyn StringBounder) -> f64;
    fn draw_u(&self, ug: &UGraphic);

    /// Attaches a comment drawn above the tile; only box-like tiles keep it.
    fn add_comment_above(&mut self, _comment: String) {}

    /// Attaches a comment drawn below the tile; only box-like tiles keep it.
    fn add_comment_below(&mut self, _comment: String) {}

    /// Text of the rule when used with the `#n` repetition symbol.
    fn repetition_label(&self) -> String {
        "?".to_string()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(self.get_width(sb), self.get_h1(sb) + self.get_h2(sb))
    }

    /// The tile itself when it is an alternation; `EbnfEngine`'s `instanceof` check.
    fn as_alternation(&mut self) -> Option<&mut super::etile_alternation::ETileAlternation> {
        None
    }

    /// The tile itself when it is a concatenation; `EbnfEngine`'s `instanceof` check.
    fn as_concatenation(&mut self) -> Option<&mut super::etile_concatenation::ETileConcatenation> {
        None
    }
}

/// Wraps the root tile so that it can be drawn (`TextBlockMemoized` of the tiles).
pub struct TileBlock {
    tile: Rc<dyn ETile>,
}

impl TileBlock {
    pub fn new(tile: Rc<dyn ETile>) -> Self {
        Self { tile }
    }
}

impl TextBlock for TileBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.tile.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        self.tile.draw_u(ug);
    }
}

/// Draws a plain horizontal line (`ETile.drawHline`).
pub fn draw_hline(ug: &UGraphic, y: f64, x1: f64, x2: f64) {
    ug.apply_translate(UTranslate::new(x1, y))
        .draw(ULine::hline(x2 - x1));
}

/// Draws a horizontal line with an arrow in the middle pointing right
/// (`ETile.drawHlineDirected`).
pub fn draw_hline_directed(
    ug: &UGraphic,
    y: f64,
    x1: f64,
    x2: f64,
    coef: f64,
    minimum_size_for_arrow: f64,
) {
    ug.apply_translate(UTranslate::new(x1, y))
        .draw(ULine::hline(x2 - x1));
    if x2 > x1 + minimum_size_for_arrow {
        let x = x1 * (1.0 - coef) + x2 * coef - 2.0;
        ug.apply_back_color(ug.color().clone())
            .apply_translate(UTranslate::new(x, y))
            .draw(arrow_to_right());
    }
}

/// Draws a horizontal line with an arrow in the middle pointing left
/// (`ETile.drawHlineAntiDirected`).
pub fn draw_hline_anti_directed(ug: &UGraphic, y: f64, x1: f64, x2: f64, coef: f64) {
    ug.apply_translate(UTranslate::new(x1, y))
        .draw(ULine::hline(x2 - x1));
    let x = x1 * (1.0 - coef) + x2 * coef - 2.0;
    ug.apply_back_color(ug.color().clone())
        .apply_translate(UTranslate::new(x, y))
        .draw(arrow_to_left());
}

/// Draws a vertical line (`ETile.drawVline`).
pub fn draw_vline(ug: &UGraphic, x: f64, y1: f64, y2: f64) {
    ug.apply_translate(UTranslate::new(x, y1))
        .draw(ULine::vline(y2 - y1));
}

/// Left-pointing filled arrow, centered on its origin (`ETile.getArrowToLeft`).
pub fn arrow_to_left() -> UPath {
    let mut arrow = UPath::new();
    arrow.move_to(0.0, 0.0);
    arrow.line_to(0.0, -3.0);
    arrow.line_to(-6.0, 0.0);
    arrow.line_to(0.0, 3.0);
    arrow.line_to(0.0, 0.0);
    arrow.close_path();
    arrow
}

/// Right-pointing filled arrow, centered on its origin (`ETile.getArrowToRight`).
pub fn arrow_to_right() -> UPath {
    let mut arrow = UPath::new();
    arrow.move_to(0.0, 0.0);
    arrow.line_to(0.0, -3.0);
    arrow.line_to(6.0, 0.0);
    arrow.line_to(0.0, 3.0);
    arrow.line_to(0.0, 0.0);
    arrow.close_path();
    arrow
}
