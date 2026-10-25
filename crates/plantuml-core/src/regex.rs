//! Composable regular expressions for command grammars, ported from PlantUML's `regex` package.
//!
//! Patterns use the `%s`, `%q`, `%g` and `%pLN` shorthands and are case-insensitive.

use fancy_regex::Regex as Engine;
use std::cell::OnceCell;

/// Java treats `\<` and `\>` as literal characters; the regex engine uses them as
/// word-boundary assertions, so they are unescaped first.
fn unescape_angle_brackets(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut it = input.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.peek() {
                Some('<') | Some('>') => out.push(it.next().expect("peeked")),
                Some(_) => {
                    out.push(c);
                    out.push(it.next().expect("peeked"));
                }
                None => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Expands the PlantUML shorthands (`Pattern2.transform`).
pub fn transform(input: &str) -> String {
    let input = unescape_angle_brackets(input);
    let input = input.as_str();
    let mut out = String::with_capacity(input.len() + 16);
    let mut rest = input;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let (rep, len) = if after.starts_with("pLN") {
            (r"\p{L}\p{N}", 3)
        } else if after.starts_with('s') {
            ("\\s\u{00A0}", 1)
        } else if after.starts_with('q') {
            ("'\u{2018}\u{2019}", 1)
        } else if after.starts_with('g') {
            ("\"\u{201c}\u{201d}\u{E121}", 1)
        } else {
            ("%", 0)
        };
        out.push_str(rep);
        rest = &after[len..];
    }
    out.push_str(rest);
    out
}

/// Compiles with PlantUML semantics (case-insensitive, shorthands expanded).
pub fn compile(pattern: &str) -> Engine {
    let full = format!("(?i){}", transform(pattern));
    Engine::new(&full).unwrap_or_else(|e| panic!("invalid regex {full}: {e}"))
}

/// Values of the named groups of one match (`RegexResult`).
#[derive(Debug, Clone, Default)]
pub struct RegexResult {
    entries: Vec<(String, Vec<Option<String>>)>,
}

impl RegexResult {
    fn add(&mut self, name: &str, values: Vec<Option<String>>) {
        self.entries.push((name.to_string(), values));
    }

    /// `num`-th group of the leaf called `key`, `None` when it did not participate.
    pub fn get(&self, key: &str, num: usize) -> Option<&str> {
        self.entries
            .iter()
            .find(|(n, _)| n == key)
            .and_then(|(_, v)| v.get(num))
            .and_then(|v| v.as_deref())
    }

    /// First leaf whose name starts with `key` that has a value at `num` (`getLazzy`).
    pub fn get_lazzy(&self, key: &str, num: usize) -> Option<&str> {
        self.entries
            .iter()
            .filter(|(n, _)| n.starts_with(key))
            .find_map(|(_, v)| v.get(num).and_then(|v| v.as_deref()))
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

/// A node of the grammar tree (`IRegex`).
#[derive(Debug)]
pub enum IRegex {
    Leaf(Leaf),
    Concat(Concat),
    Or {
        name: Option<String>,
        parts: Vec<IRegex>,
    },
    Optional(Box<IRegex>),
    OneOrMore {
        name: Option<String>,
        part: Box<IRegex>,
    },
    ZeroOrMore(Box<IRegex>),
}

/// A plain pattern fragment with an optional name.
#[derive(Debug)]
pub struct Leaf {
    pattern: String,
    name: Option<String>,
    count: usize,
}

/// A full command pattern: starts with `^` and ends with `$`.
#[derive(Debug)]
pub struct Concat {
    parts: Vec<IRegex>,
    compiled: OnceCellEngine,
    limit_size: usize,
}

struct OnceCellEngine(OnceCell<Engine>);

impl std::fmt::Debug for OnceCellEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Engine")
    }
}

fn count_groups(pattern: &str) -> usize {
    compile(pattern).captures_len() - 1
}

impl IRegex {
    /// Unnamed fragment; its group count is computed.
    pub fn leaf(pattern: &str) -> IRegex {
        IRegex::Leaf(Leaf {
            pattern: pattern.to_string(),
            name: None,
            count: count_groups(pattern),
        })
    }

    /// Fragment whose groups are exposed under `name`.
    pub fn named(name: &str, pattern: &str) -> IRegex {
        IRegex::Leaf(Leaf {
            pattern: pattern.to_string(),
            name: Some(name.to_string()),
            count: count_groups(pattern),
        })
    }

    pub fn start() -> IRegex {
        IRegex::leaf("^")
    }

    pub fn end() -> IRegex {
        IRegex::leaf("$")
    }

    pub fn space_zero_or_more() -> IRegex {
        IRegex::leaf("[%s]*")
    }

    pub fn space_one() -> IRegex {
        IRegex::leaf("[%s]")
    }

    pub fn space_zero_or_one() -> IRegex {
        IRegex::optional(IRegex::space_one())
    }

    pub fn space_one_or_more() -> IRegex {
        IRegex::leaf("[%s]+")
    }

    pub fn optional(r: IRegex) -> IRegex {
        IRegex::Optional(Box::new(r))
    }

    pub fn zero_or_more(r: IRegex) -> IRegex {
        IRegex::ZeroOrMore(Box::new(r))
    }

    pub fn one_or_more(name: Option<&str>, r: IRegex) -> IRegex {
        IRegex::OneOrMore {
            name: name.map(str::to_string),
            part: Box::new(r),
        }
    }

    /// Non-capturing alternative.
    pub fn or(parts: Vec<IRegex>) -> IRegex {
        IRegex::Or { name: None, parts }
    }

    /// Alternative whose whole match is exposed under `name`.
    pub fn or_named(name: &str, parts: Vec<IRegex>) -> IRegex {
        IRegex::Or {
            name: Some(name.to_string()),
            parts,
        }
    }

    /// Complete command pattern; must start with [`start`](Self::start) and end with
    /// [`end`](Self::end).
    pub fn concat(parts: Vec<IRegex>) -> IRegex {
        assert!(
            matches!(parts.first(), Some(IRegex::Leaf(l)) if l.pattern == "^"),
            "pattern must start with ^"
        );
        assert!(
            matches!(parts.last(), Some(IRegex::Leaf(l)) if l.pattern == "$"),
            "pattern must end with $"
        );
        IRegex::Concat(Concat {
            parts,
            compiled: OnceCellEngine(OnceCell::new()),
            limit_size: 0,
        })
    }

    /// Unanchored sequence used as a part of a larger pattern (`new RegexConcat(...)`).
    pub fn group(parts: Vec<IRegex>) -> IRegex {
        IRegex::Concat(Concat {
            parts,
            compiled: OnceCellEngine(OnceCell::new()),
            limit_size: 0,
        })
    }

    /// Rejects lines longer than `size` before matching (`protectSize`).
    pub fn protect_size(mut self, size: usize) -> IRegex {
        if let IRegex::Concat(c) = &mut self {
            c.limit_size = size;
        }
        self
    }

    /// The regex source, shorthands not yet expanded.
    pub fn pattern(&self) -> String {
        match self {
            IRegex::Leaf(l) => l.pattern.clone(),
            IRegex::Concat(c) => c.parts.iter().map(IRegex::pattern).collect(),
            IRegex::Or { name, parts } => {
                let body: Vec<String> = parts.iter().map(IRegex::pattern).collect();
                format!(
                    "({}{})",
                    if name.is_none() { "?:" } else { "" },
                    body.join("|")
                )
            }
            IRegex::Optional(r) => format!("(?:{})?", r.pattern()),
            IRegex::OneOrMore { part, .. } => format!("({})+", part.pattern()),
            IRegex::ZeroOrMore(r) => format!("(?:{})*", r.pattern()),
        }
    }

    fn engine(&self) -> &Engine {
        match self {
            IRegex::Concat(c) => c.compiled.0.get_or_init(|| compile(&self.pattern())),
            _ => panic!("only a concatenation can be matched"),
        }
    }

    /// Whether the (trimmed) line matches (`match(StringLocated)`).
    pub fn is_match(&self, line: &str) -> bool {
        if let IRegex::Concat(c) = self {
            if c.limit_size != 0 && line.chars().count() > c.limit_size {
                return false;
            }
        }
        self.engine().is_match(line).unwrap_or(false)
    }

    /// Matches and distributes the groups (`matcher(String)`).
    pub fn matcher(&self, line: &str) -> Option<RegexResult> {
        let caps = self.engine().captures(line).ok().flatten()?;
        let mut idx = 1;
        let mut result = RegexResult::default();
        self.fill(&caps, &mut idx, &mut result);
        Some(result)
    }

    fn fill(&self, caps: &fancy_regex::Captures<'_, str>, idx: &mut usize, out: &mut RegexResult) {
        let take = |idx: &mut usize| {
            let v = caps.get(*idx).map(|m| m.as_str().to_string());
            *idx += 1;
            v
        };
        match self {
            IRegex::Leaf(l) => {
                let values: Vec<Option<String>> = (0..l.count).map(|_| take(idx)).collect();
                if let Some(name) = &l.name {
                    out.add(name, values);
                }
            }
            IRegex::Concat(c) => {
                for p in &c.parts {
                    p.fill(caps, idx, out);
                }
            }
            IRegex::Or { name, parts } => {
                let full = name.as_ref().map(|_| take(idx));
                for p in parts {
                    p.fill(caps, idx, out);
                }
                if let (Some(name), Some(full)) = (name, full) {
                    out.add(name, vec![full]);
                }
            }
            IRegex::Optional(r) | IRegex::ZeroOrMore(r) => r.fill(caps, idx, out),
            IRegex::OneOrMore { name, part } => {
                let full = take(idx);
                part.fill(caps, idx, out);
                if let Some(name) = name {
                    out.add(name, vec![full]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd() -> IRegex {
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("KEY", "(participant|actor)"),
            IRegex::space_one_or_more(),
            IRegex::named("NAME", "([\\p{L}0-9_.]+)"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("ALIAS", "(\\w+)"),
            ])),
            IRegex::end(),
        ])
    }

    #[test]
    fn angle_brackets_are_literal() {
        assert_eq!(transform(r"\<a\>"), "<a>");
        assert_eq!(transform(r"\\<"), r"\\<");
        assert_eq!(transform(r"x\"), r"x\");
        let r = IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf(r"\<\<x\>\>"),
            IRegex::end(),
        ]);
        assert!(r.is_match("<<x>>"));
    }

    #[test]
    fn transform_shorthands() {
        assert_eq!(transform("[%s]+"), "[\\s\u{a0}]+");
        assert_eq!(transform("%pLN"), "\\p{L}\\p{N}");
        assert!(transform("[%g]").contains('\u{201c}'));
        assert_eq!(transform("100%"), "100%");
        assert_eq!(transform("%q"), "'\u{2018}\u{2019}");
    }

    #[test]
    fn match_and_groups() {
        let r = cmd();
        assert!(r.is_match("participant Bob"));
        assert!(r.is_match("ACTOR Bob as B"));
        assert!(!r.is_match("class Bob"));
        let m = r.matcher("actor Alice as A").unwrap();
        assert_eq!(m.get("KEY", 0), Some("actor"));
        assert_eq!(m.get("NAME", 0), Some("Alice"));
        assert_eq!(m.get("ALIAS", 0), Some("A"));
        assert_eq!(m.get_lazzy("AL", 0), Some("A"));
        let m = r.matcher("actor Alice").unwrap();
        assert_eq!(m.get("ALIAS", 0), None);
        assert!(r.matcher("nothing").is_none());
        assert_eq!(m.size(), 3);
        assert!(r.pattern().starts_with('^'));
    }

    #[test]
    fn combinators() {
        let r = IRegex::concat(vec![
            IRegex::start(),
            IRegex::or_named("T", vec![IRegex::leaf("a"), IRegex::leaf("b")]),
            IRegex::one_or_more(Some("DIGITS"), IRegex::leaf("[0-9]")),
            IRegex::zero_or_more(IRegex::leaf("x")),
            IRegex::or(vec![IRegex::leaf("p"), IRegex::leaf("q")]),
            IRegex::space_zero_or_more(),
            IRegex::space_zero_or_one(),
            IRegex::end(),
        ])
        .protect_size(10);
        let m = r.matcher("b123xxp").unwrap();
        assert_eq!(m.get("T", 0), Some("b"));
        assert_eq!(m.get("DIGITS", 0), Some("3"));
        assert!(!r.is_match("b1234567890xxxp"));
    }

    #[test]
    #[should_panic(expected = "start with")]
    fn concat_requires_anchors() {
        IRegex::concat(vec![IRegex::leaf("a"), IRegex::end()]);
    }
}
