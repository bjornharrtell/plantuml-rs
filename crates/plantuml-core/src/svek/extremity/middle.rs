//! `MiddleCircle`, `MiddleCircleCircled`, `MiddleSubset` and their factories.

use super::{MiddleFactory, UDrawable};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, ULine, UStroke, UTranslate};

struct MiddleCircle {
    back: HColor,
}

impl UDrawable for MiddleCircle {
    fn draw(&self, ug: &UGraphic) {
        let r = 6.0;
        ug.apply_back_color(self.back.clone())
            .apply_stroke(UStroke::with_thickness(1.5))
            .apply_translate(UTranslate::new(-r, -r))
            .draw(UEllipse::build(2.0 * r, 2.0 * r));
    }
}

/// `MiddleFactoryCircle`.
pub struct MiddleFactoryCircle {
    back: HColor,
}

impl MiddleFactoryCircle {
    pub fn new(back: HColor) -> Self {
        Self { back }
    }
}

impl MiddleFactory for MiddleFactoryCircle {
    fn create(&self, _angle: f64) -> Box<dyn UDrawable> {
        Box::new(MiddleCircle {
            back: self.back.clone(),
        })
    }
}

/// `MiddleCircleCircledMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddleCircleCircledMode {
    Both,
    Mode1,
    Mode2,
}

struct MiddleCircleCircled {
    angle: f64,
    mode: MiddleCircleCircledMode,
    back: HColor,
    diagram_back: HColor,
}

impl UDrawable for MiddleCircleCircled {
    fn draw(&self, ug: &UGraphic) {
        let (radius1, radius2) = (6.0, 10.0);
        if self.mode == MiddleCircleCircledMode::Both {
            ug.apply_color(self.diagram_back.clone())
                .apply_back_color(self.diagram_back.clone())
                .apply_translate(UTranslate::new(-radius2, -radius2))
                .draw(UEllipse::build(2.0 * radius2, 2.0 * radius2));
        }
        let ug = ug
            .apply_back_color(self.back.clone())
            .apply_stroke(UStroke::with_thickness(1.5));
        let arc = |start: f64| {
            ug.apply_back_color(HColor::TRANSPARENT)
                .apply_translate(UTranslate::new(-radius2, -radius2))
                .draw(UEllipse::arc(2.0 * radius2, 2.0 * radius2, start, 90.0));
        };
        if matches!(
            self.mode,
            MiddleCircleCircledMode::Mode1 | MiddleCircleCircledMode::Both
        ) {
            arc(self.angle);
        }
        if matches!(
            self.mode,
            MiddleCircleCircledMode::Mode2 | MiddleCircleCircledMode::Both
        ) {
            arc(self.angle + 180.0);
        }
        ug.apply_translate(UTranslate::new(-radius1, -radius1))
            .draw(UEllipse::build(2.0 * radius1, 2.0 * radius1));
    }
}

/// `MiddleFactoryCircleCircled`.
pub struct MiddleFactoryCircleCircled {
    mode: MiddleCircleCircledMode,
    back: HColor,
    diagram_back: HColor,
}

impl MiddleFactoryCircleCircled {
    pub fn new(mode: MiddleCircleCircledMode, back: HColor, diagram_back: HColor) -> Self {
        Self {
            mode,
            back,
            diagram_back,
        }
    }
}

impl MiddleFactory for MiddleFactoryCircleCircled {
    fn create(&self, angle: f64) -> Box<dyn UDrawable> {
        Box::new(MiddleCircleCircled {
            angle,
            mode: self.mode,
            back: self.back.clone(),
            diagram_back: self.diagram_back.clone(),
        })
    }
}

struct MiddleSubset {
    angle: f64,
    reverse: bool,
}

impl UDrawable for MiddleSubset {
    fn draw(&self, ug: &UGraphic) {
        let (radius, length) = (6.0, 10.0);
        let ug = ug.apply_stroke(UStroke::with_thickness(1.5));
        let rotate = if self.reverse { -45.0 } else { 135.0 };
        ug.apply_translate(UTranslate::new(-radius, -radius))
            .draw(UEllipse::arc(
                2.0 * radius,
                2.0 * radius,
                self.angle + rotate,
                180.0,
            ));
        let rad = (self.angle + rotate).to_radians();
        let (sin, cos) = (rad.sin(), rad.cos());
        ug.apply_translate(UTranslate::new(radius * cos, radius * -sin))
            .draw(ULine::new(length * sin, length * cos));
        ug.apply_translate(UTranslate::new(radius * -cos, radius * sin))
            .draw(ULine::new(length * sin, length * cos));
    }
}

/// `MiddleFactorySubset`.
pub struct MiddleFactorySubset {
    reverse: bool,
}

impl MiddleFactorySubset {
    pub fn new(reverse: bool) -> Self {
        Self { reverse }
    }
}

impl MiddleFactory for MiddleFactorySubset {
    fn create(&self, angle: f64) -> Box<dyn UDrawable> {
        Box::new(MiddleSubset {
            angle,
            reverse: self.reverse,
        })
    }
}
