//! Stick-man figures of actors (`ActorStyle`, `ActorStickMan`, `ActorAwesome`, `ActorHollow`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, ULine, UPath, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStyle {
    Stickman,
    StickmanBusiness,
    Awesome,
    Hollow,
}

impl ActorStyle {
    pub fn text_block(self, fashion: Fashion) -> Rc<dyn TextBlock> {
        match self {
            ActorStyle::Stickman => Rc::new(ActorStickMan {
                fashion,
                business: false,
            }),
            ActorStyle::StickmanBusiness => Rc::new(ActorStickMan {
                fashion,
                business: true,
            }),
            ActorStyle::Awesome => Rc::new(ActorAwesome { fashion }),
            ActorStyle::Hollow => Rc::new(ActorHollow { fashion }),
        }
    }
}

fn thickness(fashion: &Fashion) -> f64 {
    fashion.stroke().thickness
}

pub struct ActorStickMan {
    fashion: Fashion,
    business: bool,
}

impl ActorStickMan {
    const ARMS_Y: f64 = 8.0;
    const ARMS_LENGTH: f64 = 13.0;
    const BODY_LENGTH: f64 = 27.0;
    const LEGS_X: f64 = 13.0;
    const LEGS_Y: f64 = 15.0;
    const HEAD_DIAM: f64 = 16.0;

    fn on_circle(alpha: f64) -> (f64, f64) {
        (
            Self::HEAD_DIAM / 2.0 * alpha.cos(),
            Self::HEAD_DIAM / 2.0 * alpha.sin(),
        )
    }
}

impl TextBlock for ActorStickMan {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        let t = thickness(&self.fashion);
        Dimension::new(
            Self::ARMS_LENGTH.max(Self::LEGS_X) * 2.0 + 2.0 * t,
            Self::HEAD_DIAM
                + Self::BODY_LENGTH
                + Self::LEGS_Y
                + 2.0 * t
                + self.fashion.delta_shadow()
                + 1.0,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let t = thickness(&self.fashion);
        let start_x = Self::ARMS_LENGTH.max(Self::LEGS_X) - Self::HEAD_DIAM / 2.0 + t;
        let mut head = UEllipse::build(Self::HEAD_DIAM, Self::HEAD_DIAM);
        let center_x = start_x + Self::HEAD_DIAM / 2.0;
        let mut path = UPath::new();
        path.move_to(0.0, 0.0);
        path.line_to(0.0, Self::BODY_LENGTH);
        path.move_to(-Self::ARMS_LENGTH, Self::ARMS_Y);
        path.line_to(Self::ARMS_LENGTH, Self::ARMS_Y);
        path.move_to(0.0, Self::BODY_LENGTH);
        path.line_to(-Self::LEGS_X, Self::BODY_LENGTH + Self::LEGS_Y);
        path.move_to(0.0, Self::BODY_LENGTH);
        path.line_to(Self::LEGS_X, Self::BODY_LENGTH + Self::LEGS_Y);
        if self.fashion.delta_shadow() != 0.0 {
            head.delta_shadow = self.fashion.delta_shadow();
            path.delta_shadow = self.fashion.delta_shadow();
        }
        let ug = self.fashion.apply(ug);
        ug.apply_translate(UTranslate::new(start_x, t)).draw(head);
        if self.business {
            let ug = ug.apply_translate(UTranslate::new(
                start_x + Self::HEAD_DIAM / 2.0,
                t + Self::HEAD_DIAM / 2.0,
            ));
            let alpha = 21.0 * std::f64::consts::PI / 64.0;
            let p1 = Self::on_circle(std::f64::consts::PI / 4.0 + alpha);
            let p2 = Self::on_circle(std::f64::consts::PI / 4.0 - alpha);
            ug.apply_translate(UTranslate::new(p1.0, p1.1))
                .draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
        }
        ug.apply_translate(UTranslate::new(center_x, Self::HEAD_DIAM + t))
            .apply_back_color(HColor::TRANSPARENT)
            .draw(path);
    }
}

pub struct ActorAwesome {
    fashion: Fashion,
}

impl ActorAwesome {
    const HEAD_DIAM: f64 = 32.0;
    const BODY_WIDTH: f64 = 54.0;
    const SHOULDER: f64 = 16.0;
    const COLLAR: f64 = 4.0;
    const RADIUS: f64 = 8.0;
    const BODY_HEIGHT: f64 = 28.0;

    fn width(&self) -> f64 {
        Self::BODY_WIDTH + thickness(&self.fashion) * 2.0
    }
}

impl TextBlock for ActorAwesome {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(
            self.width(),
            Self::HEAD_DIAM + Self::BODY_HEIGHT + thickness(&self.fashion) * 2.0,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let (bw, sh, co, r, bh) = (
            Self::BODY_WIDTH,
            Self::SHOULDER,
            Self::COLLAR,
            Self::RADIUS,
            Self::BODY_HEIGHT,
        );
        let mut head = UEllipse::build(Self::HEAD_DIAM, Self::HEAD_DIAM);
        let center_x = self.width() / 2.0;
        let mut path = UPath::new();
        path.move_to(0.0, co);
        path.cubic_to(co, co, bw / 2.0 - sh - co, co, bw / 2.0 - sh, 0.0);
        path.cubic_to(bw / 2.0 - sh / 2.0, 0.0, bw / 2.0, sh / 2.0, bw / 2.0, sh);
        path.line_to(bw / 2.0, bh - r);
        path.cubic_to(
            bw / 2.0,
            bh - r / 2.0,
            bw / 2.0 - r / 2.0,
            bh,
            bw / 2.0 - r,
            bh,
        );
        path.line_to(-bw / 2.0 + r, bh);
        path.cubic_to(
            -bw / 2.0 + r / 2.0,
            bh,
            -bw / 2.0,
            bh - r / 2.0,
            -bw / 2.0,
            bh - r,
        );
        path.line_to(-bw / 2.0, sh);
        path.cubic_to(
            -bw / 2.0,
            sh / 2.0,
            -bw / 2.0 + sh / 2.0,
            0.0,
            -bw / 2.0 + sh,
            0.0,
        );
        path.cubic_to(-bw / 2.0 + sh + co, co, -co, co, 0.0, co);
        path.close_path();
        if self.fashion.delta_shadow() != 0.0 {
            head.delta_shadow = self.fashion.delta_shadow();
            path.delta_shadow = self.fashion.delta_shadow();
        }
        let t = thickness(&self.fashion);
        let head_height = head.height;
        let head_width = head.width;
        let ug = self.fashion.apply(ug);
        ug.apply_translate(UTranslate::new(center_x - head_width / 2.0, t))
            .draw(head);
        ug.apply_translate(UTranslate::new(center_x, head_height + t))
            .draw(path);
    }
}

pub struct ActorHollow {
    fashion: Fashion,
}

impl ActorHollow {
    const HEAD_DIAM: f64 = 9.0;
    const BODY_WIDTH: f64 = 25.0;
    const BODY_HEIGHT: f64 = 21.0;
    const NECK_HEIGHT: f64 = 2.0;
    const ARM_THICKNESS: f64 = 5.0;
    const BODY_THICKNESS: f64 = 6.0;
    const LEG_THICKNESS: f64 = 6.0;

    fn width(&self) -> f64 {
        Self::BODY_WIDTH + thickness(&self.fashion) * 2.0
    }
}

impl TextBlock for ActorHollow {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(
            self.width(),
            Self::HEAD_DIAM
                + Self::NECK_HEIGHT
                + Self::BODY_HEIGHT
                + thickness(&self.fashion) * 2.0
                + self.fashion.delta_shadow(),
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let (bw, bh, bt, lt, at) = (
            Self::BODY_WIDTH,
            Self::BODY_HEIGHT,
            Self::BODY_THICKNESS,
            Self::LEG_THICKNESS,
            Self::ARM_THICKNESS,
        );
        let sqrt2 = 2.0_f64.sqrt();
        let mut head = UEllipse::build(Self::HEAD_DIAM, Self::HEAD_DIAM);
        let center_x = self.width() / 2.0;
        let mut path = UPath::new();
        path.move_to(-bw / 2.0, 0.0);
        path.line_to(-bw / 2.0, at);
        path.line_to(-bt / 2.0, at);
        path.line_to(-bt / 2.0, bh - (bw + lt * sqrt2 - bt) / 2.0);
        path.line_to(-bw / 2.0, bh - lt * sqrt2 / 2.0);
        path.line_to(-(bw / 2.0 - lt * sqrt2 / 2.0), bh);
        path.line_to(0.0, bh - (bw / 2.0 - lt * sqrt2 / 2.0));
        path.line_to(bw / 2.0 - lt * sqrt2 / 2.0, bh);
        path.line_to(bw / 2.0, bh - lt * sqrt2 / 2.0);
        path.line_to(bt / 2.0, bh - (bw + lt * sqrt2 - bt) / 2.0);
        path.line_to(bt / 2.0, at);
        path.line_to(bw / 2.0, at);
        path.line_to(bw / 2.0, 0.0);
        path.line_to(-bw / 2.0, 0.0);
        path.close_path();
        if self.fashion.delta_shadow() != 0.0 {
            head.delta_shadow = self.fashion.delta_shadow();
            path.delta_shadow = self.fashion.delta_shadow();
        }
        let t = thickness(&self.fashion);
        let (head_height, head_width) = (head.height, head.width);
        let ug = self.fashion.apply(ug);
        ug.apply_translate(UTranslate::new(center_x - head_width / 2.0, t))
            .draw(head);
        ug.apply_translate(UTranslate::new(
            center_x,
            head_height + t + Self::NECK_HEIGHT,
        ))
        .draw(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::SvgOption;
    use crate::klimt::drawing::ugraphic::{render_svg, UStroke};
    use crate::klimt::font::EmbeddedFontBounder;

    #[test]
    fn styles_have_dimensions_and_draw() {
        let sb = EmbeddedFontBounder;
        for style in [
            ActorStyle::Stickman,
            ActorStyle::StickmanBusiness,
            ActorStyle::Awesome,
            ActorStyle::Hollow,
        ] {
            for shadow in [0.0, 3.0] {
                let fashion = Fashion::new(HColor::parse("red"), HColor::parse("blue"))
                    .with_stroke(UStroke::with_thickness(1.5))
                    .with_delta_shadow(shadow);
                let block = style.text_block(fashion);
                let dim = block.calculate_dimension(&sb);
                assert!(dim.width > 20.0 && dim.height > 30.0);
                let out = render_svg(
                    SvgOption::default(),
                    0,
                    Rc::new(EmbeddedFontBounder),
                    |ug| block.draw(ug),
                );
                assert!(out.contains("<ellipse"), "{out}");
            }
        }
    }
}
