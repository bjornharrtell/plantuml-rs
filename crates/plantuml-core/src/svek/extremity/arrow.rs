//! `ExtremityArrow`, `ExtremityHalfArrow`, `ExtremityArrowAndCircle` and their factories.

use super::{
    change_back, manage_round, rotate_point, Extremity, ExtremityFactory, Point, UDrawable,
};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, ULine, UPolygon, UStroke, UTranslate};
use crate::klimt::geom::Side;
use std::f64::consts::PI;

struct ExtremityArrow {
    polygon: UPolygon,
    contact: Point,
}

impl ExtremityArrow {
    fn new(p0: Point, angle: f64) -> Self {
        let angle = manage_round(angle);
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(-9.0, -4.0);
        polygon.add_point(-5.0, 0.0);
        polygon.add_point(-9.0, 4.0);
        polygon.add_point(0.0, 0.0);
        polygon.rotate(angle);
        Self {
            polygon: polygon.translate(p0.0, p0.1),
            contact: p0,
        }
    }
}

impl UDrawable for ExtremityArrow {
    fn draw(&self, ug: &UGraphic) {
        change_back(ug).draw(self.polygon.clone());
    }
}

impl Extremity for ExtremityArrow {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        5.0
    }
    fn draw_line_if_transparent(&self, ug: &UGraphic) {
        let pt1 = self.polygon.points[0];
        let pt2 = self.polygon.points[2];
        ug.apply_translate(UTranslate::new(pt1.0, pt1.1))
            .draw(ULine::new(pt2.0 - pt1.0, pt2.1 - pt1.1));
    }
}

/// `ExtremityFactoryArrow`.
pub struct ExtremityFactoryArrow;

impl ExtremityFactory for ExtremityFactoryArrow {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityArrow::new(p0, angle))
    }
}

struct ExtremityHalfArrow {
    contact: Point,
    line: ULine,
    other_line: ULine,
}

impl ExtremityHalfArrow {
    fn new(p0: Point, angle: f64, direction: i32) -> Self {
        let angle = manage_round(angle);
        let other = rotate_point((-9.0, -f64::from(4 * direction)), angle);
        let other2 = rotate_point((-8.0, 0.0), angle);
        Self {
            contact: p0,
            line: ULine::new(other.0, other.1),
            other_line: ULine::new(other2.0, other2.1),
        }
    }
}

impl UDrawable for ExtremityHalfArrow {
    fn draw(&self, ug: &UGraphic) {
        let ug = change_back(ug);
        if self.line.dx.hypot(self.line.dy) > 2.0 {
            let position = UTranslate::new(self.contact.0, self.contact.1);
            ug.apply_translate(position).draw(self.line.clone());
            ug.apply_translate(position).draw(self.other_line.clone());
        }
    }
}

impl Extremity for ExtremityHalfArrow {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryHalfArrow`.
pub struct ExtremityFactoryHalfArrow {
    direction: i32,
}

impl ExtremityFactoryHalfArrow {
    pub fn new(direction: i32) -> Self {
        Self { direction }
    }
}

impl ExtremityFactory for ExtremityFactoryHalfArrow {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityHalfArrow::new(p0, angle, self.direction))
    }
}

const ARROW_CIRCLE_RADIUS: f64 = 5.0;

struct ExtremityArrowAndCircle {
    polygon: UPolygon,
    contact: Point,
    dest: Point,
    background: HColor,
}

impl ExtremityArrowAndCircle {
    fn new(p1: Point, angle: f64, background: HColor) -> Self {
        let angle = manage_round(angle);
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(-9.0, -4.0);
        polygon.add_point(-5.0, 0.0);
        polygon.add_point(-9.0, 4.0);
        polygon.add_point(0.0, 0.0);
        polygon.rotate(angle + PI / 2.0);
        let polygon = polygon.translate(
            p1.0 + ARROW_CIRCLE_RADIUS * angle.sin(),
            p1.1 - ARROW_CIRCLE_RADIUS * angle.cos(),
        );
        let contact = (
            p1.0 - 5.0 * (angle + PI / 2.0).cos(),
            p1.1 - 5.0 * (angle + PI / 2.0).sin(),
        );
        Self {
            polygon,
            contact,
            dest: p1,
            background,
        }
    }
}

impl UDrawable for ExtremityArrowAndCircle {
    fn draw(&self, ug: &UGraphic) {
        change_back(ug).draw(self.polygon.clone());
        let r = ARROW_CIRCLE_RADIUS;
        ug.apply_stroke(UStroke::with_thickness(1.5))
            .apply_back_color(self.background.clone())
            .apply_translate(UTranslate::new(self.dest.0 - r, self.dest.1 - r))
            .draw(UEllipse::build(r * 2.0, r * 2.0));
    }
}

impl Extremity for ExtremityArrowAndCircle {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryArrowAndCircle`.
pub struct ExtremityFactoryArrowAndCircle {
    background: HColor,
}

impl ExtremityFactoryArrowAndCircle {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactoryArrowAndCircle {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityArrowAndCircle::new(
            p0,
            angle - PI / 2.0,
            self.background.clone(),
        ))
    }
}
