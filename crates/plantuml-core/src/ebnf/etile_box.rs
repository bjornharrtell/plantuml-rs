//! A box of text: identifier, terminal string or special sequence (`ETileBox`).

use super::etile::{draw_hline_directed, ETile};
use super::note;
use super::symbol::Symbol;
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UStroke, UText, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{Margins, PName, Style};
use std::rc::Rc;

/// Padding between the text and its box.
const PADDING: f64 = 5.0;
/// Extra space taken by a comment above or below the box.
const NOTE_SPACE: f64 = 20.0;

/// One token of the rule drawn as a shape (`ETileBox`).
pub struct ETileBox {
    value: String,
    symbol: Symbol,
    fc: FontConfiguration,
    style: Style,
    skin: Rc<SkinParam>,
    comment_above: Option<String>,
    comment_below: Option<String>,
}

impl ETileBox {
    pub fn new(
        value: &str,
        symbol: Symbol,
        fc: FontConfiguration,
        style: Style,
        skin: Rc<SkinParam>,
    ) -> Self {
        Self {
            value: value.to_string(),
            symbol,
            fc,
            style,
            skin,
            comment_above: None,
            comment_below: None,
        }
    }

    fn text_dim(&self, sb: &dyn StringBounder) -> Dimension {
        sb.calculate_dimension(&self.fc.font(), &self.value)
    }

    fn box_dim(&self, sb: &dyn StringBounder) -> Dimension {
        self.text_dim(sb).delta(2.0 * PADDING, 2.0 * PADDING)
    }

    fn note_above(&self) -> Option<Rc<dyn TextBlock>> {
        self.comment_above
            .as_ref()
            .map(|text| note::create(&self.skin, &self.fc, text, Margins::default()))
    }

    fn note_below(&self) -> Option<Rc<dyn TextBlock>> {
        self.comment_below
            .as_ref()
            .map(|text| note::create(&self.skin, &self.fc, text, Margins::default()))
    }
}

impl ETile for ETileBox {
    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        let mut width = self.text_dim(sb).width + 2.0 * PADDING;
        if let Some(note) = self.note_above() {
            width = width.max(note.calculate_dimension(sb).width);
        }
        if let Some(note) = self.note_below() {
            width = width.max(note.calculate_dimension(sb).width);
        }
        width
    }

    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        let pure = (self.text_dim(sb).height + 2.0 * PADDING) / 2.0;
        match self.note_above() {
            None => pure,
            Some(note) => pure + note.calculate_dimension(sb).height + NOTE_SPACE,
        }
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        let pure = (self.text_dim(sb).height + 2.0 * PADDING) / 2.0;
        match self.note_below() {
            None => pure,
            Some(note) => pure + note.calculate_dimension(sb).height + NOTE_SPACE,
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = self.calculate_dimension(sb);
        let dim_text = self.text_dim(sb);
        let dim_box = self.box_dim(sb);
        let line_color = self.style.value(PName::LineColor).as_color();
        let background_color = self.style.value(PName::BackGroundColor).as_color();
        let note_above = self.note_above();
        let note_below = self.note_below();
        let pos_y = note_above
            .as_ref()
            .map_or(0.0, |n| n.calculate_dimension(sb).height + NOTE_SPACE);
        let pos_x_box = (dim.width - dim_box.width) / 2.0;

        let rect = URectangle::build(dim_box.width, dim_box.height);
        if matches!(
            self.symbol,
            Symbol::TerminalString1 | Symbol::TerminalString2
        ) {
            // A terminal is an unfilled box: the ambient background colour would otherwise fill it.
            ug.apply_translate(UTranslate::new(pos_x_box, pos_y))
                .apply_color(line_color)
                .apply_back_color(crate::klimt::color::HColor::TRANSPARENT)
                .apply_stroke(UStroke::with_thickness(0.5))
                .draw(rect);
        } else if self.symbol == Symbol::SpecialSequence {
            ug.apply_translate(UTranslate::new(pos_x_box, pos_y))
                .apply_color(line_color)
                .apply_back_color(crate::klimt::color::HColor::TRANSPARENT)
                .apply_stroke(UStroke::new(5.0, 5.0, 1.0))
                .draw(rect);
        } else {
            ug.apply_translate(UTranslate::new(pos_x_box, pos_y))
                .apply_color(line_color)
                .apply_back_color(background_color)
                .apply_stroke(UStroke::with_thickness(1.5))
                .draw(rect.rounded(10.0));
        }

        let descent = sb.descent(&self.fc.font(), &self.value);
        ug.apply_translate(UTranslate::new(
            PADDING + pos_x_box,
            pos_y + PADDING + dim_text.height - descent,
        ))
        .draw(UText::new(self.value.as_str(), self.fc.clone()));

        if let Some(note) = &note_above {
            let note_dim = note.calculate_dimension(sb);
            note.draw(
                &ug.apply_translate(UTranslate::new((dim.width - note_dim.width) / 2.0, 0.0)),
            );
        }

        if let Some(note) = &note_below {
            let note_dim = note.calculate_dimension(sb);
            note.draw(&ug.apply_translate(UTranslate::new(
                (dim.width - note_dim.width) / 2.0,
                dim.height - note_dim.height,
            )));
        }

        if pos_x_box > 0.0 {
            let line_pos = self.get_h1(sb);
            draw_hline_directed(ug, line_pos, 0.0, pos_x_box, 0.5, 25.0);
            draw_hline_directed(
                ug,
                line_pos,
                pos_x_box + dim_box.width,
                dim.width,
                0.5,
                25.0,
            );
        }
    }

    fn add_comment_above(&mut self, comment: String) {
        self.comment_above = Some(comment);
    }

    fn add_comment_below(&mut self, comment: String) {
        self.comment_below = Some(comment);
    }

    fn repetition_label(&self) -> String {
        self.value.clone()
    }
}
