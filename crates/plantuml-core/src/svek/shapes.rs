//! Participant figures: boundary, control and entity (`Boundary`, `Control`, `EntityDomain`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPath, UPolygon, UStroke, UTranslate,
};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;

const MARGIN: f64 = 4.0;
const RADIUS: f64 = 12.0;

pub struct Boundary {
    pub symbol_context: Fashion,
}

impl TextBlock for Boundary {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        let left = 17.0;
        Dimension::new(
            RADIUS * 2.0 + left + 2.0 * MARGIN,
            RADIUS * 2.0 + 2.0 * MARGIN,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let left = 17.0;
        let ug = self.symbol_context.apply(ug);
        let shadow = self.symbol_context.delta_shadow();
        let mut circle = UEllipse::build(RADIUS * 2.0, RADIUS * 2.0);
        circle.delta_shadow = shadow;
        let mut path = UPath::new();
        path.move_to(0.0, 0.0);
        path.line_to(0.0, RADIUS * 2.0);
        path.move_to(0.0, RADIUS);
        path.line_to(left, RADIUS);
        path.delta_shadow = shadow;
        ug.apply_translate(UTranslate::new(MARGIN, MARGIN))
            .apply_back_color(HColor::TRANSPARENT)
            .draw(path);
        ug.apply_translate(UTranslate::new(MARGIN + left, MARGIN))
            .draw(circle);
    }
}

pub struct Control {
    pub symbol_context: Fashion,
}

impl TextBlock for Control {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(RADIUS * 2.0 + 2.0 * MARGIN, RADIUS * 2.0 + 2.0 * MARGIN)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = self.symbol_context.apply(ug);
        let mut circle = UEllipse::build(RADIUS * 2.0, RADIUS * 2.0);
        circle.delta_shadow = self.symbol_context.delta_shadow();
        ug.apply_translate(UTranslate::new(MARGIN, MARGIN))
            .draw(circle);
        let ug = ug.apply_stroke(UStroke::simple());
        let fore = self
            .symbol_context
            .fore_color()
            .cloned()
            .unwrap_or(HColor::TRANSPARENT);
        let ug = ug.apply_back_color(fore);
        let (x_wing, y_aperture, x_contact) = (6.0, 5.0, 4.0);
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(x_wing, -y_aperture);
        polygon.add_point(x_contact, 0.0);
        polygon.add_point(x_wing, y_aperture);
        polygon.add_point(0.0, 0.0);
        ug.apply_translate(UTranslate::new(MARGIN + RADIUS - x_contact, MARGIN))
            .draw(polygon);
    }
}

pub struct EntityDomain {
    pub symbol_context: Fashion,
}

impl TextBlock for EntityDomain {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(RADIUS * 2.0 + 2.0 * MARGIN, RADIUS * 2.0 + 2.0 * MARGIN)
    }

    fn draw(&self, ug: &UGraphic) {
        let supp_y = 2.0;
        let ug = self.symbol_context.apply(ug);
        let mut circle = UEllipse::build(RADIUS * 2.0, RADIUS * 2.0);
        circle.delta_shadow = self.symbol_context.delta_shadow();
        ug.apply_translate(UTranslate::new(MARGIN, MARGIN))
            .draw(circle);
        ug.apply_translate(UTranslate::new(MARGIN, MARGIN + 2.0 * RADIUS + supp_y))
            .draw(ULine::hline(2.0 * RADIUS));
    }
}
