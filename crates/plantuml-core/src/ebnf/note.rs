//! Text of the comments of an EBNF diagram (`FloatingNote` of Java).

use crate::klimt::creole::display::Display;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::style::Margins;
use std::rc::Rc;

/// A comment shown next to a rule or on its own.
///
/// Java draws the comments of an EBNF diagram with a `FloatingNote` wrapped in an "opale" bubble;
/// the port keeps the text and its margins and drops the bubble.
pub fn create(
    skin: &SkinParam,
    fc: &FontConfiguration,
    text: &str,
    margins: Margins,
) -> Rc<dyn TextBlock> {
    let display = Display::with_newlines(Some(text));
    let block = display.create_simple_block(fc, HorizontalAlignment::Center, &skin.skin_simple());
    with_margin(block, margins)
}
