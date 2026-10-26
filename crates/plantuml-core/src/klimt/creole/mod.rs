//! Creole rich text, ported from `klimt/creole`: line parsing into atoms, stripes and
//! sheets, then layout into a [`TextBlock`].

mod atoms;
pub mod body;
pub mod display;
mod layout;
mod parser;

pub use atoms::{Atom, DelayedDouble};
pub use layout::{create_text_block, SheetBlock};
pub use parser::{create_sheet, create_sheet_items, Sheet, SheetItem, Stripe};

use crate::klimt::geom::HorizontalAlignment;
use crate::style::Margins;

/// Which markup is understood (`CreoleMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreoleMode {
    Full,
    SimpleLine,
    NoCreole,
}

/// Rendering parameters from the skin (`ISkinSimple`, reduced).
#[derive(Debug, Clone)]
pub struct SkinSimple {
    pub monospaced_family: String,
    pub padding: Margins,
    /// `<<x>>` becomes `«x»` when true.
    pub guillemet: bool,
    pub tab_size: i32,
    /// Sprites defined by the diagram, drawn where a label says `<$name>`.
    pub sprites: Option<std::rc::Rc<crate::klimt::sprite::SpriteRegistry>>,
}

impl SkinSimple {
    /// The sprite named `name`, when the diagram defined it.
    pub fn sprite(&self, name: &str) -> Option<std::rc::Rc<crate::klimt::sprite::Sprite>> {
        self.sprites.as_ref()?.get(name).cloned()
    }
}

impl Default for SkinSimple {
    fn default() -> Self {
        Self {
            monospaced_family: "monospaced".into(),
            padding: Margins::default(),
            guillemet: true,
            tab_size: 8,
            sprites: None,
        }
    }
}

/// Maximum line width of a block (`LineBreakStrategy`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineBreakStrategy(pub Option<String>);

impl LineBreakStrategy {
    pub const NONE: LineBreakStrategy = LineBreakStrategy(None);

    pub fn new(value: &str) -> Self {
        Self(Some(value.to_string()))
    }

    pub fn is_auto(&self) -> bool {
        self.0
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case("auto"))
    }

    pub fn max_width(&self) -> f64 {
        match &self.0 {
            Some(v) if is_signed_integer(v) => v.parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }
}

fn is_signed_integer(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
}

/// Hides `~` escapes behind private-use characters (`CharHidder`).
pub fn hide(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' && c.get(i + 1) == Some(&'~') {
            out.push(char::from_u32(0xE000 + u32::from('~')).expect("valid"));
            i += 2;
        } else if c[i] == '~' && i + 1 < c.len() {
            let c2 = c[i + 1];
            if matches!(
                c2,
                '_' | '-' | '"' | '#' | ']' | '[' | '*' | '.' | '/' | '<'
            ) {
                out.push(char::from_u32(0xE000 + u32::from(c2)).expect("valid"));
            } else {
                out.push(c[i]);
                out.push(c2);
            }
            i += 2;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// Reverses [`hide`].
pub fn unhide(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{E000}'..='\u{E0FF}').contains(&c) {
                char::from_u32(u32::from(c) - 0xE000).expect("valid")
            } else {
                c
            }
        })
        .collect()
}

/// Splits on `\n` escapes like `Display.getWithNewlines` (`\t` and `\\` also handled).
pub fn with_newlines(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut cur = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' && it.peek().is_some() {
            match it.next().expect("peeked") {
                'n' => result.push(std::mem::take(&mut cur)),
                't' => cur.push('\t'),
                '\\' => cur.push('\\'),
                _ => {}
            }
        } else {
            cur.push(c);
        }
    }
    result.push(cur);
    result
}

/// Default alignment helper for blocks created without one.
pub const DEFAULT_ALIGNMENT: HorizontalAlignment = HorizontalAlignment::Left;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hide_unhide() {
        let h = hide("a~*b~x\\~c");
        assert!(!h.contains('*'));
        assert_eq!(unhide(&h), "a*b~x~c");
        assert_eq!(unhide("plain"), "plain");
    }

    #[test]
    fn newlines_and_strategy() {
        assert_eq!(with_newlines("a\\nb\\tc\\\\d"), ["a", "b\tc\\d"]);
        assert_eq!(with_newlines("x"), ["x"]);
        assert_eq!(LineBreakStrategy::new("120").max_width(), 120.0);
        assert_eq!(LineBreakStrategy::new("-5").max_width(), -5.0);
        assert_eq!(LineBreakStrategy::new("auto").max_width(), 0.0);
        assert!(LineBreakStrategy::new("AUTO").is_auto());
        assert_eq!(LineBreakStrategy::NONE.max_width(), 0.0);
        assert_eq!(SkinSimple::default().tab_size, 8);
    }
}
