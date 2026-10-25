//! Frame of `ref over A, B` (`ComponentRoseReference`).

use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, URectangle, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

const CORNER_SIZE: f64 = 10.0;
const HEIGHT_FOOTER: f64 = 5.0;
const X_MARGIN: f64 = 2.0;

pub struct ComponentRoseReference {
    core: TextualCore,
    text_header: Rc<dyn TextBlock>,
    position: HorizontalAlignment,
    symbol_context_header: Fashion,
    symbol_context_body: Fashion,
    round_corner: f64,
}

impl ComponentRoseReference {
    /// `strings` starts with the header line (`ref`), the body follows.
    pub fn new(
        style: &Style,
        style_header: &Style,
        strings: &Display,
        skin: Rc<SkinParam>,
    ) -> Self {
        let core = TextualCore::new(
            style,
            style,
            &LineBreakStrategy::NONE,
            Margins::new(4.0, 4.0, 4.0, 4.0),
            skin.clone(),
            &strings.sub_list(1, strings.size()),
        );
        let text_header = strings.sub_list(0, 1).create_simple_block(
            &style_header.font_configuration(None),
            HorizontalAlignment::Left,
            &skin.skin_simple(),
        );
        Self {
            symbol_context_header: style_header.symbol_context(None),
            symbol_context_body: style.symbol_context(None),
            round_corner: core.base.round_corner(),
            position: core.base.horizontal_alignment(),
            text_header,
            core,
        }
    }

    fn header_height(&self, sb: &dyn StringBounder) -> f64 {
        self.text_header.calculate_dimension(sb).height + 2.0
    }

    fn header_width(&self, sb: &dyn StringBounder) -> f64 {
        self.text_header.calculate_dimension(sb).width + 30.0 + 15.0
    }
}

impl Component for ComponentRoseReference {
    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + self.header_height(sb) + HEIGHT_FOOTER
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb).max(self.header_width(sb))
            + X_MARGIN * 2.0
            + self.symbol_context_body.delta_shadow()
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let dim = area.dimension_to_use;
        let sb = ug.string_bounder();
        let header_width = self.header_width(sb).trunc();
        let header_height = self.header_height(sb).trunc();
        let shadow = self.symbol_context_body.delta_shadow();
        let mut rect = URectangle::build(
            dim.width - X_MARGIN * 2.0 - shadow,
            dim.height - HEIGHT_FOOTER,
        );
        if self.round_corner != 0.0 {
            rect = rect.rounded(self.round_corner);
        }
        rect.delta_shadow = shadow;
        let ug = self.symbol_context_body.apply(ug);
        ug.apply_translate(UTranslate::dx(X_MARGIN)).draw(rect);
        let mut corner = UPath::new();
        if self.round_corner == 0.0 {
            corner.move_to(0.0, 0.0);
            corner.line_to(header_width, 0.0);
            corner.line_to(header_width, header_height - CORNER_SIZE);
            corner.line_to(header_width - CORNER_SIZE, header_height);
            corner.line_to(0.0, header_height);
            corner.line_to(0.0, 0.0);
        } else {
            let r = self.round_corner / 2.0;
            corner.move_to(r, 0.0);
            corner.line_to(header_width, 0.0);
            corner.line_to(header_width, header_height - CORNER_SIZE);
            corner.line_to(header_width - CORNER_SIZE, header_height);
            corner.line_to(0.0, header_height);
            corner.line_to(0.0, r);
            corner.arc_to(r, r, 0.0, false, true, r, 0.0);
        }
        let ug = self.symbol_context_header.apply(&ug);
        ug.apply_translate(UTranslate::dx(X_MARGIN)).draw(corner);
        let ug = ug.apply_stroke(UStroke::simple());
        self.text_header
            .draw(&ug.apply_translate(UTranslate::new(15.0, 2.0)));
        let padding = self.core.padding();
        let text_width = self.core.text_block().calculate_dimension(sb).width;
        let text_pos = match self.position {
            HorizontalAlignment::Center => (dim.width - text_width) / 2.0,
            HorizontalAlignment::Right => dim.width - text_width - padding.right - X_MARGIN,
            HorizontalAlignment::Left => padding.left + X_MARGIN,
        };
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(text_pos, padding.top + header_height)));
    }
}
