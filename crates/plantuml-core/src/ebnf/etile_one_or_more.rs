//! One or more repetitions of a tile, drawn as a loop (`ETileOneOrMore`).

use super::brace::Brace;
use super::corner_curved::CornerCurved;
use super::etile::{draw_hline, draw_hline_anti_directed, draw_vline, ETile};
use crate::klimt::drawing::ugraphic::{UGraphic, UText, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::StringBounder;

const DELTA_X: f64 = 15.0;
const DELTA_Y: f64 = 12.0;
/// Height taken by the brace above the loop.
const BRACE_HEIGHT: f64 = 15.0;

/// A tile repeated at least once (`ETileOneOrMore`).
pub struct ETileOneOrMore {
    orig: Box<dyn ETile>,
    /// Label of the brace, used by the `#n` repetition symbol.
    loop_label: Option<(String, FontConfiguration)>,
}

impl ETileOneOrMore {
    /// A plain loop without brace.
    pub fn new(orig: Box<dyn ETile>) -> Self {
        Self {
            orig,
            loop_label: None,
        }
    }

    /// A loop with a brace labelled `label` above it.
    pub fn with_label(orig: Box<dyn ETile>, label: &str, fc: FontConfiguration) -> Self {
        Self {
            orig,
            loop_label: Some((label.to_string(), fc)),
        }
    }

    fn brace_height(&self) -> f64 {
        if self.loop_label.is_some() {
            BRACE_HEIGHT
        } else {
            0.0
        }
    }
}

impl ETile for ETileOneOrMore {
    fn get_h1(&self, sb: &dyn StringBounder) -> f64 {
        DELTA_Y + self.orig.get_h1(sb) + self.brace_height()
    }

    fn get_h2(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_h2(sb)
    }

    fn get_width(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.get_width(sb) + 2.0 * DELTA_X
    }

    fn draw_u(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let full_dim = self.calculate_dimension(sb);
        let h1 = self.get_h1(sb);
        let brace = self.brace_height();

        CornerCurved::sw(8.0).draw(&ug.apply_translate(UTranslate::new(8.0, h1)));
        draw_vline(ug, 8.0, 8.0 + 5.0 + brace, h1 - 8.0);
        CornerCurved::nw(8.0).draw(&ug.apply_translate(UTranslate::new(8.0, 5.0 + brace)));

        draw_hline_anti_directed(ug, 5.0 + brace, DELTA_X, full_dim.width - DELTA_X, 0.6);

        CornerCurved::se(8.0).draw(&ug.apply_translate(UTranslate::new(full_dim.width - 8.0, h1)));
        draw_vline(ug, full_dim.width - 8.0, 8.0 + 5.0 + brace, h1 - 8.0);
        CornerCurved::ne(8.0)
            .draw(&ug.apply_translate(UTranslate::new(full_dim.width - 8.0, 5.0 + brace)));

        draw_hline(ug, h1, 0.0, DELTA_X);
        draw_hline(ug, h1, full_dim.width - DELTA_X, full_dim.width);

        self.orig
            .draw_u(&ug.apply_translate(UTranslate::new(DELTA_X, DELTA_Y + brace)));

        if let Some((label, fc)) = &self.loop_label {
            Brace::new(full_dim.width).draw(&ug.apply_translate(UTranslate::dy(10.0)));
            let dim_text = sb.calculate_dimension(&fc.font(), label);
            let descent = sb.descent(&fc.font(), label);
            ug.apply_translate(UTranslate::new(
                (full_dim.width - dim_text.width) / 2.0,
                descent,
            ))
            .draw(UText::new(label.as_str(), fc.clone()));
        }
    }
}
