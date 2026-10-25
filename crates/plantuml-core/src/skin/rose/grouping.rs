//! Frame of `alt`, `loop`, ... groups (`ComponentRoseGroupingHeader`, `ComponentRoseGroupingElse`,
//! `ComponentRoseGroupingSpace`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPath, URectangle, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, PName, Style};
use std::rc::Rc;

const CORNER_SIZE: f64 = 10.0;
const COMMENT_MARGIN: f64 = 0.0;

pub struct ComponentRoseGroupingHeader {
    core: TextualCore,
    comment_block: Option<Rc<dyn TextBlock>>,
    background: HColor,
    symbol_context: Fashion,
    symbol_context_corner: Fashion,
    round_corner: f64,
}

impl ComponentRoseGroupingHeader {
    pub fn new(
        teoz: bool,
        style: &Style,
        style_header: &Style,
        title: &str,
        comment: Option<&str>,
        skin: Rc<SkinParam>,
    ) -> Self {
        let core = TextualCore::new(
            style_header,
            style_header,
            &LineBreakStrategy::NONE,
            Margins::new(1.0, 30.0, 1.0, 15.0),
            skin.clone(),
            &Display::with_newlines(Some(title)),
        );
        let comment_block = comment.map(|c| {
            let display = Display::with_newlines(Some(&format!("[{c}]")));
            display.create_simple_block(
                &style.font_configuration(None),
                HorizontalAlignment::Left,
                &skin.skin_simple(),
            )
        });
        Self {
            round_corner: f64::from(style.value(PName::RoundCorner).as_int()),
            background: if teoz {
                HColor::TRANSPARENT
            } else {
                style.value(PName::BackGroundColor).as_color()
            },
            symbol_context: style.symbol_context(None),
            symbol_context_corner: style_header.symbol_context(None),
            comment_block,
            core,
        }
    }

    fn supp_height_for_comment(&self, sb: &dyn StringBounder) -> f64 {
        let Some(block) = &self.comment_block else {
            return 0.0;
        };
        let height = block.calculate_dimension(sb).height;
        if height > 15.0 {
            height - 15.0
        } else {
            0.0
        }
    }

    fn corner(&self, width: f64, height: f64) -> UPath {
        let mut p = UPath::new();
        if self.round_corner == 0.0 {
            p.move_to(0.0, 0.0);
            p.line_to(width, 0.0);
            p.line_to(width, height - CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, height);
            p.line_to(0.0, height);
            p.line_to(0.0, 0.0);
        } else {
            let r = self.round_corner / 2.0;
            p.move_to(r, 0.0);
            p.line_to(width, 0.0);
            p.line_to(width, height - CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, height);
            p.line_to(0.0, height);
            p.line_to(0.0, r);
            p.arc_to(r, r, 0.0, false, true, r, 0.0);
        }
        p
    }

    fn frame(&self, area: &Area) -> URectangle {
        let dim = area.dimension_to_use;
        let mut rect = URectangle::build(dim.width, dim.height).rounded(self.round_corner);
        rect.delta_shadow = self.symbol_context.delta_shadow();
        rect
    }

    fn apply_symbol(&self, ug: &UGraphic) -> UGraphic {
        let fore = self
            .symbol_context
            .fore_color()
            .cloned()
            .unwrap_or(HColor::TRANSPARENT);
        ug.apply_stroke(self.symbol_context.stroke())
            .apply_color(fore)
    }
}

impl Component for ComponentRoseGroupingHeader {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        let sup = match &self.comment_block {
            None => COMMENT_MARGIN * 2.0,
            Some(block) => {
                self.core.padding().left + COMMENT_MARGIN + block.calculate_dimension(sb).width
            }
        };
        self.core.text_width(sb) + sup
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_height(sb) + 2.0 * self.padding_y() + self.supp_height_for_comment(sb)
    }

    fn draw_background_internal(&self, ug: &UGraphic, area: &Area) {
        let ug = self.apply_symbol(ug);
        ug.apply_back_color(self.background.clone())
            .draw(self.frame(area));
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let sb = ug.string_bounder();
        let text_width = self.core.text_width(sb);
        let text_height = self.core.text_height(sb);
        self.symbol_context_corner
            .apply(ug)
            .draw(self.corner(text_width, text_height));
        let ug = self.apply_symbol(ug);
        ug.draw(self.frame(area));
        let ug = ug.apply_stroke(UStroke::simple());
        let padding = self.core.padding();
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(padding.left, padding.top)));
        if let Some(block) = &self.comment_block {
            let x1 = padding.left + text_width;
            let y2 = padding.top + 1.0;
            block.draw(&ug.apply_translate(UTranslate::new(x1 + COMMENT_MARGIN, y2)));
        }
    }
}

pub struct ComponentRoseGroupingElse {
    core: TextualCore,
    group_border: HColor,
    background: HColor,
    round_corner: f64,
    teoz: bool,
}

impl ComponentRoseGroupingElse {
    pub fn new(teoz: bool, style: &Style, comment: Option<&str>, skin: Rc<SkinParam>) -> Self {
        let label = comment.map(|c| format!("[{c}]")).unwrap_or_default();
        let core = TextualCore::new(
            style,
            style,
            &LineBreakStrategy::NONE,
            Margins::new(1.0, 5.0, 1.0, 5.0),
            skin,
            &Display::with_newlines(Some(&label)),
        );
        Self {
            round_corner: core.base.round_corner(),
            group_border: core.base.color_line(),
            background: if teoz {
                HColor::TRANSPARENT
            } else {
                core.base.color_back_ground()
            },
            teoz,
            core,
        }
    }
}

impl Component for ComponentRoseGroupingElse {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.core.text_width(sb)
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        if self.teoz {
            self.core.text_height(sb) + 4.0
        } else {
            self.core.text_height(sb)
        }
    }

    fn draw_background_internal(&self, ug: &UGraphic, area: &Area) {
        if self.background.is_transparent() {
            return;
        }
        let dim = area.dimension_to_use;
        let ug = ug
            .apply_color(HColor::TRANSPARENT)
            .apply_back_color(self.background.clone());
        let (width, height) = (dim.width, dim.height);
        if self.round_corner == 0.0 {
            ug.draw(URectangle::build(width, height));
        } else {
            let r = self.round_corner / 2.0;
            let mut path = UPath::new();
            path.move_to(0.0, 0.0);
            path.line_to(width, 0.0);
            path.line_to(width, height - r);
            path.arc_to(r, r, 0.0, false, true, width - r, height);
            path.line_to(r, height);
            path.arc_to(r, r, 0.0, false, true, 0.0, height - r);
            path.line_to(0.0, 0.0);
            ug.draw(path);
        }
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let width = area.dimension_to_use.width;
        let ug = ug
            .apply_stroke(UStroke::new(2.0, 2.0, 1.0))
            .apply_color(self.group_border.clone());
        ug.apply_translate(UTranslate::dy(1.0))
            .draw(ULine::hline(width));
        let ug = ug.apply_stroke(UStroke::simple());
        let padding = self.core.padding();
        let dy = if self.teoz {
            padding.top + 2.0
        } else {
            padding.top
        };
        self.core
            .text_block()
            .draw(&ug.apply_translate(UTranslate::new(padding.left, dy)));
    }
}

/// Vertical space between two groups (`ComponentRoseGroupingSpace`).
pub struct ComponentRoseGroupingSpace {
    space: f64,
}

impl ComponentRoseGroupingSpace {
    pub fn new(space: f64) -> Self {
        Self { space }
    }
}

impl Component for ComponentRoseGroupingSpace {
    fn preferred_width(&self, _sb: &dyn StringBounder) -> f64 {
        0.0
    }

    fn preferred_height(&self, _sb: &dyn StringBounder) -> f64 {
        self.space
    }

    fn draw_internal(&self, _ug: &UGraphic, _area: &Area) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::SvgOption;
    use crate::klimt::drawing::ugraphic::render_svg;
    use crate::klimt::font::Dimension;
    use crate::klimt::font::EmbeddedFontBounder;
    use crate::skin::rose::Rose;
    use crate::style::queries;

    fn styles(skin: &Rc<SkinParam>) -> [Option<Style>; 2] {
        [
            skin.style_builder()
                .merged_style(&queries::SEQUENCEDIAG_GROUP),
            skin.style_builder()
                .merged_style(&queries::SEQUENCEDIAG_GROUPHEADER),
        ]
    }

    fn draw(c: &dyn Component, background: bool) -> String {
        render_svg(
            SvgOption::default(),
            0,
            Rc::new(EmbeddedFontBounder),
            |ug| {
                let sb = ug.string_bounder();
                c.draw(ug, &Area::create(100.0, c.preferred_height(sb)), background);
            },
        )
    }

    #[test]
    fn header_with_and_without_comment() {
        let skin = Rc::new(SkinParam::new());
        let st = styles(&skin);
        let sb = EmbeddedFontBounder;
        let plain = Rose::create_grouping_header(true, &st, "alt", None, &skin);
        let with = Rose::create_grouping_header(false, &st, "alt", Some("why"), &skin);
        assert!(with.preferred_width(&sb) > plain.preferred_width(&sb));
        assert!(draw(plain.as_ref(), false).contains(">alt</text>"));
        assert!(draw(with.as_ref(), false).contains(">[why]</text>"));
        assert!(draw(with.as_ref(), true).contains("<rect"));
    }

    #[test]
    fn else_and_space() {
        let skin = Rc::new(SkinParam::new());
        let st = styles(&skin);
        let style = st[0].clone().unwrap();
        let sb = EmbeddedFontBounder;
        for teoz in [true, false] {
            let c = Rose::create_grouping_else(teoz, &style, Some("no"), &skin);
            assert!(draw(c.as_ref(), false).contains(">[no]</text>"));
            draw(c.as_ref(), true);
        }
        let c = Rose::create_grouping_else(false, &style, None, &skin);
        assert!(c.preferred_width(&sb) >= 10.0);
        let space = Rose::create_grouping_space();
        assert_eq!(space.preferred_dimension(&sb), Dimension::new(0.0, 7.0));
        assert!(!draw(space.as_ref(), false).contains("<rect"));
    }
}
