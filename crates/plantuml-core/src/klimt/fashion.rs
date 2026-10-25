//! Fill, line and shape settings of an element (`Fashion`, formerly `SymbolContext`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, UStroke};

#[derive(Debug, Clone, PartialEq)]
pub struct Fashion {
    back_color: Option<HColor>,
    fore_color: Option<HColor>,
    stroke: UStroke,
    delta_shadow: f64,
    round_corner: f64,
    diagonal_corner: f64,
}

impl Fashion {
    pub fn new(back_color: Option<HColor>, fore_color: Option<HColor>) -> Self {
        Self {
            back_color,
            fore_color,
            stroke: UStroke::simple(),
            delta_shadow: 0.0,
            round_corner: 0.0,
            diagonal_corner: 0.0,
        }
    }

    pub fn with_delta_shadow(mut self, d: f64) -> Self {
        self.delta_shadow = d;
        self
    }

    pub fn with_stroke(mut self, s: UStroke) -> Self {
        self.stroke = s;
        self
    }

    pub fn with_back_color(mut self, c: Option<HColor>) -> Self {
        self.back_color = c;
        self
    }

    pub fn with_fore_color(mut self, c: Option<HColor>) -> Self {
        self.fore_color = c;
        self
    }

    pub fn with_corner(mut self, round: f64, diagonal: f64) -> Self {
        self.round_corner = round;
        self.diagonal_corner = diagonal;
        self
    }

    pub fn back_color(&self) -> Option<&HColor> {
        self.back_color.as_ref()
    }

    pub fn fore_color(&self) -> Option<&HColor> {
        self.fore_color.as_ref()
    }

    pub fn stroke(&self) -> UStroke {
        self.stroke
    }

    pub fn is_shadowing(&self) -> bool {
        self.delta_shadow > 0.0
    }

    pub fn delta_shadow(&self) -> f64 {
        self.delta_shadow
    }

    pub fn round_corner(&self) -> f64 {
        self.round_corner
    }

    pub fn diagonal_corner(&self) -> f64 {
        self.diagonal_corner
    }

    /// Applies fore and back colors, then the stroke, to a drawing context.
    pub fn apply(&self, ug: &UGraphic) -> UGraphic {
        self.apply_colors(ug).apply_stroke(self.stroke)
    }

    pub fn apply_colors(&self, ug: &UGraphic) -> UGraphic {
        let fore = self.fore_color.clone().unwrap_or(HColor::TRANSPARENT);
        let back = self.back_color.clone().unwrap_or(HColor::TRANSPARENT);
        ug.apply_color(fore).apply_back_color(back)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders() {
        let f = Fashion::new(HColor::parse("red"), None)
            .with_delta_shadow(3.0)
            .with_stroke(UStroke::with_thickness(2.0))
            .with_corner(5.0, 1.0)
            .with_fore_color(HColor::parse("blue"))
            .with_back_color(None);
        assert!(f.is_shadowing());
        assert_eq!(f.delta_shadow(), 3.0);
        assert_eq!(f.stroke().thickness, 2.0);
        assert_eq!((f.round_corner(), f.diagonal_corner()), (5.0, 1.0));
        assert!(f.fore_color().is_some() && f.back_color().is_none());
    }
}
