//! Text style carried by text shapes (`FontConfiguration`, `FontStyle`, `FontPosition`).

use super::{FontFace, UFont};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::UStroke;
use std::collections::BTreeSet;

/// Text decoration and weight flags, in `FontStyle` declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontStyle {
    Plain,
    Italic,
    Bold,
    Underline,
    Strike,
    Wave,
    BackColor,
}

impl FontStyle {
    /// Styles that can carry an extra color (`canHaveExtendedColor`).
    pub fn can_have_extended_color(self) -> bool {
        matches!(
            self,
            FontStyle::Underline | FontStyle::Wave | FontStyle::BackColor | FontStyle::Strike
        )
    }

    /// Bold and italic act on the font itself (`mutateFont`).
    pub fn mutate_font(self, font: &UFont) -> UFont {
        match self {
            FontStyle::Plain => font.with_face(FontFace::NORMAL),
            FontStyle::Italic => font.with_face(FontFace {
                italic: true,
                ..font.face
            }),
            FontStyle::Bold => font.with_face(FontFace {
                weight: 700,
                ..font.face
            }),
            _ => font.clone(),
        }
    }
}

/// Superscript or subscript (`FontPosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontPosition {
    Normal,
    Exposant,
    Indice,
}

impl FontPosition {
    pub fn space(self) -> i32 {
        match self {
            FontPosition::Exposant => -6,
            FontPosition::Indice => 3,
            FontPosition::Normal => 0,
        }
    }

    pub fn mute(self, font: &UFont) -> UFont {
        if self == FontPosition::Normal {
            return font.clone();
        }
        font.with_size((font.size - 3).max(2) as f32)
    }
}

/// Font, color and decorations of a piece of text.
#[derive(Debug, Clone, PartialEq)]
pub struct FontConfiguration {
    styles: BTreeSet<FontStyle>,
    mother_font: UFont,
    mother_color: HColor,
    current_font: UFont,
    current_color: HColor,
    extended_color: Option<HColor>,
    position: FontPosition,
    attributes: Vec<(String, String)>,
    hyperlink_color: HColor,
    hyperlink_underline: Option<UStroke>,
    tab_size: i32,
}

impl FontConfiguration {
    pub fn create(
        font: UFont,
        color: HColor,
        hyperlink_color: HColor,
        hyperlink_underline: Option<UStroke>,
        tab_size: i32,
    ) -> Self {
        let mut styles = BTreeSet::new();
        if font.face.is_bold() {
            styles.insert(FontStyle::Bold);
        }
        if font.face.italic {
            styles.insert(FontStyle::Italic);
        }
        Self {
            styles,
            mother_font: font.clone(),
            mother_color: color.clone(),
            current_font: font,
            current_color: color,
            extended_color: None,
            position: FontPosition::Normal,
            attributes: Vec::new(),
            hyperlink_color,
            hyperlink_underline,
            tab_size,
        }
    }

    /// Black text, blue links (`blackBlueTrue`).
    pub fn black_blue_true(font: UFont) -> Self {
        let blue = HColor::parse_or_white("blue");
        Self::create(
            font,
            HColor::BLACK.with_dark(HColor::WHITE),
            blue,
            Some(UStroke::simple()),
            8,
        )
    }

    /// The effective font: current font mutated by styles and position.
    pub fn font(&self) -> UFont {
        let mut f = self.current_font.clone();
        for s in &self.styles {
            f = s.mutate_font(&f);
        }
        self.position.mute(&f)
    }

    /// Face of the current (unmutated) font.
    pub fn font_face(&self) -> FontFace {
        self.current_font.face
    }

    pub fn with_font_face(&self, face: FontFace) -> Self {
        Self {
            current_font: self.current_font.with_face(face),
            ..self.clone()
        }
    }

    pub fn change_color(&self, color: HColor) -> Self {
        Self {
            current_color: color,
            ..self.clone()
        }
    }

    pub fn change_extended_color(&self, color: Option<HColor>) -> Self {
        Self {
            extended_color: color,
            ..self.clone()
        }
    }

    pub fn change_size(&self, size: f32) -> Self {
        Self {
            current_font: self.current_font.with_size(size),
            ..self.clone()
        }
    }

    pub fn bigger(&self, delta: f64) -> Self {
        self.change_size((f64::from(self.current_font.size) + delta) as f32)
    }

    pub fn change_font_position(&self, position: FontPosition) -> Self {
        Self {
            position,
            ..self.clone()
        }
    }

    pub fn change_family(&self, family: &str) -> Self {
        Self {
            current_font: UFont::new(family, self.current_font.face, self.current_font.size),
            ..self.clone()
        }
    }

    pub fn change_attributes(&self, extra: &[(String, String)]) -> Self {
        let mut attributes = self.attributes.clone();
        for (k, v) in extra {
            match attributes.iter_mut().find(|(a, _)| a == k) {
                Some(e) => e.1 = v.clone(),
                None => attributes.push((k.clone(), v.clone())),
            }
        }
        attributes.sort();
        Self {
            attributes,
            ..self.clone()
        }
    }

    /// Back to the font and color the configuration was created with.
    pub fn reset_font(&self) -> Self {
        Self {
            current_font: self.mother_font.clone(),
            current_color: self.mother_color.clone(),
            extended_color: None,
            position: FontPosition::Normal,
            attributes: Vec::new(),
            ..self.clone()
        }
    }

    /// Replaces the font, keeping color (`forceFont`); no font means italic.
    pub fn force_font(&self, font: Option<UFont>, color: Option<HColor>) -> Self {
        let Some(font) = font else {
            return self.add(FontStyle::Italic);
        };
        let r = Self {
            mother_font: font.clone(),
            current_font: font,
            ..self.clone()
        };
        match color {
            Some(c) => r.change_color(c),
            None => r,
        }
    }

    pub fn add(&self, style: FontStyle) -> Self {
        let mut styles = self.styles.clone();
        if style == FontStyle::Plain {
            styles.clear();
        }
        styles.insert(style);
        Self {
            styles,
            ..self.clone()
        }
    }

    pub fn remove(&self, style: FontStyle) -> Self {
        let mut styles = self.styles.clone();
        styles.remove(&style);
        Self {
            styles,
            ..self.clone()
        }
    }

    pub fn italic(&self) -> Self {
        self.add(FontStyle::Italic)
    }
    pub fn bold(&self) -> Self {
        self.add(FontStyle::Bold)
    }
    pub fn unbold(&self) -> Self {
        self.remove(FontStyle::Bold)
    }
    pub fn unitalic(&self) -> Self {
        self.remove(FontStyle::Italic)
    }
    pub fn underline(&self) -> Self {
        self.add(FontStyle::Underline)
    }
    pub fn wave(&self, color: Option<HColor>) -> Self {
        self.add(FontStyle::Wave).change_extended_color(color)
    }

    pub fn hyperlink(&self) -> Self {
        let base = if self.hyperlink_underline.is_some() {
            self.add(FontStyle::Underline)
        } else {
            self.clone()
        };
        Self {
            current_color: base.hyperlink_color.clone(),
            ..base
        }
    }

    pub fn color(&self) -> &HColor {
        &self.current_color
    }

    pub fn extended_color(&self) -> Option<&HColor> {
        self.extended_color.as_ref()
    }

    pub fn contains_style(&self, style: FontStyle) -> bool {
        self.styles.contains(&style)
    }

    pub fn space(&self) -> i32 {
        self.position.space()
    }

    pub fn attributes(&self) -> &[(String, String)] {
        &self.attributes
    }

    pub fn size2d(&self) -> f64 {
        self.current_font.size2d()
    }

    pub fn tab_size(&self) -> i32 {
        self.tab_size
    }

    pub fn underline_stroke(&self) -> Option<UStroke> {
        self.hyperlink_underline
    }

    /// Text color against the actual background (`adjustColorForBackground`).
    pub fn adjust_color_for_background(&self, back: &HColor) -> Self {
        self.change_color(self.current_color.appropriate_for(back))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fc() -> FontConfiguration {
        FontConfiguration::black_blue_true(UFont::sans_serif(14))
    }

    #[test]
    fn styles_mutate_font() {
        let c = fc();
        assert!(!c.font().face.is_bold());
        let b = c.bold().italic();
        assert!(b.font().face.is_bold() && b.font().face.italic);
        assert!(b.contains_style(FontStyle::Bold));
        assert!(!b.unbold().unitalic().contains_style(FontStyle::Bold));
        assert!(!b.add(FontStyle::Plain).font().face.is_bold());
        assert_eq!(c.bigger(4.0).font().size, 18);
        assert_eq!(c.change_size(10.0).size2d(), 10.0);
        let sup = c.change_font_position(FontPosition::Exposant);
        assert_eq!(sup.font().size, 11);
        assert_eq!(sup.space(), -6);
        assert_eq!(c.change_font_position(FontPosition::Indice).space(), 3);
        assert_eq!(
            c.change_size(3.0)
                .change_font_position(FontPosition::Indice)
                .font()
                .size,
            2
        );
    }

    #[test]
    fn colors_and_reset() {
        let c = fc();
        let red = HColor::parse_or_white("red");
        let r = c
            .change_color(red.clone())
            .change_family("Courier")
            .underline()
            .wave(Some(red.clone()));
        assert_eq!(r.color(), &red);
        assert_eq!(r.extended_color(), Some(&red));
        assert_eq!(r.font().definition(), "Courier");
        let back = r.reset_font();
        assert_eq!(back.color(), c.color());
        assert_eq!(back.font().definition(), "SansSerif");
        assert!(back.extended_color().is_none());
        let h = c.hyperlink();
        assert!(h.contains_style(FontStyle::Underline));
        assert_eq!(h.color(), &HColor::parse_or_white("blue"));
        assert!(
            c.change_attributes(&[("a".into(), "b".into())])
                .change_attributes(&[("a".into(), "c".into())])
                .attributes()
                == [("a".to_string(), "c".to_string())]
        );
        assert_eq!(c.tab_size(), 8);
        assert!(c.underline_stroke().is_some());
        assert!(FontStyle::Wave.can_have_extended_color());
        assert!(!FontStyle::Bold.can_have_extended_color());
        let f = c.force_font(Some(UFont::monospace(9)), Some(red.clone()));
        assert_eq!(f.size2d(), 9.0);
        assert_eq!(f.color(), &red);
        assert!(c.force_font(None, None).contains_style(FontStyle::Italic));
        assert_eq!(c.with_font_face(FontFace::BOLD).font_face(), FontFace::BOLD);
        let adj = c.adjust_color_for_background(&HColor::WHITE);
        assert_eq!(adj.color().rgba(), c.color().rgba());
    }
}
