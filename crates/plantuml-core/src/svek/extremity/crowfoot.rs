//! `ExtremityCrowfoot`, `ExtremityLineCrowfoot`, `ExtremityDoubleLine` and their factories.

use super::{draw_line, manage_round, rotate_point, Extremity, ExtremityFactory, Point, UDrawable};
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::geom::Side;
use std::f64::consts::PI;

struct ExtremityCrowfoot {
    contact: Point,
    angle: f64,
    side: Option<Side>,
}

impl UDrawable for ExtremityCrowfoot {
    fn draw(&self, ug: &UGraphic) {
        let (x_wing, y_aperture) = (8.0, 8.0);
        let middle = (0.0, 0.0);
        let mut left = rotate_point((0.0, -y_aperture), self.angle);
        let base = rotate_point((-x_wing, 0.0), self.angle);
        let mut right = rotate_point((0.0, y_aperture), self.angle);
        if matches!(self.side, Some(Side::West | Side::East)) {
            left = (middle.0, left.1);
            right = (middle.0, right.1);
        }
        if matches!(self.side, Some(Side::South | Side::North)) {
            left = (left.0, middle.1);
            right = (right.0, middle.1);
        }
        draw_line(ug, self.contact.0, self.contact.1, base, left);
        draw_line(ug, self.contact.0, self.contact.1, base, right);
        draw_line(ug, self.contact.0, self.contact.1, base, middle);
    }
}

impl Extremity for ExtremityCrowfoot {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryCrowfoot`.
pub struct ExtremityFactoryCrowfoot;

impl ExtremityFactory for ExtremityFactoryCrowfoot {
    fn create(&self, p0: Point, angle: f64, side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityCrowfoot {
            contact: p0,
            angle: manage_round(angle + PI / 2.0),
            side,
        })
    }
}

struct ExtremityLineCrowfoot {
    contact: Point,
    angle: f64,
}

impl UDrawable for ExtremityLineCrowfoot {
    fn draw(&self, ug: &UGraphic) {
        let (x_wing, y_aperture, line_height) = (8.0, 6.0, 4.0);
        let middle = (0.0, 0.0);
        let left = rotate_point((0.0, -y_aperture), self.angle);
        let base = rotate_point((-x_wing, 0.0), self.angle);
        let right = rotate_point((0.0, y_aperture), self.angle);
        let line_top = rotate_point((-x_wing - 2.0, -line_height), self.angle);
        let line_bottom = rotate_point((-x_wing - 2.0, line_height), self.angle);
        draw_line(ug, self.contact.0, self.contact.1, base, left);
        draw_line(ug, self.contact.0, self.contact.1, base, right);
        draw_line(ug, self.contact.0, self.contact.1, base, middle);
        draw_line(ug, self.contact.0, self.contact.1, line_top, line_bottom);
    }
}

impl Extremity for ExtremityLineCrowfoot {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryLineCrowfoot`.
pub struct ExtremityFactoryLineCrowfoot;

impl ExtremityFactory for ExtremityFactoryLineCrowfoot {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityLineCrowfoot {
            contact: p0,
            angle: manage_round(angle + PI / 2.0),
        })
    }
}

struct ExtremityDoubleLine {
    contact: Point,
    angle: f64,
}

impl UDrawable for ExtremityDoubleLine {
    fn draw(&self, ug: &UGraphic) {
        let (x_wing, line_height) = (4.0, 4.0);
        let a = self.angle;
        let first_top = rotate_point((-x_wing, -line_height), a);
        let first_bottom = rotate_point((-x_wing, line_height), a);
        let second_top = rotate_point((-x_wing - 3.0, -line_height), a);
        let second_bottom = rotate_point((-x_wing - 3.0, line_height), a);
        let middle = rotate_point((0.0, 0.0), a);
        let base = rotate_point((-x_wing - 4.0, 0.0), a);
        let (cx, cy) = self.contact;
        draw_line(ug, cx, cy, first_top, first_bottom);
        draw_line(ug, cx, cy, second_top, second_bottom);
        draw_line(ug, cx, cy, base, middle);
    }
}

impl Extremity for ExtremityDoubleLine {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryDoubleLine`.
pub struct ExtremityFactoryDoubleLine;

impl ExtremityFactory for ExtremityFactoryDoubleLine {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityDoubleLine {
            contact: p0,
            angle: manage_round(angle + PI / 2.0),
        })
    }
}
