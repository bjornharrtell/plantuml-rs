//! Notes: folded corner, box and hexagon (`ComponentRoseNote`, `ComponentRoseNoteBox`,
//! `ComponentRoseNoteHexagonal`).

use crate::klimt::color::colors::Colors;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, UPolygon, URectangle, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::RectangleStencil;
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

const CORNER_SIZE: f64 = 10.0;

/// Outline of a note with a folded corner (`Opale.getPolygonNormal`).
pub fn polygon_normal(width: f64, height: f64, round_corner: f64) -> UPath {
    let mut p = UPath::new();
    if round_corner == 0.0 {
        p.move_to(0.0, 0.0);
        p.line_to(0.0, height);
        p.line_to(width, height);
        p.line_to(width, CORNER_SIZE);
        p.line_to(width - CORNER_SIZE, 0.0);
        p.line_to(0.0, 0.0);
    } else {
        let r = round_corner / 2.0;
        p.move_to(0.0, r);
        p.line_to(0.0, height - r);
        p.arc_to(r, r, 0.0, false, false, r, height);
        p.line_to(width - r, height);
        p.arc_to(r, r, 0.0, false, false, width, height - r);
        p.line_to(width, CORNER_SIZE);
        p.line_to(width - CORNER_SIZE, 0.0);
        p.line_to(r, 0.0);
        p.arc_to(r, r, 0.0, false, false, 0.0, r);
    }
    p.close_path();
    p
}

/// The folded corner (`Opale.getCorner`).
pub fn corner(width: f64, round_corner: f64) -> UPath {
    let mut p = UPath::new();
    p.move_to(width - CORNER_SIZE, 0.0);
    if round_corner == 0.0 {
        p.line_to(width - CORNER_SIZE, CORNER_SIZE);
    } else {
        let r = round_corner / 4.0;
        p.line_to(width - CORNER_SIZE, CORNER_SIZE - r);
        p.arc_to(
            r,
            r,
            0.0,
            false,
            false,
            width - CORNER_SIZE + r,
            CORNER_SIZE,
        );
    }
    p.line_to(width, CORNER_SIZE);
    p.line_to(width - CORNER_SIZE, 0.0);
    p.close_path();
    p
}

/// Width given to a note whose area is `area_width` (`x2` of the draw methods).
fn drawn_width(text_width: f64, preferred: f64, area_width: f64, padding_x: f64) -> f64 {
    if area_width > preferred {
        (area_width - 2.0 * padding_x).trunc()
    } else {
        text_width.trunc()
    }
}

pub struct ComponentRoseNote {
    core: TextualCore,
    padding_x: f64,
    padding_y: f64,
    symbol_context: Fashion,
    round_corner: f64,
    position: HorizontalAlignment,
}

impl ComponentRoseNote {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        style: &Style,
        display: &Display,
        padding_x: f64,
        padding_y: f64,
        skin: Rc<SkinParam>,
        text_alignment: HorizontalAlignment,
        position: HorizontalAlignment,
        colors: &Colors,
    ) -> Self {
        let padding = if text_alignment == HorizontalAlignment::Center {
            Margins::new(5.0, 15.0, 5.0, 15.0)
        } else {
            Margins::new(5.0, 15.0, 5.0, 6.0)
        };
        let core = TextualCore::with_enhanced(
            style,
            style,
            &style.wrap_width(),
            padding,
            skin,
            display,
            true,
        );
        Self {
            symbol_context: style.symbol_context(Some(colors)),
            round_corner: core.base.round_corner(),
            core,
            padding_x,
            padding_y,
            position,
        }
    }
}

impl Component for ComponentRoseNote {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb) + 2.0 * self.padding_x + self.symbol_context.delta_shadow()
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 2.0 * self.padding_y + self.symbol_context.delta_shadow()
    }

    fn padding_x(&self) -> f64 {
        self.padding_x
    }

    fn padding_y(&self) -> f64 {
        self.padding_y
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let sb = ug.string_bounder();
        let text_height = self.core.text_height(sb).trunc();
        let text_width = self.core.text_width(sb);
        let width = area.dimension_to_use.width;
        let diff_x = width - self.preferred_width(sb);
        let x2 = drawn_width(text_width, self.preferred_width(sb), width, self.padding_x);
        let mut polygon = polygon_normal(x2, text_height, self.round_corner);
        polygon.delta_shadow = self.symbol_context.delta_shadow();
        let ug = self.symbol_context.apply(ug);
        ug.draw(polygon);
        ug.draw(corner(x2, self.round_corner));
        let ug2 = ug.apply_stencil(Rc::new(RectangleStencil(text_width)), UStroke::simple());
        let padding = self.core.padding();
        let ug2 = match self.position {
            HorizontalAlignment::Left => {
                ug2.apply_translate(UTranslate::new(padding.left, padding.top))
            }
            HorizontalAlignment::Right => {
                ug2.apply_translate(UTranslate::new(width - text_width, padding.top))
            }
            HorizontalAlignment::Center => {
                ug2.apply_translate(UTranslate::new(padding.left + diff_x / 2.0, padding.top))
            }
        };
        self.core.text_block().draw(&ug2);
    }
}

pub struct ComponentRoseNoteBox {
    core: TextualCore,
    symbol_context: Fashion,
    round_corner: f64,
}

impl ComponentRoseNoteBox {
    pub fn new(style: &Style, display: &Display, skin: Rc<SkinParam>, colors: &Colors) -> Self {
        let core = TextualCore::new(
            style,
            style,
            &style.wrap_width(),
            Margins::new(4.0, 4.0, 4.0, 4.0),
            skin,
            display,
        );
        Self {
            symbol_context: style.symbol_context(Some(colors)),
            round_corner: core.base.round_corner(),
            core,
        }
    }
}

impl Component for ComponentRoseNoteBox {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb) + 2.0 * self.padding_x()
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 2.0 * self.padding_y()
    }

    fn padding_x(&self) -> f64 {
        5.0
    }

    fn padding_y(&self) -> f64 {
        5.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let sb = ug.string_bounder();
        let text_height = self.core.text_height(sb).trunc();
        let text_width = self.core.text_width(sb);
        let width = area.dimension_to_use.width;
        let diff_x = width - self.preferred_width(sb);
        let x2 = drawn_width(
            text_width,
            self.preferred_width(sb),
            width,
            self.padding_x(),
        );
        let ug = self.symbol_context.apply(ug);
        let mut rect = URectangle::build(x2, text_height).rounded(self.round_corner);
        rect.delta_shadow = self.symbol_context.delta_shadow();
        ug.draw(rect);
        let ug = ug.apply_stroke(UStroke::simple());
        let padding = self.core.padding();
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(padding.left + diff_x / 2.0, padding.top)));
    }
}

pub struct ComponentRoseNoteHexagonal {
    core: TextualCore,
    symbol_context: Fashion,
}

impl ComponentRoseNoteHexagonal {
    pub fn new(style: &Style, display: &Display, skin: Rc<SkinParam>, colors: &Colors) -> Self {
        let core = TextualCore::new(
            style,
            style,
            &style.wrap_width(),
            Margins::new(4.0, 12.0, 4.0, 12.0),
            skin,
            display,
        );
        Self {
            symbol_context: style.symbol_context(Some(colors)),
            core,
        }
    }
}

impl Component for ComponentRoseNoteHexagonal {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb) + 2.0 * self.padding_x()
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 2.0 * self.padding_y()
    }

    fn padding_x(&self) -> f64 {
        5.0
    }

    fn padding_y(&self) -> f64 {
        5.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let sb = ug.string_bounder();
        let text_height = self.core.text_height(sb).trunc();
        let text_width = self.core.text_width(sb);
        let width = area.dimension_to_use.width;
        let diff_x = width - self.preferred_width(sb);
        let x2 = drawn_width(
            text_width,
            self.preferred_width(sb),
            width,
            self.padding_x(),
        );
        let mut polygon = UPolygon::default();
        let corner = 10.0;
        polygon.add_point(corner, 0.0);
        polygon.add_point(x2 - corner, 0.0);
        polygon.add_point(x2, (text_height / 2.0).trunc());
        polygon.add_point(x2 - corner, text_height);
        polygon.add_point(corner, text_height);
        polygon.add_point(0.0, (text_height / 2.0).trunc());
        polygon.add_point(corner, 0.0);
        let ug = self.symbol_context.apply(ug);
        polygon.delta_shadow = self.symbol_context.delta_shadow();
        ug.draw(polygon);
        let ug = ug.apply_stroke(UStroke::simple());
        let padding = self.core.padding();
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(padding.left + diff_x / 2.0, padding.top)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::SvgOption;
    use crate::klimt::drawing::ugraphic::render_svg;
    use crate::klimt::font::EmbeddedFontBounder;
    use crate::sequencediagram::model::NotePosition;
    use crate::skin::component::ComponentType;
    use crate::skin::rose::Rose;
    use crate::style::queries;

    #[test]
    fn shapes_draw_text_and_grow_with_area() {
        let skin = Rc::new(SkinParam::new());
        let style = skin
            .style_builder()
            .merged_style(&queries::SEQUENCEDIAG_NOTE)
            .unwrap();
        let sb = EmbeddedFontBounder;
        for ctype in [
            ComponentType::Note,
            ComponentType::NoteBox,
            ComponentType::NoteHexagonal,
        ] {
            let c = Rose::create_component_note(
                &style,
                ctype,
                &skin,
                &Display::create(&["hello", "--t--", "world"]),
                &Colors::empty(),
                Some(NotePosition::OverSeveral),
            );
            let dim = c.preferred_dimension(&sb);
            for extra in [0.0, 20.0] {
                let out = render_svg(
                    SvgOption::default(),
                    0,
                    Rc::new(EmbeddedFontBounder),
                    |ug| c.draw(ug, &Area::create(dim.width + extra, dim.height), false),
                );
                assert!(
                    out.contains(">hello</text>") && out.contains(">world</text>"),
                    "{out}"
                );
            }
        }
    }

    #[test]
    fn text_alignment_follows_skinparam() {
        let mut skin = SkinParam::new();
        assert_eq!(skin.note_text_alignment(None), HorizontalAlignment::Left);
        skin.set_param("defaultTextAlignment", "center");
        assert_eq!(skin.note_text_alignment(None), HorizontalAlignment::Center);
        skin.set_param("noteTextAlignment", "right");
        assert_eq!(skin.note_text_alignment(None), HorizontalAlignment::Right);
    }

    #[test]
    fn corner_paths() {
        assert!(!polygon_normal(40.0, 20.0, 0.0).segments.is_empty());
        assert!(!polygon_normal(40.0, 20.0, 5.0).segments.is_empty());
        assert!(!corner(40.0, 5.0).segments.is_empty());
    }
}
