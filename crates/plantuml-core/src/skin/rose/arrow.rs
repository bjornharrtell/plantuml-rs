//! Straight message arrow (`ComponentRoseArrow`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, ULine, UPolygon, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::skin::arrow::{
    ArrowConfiguration, ArrowDecoration, ArrowDirection, ArrowDressing, ArrowHead, ArrowPart,
};
use crate::skin::component::{Area, ArrowComponent, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

const ARROW_DELTA_X: f64 = 10.0;
const ARROW_DELTA_Y: f64 = 4.0;
const SPACE_CROSS_X: f64 = 6.0;
const DIAM_CIRCLE: f64 = 8.0;
const THIN_CIRCLE: f64 = 1.5;

/// Shared state of the arrow components (`AbstractComponentRoseArrow`).
pub struct ArrowBase {
    pub core: TextualCore,
    pub foreground: HColor,
    pub background: HColor,
    pub config: ArrowConfiguration,
}

impl ArrowBase {
    pub fn new(
        style: &Style,
        display: &Display,
        config: &ArrowConfiguration,
        skin: Rc<SkinParam>,
        max_message_size: &LineBreakStrategy,
    ) -> Self {
        let core = TextualCore::new(
            style,
            style,
            max_message_size,
            Margins::new(1.0, 7.0, 1.0, 7.0),
            skin,
            display,
        );
        let stroke = core.base.stroke();
        Self {
            foreground: core.base.color_line(),
            background: core.base.color_back_ground(),
            config: config.with_thickness(stroke.thickness),
            core,
        }
    }
}

pub struct ComponentRoseArrow {
    base: ArrowBase,
    style: Style,
    message_position: HorizontalAlignment,
    nice_arrow: bool,
    below_for_response: bool,
    inclination1: i32,
    inclination2: i32,
}

impl ComponentRoseArrow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        style: &Style,
        display: &Display,
        config: &ArrowConfiguration,
        message_position: HorizontalAlignment,
        skin: Rc<SkinParam>,
        max_message_size: &LineBreakStrategy,
        nice_arrow: bool,
        below_for_response: bool,
    ) -> Self {
        Self {
            base: ArrowBase::new(style, display, config, skin, max_message_size),
            style: style.clone(),
            message_position,
            nice_arrow,
            below_for_response,
            inclination1: config.inclination1(),
            inclination2: config.inclination2(),
        }
    }

    fn is_below_for_response(&self) -> bool {
        self.below_for_response && self.base.config.is_reverse_define()
    }

    fn direction(&self) -> ArrowDirection {
        self.base.config.arrow_direction()
    }

    fn polygon_normal(&self, part: ArrowPart) -> UPolygon {
        let mut p = UPolygon::default();
        match part {
            ArrowPart::TopPart => {
                p.add_point(-ARROW_DELTA_X, -ARROW_DELTA_Y);
                p.add_point(0.0, 0.0);
                p.add_point(-ARROW_DELTA_X, 0.0);
            }
            ArrowPart::BottomPart => {
                p.add_point(-ARROW_DELTA_X, 0.0);
                p.add_point(0.0, 0.0);
                p.add_point(-ARROW_DELTA_X, ARROW_DELTA_Y);
            }
            ArrowPart::Full => {
                p.add_point(-ARROW_DELTA_X, -ARROW_DELTA_Y);
                p.add_point(0.0, 0.0);
                p.add_point(-ARROW_DELTA_X, ARROW_DELTA_Y);
                if self.nice_arrow {
                    p.add_point(-ARROW_DELTA_X + 4.0, 0.0);
                }
            }
        }
        p
    }

    fn polygon_reverse(&self, part: ArrowPart) -> UPolygon {
        let mut p = UPolygon::default();
        match part {
            ArrowPart::TopPart => {
                p.add_point(ARROW_DELTA_X, -ARROW_DELTA_Y);
                p.add_point(0.0, 0.0);
                p.add_point(ARROW_DELTA_X, 0.0);
            }
            ArrowPart::BottomPart => {
                p.add_point(ARROW_DELTA_X, 0.0);
                p.add_point(0.0, 0.0);
                p.add_point(ARROW_DELTA_X, ARROW_DELTA_Y);
            }
            ArrowPart::Full => {
                p.add_point(ARROW_DELTA_X, -ARROW_DELTA_Y);
                p.add_point(0.0, 0.0);
                p.add_point(ARROW_DELTA_X, ARROW_DELTA_Y);
                if self.nice_arrow {
                    p.add_point(ARROW_DELTA_X - 4.0, 0.0);
                }
            }
        }
        p
    }

    fn draw_dressing1(
        &self,
        ug: &UGraphic,
        dressing: ArrowDressing,
        decoration: ArrowDecoration,
        len_full: f64,
    ) {
        let mut ug = ug.clone();
        if decoration == ArrowDecoration::Circle {
            let circle = UEllipse::build(DIAM_CIRCLE, DIAM_CIRCLE);
            ug.apply_stroke(UStroke::with_thickness(THIN_CIRCLE))
                .apply_color(self.base.foreground.clone())
                .apply_back_color(self.base.background.clone())
                .apply_translate(UTranslate::new(
                    -DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                    -DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
                ))
                .draw(circle);
            if dressing.head() != ArrowHead::CrossX {
                ug = ug.apply_translate(UTranslate::dx(DIAM_CIRCLE / 2.0 + THIN_CIRCLE));
            }
        }
        let angle = (-f64::from(self.inclination1)).atan2(len_full);
        match dressing.head() {
            ArrowHead::Async => {
                let thin = self.base.config.apply_thickness_only(&ug);
                if dressing.part() != ArrowPart::BottomPart {
                    thin.draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_Y).rotate(angle));
                }
                if dressing.part() != ArrowPart::TopPart {
                    thin.draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_Y).rotate(angle));
                }
            }
            ArrowHead::CrossX => {
                let ug = ug.apply_stroke(UStroke::with_thickness(2.0));
                ug.apply_translate(UTranslate::new(SPACE_CROSS_X, -ARROW_DELTA_X / 2.0))
                    .draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_X));
                ug.apply_translate(UTranslate::new(SPACE_CROSS_X, ARROW_DELTA_X / 2.0))
                    .draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_X));
            }
            ArrowHead::Normal => {
                let mut polygon = self.polygon_reverse(dressing.part());
                if self.inclination1 != 0 {
                    polygon.rotate(angle);
                }
                ug.apply_back_color(self.base.foreground.clone())
                    .draw(polygon);
            }
            ArrowHead::None => {}
        }
    }

    fn draw_dressing2(
        &self,
        ug: &UGraphic,
        dressing: ArrowDressing,
        decoration: ArrowDecoration,
        len_full: f64,
    ) {
        let mut ug = ug.clone();
        if decoration == ArrowDecoration::Circle {
            ug = ug
                .apply_stroke(UStroke::with_thickness(THIN_CIRCLE))
                .apply_color(self.base.foreground.clone())
                .apply_back_color(self.base.background.clone());
            let circle = UEllipse::build(DIAM_CIRCLE, DIAM_CIRCLE);
            ug.apply_translate(UTranslate::new(
                -DIAM_CIRCLE / 2.0 + THIN_CIRCLE,
                -DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            ))
            .draw(circle);
            ug = ug.apply_stroke(UStroke::simple());
            ug = ug.apply_translate(UTranslate::dx(-DIAM_CIRCLE / 2.0 - THIN_CIRCLE));
        }
        let angle = f64::from(self.inclination2).atan2(len_full);
        match dressing.head() {
            ArrowHead::Async => {
                let thin = self.base.config.apply_thickness_only(&ug);
                if dressing.part() != ArrowPart::BottomPart {
                    thin.draw(ULine::new(-ARROW_DELTA_X, -ARROW_DELTA_Y).rotate(angle));
                }
                if dressing.part() != ArrowPart::TopPart {
                    thin.draw(ULine::new(-ARROW_DELTA_X, ARROW_DELTA_Y).rotate(angle));
                }
            }
            ArrowHead::CrossX => {
                let ug = ug.apply_stroke(UStroke::with_thickness(2.0));
                ug.apply_translate(UTranslate::new(
                    -SPACE_CROSS_X - ARROW_DELTA_X,
                    -ARROW_DELTA_X / 2.0,
                ))
                .draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_X));
                ug.apply_translate(UTranslate::new(
                    -SPACE_CROSS_X - ARROW_DELTA_X,
                    ARROW_DELTA_X / 2.0,
                ))
                .draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_X));
            }
            ArrowHead::Normal => {
                let mut polygon = self.polygon_normal(dressing.part());
                if self.inclination2 != 0 {
                    polygon.rotate(angle);
                }
                ug.apply_back_color(self.base.foreground.clone())
                    .draw(polygon);
            }
            ArrowHead::None => {}
        }
    }
}

impl Component for ComponentRoseArrow {
    fn padding_y(&self) -> f64 {
        4.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let config = &self.base.config;
        if config.is_hidden() {
            return;
        }
        let dim = area.dimension_to_use;
        let sb = ug.string_bounder();
        let ug = ug.apply_color(self.base.foreground.clone());
        let dressing1 = config.dressing1();
        let dressing2 = config.dressing2();
        let mut start = 0.0;
        let mut len = dim.width - 1.0;
        let len_full = dim.width;
        let pos1 = start + 1.0;
        let pos2 = len - 1.0;
        if config.decoration2() == ArrowDecoration::Circle {
            if dressing2.head() == ArrowHead::None {
                len -= DIAM_CIRCLE / 2.0;
            } else {
                len -= DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
            }
        }
        if config.decoration1() == ArrowDecoration::Circle {
            match dressing1.head() {
                ArrowHead::None => {
                    start += DIAM_CIRCLE / 2.0;
                    len -= DIAM_CIRCLE / 2.0;
                }
                ArrowHead::Async | ArrowHead::Normal => {
                    start += DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
                    len -= DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
                }
                ArrowHead::CrossX => {}
            }
        }
        if dressing2.part() == ArrowPart::Full && dressing2.head() == ArrowHead::Normal {
            len -= (ARROW_DELTA_X as i32 / 2) as f64;
        }
        if dressing1.part() == ArrowPart::Full && dressing1.head() == ArrowHead::Normal {
            start += (ARROW_DELTA_X as i32 / 2) as f64;
            len -= (ARROW_DELTA_X as i32 / 2) as f64;
        }
        if dressing2.head() == ArrowHead::CrossX {
            len -= 2.0 * SPACE_CROSS_X;
        }
        if dressing1.head() == ArrowHead::CrossX {
            start += 2.0 * SPACE_CROSS_X;
            len -= 2.0 * SPACE_CROSS_X;
        }
        let (pos_arrow, y_text) = if self.is_below_for_response() {
            (0.0, self.base.core.padding().top)
        } else {
            (self.base.core.text_height(sb), 0.0)
        };
        let i1 = f64::from(self.inclination1);
        let i2 = f64::from(self.inclination2);
        self.draw_dressing1(
            &ug.apply_translate(UTranslate::new(pos1, pos_arrow + i1)),
            dressing1,
            config.decoration1(),
            len_full,
        );
        self.draw_dressing2(
            &ug.apply_translate(UTranslate::new(pos2, pos_arrow + i2)),
            dressing2,
            config.decoration2(),
            len_full,
        );
        let body = config.apply_stroke(&ug, &self.style);
        if self.inclination1 == 0 && self.inclination2 == 0 {
            body.apply_translate(UTranslate::new(start, pos_arrow))
                .draw(ULine::new(len, 0.0));
        } else if self.inclination1 != 0 {
            draw_line(&body, start + len, pos_arrow, 0.0, pos_arrow + i1);
        } else if self.inclination2 != 0 {
            draw_line(&body, start, pos_arrow, pos2, pos_arrow + i2);
        }
        let direction2 = self.direction();
        let text_width = self.base.core.text_block().calculate_dimension(sb).width;
        let padding = self.base.core.padding();
        let text_pos = match self.message_position {
            HorizontalAlignment::Center => (dim.width - area.text_delta_x.abs() - text_width) / 2.0,
            HorizontalAlignment::Right => {
                dim.width
                    - area.text_delta_x.abs()
                    - text_width
                    - padding.right
                    - if direction2 == ArrowDirection::LeftToRightNormal {
                        ARROW_DELTA_X
                    } else {
                        0.0
                    }
            }
            HorizontalAlignment::Left => {
                padding.left
                    + if matches!(
                        direction2,
                        ArrowDirection::RightToLeftReverse | ArrowDirection::BothDirection
                    ) {
                        ARROW_DELTA_X
                    } else {
                        0.0
                    }
            }
        };
        self.base
            .core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(
                text_pos + area.text_delta_x.max(0.0),
                y_text,
            )));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.base.core.text_height(sb)
            + ARROW_DELTA_Y
            + 2.0 * self.padding_y()
            + f64::from(self.inclination1)
            + f64::from(self.inclination2)
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.base.core.text_width(sb) + ARROW_DELTA_X
    }
}

fn draw_line(ug: &UGraphic, x1: f64, y1: f64, x2: f64, y2: f64) {
    ug.apply_translate(UTranslate::new(x1, y1))
        .draw(ULine::new(x2 - x1, y2 - y1));
}

impl ArrowComponent for ComponentRoseArrow {
    fn start_point(&self, sb: &dyn StringBounder, dim: Dimension) -> (f64, f64) {
        let y = self.y_point(sb);
        let i2 = f64::from(self.inclination2);
        if self.direction() == ArrowDirection::LeftToRightNormal {
            (self.padding_x(), y + i2)
        } else {
            (dim.width + self.padding_x(), y + i2)
        }
    }

    fn end_point(&self, sb: &dyn StringBounder, dim: Dimension) -> (f64, f64) {
        let y = self.y_point(sb);
        if self.direction() == ArrowDirection::LeftToRightNormal {
            (dim.width + self.padding_x(), y)
        } else {
            (self.padding_x(), y)
        }
    }

    fn y_point(&self, sb: &dyn StringBounder) -> f64 {
        if self.is_below_for_response() {
            return self.padding_y();
        }
        self.base.core.text_height(sb) + self.padding_y()
    }

    fn pos_arrow(&self, sb: &dyn StringBounder) -> f64 {
        if self.is_below_for_response() {
            return 0.0;
        }
        self.base.core.text_height(sb) - 2.0 * self.base.core.padding().top
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::SvgOption;
    use crate::klimt::drawing::ugraphic::render_svg;
    use crate::klimt::font::EmbeddedFontBounder;
    use crate::skin::arrow::ArrowBody;
    use crate::style::{SName, StyleQuery};

    fn arrow(config: ArrowConfiguration, label: &str) -> (ComponentRoseArrow, Rc<SkinParam>) {
        let skin = Rc::new(SkinParam::new());
        let style = skin
            .style_builder()
            .merged_style(&crate::style::queries::SEQUENCEDIAG_ARROW)
            .unwrap();
        let _ = (SName::Root, StyleQuery::empty());
        let c = ComponentRoseArrow::new(
            &style,
            &Display::create(&[label]),
            &config,
            HorizontalAlignment::Left,
            skin.clone(),
            &LineBreakStrategy::NONE,
            true,
            false,
        );
        (c, skin)
    }

    fn draw(c: &ComponentRoseArrow, w: f64) -> String {
        render_svg(
            SvgOption::default(),
            0,
            Rc::new(EmbeddedFontBounder),
            |ug| {
                let area = Area::create(w, c.preferred_height(ug.string_bounder()));
                c.draw(ug, &area, false);
            },
        )
    }

    #[test]
    fn plain_arrow_dimensions_and_drawing() {
        let (c, _) = arrow(ArrowConfiguration::with_direction_normal(), "hi");
        let sb = EmbeddedFontBounder;
        assert!((c.preferred_height(&sb) - 30.3515625).abs() < 1e-6);
        assert_eq!(c.padding_y(), 4.0);
        let out = draw(&c, 100.0);
        assert!(out.contains("<polygon"), "{out}");
        assert!(out.contains(">hi</text>"), "{out}");
        assert!(out.contains("<line"), "{out}");
        let (sx, sy) = c.start_point(&sb, Dimension::new(50.0, 10.0));
        let (ex, ey) = c.end_point(&sb, Dimension::new(50.0, 10.0));
        assert!(sx < ex && sy == ey - 0.0 || sy >= 0.0);
        assert!(c.y_point(&sb) > 0.0 && c.pos_arrow(&sb) > 0.0);
    }

    #[test]
    fn variants_draw_without_panicking() {
        let base = ArrowConfiguration::with_direction_normal();
        for cfg in [
            base.clone(),
            base.with_body(ArrowBody::Dotted),
            base.with_body(ArrowBody::Hidden),
            base.with_head(ArrowHead::Async),
            base.with_head2(ArrowHead::CrossX),
            base.with_decoration1(ArrowDecoration::Circle),
            base.with_decoration2(ArrowDecoration::Circle),
            base.with_part(ArrowPart::TopPart),
            base.with_part(ArrowPart::BottomPart),
            base.with_inclination(5),
            ArrowConfiguration::with_direction_reverse(),
            ArrowConfiguration::with_direction_reverse().with_inclination(4),
            ArrowConfiguration::with_direction_reverse().with_head(ArrowHead::Async),
            ArrowConfiguration::with_direction_reverse().with_head1(ArrowHead::CrossX),
            ArrowConfiguration::with_direction_reverse().with_decoration1(ArrowDecoration::Circle),
            ArrowConfiguration::with_direction_both(),
            ArrowConfiguration::with_direction_both()
                .with_head1(ArrowHead::Async)
                .with_decoration2(ArrowDecoration::Circle),
        ] {
            let (c, _) = arrow(cfg, "x");
            let out = draw(&c, 120.0);
            assert!(out.contains("<svg"), "{out}");
        }
    }

    #[test]
    fn text_alignment_and_response() {
        let skin = Rc::new(SkinParam::new());
        let style = skin
            .style_builder()
            .merged_style(&crate::style::queries::SEQUENCEDIAG_ARROW)
            .unwrap();
        for align in [
            HorizontalAlignment::Left,
            HorizontalAlignment::Center,
            HorizontalAlignment::Right,
        ] {
            let c = ComponentRoseArrow::new(
                &style,
                &Display::create(&["label"]),
                &ArrowConfiguration::with_direction_reverse().reverse_define(),
                align,
                skin.clone(),
                &LineBreakStrategy::NONE,
                false,
                true,
            );
            assert_eq!(c.pos_arrow(&EmbeddedFontBounder), 0.0);
            assert_eq!(c.y_point(&EmbeddedFontBounder), 4.0);
            assert!(draw(&c, 150.0).contains(">label</text>"));
        }
    }
}
