//! Title band of a participant box (`ComponentRoseEnglober`).

use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

pub struct ComponentRoseEnglober {
    core: TextualCore,
    symbol_context: Fashion,
    round_corner: f64,
}

impl ComponentRoseEnglober {
    pub fn new(style: &Style, display: &Display, skin: Rc<SkinParam>) -> Self {
        let core = TextualCore::new(
            style,
            style,
            &LineBreakStrategy::NONE,
            Margins::new(1.0, 3.0, 1.0, 3.0),
            skin,
            display,
        );
        Self {
            round_corner: core.base.round_corner(),
            symbol_context: style.symbol_context(None),
            core,
        }
    }
}

impl Component for ComponentRoseEnglober {
    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 3.0
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb)
    }

    fn draw_background_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let ug = self.symbol_context.apply(ug);
        ug.draw(URectangle::build(dim.width, dim.height).rounded(self.round_corner));
        let xpos = (dim.width - self.core.pure_text_width(ug.string_bounder())) / 2.0;
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::dx(xpos)));
    }

    fn draw_internal(&self, _ug: &UGraphic, _area: &Area) {}
}
