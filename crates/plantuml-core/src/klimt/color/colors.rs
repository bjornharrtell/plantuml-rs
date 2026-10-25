//! Per-element color overrides (`Colors`, `ColorType`, `LinkStyle`).

use super::HColor;
use crate::klimt::drawing::ugraphic::UStroke;
use crate::style::{PName, Style};
use std::collections::BTreeMap;

/// Which part of an element a color applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorType {
    Text,
    Line,
    Back,
    Header,
    Arrow,
}

impl ColorType {
    /// `type` or `type.something` (`ColorType.getType`).
    pub fn parse(s: &str) -> Option<ColorType> {
        let s = s.split('.').next().unwrap_or(s);
        match s.to_uppercase().as_str() {
            "TEXT" => Some(ColorType::Text),
            "LINE" => Some(ColorType::Line),
            "BACK" => Some(ColorType::Back),
            "HEADER" => Some(ColorType::Header),
            "ARROW" => Some(ColorType::Arrow),
            _ => None,
        }
    }
}

/// Line style keyword (`LinkStyle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkStyle {
    kind: LinkKind,
    thickness: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkKind {
    Normal,
    Dashed,
    Dotted,
    Bold,
    Invisible,
}

impl LinkStyle {
    pub const NORMAL: LinkStyle = LinkStyle::of(LinkKind::Normal);
    pub const DASHED: LinkStyle = LinkStyle::of(LinkKind::Dashed);
    pub const DOTTED: LinkStyle = LinkStyle::of(LinkKind::Dotted);
    pub const BOLD: LinkStyle = LinkStyle::of(LinkKind::Bold);
    pub const INVISIBLE: LinkStyle = LinkStyle::of(LinkKind::Invisible);

    const fn of(kind: LinkKind) -> Self {
        Self {
            kind,
            thickness: None,
        }
    }

    pub fn is_normal(&self) -> bool {
        self.kind == LinkKind::Normal
    }

    pub fn is_invisible(&self) -> bool {
        self.kind == LinkKind::Invisible
    }

    pub fn go_thickness(self, thickness: f64) -> Self {
        Self {
            thickness: Some(thickness),
            ..self
        }
    }

    pub fn is_thickness_overridden(&self) -> bool {
        self.thickness.is_some()
    }

    fn non_zero_thickness(&self) -> f64 {
        self.thickness.unwrap_or(1.0)
    }

    pub fn stroke(&self) -> UStroke {
        match self.kind {
            LinkKind::Dashed => UStroke::new(7.0, 7.0, self.non_zero_thickness()),
            LinkKind::Dotted => UStroke::new(1.0, 3.0, self.non_zero_thickness()),
            LinkKind::Bold => UStroke::with_thickness(2.0),
            _ => UStroke::with_thickness(self.non_zero_thickness()),
        }
    }

    pub fn mute_stroke(&self, stroke: UStroke) -> UStroke {
        match self.kind {
            LinkKind::Dashed | LinkKind::Dotted | LinkKind::Bold => self.stroke(),
            _ => stroke,
        }
    }

    /// `fromString2`: unknown keywords give `None`.
    pub fn parse(s: &str) -> Option<LinkStyle> {
        match s.to_lowercase().as_str() {
            "dashed" => Some(Self::DASHED),
            "dotted" => Some(Self::DOTTED),
            "bold" => Some(Self::BOLD),
            "hidden" => Some(Self::INVISIBLE),
            _ => None,
        }
    }

    /// `fromString1`: unknown keywords give a normal line.
    pub fn parse_or_normal(s: &str) -> LinkStyle {
        Self::parse(s).unwrap_or(Self::NORMAL)
    }
}

/// Colors given with `#back;line:red;text:blue;line.dashed` (`Colors`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Colors {
    map: BTreeMap<ColorType, Option<HColor>>,
    line_style: Option<LinkStyle>,
    shadowing: Option<bool>,
}

impl Colors {
    pub fn empty() -> Colors {
        Colors::default()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Parses the `;`-separated settings; `main_type` receives bare colors.
    pub fn parse(data: &str, main_type: ColorType) -> Option<Colors> {
        let data = data.to_lowercase().replace('#', "");
        let mut c = Colors::empty();
        for s in data.split(';').filter(|s| !s.is_empty()) {
            match s.split_once(':') {
                None => {
                    if !s.contains('.') {
                        c.map.insert(main_type, Some(HColor::parse(s)?));
                    }
                }
                Some((name, value)) => {
                    if name.eq_ignore_ascii_case("shadowing") {
                        c.shadowing = Some(value.eq_ignore_ascii_case("true"));
                    } else {
                        let key = ColorType::parse(name)?;
                        c.map.insert(key, HColor::parse(value));
                    }
                }
            }
        }
        if data.contains("line.dashed") {
            c.line_style = Some(LinkStyle::DASHED);
        } else if data.contains("line.dotted") {
            c.line_style = Some(LinkStyle::DOTTED);
        } else if data.contains("line.bold") {
            c.line_style = Some(LinkStyle::BOLD);
        }
        Some(c)
    }

    pub fn merge_with(&self, other: Option<&Colors>) -> Colors {
        let Some(o) = other else {
            return self.clone();
        };
        let mut r = self.clone();
        r.map.extend(o.map.iter().map(|(k, v)| (*k, v.clone())));
        if o.line_style.is_some() {
            r.line_style = o.line_style;
        }
        r
    }

    pub fn color(&self, key: ColorType) -> Option<&HColor> {
        self.map.get(&key).and_then(Option::as_ref)
    }

    pub fn color_or(&self, key1: ColorType, key2: ColorType) -> Option<&HColor> {
        self.color(key1).or_else(|| self.color(key2))
    }

    pub fn add(&self, key: ColorType, color: Option<HColor>) -> Colors {
        let Some(color) = color else {
            return self.clone();
        };
        let mut r = self.clone();
        r.map.insert(key, Some(color));
        r
    }

    pub fn line_style(&self) -> Option<LinkStyle> {
        self.line_style
    }

    pub fn specific_line_stroke(&self) -> Option<UStroke> {
        self.line_style.map(|s| s.stroke())
    }

    pub fn mute_stroke(&self, stroke: UStroke) -> UStroke {
        match self.line_style {
            None => stroke,
            Some(s) => s.mute_stroke(stroke),
        }
    }

    pub fn shadowing(&self) -> Option<bool> {
        self.shadowing
    }

    pub fn with_shadowing(&self, shadowing: bool) -> Colors {
        Colors {
            shadowing: Some(shadowing),
            ..self.clone()
        }
    }

    pub fn add_legacy_stroke(&self, s: &str) -> Colors {
        Colors {
            line_style: Some(LinkStyle::parse_or_normal(s)),
            ..self.clone()
        }
    }

    /// Takes back, line and text colors from a style (`applyStyle`).
    pub fn apply_style(&self, style: &Style) -> Colors {
        let back = style.value(PName::BackGroundColor).as_color();
        let line = style.value(PName::LineColor).as_color();
        let text = style.value(PName::FontColor).as_color();
        self.add(ColorType::Back, Some(back))
            .add(ColorType::Line, Some(line.clone()))
            .add(ColorType::Arrow, Some(line))
            .add(ColorType::Text, Some(text))
    }

    /// Color for `name`: the override when set, else the style value.
    pub fn color_for(&self, style: &Style, name: PName) -> HColor {
        if name == PName::BackGroundColor {
            if let Some(c) = self.color(ColorType::Back) {
                return c.clone();
            }
        }
        if name == PName::LineColor {
            if let Some(c) = self.color(ColorType::Line) {
                return c.clone();
            }
        }
        style.value(name).as_color()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colors() {
        let c = Colors::parse(
            "#red;line:blue;text:#00FF00;line.dashed;shadowing:true",
            ColorType::Back,
        )
        .unwrap();
        assert_eq!(c.color(ColorType::Back), HColor::parse("red").as_ref());
        assert_eq!(c.color(ColorType::Line), HColor::parse("blue").as_ref());
        assert!(c.color(ColorType::Text).is_some());
        assert_eq!(c.line_style(), Some(LinkStyle::DASHED));
        assert_eq!(c.shadowing(), Some(true));
        assert!(c.color_or(ColorType::Header, ColorType::Line).is_some());
        assert!(Colors::parse("nosuchcolorxyz", ColorType::Back).is_none());
        assert!(Colors::parse("bogus:red", ColorType::Back).is_none());
        let d = Colors::parse("line.dotted", ColorType::Back).unwrap();
        assert!(d.is_empty() && d.line_style() == Some(LinkStyle::DOTTED));
        assert_eq!(
            Colors::parse("line.bold", ColorType::Back)
                .unwrap()
                .line_style(),
            Some(LinkStyle::BOLD)
        );
    }

    #[test]
    fn merge_and_strokes() {
        let a = Colors::empty().add(ColorType::Back, HColor::parse("red"));
        let b = Colors::empty()
            .add(ColorType::Line, HColor::parse("blue"))
            .add_legacy_stroke("dotted")
            .with_shadowing(false);
        let m = a.merge_with(Some(&b));
        assert!(m.color(ColorType::Back).is_some() && m.color(ColorType::Line).is_some());
        assert_eq!(a.merge_with(None), a);
        assert_eq!(
            m.specific_line_stroke().unwrap(),
            UStroke::new(1.0, 3.0, 1.0)
        );
        assert_eq!(
            m.mute_stroke(UStroke::simple()),
            UStroke::new(1.0, 3.0, 1.0)
        );
        assert_eq!(a.mute_stroke(UStroke::simple()), UStroke::simple());
        assert_eq!(a.add(ColorType::Text, None), a);
        assert_eq!(b.shadowing(), Some(false));
        assert_eq!(m.shadowing(), None);
        assert_eq!(a.specific_line_stroke(), None);
        assert_eq!(ColorType::parse("line.x"), Some(ColorType::Line));
        assert_eq!(ColorType::parse("zzz"), None);
    }

    #[test]
    fn link_styles() {
        assert_eq!(LinkStyle::parse("DASHED"), Some(LinkStyle::DASHED));
        assert_eq!(LinkStyle::parse("hidden"), Some(LinkStyle::INVISIBLE));
        assert!(LinkStyle::parse("x").is_none());
        assert!(LinkStyle::parse_or_normal("x").is_normal());
        assert!(LinkStyle::INVISIBLE.is_invisible());
        let t = LinkStyle::NORMAL.go_thickness(3.0);
        assert!(t.is_thickness_overridden());
        assert_eq!(t.stroke(), UStroke::with_thickness(3.0));
        assert_eq!(LinkStyle::BOLD.stroke(), UStroke::with_thickness(2.0));
        assert_eq!(LinkStyle::DASHED.stroke(), UStroke::new(7.0, 7.0, 1.0));
        assert_eq!(
            LinkStyle::NORMAL.mute_stroke(UStroke::simple()),
            UStroke::simple()
        );
    }

    #[test]
    fn from_style() {
        let b = crate::style::StyleBuilder::default_skin();
        let s = b
            .merged_style(&crate::style::StyleQuery::of(&[crate::style::SName::Root]))
            .unwrap();
        let c = Colors::empty().apply_style(&s);
        assert!(c.color(ColorType::Text).is_some() && c.color(ColorType::Arrow).is_some());
        let red = Colors::empty().add(ColorType::Back, HColor::parse("red"));
        assert_eq!(
            red.color_for(&s, PName::BackGroundColor),
            HColor::parse("red").unwrap()
        );
        assert_ne!(
            red.color_for(&s, PName::LineColor),
            HColor::parse("red").unwrap()
        );
    }
}
