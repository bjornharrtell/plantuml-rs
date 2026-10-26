//! Brace drawn above a repetition (`Brace`).

use super::corner_curved::CornerCurved;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UStroke, UTranslate};

/// A horizontal brace of the given width, drawn relatively to its origin (`Brace`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brace {
    width: f64,
}

impl Brace {
    pub fn new(width: f64) -> Self {
        Self { width }
    }

    /// Draws the brace; Java's `height` argument is unused there too.
    pub fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_stroke(UStroke::with_thickness(0.5));
        let width = self.width;
        CornerCurved::nw(5.0).draw(&ug);
        CornerCurved::se(5.0).draw(&ug.apply_translate(UTranslate::new(width / 2.0, 0.0)));
        CornerCurved::sw(5.0).draw(&ug.apply_translate(UTranslate::new(width / 2.0, 0.0)));
        CornerCurved::ne(5.0).draw(&ug.apply_translate(UTranslate::new(width, 0.0)));
        ug.apply_translate(UTranslate::new(5.0, 0.0))
            .draw(ULine::hline(width / 2.0 - 10.0));
        ug.apply_translate(UTranslate::new(5.0 + width / 2.0, 0.0))
            .draw(ULine::hline(width / 2.0 - 10.0));
    }
}
