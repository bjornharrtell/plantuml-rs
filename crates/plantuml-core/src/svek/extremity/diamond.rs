//! `ExtremityDiamond` and its factory.

use super::{change_back, manage_round, Extremity, ExtremityFactory, Point, UDrawable};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, UPolygon, UTranslate};
use crate::klimt::geom::Side;
use std::f64::consts::PI;

struct ExtremityDiamond {
    polygon: UPolygon,
    fill: bool,
    contact: Point,
    delta_for_kal: UTranslate,
}

impl ExtremityDiamond {
    fn new(p1: Point, angle: f64, fill: bool) -> Self {
        let angle = manage_round(angle);
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(-6.0, -4.0);
        polygon.add_point(-12.0, 0.0);
        polygon.add_point(-6.0, 4.0);
        polygon.add_point(0.0, 0.0);
        polygon.rotate(angle + PI / 2.0);
        let p2 = polygon.points[2];
        let delta_for_kal = UTranslate::new(p2.0, p2.1).reverse();
        Self {
            polygon: polygon.translate(p1.0, p1.1),
            fill,
            contact: p1,
            delta_for_kal,
        }
    }
}

impl UDrawable for ExtremityDiamond {
    fn draw(&self, ug: &UGraphic) {
        let ug = if self.fill {
            change_back(ug)
        } else {
            ug.apply_back_color(HColor::TRANSPARENT)
        };
        ug.draw(self.polygon.clone());
    }
}

impl Extremity for ExtremityDiamond {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn delta_for_kal(&self) -> UTranslate {
        self.delta_for_kal
    }
    fn decoration_length(&self) -> f64 {
        12.0
    }
}

/// `ExtremityFactoryDiamond`.
pub struct ExtremityFactoryDiamond {
    fill: bool,
}

impl ExtremityFactoryDiamond {
    pub fn new(fill: bool) -> Self {
        Self { fill }
    }
}

impl ExtremityFactory for ExtremityFactoryDiamond {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityDiamond::new(p0, angle - PI / 2.0, self.fill))
    }
}
