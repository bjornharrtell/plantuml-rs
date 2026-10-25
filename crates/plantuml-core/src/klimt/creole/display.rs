//! Text content of a label (`Display`) and its conversion to a text block.

use super::{create_sheet_items, CreoleMode, LineBreakStrategy, SheetBlock, SheetItem, SkinSimple};
use crate::klimt::color::HColor;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::UFont;
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::{HorizontalBlock, MargedBlock, TextBlock};
use crate::stereo::{Guillemet, Stereotype};
use crate::style::{Margins, PName, Style};
use regex::Regex;
use std::rc::Rc;
use std::sync::LazyLock;

/// One line of a [`Display`].
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayLine {
    Text(String),
    Stereotype(Stereotype),
    /// Autonumber prefix of a message.
    MessageNumber(String),
}

impl DisplayLine {
    /// Text of the line as Java's `toString()`.
    pub fn text(&self) -> String {
        match self {
            DisplayLine::Text(s) | DisplayLine::MessageNumber(s) => s.clone(),
            DisplayLine::Stereotype(s) => s.to_string(),
        }
    }
}

/// Lines of text, possibly with a stereotype and a natural alignment.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    lines: Vec<DisplayLine>,
    natural_alignment: Option<HorizontalAlignment>,
    is_null: bool,
    creole_mode: CreoleMode,
    show_stereotype: bool,
}

const NEWLINE: char = '\u{E100}';
const NEWLINE_LEFT: char = '\u{E101}';
const NEWLINE_RIGHT: char = '\u{E102}';
const BREAKLINE: char = '\u{E103}';
const REAL_BACKSLASH: char = '\u{E110}';
const INVISIBLE_QUOTE: char = '\u{E121}';

static ENDING_STEREOTYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)^(.*?)(<<\s*(.*)\s*>>)\s*$").expect("valid"));
static UNDERLINED_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)^([^:]+?)(\s*:.+)$").expect("valid"));

impl Display {
    pub fn null() -> Display {
        Display {
            lines: Vec::new(),
            natural_alignment: None,
            is_null: true,
            creole_mode: CreoleMode::Full,
            show_stereotype: true,
        }
    }

    pub fn empty() -> Display {
        Display {
            is_null: false,
            ..Display::null()
        }
    }

    /// One text line per element.
    pub fn create<S: AsRef<str>>(lines: &[S]) -> Display {
        Display {
            lines: lines
                .iter()
                .map(|l| DisplayLine::Text(l.as_ref().to_string()))
                .collect(),
            ..Display::empty()
        }
    }

    /// Splits `\n`, `\t`, `\\` escapes and the internal newline marks (`getWithNewlines`).
    pub fn with_newlines(s: Option<&str>) -> Display {
        let Some(s) = s else {
            return Display::null();
        };
        let chars: Vec<char> = s.chars().collect();
        let mut result = Vec::new();
        let mut current = String::new();
        let mut natural = None;
        let mut raw_mode = false;
        let mut i = 0;
        let starts = |i: usize, p: &str| {
            chars[i..]
                .iter()
                .take(p.chars().count())
                .collect::<String>()
                == p
        };
        while i < chars.len() {
            let c = chars[i];
            if starts(i, "<math>") || starts(i, "<latex>") || starts(i, "[[") {
                raw_mode = true;
            } else if starts(i, "</math>") || starts(i, "</latex>") || starts(i, "]]") {
                raw_mode = false;
            }
            if !raw_mode && c == '\\' && i + 1 < chars.len() {
                let c2 = chars[i + 1];
                i += 1;
                match c2 {
                    'n' | 'r' | 'l' => {
                        if c2 == 'r' {
                            natural = Some(HorizontalAlignment::Right);
                        } else if c2 == 'l' {
                            natural = Some(HorizontalAlignment::Left);
                        }
                        result.push(std::mem::take(&mut current));
                    }
                    't' => current.push('\t'),
                    '\\' => current.push('\\'),
                    other => {
                        current.push(c);
                        current.push(other);
                    }
                }
            } else if c == '\u{E111}' {
                current.push(c);
            } else if c == REAL_BACKSLASH {
                current.push('\\');
            } else if c == NEWLINE_LEFT {
                natural = Some(HorizontalAlignment::Left);
                result.push(std::mem::take(&mut current));
            } else if c == INVISIBLE_QUOTE {
            } else if c == NEWLINE_RIGHT {
                natural = Some(HorizontalAlignment::Right);
                result.push(std::mem::take(&mut current));
            } else if (!raw_mode && c == NEWLINE) || c == BREAKLINE {
                result.push(std::mem::take(&mut current));
            } else {
                current.push(c);
            }
            i += 1;
        }
        result.push(current);
        Display {
            natural_alignment: natural,
            ..Display::create(&result)
        }
    }

    pub fn is_null(&self) -> bool {
        self.is_null
    }

    pub fn size(&self) -> usize {
        self.lines.len()
    }

    pub fn lines(&self) -> &[DisplayLine] {
        &self.lines
    }

    /// Text of line `i` (stereotypes use their `toString`).
    pub fn text_at(&self, i: usize) -> String {
        self.lines[i].text()
    }

    pub fn natural_alignment(&self) -> Option<HorizontalAlignment> {
        self.natural_alignment
    }

    pub fn is_white(&self) -> bool {
        self.is_null
            || self.lines.is_empty()
            || (self.lines.len() == 1 && self.lines[0].text().chars().all(char::is_whitespace))
    }

    pub fn show_stereotype(&self) -> bool {
        self.show_stereotype
    }

    pub fn without_stereotype_if_needed(&self, used_style: &Style) -> Display {
        if self.is_null {
            return self.clone();
        }
        let v = used_style.value(PName::ShowStereotype);
        if matches!(v, crate::style::Value::Null) || v.as_boolean() {
            return self.clone();
        }
        Display {
            show_stereotype: false,
            ..self.clone()
        }
    }

    pub fn stereotype_if_any(&self) -> Option<&Stereotype> {
        self.lines.iter().find_map(|l| match l {
            DisplayLine::Stereotype(s) => Some(s),
            _ => None,
        })
    }

    fn map_text(&self, f: impl Fn(&str) -> String) -> Display {
        Display {
            lines: self
                .lines
                .iter()
                .map(|l| match l {
                    DisplayLine::Text(s) => DisplayLine::Text(f(s)),
                    other => DisplayLine::Text(f(&other.text())),
                })
                .collect(),
            ..self.clone()
        }
    }

    pub fn replace(&self, src: &str, dest: &str) -> Display {
        if self.is_null {
            return self.clone();
        }
        Display {
            lines: self
                .lines
                .iter()
                .map(|l| match l {
                    DisplayLine::Text(s) if s.contains(src) => {
                        DisplayLine::Text(s.replace(src, dest))
                    }
                    other => other.clone(),
                })
                .collect(),
            ..self.clone()
        }
    }

    pub fn replace_backslash_t(&self) -> Display {
        self.replace("\\t", "\t")
    }

    pub fn underlined(&self) -> Display {
        self.map_text(|s| format!("<u>{s}"))
    }

    pub fn underlined_name(&self) -> Display {
        let mut first = true;
        Display {
            lines: self
                .lines
                .iter()
                .map(|l| {
                    let text = l.text();
                    let out = if first {
                        match UNDERLINED_NAME.captures(&text) {
                            Some(m) => format!("<u>{}</u>{}", &m[1], &m[2]),
                            None => format!("<u>{text}"),
                        }
                    } else {
                        format!("<u>{text}")
                    };
                    first = false;
                    DisplayLine::Text(out)
                })
                .collect(),
            ..self.clone()
        }
    }

    pub fn add(&self, line: DisplayLine) -> Display {
        let mut d = self.clone();
        d.lines.push(line);
        d
    }

    pub fn add_first(&self, line: DisplayLine) -> Display {
        let mut d = self.clone();
        d.lines.insert(0, line);
        d
    }

    pub fn add_all(&self, other: &Display) -> Display {
        let mut d = self.clone();
        d.lines.extend(other.lines.iter().cloned());
        d
    }

    pub fn append_first_line(&self, appended: &str) -> Display {
        let mut d = self.clone();
        if let Some(first) = d.lines.first_mut() {
            *first = DisplayLine::Text(format!("{appended}{}", first.text()));
        }
        d
    }

    /// Appends `<generic>` to the last line (`addGeneric`).
    pub fn add_generic(&self, generic: &str) -> Display {
        let mut d = self.clone();
        match d.lines.last_mut() {
            None => d.lines.push(DisplayLine::Text(format!("<{generic}>"))),
            Some(last) => *last = DisplayLine::Text(format!("{}<{generic}>", last.text())),
        }
        d
    }

    pub fn sub_list(&self, from: usize, to: usize) -> Display {
        Display {
            lines: self.lines[from..to].to_vec(),
            ..self.clone()
        }
    }

    pub fn with_creole_mode(&self, mode: CreoleMode) -> Display {
        Display {
            creole_mode: mode,
            ..self.clone()
        }
    }

    pub fn creole_mode(&self) -> CreoleMode {
        self.creole_mode
    }

    pub fn manage_guillemet(&self, manage_visibility_modifier: bool) -> Display {
        let g = Guillemet::Guillemet;
        let _ = g;
        Display {
            lines: self
                .lines
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut s = l.text();
                    if manage_visibility_modifier
                        && i == 0
                        && matches!(s.chars().next(), Some('+' | '-' | '#' | '~'))
                    {
                        s = s[1..].trim().to_string();
                    }
                    DisplayLine::Text(manage_guillemet_text(&s))
                })
                .collect(),
            ..self.clone()
        }
    }

    pub fn with_page(&self, page: usize, last_page: usize) -> Display {
        self.map_text(|s| {
            s.replace("%page%", &page.to_string())
                .replace("%lastpage%", &last_page.to_string())
        })
    }

    pub fn ending_stereotype(&self) -> Option<Stereotype> {
        let last = self.lines.last()?.text();
        let m = ENDING_STEREOTYPE.captures(&last)?;
        Some(Stereotype::build(&m[2]))
    }

    pub fn remove_ending_stereotype(&self) -> Display {
        let mut d = self.clone();
        if let Some(last) = self.lines.last() {
            if let Some(m) = ENDING_STEREOTYPE.captures(&last.text()) {
                let n = d.lines.len() - 1;
                d.lines[n] = DisplayLine::Text(m[1].to_string());
            }
        }
        d
    }

    fn items(&self) -> Vec<SheetItem> {
        let mut items = Vec::new();
        for l in &self.lines {
            match l {
                DisplayLine::Text(s) | DisplayLine::MessageNumber(s) => {
                    items.push(SheetItem::Text(s.clone()));
                }
                DisplayLine::Stereotype(s) => {
                    if self.show_stereotype {
                        items.push(SheetItem::Stereotype(s.labels(Guillemet::Guillemet)));
                    }
                }
            }
        }
        items
    }

    #[allow(clippy::too_many_arguments)]
    fn creole_block(
        &self,
        fc: &FontConfiguration,
        stereotype_fc: &FontConfiguration,
        align: HorizontalAlignment,
        skin: &SkinSimple,
        max: &LineBreakStrategy,
        mode: CreoleMode,
        margin_x1: f64,
        margin_x2: f64,
    ) -> Rc<dyn TextBlock> {
        let sheet = create_sheet_items(&self.items(), fc, stereotype_fc, align, skin, mode);
        Rc::new(SheetBlock::new(
            sheet,
            max.clone(),
            skin.padding,
            margin_x1,
            margin_x2,
        ))
    }

    /// Lays the text out (`Display.create0`). Spotted stereotypes are drawn by callers.
    #[allow(clippy::too_many_arguments)]
    pub fn create_block(
        &self,
        fc: &FontConfiguration,
        align: HorizontalAlignment,
        skin: &SkinSimple,
        max: &LineBreakStrategy,
        mode: CreoleMode,
        stereotype_font: Option<UFont>,
        stereotype_color: Option<HColor>,
        margin_x1: f64,
        margin_x2: f64,
    ) -> Rc<dyn TextBlock> {
        let align = self.natural_alignment.unwrap_or(align);
        let stereotype_fc = fc.force_font(stereotype_font, stereotype_color);
        if matches!(self.lines.first(), Some(DisplayLine::MessageNumber(_))) {
            let tb1 = self.sub_list(0, 1).creole_block(
                fc,
                &stereotype_fc,
                align,
                skin,
                max,
                CreoleMode::Full,
                margin_x1,
                margin_x2,
            );
            let tb1: Rc<dyn TextBlock> = Rc::new(MargedBlock {
                inner: tb1,
                margins: Margins::new(0.0, 4.0, 0.0, 0.0),
            });
            let tb2 = self.sub_list(1, self.size()).creole_block(
                fc,
                &stereotype_fc,
                align,
                skin,
                max,
                CreoleMode::Full,
                margin_x1,
                margin_x2,
            );
            return Rc::new(HorizontalBlock {
                blocks: vec![tb1, tb2],
                alignment: VerticalAlignment::Center,
            });
        }
        self.creole_block(
            fc,
            &stereotype_fc,
            align,
            skin,
            max,
            mode,
            margin_x1,
            margin_x2,
        )
    }

    /// Shorthand for plain text blocks without stereotype handling.
    pub fn create_simple_block(
        &self,
        fc: &FontConfiguration,
        align: HorizontalAlignment,
        skin: &SkinSimple,
    ) -> Rc<dyn TextBlock> {
        self.create_block(
            fc,
            align,
            skin,
            &LineBreakStrategy::NONE,
            self.creole_mode,
            None,
            None,
            0.0,
            0.0,
        )
    }
}

/// `<<x>>` to `«x»` for the default guillemet.
pub fn manage_guillemet_text(s: &str) -> String {
    static G: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<<\s?((?:<&\w+>|[^<>])+?)\s?>>").expect("valid"));
    if !s.contains('<') {
        return s.to_string();
    }
    G.replace_all(s, "\u{AB}$1\u{BB}").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::font::{EmbeddedFontBounder, UFont};

    fn fc() -> FontConfiguration {
        FontConfiguration::black_blue_true(UFont::sans_serif(14))
    }

    #[test]
    fn newlines_and_alignment() {
        let d = Display::with_newlines(Some("a\\nb\\rc\\ld\\te\\\\f\\z"));
        let t: Vec<String> = (0..d.size()).map(|i| d.text_at(i)).collect();
        assert_eq!(t, ["a", "b", "c", "d\te\\f\\z"]);
        assert_eq!(d.natural_alignment(), Some(HorizontalAlignment::Left));
        assert!(Display::with_newlines(None).is_null());
        let d = Display::with_newlines(Some(
            "x\u{E100}y\u{E101}z\u{E102}w\u{E103}v\u{E110}\u{E121}",
        ));
        assert_eq!(d.size(), 5);
        assert_eq!(d.text_at(4), "v\\");
        let raw = Display::with_newlines(Some("[[u a\\nb]]\\nc"));
        assert_eq!(raw.size(), 2);
        assert_eq!(raw.text_at(0), "[[u a\\nb]]");
    }

    #[test]
    fn editing() {
        let d = Display::create(&["a", "b"]);
        assert_eq!(d.add(DisplayLine::Text("c".into())).size(), 3);
        assert_eq!(d.add_first(DisplayLine::Text("z".into())).text_at(0), "z");
        assert_eq!(d.add_all(&d).size(), 4);
        assert_eq!(d.append_first_line("> ").text_at(0), "> a");
        assert_eq!(d.add_generic("T").text_at(1), "b<T>");
        assert_eq!(Display::empty().add_generic("T").text_at(0), "<T>");
        assert_eq!(d.replace("a", "x").text_at(0), "x");
        assert_eq!(d.underlined().text_at(1), "<u>b");
        assert_eq!(
            Display::create(&["name : type", "other"])
                .underlined_name()
                .text_at(0),
            "<u>name</u> : type"
        );
        assert_eq!(
            Display::create(&["plain"]).underlined_name().text_at(0),
            "<u>plain"
        );
        assert_eq!(d.sub_list(1, 2).size(), 1);
        assert_eq!(d.with_page(2, 5).size(), 2);
        assert_eq!(
            Display::create(&["p%page%/%lastpage%"])
                .with_page(2, 5)
                .text_at(0),
            "p2/5"
        );
        assert!(Display::empty().is_white());
        assert!(Display::create(&["  "]).is_white());
        assert!(!d.is_white());
        assert_eq!(
            Display::create(&["a\\tb"]).replace_backslash_t().text_at(0),
            "a\tb"
        );
        assert_eq!(
            d.with_creole_mode(CreoleMode::NoCreole).creole_mode(),
            CreoleMode::NoCreole
        );
        assert!(Display::null().is_null() && !d.is_null());
    }

    #[test]
    fn stereotypes_and_guillemets() {
        let d = Display::create(&["Foo <<bar>>"]);
        let s = d.ending_stereotype().unwrap();
        assert_eq!(s.multiple_labels(), ["bar"]);
        assert_eq!(d.remove_ending_stereotype().text_at(0), "Foo ");
        assert!(Display::create(&["x"]).ending_stereotype().is_none());
        assert_eq!(
            Display::create(&["x"])
                .remove_ending_stereotype()
                .text_at(0),
            "x"
        );
        let with = d.add_first(DisplayLine::Stereotype(Stereotype::build("<<a>>")));
        assert!(with.stereotype_if_any().is_some());
        assert_eq!(with.text_at(0), "<<a>>");
        assert_eq!(
            Display::create(&["+m <<s>>"])
                .manage_guillemet(true)
                .text_at(0),
            "m \u{AB}s\u{BB}"
        );
        assert_eq!(
            Display::create(&["+m"]).manage_guillemet(false).text_at(0),
            "+m"
        );
    }

    #[test]
    fn blocks() {
        let skin = SkinSimple::default();
        let sb = EmbeddedFontBounder;
        let one = Display::create(&["Hello"]).create_simple_block(
            &fc(),
            HorizontalAlignment::Left,
            &skin,
        );
        let d1 = one.calculate_dimension(&sb);
        let st =
            Display::create(&["Hello"]).add(DisplayLine::Stereotype(Stereotype::build("<<s>>")));
        let b = st.create_block(
            &fc(),
            HorizontalAlignment::Center,
            &skin,
            &LineBreakStrategy::NONE,
            CreoleMode::Full,
            Some(UFont::sans_serif(12)),
            None,
            0.0,
            0.0,
        );
        assert!(b.calculate_dimension(&sb).height > d1.height);
        let hidden = st.without_stereotype_if_needed(
            &crate::style::StyleBuilder::default_skin()
                .merged_style(&crate::style::StyleQuery::of(&[crate::style::SName::Root]))
                .unwrap(),
        );
        assert_eq!(hidden.size(), 2);
        let n = Display::empty()
            .add(DisplayLine::MessageNumber("1".into()))
            .add(DisplayLine::Text("msg".into()));
        let nb = n.create_simple_block(&fc(), HorizontalAlignment::Left, &skin);
        assert!(nb.calculate_dimension(&sb).width > d1.width * 0.5);
        let _ = DisplayLine::MessageNumber("2".into()).text();
    }
}
