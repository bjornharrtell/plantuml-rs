//! Note attached to a link (`EntityImageNoteLink`); the box drawing is added with notes.

use crate::abel::CucaNote;
use crate::klimt::creole::display::Display;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::rc::Rc;

pub fn note_link_block(note: &CucaNote, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let _ = Display::empty();
    let style = skin
        .style_builder()
        .merged_style(&crate::style::queries::NOTE.clone())
        .expect("note style");
    style.create_text_block_bordered(&note.display, skin, &style.wrap_width())
}
