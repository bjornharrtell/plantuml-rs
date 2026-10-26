//! Zero or more repetitions of a tile (`ETileZeroOrMore`).

use super::etile::ETile;
use super::etile_one_or_more::ETileOneOrMore;
use super::etile_optional2::ETileOptional2;
use crate::klimt::font::config::FontConfiguration;
use crate::skin::SkinParam;
use std::rc::Rc;

/// A tile repeated any number of times: an optional [`ETileOneOrMore`] (`ETileZeroOrMore`).
pub struct ETileZeroOrMore {
    optional: ETileOptional2,
}

impl ETileZeroOrMore {
    pub fn new(orig: Box<dyn ETile>, skin: Rc<SkinParam>, fc: FontConfiguration) -> Self {
        Self {
            optional: ETileOptional2::new(Box::new(ETileOneOrMore::new(orig)), skin, fc),
        }
    }
}

impl ETile for ETileZeroOrMore {
    fn get_width(&self, sb: &dyn crate::klimt::font::StringBounder) -> f64 {
        self.optional.get_width(sb)
    }

    fn get_h1(&self, sb: &dyn crate::klimt::font::StringBounder) -> f64 {
        self.optional.get_h1(sb)
    }

    fn get_h2(&self, sb: &dyn crate::klimt::font::StringBounder) -> f64 {
        self.optional.get_h2(sb)
    }

    fn draw_u(&self, ug: &crate::klimt::drawing::ugraphic::UGraphic) {
        self.optional.draw_u(ug);
    }
}
