//! `hide` and `remove` rules (`HideOrShow`).

use super::{Entity, LeafType};
use crate::plasma::MAGIC_SEPARATOR;
use crate::stereo::Stereotype;

#[derive(Debug, Clone)]
pub struct HideOrShow {
    what: String,
    show: bool,
}

impl HideOrShow {
    pub fn new(what: &str, show: bool) -> HideOrShow {
        HideOrShow {
            what: what.to_string(),
            show,
        }
    }

    pub fn is_about_unlinked(&self) -> bool {
        self.what.eq_ignore_ascii_case("@unlinked")
    }

    /// `is_alone_and_unlinked` is evaluated by the caller (it needs the links).
    fn is_applyable(&self, leaf: &Entity, alone_and_unlinked: &dyn Fn(&Entity) -> bool) -> bool {
        assert!(!leaf.is_root());
        let what = &self.what;
        if let Some(tag) = what.strip_prefix('$') {
            return leaf.stereotags().iter().any(|t| matches(&t.0, tag));
        }
        if what.starts_with("<<") && what.ends_with(">>") {
            return applyable_stereotype(
                leaf.stereotype().as_ref(),
                what[2..what.len() - 2].trim(),
            );
        }
        if self.is_about_unlinked() {
            return alone_and_unlinked(leaf);
        }
        matches(&leaf.qualified_name(), what)
    }

    fn is_applyable_stereotype(&self, stereotype: Option<&Stereotype>) -> bool {
        let what = &self.what;
        if what.starts_with("<<") && what.ends_with(">>") {
            return applyable_stereotype(stereotype, what[2..what.len() - 2].trim());
        }
        false
    }

    pub fn apply(
        &self,
        hidden: bool,
        leaf: &Entity,
        alone_and_unlinked: &dyn Fn(&Entity) -> bool,
    ) -> bool {
        if self.is_applyable(leaf, alone_and_unlinked) {
            return !self.show;
        }
        hidden
    }

    pub fn apply_stereotype(&self, hidden: bool, stereotype: Option<&Stereotype>) -> bool {
        if self.is_applyable_stereotype(stereotype) {
            return !self.show;
        }
        hidden
    }
}

fn applyable_stereotype(stereotype: Option<&Stereotype>, pattern: &str) -> bool {
    match stereotype {
        None => false,
        Some(s) => s.multiple_labels().iter().any(|l| matches(l, pattern)),
    }
}

fn matches(name: &str, pattern: &str) -> bool {
    let name = match name.rfind(MAGIC_SEPARATOR) {
        Some(idx) => &name[idx + 1..],
        None => name,
    };
    if pattern.contains('*') {
        let mut re = String::from("^");
        for (i, part) in pattern.split('*').enumerate() {
            if i > 0 {
                re.push_str(".*");
            }
            re.push_str(&regex::escape(part));
        }
        re.push('$');
        return regex::Regex::new(&re).is_ok_and(|r| r.is_match(name));
    }
    name == pattern
}

#[allow(dead_code)]
fn _unused(_: LeafType) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard() {
        assert!(matches("foo.bar", "foo*"));
        assert!(!matches("foo.bar", "bar"));
        assert!(matches("a\u{1}b", "b"));
        assert!(matches("abc", "abc"));
    }
}
