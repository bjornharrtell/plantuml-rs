//! The empty tile, drawn as nothing (`ETileEmpty`).

use super::etile::ETile;
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::StringBounder;

/// Rule without a right-hand side (`ETileEmpty`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ETileEmpty;

impl ETile for ETileEmpty {
    fn get_width(&self, _sb: &dyn StringBounder) -> f64 {
        0.0
    }

    fn get_h1(&self, _sb: &dyn StringBounder) -> f64 {
        0.0
    }

    fn get_h2(&self, _sb: &dyn StringBounder) -> f64 {
        0.0
    }

    fn draw_u(&self, _ug: &UGraphic) {}
}
