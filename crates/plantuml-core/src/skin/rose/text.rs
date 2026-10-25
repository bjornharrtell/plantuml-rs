//! Rose components made of text: divider, delay text, new page (`ComponentRoseDivider`,
//! `ComponentRoseDelayText`, `ComponentRoseNewpage`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::skin::component::{Area, Component, ComponentBase, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

pub struct ComponentRoseDivider {
    core: TextualCore,
    border_color: HColor,
    background: HColor,
    empty: bool,
    shadow: f64,
    stroke: UStroke,
    round_corner: f64,
}

impl ComponentRoseDivider {
    pub fn new(style: &Style, display: &Display, skin: Rc<SkinParam>) -> Self {
        let core = TextualCore::new(
            style,
            style,
            &LineBreakStrategy::NONE,
            Margins::new(4.0, 4.0, 4.0, 4.0),
            skin,
            display,
        );
        Self {
            background: core.base.color_back_ground(),
            border_color: core.base.color_line(),
            stroke: core.base.stroke(),
            round_corner: core.base.round_corner(),
            shadow: core.base.shadowing(),
            empty: display.text_at(0).is_empty(),
            core,
        }
    }

    fn draw_sep(&self, ug: &UGraphic, width: f64) {
        let ug = ug.apply_color(self.background.clone());
        let mut rect = URectangle::build(width, 3.0).rounded(self.round_corner);
        rect.delta_shadow = self.shadow;
        ug.apply_translate(UTranslate::dy(-1.0))
            .apply_stroke(UStroke::simple())
            .draw(rect);
        let ug = ug
            .apply_stroke(UStroke::with_thickness(self.stroke.thickness / 2.0))
            .apply_color(self.border_color.clone());
        let line = ULine::hline(width);
        ug.apply_translate(UTranslate::dy(-1.0)).draw(line.clone());
        ug.apply_translate(UTranslate::dy(2.0)).draw(line);
    }
}

impl Component for ComponentRoseDivider {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let ug = ug.apply_back_color(self.background.clone());
        if self.empty {
            self.draw_sep(
                &ug.apply_translate(UTranslate::dy(dim.height / 2.0)),
                dim.width,
            );
            return;
        }
        let sb = ug.string_bounder();
        let text_width = self.core.text_width(sb);
        let text_height = self.core.text_height(sb);
        let delta_x = 6.0;
        let xpos = (dim.width - text_width - delta_x) / 2.0;
        let ypos = (dim.height - text_height) / 2.0;
        self.draw_sep(
            &ug.apply_translate(UTranslate::dy(dim.height / 2.0)),
            dim.width,
        );
        let ug = ug
            .apply_color(self.border_color.clone())
            .apply_stroke(self.stroke);
        let mut rect =
            URectangle::build(text_width + delta_x, text_height).rounded(self.round_corner);
        rect.delta_shadow = self.shadow;
        ug.apply_translate(UTranslate::new(xpos, ypos)).draw(rect);
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(
                xpos + delta_x,
                ypos + self.core.padding().top,
            )));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 20.0
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb) + 30.0
    }
}

pub struct ComponentRoseDelayText {
    core: TextualCore,
}

impl ComponentRoseDelayText {
    pub fn new(style: &Style, display: &Display, skin: Rc<SkinParam>) -> Self {
        Self {
            core: TextualCore::new(
                style,
                style,
                &LineBreakStrategy::NONE,
                Margins::new(4.0, 0.0, 4.0, 0.0),
                skin,
                display,
            ),
        }
    }
}

impl Component for ComponentRoseDelayText {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let sb = ug.string_bounder();
        let text_width = self.core.text_width(sb);
        let text_height = self.core.text_height(sb);
        let xpos = (dim.width - text_width) / 2.0;
        let ypos = (dim.height - text_height) / 2.0;
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(xpos, ypos + self.core.padding().top)));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 20.0
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.pure_text_width(sb)
    }
}

pub struct ComponentRoseNewpage {
    base: ComponentBase,
    foreground: HColor,
}

impl ComponentRoseNewpage {
    pub fn new(style: &Style, skin: Rc<SkinParam>) -> Self {
        let base = ComponentBase::new(style.clone(), skin);
        Self {
            foreground: base.color_line(),
            base,
        }
    }
}

impl Component for ComponentRoseNewpage {
    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let ug = ug
            .apply_stroke(self.base.stroke())
            .apply_color(self.foreground.clone());
        ug.draw(ULine::hline(area.dimension_to_use.width));
    }

    fn preferred_height(&self, _: &dyn StringBounder) -> f64 {
        1.0
    }

    fn preferred_width(&self, _: &dyn StringBounder) -> f64 {
        0.0
    }
}
