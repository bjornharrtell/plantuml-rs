//! A field or a method of a class (`Member`).

use crate::skin::visibility::VisibilityModifier;
use crate::stereo::Guillemet;
use crate::url::{Url, UrlBuilder, UrlMode};
use std::sync::LazyLock;

static URL_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    crate::regex::compile(&format!("^(.*?)(?:\\[({})\\])?$", crate::url::regexp()))
});
static REMOVE_TAG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\{(method|field)\}\s*").expect("valid"));
static REMOVE_STATIC: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\{(static|classifier|abstract)\}\s*").expect("valid"));

/// A line of a class body with its modifiers.
#[derive(Debug, Clone)]
pub struct Member {
    raw: String,
    display: String,
    static_modifier: bool,
    abstract_modifier: bool,
    url: Option<Url>,
    visibility: Option<VisibilityModifier>,
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.display == other.display
    }
}

impl Member {
    pub fn method(display: &str) -> Member {
        Member::build(true, display, true)
    }

    pub fn field(display: &str) -> Member {
        Member::build(true, display, false)
    }

    pub fn method_with(display: &str, manage_modifier: bool) -> Member {
        Member::build(manage_modifier, display, true)
    }

    pub fn field_with(display: &str, manage_modifier: bool) -> Member {
        Member::build(manage_modifier, display, false)
    }

    fn build(manage_modifier: bool, tmp_display: &str, is_method: bool) -> Member {
        let raw = tmp_display.to_string();
        let mut tmp = REMOVE_TAG.replace_all(tmp_display, "").into_owned();
        let mut url = None;
        if manage_modifier {
            if let Some(caps) = URL_RE.captures(&tmp).ok().flatten() {
                let first = caps.get(1).map(|m| m.as_str().to_string());
                let url_string = caps.get(2).map(|m| m.as_str().to_string());
                if let Some(f) = first {
                    tmp = f;
                }
                if let Some(u) = url_string {
                    url = UrlBuilder::new(None, UrlMode::Strict).url(&u);
                }
            }
        }
        let lower = tmp.to_lowercase();
        if manage_modifier {
            let static_modifier = lower.contains("{static}") || lower.contains("{classifier}");
            let abstract_modifier = lower.contains("{abstract}");
            let mut clean = REMOVE_STATIC.replace_all(&tmp, "").trim().to_string();
            if clean.is_empty() {
                clean = " ".to_string();
            }
            let (display, visibility) = if VisibilityModifier::is_visibility_character(&clean) {
                let vm = VisibilityModifier::from_text(&clean, !is_method);
                let rest: String = clean.chars().skip(1).collect();
                (
                    crate::string_utils::trin(&manage_guillemet(&rest)).to_string(),
                    vm,
                )
            } else {
                (manage_guillemet(&clean), None)
            };
            Member {
                raw,
                display,
                static_modifier,
                abstract_modifier,
                url,
                visibility,
            }
        } else {
            let t = crate::string_utils::trin(&tmp).to_string();
            let display = if t.is_empty() {
                " ".to_string()
            } else {
                manage_guillemet(&t)
            };
            Member {
                raw,
                display,
                static_modifier: false,
                abstract_modifier: false,
                url: None,
                visibility: None,
            }
        }
    }

    /// The source text.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn display(&self, with_visibility_char: bool) -> String {
        if !with_visibility_char {
            return self.display.clone();
        }
        use VisibilityModifier::*;
        let prefix = match self.visibility {
            Some(PrivateField | PrivateMethod) => "-",
            Some(PublicField | PublicMethod) => "+",
            Some(PackagePrivateField | PackagePrivateMethod) => "~",
            Some(ProtectedField | ProtectedMethod) => "#",
            Some(IeMandatory) => "*",
            None => "",
        };
        format!("{prefix}{}", self.display)
    }

    pub fn is_static(&self) -> bool {
        self.static_modifier
    }

    pub fn is_abstract(&self) -> bool {
        self.abstract_modifier
    }

    pub fn visibility_modifier(&self) -> Option<VisibilityModifier> {
        self.visibility
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    pub fn has_url(&self) -> bool {
        self.url.is_some()
    }
}

fn manage_guillemet(s: &str) -> String {
    let _ = Guillemet::Guillemet;
    crate::klimt::creole::display::manage_guillemet_text(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers() {
        let m = Member::field("-name : String");
        assert_eq!(
            m.visibility_modifier(),
            Some(VisibilityModifier::PrivateField)
        );
        assert_eq!(m.display(false), "name : String");
        assert_eq!(m.display(true), "-name : String");
        let m = Member::method("{static} +foo()");
        assert!(m.is_static());
        assert_eq!(
            m.visibility_modifier(),
            Some(VisibilityModifier::PublicMethod)
        );
        assert_eq!(Member::field("  ").display(false), " ");
        assert!(Member::method("{abstract} run()").is_abstract());
    }
}
