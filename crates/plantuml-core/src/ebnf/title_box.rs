//! Name of a rule, drawn above its syntax diagram (`TitleBox`).

use crate::klimt::drawing::ugraphic::{UGraphic, UText, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;

/// The left-hand side of a rule, in bold (`TitleBox`).
#[derive(Debug, Clone)]
pub struct TitleBox {
    value: String,
    fc: FontConfiguration,
}

impl TitleBox {
    pub fn new(value: &str, fc: &FontConfiguration) -> Self {
        Self {
            value: value.to_string(),
            fc: fc.bold(),
        }
    }
}

impl TextBlock for TitleBox {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        sb.calculate_dimension(&self.fc.font(), &self.value)
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        let descent = ug.string_bounder().descent(&self.fc.font(), &self.value);
        ug.apply_translate(UTranslate::new(0.0, dim.height - descent))
            .draw(UText::new(self.value.as_str(), self.fc.clone()));
    }
}
