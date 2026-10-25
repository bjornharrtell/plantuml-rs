//! Stereotypes (`<<...>>`), ported from the `stereo` package.

use crate::klimt::color::HColor;
use regex::Regex;
use std::sync::LazyLock;

static CIRCLE_CHAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^<<\s*\((\S)(?:\s*,\s*(#[0-9a-fA-F]{6}|\w+)\s*)?\)(?:,?(.*?))?>>$")
        .expect("valid")
});
static CIRCLE_SPRITE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^<<\s*\(?\$([\w/]+)((?:\{scale=|\*)([0-9.]+)\}?)?\s*(?:,\s*(#[0-9a-fA-F]{6}|\w+))?\s*(?:[),](.*?))?>>$",
    )
    .expect("valid")
});
static CUT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<{2,3}.*?>{2,3}").expect("valid"));
static MULTIPLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<<\s?((?:<&\w+>|[^<>])+?)\s?>>").expect("valid"));

/// Strictness of guillemet replacement (`Guillemet.manageGuillemetStrict`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guillemet {
    None,
    DoubleComparator,
    Guillemet,
}

impl Guillemet {
    fn marks(self) -> (&'static str, &'static str) {
        match self {
            Guillemet::None => ("", ""),
            Guillemet::DoubleComparator => ("<<", ">>"),
            Guillemet::Guillemet => ("\u{AB}", "\u{BB}"),
        }
    }

    pub fn manage_strict(self, st: &str) -> String {
        if self == Guillemet::DoubleComparator {
            return st.to_string();
        }
        let (start, end) = self.marks();
        let mut st = st.to_string();
        if let Some(r) = st.strip_prefix("<< ") {
            st = format!("{start}{r}");
        } else if let Some(r) = st.strip_prefix("<<") {
            st = format!("{start}{r}");
        }
        if let Some(r) = st.strip_suffix(" >>") {
            st = format!("{r}{end}");
        } else if let Some(r) = st.strip_suffix(">>") {
            st = format!("{r}{end}");
        }
        st
    }
}

/// A stereotype with its optional spot character or sprite (`Stereotype`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stereotype {
    label: String,
    color: Option<HColor>,
    character: char,
    sprite_name: Option<String>,
    sprite_scale: f64,
    radius: f64,
}

impl Stereotype {
    /// `<<label>>` as written in the source.
    pub fn build(label: &str) -> Stereotype {
        assert!(
            label.starts_with("<<") && label.ends_with(">>"),
            "bad stereotype {label}"
        );
        let (sprite_name, sprite_scale) = if label.starts_with("<<$") {
            match CIRCLE_SPRITE.captures(label) {
                Some(c) => (
                    c.get(1).map(|m| m.as_str().to_string()),
                    c.get(3)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(1.0),
                ),
                None => (None, 0.0),
            }
        } else {
            (None, 0.0)
        };
        Stereotype {
            label: label.to_string(),
            color: None,
            character: '\0',
            sprite_name,
            sprite_scale,
            radius: 0.0,
        }
    }

    /// Parses `<<(C,#color) label>>` spots, as used for class entities (`build` with radius).
    pub fn build_complex(label: &str, radius: f64) -> Stereotype {
        assert!(
            label.starts_with("<<") && label.ends_with(">>"),
            "bad stereotype {label}"
        );
        let mut full = String::new();
        let mut color = None;
        let mut character = '\0';
        let mut sprite_name = None;
        let mut sprite_scale = 0.0;
        for name in cut_labels(label, Guillemet::DoubleComparator) {
            let mut name = name;
            let source = name.clone();
            if let Some(c) = CIRCLE_SPRITE.captures(&source) {
                name = match c.get(5).map(|m| m.as_str()) {
                    Some(l) if !l.is_empty() => format!("<<{l}>>"),
                    _ => String::new(),
                };
                color = Some(
                    c.get(4)
                        .and_then(|m| HColor::parse(m.as_str()))
                        .unwrap_or(HColor::BLACK),
                );
                character = '\0';
                sprite_name = c.get(1).map(|m| m.as_str().to_string());
                sprite_scale = c
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1.0);
            } else if let Some(c) = CIRCLE_CHAR.captures(&source) {
                name = match c.get(3).map(|m| m.as_str()) {
                    Some(l) if !l.is_empty() => format!("<<{l}>>"),
                    _ => String::new(),
                };
                color = c.get(2).and_then(|m| HColor::parse(m.as_str()));
                character = c[1].chars().next().unwrap_or('\0');
            }
            full.push_str(&name);
        }
        Stereotype {
            label: full,
            color,
            character,
            sprite_name,
            sprite_scale,
            radius,
        }
    }

    pub fn html_color(&self) -> Option<&HColor> {
        self.color.as_ref()
    }

    pub fn character(&self) -> char {
        self.character
    }

    pub fn is_spotted(&self) -> bool {
        self.character != '\0'
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }

    pub fn sprite_name(&self) -> Option<&str> {
        self.sprite_name.as_deref()
    }

    pub fn sprite_scale(&self) -> f64 {
        self.sprite_scale
    }

    pub fn is_with_oo_symbol(&self) -> bool {
        self.label.eq_ignore_ascii_case("<<O-O>>")
    }

    /// Each `<<x>>` of the label without the markers.
    pub fn multiple_labels(&self) -> Vec<String> {
        MULTIPLE
            .captures_iter(&self.label)
            .map(|c| c[1].to_string())
            .collect()
    }

    pub fn label(&self, guillemet: Guillemet) -> Option<String> {
        if self.is_with_oo_symbol() {
            return None;
        }
        if let Some(rest) = self
            .sprite_name
            .as_deref()
            .and_then(|n| n.strip_prefix("archimate/"))
        {
            return Some(guillemet_apply(guillemet, &format!("<<{rest}>>")));
        }
        Some(guillemet_apply(guillemet, &self.label))
    }

    pub fn labels(&self, guillemet: Guillemet) -> Vec<String> {
        match self.label(Guillemet::DoubleComparator) {
            None => Vec::new(),
            Some(l) => cut_labels(&l, guillemet),
        }
    }

    /// Names usable as `.name` style selectors.
    pub fn style_names(&self) -> Vec<String> {
        let mut result = cut_labels(&self.label, Guillemet::None);
        if let Some(sprite) = &self.sprite_name {
            if let Some(i) = sprite.rfind('/') {
                result.push(sprite[i + 1..].to_string());
            }
        }
        result
    }

    pub fn is_icon(&self) -> bool {
        self.label.eq_ignore_ascii_case("<<icon>>")
    }
}

impl std::fmt::Display for Stereotype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.character == '\0' {
            f.write_str(&self.label)
        } else {
            write!(f, "{} {}", self.character, self.label)
        }
    }
}

fn guillemet_apply(g: Guillemet, st: &str) -> String {
    if g == Guillemet::DoubleComparator || !st.contains('<') {
        return st.to_string();
    }
    let (start, end) = g.marks();
    MULTIPLE
        .replace_all(st, |c: &regex::Captures<'_>| {
            format!("{start}{}{end}", &c[1])
        })
        .into_owned()
}

fn cut_labels(label: &str, guillemet: Guillemet) -> Vec<String> {
    CUT.find_iter(label)
        .map(|m| m.as_str())
        .filter(|g| !g.starts_with("<<<"))
        .map(|g| guillemet.manage_strict(g))
        .collect()
}

/// `<<<name>>>` style references (`Stereostyles`).
pub fn stereostyles(label: &str) -> Vec<String> {
    static P: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<{3}(.*?)>{3}").expect("valid"));
    let mut v: Vec<String> = Vec::new();
    for c in P.captures_iter(label) {
        let n = c[1].to_string();
        if !v.contains(&n) {
            v.push(n);
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_and_multiple() {
        let s = Stereotype::build("<<foo>><<bar>>");
        assert_eq!(s.multiple_labels(), ["foo", "bar"]);
        assert_eq!(s.labels(Guillemet::None), ["foo", "bar"]);
        assert_eq!(
            s.labels(Guillemet::Guillemet),
            ["\u{AB}foo\u{BB}", "\u{AB}bar\u{BB}"]
        );
        assert_eq!(s.style_names(), ["foo", "bar"]);
        assert_eq!(s.to_string(), "<<foo>><<bar>>");
        assert!(!s.is_spotted() && !s.is_icon());
        assert_eq!(
            s.label(Guillemet::Guillemet).unwrap(),
            "\u{AB}foo\u{BB}\u{AB}bar\u{BB}"
        );
        assert!(Stereotype::build("<<O-O>>")
            .label(Guillemet::None)
            .is_none());
        assert!(Stereotype::build("<<O-O>>")
            .labels(Guillemet::None)
            .is_empty());
        assert!(Stereotype::build("<<icon>>").is_icon());
    }

    #[test]
    fn spot_and_sprite() {
        let s = Stereotype::build_complex("<<(C,#ADD1B2) Foo>>", 11.0);
        assert!(s.is_spotted());
        assert_eq!(s.character(), 'C');
        assert!(s.html_color().is_some());
        assert_eq!(s.radius(), 11.0);
        assert_eq!(s.labels(Guillemet::None), ["Foo"]);
        let s = Stereotype::build_complex("<<(X)>>", 0.0);
        assert_eq!(s.character(), 'X');
        assert!(s.labels(Guillemet::None).is_empty());
        let s = Stereotype::build_complex("<<$tag{scale=2},red>>", 0.0);
        assert_eq!(s.sprite_name(), Some("tag"));
        assert_eq!(s.sprite_scale(), 2.0);
        let s = Stereotype::build("<<$arch/foo*3>>");
        assert_eq!(s.sprite_name(), Some("arch/foo"));
        assert_eq!(s.sprite_scale(), 3.0);
        assert_eq!(s.style_names().last().unwrap(), "foo");
        let s = Stereotype::build_complex("<<$archimate/xyz>>", 0.0);
        assert_eq!(s.label(Guillemet::DoubleComparator).unwrap(), "<<xyz>>");
    }

    #[test]
    fn guillemets_and_styles() {
        assert_eq!(
            Guillemet::Guillemet.manage_strict("<< a >>"),
            "\u{AB}a\u{BB}"
        );
        assert_eq!(
            Guillemet::DoubleComparator.manage_strict("<< a >>"),
            "<< a >>"
        );
        assert_eq!(Guillemet::None.manage_strict("<<a>>"), "a");
        assert_eq!(stereostyles("<<<a>>> <<<b>>> <<<a>>>"), ["a", "b"]);
    }

    #[test]
    #[should_panic]
    fn rejects_bad_label() {
        Stereotype::build("foo");
    }
}
