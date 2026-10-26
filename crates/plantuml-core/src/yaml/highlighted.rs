//! `#highlight` lines (`yaml/Highlighted`).

use crate::stereo::Stereotype;
use crate::string_utils::eventually_remove_starting_and_ending_double_quote;
use regex::Regex;
use std::sync::LazyLock;

const HIGHLIGHTED: &str = "#highlight ";

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([^<>]+)(<<.*>>)?$").expect("valid"));

/// A path of keys to highlight, with an optional stereotype.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlighted {
    paths: Vec<String>,
    stereotype: Option<Stereotype>,
}

impl Highlighted {
    pub fn matches_definition(line: &str) -> bool {
        line.starts_with(HIGHLIGHTED)
    }

    /// Parses a definition line; `None` when it is malformed.
    pub fn build(line: &str) -> Option<Highlighted> {
        let line = line.strip_prefix(HIGHLIGHTED)?.trim();
        let m = PATTERN.captures(line)?;
        let paths = m[1].trim();
        let stereotype = m.get(2).map(|s| Stereotype::build(s.as_str()));
        Some(Highlighted {
            paths: paths
                .split('/')
                .map(|s| eventually_remove_starting_and_ending_double_quote(s.trim()))
                .collect(),
            stereotype,
        })
    }

    /// The highlight as seen from the child named `key`.
    pub fn up_one_level(&self, key: &str) -> Option<Highlighted> {
        if self.paths.len() <= 1 {
            return None;
        }
        let first = &self.paths[0];
        if first == "**" {
            return Some(self.clone());
        }
        if first == "*" || first == key {
            return Some(Highlighted {
                paths: self.paths[1..].to_vec(),
                stereotype: self.stereotype.clone(),
            });
        }
        None
    }

    pub fn is_key_highlight(&self, key: &str) -> bool {
        if self.paths.len() == 2 && self.paths[0] == "**" && self.paths[1] == key {
            return true;
        }
        self.paths.len() == 1 && self.paths[0] == key
    }

    pub fn stereotype(&self) -> Option<&Stereotype> {
        self.stereotype.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert!(Highlighted::matches_definition("#highlight a"));
        assert!(!Highlighted::matches_definition("# highlight a"));
        let h = Highlighted::build("#highlight \"a b\" / * / c <<red>>").unwrap();
        assert!(h.stereotype().is_some());
        assert!(!h.is_key_highlight("c"));
        let h = h.up_one_level("a b").unwrap();
        let h = h.up_one_level("anything").unwrap();
        assert!(h.is_key_highlight("c"));
        assert!(h.up_one_level("c").is_none());
        let h = Highlighted::build("#highlight ** / x").unwrap();
        assert!(h.is_key_highlight("x"));
        assert_eq!(h.up_one_level("k"), Some(h.clone()));
        let h = Highlighted::build("#highlight a / b").unwrap();
        assert!(h.up_one_level("z").is_none());
        assert!(Highlighted::build("#highlight <<x>>").is_none());
        assert!(Highlighted::build("plain").is_none());
    }
}
