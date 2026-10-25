//! Skin parameters, ported from `skin/SkinParam` (the settings part).

pub mod arrow;
pub mod component;
pub mod rose;

use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::klimt::creole::{LineBreakStrategy, SkinSimple};
use crate::klimt::geom::HorizontalAlignment;
use crate::stereo::{Guillemet, Stereotype};
use crate::style::skinparam::FromSkinparamToStyle;
use crate::style::{Margins, PName, SName, Style, StyleBuilder, StyleQuery};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static DEFAULT_SKIN: StyleBuilder = StyleBuilder::default_skin();
}

/// Diagram orientation (`Rankdir`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rankdir {
    #[default]
    TopToBottom,
    LeftToRight,
}

/// `skinparam` values plus the style sheet they modify.
#[derive(Debug, Default, Clone)]
pub struct SkinParam {
    params: HashMap<String, String>,
    style_builder: RefCell<Option<StyleBuilder>>,
    pub rankdir: Rankdir,
}

impl std::fmt::Debug for StyleBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StyleBuilder")
    }
}

/// Normalizes a parameter name; `key<<a>><<b>>` expands to one key per stereotype.
fn clean_for_key(key: &str) -> Vec<String> {
    let mut key = key.trim().to_lowercase().replace(['_', '.'], "");
    for (from, to) in [
        ("sequenceparticipant", "participant"),
        ("sequenceactor", "actor"),
    ] {
        key = key.replace(from, to);
    }
    for kind in [
        "activity",
        "class",
        "component",
        "object",
        "sequence",
        "state",
        "usecase",
    ] {
        key = key.replace(&format!("{kind}arrow"), "arrow");
    }
    if key.ends_with("align") {
        key.push_str("ment");
    }
    let mut result = Vec::new();
    let mut base = String::new();
    let mut stereos = Vec::new();
    let mut rest = key.as_str();
    while let Some(i) = rest.find("<<") {
        base.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        match after.find(">>") {
            Some(j) => {
                stereos.push(after[..j].to_string());
                rest = &after[j + 2..];
            }
            None => {
                base.push_str(&rest[i..]);
                rest = "";
                break;
            }
        }
    }
    base.push_str(rest);
    for s in stereos {
        result.push(format!("{base}<<{s}>>"));
    }
    if result.is_empty() {
        result.push(key);
    }
    result
}

impl SkinParam {
    pub fn new() -> Self {
        Self::default()
    }

    /// The current style sheet, loaded on first use.
    pub fn style_builder(&self) -> StyleBuilder {
        self.style_builder
            .borrow_mut()
            .get_or_insert_with(|| DEFAULT_SKIN.with(StyleBuilder::clone))
            .clone()
    }

    pub fn mute_style(&self, styles: Vec<Style>) {
        let current = self.style_builder();
        *self.style_builder.borrow_mut() = Some(current.mute_style(styles));
    }

    /// `skinparam key value`.
    pub fn set_param(&mut self, key: &str, value: &str) {
        for key2 in clean_for_key(key) {
            self.params.insert(key2.clone(), value.trim().to_string());
            let mut builder = self.style_builder();
            let mut conv = FromSkinparamToStyle::new(&key2);
            conv.convert_now(value, &mut builder);
            self.mute_style(conv.into_styles());
        }
        if key.eq_ignore_ascii_case("style") && value.eq_ignore_ascii_case("strictuml") {
            let mut builder = self.style_builder();
            match builder.parse_style_text(include_str!("../style/strictuml.skin")) {
                Ok(styles) => self.mute_style(styles),
                Err(e) => log::error!("cannot load strictuml skin: {e}"),
            }
        }
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        clean_for_key(key)
            .iter()
            .find_map(|k| self.params.get(k))
            .map(String::as_str)
    }

    pub fn get_value_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get_value(key).unwrap_or(default)
    }

    pub fn value_is(&self, key: &str, expected: &str) -> bool {
        self.get_value(key)
            .is_some_and(|v| v.eq_ignore_ascii_case(expected))
    }

    pub fn is_true(&self, key: &str) -> bool {
        self.value_is(key, "true")
    }

    pub fn as_int(&self, key: &str, default: i32) -> i32 {
        match self.get_value(key) {
            Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                v.parse().unwrap_or(default)
            }
            _ => default,
        }
    }

    pub fn strict_uml_style(&self) -> bool {
        self.value_is("style", "strictuml")
    }

    pub fn is_dark(&self) -> bool {
        self.value_is("mode", "dark")
    }

    pub fn dpi(&self) -> i32 {
        let dpi = self.as_int("dpi", 96);
        if dpi <= 0 {
            96
        } else {
            dpi
        }
    }

    pub fn handwritten(&self) -> bool {
        self.is_true("handwritten")
    }

    pub fn tab_size(&self) -> i32 {
        self.as_int("tabsize", 8)
    }

    pub fn monospaced_family(&self) -> String {
        self.get_value_or("defaultMonospacedFontName", "monospaced")
            .to_string()
    }

    pub fn svg_link_target(&self) -> String {
        self.get_value_or("svglinktarget", "_top").to_string()
    }

    pub fn preserve_aspect_ratio(&self) -> String {
        self.get_value_or("preserveaspectratio", "none").to_string()
    }

    pub fn svg_dimension_style(&self) -> bool {
        !self.value_is("svgdimensionstyle", "false")
    }

    /// Padding of text blocks (`skinparam padding`).
    pub fn padding(&self) -> Margins {
        self.margins_of("padding")
    }

    fn margins_of(&self, name: &str) -> Margins {
        match self.get_value(name) {
            Some(v) if is_int_or_decimal(v) => Margins::same(v.parse().unwrap_or(0.0)),
            _ => Margins::same(0.0),
        }
    }

    pub fn max_message_size(&self) -> LineBreakStrategy {
        let v = self
            .get_value("wrapmessagewidth")
            .or_else(|| self.get_value("maxmessagesize"));
        LineBreakStrategy(v.map(str::to_string))
    }

    /// Colors set by `skinparam <param>color` (`getColors`); `None` for an unknown color.
    pub fn get_colors(
        &self,
        param: &str,
        color_type: ColorType,
        stereotype: Option<&Stereotype>,
    ) -> Option<Colors> {
        if let Some(label) = stereotype.and_then(|st| st.label(Guillemet::DoubleComparator)) {
            if let Some(v) = self.get_value(&format!("{param}color{label}")) {
                return Colors::parse(v, color_type);
            }
        }
        match self.get_value(&format!("{param}color")) {
            None => Some(Colors::empty()),
            Some(v) => Colors::parse(v, color_type),
        }
    }

    /// Colors of a note given its stereotype (`applyStereotypeForNote`).
    pub fn colors_for_note(&self, colors: &Colors, stereotype: &Stereotype) -> Option<Colors> {
        let mut result = colors.clone();
        for (param, color_type) in [
            ("notebackground", ColorType::Back),
            ("noteborder", ColorType::Line),
        ] {
            if result.color(color_type).is_none() {
                let found = self.get_colors(param, color_type, Some(stereotype))?;
                result = result.add_colors(&found);
            }
        }
        Some(result.with_shadowing(self.shadowing_for_note(Some(stereotype))))
    }

    pub fn shadowing_for_note(&self, stereotype: Option<&Stereotype>) -> bool {
        if let Some(label) = stereotype.and_then(|st| st.label(Guillemet::DoubleComparator)) {
            if let Some(v) = self.get_value(&format!("noteshadowing{label}")) {
                return v.eq_ignore_ascii_case("true");
            }
        }
        match self.get_value("noteshadowing") {
            Some(v) => v.eq_ignore_ascii_case("true"),
            None => self.shadowing(),
        }
    }

    /// `skinparam defaultTextAlignment`, else `default` (`getDefaultTextAlignment`).
    pub fn default_text_alignment(&self, default: HorizontalAlignment) -> HorizontalAlignment {
        self.get_value("defaulttextalignment")
            .and_then(HorizontalAlignment::from_string)
            .unwrap_or(default)
    }

    /// Alignment of the text of notes (`getHorizontalAlignment(noteTextAlignment, ...)`).
    pub fn note_text_alignment(
        &self,
        override_default: Option<HorizontalAlignment>,
    ) -> HorizontalAlignment {
        self.get_value("notetextalignment")
            .and_then(HorizontalAlignment::from_string)
            .unwrap_or_else(|| {
                self.default_text_alignment(override_default.unwrap_or(HorizontalAlignment::Left))
            })
    }

    pub fn shadowing(&self) -> bool {
        match self.get_value("shadowing") {
            Some(v) if v.eq_ignore_ascii_case("false") => false,
            Some(v) if v.eq_ignore_ascii_case("true") => true,
            _ => !self.strict_uml_style(),
        }
    }

    /// `<<x>>` handling: `None` keeps `<<x>>` (guillemet `false` or `<< >>`).
    pub fn guillemet(&self) -> bool {
        !matches!(self.get_value("guillemet"), Some(v)
            if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("<< >>"))
    }

    pub fn hyperlink_color(&self) -> HColor {
        self.get_value("hyperlinkcolor")
            .and_then(HColor::parse)
            .unwrap_or_else(|| HColor::parse_or_white("blue"))
    }

    /// Background of the whole image (`getBackgroundColor`).
    pub fn background_color(&self) -> HColor {
        if let Some(v) = self.get_value("backgroundcolor") {
            if v.eq_ignore_ascii_case("transparent") || v.eq_ignore_ascii_case("none") {
                return HColor::TRANSPARENT;
            }
            return HColor::parse_or_white(v);
        }
        self.style_builder()
            .merged_style(&StyleQuery::of(&[SName::Document]))
            .map_or(HColor::WHITE, |s| {
                s.value(PName::BackGroundColor).as_color()
            })
    }

    /// Parameters for the creole text engine.
    pub fn skin_simple(&self) -> SkinSimple {
        SkinSimple {
            monospaced_family: self.monospaced_family(),
            padding: self.padding(),
            guillemet: self.guillemet(),
            tab_size: self.tab_size(),
        }
    }
}

fn is_int_or_decimal(s: &str) -> bool {
    let (a, b) = s.split_once('.').map_or((s, None), |(a, b)| (a, Some(b)));
    !a.is_empty()
        && a.bytes().all(|c| c.is_ascii_digit())
        && b.is_none_or(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_cleaning() {
        assert_eq!(
            clean_for_key(" Sequence_Participant.Font "),
            ["participantfont"]
        );
        assert_eq!(clean_for_key("ClassArrowColor"), ["arrowcolor"]);
        assert_eq!(clean_for_key("noteTextAlign"), ["notetextalignment"]);
        assert_eq!(
            clean_for_key("classBackgroundColor<<A>><<B>>"),
            ["classbackgroundcolor<<a>>", "classbackgroundcolor<<b>>"]
        );
        assert_eq!(clean_for_key("x<<"), ["x<<"]);
    }

    #[test]
    fn values_and_styles() {
        let mut p = SkinParam::new();
        p.set_param("ClassBackgroundColor", "red");
        p.set_param("dpi", "300");
        p.set_param("handwritten", "true");
        p.set_param("padding", "4.5");
        p.set_param("shadowing", "false");
        assert_eq!(p.get_value("classbackgroundcolor"), Some("red"));
        assert_eq!(p.get_value_or("zzz", "d"), "d");
        assert_eq!(p.dpi(), 300);
        assert!(p.handwritten());
        assert_eq!(p.padding(), Margins::same(4.5));
        assert!(!p.shadowing());
        let s = p
            .style_builder()
            .merged_style(&StyleQuery::of(&[
                SName::Root,
                SName::Element,
                SName::ClassDiagram,
                SName::Class,
            ]))
            .unwrap();
        assert_eq!(s.value(PName::BackGroundColor).as_string(), "red");
    }

    #[test]
    fn defaults_and_flags() {
        let mut p = SkinParam::new();
        assert_eq!(p.dpi(), 96);
        assert!(p.shadowing());
        assert_eq!(p.tab_size(), 8);
        assert_eq!(p.monospaced_family(), "monospaced");
        assert_eq!(p.svg_link_target(), "_top");
        assert_eq!(p.preserve_aspect_ratio(), "none");
        assert!(p.svg_dimension_style());
        assert!(p.guillemet());
        assert_eq!(p.background_color().rgba(), HColor::WHITE.rgba());
        assert_eq!(p.hyperlink_color(), HColor::parse_or_white("blue"));
        assert_eq!(p.padding(), Margins::same(0.0));
        assert!(!p.is_dark());
        p.set_param("mode", "dark");
        p.set_param("dpi", "0");
        p.set_param("guillemet", "false");
        p.set_param("svgdimensionstyle", "false");
        p.set_param("backgroundcolor", "transparent");
        p.set_param("wrapmessagewidth", "100");
        p.set_param("hyperlinkcolor", "red");
        assert!(p.is_dark() && !p.guillemet() && !p.svg_dimension_style());
        assert_eq!(p.dpi(), 96);
        assert_eq!(p.background_color(), HColor::TRANSPARENT);
        assert_eq!(p.max_message_size().max_width(), 100.0);
        assert_eq!(p.hyperlink_color(), HColor::parse_or_white("red"));
        p.set_param("backgroundcolor", "yellow");
        assert_ne!(p.background_color(), HColor::WHITE);
        assert_eq!(p.skin_simple().tab_size, 8);
        assert_eq!(p.as_int("nodesep", 7), 7);
        assert_eq!(p.rankdir, Rankdir::TopToBottom);
    }

    #[test]
    fn strict_uml() {
        let mut p = SkinParam::new();
        p.set_param("style", "strictUml");
        assert!(p.strict_uml_style());
        assert!(!p.shadowing());
    }

    #[test]
    fn decimal_check() {
        assert!(is_int_or_decimal("3"));
        assert!(is_int_or_decimal("3.5"));
        assert!(!is_int_or_decimal("3."));
        assert!(!is_int_or_decimal("x"));
        assert!(!is_int_or_decimal(""));
    }
}
