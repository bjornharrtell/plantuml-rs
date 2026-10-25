//! Decorations at the ends and in the middle of links (`svek/extremity`).
//!
//! Only the "simplest" link strategy is ported: every factory builds its drawable with
//! `create(center, angle, side)` (`createUDrawable`).

use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::geom::Side;

pub type Point = (f64, f64);

/// Something that draws itself at the origin of `ug` (`UDrawable`).
pub trait UDrawable {
    fn draw(&self, ug: &UGraphic);
}

/// The decoration at one end of a link (`Extremity`).
pub trait Extremity: UDrawable {
    fn some_point(&self) -> Option<Point>;

    fn delta_for_kal(&self) -> UTranslate {
        UTranslate::new(0.0, 0.0)
    }

    /// How much of the line the decoration covers.
    fn decoration_length(&self) -> f64 {
        8.0
    }

    /// Redraws the line of a transparent arrow head (`ExtremityArrow.drawLineIfTransparent`).
    fn draw_line_if_transparent(&self, _ug: &UGraphic) {}
}

/// Builds an [`Extremity`] (`ExtremityFactory`).
pub trait ExtremityFactory {
    fn create(&self, p0: Point, angle: f64, side: Option<Side>) -> Box<dyn Extremity>;
}

/// Builds the decoration in the middle of a link (`MiddleFactory`).
pub trait MiddleFactory {
    fn create(&self, angle: f64) -> Box<dyn UDrawable>;
}

/// Snaps angles close to a multiple of 90 degrees (`Extremity.manageround`).
pub fn manage_round(angle: f64) -> f64 {
    let deg = angle * 180.0 / std::f64::consts::PI;
    let close = |v: f64| (v - deg).abs() < 0.05;
    if close(0.0) {
        0.0
    } else if close(90.0) {
        90.0 * std::f64::consts::PI / 180.0
    } else if close(180.0) {
        180.0 * std::f64::consts::PI / 180.0
    } else if close(270.0) {
        270.0 * std::f64::consts::PI / 180.0
    } else if close(360.0) {
        0.0
    } else {
        angle
    }
}
