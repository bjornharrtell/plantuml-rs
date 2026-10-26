//! The rounded box around a node label (`FtileBoxOld.createWbs` and `createMindMap`).

use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{create_sheet, CreoleMode, SheetBlock};
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{PName, Style};
use std::cell::Cell;

/// A label in a rounded rectangle, at least `minimum_width` wide.
pub struct BoxBlock {
    tb: SheetBlock,
    border: HColor,
    back: HColor,
    stroke: crate::klimt::drawing::ugraphic::UStroke,
    horizontal_alignment: HorizontalAlignment,
    round_corner: f64,
    shadowing: f64,
    minimum_width: Cell<f64>,
}

impl BoxBlock {
    /// `back_color` overrides the background of the style (`SkinParamColors`).
    pub fn new(
        style: &Style,
        skin: &SkinParam,
        label: &Display,
        back_color: Option<&HColor>,
    ) -> Self {
        let style = match back_color {
            Some(c) => {
                style.override_colors(&Colors::empty().add(ColorType::Back, Some(c.clone())))
            }
            None => style.clone(),
        };
        let fc = style.font_configuration(None);
        let horizontal_alignment = style.horizontal_alignment();
        let minimum_width = style.value(PName::MinimumWidth).as_double();
        let lines: Vec<String> = label.lines().iter().map(|l| l.text()).collect();
        let sheet = create_sheet(
            &lines,
            &fc,
            horizontal_alignment,
            &skin.skin_simple(),
            CreoleMode::Full,
        );
        let tb = SheetBlock::new(sheet, style.wrap_width(), style.padding(), 0.0, 0.0)
            .with_minimum_width(minimum_width);
        Self {
            tb,
            border: style.value(PName::LineColor).as_color(),
            back: style.value(PName::BackGroundColor).as_color(),
            stroke: style.stroke(),
            horizontal_alignment,
            round_corner: style.value(PName::RoundCorner).as_double(),
            shadowing: style.shadowing(),
            minimum_width: Cell::new(minimum_width),
        }
    }

    /// Widens the box if needed (`setMinimumWidth`).
    pub fn set_minimum_width(&self, width: f64) {
        self.minimum_width.set(self.minimum_width.get().max(width));
    }

    fn text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.minimum_width
            .get()
            .max(self.tb.calculate_dimension(sb).width)
    }
}

impl TextBlock for BoxBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.tb
            .calculate_dimension(sb)
            .at_least(self.minimum_width.get(), 0.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        let ug = ug
            .apply_color(self.border.clone())
            .apply_back_color(self.back.clone())
            .apply_stroke(self.stroke);
        let mut rect = URectangle::build(dim.width, dim.height).rounded(self.round_corner);
        rect.delta_shadow = self.shadowing;
        ug.draw(rect);
        match self.horizontal_alignment {
            HorizontalAlignment::Left => self.tb.draw(&ug),
            HorizontalAlignment::Right => {
                let dx = dim.width - self.text_width(ug.string_bounder());
                self.tb.draw(&ug.apply_translate(UTranslate::dx(dx)));
            }
            HorizontalAlignment::Center => {
                let dx = (dim.width - self.text_width(ug.string_bounder())) / 2.0;
                self.tb.draw(&ug.apply_translate(UTranslate::dx(dx)));
            }
        }
    }
}
