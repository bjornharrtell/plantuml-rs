//! Message from a participant to itself (`ComponentRoseSelfArrow`).

use super::arrow::{
    ArrowBase, ARROW_DELTA_X, ARROW_DELTA_Y, DIAM_CIRCLE, SPACE_CROSS_X, THIN_CIRCLE,
};
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, ULine, UPolygon, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::skin::arrow::{ArrowConfiguration, ArrowDecoration, ArrowHead, ArrowPart};
use crate::skin::component::{Area, ArrowComponent, Component};
use crate::skin::SkinParam;
use crate::style::Style;
use std::rc::Rc;

const ARROW_WIDTH: f64 = 45.0;
const X_RIGHT: f64 = ARROW_WIDTH - 3.0;

pub struct ComponentRoseSelfArrow {
    base: ArrowBase,
    nice_arrow: bool,
}

impl ComponentRoseSelfArrow {
    pub fn new(
        style: &Style,
        display: &Display,
        config: &ArrowConfiguration,
        skin: Rc<SkinParam>,
        max_message_size: &LineBreakStrategy,
        nice_arrow: bool,
    ) -> Self {
        Self {
            base: ArrowBase::new(style, display, config, skin, max_message_size),
            nice_arrow,
        }
    }

    fn arrow_only_height(&self) -> f64 {
        13.0
    }

    /// Width really covered by the drawing (`getDrawnWidth`).
    pub fn drawn_width(&self, sb: &dyn StringBounder) -> f64 {
        let pure = self.base.core.pure_text_width(sb);
        let label_reach = if pure == 0.0 {
            0.0
        } else {
            self.base.core.padding().left + pure
        };
        X_RIGHT.max(label_reach)
    }

    fn polygon(&self) -> UPolygon {
        let config = &self.base.config;
        let mut p = UPolygon::default();
        let direction = if config.is_reverse_define() {
            -1.0
        } else {
            1.0
        };
        let x = direction * ARROW_DELTA_X;
        match config.part() {
            ArrowPart::TopPart => {
                p.add_point(x - 1.0, -ARROW_DELTA_Y);
                p.add_point(-1.0, 0.0);
                p.add_point(x - 1.0, 0.0);
            }
            ArrowPart::BottomPart => {
                p.add_point(x - 1.0, 0.0);
                p.add_point(-1.0, 0.0);
                p.add_point(x - 1.0, ARROW_DELTA_Y);
            }
            ArrowPart::Full => {
                p.add_point(x, -ARROW_DELTA_Y);
                p.add_point(0.0, 0.0);
                p.add_point(x, ARROW_DELTA_Y);
                if self.nice_arrow {
                    p.add_point(x - direction * 4.0, 0.0);
                }
            }
        }
        p
    }

    fn circle(&self, ug2: &UGraphic, x: f64, y: f64) {
        ug2.apply_stroke(UStroke::with_thickness(THIN_CIRCLE))
            .apply_color(self.base.foreground.clone())
            .apply_back_color(self.base.background.clone())
            .apply_translate(UTranslate::new(x, y))
            .draw(UEllipse::build(DIAM_CIRCLE, DIAM_CIRCLE));
    }

    fn cross(&self, ug: &UGraphic, x: f64, y: f64) {
        let ug = ug.apply_stroke(UStroke::with_thickness(2.0));
        ug.apply_translate(UTranslate::new(x, y - ARROW_DELTA_X / 2.0))
            .draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_X));
        ug.apply_translate(UTranslate::new(x, y + ARROW_DELTA_X / 2.0))
            .draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_X));
    }

    fn draw_right_side(
        &self,
        ug: &UGraphic,
        ug2: &UGraphic,
        area: &Area,
        arrow_height: f64,
        text_height: f64,
    ) {
        let config = &self.base.config;
        let mut x1 = if area.delta_x1 < 0.0 {
            area.delta_x1
        } else {
            0.0
        };
        let mut x2 = if area.delta_x1 > 0.0 {
            -area.delta_x1
        } else {
            1.0
        };
        if config.decoration1() == ArrowDecoration::Circle {
            self.circle(
                ug2,
                x1 + 1.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                text_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            );
            x1 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE + 1.0;
            if config.dressing1().head() == ArrowHead::None {
                x1 -= THIN_CIRCLE + 1.0;
            }
        }
        if config.decoration2() == ArrowDecoration::Circle {
            self.circle(
                ug2,
                x2 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                text_height + arrow_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            );
            x2 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
        }
        let starting_cross = config.dressing1().head() == ArrowHead::CrossX;
        if starting_cross {
            x1 += 2.0 * SPACE_CROSS_X;
        }
        let final_cross = config.dressing2().head() == ArrowHead::CrossX;
        if final_cross {
            x2 += 2.0 * SPACE_CROSS_X;
        }
        if config.dressing2().head() == ArrowHead::Async {
            x2 -= 1.0;
        }
        ug2.apply_translate(UTranslate::new(x1, text_height))
            .draw(ULine::hline(X_RIGHT - x1));
        ug2.apply_translate(UTranslate::new(X_RIGHT, text_height))
            .draw(ULine::vline(arrow_height));
        ug2.apply_translate(UTranslate::new(x2, text_height + arrow_height))
            .draw(ULine::hline(X_RIGHT - x2));
        if config.dressing2().head() == ArrowHead::Async {
            x2 += 1.0;
        }
        if starting_cross {
            self.cross(ug, SPACE_CROSS_X, text_height);
        } else if config.is_async1() {
            let thin = config.apply_thickness_only(ug);
            if config.part() != ArrowPart::BottomPart {
                thin.apply_translate(UTranslate::new(x1, text_height))
                    .draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_Y));
            }
            if config.part() != ArrowPart::TopPart {
                thin.apply_translate(UTranslate::new(x1, text_height))
                    .draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_Y));
            }
        } else if config.dressing1().head() == ArrowHead::Normal {
            let polygon = self.polygon().translate(0.0, text_height);
            ug.apply_back_color(self.base.foreground.clone())
                .apply_translate(UTranslate::dx(x1))
                .draw(polygon);
        }
        if final_cross {
            self.cross(ug, SPACE_CROSS_X, text_height + arrow_height);
        } else if config.is_async2() {
            let thin = config.apply_thickness_only(ug);
            let origin = UTranslate::new(x2, text_height + arrow_height);
            if config.part() != ArrowPart::BottomPart {
                thin.apply_translate(origin)
                    .draw(ULine::new(ARROW_DELTA_X, -ARROW_DELTA_Y));
            }
            if config.part() != ArrowPart::TopPart {
                thin.apply_translate(origin)
                    .draw(ULine::new(ARROW_DELTA_X, ARROW_DELTA_Y));
            }
        } else if config.dressing2().head() == ArrowHead::Normal {
            let polygon = self.polygon().translate(0.0, text_height + arrow_height);
            ug.apply_back_color(self.base.foreground.clone())
                .apply_translate(UTranslate::dx(x2))
                .draw(polygon);
        }
    }

    fn draw_left_side(
        &self,
        ug: &UGraphic,
        ug2: &UGraphic,
        area: &Area,
        arrow_height: f64,
        text_height: f64,
        pref_text_width: f64,
    ) {
        let config = &self.base.config;
        let delta_size = area.live_delta_size;
        let dx = area.delta_x1;
        let level = area.level;
        let mut x1 = 0.0;
        let mut x2 = 1.0;
        let extra = f64::from(level) * delta_size;
        if dx < 0.0 {
            x2 += if level > 0 { -extra } else { delta_size };
        } else if dx > 0.0 {
            x1 += if level > 1 { delta_size - extra } else { 0.0 };
            x2 += if level == 1 { -delta_size } else { 0.0 };
        } else if level > 1 {
            x1 -= extra - delta_size;
            x2 -= extra - delta_size;
        }
        if config.decoration1() == ArrowDecoration::Circle {
            self.circle(
                ug2,
                pref_text_width - x1 + 1.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                text_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            );
            x1 += DIAM_CIRCLE / 2.0 - THIN_CIRCLE;
            if config.dressing1().head() == ArrowHead::None {
                x1 += THIN_CIRCLE;
            }
        }
        if config.decoration2() == ArrowDecoration::Circle {
            self.circle(
                ug2,
                pref_text_width - x2 + 2.0 - DIAM_CIRCLE / 2.0 - THIN_CIRCLE,
                text_height + arrow_height - DIAM_CIRCLE / 2.0 - THIN_CIRCLE / 2.0,
            );
            x2 += DIAM_CIRCLE / 2.0 + THIN_CIRCLE;
        }
        let starting_cross = config.dressing1().head() == ArrowHead::CrossX;
        if starting_cross {
            x1 += SPACE_CROSS_X + if level > 0 { extra } else { SPACE_CROSS_X };
        }
        let final_cross = config.dressing2().head() == ArrowHead::CrossX;
        if final_cross {
            x2 += SPACE_CROSS_X + if level > 0 { extra } else { SPACE_CROSS_X };
        }
        let full_normal2 = config.dressing2().part() == ArrowPart::Full
            && config.dressing2().head() == ArrowHead::Normal;
        let extraline = if full_normal2 { 1.0 } else { 0.0 };
        x1 += 1.0;
        let left = UTranslate::new(pref_text_width - X_RIGHT, text_height);
        ug2.apply_translate(left).draw(ULine::hline(X_RIGHT - x1));
        ug2.apply_translate(left).draw(ULine::vline(arrow_height));
        ug2.apply_translate(UTranslate::new(
            pref_text_width - X_RIGHT,
            text_height + arrow_height,
        ))
        .draw(ULine::hline(X_RIGHT - x2 - extraline));
        if full_normal2 {
            x2 += 1.0;
        }
        if config.dressing2().head() == ArrowHead::Async {
            x2 += 1.0;
        }
        if starting_cross {
            self.cross(ug, pref_text_width - x1 - SPACE_CROSS_X / 2.0, text_height);
        } else if config.dressing1().head() == ArrowHead::Normal {
            let polygon = self.polygon().translate(pref_text_width - x1, text_height);
            ug.apply_back_color(self.base.foreground.clone())
                .apply_translate(UTranslate::dx(x1))
                .draw(polygon);
        }
        if final_cross {
            self.cross(
                ug,
                pref_text_width - x2 - SPACE_CROSS_X / 2.0,
                text_height + arrow_height,
            );
        } else if config.is_async2() {
            let thin = config.apply_thickness_only(ug);
            let origin = UTranslate::new(pref_text_width - x2, text_height + arrow_height);
            if config.part() != ArrowPart::BottomPart {
                thin.apply_translate(origin)
                    .draw(ULine::new(-ARROW_DELTA_X, -ARROW_DELTA_Y));
            }
            if config.part() != ArrowPart::TopPart {
                thin.apply_translate(origin)
                    .draw(ULine::new(-ARROW_DELTA_X, ARROW_DELTA_Y));
            }
        } else if config.dressing2().head() == ArrowHead::Normal {
            let polygon = self.polygon().translate(0.0, text_height + arrow_height);
            ug.apply_back_color(self.base.foreground.clone())
                .apply_translate(UTranslate::dx(pref_text_width - x2))
                .draw(polygon);
        }
    }
}

impl Component for ComponentRoseSelfArrow {
    fn padding_y(&self) -> f64 {
        4.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let config = &self.base.config;
        if config.is_hidden() {
            return;
        }
        let sb = ug.string_bounder();
        let text_height = self.base.core.text_height(sb);
        let pref_text_width = self.preferred_width(sb);
        let arrow_height = self.arrow_only_height();
        let ug = ug.apply_color(self.base.foreground.clone());
        let ug2 = config.apply_stroke(&ug, &self.base.style);
        if config.is_reverse_define() {
            self.draw_left_side(&ug, &ug2, area, arrow_height, text_height, pref_text_width);
        } else {
            self.draw_right_side(&ug, &ug2, area, arrow_height, text_height);
        }
        self.base
            .core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::dx(self.base.core.padding().left)));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.base.core.text_height(sb)
            + ARROW_DELTA_Y
            + self.arrow_only_height()
            + 2.0 * self.padding_y()
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.base.core.text_width(sb).max(ARROW_WIDTH + 5.0)
    }
}

impl ArrowComponent for ComponentRoseSelfArrow {
    fn start_point(&self, sb: &dyn StringBounder, _dim: Dimension) -> (f64, f64) {
        let text_height = self.base.core.text_height(sb);
        (self.padding_x(), text_height + self.padding_y())
    }

    fn end_point(&self, sb: &dyn StringBounder, _dim: Dimension) -> (f64, f64) {
        let text_height = self.base.core.text_height(sb);
        let total = text_height + self.arrow_only_height();
        (self.padding_x(), total + self.padding_y())
    }

    fn y_point(&self, sb: &dyn StringBounder) -> f64 {
        let text_height = self.base.core.text_height(sb);
        let total = text_height + self.arrow_only_height();
        (text_height + total) / 2.0 + self.padding_x()
    }

    fn pos_arrow(&self, _sb: &dyn StringBounder) -> f64 {
        panic!("unsupported operation")
    }

    fn drawn_width(&self, sb: &dyn StringBounder) -> f64 {
        ComponentRoseSelfArrow::drawn_width(self, sb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::SvgOption;
    use crate::klimt::drawing::ugraphic::render_svg;
    use crate::klimt::font::EmbeddedFontBounder;

    fn draw(cfg: ArrowConfiguration, area: Area) -> String {
        let skin = Rc::new(SkinParam::new());
        let style = skin
            .style_builder()
            .merged_style(&crate::style::queries::SEQUENCEDIAG_ARROW)
            .unwrap();
        let c = ComponentRoseSelfArrow::new(
            &style,
            &Display::create(&["self"]),
            &cfg,
            skin,
            &LineBreakStrategy::NONE,
            true,
        );
        let sb = EmbeddedFontBounder;
        assert!(c.preferred_width(&sb) >= 50.0);
        assert!(c.drawn_width(&sb) >= X_RIGHT);
        assert!(c.preferred_height(&sb) > 17.0);
        assert!(c.y_point(&sb) > 0.0);
        let (_, sy) = c.start_point(&sb, Dimension::new(1.0, 1.0));
        let (_, ey) = c.end_point(&sb, Dimension::new(1.0, 1.0));
        assert!(ey > sy);
        render_svg(
            SvgOption::default(),
            0,
            Rc::new(EmbeddedFontBounder),
            |ug| c.draw(ug, &area, false),
        )
    }

    #[test]
    fn variants_draw() {
        let base = ArrowConfiguration::with_direction_self(false);
        let rev = ArrowConfiguration::with_direction_self(true);
        let mut areas = vec![Area::create(50.0, 30.0)];
        for (dx, level) in [(-5.0, 0), (-5.0, 2), (5.0, 1), (5.0, 3), (0.0, 2)] {
            let mut a = Area::create(50.0, 30.0);
            a.delta_x1 = dx;
            a.level = level;
            a.live_delta_size = 5.0;
            areas.push(a);
        }
        for cfg in [
            base.clone(),
            base.with_head(ArrowHead::Async),
            base.with_head2(ArrowHead::CrossX),
            base.with_head1(ArrowHead::CrossX),
            base.with_decoration1(ArrowDecoration::Circle),
            base.with_decoration2(ArrowDecoration::Circle),
            base.with_part(ArrowPart::TopPart),
            base.with_part(ArrowPart::BottomPart),
            rev.clone(),
            rev.with_head(ArrowHead::Async),
            rev.with_head1(ArrowHead::CrossX),
            rev.with_head2(ArrowHead::CrossX),
            rev.with_decoration1(ArrowDecoration::Circle),
            rev.with_decoration2(ArrowDecoration::Circle),
        ] {
            for a in &areas {
                let out = draw(cfg.clone(), *a);
                assert!(out.contains(">self</text>"), "{out}");
            }
        }
    }

    #[test]
    fn hidden_draws_nothing() {
        let cfg = ArrowConfiguration::with_direction_self(false)
            .with_body(crate::skin::arrow::ArrowBody::Hidden);
        assert!(!draw(cfg, Area::create(50.0, 30.0)).contains("<line"));
    }
}
