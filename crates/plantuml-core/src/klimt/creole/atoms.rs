//! Inline pieces of a text line (`creole/atom`, `AtomText`).

use super::layout::SheetBlock;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, URectangle, UStroke, UText, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{TextBlock, UHorizontalLine};
use std::rc::Rc;

/// Horizontal margin of a text atom, resolved when measuring (`DelayedDouble`).
#[derive(Debug, Clone, PartialEq)]
pub enum DelayedDouble {
    Zero,
    /// Indentation of a numbered list item.
    ListNumberLeft(FontConfiguration, i32),
    ListNumberRight(FontConfiguration),
}

impl DelayedDouble {
    fn get(&self, sb: &dyn StringBounder) -> f64 {
        match self {
            DelayedDouble::Zero => 0.0,
            DelayedDouble::ListNumberLeft(fc, order) => {
                sb.calculate_dimension(&fc.font(), "9. ").width * f64::from(*order)
            }
            DelayedDouble::ListNumberRight(fc) => sb.calculate_dimension(&fc.font(), ".").width,
        }
    }
}

/// A run of text with one style.
#[derive(Debug, Clone, PartialEq)]
pub struct AtomText {
    pub text: String,
    pub fc: FontConfiguration,
    pub margin_left: DelayedDouble,
    pub margin_right: DelayedDouble,
}

/// Kind of break opportunity (`NeutronType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeutronType {
    Unbreakable,
    Whitespace,
    CjkIdeograph,
    ZwspSeparator,
    Unknown,
}

fn is_cjk_or_japanese(c: char) -> bool {
    matches!(u32::from(c),
        0x4E00..=0x9FFF | 0x3040..=0x309F | 0x30A0..=0x30FF | 0x3000..=0x303F | 0xFF65..=0xFF9F)
}

/// Break classification of one character.
pub fn neutron_type_of(c: char) -> NeutronType {
    if c.is_whitespace() {
        NeutronType::Whitespace
    } else if is_cjk_or_japanese(c) {
        NeutronType::CjkIdeograph
    } else {
        NeutronType::Unbreakable
    }
}

/// Smallest unit for line breaking (`Neutron`).
#[derive(Debug, Clone)]
pub struct Neutron {
    pub kind: NeutronType,
    pub atom: Option<Atom>,
}

impl Neutron {
    pub fn zwsp() -> Self {
        Self {
            kind: NeutronType::ZwspSeparator,
            atom: None,
        }
    }

    pub fn width(&self, sb: &dyn StringBounder) -> f64 {
        match (&self.kind, &self.atom) {
            (NeutronType::ZwspSeparator, _) | (_, None) => 0.0,
            (_, Some(a)) => a.dimension(sb).width,
        }
    }
}

/// An inline element (`Atom`).
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Atom {
    Text(AtomText),
    /// `----`, `====`, `....` lines, with an optional centered title.
    HLine {
        title: Option<Rc<SheetBlock>>,
        style: char,
    },
    Space(f64),
    Bullet(FontConfiguration, i32),
    /// Atoms drawn left to right (`AtomHorizontalTexts`).
    Horizontal(Vec<Atom>),
    /// Invisible placeholder of the same size (`blank(header)`).
    Blank(Box<Atom>),
}

impl PartialEq for Atom {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Atom::Text(a), Atom::Text(b)) => a == b,
            (Atom::Space(a), Atom::Space(b)) => a == b,
            (Atom::Bullet(f, o), Atom::Bullet(g, p)) => f == g && o == p,
            (Atom::Horizontal(a), Atom::Horizontal(b)) => a == b,
            (Atom::Blank(a), Atom::Blank(b)) => a == b,
            _ => false,
        }
    }
}

/// Replaces `&#NNN;`, `<U+XXXX>`, `~@start` and `\t` (`AtomText.manageSpecialChars`).
fn manage_special_chars(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < c.len() {
        let mut rep: Option<(String, usize)> = None;
        match c[i] {
            '&' if c.get(i + 1) == Some(&'#') => {
                let mut j = i + 2;
                while j < c.len() && c[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 2 && c.get(j) == Some(&';') {
                    let num: String = c[i + 2..j].iter().collect();
                    if let Some(ch) = num.parse::<u32>().ok().and_then(char::from_u32) {
                        rep = Some((ch.to_string(), j + 1 - i));
                    }
                }
            }
            '<' if i + 6 < c.len() && c[i + 1] == 'U' && c[i + 2] == '+' => {
                let mut j = i + 3;
                while j < c.len() && j - (i + 3) < 5 && c[j].is_ascii_hexdigit() {
                    j += 1;
                }
                if j - (i + 3) >= 4 && c.get(j) == Some(&'>') {
                    let hex: String = c[i + 3..j].iter().collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        rep = Some((ch.to_string(), j + 1 - i));
                    }
                }
            }
            '~' if c[i..].iter().take(7).collect::<String>() == "~@start" => {
                rep = Some(("@start".into(), 7));
            }
            '\\' if c.get(i + 1) == Some(&'t') => rep = Some(("\t".into(), 2)),
            _ => {}
        }
        match rep {
            Some((r, n)) => {
                out.push_str(&r);
                i += n;
            }
            None => {
                out.push(c[i]);
                i += 1;
            }
        }
    }
    out
}

impl AtomText {
    /// `AtomTextUtils.createLegacy`: special characters are expanded.
    pub fn legacy(text: &str, fc: FontConfiguration) -> AtomText {
        Self::build(text, fc, DelayedDouble::Zero, DelayedDouble::Zero, true)
    }

    /// `AtomTextUtils.create`: text used verbatim.
    pub fn plain(text: &str, fc: FontConfiguration) -> AtomText {
        Self::build(text, fc, DelayedDouble::Zero, DelayedDouble::Zero, false)
    }

    pub fn build(
        text: &str,
        fc: FontConfiguration,
        margin_left: DelayedDouble,
        margin_right: DelayedDouble,
        special: bool,
    ) -> AtomText {
        let mut s = super::unhide(text);
        if special {
            s = manage_special_chars(&s);
        }
        AtomText {
            text: s,
            fc,
            margin_left,
            margin_right,
        }
    }

    fn tab_string(&self) -> &'static str {
        match self.fc.tab_size() {
            n @ 1..=6 => &"        "[..n as usize],
            _ => "        ",
        }
    }

    fn tab_size(&self, sb: &dyn StringBounder) -> f64 {
        let w = sb
            .calculate_dimension(&self.fc.font(), self.tab_string())
            .width;
        if w == 0.0 {
            self.fc.font().size2d() * 4.0
        } else {
            w
        }
    }

    fn tabbed_width(&self, sb: &dyn StringBounder) -> f64 {
        let tab = self.tab_size(sb);
        let mut x = 0.0;
        for tok in split_tabs(&self.text) {
            if tok == "\t" {
                x += tab - x % tab;
            } else {
                x += sb.calculate_dimension(&self.fc.font(), tok).width;
            }
        }
        x
    }

    pub fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let rect = sb.calculate_dimension(&self.fc.font(), &self.text);
        let h = rect.height.max(10.0);
        let width = if self.text.contains('\t') {
            self.tabbed_width(sb)
        } else {
            rect.width
        };
        Dimension::new(
            width + self.margin_left.get(sb) + self.margin_right.get(sb),
            h,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let back = if ug.back_color().is_transparent() {
            ug.default_background().clone()
        } else {
            ug.back_color().clone()
        };
        let fc = self.fc.adjust_color_for_background(&back);
        let sb = ug.string_bounder();
        let ug = if self.margin_left == DelayedDouble::Zero {
            ug.clone()
        } else {
            ug.apply_translate(UTranslate::dx(self.margin_left.get(sb)))
        };
        let rect = sb.calculate_dimension(&self.fc.font(), &self.text);
        let ypos = rect.height - sb.descent(&self.fc.font(), &self.text);
        let tab = self.tab_size(sb);
        let mut x = 0.0;
        for tok in split_tabs(&self.text) {
            if tok == "\t" {
                x += tab - x % tab;
            } else {
                let dim = sb.calculate_dimension(&self.fc.font(), tok);
                ug.apply_translate(UTranslate::new(x, ypos))
                    .draw(UText::new(tok, fc.clone()));
                x += dim.width;
            }
        }
    }

    /// Splits into break units (`getNeutrons`).
    fn neutrons(&self) -> Vec<Neutron> {
        let mut out = Vec::new();
        let mut pending = String::new();
        let flush = |out: &mut Vec<Neutron>, pending: &str, me: &AtomText| {
            let first = pending.chars().next().expect("not empty");
            let kind = neutron_type_of(first);
            let atom = Atom::Text(AtomText {
                text: pending.to_string(),
                ..me.clone()
            });
            let sep = matches!(kind, NeutronType::Whitespace | NeutronType::CjkIdeograph);
            if sep {
                out.push(Neutron::zwsp());
            }
            out.push(Neutron {
                kind,
                atom: Some(atom),
            });
            if sep {
                out.push(Neutron::zwsp());
            }
        };
        for ch in self.text.chars() {
            if let Some(first) = pending.chars().next() {
                let pt = neutron_type_of(first);
                let ct = neutron_type_of(ch);
                if pt != ct || pt == NeutronType::CjkIdeograph {
                    flush(&mut out, &pending, self);
                    pending.clear();
                }
            }
            pending.push(ch);
        }
        if !pending.is_empty() {
            flush(&mut out, &pending, self);
        }
        out
    }
}

/// Splits keeping tab characters as their own tokens.
fn split_tabs(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if c == '\t' {
            if i > start {
                out.push(&s[start..i]);
            }
            out.push(&s[i..i + 1]);
            start = i + 1;
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

impl Atom {
    pub fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        match self {
            Atom::Text(t) => t.dimension(sb),
            Atom::HLine { title, .. } => match title {
                None => Dimension::new(10.0, 10.0),
                Some(t) => t.calculate_dimension(sb),
            },
            Atom::Space(w) => Dimension::new(*w, 1.0),
            Atom::Bullet(_, order) => {
                if *order == 0 {
                    Dimension::new(12.0, 5.0)
                } else {
                    Dimension::new(8.0 + 8.0 * f64::from(*order), 3.0)
                }
            }
            Atom::Horizontal(all) => all.iter().fold(Dimension::default(), |d, a| {
                let b = a.dimension(sb);
                Dimension::new(d.width + b.width, d.height.max(b.height))
            }),
            Atom::Blank(a) => a.dimension(sb),
        }
    }

    /// Vertical offset of the baseline against other atoms of the same line.
    pub fn starting_altitude(&self) -> f64 {
        match self {
            Atom::Text(t) => f64::from(t.fc.space()),
            Atom::HLine { .. } | Atom::Space(_) => 0.0,
            Atom::Bullet(_, order) => {
                if *order == 0 {
                    -5.0
                } else {
                    -7.0
                }
            }
            Atom::Horizontal(all) => all.first().map_or(0.0, Atom::starting_altitude),
            Atom::Blank(a) => a.starting_altitude(),
        }
    }

    pub fn draw(&self, ug: &UGraphic) {
        match self {
            Atom::Text(t) => t.draw(ug),
            Atom::HLine { title, style } => {
                let dim = self.dimension(ug.string_bounder());
                let line = UHorizontalLine::infinite(
                    1.0,
                    *style,
                    title.clone().map(|t| t as Rc<dyn TextBlock>),
                );
                ug.apply_translate(UTranslate::dy(dim.height / 2.0))
                    .draw(line);
            }
            Atom::Space(_) | Atom::Blank(_) => {}
            Atom::Bullet(fc, order) => {
                let color = fc.color().clone();
                let u = ug.apply_color(color.clone());
                let u = u.apply_back_color(color);
                let u = u.apply_stroke(UStroke::with_thickness(0.0));
                if *order == 0 {
                    u.apply_translate(UTranslate::dx(3.0))
                        .draw(UEllipse::build(5.0, 5.0));
                } else {
                    u.apply_translate(UTranslate::dx(1.0 + 8.0 * f64::from(*order)))
                        .draw(URectangle::build(3.5, 3.5));
                }
            }
            Atom::Horizontal(all) => {
                let sb = ug.string_bounder();
                let alt = self.starting_altitude();
                let mut x = 0.0;
                for a in all {
                    let dim = a.dimension(sb);
                    a.draw(&ug.apply_translate(UTranslate::new(x, alt - a.starting_altitude())));
                    x += dim.width;
                }
            }
        }
    }

    /// Break units of this atom.
    pub fn neutrons(&self) -> Vec<Neutron> {
        match self {
            Atom::Text(t) => t.neutrons(),
            other => vec![Neutron {
                kind: NeutronType::Unknown,
                atom: Some(other.clone()),
            }],
        }
    }

    pub fn as_text(&self) -> Option<&AtomText> {
        match self {
            Atom::Text(t) => Some(t),
            _ => None,
        }
    }
}

/// `AtomTextUtils.createListNumber`.
pub fn list_number(fc: &FontConfiguration, order: i32, local_number: i32) -> Atom {
    Atom::Text(AtomText::build(
        &format!("{}.", local_number + 1),
        fc.clone(),
        DelayedDouble::ListNumberLeft(fc.clone(), order),
        DelayedDouble::ListNumberRight(fc.clone()),
        true,
    ))
}
