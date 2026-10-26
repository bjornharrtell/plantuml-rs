//! Optional part of a rule, drawn with a zigzag shortcut (`ETileOptional2`).

use super::etile::{draw_hline_directed, ETile};
use super::note;
use super::zigzag::Zigzag;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::Margins;
use std::rc::Rc;

const DELTA_X: f64 = 24.0;
const DELTA_Y: f64 = 20.0;

/// A tile that may be skipped (`ETileOptional2`).
pub struct ETileOptional2 {
    orig: Box<dyn ETile>,
    skin: Rc<SkinParam>,
    fc: FontConfiguration,
    comment_above: Option<String>,
    comment_below: Option<String>,
}

impl ETileOptional2 {
    pub fn new(orig: Box<dyn ETile>, skin: Rc<SkinParam>, fc: FontConfiguration) -> Self {
        Self {
            orig,
            skin,
            fc,
            comment_above: None,
            comment_below: None,
        }
    }

    fn note_above(&self) -> Option<Rc<dyn TextBlock>> {
        self.comment_above.as_ref().map(|text| {
            note::create(
                &self.skin,
                &self.fc,
                text,
                Margins::new(0.0, 0.0, 0.0, 10.0),
            )
        })
    }

    fn note_below(&self) -> Option<Rc<dyn TextBlock>> {
        self.comment_below.as_ref().map(|text| {
            note::create(
                &self.skin,
                &self.fc,
                text,
                Margins::new(10.0, 0.0, 0.0, 0.0),
            )
        })
    }

    fn delta_y(&self, sb: &dyn StringBounder) -> f64 {
        let note = self
            .note_above()
            .map_or(0.0, |n| n.calculate_dimension(sb).height);
        DELTA_Y + note
    }
}

impl ETile for ETileOptional2 {
    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        let note = self
            .note_above()
            .map_or(0.0, |n| n.calculate_dimension(sb).height);
        10.0 + note
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        let note = self
            .note_below()
            .map_or(0.0, |n| n.calculate_dimension(sb).height);
        10.0 + self.orig.get_h1(sb) + self.orig.get_h2(sb) + note
    }

    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_width(sb) + 2.0 * DELTA_X
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = self.calculate_dimension(sb);
        let line_pos = self.get_h1(sb);

        draw_hline_directed(ug, line_pos, 0.0, dim.width, 0.4, 25.0);
        let corner = 12.0;
        let zigzag = Zigzag::new(
            9.0,
            2.0 * corner,
            self.delta_y(sb) + self.orig.get_h1(sb) - line_pos,
        );
        ug.apply_translate(UTranslate::new(0.0, line_pos))
            .draw(zigzag.path_down());
        ug.apply_translate(UTranslate::new(dim.width - 2.0 * corner, line_pos))
            .draw(zigzag.path_up());

        self.orig
            .draw_u(&ug.apply_translate(UTranslate::new(DELTA_X, self.delta_y(sb))));

        if let Some(note) = self.note_above() {
            let note_dim = note.calculate_dimension(sb);
            note.draw(&ug.apply_translate(UTranslate::dx((dim.width - note_dim.width) / 2.0)));
        }

        if let Some(note) = self.note_below() {
            let note_dim = note.calculate_dimension(sb);
            note.draw(&ug.apply_translate(UTranslate::new(
                (dim.width - note_dim.width) / 2.0,
                dim.height - note_dim.height,
            )));
        }
    }

    fn add_comment_above(&mut self, comment: String) {
        self.comment_above = Some(comment);
    }

    fn add_comment_below(&mut self, comment: String) {
        self.comment_below = Some(comment);
    }
}
