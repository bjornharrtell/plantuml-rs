//! Circle-like extremities (`ExtremityCircle*`, `ExtremityParenthesis`, `ExtremitySquare`,
//! `ExtremityPlus`) and their factories.

use super::{
    change_back, draw_line, manage_round, rotate_point, Extremity, ExtremityFactory, Point,
    UDrawable,
};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, URectangle, UStroke, UTranslate};
use crate::klimt::geom::Side;
use std::f64::consts::PI;

struct ExtremityCircle {
    dest: Point,
    fill: bool,
    background: HColor,
}

const CIRCLE_RADIUS: f64 = 6.0;

impl UDrawable for ExtremityCircle {
    fn draw(&self, ug: &UGraphic) {
        let mut ug = ug.apply_stroke(UStroke::with_thickness(1.5));
        if self.fill {
            ug = change_back(&ug);
        } else {
            ug = ug.apply_back_color(self.background.clone());
        }
        let r = CIRCLE_RADIUS;
        ug.apply_translate(UTranslate::new(self.dest.0 - r, self.dest.1 - r))
            .draw(UEllipse::build(r * 2.0, r * 2.0));
    }
}

impl Extremity for ExtremityCircle {
    fn some_point(&self) -> Option<Point> {
        Some(self.dest)
    }
    fn decoration_length(&self) -> f64 {
        12.0
    }
}

/// `ExtremityFactoryCircle`.
pub struct ExtremityFactoryCircle {
    fill: bool,
    background: HColor,
}

impl ExtremityFactoryCircle {
    pub fn new(fill: bool, background: HColor) -> Self {
        Self { fill, background }
    }
}

impl ExtremityFactory for ExtremityFactoryCircle {
    fn create(&self, center: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        let dest = (
            center.0 - CIRCLE_RADIUS * (angle + PI / 2.0).cos(),
            center.1 - CIRCLE_RADIUS * (angle + PI / 2.0).sin(),
        );
        Box::new(ExtremityCircle {
            dest,
            fill: self.fill,
            background: self.background.clone(),
        })
    }
}

struct ExtremityCircleConnect {
    dest: Point,
    ortho: f64,
    background: HColor,
}

impl UDrawable for ExtremityCircleConnect {
    fn draw(&self, ug: &UGraphic) {
        let (radius, radius2) = (6.0, 10.0);
        let ug = ug
            .apply_stroke(UStroke::with_thickness(1.5))
            .apply_back_color(self.background.clone());
        ug.apply_translate(UTranslate::new(self.dest.0 - radius, self.dest.1 - radius))
            .draw(UEllipse::build(radius * 2.0, radius * 2.0));
        let deg = -self.ortho * 180.0 / PI + 90.0 - 45.0;
        ug.apply_translate(UTranslate::new(
            self.dest.0 - radius2,
            self.dest.1 - radius2,
        ))
        .draw(UEllipse::arc(2.0 * radius2, 2.0 * radius2, deg, 90.0));
    }
}

impl Extremity for ExtremityCircleConnect {
    fn some_point(&self) -> Option<Point> {
        Some(self.dest)
    }
    fn decoration_length(&self) -> f64 {
        10.0
    }
}

/// `ExtremityFactoryCircleConnect`.
pub struct ExtremityFactoryCircleConnect {
    background: HColor,
}

impl ExtremityFactoryCircleConnect {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactoryCircleConnect {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleConnect {
            dest: p0,
            ortho: angle - PI / 2.0,
            background: self.background.clone(),
        })
    }
}

struct ExtremityCircleCross {
    px: f64,
    py: f64,
    dest: Point,
    background: HColor,
}

const CROSS_RADIUS: f64 = 7.0;

impl ExtremityCircleCross {
    fn point_on_circle(&self, angle: f64) -> Point {
        (
            self.px + CROSS_RADIUS + CROSS_RADIUS * angle.cos(),
            self.py + CROSS_RADIUS + CROSS_RADIUS * angle.sin(),
        )
    }
}

impl UDrawable for ExtremityCircleCross {
    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_back_color(self.background.clone());
        let r = CROSS_RADIUS;
        ug.apply_stroke(UStroke::with_thickness(1.5))
            .apply_translate(UTranslate::new(self.dest.0 - r, self.dest.1 - r))
            .draw(UEllipse::build(r * 2.0, r * 2.0));
        draw_line(
            &ug,
            0.0,
            0.0,
            self.point_on_circle(PI / 4.0),
            self.point_on_circle(PI + PI / 4.0),
        );
        draw_line(
            &ug,
            0.0,
            0.0,
            self.point_on_circle(-PI / 4.0),
            self.point_on_circle(PI - PI / 4.0),
        );
    }
}

impl Extremity for ExtremityCircleCross {
    fn some_point(&self) -> Option<Point> {
        Some(self.dest)
    }
}

/// `ExtremityFactoryCircleCross`.
pub struct ExtremityFactoryCircleCross {
    background: HColor,
}

impl ExtremityFactoryCircleCross {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactoryCircleCross {
    fn create(&self, p0: Point, _angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityCircleCross {
            px: p0.0 - CROSS_RADIUS,
            py: p0.1 - CROSS_RADIUS,
            dest: p0,
            background: self.background.clone(),
        })
    }
}

struct ExtremityCircleCrowfoot {
    contact: Point,
    angle: f64,
}

impl UDrawable for ExtremityCircleCrowfoot {
    fn draw(&self, ug: &UGraphic) {
        let radius = 4.0;
        let (x_wing, y_aperture) = (8.0, 6.0);
        let middle = (0.0, 0.0);
        let left = rotate_point((0.0, -y_aperture), self.angle);
        let base = rotate_point((-x_wing, 0.0), self.angle);
        let right = rotate_point((0.0, y_aperture), self.angle);
        let circle_base = rotate_point((-x_wing - radius - 2.0, 0.0), self.angle);
        let (cx, cy) = self.contact;
        draw_line(ug, cx, cy, base, left);
        draw_line(ug, cx, cy, base, right);
        draw_line(ug, cx, cy, base, middle);
        ug.apply_translate(UTranslate::new(
            cx + circle_base.0 - radius,
            cy + circle_base.1 - radius,
        ))
        .draw(UEllipse::build(2.0 * radius, 2.0 * radius));
    }
}

impl Extremity for ExtremityCircleCrowfoot {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        18.0
    }
}

/// `ExtremityFactoryCircleCrowfoot`.
pub struct ExtremityFactoryCircleCrowfoot;

impl ExtremityFactory for ExtremityFactoryCircleCrowfoot {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityCircleCrowfoot {
            contact: p0,
            angle: manage_round(angle + PI / 2.0),
        })
    }
}

struct ExtremityCircleLine {
    contact: Point,
    angle: f64,
}

impl UDrawable for ExtremityCircleLine {
    fn draw(&self, ug: &UGraphic) {
        let thickness = ug.stroke().thickness;
        let radius = 4.0 + thickness - 1.0;
        let line_height = 4.0 + thickness - 1.0;
        let x_wing = 4.0;
        let middle = (0.0, 0.0);
        let base = rotate_point((-x_wing - radius - 3.0, 0.0), self.angle);
        let circle_base = rotate_point((-x_wing - radius - 3.0, 0.0), self.angle);
        let line_top = rotate_point((-x_wing, -line_height), self.angle);
        let line_bottom = rotate_point((-x_wing, line_height), self.angle);
        let (cx, cy) = self.contact;
        draw_line(ug, cx, cy, base, middle);
        let stroke = UStroke::with_thickness(thickness);
        ug.apply_translate(UTranslate::new(
            cx + circle_base.0 - radius,
            cy + circle_base.1 - radius,
        ))
        .apply_stroke(stroke)
        .draw(UEllipse::build(2.0 * radius, 2.0 * radius));
        draw_line(&ug.apply_stroke(stroke), cx, cy, line_top, line_bottom);
    }
}

impl Extremity for ExtremityCircleLine {
    fn some_point(&self) -> Option<Point> {
        Some(self.contact)
    }
    fn decoration_length(&self) -> f64 {
        15.0
    }
}

/// `ExtremityFactoryCircleLine`.
pub struct ExtremityFactoryCircleLine;

impl ExtremityFactory for ExtremityFactoryCircleLine {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityCircleLine {
            contact: p0,
            angle: manage_round(angle + PI / 2.0),
        })
    }
}

struct ExtremityParenthesis {
    dest: Point,
    ortho: f64,
}

impl UDrawable for ExtremityParenthesis {
    fn draw(&self, ug: &UGraphic) {
        let radius2 = 9.0;
        let ang = 70.0;
        let deg = -self.ortho * 180.0 / PI + 90.0 - ang;
        ug.apply_stroke(UStroke::with_thickness(1.5))
            .apply_translate(UTranslate::new(
                self.dest.0 - radius2,
                self.dest.1 - radius2,
            ))
            .draw(UEllipse::arc(2.0 * radius2, 2.0 * radius2, deg, 2.0 * ang));
    }
}

impl Extremity for ExtremityParenthesis {
    fn some_point(&self) -> Option<Point> {
        Some(self.dest)
    }
    fn decoration_length(&self) -> f64 {
        10.0
    }
}

/// `ExtremityFactoryParenthesis`.
pub struct ExtremityFactoryParenthesis;

impl ExtremityFactory for ExtremityFactoryParenthesis {
    fn create(&self, p0: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremityParenthesis {
            dest: p0,
            ortho: angle - PI / 2.0,
        })
    }
}

struct ExtremitySquare {
    dest: Point,
    background: HColor,
}

impl UDrawable for ExtremitySquare {
    fn draw(&self, ug: &UGraphic) {
        let r = 5.0;
        ug.apply_stroke(UStroke::with_thickness(1.5))
            .apply_back_color(self.background.clone())
            .apply_translate(UTranslate::new(self.dest.0 - r, self.dest.1 - r))
            .draw(URectangle::build(r * 2.0, r * 2.0));
    }
}

impl Extremity for ExtremitySquare {
    fn some_point(&self) -> Option<Point> {
        Some(self.dest)
    }
    fn decoration_length(&self) -> f64 {
        5.0
    }
}

/// `ExtremityFactorySquare`.
pub struct ExtremityFactorySquare {
    background: HColor,
}

impl ExtremityFactorySquare {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactorySquare {
    fn create(&self, p0: Point, _angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        Box::new(ExtremitySquare {
            dest: p0,
            background: self.background.clone(),
        })
    }
}

const PLUS_RADIUS: f64 = 8.0;

struct ExtremityPlus {
    px: f64,
    py: f64,
    angle: f64,
    background: HColor,
}

impl ExtremityPlus {
    fn point_on_circle(&self, angle: f64) -> Point {
        (
            self.px + PLUS_RADIUS + PLUS_RADIUS * angle.cos(),
            self.py + PLUS_RADIUS + PLUS_RADIUS * angle.sin(),
        )
    }
}

impl UDrawable for ExtremityPlus {
    fn draw(&self, ug: &UGraphic) {
        ug.apply_back_color(self.background.clone())
            .apply_translate(UTranslate::new(self.px, self.py))
            .draw(UEllipse::build(2.0 * PLUS_RADIUS, 2.0 * PLUS_RADIUS));
        let a = self.angle;
        draw_line(
            ug,
            0.0,
            0.0,
            self.point_on_circle(a - PI / 2.0),
            self.point_on_circle(a + PI / 2.0),
        );
        draw_line(
            ug,
            0.0,
            0.0,
            self.point_on_circle(a),
            self.point_on_circle(a + PI),
        );
    }
}

impl Extremity for ExtremityPlus {
    fn some_point(&self) -> Option<Point> {
        Some((self.px, self.py))
    }
    fn decoration_length(&self) -> f64 {
        16.0
    }
}

/// `ExtremityFactoryPlus`.
pub struct ExtremityFactoryPlus {
    background: HColor,
}

impl ExtremityFactoryPlus {
    pub fn new(background: HColor) -> Self {
        Self { background }
    }
}

impl ExtremityFactory for ExtremityFactoryPlus {
    fn create(&self, center: Point, angle: f64, _side: Option<Side>) -> Box<dyn Extremity> {
        let angle = angle - PI / 2.0;
        Box::new(ExtremityPlus {
            px: center.0 - PLUS_RADIUS + PLUS_RADIUS * angle.sin(),
            py: center.1 - PLUS_RADIUS - PLUS_RADIUS * angle.cos(),
            angle,
            background: self.background.clone(),
        })
    }
}
