//! Lifeline segment (`ComponentRoseLine`).

use crate::klimt::color::{HColor, TransparentFillBehavior};
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::skin::component::{Area, Component, ComponentBase};
use crate::skin::SkinParam;
use crate::style::Style;
use std::rc::Rc;

pub struct ComponentRoseLine {
    base: ComponentBase,
    color: HColor,
    stroke: UStroke,
    tooltip: String,
}

impl ComponentRoseLine {
    pub fn new(style: &Style, display: Option<&Display>, skin: Rc<SkinParam>) -> Self {
        let base = ComponentBase::new(style.clone(), skin);
        let tooltip = display
            .filter(|d| !d.is_null() && d.size() > 0)
            .map(|d| d.text_at(0))
            .unwrap_or_default();
        Self {
            color: base.color_line(),
            stroke: base.stroke(),
            base,
            tooltip,
        }
    }
}

/// Characters other than word chars, `-` and space become `.` (`UGroup.fix`).
pub fn fix_group_value(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' {
                c
            } else {
                '.'
            }
        })
        .collect()
}

impl Component for ComponentRoseLine {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let ug = ug.apply_color(self.color.clone()).apply_stroke(self.stroke);
        ug.start_group(&[("title", fix_group_value(&self.tooltip))]);
        if dim.height > 0.0 {
            let hover_width = 8.0;
            let hover = ug
                .apply_stroke(UStroke::with_thickness(0.0))
                .apply_color(HColor::TRANSPARENT)
                .apply_back_color(
                    HColor::TRANSPARENT
                        .with_fill_behavior(TransparentFillBehavior::WithFillOpacity),
                )
                .apply_translate(UTranslate::dx((dim.width - hover_width) / 2.0));
            hover.draw(URectangle::build(hover_width, dim.height));
        }
        let x = (dim.width / 2.0).trunc();
        ug.apply_translate(UTranslate::dx(x))
            .draw(ULine::vline(dim.height));
        ug.close_group();
    }

    fn preferred_height(&self, _: &dyn StringBounder) -> f64 {
        20.0
    }

    fn preferred_width(&self, _: &dyn StringBounder) -> f64 {
        let _ = &self.base;
        1.0
    }
}
