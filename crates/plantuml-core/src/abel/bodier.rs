//! The lines inside the braces of an entity (`Bodier`).

use super::member::Member;
use super::LeafType;
use crate::skin::visibility::VisibilityModifier;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::LazyLock;

/// Visibility modifiers hidden by `hide ... members` commands, shared with the diagram.
pub type HiddenVisibilities = Rc<RefCell<HashSet<VisibilityModifier>>>;

static TABLE_LINE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(<#\w+(,#?\w+)?>)?\|(=)?.*\|$").expect("valid"));
static URL_PATTERN: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| crate::regex::compile(&crate::url::regexp()));

#[derive(Debug, Clone, PartialEq)]
pub enum BodierKind {
    /// Fields and methods (`BodierLikeClassOrObject`).
    LikeClassOrObject(LeafType),
    /// Plain lines (`BodierSimple`).
    Simple,
    /// `key => value` rows (`BodierMap`).
    Map,
    /// Not text (`BodierJSon`).
    Json,
}

#[derive(Debug, Clone)]
pub struct Bodier {
    kind: BodierKind,
    raw_body: Vec<String>,
    map: Vec<(String, String)>,
    hide_visibility: HiddenVisibilities,
}

impl Bodier {
    pub fn create_leaf(leaf_type: LeafType, hide_visibility: HiddenVisibilities) -> Bodier {
        let kind = match leaf_type {
            LeafType::Map => BodierKind::Map,
            LeafType::Json => BodierKind::Json,
            t if t.is_like_class() || t == LeafType::Object => BodierKind::LikeClassOrObject(t),
            _ => BodierKind::Simple,
        };
        Bodier {
            kind,
            raw_body: Vec::new(),
            map: Vec::new(),
            hide_visibility,
        }
    }

    pub fn create_group() -> Bodier {
        Bodier {
            kind: BodierKind::Simple,
            raw_body: Vec::new(),
            map: Vec::new(),
            hide_visibility: Rc::default(),
        }
    }

    pub fn kind(&self) -> &BodierKind {
        &self.kind
    }

    pub fn mute_class_to_object(&mut self) {
        if let BodierKind::LikeClassOrObject(t) = &mut self.kind {
            *t = LeafType::Object;
        }
    }

    pub fn raw_body(&self) -> &[String] {
        &self.raw_body
    }

    pub fn map_entries(&self) -> &[(String, String)] {
        &self.map
    }

    /// Adds a line; false when it is not understood (`addFieldOrMethod`).
    pub fn add_field_or_method(&mut self, s: &str) -> bool {
        match self.kind {
            BodierKind::LikeClassOrObject(_) => {
                self.raw_body.push(s.to_string());
                true
            }
            BodierKind::Simple => {
                let d = crate::klimt::creole::display::Display::with_newlines(Some(s));
                for i in 0..d.size() {
                    self.raw_body.push(d.text_at(i));
                }
                true
            }
            BodierKind::Map => {
                if let Some(x) = s.find("=>") {
                    self.map
                        .push((s[..x].trim().to_string(), s[x + 2..].trim().to_string()));
                    true
                } else if let Some(link) = linked_entry(s) {
                    let pos = s.find(&link).unwrap_or(0);
                    self.map
                        .push((s[..pos].trim().to_string(), "\0".to_string()));
                    true
                } else {
                    false
                }
            }
            BodierKind::Json => false,
        }
    }

    pub fn is_object(&self) -> bool {
        self.kind == BodierKind::LikeClassOrObject(LeafType::Object)
    }

    /// Whether the body needs the creole machinery (separators, tables, trees).
    pub fn is_body_enhanced(&self) -> bool {
        self.raw_body.iter().any(|s| {
            crate::klimt::creole::body::is_block_separator(s)
                || TABLE_LINE.is_match(s)
                || (s.len() >= 2 && s.starts_with("|_"))
        })
    }

    fn is_method_line(s: &str) -> bool {
        let purged = URL_PATTERN.replace_all(s, "");
        if purged.contains("{method}") {
            return true;
        }
        if purged.contains("{field}") {
            return false;
        }
        purged.contains('(') || purged.contains(')')
    }

    fn is_method_at(&self, i: usize) -> bool {
        let b = &self.raw_body;
        if i > 0
            && i + 1 < b.len()
            && b[i].is_empty()
            && Self::is_method_line(&b[i - 1])
            && Self::is_method_line(&b[i + 1])
        {
            return true;
        }
        Self::is_method_line(&b[i])
    }

    fn hidden(&self, m: &Member) -> bool {
        match m.visibility_modifier() {
            Some(v) => self.hide_visibility.borrow().contains(&v),
            None => false,
        }
    }

    fn remove_final_empty(result: &mut Vec<Member>) {
        while let Some(last) = result.last() {
            if crate::string_utils::trin(&last.display(false)).is_empty() {
                result.pop();
            } else {
                break;
            }
        }
    }

    pub fn methods_to_display(&self) -> Vec<Member> {
        let mut out: Vec<Member> = Vec::new();
        for (i, s) in self.raw_body.iter().enumerate() {
            if !self.is_method_at(i) {
                continue;
            }
            if s.is_empty() && out.is_empty() {
                continue;
            }
            let m = Member::method(s);
            if !self.hidden(&m) {
                out.push(m);
            }
        }
        Self::remove_final_empty(&mut out);
        out
    }

    pub fn fields_to_display(&self) -> Vec<Member> {
        let mut out: Vec<Member> = Vec::new();
        for s in &self.raw_body {
            if !self.is_object() && Self::is_method_line(s) {
                continue;
            }
            if s.is_empty() && out.is_empty() {
                continue;
            }
            let m = Member::field(s);
            if !self.hidden(&m) {
                out.push(m);
            }
        }
        Self::remove_final_empty(&mut out);
        out
    }

    /// The body lines as members, hidden ones removed (`rawBodyWithoutHidden`).
    pub fn raw_body_without_hidden(&self) -> Vec<String> {
        self.raw_body
            .iter()
            .filter(|s| {
                let m = if Self::is_method_line(s) {
                    Member::method(s)
                } else {
                    Member::field(s)
                };
                !self.hidden(&m)
            })
            .cloned()
            .collect()
    }

    pub fn has_url(&self) -> bool {
        if !matches!(self.kind, BodierKind::LikeClassOrObject(_)) {
            return false;
        }
        self.fields_to_display().iter().any(Member::has_url)
            || self.methods_to_display().iter().any(Member::has_url)
    }

    /// The body line closest to `candidate` (`getBestMatch`).
    pub fn best_match(&self, candidate: &str) -> Option<&str> {
        if candidate.is_empty() {
            return None;
        }
        let mut best: Option<&str> = None;
        let mut best_score = u128::MAX;
        for line in &self.raw_body {
            let score = match_score(line, candidate);
            if score < best_score {
                best = Some(line);
                best_score = score;
                if best_score == 0 {
                    return best;
                }
            }
        }
        best
    }
}

fn linked_entry(s: &str) -> Option<String> {
    static P: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(\*-+_?>)").expect("valid"));
    P.captures(s).map(|c| c[1].to_string())
}

fn is_alphanum(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == '_'
}

fn match_score(full: &str, candidate: &str) -> u128 {
    const BEFORE_STEP: u128 = 1;
    const AFTER_SEP: u128 = 1_000;
    const TRAILING: u128 = 1_000_000;
    const BEFORE_LETTER: u128 = 1_000_000_000;
    let f: Vec<char> = full.chars().collect();
    let c: Vec<char> = candidate.chars().collect();
    if c.is_empty() || f.len() < c.len() {
        return u128::MAX;
    }
    let mut score = 0u128;
    for i in 0..=(f.len() - c.len()) {
        if f[i..i + c.len()] == c[..] {
            let mut separator_seen = false;
            for ch in &f[i + c.len()..] {
                if !separator_seen && is_alphanum(*ch) {
                    score += TRAILING;
                } else {
                    separator_seen = true;
                    score += AFTER_SEP;
                }
            }
            return score;
        }
        score += if f[i].is_alphabetic() {
            BEFORE_LETTER
        } else {
            BEFORE_STEP
        };
    }
    u128::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_methods() {
        let mut b = Bodier::create_leaf(LeafType::Class, Rc::default());
        b.add_field_or_method("-name : String");
        b.add_field_or_method("+getName()");
        b.add_field_or_method("{field} run()");
        assert_eq!(b.fields_to_display().len(), 2);
        assert_eq!(b.methods_to_display().len(), 1);
        assert!(!b.is_body_enhanced());
        b.add_field_or_method("--");
        assert!(b.is_body_enhanced());
    }

    #[test]
    fn hidden_visibility() {
        let hide: HiddenVisibilities = Rc::default();
        let mut b = Bodier::create_leaf(LeafType::Class, hide.clone());
        b.add_field_or_method("-secret");
        b.add_field_or_method("+open");
        hide.borrow_mut().insert(VisibilityModifier::PrivateField);
        assert_eq!(b.fields_to_display().len(), 1);
    }

    #[test]
    fn map_rows() {
        let mut b = Bodier::create_leaf(LeafType::Map, Rc::default());
        assert!(b.add_field_or_method("a => 1"));
        assert!(b.add_field_or_method("b *-> c"));
        assert!(!b.add_field_or_method("nothing"));
        assert_eq!(b.map_entries().len(), 2);
    }

    #[test]
    fn best_match() {
        let mut b = Bodier::create_leaf(LeafType::Class, Rc::default());
        b.add_field_or_method("name : String");
        b.add_field_or_method("other");
        assert_eq!(b.best_match("name"), Some("name : String"));
    }
}
