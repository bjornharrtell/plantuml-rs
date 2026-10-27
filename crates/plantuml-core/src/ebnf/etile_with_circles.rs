//! Start and end markers of the whole diagram (`ETileWithCircles`).

use super::etile::{draw_hline_directed, ETile};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use std::rc::Rc;

/// Diameter of the two circles.
const SIZE: f64 = 8.0;
/// Space between a circle and the diagram.
const DELTA_X: f64 = 30.0;

/// The rule drawing framed by a filled circle on the left and an open one on the right
/// (`ETileWithCircles`).
pub struct ETileWithCircles {
    orig: Rc<dyn ETile>,
    line_color: HColor,
}

impl ETileWithCircles {
    pub fn new(orig: Rc<dyn ETile>, line_color: HColor) -> Self {
        Self { orig, line_color }
    }
}

impl ETile for ETileWithCircles {
    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_width(sb) + 2.0 * DELTA_X
    }

    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_h1(sb)
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_h2(sb)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let line_pos = self.get_h1(sb);
        let full_dim = self.calculate_dimension(sb);
        let ug = ug
            .apply_color(self.line_color.clone())
            .apply_stroke(UStroke::with_thickness(1.5));
        self.orig
            .draw_u(&ug.apply_translate(UTranslate::dx(DELTA_X)));

        let circle = UEllipse::build(SIZE, SIZE);
        // Java draws the entry marker as an open circle and the exit marker as a filled one.
        ug.apply_stroke(UStroke::with_thickness(2.0))
            .apply_back_color(HColor::TRANSPARENT)
            .apply_translate(UTranslate::new(0.0, line_pos - SIZE / 2.0))
            .draw(circle.clone());
        ug.apply_stroke(UStroke::with_thickness(1.0))
            .apply_back_color(self.line_color.clone())
            .apply_translate(UTranslate::new(
                full_dim.width - SIZE / 2.0,
                line_pos - SIZE / 2.0,
            ))
            .draw(circle);

        draw_hline_directed(&ug, line_pos, SIZE, DELTA_X, 0.5, 25.0);
        draw_hline_directed(
            &ug,
            line_pos,
            full_dim.width - DELTA_X,
            full_dim.width - SIZE / 2.0,
            0.5,
            25.0,
        );
    }
}
