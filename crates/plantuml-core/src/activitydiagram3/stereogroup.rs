//! `<<...>><<...>>` groups written after an element (`Stereogroup`).

use super::ftile::box_style::BoxStyle;
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::style::StyleBuilder;

const KEY: &str = "STEREOGROUP";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stereogroup {
    definition: Option<String>,
}

impl Stereogroup {
    pub fn none() -> Self {
        Self::default()
    }

    /// The optional group of a command regex.
    pub fn optional_stereogroup() -> IRegex {
        IRegex::optional(IRegex::named(KEY, r"(<<[^<>]+>>(?:[%s]*<<[^<>]+>>)*)"))
    }

    pub fn build(arg: &RegexResult) -> Self {
        Self {
            definition: arg.get(KEY, 0).map(str::to_string),
        }
    }

    pub fn from_string(full: &str) -> Self {
        Self {
            definition: Some(full.to_string()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.definition.is_none()
    }

    pub fn build_stereotype(&self) -> Option<Stereotype> {
        self.definition.as_deref().map(Stereotype::build)
    }

    pub fn box_style(&self) -> BoxStyle {
        self.labels()
            .iter()
            .map(|l| BoxStyle::from_string(l))
            .find(|b| *b != BoxStyle::Plain)
            .unwrap_or(BoxStyle::Plain)
    }

    /// The trimmed contents of each `<<...>>`.
    pub fn labels(&self) -> Vec<String> {
        let Some(def) = &self.definition else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut rest = def.as_str();
        while let Some(i) = rest.find("<<") {
            let after = &rest[i + 2..];
            let Some(j) = after.find(">>") else { break };
            let inner = &after[..j];
            if !inner.is_empty() && !inner.contains(['<', '>']) {
                result.push(inner.trim().to_string());
                rest = &after[j + 2..];
            } else {
                rest = &rest[i + 1..];
            }
        }
        result
    }

    /// Colors given inside the group: `#back`, `##line`, `###text`, or an inline `Key:value;` list.
    pub fn inner_colors(&self) -> Colors {
        let mut colors = Colors::empty();
        for label in self.labels() {
            if let Some(l) = label.strip_prefix("###") {
                colors = colors.add(ColorType::Text, HColor::parse(l));
            } else if let Some(mut l) = label.strip_prefix("##") {
                let mut owned = String::new();
                if l.starts_with('[') {
                    if let Some(x) = l.find(']') {
                        colors = colors.add_legacy_stroke(&l[1..x]);
                        owned = l[x + 1..].to_string();
                    }
                    l = &owned;
                }
                if !l.is_empty() {
                    colors = colors.add(ColorType::Line, HColor::parse(l));
                }
            } else if label.starts_with('#') {
                if let Some(c) = Colors::parse(&label, ColorType::Back) {
                    colors = colors.merge_with(Some(&c));
                }
            } else if label.contains(':') && label.contains(';') {
                let text = format!("inlineStereogroupColors {{{label}}}");
                if let Ok(styles) = StyleBuilder::new().parse_style_text(&text) {
                    if let Some(first) = styles.first() {
                        colors = colors.apply_style(first);
                    }
                }
            }
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_box_style() {
        let g = Stereogroup::from_string("<<input>> <<#red>>");
        assert_eq!(g.labels(), ["input", "#red"]);
        assert_eq!(g.box_style(), BoxStyle::SdlInput);
        assert!(Stereogroup::none().labels().is_empty());
        assert!(Stereogroup::none().is_empty());
        assert!(Stereogroup::none().build_stereotype().is_none());
        assert!(g.build_stereotype().is_some());
    }

    #[test]
    fn colors() {
        let back = Stereogroup::from_string("<<#palegreen>>").inner_colors();
        assert_eq!(
            back.color(ColorType::Back),
            HColor::parse("palegreen").as_ref()
        );
        let g = Stereogroup::from_string("<<###red>><<##[dashed]blue>>").inner_colors();
        assert_eq!(g.color(ColorType::Text), HColor::parse("red").as_ref());
        assert_eq!(g.color(ColorType::Line), HColor::parse("blue").as_ref());
        assert!(g.line_style().is_some());
        let inline =
            Stereogroup::from_string("<<FontColor:red;BackGroundColor:blue;>>").inner_colors();
        assert_eq!(
            inline.color(ColorType::Back),
            HColor::parse("blue").as_ref()
        );
        assert_eq!(inline.color(ColorType::Text), HColor::parse("red").as_ref());
        assert!(Stereogroup::from_string("<<plain>>")
            .inner_colors()
            .is_empty());
    }
}
