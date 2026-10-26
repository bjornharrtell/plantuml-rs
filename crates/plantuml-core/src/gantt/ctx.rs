//! Styles and text blocks shared by the drawing code.

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{CreoleMode, LineBreakStrategy, SkinSimple};
use crate::klimt::drawing::ugraphic::UStroke;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{FontFace, StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName, SName, Style, StyleBuilder, StyleQuery};
use std::collections::BTreeMap;
use std::rc::Rc;

/// What the drawing code needs from the skin.
pub struct Ctx {
    pub builder: StyleBuilder,
    pub simple: SkinSimple,
    pub sb: Rc<dyn StringBounder>,
}

/// `StyleQueries.GANTTDIAG.add(...)`.
pub fn gantt_query(names: &[SName]) -> StyleQuery {
    queries::ELEMENT
        .clone()
        .add(SName::GanttDiagram)
        .add_all(names)
}

impl Ctx {
    pub fn new(skin: &SkinParam, sb: &Rc<dyn StringBounder>) -> Ctx {
        Ctx {
            builder: skin.style_builder(),
            simple: skin.skin_simple(),
            sb: sb.clone(),
        }
    }

    pub fn style_q(&self, q: &StyleQuery) -> Style {
        self.builder
            .merged_style(q)
            .unwrap_or_else(|| Style::new(q.clone(), BTreeMap::new()))
    }

    pub fn style(&self, names: &[SName]) -> Style {
        self.style_q(&gantt_query(names))
    }

    /// A block showing `text`, with Creole (`Display.getWithNewlines(...).create(...)`).
    pub fn block(&self, text: &str, fc: &FontConfiguration) -> Rc<dyn TextBlock> {
        Display::with_newlines(Some(text)).create_block(
            fc,
            HorizontalAlignment::Left,
            &self.simple,
            &LineBreakStrategy::NONE,
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        )
    }
}

pub fn font_configuration(font: UFont, bold: bool, color: HColor) -> FontConfiguration {
    let font = if bold {
        font.with_face(FontFace::BOLD)
    } else {
        font
    };
    FontConfiguration::create(font, color.clone(), color, None, 8)
}

/// Fonts, colors and sizes of the timeline (`TimelineStyleData`).
pub struct TimelineStyle {
    pub font_size_day: f64,
    pub font_size_month: f64,
    pub font_size_year: f64,
    pub font_day: UFont,
    pub font_month: UFont,
    pub font_year: UFont,
    pub closed_background: HColor,
    pub closed_font: HColor,
    pub open_font: HColor,
    pub line_color: HColor,
    pub vsep_color: HColor,
    pub vsep_stroke: UStroke,
    pub back: HColor,
    pub text_fc: FontConfiguration,
}

impl TimelineStyle {
    pub fn new(ctx: &Ctx) -> TimelineStyle {
        let day = ctx.style(&[SName::Timeline, SName::Day]);
        let month = ctx.style(&[SName::Timeline, SName::Month]);
        let year = ctx.style(&[SName::Timeline, SName::Year]);
        let timeline = ctx.style(&[SName::Timeline]);
        let closed = ctx.style(&[SName::Closed]);
        let vsep = ctx.style(&[SName::VerticalSeparator]);
        TimelineStyle {
            font_size_day: day.value(PName::FontSize).as_double(),
            font_size_month: month.value(PName::FontSize).as_double(),
            font_size_year: year.value(PName::FontSize).as_double(),
            font_day: day.font(),
            font_month: month.font(),
            font_year: year.font(),
            closed_background: closed.value(PName::BackGroundColor).as_color(),
            closed_font: closed.value(PName::FontColor).as_color(),
            open_font: timeline.value(PName::FontColor).as_color(),
            line_color: timeline.value(PName::LineColor).as_color(),
            vsep_color: vsep.value(PName::LineColor).as_color(),
            vsep_stroke: vsep.stroke(),
            back: timeline.value(PName::BackGroundColor).as_color(),
            text_fc: timeline.font_configuration(None),
        }
    }

    pub fn cell_width(&self) -> f64 {
        self.font_size_day * 1.6
    }

    pub fn font(&self, param: SName) -> UFont {
        match param {
            SName::Month => self.font_month.clone(),
            SName::Year => self.font_year.clone(),
            _ => self.font_day.clone(),
        }
    }
}
