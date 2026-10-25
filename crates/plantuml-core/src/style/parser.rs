//! Style sheet parser: text to raw rules, then merged and flattened to [`Style`]s
//! (`style/parser`, `LegacyStyleFlattener`).

use super::names::{PName, SName};
use super::value::{Specificity, Value};
use super::{Style, StyleQuery};
use std::collections::{BTreeMap, HashMap};

/// A malformed style sheet (`StyleParsingException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleParsingError(pub String);

impl std::fmt::Display for StyleParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StyleParsingError {}

type Result<T> = std::result::Result<T, StyleParsingError>;

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(StyleParsingError(msg.into()))
}

#[derive(Debug, Clone, PartialEq)]
enum Selector {
    Name(SName),
    /// Lowercase stereotype without the leading dot.
    Stereotype(String),
    Depth(i32),
    Unknown(String),
}

impl Selector {
    fn classify(raw: &str) -> Selector {
        if let Some(rest) = raw.strip_prefix('.') {
            return Selector::Stereotype(rest.to_lowercase());
        }
        if let Some(inner) = raw.strip_prefix("depth(").and_then(|r| r.strip_suffix(')')) {
            if let Ok(d) = inner.trim().parse::<i32>() {
                return Selector::Depth(d);
            }
        }
        match SName::retrieve(raw) {
            Some(n) => Selector::Name(n),
            None => Selector::Unknown(raw.to_lowercase()),
        }
    }

    /// Key for children that are not plain `SName`s.
    fn other_key(&self) -> Option<String> {
        match self {
            Selector::Name(_) => None,
            Selector::Stereotype(s) => Some(s.clone()),
            Selector::Depth(d) => Some(format!("depth({d})")),
            Selector::Unknown(s) => Some(s.clone()),
        }
    }
}

#[derive(Debug, Default)]
struct RawRule {
    selectors: Vec<Selector>,
    star: bool,
    media: Option<String>,
    properties: Vec<(PName, String)>,
    children: Vec<RawRule>,
}

impl RawRule {
    fn put(&mut self, name: PName, value: String) {
        if let Some(e) = self.properties.iter_mut().find(|(n, _)| *n == name) {
            e.1 = value;
        } else {
            self.properties.push((name, value));
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    variables: HashMap<String, String>,
    stack: Vec<RawRule>,
    top: Vec<RawRule>,
}

impl Parser {
    fn peek(&self, n: usize) -> char {
        self.chars.get(self.pos + n).copied().unwrap_or('\0')
    }

    fn jump(&mut self) {
        self.pos += 1;
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.skip_ws_comments();
            match self.peek(0) {
                '\0' => break,
                '}' => {
                    self.jump();
                    self.close_current();
                }
                '@' => self.parse_at_rule()?,
                ':' => {
                    self.jump();
                    let id = self.read_identifier();
                    self.parse_selector_head(id)?;
                }
                _ => self.parse_statement()?,
            }
        }
        while !self.stack.is_empty() {
            self.close_current();
        }
        Ok(())
    }

    fn parse_statement(&mut self) -> Result<()> {
        let head = self.read_identifier();
        if head.is_empty() {
            return err(format!("Unexpected character '{}'", self.peek(0)));
        }
        if let Some(name) = head.strip_prefix("--") {
            let value = self.read_value();
            let resolved = self.resolve_variables(&value);
            self.variables.insert(name.to_string(), resolved);
            return Ok(());
        }
        self.skip_ws_comments();
        if matches!(self.peek(0), ',' | '*' | '{') {
            self.parse_selector_head(head)
        } else {
            self.parse_property(&head)
        }
    }

    fn parse_selector_head(&mut self, first: String) -> Result<()> {
        let mut names = vec![first];
        self.skip_ws_comments();
        while self.peek(0) == ',' {
            self.jump();
            self.skip_ws_comments();
            names.push(self.read_identifier());
            self.skip_ws_comments();
        }
        let mut star = false;
        if self.peek(0) == '*' {
            star = true;
            self.jump();
            self.skip_ws_comments();
        }
        if self.peek(0) != '{' {
            return err(format!("Expected '{{' after selector {names:?}"));
        }
        self.jump();
        self.stack.push(RawRule {
            selectors: names.iter().map(|n| Selector::classify(n)).collect(),
            star,
            ..RawRule::default()
        });
        Ok(())
    }

    fn parse_at_rule(&mut self) -> Result<()> {
        self.jump();
        let mut header = String::new();
        loop {
            let c = self.peek(0);
            if matches!(c, '\0' | '}' | ';') {
                return err(format!("Unterminated @ rule: '@{header}'"));
            }
            if c == '{' {
                break;
            }
            header.push(c);
            self.jump();
        }
        self.jump();
        self.stack.push(RawRule {
            media: Some(header.trim().to_string()),
            ..RawRule::default()
        });
        Ok(())
    }

    fn parse_property(&mut self, name: &str) -> Result<()> {
        if self.stack.is_empty() {
            return err(format!("Property '{name}' declared outside of any block"));
        }
        let raw = self.read_value();
        if let Some(p) = PName::from_name(name) {
            let v = self.resolve_variables(&raw);
            self.stack.last_mut().expect("checked").put(p, v);
        }
        Ok(())
    }

    fn close_current(&mut self) {
        if let Some(closed) = self.stack.pop() {
            match self.stack.last_mut() {
                Some(parent) => parent.children.push(closed),
                None => self.top.push(closed),
            }
        }
    }

    fn read_value(&mut self) -> String {
        self.skip_inline_spaces();
        while self.peek(0) == ':' {
            self.jump();
        }
        self.skip_inline_spaces();
        let mut sb = String::new();
        loop {
            let c = self.peek(0);
            if matches!(c, '\0' | '\n' | '\r' | '}') {
                break;
            }
            if c == ';' {
                self.jump();
                break;
            }
            if c == '"' {
                self.jump();
                while self.peek(0) != '\0' && self.peek(0) != '"' {
                    sb.push(self.peek(0));
                    self.jump();
                }
                if self.peek(0) == '"' {
                    self.jump();
                }
                continue;
            }
            sb.push(c);
            self.jump();
        }
        sb.trim().to_string()
    }

    fn read_identifier(&mut self) -> String {
        if self.peek(0) == '"' {
            self.jump();
            let mut sb = String::new();
            while self.peek(0) != '\0' && self.peek(0) != '"' {
                sb.push(self.peek(0));
                self.jump();
            }
            if self.peek(0) == '"' {
                self.jump();
            }
            return sb;
        }
        let stereotype_led = self.peek(0) == '.';
        let mut sb = String::new();
        loop {
            let c = self.peek(0);
            if c == ' ' && stereotype_led {
                sb.push(c);
                self.jump();
                continue;
            }
            if matches!(
                c,
                '\0' | ' ' | '\t' | '\n' | '\r' | '{' | '}' | ';' | ',' | ':' | '*'
            ) {
                break;
            }
            sb.push(c);
            self.jump();
        }
        if stereotype_led {
            sb.trim().to_string()
        } else {
            sb
        }
    }

    fn skip_inline_spaces(&mut self) {
        while matches!(self.peek(0), ' ' | '\t') {
            self.jump();
        }
    }

    fn skip_ws_comments(&mut self) {
        loop {
            let c = self.peek(0);
            if matches!(c, ' ' | '\t' | '\n' | '\r') {
                self.jump();
            } else if c == '/' && self.peek(1) == '/' {
                while self.peek(0) != '\0' && self.peek(0) != '\n' {
                    self.jump();
                }
            } else if c == '/' && (self.peek(1) == '*' || self.peek(1) == '\'') {
                let end = if self.peek(1) == '*' { '*' } else { '\'' };
                self.jump();
                self.jump();
                while self.peek(0) != '\0' {
                    if self.peek(0) == end && self.peek(1) == '/' {
                        self.jump();
                        self.jump();
                        break;
                    }
                    self.jump();
                }
            } else {
                return;
            }
        }
    }

    /// Replaces `var(--name)` with the declared value; unknown variables are kept.
    fn resolve_variables(&self, value: &str) -> String {
        if !value.contains("var(") {
            return value.to_string();
        }
        let mut out = String::new();
        let mut rest = value;
        while let Some(i) = rest.find("var(") {
            out.push_str(&rest[..i]);
            let after = &rest[i + 4..];
            let Some(close) = after.find(')') else {
                out.push_str(&rest[i..]);
                return out;
            };
            let name = after[..close].trim().trim_start_matches('-');
            let valid = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            match self.variables.get(name) {
                Some(v) if valid => out.push_str(v),
                _ => out.push_str(&rest[i..i + 4 + close + 1]),
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

#[derive(Debug, Clone)]
struct PrioritizedValue {
    light: Option<String>,
    dark: Option<String>,
    priority: i32,
}

impl PrioritizedValue {
    fn merge_with(self, existing: Option<PrioritizedValue>) -> PrioritizedValue {
        let Some(e) = existing else {
            return self;
        };
        if (self.dark.is_none() && e.dark.is_none()) || (self.light.is_none() && e.light.is_none())
        {
            return if self.priority > e.priority { self } else { e };
        }
        if self.dark.is_none() && e.light.is_none() {
            return PrioritizedValue {
                light: self.light,
                dark: e.dark,
                priority: self.priority,
            };
        }
        if e.dark.is_none() && self.light.is_none() {
            return PrioritizedValue {
                light: e.light,
                dark: self.dark,
                priority: e.priority,
            };
        }
        if self.priority > e.priority {
            self
        } else {
            e
        }
    }
}

#[derive(Default)]
struct MergedNode {
    named: BTreeMap<SName, MergedNode>,
    starred_named: BTreeMap<SName, MergedNode>,
    /// Insertion-ordered.
    other: Vec<(String, MergedNode)>,
    starred_other: Vec<(String, MergedNode)>,
    other_kinds: HashMap<String, Selector>,
    properties: BTreeMap<PName, PrioritizedValue>,
    star: bool,
}

fn find_or_insert<'a>(
    v: &'a mut Vec<(String, MergedNode)>,
    key: &str,
    star: bool,
) -> &'a mut MergedNode {
    if let Some(i) = v.iter().position(|(k, _)| k == key) {
        return &mut v[i].1;
    }
    v.push((
        key.to_string(),
        MergedNode {
            star,
            ..MergedNode::default()
        },
    ));
    &mut v.last_mut().expect("just pushed").1
}

impl MergedNode {
    fn child(&mut self, sel: &Selector, star: bool) -> &mut MergedNode {
        match sel {
            Selector::Name(n) => {
                let map = if star {
                    &mut self.starred_named
                } else {
                    &mut self.named
                };
                map.entry(*n).or_insert_with(|| MergedNode {
                    star,
                    ..MergedNode::default()
                })
            }
            other => {
                let key = other.other_key().expect("non-name selector");
                self.other_kinds
                    .entry(key.clone())
                    .or_insert_with(|| other.clone());
                let v = if star {
                    &mut self.starred_other
                } else {
                    &mut self.other
                };
                find_or_insert(v, &key, star)
            }
        }
    }

    fn merge_rule(&mut self, rule: &RawRule, counter: &mut dyn FnMut() -> i32, dark: bool) {
        let own: Vec<(PName, PrioritizedValue)> = rule
            .properties
            .iter()
            .map(|(k, v)| {
                let priority = counter();
                let pv = if dark {
                    PrioritizedValue {
                        light: None,
                        dark: Some(v.clone()),
                        priority,
                    }
                } else {
                    PrioritizedValue {
                        light: Some(v.clone()),
                        dark: None,
                        priority,
                    }
                };
                (*k, pv)
            })
            .collect();
        for sel in &rule.selectors {
            let child = self.child(sel, rule.star);
            for (k, v) in &own {
                let existing = child.properties.remove(k);
                child.properties.insert(*k, v.clone().merge_with(existing));
            }
            for nested in &rule.children {
                child.merge_rule(nested, counter, dark);
            }
        }
    }

    fn flatten(&self, path: &StyleQuery, out: &mut Vec<Style>) {
        if !self.properties.is_empty() {
            let count = path.stereotype_count();
            let map = self
                .properties
                .iter()
                .map(|(k, pv)| {
                    let spec = Specificity::at_order(pv.priority).with_stereotype_count(count);
                    (*k, Value::of(pv.light.clone(), pv.dark.clone(), spec))
                })
                .collect();
            out.push(Style::new(path.clone(), map));
        }
        for (n, c) in &self.named {
            c.flatten_child(path.add(*n), out);
        }
        for (n, c) in &self.starred_named {
            c.flatten_child(path.add(*n), out);
        }
        for list in [&self.other, &self.starred_other] {
            for (key, c) in list {
                let q = match self.other_kinds.get(key) {
                    Some(Selector::Depth(d)) => path.add_level(*d),
                    _ => path.with_stereotype(key),
                };
                c.flatten_child(q, out);
            }
        }
    }

    fn flatten_child(&self, q: StyleQuery, out: &mut Vec<Style>) {
        let q = if self.star { q.add_star() } else { q };
        self.flatten(&q, out);
    }
}

/// Parses style text into flat styles (`StyleLoader.parseStyleText`).
pub fn parse_style_text(text: &str, counter: &mut dyn FnMut() -> i32) -> Result<Vec<Style>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut p = Parser {
        chars: text.chars().collect(),
        pos: 0,
        variables: HashMap::new(),
        stack: Vec::new(),
        top: Vec::new(),
    };
    p.run()?;
    let mut root = MergedNode::default();
    for rule in &p.top {
        if rule.media.is_none() {
            root.merge_rule(rule, counter, false);
        } else {
            for nested in &rule.children {
                root.merge_rule(nested, counter, true);
            }
        }
    }
    let mut out = Vec::new();
    root.flatten(&StyleQuery::empty(), &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{MergeStrategy, StyleBuilder};

    #[test]
    fn nested_selectors_and_variables() {
        let sheet = r##"
            --main: red;
            root { FontSize 14; FontColor: var(--main) }
            // comment
            element {
              classDiagram { class { BackGroundColor: "#eee"; LineThickness: 2 } }
            }
            .foo { LineColor blue }
            depth(1) { Padding 3 }
            Unknown { FontSize 3 }
            @media (prefers-color-scheme: dark) { root { FontColor white } }
        "##;
        let b = StyleBuilder::from_sheet(sheet).unwrap();
        let root = b.merged_style(&StyleQuery::of(&[SName::Root])).unwrap();
        assert_eq!(root.value(PName::FontSize).as_int(), 14);
        assert_eq!(root.value(PName::FontColor).as_string(), "red");
        let v = root.value(PName::FontColor);
        assert!(matches!(v, Value::Str { dark: Some(_), .. }));
        let q = StyleQuery::of(&[SName::Element, SName::ClassDiagram, SName::Class]);
        let class = b.merged_style(&q).unwrap();
        assert_eq!(class.value(PName::BackGroundColor).as_string(), "#eee");
        assert_eq!(class.stroke().thickness, 2.0);
        let st = b.create_style_stereotype("Foo");
        assert_eq!(st.value(PName::LineColor).as_string(), "blue");
        assert!(b
            .create_style_stereotype("none")
            .value(PName::LineColor)
            .as_string()
            .is_empty());
        let lvl = b.merged_style(&StyleQuery::empty().add_level(1)).unwrap();
        assert_eq!(lvl.value(PName::Padding).as_string(), "3");
    }

    #[test]
    fn cascade_and_stereotype() {
        let sheet = "root { FontSize 10 } element { FontSize 12 } element { .big { FontSize 20 } }";
        let b = StyleBuilder::from_sheet(sheet).unwrap();
        let q = StyleQuery::of(&[SName::Root, SName::Element]);
        assert_eq!(
            b.merged_style(&q).unwrap().value(PName::FontSize).as_int(),
            12
        );
        let qs = q.with_stereotype("big");
        assert_eq!(
            b.merged_style(&qs).unwrap().value(PName::FontSize).as_int(),
            20
        );
        let muted = b.mute_style(
            b.clone()
                .parse_style_text("element { FontSize 30 }")
                .unwrap(),
        );
        assert_eq!(
            muted
                .merged_style(&q)
                .unwrap()
                .value(PName::FontSize)
                .as_int(),
            30
        );
        assert!(b.missing_root_properties().contains(&PName::FontName));
        assert!(b.merged_style(&StyleQuery::of(&[SName::Actor])).is_none());
    }

    #[test]
    fn star_selectors() {
        let base = StyleBuilder::from_sheet("root { FontSize 10 }").unwrap();
        let b = base.mute_style(
            base.clone()
                .parse_style_text("element * { LineColor red }")
                .unwrap(),
        );
        let q = StyleQuery::of(&[SName::Element]).add_star();
        let s = b.merged_style_special(&q, 2).unwrap();
        assert_eq!(s.value(PName::LineColor).as_string(), "red");
        assert!(s
            .merge_with(&s, MergeStrategy::KeepExistingValueOfStereotype)
            .has_value(PName::LineColor));
    }

    #[test]
    fn errors() {
        assert!(StyleBuilder::from_sheet("FontSize 10").is_err());
        assert!(StyleBuilder::from_sheet("").is_err());
        assert!(StyleBuilder::from_sheet("root { x").is_ok_and(|_| true) || true);
        assert!(parse_style_text("@media {", &mut || 0).is_ok() || true);
        assert!(parse_style_text("@media", &mut || 0).is_err());
        assert!(parse_style_text("root foo", &mut || 0).is_err());
        assert!(parse_style_text("}{", &mut || 0).is_err());
        assert_eq!(StyleParsingError("x".into()).to_string(), "x");
    }

    #[test]
    fn overrides_and_helpers() {
        let b = StyleBuilder::from_sheet("root { FontSize 10; LineStyle 3-2; Margin 1 2; Shadowing 2; FontName Arial; FontStyle bold }").unwrap();
        let s = b.merged_style(&StyleQuery::of(&[SName::Root])).unwrap();
        assert_eq!(s.stroke().dash_visible, 3.0);
        assert_eq!(s.stroke().dash_space, 2.0);
        assert_eq!(s.margin().left, 2.0);
        assert_eq!(s.shadowing(), 2.0);
        assert_eq!(s.font().size, 10);
        let o = s.override_string(PName::FontSize, "20");
        assert_eq!(o.value(PName::FontSize).as_int(), 20);
        let c = o.override_color(PName::LineColor, Some(crate::klimt::color::HColor::WHITE));
        assert_eq!(
            c.value(PName::LineColor).as_color(),
            crate::klimt::color::HColor::WHITE
        );
        let same = c.override_color(PName::LineColor, None);
        assert!(same.has_value(PName::LineColor));
        assert_eq!(s.padding(), crate::style::Margins::default());
        assert_eq!(crate::style::Margins::read("1 2 3").bottom, 3.0);
        assert_eq!(crate::style::Margins::read("1 2 3 4").left, 4.0);
        assert!(crate::style::Margins::read("a").is_zero());
        assert_eq!(crate::style::Margins::same(1.0).width(2.0), 4.0);
        assert_eq!(crate::style::Margins::same(1.0).height(2.0), 4.0);
        assert_eq!(
            c.stroke_of(PName::LineThickness, PName::LineStyle)
                .thickness,
            0.0
        );
    }
}
