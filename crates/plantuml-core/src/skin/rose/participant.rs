//! Participant box (`ComponentRoseParticipant`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::drawing::ugraphic::{URectangle, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

pub struct ComponentRoseParticipant {
    core: TextualCore,
    back: Option<HColor>,
    foreground: Option<HColor>,
    delta_shadow: f64,
    round_corner: f64,
    stroke: UStroke,
    min_width: f64,
    collections: bool,
    padding: Margins,
    margin: Margins,
}

impl ComponentRoseParticipant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        style: &Style,
        stereo: &Style,
        display: &Display,
        skin: Rc<SkinParam>,
        min_width: f64,
        collections: bool,
        padding: Margins,
        margin: Margins,
    ) -> Self {
        let core = TextualCore::new(
            style,
            stereo,
            &LineBreakStrategy::NONE,
            style.padding(),
            skin,
            display,
        );
        let fashion = style.symbol_context(None);
        Self {
            round_corner: core.base.round_corner(),
            stroke: core.base.stroke(),
            back: fashion.back_color().cloned(),
            foreground: fashion.fore_color().cloned(),
            delta_shadow: fashion.delta_shadow(),
            core,
            min_width,
            collections,
            padding,
            margin,
        }
    }

    fn delta_collection(&self) -> f64 {
        if self.collections {
            4.0
        } else {
            0.0
        }
    }

    fn pure_text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.pure_text_width(sb).max(self.min_width)
    }

    fn text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.pure_text_width(sb) + self.core.padding().left + self.core.padding().right
    }
}

impl Component for ComponentRoseParticipant {
    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let sb = ug.string_bounder();
        let mut ug = ug.apply_translate(UTranslate::new(self.margin.left, self.margin.top));
        if let Some(f) = &self.foreground {
            ug = ug.apply_color(f.clone());
        }
        if let Some(b) = &self.back {
            ug = ug.apply_back_color(b.clone());
        }
        ug = ug.apply_stroke(self.stroke);
        let box_width = self.text_width(sb);
        let box_height = self.core.text_height(sb);
        let mut rect = URectangle::build(box_width, box_height).rounded(self.round_corner);
        rect.delta_shadow = self.delta_shadow;
        if self.collections {
            ug.apply_translate(UTranslate::dx(self.delta_collection()))
                .draw(rect.clone());
            ug = ug.apply_translate(UTranslate::dy(self.delta_collection()));
        }
        ug.draw(rect);
        ug = ug.apply_stroke(UStroke::simple());
        let text_width = self.core.text_block().calculate_dimension(sb).width;
        let extra = self.pure_text_width(sb) - text_width;
        let dx = if extra > 0.0 {
            self.core
                .base
                .horizontal_alignment()
                .position(text_width, text_width + extra)
        } else {
            0.0
        };
        let tr = UTranslate::new(self.padding.left, self.padding.top);
        self.core
            .text_block()
            .draw(&ug.apply_translate(tr).apply_translate(UTranslate::dx(dx)));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb)
            + self.margin.top
            + self.margin.bottom
            + self.delta_shadow
            + 1.0
            + self.delta_collection()
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.text_width(sb)
            + self.margin.left
            + self.margin.right
            + self.delta_shadow
            + self.delta_collection()
    }
}
