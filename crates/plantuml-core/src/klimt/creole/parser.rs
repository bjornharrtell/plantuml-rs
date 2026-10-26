//! Line parsing: creole commands to atoms (`creole/legacy`, `creole/command`).

use super::atoms::{list_number, Atom, AtomText};
use super::{hide, unhide, CreoleMode, SkinSimple};
use crate::klimt::color::HColor;
use crate::klimt::font::config::{FontConfiguration, FontPosition, FontStyle};
use crate::klimt::geom::HorizontalAlignment;
use regex::Regex;
use std::rc::Rc;
use std::sync::LazyLock;

const NEWLINE: char = '\u{E100}';

/// One display line: its atoms plus alignment (`StripeSimple`).
#[derive(Debug, Clone)]
pub struct Stripe {
    atoms: Vec<Atom>,
    has_header: bool,
    pub align: HorizontalAlignment,
    fc: FontConfiguration,
}

impl Stripe {
    /// Atoms of the stripe, header first; an empty stripe holds one space.
    pub fn atoms(&self) -> Vec<Atom> {
        if self.atoms.is_empty() {
            vec![Atom::Text(AtomText::legacy(" ", self.fc.clone()))]
        } else {
            self.atoms.clone()
        }
    }

    pub fn has_header(&self) -> bool {
        self.has_header
    }
}

/// All stripes of a text (`Sheet`).
#[derive(Debug, Clone)]
pub struct Sheet {
    pub stripes: Vec<Stripe>,
    pub alignment: HorizontalAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StripeType {
    Normal,
    Heading,
    HorizontalLine,
    ListWithoutNumber,
    ListWithNumber,
}

#[derive(Debug, Clone, Copy)]
struct StripeStyle {
    kind: StripeType,
    order: i32,
    style: char,
}

#[derive(Default)]
struct CreoleContext {
    stack: Vec<i32>,
}

impl CreoleContext {
    fn local_number(&mut self, order: i32) -> i32 {
        let order = order as usize;
        while self.stack.len() <= order {
            self.stack.push(0);
        }
        self.stack.truncate(order + 1);
        let n = self.stack[order];
        self.stack[order] += 1;
        n
    }
}

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap_or_else(|e| panic!("bad regex {p}: {e}"))
}

static SECTION_HEADER: LazyLock<Regex> = LazyLock::new(|| re(r"^--([^-]*)--$"));
static SECTION_TITLE: LazyLock<Regex> = LazyLock::new(|| re(r"^==([^=]*)==$"));
static SECTION_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| re(r"^====+$"));
static DOUBLE_DOT: LazyLock<Regex> = LazyLock::new(|| re(r"^\.\.([^.]*)\.\.$"));
static ASTERISK_PREFIXED: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(\*+)([^*]+(?:[^*]|\*\*[^*]+\*\*)*)$"));
static ASTERISK_HEADER: LazyLock<Regex> = LazyLock::new(|| re(r"^(\*+)(\s.+)$"));
static HASH_HEADING: LazyLock<Regex> = LazyLock::new(|| re(r"^(#+)(.+)$"));
static EQUALS_HEADING: LazyLock<Regex> = LazyLock::new(|| re(r"^(=+)(.+)$"));

/// Splits one line into `(line, style)` (`CreoleStripeSimpleParser` constructor).
fn classify(line: &str, mode: CreoleMode) -> (String, StripeStyle) {
    let normal = |l: &str| {
        (
            l.to_string(),
            StripeStyle {
                kind: StripeType::Normal,
                order: 0,
                style: '\0',
            },
        )
    };
    if mode == CreoleMode::NoCreole {
        return normal(line);
    }
    let hline = |t: &str, c: char| {
        (
            t.to_string(),
            StripeStyle {
                kind: StripeType::HorizontalLine,
                order: 0,
                style: c,
            },
        )
    };
    if let Some(m) = SECTION_HEADER.captures(line) {
        return hline(&m[1], '-');
    }
    if let Some(m) = SECTION_TITLE.captures(line) {
        return hline(&m[1], '=');
    }
    if SECTION_SEPARATOR.is_match(line) {
        return hline("", '=');
    }
    if let Some(m) = DOUBLE_DOT.captures(line) {
        return hline(&m[1], '.');
    }
    let list = |m: &regex::Captures<'_>, kind: StripeType, text: String| {
        (
            text,
            StripeStyle {
                kind,
                order: m[1].chars().count() as i32 - 1,
                style: '\0',
            },
        )
    };
    if mode == CreoleMode::Full {
        if let Some(m) = ASTERISK_PREFIXED.captures(line) {
            return list(&m, StripeType::ListWithoutNumber, m[2].trim().to_string());
        }
        if let Some(m) = ASTERISK_HEADER.captures(line) {
            return list(&m, StripeType::ListWithoutNumber, m[2].trim().to_string());
        }
        let hidden = hide(line);
        if let Some(m) = HASH_HEADING.captures(&hidden) {
            let order = unhide(&m[1]).chars().count() as i32 - 1;
            return (
                unhide(&m[2]).trim().to_string(),
                StripeStyle {
                    kind: StripeType::ListWithNumber,
                    order,
                    style: '\0',
                },
            );
        }
    }
    if let Some(m) = EQUALS_HEADING.captures(line) {
        return list(&m, StripeType::Heading, m[2].trim().to_string());
    }
    normal(line)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Syntax {
    Creole,
    Legacy,
    LegacyEol,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Cmd {
    Style(FontStyle, Syntax),
    Size { eol: bool },
    Color { eol: bool },
    ColorAndSize { eol: bool },
    Exposant(FontPosition),
    Space,
    FontFamily { eol: bool },
    Monospaced,
    Url,
    SvgAttribute { eol: bool },
}

fn commands(mode: CreoleMode) -> Vec<Cmd> {
    use FontStyle::*;
    use Syntax::*;
    let mut v = Vec::new();
    for s in [Bold, Italic] {
        v.extend([
            Cmd::Style(s, Creole),
            Cmd::Style(s, Legacy),
            Cmd::Style(s, LegacyEol),
        ]);
    }
    v.extend([Cmd::Style(Plain, Legacy), Cmd::Style(Plain, LegacyEol)]);
    if mode == CreoleMode::Full {
        v.push(Cmd::Style(Underline, Creole));
    }
    v.extend([
        Cmd::Style(Underline, Legacy),
        Cmd::Style(Underline, LegacyEol),
    ]);
    for s in [Strike, Wave] {
        v.extend([
            Cmd::Style(s, Creole),
            Cmd::Style(s, Legacy),
            Cmd::Style(s, LegacyEol),
        ]);
    }
    v.extend([
        Cmd::Style(BackColor, Legacy),
        Cmd::Style(BackColor, LegacyEol),
    ]);
    v.extend([
        Cmd::Size { eol: false },
        Cmd::Size { eol: true },
        Cmd::Color { eol: false },
        Cmd::Color { eol: true },
        Cmd::ColorAndSize { eol: false },
        Cmd::ColorAndSize { eol: true },
        Cmd::Exposant(FontPosition::Exposant),
        Cmd::Exposant(FontPosition::Indice),
        Cmd::Space,
        Cmd::FontFamily { eol: false },
        Cmd::FontFamily { eol: true },
        Cmd::Monospaced,
        Cmd::Url,
        Cmd::SvgAttribute { eol: false },
        Cmd::SvgAttribute { eol: true },
    ]);
    v
}

fn creole_marker(s: FontStyle) -> &'static str {
    match s {
        FontStyle::Italic => "//",
        FontStyle::Bold => "**",
        FontStyle::Underline => "__",
        FontStyle::Wave => "~~",
        FontStyle::Strike => "--",
        _ => unreachable!("no creole syntax"),
    }
}

fn starters(cmd: Cmd) -> &'static [&'static str] {
    match cmd {
        Cmd::Style(s, Syntax::Creole) => match s {
            FontStyle::Italic => &["//"],
            FontStyle::Bold => &["**"],
            FontStyle::Underline => &["__"],
            FontStyle::Wave => &["~~"],
            _ => &["--"],
        },
        Cmd::Style(s, _) => match s {
            FontStyle::Plain => &["<p", "<P"],
            FontStyle::Italic => &["<i", "<I"],
            FontStyle::Bold | FontStyle::BackColor => &["<b", "<B"],
            FontStyle::Underline => &["<u", "<U"],
            FontStyle::Strike => &["<s", "<S", "<d", "<D"],
            FontStyle::Wave => &["<w"],
        },
        Cmd::Size { .. } | Cmd::Exposant(_) | Cmd::Space => &["<s"],
        Cmd::Color { .. } => &["<c"],
        Cmd::ColorAndSize { .. } | Cmd::FontFamily { .. } => &["<f"],
        Cmd::Monospaced => &["\"\""],
        Cmd::Url => &["[["],
        Cmd::SvgAttribute { .. } => &["<t"],
    }
}

const XC: &str = r"(#[0-9a-fA-F]{6}|\w+)";

fn activation(s: FontStyle) -> String {
    match s {
        FontStyle::Plain => "(?i:<plain>)".into(),
        FontStyle::Italic => "<[iI]>".into(),
        FontStyle::Bold => "<[bB]>".into(),
        FontStyle::Underline => format!("<[uU](?::{XC})?>"),
        FontStyle::Wave => format!("<[wW](?::{XC})?>"),
        FontStyle::BackColor => r"(?i:<back)(?::(#?\w+(?:[-\\|/]#?\w+)?))?>".into(),
        FontStyle::Strike => format!("<(?:strike|STRIKE|s|S|del|DEL)(?::{XC})?>"),
    }
}

fn deactivation(s: FontStyle) -> &'static str {
    match s {
        FontStyle::Plain => "(?i:</plain>)",
        FontStyle::Italic => "</[iI]>",
        FontStyle::Bold => "</[bB]>",
        FontStyle::Underline => "</[uU]>",
        FontStyle::Wave => "</[wW]>",
        FontStyle::BackColor => "(?i:</back>)",
        FontStyle::Strike => "</(?:s|S|strike|STRIKE|del|DEL)>",
    }
}

static STYLE_REGEX: LazyLock<Vec<(FontStyle, Syntax, Regex)>> = LazyLock::new(|| {
    let mut v = Vec::new();
    for s in [
        FontStyle::Plain,
        FontStyle::Italic,
        FontStyle::Bold,
        FontStyle::Underline,
        FontStyle::Strike,
        FontStyle::Wave,
        FontStyle::BackColor,
    ] {
        v.push((
            s,
            Syntax::Legacy,
            re(&format!("^{}(.*?){}", activation(s), deactivation(s))),
        ));
        v.push((
            s,
            Syntax::LegacyEol,
            re(&format!("^{}(.+)$", activation(s))),
        ));
    }
    v
});

const Q: &str = "[\"\u{201c}\u{201d}]";

static SIZE: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    let p = r"<size[\s:]+(\d+)\s*>";
    (
        re(&format!("^({p}(.*?)</size>)")),
        re(&format!("^({p}(.*)$)")),
    )
});
static COLOR: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    let p = r"<color[\s:]+(#[0-9a-fA-F]{1,6}|#?\w+)\s*>";
    (
        re(&format!("^({p}(.*?)</color>)")),
        re(&format!("^({p}(.*)$)")),
    )
});
static COLOR_SIZE: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    let p = format!(
        r"<font(?:\s+size\s*=\s*{Q}?(\d+){Q}?|\s+color\s*=\s*{Q}?(#[0-9a-fA-F]{{6}}|\w+){Q}?)+\s*>"
    );
    (
        re(&format!("^({p}(.*?)</font>)")),
        re(&format!("^({p}(.*))$")),
    )
});
static FAMILY: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    let p = r"<font[\s:]+([^>]+)/?>";
    (
        re(&format!("^({p}(.*?)</font>)")),
        re(&format!("^({p}(.*)$)")),
    )
});
static SVG_ATTR: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    let p = r"<text[\s:]+([^>]+)/?>";
    (
        re(&format!("^({p}(.*?)</text>)")),
        re(&format!("^({p}(.*)$)")),
    )
});
static ATTR_PAIR: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"(\w+)\s*=\s*({Q}[^\u{{201c}}\u{{201d}}\x22]*{Q}|\w+)"
    ))
});
static SPACE: LazyLock<Regex> = LazyLock::new(|| re(r"^(<space:(\d+)/?>)"));
static MONO: LazyLock<Regex> = LazyLock::new(|| re(r#"^(""(.*?)"")"#));
static LINK: LazyLock<Regex> = LazyLock::new(|| re(r"^(\[\[([^\[\]]+)\]\])"));

fn unquote(s: &str) -> String {
    let t = s.trim_matches(|c| c == '"' || c == '\u{201c}' || c == '\u{201d}');
    t.to_string()
}

struct StripeBuilder<'a> {
    atoms: Vec<Atom>,
    has_header: bool,
    align: HorizontalAlignment,
    fc: FontConfiguration,
    style: StripeStyle,
    skin: &'a SkinSimple,
    cmds: Vec<Cmd>,
}

impl<'a> StripeBuilder<'a> {
    fn new(
        fc: FontConfiguration,
        style: StripeStyle,
        ctx: &mut CreoleContext,
        skin: &'a SkinSimple,
        mode: CreoleMode,
    ) -> Self {
        let header = match style.kind {
            StripeType::ListWithoutNumber => Some(Atom::Bullet(fc.clone(), style.order)),
            StripeType::ListWithNumber => {
                Some(list_number(&fc, style.order, ctx.local_number(style.order)))
            }
            _ => None,
        };
        Self {
            has_header: header.is_some(),
            atoms: header.into_iter().collect(),
            align: HorizontalAlignment::Left,
            fc,
            style,
            skin,
            cmds: commands(if mode == CreoleMode::Full {
                CreoleMode::Full
            } else {
                CreoleMode::SimpleLine
            }),
        }
    }

    fn manage_cell_alignment<'l>(&mut self, line: &'l str) -> &'l str {
        for (tag, a) in [
            ("<l>", HorizontalAlignment::Left),
            ("<left>", HorizontalAlignment::Left),
            ("<center>", HorizontalAlignment::Center),
            ("<c>", HorizontalAlignment::Center),
            ("<right>", HorizontalAlignment::Right),
            ("<r>", HorizontalAlignment::Right),
        ] {
            if let Some(rest) = line.strip_prefix(tag) {
                self.align = a;
                return rest;
            }
        }
        line
    }

    fn analyze_and_add(&mut self, line: &str) {
        let line = self.manage_cell_alignment(line);
        let line = hide(line);
        match self.style.kind {
            StripeType::Heading => {
                self.fc = match self.style.order {
                    0 => self.fc.bigger(4.0).bold(),
                    1 => self.fc.bigger(2.0).bold(),
                    2 => self.fc.bigger(1.0).bold(),
                    _ => self.fc.italic(),
                };
                self.modify_stripe(&line);
            }
            StripeType::HorizontalLine => {
                let title = if line.is_empty() {
                    None
                } else {
                    let sheet = create_sheet(
                        &[line],
                        &self.fc,
                        HorizontalAlignment::Left,
                        self.skin,
                        CreoleMode::Full,
                    );
                    Some(Rc::new(super::layout::SheetBlock::new(
                        sheet,
                        super::LineBreakStrategy::NONE,
                        self.skin.padding,
                        0.0,
                        0.0,
                    )))
                };
                self.atoms.push(Atom::HLine {
                    title,
                    style: self.style.style,
                });
            }
            _ => self.modify_stripe(&line),
        }
    }

    fn add_pending(&mut self, pending: &mut String) {
        if !pending.is_empty() {
            self.atoms
                .push(Atom::Text(AtomText::legacy(pending, self.fc.clone())));
            pending.clear();
        }
    }

    fn modify_stripe(&mut self, line: &str) {
        let mut pending = String::new();
        let mut pos = 0;
        while pos < line.len() {
            let consumed = self.search_command(line, pos, &mut pending);
            match consumed {
                Some(n) => pos += n,
                None => {
                    let ch = line[pos..].chars().next().expect("in range");
                    pending.push(ch);
                    pos += ch.len_utf8();
                }
            }
        }
        self.add_pending(&mut pending);
    }

    /// Tries every command whose starter matches; returns bytes consumed.
    fn search_command(&mut self, line: &str, pos: usize, pending: &mut String) -> Option<usize> {
        let rest = &line[pos..];
        if rest.chars().count() <= 2 {
            return None;
        }
        let key: String = rest.chars().take(2).collect();
        let cmds = self.cmds.clone();
        for cmd in cmds {
            if !starters(cmd).contains(&key.as_str()) {
                continue;
            }
            if let Some(m) = self.match_command(cmd, rest) {
                self.add_pending(pending);
                return Some(self.execute(m));
            }
        }
        None
    }

    fn match_command(&self, cmd: Cmd, rest: &str) -> Option<Matched> {
        match cmd {
            Cmd::Style(style, Syntax::Creole) => {
                let m = creole_marker(style);
                let body = rest.strip_prefix(m)?;
                let end = body.find(m).filter(|e| *e > 0)?;
                Some(Matched {
                    consumed: m.len() * 2 + end,
                    inner: body[..end].to_string(),
                    change: Change::Style(style, None),
                })
            }
            Cmd::Style(style, syn) => {
                let (_, _, rx) = STYLE_REGEX
                    .iter()
                    .find(|(s, y, _)| *s == style && *y == syn)?;
                let c = rx.captures(rest)?;
                let n = c.len();
                // The last group is the body; an optional earlier group is the color.
                let body = c.get(n - 1)?;
                if body.as_str().is_empty() {
                    return None;
                }
                let color = if n > 2 && style.can_have_extended_color() {
                    c.get(1).map(|m| HColor::parse_or_white(m.as_str()))
                } else {
                    None
                };
                Some(Matched {
                    consumed: c.get(0)?.end(),
                    inner: body.as_str().to_string(),
                    change: Change::Style(style, color),
                })
            }
            Cmd::Size { eol } => {
                let c = if eol { &SIZE.1 } else { &SIZE.0 }.captures(rest)?;
                let size: f32 = c[2].parse().ok()?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[3].to_string(),
                    change: Change::Size(size),
                })
            }
            Cmd::Color { eol } => {
                let c = if eol { &COLOR.1 } else { &COLOR.0 }.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[3].to_string(),
                    change: Change::Color(HColor::parse(&c[2])),
                })
            }
            Cmd::ColorAndSize { eol } => {
                let c = if eol { &COLOR_SIZE.1 } else { &COLOR_SIZE.0 }.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[4].to_string(),
                    change: Change::ColorSize(
                        c.get(2).and_then(|m| m.as_str().parse().ok()),
                        c.get(3).map(|m| HColor::parse_or_white(m.as_str())),
                    ),
                })
            }
            Cmd::Exposant(pos) => {
                let tag = if pos == FontPosition::Exposant {
                    "sup"
                } else {
                    "sub"
                };
                let body = rest.strip_prefix(&format!("<{tag}>"))?;
                let close = format!("</{tag}>");
                let end = body.find(&close).filter(|e| *e > 0)?;
                Some(Matched {
                    consumed: tag.len() + 2 + end + close.len(),
                    inner: body[..end].to_string(),
                    change: Change::Position(pos),
                })
            }
            Cmd::Space => {
                let c = SPACE.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: String::new(),
                    change: Change::Space(c[2].parse().ok()?),
                })
            }
            Cmd::FontFamily { eol } => {
                let c = if eol { &FAMILY.1 } else { &FAMILY.0 }.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[3].to_string(),
                    change: Change::Family(c[2].to_string()),
                })
            }
            Cmd::Monospaced => {
                let c = MONO.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[2].to_string(),
                    change: Change::Family(self.skin.monospaced_family.clone()),
                })
            }
            Cmd::Url => {
                let c = LINK.captures(rest)?;
                Some(Matched {
                    consumed: c[1].len(),
                    inner: String::new(),
                    change: Change::Url(link_label(&c[2])),
                })
            }
            Cmd::SvgAttribute { eol } => {
                let c = if eol { &SVG_ATTR.1 } else { &SVG_ATTR.0 }.captures(rest)?;
                let attrs = ATTR_PAIR
                    .captures_iter(&c[2])
                    .map(|a| (a[1].to_string(), unquote(&a[2])))
                    .collect();
                Some(Matched {
                    consumed: c[1].len(),
                    inner: c[3].to_string(),
                    change: Change::Attributes(attrs),
                })
            }
        }
    }

    fn execute(&mut self, m: Matched) -> usize {
        let saved = self.fc.clone();
        match m.change {
            Change::Style(style, color) => {
                self.fc = self.fc.add(style);
                if color.is_some() {
                    self.fc = self.fc.change_extended_color(color);
                }
            }
            Change::Size(s) => self.fc = self.fc.change_size(s),
            Change::Color(c) => {
                if let Some(c) = c {
                    self.fc = self.fc.change_color(c);
                }
            }
            Change::ColorSize(size, color) => {
                if let Some(s) = size {
                    self.fc = self.fc.change_size(s);
                }
                if let Some(c) = color {
                    self.fc = self.fc.change_color(c);
                }
            }
            Change::Position(p) => self.fc = self.fc.change_font_position(p),
            Change::Family(f) => self.fc = self.fc.change_family(&f),
            Change::Attributes(a) => self.fc = self.fc.change_attributes(&a),
            Change::Space(n) => {
                self.atoms.push(Atom::Space(n));
                return m.consumed;
            }
            Change::Url(label) => {
                self.atoms
                    .push(Atom::Text(AtomText::legacy(&label, self.fc.hyperlink())));
                return m.consumed;
            }
        }
        self.analyze_and_add(&m.inner);
        self.fc = saved;
        m.consumed
    }
}

/// Label of `[[url label]]`, `[[url{tooltip} label]]` or `[[url]]`.
fn link_label(inner: &str) -> String {
    let inner = inner.trim();
    // The URL runs up to the first whitespace or the optional tooltip block, so a tooltip that
    // contains spaces (`[[http://x.com{a b}]]`) does not truncate the URL.
    let cut = inner
        .find(|c: char| c.is_whitespace() || c == '{')
        .unwrap_or(inner.len());
    let url = &inner[..cut];
    let mut rest = inner[cut..].trim_start();
    if let Some(after_open) = rest.strip_prefix('{') {
        rest = match after_open.rfind('}') {
            Some(close) => &after_open[close + 1..],
            None => "",
        };
    }
    let label = rest.trim();
    if label.is_empty() {
        url.to_string()
    } else {
        label.to_string()
    }
}

enum Change {
    Style(FontStyle, Option<HColor>),
    Size(f32),
    Color(Option<HColor>),
    ColorSize(Option<f32>, Option<HColor>),
    Position(FontPosition),
    Family(String),
    Attributes(Vec<(String, String)>),
    Space(f64),
    Url(String),
}

struct Matched {
    consumed: usize,
    inner: String,
    change: Change,
}

/// Applies `<<x>>` to `«x»` (`Guillemet.manageGuillemet`).
fn manage_guillemet(s: &str) -> String {
    static G: LazyLock<Regex> = LazyLock::new(|| re(r"<<\s?((?:<&\w+>|[^<>])+?)\s?>>"));
    if !s.contains('<') {
        return s.to_string();
    }
    G.replace_all(s, "\u{00AB}$1\u{00BB}").into_owned()
}

/// One entry of a text to lay out.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetItem {
    Text(String),
    /// Labels of a stereotype, drawn with the stereotype font.
    Stereotype(Vec<String>),
}

/// Builds the sheet of a text (`CreoleParser.createSheet`).
pub fn create_sheet(
    lines: &[String],
    fc: &FontConfiguration,
    align: HorizontalAlignment,
    skin: &SkinSimple,
    mode: CreoleMode,
) -> Sheet {
    let items: Vec<SheetItem> = lines.iter().cloned().map(SheetItem::Text).collect();
    create_sheet_items(&items, fc, fc, align, skin, mode)
}

/// Like [`create_sheet`], with stereotype entries and their own font.
pub fn create_sheet_items(
    items: &[SheetItem],
    fc: &FontConfiguration,
    stereotype_fc: &FontConfiguration,
    align: HorizontalAlignment,
    skin: &SkinSimple,
    mode: CreoleMode,
) -> Sheet {
    let mut ctx = CreoleContext::default();
    let mut stripes: Vec<Stripe> = Vec::new();
    for item in items {
        let (raw, font, manage) = match item {
            SheetItem::Text(t) => (std::slice::from_ref(t), fc, true),
            SheetItem::Stereotype(labels) => (labels.as_slice(), stereotype_fc, false),
        };
        for raw in raw {
            let text = if manage && skin.guillemet {
                manage_guillemet(raw)
            } else {
                raw.clone()
            };
            let cell_align = stripes.last().map_or(align, |s| s.align);
            let (line, style) = classify(&text, mode);
            for single in line.split(NEWLINE) {
                let mut b = StripeBuilder::new(font.clone(), style, &mut ctx, skin, mode);
                b.align = cell_align;
                b.analyze_and_add(single);
                stripes.push(Stripe {
                    atoms: b.atoms,
                    has_header: b.has_header,
                    align: b.align,
                    fc: b.fc,
                });
            }
        }
    }
    Sheet {
        stripes,
        alignment: align,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::font::UFont;

    fn fc() -> FontConfiguration {
        FontConfiguration::black_blue_true(UFont::sans_serif(14))
    }

    fn sheet(lines: &[&str]) -> Sheet {
        let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        create_sheet(
            &lines,
            &fc(),
            HorizontalAlignment::Left,
            &SkinSimple::default(),
            CreoleMode::Full,
        )
    }

    fn texts(s: &Stripe) -> Vec<(String, bool, bool)> {
        s.atoms()
            .iter()
            .filter_map(|a| a.as_text())
            .map(|t| {
                (
                    t.text.clone(),
                    t.fc.contains_style(FontStyle::Bold),
                    t.fc.contains_style(FontStyle::Italic),
                )
            })
            .collect()
    }

    #[test]
    fn plain_and_creole() {
        let s = sheet(&["hello **bold** and //it//"]);
        assert_eq!(
            texts(&s.stripes[0]),
            [
                ("hello ".into(), false, false),
                ("bold".into(), true, false),
                (" and ".into(), false, false),
                ("it".into(), false, true),
            ]
        );
    }

    #[test]
    fn legacy_tags() {
        let s = sheet(&["<b>x</b><i>y</i><b>eol <i>z"]);
        let t = texts(&s.stripes[0]);
        assert_eq!(t[0], ("x".into(), true, false));
        assert_eq!(t[1], ("y".into(), false, true));
        assert_eq!(t[2], ("eol ".into(), true, false));
        assert_eq!(t[3], ("z".into(), true, true));
        let s = sheet(&["<b></b>"]);
        assert_eq!(texts(&s.stripes[0])[0], ("</b>".into(), true, false));
    }

    #[test]
    fn color_size_family() {
        let s = sheet(&[
            "<color:red>r</color><size:20>big</size><font color=blue size=9>f</font><font:Courier>c</font>\"\"m\"\"",
        ]);
        let a: Vec<_> = s.stripes[0]
            .atoms()
            .into_iter()
            .filter_map(|a| a.as_text().cloned())
            .collect();
        assert_eq!(a[0].fc.color().rgba().r, 255);
        assert_eq!(a[1].fc.size2d(), 20.0);
        assert_eq!(a[2].fc.size2d(), 9.0);
        assert_eq!(a[3].fc.font().definition(), "Courier");
        assert_eq!(a[4].fc.font().definition(), "monospaced");
    }

    #[test]
    fn styles_underline_wave_strike_back() {
        let s = sheet(&[
            "<u:red>u</u><w:#00FF00>w</w><s>s</s><back:yellow>b</back>__x__--y--~~z~~<plain>p",
        ]);
        let a: Vec<_> = s.stripes[0]
            .atoms()
            .into_iter()
            .filter_map(|a| a.as_text().cloned())
            .collect();
        assert!(a[0].fc.contains_style(FontStyle::Underline));
        assert!(a[0].fc.extended_color().is_some());
        assert!(a[1].fc.contains_style(FontStyle::Wave));
        assert!(a[2].fc.contains_style(FontStyle::Strike));
        assert!(a[3].fc.contains_style(FontStyle::BackColor));
        assert!(a[4].fc.contains_style(FontStyle::Underline));
        assert!(a[5].fc.contains_style(FontStyle::Strike));
        assert!(a[6].fc.contains_style(FontStyle::Wave));
        assert_eq!(a[7].text, "p");
    }

    #[test]
    fn lists_headings_and_lines() {
        let s = sheet(&[
            "* one", "** two", "# n1", "# n2", "= H", "----", "== T ==", ".. d ..", "====",
        ]);
        assert!(s.stripes[0].has_header());
        assert!(matches!(s.stripes[0].atoms()[0], Atom::Bullet(_, 0)));
        assert!(matches!(s.stripes[1].atoms()[0], Atom::Bullet(_, 1)));
        let n2 = s.stripes[3].atoms();
        assert_eq!(n2[0].as_text().unwrap().text, "2.");
        assert_eq!(s.stripes[4].atoms()[0].as_text().unwrap().fc.size2d(), 18.0);
        assert!(matches!(
            s.stripes[5].atoms()[0],
            Atom::HLine { style: '-', .. }
        ));
        assert!(matches!(
            s.stripes[6].atoms()[0],
            Atom::HLine {
                style: '=',
                title: Some(_)
            }
        ));
        assert!(matches!(
            s.stripes[7].atoms()[0],
            Atom::HLine { style: '.', .. }
        ));
        assert!(matches!(
            s.stripes[8].atoms()[0],
            Atom::HLine {
                style: '=',
                title: None
            }
        ));
    }

    #[test]
    fn misc() {
        let s = sheet(&[
            "a<sup>2</sup>b<sub>i</sub><space:5>~*not~* <text:fill=red>t</text>",
            "<right>r",
            "",
            "«x» <<y>>",
        ]);
        let atoms = s.stripes[0].atoms();
        assert_eq!(atoms[1].as_text().unwrap().fc.space(), -6);
        assert!(matches!(atoms[4], Atom::Space(w) if w == 5.0));
        assert!(atoms
            .iter()
            .any(|a| a.as_text().is_some_and(|t| t.text == "*not* ")));
        let last = atoms.last().unwrap().as_text().unwrap();
        assert_eq!(
            last.fc.attributes(),
            [("fill".to_string(), "red".to_string())]
        );
        assert_eq!(s.stripes[1].align, HorizontalAlignment::Right);
        assert_eq!(s.stripes[2].atoms()[0].as_text().unwrap().text, " ");
        assert_eq!(s.stripes[3].atoms()[0].as_text().unwrap().text, "«x» «y»");
        let s = sheet(&["see [[http://a.b label]] and [[http://c.d]]"]);
        let t: Vec<_> = s.stripes[0]
            .atoms()
            .iter()
            .filter_map(|a| a.as_text().map(|t| t.text.clone()))
            .collect();
        assert_eq!(t, ["see ", "label", " and ", "http://c.d"]);
        assert_eq!(link_label("u{tip}"), "u");
        // A tooltip may contain spaces; the label follows the closing brace, and without one the URL
        // is the label. Nested braces stay inside the tooltip.
        assert_eq!(link_label("http://x.com{tooltip test}"), "http://x.com");
        assert_eq!(link_label("http://x.com{tooltip test} label"), "label");
        assert_eq!(link_label("http://x.com{tooltip {test}}"), "http://x.com");
        assert_eq!(link_label("http://x.com"), "http://x.com");
        let s = create_sheet(
            &["**x**".to_string(), "= y".to_string()],
            &fc(),
            HorizontalAlignment::Left,
            &SkinSimple {
                guillemet: false,
                ..SkinSimple::default()
            },
            CreoleMode::NoCreole,
        );
        assert_eq!(texts(&s.stripes[1])[0].0, "= y");
    }
}
