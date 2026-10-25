//! Links in labels (`[[url{tooltip} label]]`), ported from the `url` package.

use crate::regex::{compile, IRegex};
use std::sync::LazyLock;

pub const URL_KEY: &str = "URL";

/// A hyperlink (`Url`).
#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    url: String,
    tooltip: String,
    label: String,
}

impl Url {
    pub fn new(url: &str, tooltip: Option<&str>, label: Option<&str>) -> Url {
        let url = url
            .strip_prefix('"')
            .and_then(|u| u.strip_suffix('"'))
            .unwrap_or(url)
            .to_string();
        Url {
            tooltip: tooltip.map_or_else(|| url.clone(), str::to_string),
            label: match label {
                Some(l) if !l.is_empty() => l.to_string(),
                _ => url.clone(),
            },
            url,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

const START_PART: &str = r"\[\[[%s]*";
const END_PART: &str = r"[%s]*\]\]";

fn quoted() -> String {
    format!(
        "{START_PART}[%g]([^%g]+)[%g](?:[%s]*\\{{([^{{}}]*)\\}})?(?:[%s]([^%s\\{{\\}}\\[\\]][^\\[\\]]*))?{END_PART}"
    )
}

fn only_tooltip() -> String {
    format!("{START_PART}\\{{(.*)\\}}{END_PART}")
}

fn only_tooltip_and_label() -> String {
    format!("{START_PART}\\{{([^{{}}]*)\\}}[%s]*([^\\[%s\\{{\\}}\\[\\]][^\\[\\]]*){END_PART}")
}

fn link_tooltip_nolabel() -> String {
    format!("{START_PART}([^\\s%g{{}}\\[\\]]+?)[%s]*\\{{(.+)\\}}{END_PART}")
}

fn link_with_optional() -> String {
    format!(
        "{START_PART}([^%s%g\\[\\]]+?)(?:[%s]*\\{{([^{{}}]*)\\}})?(?:[%s]([^%s\\{{\\}}\\[\\]][^\\[\\]]*))?{END_PART}"
    )
}

/// Source of the regexp matching any link syntax (`UrlBuilder.getRegexp`).
pub fn regexp() -> String {
    [
        quoted(),
        only_tooltip(),
        only_tooltip_and_label(),
        link_tooltip_nolabel(),
        link_with_optional(),
    ]
    .join("|")
}

/// The URL group of a command regex.
pub fn mandatory() -> IRegex {
    IRegex::named(URL_KEY, &format!("({})", regexp()))
}

pub fn optional() -> IRegex {
    IRegex::optional(mandatory())
}

/// How strictly a text must be a link (`UrlMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlMode {
    /// The whole text is the link.
    Strict,
    /// A link may appear anywhere.
    Anywhere,
}

static PATTERNS: LazyLock<[fancy_regex::Regex; 5]> = LazyLock::new(|| {
    [
        compile(&quoted()),
        compile(&only_tooltip()),
        compile(&only_tooltip_and_label()),
        compile(&link_tooltip_nolabel()),
        compile(&link_with_optional()),
    ]
});

/// Parses link text (`UrlBuilder`).
pub struct UrlBuilder {
    top_url: Option<String>,
    mode: UrlMode,
}

impl UrlBuilder {
    pub fn new(top_url: Option<&str>, mode: UrlMode) -> UrlBuilder {
        UrlBuilder {
            top_url: top_url.map(str::to_string),
            mode,
        }
    }

    fn with_top_url(&self, url: &str) -> String {
        match &self.top_url {
            Some(top)
                if !url.starts_with("http:")
                    && !url.starts_with("https:")
                    && !url.starts_with("file:") =>
            {
                format!("{top}{url}")
            }
            _ => url.to_string(),
        }
    }

    fn matched<'a>(
        &self,
        re: &fancy_regex::Regex,
        s: &'a str,
    ) -> Option<fancy_regex::Captures<'a, str>> {
        let caps = re.captures(s).ok().flatten()?;
        if self.mode == UrlMode::Strict {
            let m = caps.get(0)?;
            if m.start() != 0 || m.end() != s.len() {
                return None;
            }
        }
        Some(caps)
    }

    pub fn url(&self, s: &str) -> Option<Url> {
        let g =
            |c: &fancy_regex::Captures<'_, str>, i: usize| c.get(i).map(|m| m.as_str().to_string());
        let p = &*PATTERNS;
        if let Some(c) = self.matched(&p[0], s) {
            return Some(Url::new(
                &self.with_top_url(&g(&c, 1)?),
                g(&c, 2).as_deref(),
                g(&c, 3).as_deref(),
            ));
        }
        if let Some(c) = self.matched(&p[1], s) {
            return Some(Url::new("", g(&c, 1).as_deref(), None));
        }
        if let Some(c) = self.matched(&p[2], s) {
            return Some(Url::new("", g(&c, 1).as_deref(), g(&c, 2).as_deref()));
        }
        if let Some(c) = self.matched(&p[3], s) {
            return Some(Url::new(
                &self.with_top_url(&g(&c, 1)?),
                g(&c, 2).as_deref(),
                None,
            ));
        }
        if let Some(c) = self.matched(&p[4], s) {
            return Some(Url::new(
                &self.with_top_url(&g(&c, 1)?),
                g(&c, 2).as_deref(),
                g(&c, 3).as_deref(),
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_forms() {
        let b = UrlBuilder::new(None, UrlMode::Strict);
        let u = b.url("[[http://a.b]]").unwrap();
        assert_eq!(
            (u.url(), u.label(), u.tooltip()),
            ("http://a.b", "http://a.b", "http://a.b")
        );
        let u = b.url("[[http://a.b{tip} label]]").unwrap();
        assert_eq!(
            (u.url(), u.tooltip(), u.label()),
            ("http://a.b", "tip", "label")
        );
        let u = b.url("[[\"my file\" label]]").unwrap();
        assert_eq!((u.url(), u.label()), ("my file", "label"));
        let u = b.url("[[{only tip}]]").unwrap();
        assert_eq!((u.url(), u.tooltip()), ("", "only tip"));
        let u = b.url("[[{tip} lbl]]").unwrap();
        assert_eq!((u.tooltip(), u.label()), ("tip", "lbl"));
        assert!(b.url("no link").is_none());
        assert!(b.url("x [[a]] y").is_none());
    }

    #[test]
    fn modes_and_top_url() {
        let b = UrlBuilder::new(Some("http://top/"), UrlMode::Anywhere);
        assert_eq!(
            b.url("see [[page lbl]] now").unwrap().url(),
            "http://top/page"
        );
        assert_eq!(b.url("[[https://x.y]]").unwrap().url(), "https://x.y");
        assert_eq!(b.url("[[page{t}]]").unwrap().tooltip(), "t");
    }

    #[test]
    fn regex_group() {
        let r = IRegex::concat(vec![IRegex::start(), optional(), IRegex::end()]);
        let m = r.matcher("[[http://a.b lbl]]").unwrap();
        assert_eq!(m.get(URL_KEY, 0), Some("[[http://a.b lbl]]"));
        assert!(r.matcher("").unwrap().get(URL_KEY, 0).is_none());
        assert_eq!(Url::new("\"q\"", None, Some("")).url(), "q");
    }
}
