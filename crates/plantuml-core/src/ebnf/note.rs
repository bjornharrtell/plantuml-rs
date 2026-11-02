//! Comments of an EBNF diagram: a note with a folded corner (`FloatingNote` of Java).

use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::style::{queries, Margins, PName, SName};
use crate::svek::image_note::{note_body, Opale};
use std::rc::Rc;

/// The note box of a comment (`FloatingNote`).
struct FloatingNote {
    opale: Opale,
}

impl TextBlock for FloatingNote {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.opale.dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        self.opale.draw(ug, None);
    }
}

/// A comment shown next to a rule or on its own, in a note box with `margins` around it.
pub fn create(
    skin: &SkinParam,
    _fc: &FontConfiguration,
    text: &str,
    margins: Margins,
) -> Rc<dyn TextBlock> {
    let style = skin
        .style_builder()
        .merged_style(&queries::NOTE.add(SName::Ebnf))
        .expect("note style");
    let display = Display::with_newlines(Some(text));
    let opale = Opale {
        shadowing: style.shadowing(),
        border: style.value(PName::LineColor).as_color(),
        back: style.value(PName::BackGroundColor).as_color(),
        text: note_body(display, skin, &style, HorizontalAlignment::Left),
        stroke: style.stroke(),
        round_corner: f64::from(skin.as_int("roundcorner", 0)),
    };
    with_margin(Rc::new(FloatingNote { opale }), margins)
}
