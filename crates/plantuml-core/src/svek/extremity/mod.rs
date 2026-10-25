//! Decorations at the ends and in the middle of links (`svek/extremity`).
//!
//! Only the "simplest" link strategy is ported: every factory builds its drawable with
//! `create(center, angle, side)` (`createUDrawable`).

use crate::decoration::link::{LinkDecor, LinkMiddleDecor};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::ULine;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::geom::Side;

mod arrow;
mod circle;
mod crowfoot;
mod diamond;
mod middle;
mod triangle;

#[cfg(test)]
mod tests;

pub use arrow::{ExtremityFactoryArrow, ExtremityFactoryArrowAndCircle, ExtremityFactoryHalfArrow};
pub use circle::{
    ExtremityFactoryCircle, ExtremityFactoryCircleConnect, ExtremityFactoryCircleCross,
    ExtremityFactoryCircleCrowfoot, ExtremityFactoryCircleLine, ExtremityFactoryParenthesis,
    ExtremityFactoryPlus, ExtremityFactorySquare,
};
pub use crowfoot::{
    ExtremityFactoryCrowfoot, ExtremityFactoryDoubleLine, ExtremityFactoryLineCrowfoot,
};
pub use diamond::ExtremityFactoryDiamond;
pub use middle::{
    MiddleCircleCircledMode, MiddleFactoryCircle, MiddleFactoryCircleCircled, MiddleFactorySubset,
};
pub use triangle::{
    ExtremityFactoryExtends, ExtremityFactoryExtendsLike, ExtremityFactoryNotNavigable,
    ExtremityFactoryTriangle,
};

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

/// Rotates `p` around the origin (`XPoint2D.transform(XAffineTransform.getRotateInstance)`).
pub(crate) fn rotate_point(p: Point, theta: f64) -> Point {
    let (sin, cos) = theta.sin_cos();
    (p.0 * cos - p.1 * sin, p.0 * sin + p.1 * cos)
}

/// Draws the line `p1`-`p2` relative to `(x, y)`.
pub(crate) fn draw_line(ug: &UGraphic, x: f64, y: f64, p1: Point, p2: Point) {
    ug.apply_translate(UTranslate::new(x + p1.0, y + p1.1))
        .draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
}

/// Fills with the current line color (`HColors.changeBack`).
pub(crate) fn change_back(ug: &UGraphic) -> UGraphic {
    ug.apply_back_color(ug.color().clone())
}

/// `LinkDecor.getExtremityFactoryComplete`.
pub fn extremity_factory_complete(
    decor: LinkDecor,
    background: &HColor,
) -> Option<Box<dyn ExtremityFactory>> {
    if decor == LinkDecor::Extends {
        return Some(Box::new(ExtremityFactoryTriangle::new(None, 18, 6, 18)));
    }
    extremity_factory_legacy(decor, background)
}

/// `LinkDecor.getExtremityFactoryLegacy`.
pub fn extremity_factory_legacy(
    decor: LinkDecor,
    bg: &HColor,
) -> Option<Box<dyn ExtremityFactory>> {
    let bg = bg.clone();
    Some(match decor {
        LinkDecor::Plus => Box::new(ExtremityFactoryPlus::new(bg)),
        LinkDecor::Redefines => Box::new(ExtremityFactoryExtendsLike::new(bg, false)),
        LinkDecor::DefinedBy => Box::new(ExtremityFactoryExtendsLike::new(bg, true)),
        LinkDecor::HalfArrowUp => Box::new(ExtremityFactoryHalfArrow::new(1)),
        LinkDecor::HalfArrowDown => Box::new(ExtremityFactoryHalfArrow::new(-1)),
        LinkDecor::ArrowTriangle => Box::new(ExtremityFactoryTriangle::new(None, 8, 3, 8)),
        LinkDecor::Crowfoot => Box::new(ExtremityFactoryCrowfoot),
        LinkDecor::CircleCrowfoot => Box::new(ExtremityFactoryCircleCrowfoot),
        LinkDecor::LineCrowfoot => Box::new(ExtremityFactoryLineCrowfoot),
        LinkDecor::CircleLine => Box::new(ExtremityFactoryCircleLine),
        LinkDecor::DoubleLine => Box::new(ExtremityFactoryDoubleLine),
        LinkDecor::CircleCross => Box::new(ExtremityFactoryCircleCross::new(bg)),
        LinkDecor::Arrow => Box::new(ExtremityFactoryArrow),
        LinkDecor::ArrowAndCircle => Box::new(ExtremityFactoryArrowAndCircle::new(bg)),
        LinkDecor::NotNavigable => Box::new(ExtremityFactoryNotNavigable),
        LinkDecor::Aggregation => Box::new(ExtremityFactoryDiamond::new(false)),
        LinkDecor::Composition => Box::new(ExtremityFactoryDiamond::new(true)),
        LinkDecor::Circle => Box::new(ExtremityFactoryCircle::new(false, bg)),
        LinkDecor::CircleFill => Box::new(ExtremityFactoryCircle::new(true, bg)),
        LinkDecor::Square => Box::new(ExtremityFactorySquare::new(bg)),
        LinkDecor::Parenthesis => Box::new(ExtremityFactoryParenthesis),
        LinkDecor::CircleConnect => Box::new(ExtremityFactoryCircleConnect::new(bg)),
        _ => return None,
    })
}

/// `LinkMiddleDecor.getMiddleFactory`; `None` has no factory in Java (it throws), so it draws nothing.
pub fn middle_factory(
    decor: LinkMiddleDecor,
    back: &HColor,
    diagram_back: &HColor,
) -> Box<dyn MiddleFactory> {
    let (b, d) = (back.clone(), diagram_back.clone());
    match decor {
        LinkMiddleDecor::Circle => Box::new(MiddleFactoryCircle::new(b)),
        LinkMiddleDecor::CircleCircled => Box::new(MiddleFactoryCircleCircled::new(
            MiddleCircleCircledMode::Both,
            b,
            d,
        )),
        LinkMiddleDecor::CircleCircled1 => Box::new(MiddleFactoryCircleCircled::new(
            MiddleCircleCircledMode::Mode1,
            b,
            d,
        )),
        LinkMiddleDecor::CircleCircled2 => Box::new(MiddleFactoryCircleCircled::new(
            MiddleCircleCircledMode::Mode2,
            b,
            d,
        )),
        LinkMiddleDecor::Subset => Box::new(MiddleFactorySubset::new(false)),
        LinkMiddleDecor::Superset => Box::new(MiddleFactorySubset::new(true)),
        LinkMiddleDecor::None => Box::new(NoMiddle),
    }
}

struct NoMiddle;

impl MiddleFactory for NoMiddle {
    fn create(&self, _angle: f64) -> Box<dyn UDrawable> {
        struct Nothing;
        impl UDrawable for Nothing {
            fn draw(&self, _ug: &UGraphic) {}
        }
        Box::new(Nothing)
    }
}
