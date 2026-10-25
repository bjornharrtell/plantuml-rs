//! Small rose components: activation box, destroy cross, delay line.

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UTranslate};
use crate::klimt::font::StringBounder;
use crate::skin::component::{Area, Component, ComponentBase};
use crate::skin::rose::line::fix_group_value;
use crate::skin::SkinParam;
use crate::style::Style;
use std::rc::Rc;

/// Activation box segment (`ComponentRoseActiveLine`).
pub struct ComponentRoseActiveLine {
    fashion: crate::klimt::fashion::Fashion,
    close_up: bool,
    close_down: bool,
    tooltip: String,
}

impl ComponentRoseActiveLine {
    pub fn new(style: &Style, close_up: bool, close_down: bool, display: Option<&Display>) -> Self {
        Self {
            fashion: style.symbol_context(None),
            close_up,
            close_down,
            tooltip: display
                .filter(|d| !d.is_null() && d.size() > 0)
                .map(|d| d.text_at(0))
                .unwrap_or_default(),
        }
    }
}

impl Component for ComponentRoseActiveLine {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let width = self.preferred_width(ug.string_bounder());
        let x = ((dim.width - width) / 2.0).trunc();
        if dim.height == 0.0 {
            return;
        }
        ug.start_group(&[("title", fix_group_value(&self.tooltip))]);
        let mut rect = URectangle::build(width, dim.height);
        if self.fashion.is_shadowing() {
            rect.delta_shadow = 1.0;
        }
        let fore = self
            .fashion
            .fore_color()
            .cloned()
            .unwrap_or(HColor::TRANSPARENT);
        let back = self
            .fashion
            .back_color()
            .cloned()
            .unwrap_or(HColor::TRANSPARENT);
        let ug = ug.apply_color(fore);
        if self.close_up && self.close_down {
            ug.apply_back_color(back)
                .apply_translate(UTranslate::dx(x))
                .draw(rect);
        } else {
            ug.apply_back_color(back.clone())
                .apply_color(back)
                .apply_translate(UTranslate::dx(x))
                .draw(rect);
            let vline = ULine::vline(dim.height);
            ug.apply_translate(UTranslate::dx(x)).draw(vline.clone());
            ug.apply_translate(UTranslate::dx(x + width)).draw(vline);
            let hline = ULine::hline(width);
            if self.close_up {
                ug.apply_translate(UTranslate::dx(x)).draw(hline.clone());
            }
            if self.close_down {
                ug.apply_translate(UTranslate::new(x, dim.height))
                    .draw(hline);
            }
        }
        ug.close_group();
    }

    fn preferred_height(&self, _: &dyn StringBounder) -> f64 {
        0.0
    }

    fn preferred_width(&self, _: &dyn StringBounder) -> f64 {
        10.0
    }
}

/// Cross shown when a participant is destroyed (`ComponentRoseDestroy`).
pub struct ComponentRoseDestroy {
    base: ComponentBase,
    foreground: HColor,
}

const CROSS_SIZE: f64 = 9.0;

impl ComponentRoseDestroy {
    pub fn new(style: &Style, skin: Rc<SkinParam>) -> Self {
        let base = ComponentBase::new(style.clone(), skin);
        Self {
            foreground: base.color_line(),
            base,
        }
    }
}

impl Component for ComponentRoseDestroy {
    fn draw_internal(&self, ug: &UGraphic, _: &Area) {
        let ug = ug
            .apply_stroke(self.base.stroke())
            .apply_color(self.foreground.clone());
        ug.draw(ULine::new(2.0 * CROSS_SIZE, 2.0 * CROSS_SIZE));
        ug.apply_translate(UTranslate::dy(2.0 * CROSS_SIZE))
            .draw(ULine::new(2.0 * CROSS_SIZE, -2.0 * CROSS_SIZE));
    }

    fn preferred_height(&self, _: &dyn StringBounder) -> f64 {
        CROSS_SIZE * 2.0
    }

    fn preferred_width(&self, _: &dyn StringBounder) -> f64 {
        CROSS_SIZE * 2.0
    }
}

/// Dashed lifeline part below a delay (`ComponentRoseDelayLine`).
pub struct ComponentRoseDelayLine {
    base: ComponentBase,
    color: HColor,
}

impl ComponentRoseDelayLine {
    pub fn new(style: &Style, skin: Rc<SkinParam>) -> Self {
        let base = ComponentBase::new(style.clone(), skin);
        Self {
            color: base.color_line(),
            base,
        }
    }
}

impl Component for ComponentRoseDelayLine {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let ug = ug
            .apply_stroke(self.base.stroke())
            .apply_color(self.color.clone());
        let x = (dim.width / 2.0).trunc();
        ug.apply_translate(UTranslate::dx(x))
            .draw(ULine::vline(dim.height));
    }

    fn preferred_height(&self, _: &dyn StringBounder) -> f64 {
        20.0
    }

    fn preferred_width(&self, _: &dyn StringBounder) -> f64 {
        1.0
    }
}
