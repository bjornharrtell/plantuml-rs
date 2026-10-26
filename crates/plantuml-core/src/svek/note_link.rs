//! Note attached to a link (`EntityImageNoteLink`).

use crate::abel::CucaNote;
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::component::{Area, Component, ComponentType};
use crate::skin::rose::Rose;
use crate::skin::SkinParam;
use std::rc::Rc;

struct EntityImageNoteLink {
    comp: Rc<dyn Component>,
}

impl TextBlock for EntityImageNoteLink {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.comp.preferred_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        self.comp.draw(ug, &Area::new(dim), false);
    }
}

pub fn note_link_block(note: &CucaNote, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let style = skin
        .style_builder()
        .merged_style(&ComponentType::Note.style_query())
        .expect("note style");
    let comp = Rose::create_component_note(
        &style,
        ComponentType::Note,
        &Rc::new(skin.clone()),
        &note.display,
        &note.colors,
        None,
    );
    Rc::new(EntityImageNoteLink { comp })
}
