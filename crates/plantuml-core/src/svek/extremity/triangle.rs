//! `ExtremityTriangle`, `ExtremityExtends`, `ExtremityExtendsLike`, `ExtremityNotNavigable`
//! and their factories.

use super::{change_back, manage_round, Extremity, ExtremityFactory, Point, UDrawable};
use crate::klimt::color::HColor;
use crate::klimt::drawing::svg::PathSeg;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPath, UPolygon, UStroke, UTranslate,
};
use crate::klimt::geom::Side;
use std::f64::consts::PI;

struct ExtremityTriangle {
    polygon: UPolygon,
    fill: bool,
    background: Option<HColor>,
    contact: Point,
    decoration_length: i32,
}

impl UDrawable for ExtremityTriangle {
    fn draw(&self, ug: &UGraphic) {
        let ug = if let Some(bg) = &self.background {
            ug.apply_back_color(bg.clone())
        } else if self.fill {
            change_back(ug)
        } else {
            ug.clone()
        };
        ug.draw(self.polygon.clone());
    }
}

impl Extremity for ExtremityTriangle {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        f64::from(self.decoration_length)
    }
}

/// `ExtremityFactoryTriangle`.
pub struct ExtremityFactoryTriangle {
    background: Option<HColor>,
    x_wing: i32,
    y_aperture: i32,
    decoration_length: i32,
}

impl ExtremityFactoryTriangle {
    pub fn new(
        background: Option<HColor>,
        x_wing: i32,
        y_aperture: i32,
        decoration_length: i32,
    ) -> Self {
        Self {
            background,
            x_wing,
            y_aperture,
            decoration_length,
        }
    }
}

impl ExtremityFactory for ExtremityFactoryTriangle {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = manage_round(angle - PI / 2.0);
        let (xw, ya) = (f64::from(self.x_wing), f64::from(self.y_aperture));
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(-xw, -ya);
        polygon.add_point(-xw, ya);
        polygon.add_point(0.0, 0.0);
        polygon.rotate(angle + PI / 2.0);
        Box::new(ExtremityTriangle {
            polygon: polygon.translate(p0.0, p0.1),
            fill: false,
            background: self.background.clone(),
            contact: p0,
            decoration_length: self.decoration_length,
        })
    }
}

struct ExtremityExtends {
    polygon: UPolygon,
    fill: HColor,
    contact: Point,
}

impl UDrawable for ExtremityExtends {
    fn draw(&self, ug: &UGraphic) {
        ug.apply_back_color(self.fill.clone())
            .draw(self.polygon.clone());
    }
}

impl Extremity for ExtremityExtends {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        18.0
    }
}

/// `ExtremityFactoryExtends`.
pub struct ExtremityFactoryExtends {
    background: HColor,
}

impl ExtremityFactoryExtends {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactoryExtends {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = manage_round(angle - PI / 2.0);
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(-19.0, -7.0);
        polygon.add_point(-19.0, 7.0);
        polygon.add_point(0.0, 0.0);
        polygon.rotate(angle + PI / 2.0);
        Box::new(ExtremityExtends {
            polygon: polygon.translate(p0.0, p0.1),
            fill: self.background.clone(),
            contact: p0,
        })
    }
}

const XLEN: f64 = -19.0;
const HALF_WIDTH: f64 = 7.0;

/// `ExtremityExtendsLike.Point.rotate` (note the reflection, kept as in Java).
fn rotate_like(x: f64, y: f64, theta: f64) -> (f64, f64) {
    let ct = theta.cos();
    let st = -theta.sin();
    (x * ct - y * st, -x * st - y * ct)
}

enum Decoration {
    Redefines {
        bar_stroke: UStroke,
        pos: UTranslate,
        bar: ULine,
    },
    DefinedBy {
        pos1: UTranslate,
        pos2: UTranslate,
        dot: UEllipse,
    },
}

struct ExtremityExtendsLike {
    trig: UPolygon,
    back: HColor,
    contact: Point,
    decoration: Decoration,
}

impl UDrawable for ExtremityExtendsLike {
    fn draw(&self, ug: &UGraphic) {
        ug.apply_back_color(self.back.clone())
            .draw(self.trig.clone());
        match &self.decoration {
            Decoration::Redefines {
                bar_stroke,
                pos,
                bar,
            } => {
                ug.apply_stroke(*bar_stroke)
                    .apply_translate(*pos)
                    .draw(bar.clone());
            }
            Decoration::DefinedBy { pos1, pos2, dot } => {
                let ug = change_back(ug);
                ug.apply_translate(*pos1).draw(dot.clone());
                ug.apply_translate(*pos2).draw(dot.clone());
            }
        }
    }
}

impl Extremity for ExtremityExtendsLike {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        18.0
    }
}

/// `ExtremityFactoryExtendsLike`.
pub struct ExtremityFactoryExtendsLike {
    background: HColor,
    defined_by: bool,
}

impl ExtremityFactoryExtendsLike {
    pub fn new(background: HColor, defined_by: bool) -> Self {
        Self {
            background,
            defined_by,
        }
    }
}

impl ExtremityFactory for ExtremityFactoryExtendsLike {
    fn create(&self, porig: Point, raw_angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = manage_round(raw_angle);
        let mut trig = UPolygon::default();
        trig.add_point(porig.0, porig.1);
        for y in [-HALF_WIDTH, HALF_WIDTH] {
            let p = rotate_like(XLEN, y, angle);
            trig.add_point(p.0 + porig.0, p.1 + porig.1);
        }
        trig.add_point(porig.0, porig.1);
        let decoration = if self.defined_by {
            let xsuffix = XLEN * 1.3;
            let dothsize = 2.0;
            let dot_pos = |x: f64, y: f64| {
                let p = rotate_like(x, y, raw_angle);
                UTranslate::new(p.0 - dothsize + porig.0, p.1 - dothsize + porig.1)
            };
            let w = HALF_WIDTH - dothsize;
            let s = dothsize + dothsize;
            Decoration::DefinedBy {
                pos1: dot_pos(xsuffix, -w),
                pos2: dot_pos(xsuffix, w),
                dot: UEllipse::build(s, s),
            }
        } else {
            let xsuffix = XLEN * 1.2;
            let p1 = rotate_like(xsuffix, -HALF_WIDTH, raw_angle);
            let p2 = rotate_like(xsuffix, HALF_WIDTH, raw_angle);
            Decoration::Redefines {
                bar_stroke: UStroke::with_thickness(2.0),
                pos: UTranslate::new(p1.0 + porig.0, p1.1 + porig.1),
                bar: ULine::new(p2.0 - p1.0, p2.1 - p1.1),
            }
        };
        Box::new(ExtremityExtendsLike {
            trig,
            back: self.background.clone(),
            contact: porig,
            decoration,
        })
    }
}

struct ExtremityNotNavigable {
    path: UPath,
    contact: Point,
}

/// `UPath.rotate` (only move/line segments are supported, like `USegment.rotate`).
fn rotate_path(path: &UPath, theta: f64) -> UPath {
    let (sin, cos) = theta.sin_cos();
    let r = |x: f64, y: f64| (x * cos - y * sin, x * sin + y * cos);
    let segments = path
        .segments
        .iter()
        .map(|s| match *s {
            PathSeg::MoveTo(x, y) => {
                let p = r(x, y);
                PathSeg::MoveTo(p.0, p.1)
            }
            PathSeg::LineTo(x, y) => {
                let p = r(x, y);
                PathSeg::LineTo(p.0, p.1)
            }
            other => other,
        })
        .collect();
    UPath {
        segments,
        delta_shadow: path.delta_shadow,
    }
}

impl UDrawable for ExtremityNotNavigable {
    fn draw(&self, ug: &UGraphic) {
        ug.draw(self.path.clone());
    }
}

impl Extremity for ExtremityNotNavigable {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
}

/// `ExtremityFactoryNotNavigable`.
pub struct ExtremityFactoryNotNavigable;

impl ExtremityFactory for ExtremityFactoryNotNavigable {
    fn create(&self, p1: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = manage_round(angle - PI / 2.0);
        let (size, mv) = (4.0, 5.0);
        let mut path = UPath::new();
        path.move_to(-size, 0.0);
        path.line_to(size, 2.0 * size);
        path.move_to(size, 0.0);
        path.line_to(-size, 2.0 * size);
        let path = path.translate(0.0, mv);
        let path = rotate_path(&path, angle + PI).translate(p1.0, p1.1);
        Box::new(ExtremityNotNavigable { path, contact: p1 })
    }
}
