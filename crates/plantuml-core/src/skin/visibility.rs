//! Visibility icons of class members (`VisibilityModifier`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, UPolygon, URectangle, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::style::queries::*;
use crate::style::StyleQuery;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibilityModifier {
    PrivateField,
    ProtectedField,
    PackagePrivateField,
    PublicField,
    PrivateMethod,
    ProtectedMethod,
    PackagePrivateMethod,
    PublicMethod,
    IeMandatory,
}

use VisibilityModifier::*;

const ALL: [VisibilityModifier; 9] = [
    PrivateField,
    ProtectedField,
    PackagePrivateField,
    PublicField,
    PrivateMethod,
    ProtectedMethod,
    PackagePrivateMethod,
    PublicMethod,
    IeMandatory,
];

impl VisibilityModifier {
    pub fn regex_for_visibility_character() -> &'static str {
        "[-#+~]"
    }

    /// Constant name used in `data-visibility-modifier`.
    pub fn name(self) -> &'static str {
        match self {
            PrivateField => "PRIVATE_FIELD",
            ProtectedField => "PROTECTED_FIELD",
            PackagePrivateField => "PACKAGE_PRIVATE_FIELD",
            PublicField => "PUBLIC_FIELD",
            PrivateMethod => "PRIVATE_METHOD",
            ProtectedMethod => "PROTECTED_METHOD",
            PackagePrivateMethod => "PACKAGE_PRIVATE_METHOD",
            PublicMethod => "PUBLIC_METHOD",
            IeMandatory => "IE_MANDATORY",
        }
    }

    pub fn unicode(self) -> char {
        match self {
            PrivateField => '\u{EEFB}',
            ProtectedField => '\u{EEFA}',
            PackagePrivateField => '\u{EEF9}',
            PublicField => '\u{EEF8}',
            PrivateMethod => '\u{EEF7}',
            ProtectedMethod => '\u{EEF6}',
            PackagePrivateMethod => '\u{EEF5}',
            PublicMethod => '\u{EEF4}',
            IeMandatory => '\u{EEF3}',
        }
    }

    pub fn get_by_unicode(c: char) -> Option<VisibilityModifier> {
        ALL.iter().copied().find(|m| m.unicode() == c)
    }

    pub fn is_field(self) -> bool {
        matches!(
            self,
            PublicField | PrivateField | ProtectedField | PackagePrivateField
        )
    }

    pub fn is_visibility_character(s: &str) -> bool {
        let mut it = s.chars();
        let (Some(c), Some(c2)) = (it.next(), it.next()) else {
            return false;
        };
        if s.chars().count() <= 2 || c2 == c {
            return false;
        }
        matches!(c, '-' | '#' | '+' | '~' | '*')
    }

    /// `getVisibilityModifier`.
    pub fn from_text(s: &str, is_field: bool) -> Option<VisibilityModifier> {
        let mut it = s.chars();
        let c = it.next()?;
        let c2 = it.next()?;
        if s.chars().count() <= 2 || c2 == c {
            return None;
        }
        Some(match (c, is_field) {
            ('-', true) => PrivateField,
            ('#', true) => ProtectedField,
            ('+', true) => PublicField,
            ('~', true) => PackagePrivateField,
            ('-', false) => PrivateMethod,
            ('#', false) => ProtectedMethod,
            ('+', false) => PublicMethod,
            ('~', false) => PackagePrivateMethod,
            ('*', _) => IeMandatory,
            _ => return None,
        })
    }

    pub fn replace_by_unicode_char(s: &str, is_field: bool) -> String {
        match Self::from_text(s, is_field) {
            None => s.to_string(),
            Some(m) => {
                let mut out = String::new();
                out.push(m.unicode());
                out.extend(s.chars().skip(1));
                out
            }
        }
    }

    /// Style of the icon (`getStyleSignature`).
    pub fn style_query(self) -> StyleQuery {
        match self {
            IeMandatory => VISIBILITYICON_IE_MANDATORY.clone(),
            PublicField | PublicMethod => VISIBILITYICON_PUBLIC.clone(),
            PrivateField | PrivateMethod => VISIBILITYICON_PRIVATE.clone(),
            ProtectedField | ProtectedMethod => VISIBILITYICON_PROTECTED.clone(),
            PackagePrivateField | PackagePrivateMethod => VISIBILITYICON_PACKAGE.clone(),
        }
    }

    /// Icon as a block (`getUBlock`).
    pub fn u_block(
        self,
        size: i32,
        foreground: HColor,
        background: Option<HColor>,
        with_invisible_rectangle: bool,
    ) -> Rc<dyn TextBlock> {
        Rc::new(IconBlock {
            modifier: self,
            size,
            foreground,
            background,
            with_invisible_rectangle,
        })
    }

    fn draw_with_group(
        self,
        ug: &UGraphic,
        size: i32,
        foreground: &HColor,
        background: &Option<HColor>,
    ) {
        ug.start_group(&[("data-visibility-modifier", self.name().to_string())]);
        self.draw_internal(ug, size, foreground, background);
        ug.close_group();
    }

    fn draw_internal(
        self,
        ug: &UGraphic,
        size: i32,
        foreground: &HColor,
        background: &Option<HColor>,
    ) {
        let ug = match background {
            None => ug.apply_back_color(HColor::TRANSPARENT),
            Some(b) => ug.apply_back_color(b.clone()),
        };
        let ug = ug.apply_color(foreground.clone());
        let size = if size % 2 == 1 { size - 1 } else { size };
        match self {
            PackagePrivateField | PackagePrivateMethod => draw_triangle(&ug, size),
            PrivateField | PrivateMethod => draw_square(&ug, size),
            ProtectedField | ProtectedMethod => draw_diamond(&ug, size),
            PublicField | PublicMethod | IeMandatory => draw_circle(&ug, size),
        }
    }
}

fn draw_square(ug: &UGraphic, size: i32) {
    let s = f64::from(size - 4);
    ug.apply_translate(UTranslate::new(2.0, 2.0))
        .draw(URectangle::build(s, s));
}

fn draw_circle(ug: &UGraphic, size: i32) {
    let s = f64::from(size - 4);
    ug.apply_translate(UTranslate::new(2.0, 2.0))
        .draw(UEllipse::build(s, s));
}

fn draw_diamond(ug: &UGraphic, size: i32) {
    let size = f64::from(size - 2);
    let mut poly = UPolygon::default();
    poly.add_point(size / 2.0, 0.0);
    poly.add_point(size, size / 2.0);
    poly.add_point(size / 2.0, size);
    poly.add_point(0.0, size / 2.0);
    ug.apply_translate(UTranslate::new(1.0, 0.0)).draw(poly);
}

fn draw_triangle(ug: &UGraphic, size: i32) {
    let size = f64::from(size - 2);
    let mut poly = UPolygon::default();
    poly.add_point(size / 2.0, 1.0);
    poly.add_point(0.0, size - 1.0);
    poly.add_point(size, size - 1.0);
    ug.apply_translate(UTranslate::new(1.0, 0.0)).draw(poly);
}

struct IconBlock {
    modifier: VisibilityModifier,
    size: i32,
    foreground: HColor,
    background: Option<HColor>,
    with_invisible_rectangle: bool,
}

impl TextBlock for IconBlock {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(f64::from(self.size + 1), f64::from(self.size + 1))
    }

    fn draw(&self, ug: &UGraphic) {
        if self.with_invisible_rectangle {
            ug.apply_color(HColor::TRANSPARENT).draw(URectangle::build(
                f64::from(self.size * 2),
                f64::from(self.size),
            ));
        }
        self.modifier
            .draw_with_group(ug, self.size, &self.foreground, &self.background);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(
            VisibilityModifier::from_text("-foo", true),
            Some(PrivateField)
        );
        assert_eq!(
            VisibilityModifier::from_text("+foo()", false),
            Some(PublicMethod)
        );
        assert_eq!(VisibilityModifier::from_text("--x", true), None);
        assert_eq!(VisibilityModifier::from_text("-a", true), None);
        assert!(VisibilityModifier::is_visibility_character("*abc"));
        assert!(!VisibilityModifier::is_visibility_character("abc"));
        assert_eq!(
            VisibilityModifier::get_by_unicode('\u{EEFB}'),
            Some(PrivateField)
        );
        assert_eq!(
            VisibilityModifier::replace_by_unicode_char("-ab", true),
            "\u{EEFB}ab"
        );
    }
}
