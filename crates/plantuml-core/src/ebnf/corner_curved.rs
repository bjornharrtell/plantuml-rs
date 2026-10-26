//! Rounded corner of a connector (`CornerCurved`).

use crate::klimt::drawing::ugraphic::{UGraphic, UPath};

/// Which corner is drawn (`CornerType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CornerType {
    Nw,
    Ne,
    Se,
    Sw,
}

/// A quarter-circle joining a horizontal and a vertical line (`CornerCurved`).
///
/// It draws relatively to its origin; callers translate the drawing context first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerCurved {
    delta: f64,
    corner: CornerType,
}

impl CornerCurved {
    fn new(corner: CornerType, delta: f64) -> Self {
        assert!(delta > 0.0, "corner delta must be positive");
        Self { delta, corner }
    }

    pub fn sw(delta: f64) -> Self {
        Self::new(CornerType::Sw, delta)
    }

    pub fn se(delta: f64) -> Self {
        Self::new(CornerType::Se, delta)
    }

    pub fn ne(delta: f64) -> Self {
        Self::new(CornerType::Ne, delta)
    }

    pub fn nw(delta: f64) -> Self {
        Self::new(CornerType::Nw, delta)
    }

    pub fn draw(&self, ug: &UGraphic) {
        let mut path = UPath::new();
        let a = self.delta / 4.0;
        let delta = self.delta;
        match self.corner {
            CornerType::Sw => {
                path.move_to(0.0, -delta);
                path.cubic_to(0.0, -a, a, 0.0, delta, 0.0);
            }
            CornerType::Se => {
                path.move_to(0.0, -delta);
                path.cubic_to(0.0, -a, -a, 0.0, -delta, 0.0);
            }
            CornerType::Ne => {
                path.move_to(-delta, 0.0);
                path.cubic_to(-a, 0.0, 0.0, a, 0.0, delta);
            }
            CornerType::Nw => {
                path.move_to(0.0, delta);
                path.cubic_to(0.0, a, a, 0.0, delta, 0.0);
            }
        }
        ug.draw(path);
    }
}
