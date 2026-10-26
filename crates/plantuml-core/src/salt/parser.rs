//! Tokenizer and element factories of salt diagrams (`DataSourceImpl`, `salt/factory`).

use super::element::{
    default_font, Elmt, ElementBorder, ElementButton, ElementDroplist, ElementImage,
    ElementLine, ElementMenuBar, ElementPyramid, ElementPyramidScrolled, ElementRadioCheckbox,
    ElementTabBar, ElementText, ElementTextField, ElementTree, Positionner, ScrollStrategy,
    TableStrategy, WrappedElement,
};
use crate::string_utils::{eventually_remove_starting_and_ending_double_quote_with, trin};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

type Res<T> = Result<T, String>;

const ESCAPED_PIPE: &str = "\u{E000}";
const ESCAPED_OPEN: &str = "\u{E001}";
const ESCAPED_CLOSE: &str = "\u{E002}";

static BLOCK_START: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\{(?:[-+^#!*/]|S-|SI|S)?").expect("valid"));
static IMAGE_HEADER: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^<<[A-Za-z0-9_]+$").expect("valid"));
static RETRIEVE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^<<[A-Za-z0-9_]+>>$").expect("valid"));

/// Splits `s` like `StringTokenizer(s, "|}", true)`.
fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c == '|' || c == '}' {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            tokens.push(c.to_string());
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

/// An item followed by a column or a row break (`Terminated<String>`).
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub text: String,
    /// True for `Terminator.NEWCOL`, false for `NEWLINE`.
    pub newcol: bool,
}

/// The items of the diagram (`DataSourceImpl`).
pub struct DataSource {
    data: Vec<Item>,
    i: usize,
}

impl DataSource {
    pub fn new(lines: &[String]) -> Self {
        let mut ds = DataSource {
            data: Vec::new(),
            i: 0,
        };
        for raw in lines {
            let s = raw
                .replace("\\|", ESCAPED_PIPE)
                .replace("\\{", ESCAPED_OPEN)
                .replace("\\}", ESCAPED_CLOSE);
            let tokens = tokenize(&s);
            for (idx, tok) in tokens.iter().enumerate() {
                let token = trin(tok);
                if token == "|" {
                    continue;
                }
                let terminator = idx + 1 < tokens.len();
                ds.add_token(token, terminator);
            }
        }
        ds
    }

    fn add_token(&mut self, token: &str, terminator: bool) {
        let mut matches = BLOCK_START.find_iter(token).peekable();
        if matches.peek().is_none() {
            self.add_internal(token, terminator);
            return;
        }
        let (mut last_start, mut end) = (0, 0);
        for m in matches {
            if m.start() > last_start {
                self.add_internal(&token[last_start..m.start()], true);
            }
            end = m.end();
            let t = if end == token.len() { terminator } else { true };
            self.add_internal(&token[m.start()..end], t);
            last_start = end;
        }
        if end < token.len() {
            self.add_internal(&token[end..], terminator);
        }
    }

    fn add_internal(&mut self, s: &str, newcol: bool) {
        let s = trin(s)
            .replace(ESCAPED_PIPE, "|")
            .replace(ESCAPED_OPEN, "{")
            .replace(ESCAPED_CLOSE, "}");
        if !s.is_empty() {
            self.data.push(Item { text: s, newcol });
        }
    }

    fn peek(&self, n: usize) -> Res<&Item> {
        self.data
            .get(self.i + n)
            .ok_or_else(|| "Unexpected end of salt data".to_string())
    }

    fn peek_text(&self, n: usize) -> Res<&str> {
        self.peek(n).map(|i| i.text.as_str())
    }

    fn next(&mut self) -> Res<Item> {
        let item = self.peek(0)?.clone();
        self.i += 1;
        Ok(item)
    }
}

/// Builds the element tree from the items (the `ElementFactory*` classes).
pub struct Parser {
    ds: DataSource,
    dictionary: HashMap<String, Elmt>,
}

fn is_tree_marker(s: &str) -> bool {
    if s == "T" {
        return true;
    }
    let c: Vec<char> = s.chars().collect();
    c.len() == 2 && c[0] == 'T' && TableStrategy::from_char(c[1]).is_some()
}

fn is_border_marker(s: &str) -> bool {
    let c: Vec<char> = s.chars().collect();
    c.len() == 2 && "NSEW".contains(c[0]) && c[1] == '='
}

fn is_line(text: &str, c: char) -> bool {
    let s: String = [c, c].iter().collect();
    text.starts_with(&s) && text.ends_with(&s)
}

fn is_quoted(text: &str, q: char) -> bool {
    text.len() >= 2 && text.starts_with(q) && text.ends_with(q)
}

fn inner(text: &str) -> &str {
    &text[1..text.len() - 1]
}

fn after(text: &str, c: char) -> String {
    let x = text.find(c).map_or(0, |i| i + c.len_utf8());
    trin(&text[x..]).to_string()
}

impl Parser {
    pub fn new(lines: &[String]) -> Self {
        Parser {
            ds: DataSource::new(lines),
            dictionary: HashMap::new(),
        }
    }

    /// First complex factory that is ready (`PSystemSalt.createElement`).
    pub fn create_root(&mut self) -> Res<Elmt> {
        if self.pyramid_ready()? {
            return Ok(self.pyramid()?.0);
        }
        if self.scroll_ready()? {
            return Ok(self.scroll()?.0);
        }
        if self.border_ready()? {
            return Ok(self.border()?.0);
        }
        if self.tree_ready()? {
            return Ok(self.tree()?.0);
        }
        Err("Cannot find a salt element".to_string())
    }

    /// Next element, trying the factories in order (`getNextElement`).
    fn next_element(&mut self) -> Res<(Elmt, bool)> {
        let text = self.ds.peek_text(0)?.to_string();
        let t = text.as_str();
        if t == "{*" {
            return self.menu();
        }
        if self.tree_ready()? {
            return self.tree();
        }
        if t == "{/" {
            return self.tab();
        }
        if ['-', '=', '~', '.'].iter().any(|c| is_line(t, *c)) {
            let item = self.ds.next()?;
            let c = item.text.chars().next().unwrap_or('-');
            return Ok((Rc::new(ElementLine::new(c)), item.newcol));
        }
        let font = default_font(12);
        if is_quoted(t, '"') {
            let item = self.ds.next()?;
            return Ok((
                Rc::new(ElementTextField::new(inner(&item.text), font)),
                item.newcol,
            ));
        }
        if t.starts_with("[X]") {
            return self.check_radio(false, true, ']');
        }
        if t.starts_with("[]") || t.starts_with("[ ]") {
            return self.check_radio(false, false, ']');
        }
        if !t.starts_with("[[") && t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
            let item = self.ds.next()?;
            return Ok((
                Rc::new(ElementButton::new(inner(&item.text), font)),
                item.newcol,
            ));
        }
        if is_quoted(t, '^') {
            let item = self.ds.next()?;
            return Ok((
                Rc::new(ElementDroplist::new(inner(&item.text), font)),
                item.newcol,
            ));
        }
        if t.starts_with("(X)") {
            return self.check_radio(true, true, ')');
        }
        if t.starts_with("()") || t.starts_with("( )") {
            return self.check_radio(true, false, ')');
        }
        if t == "<<" || IMAGE_HEADER.is_match(t) {
            return self.image();
        }
        if RETRIEVE.is_match(t) {
            let item = self.ds.next()?;
            let name = &item.text[2..item.text.len() - 2];
            let found = self
                .dictionary
                .get(name)
                .ok_or_else(|| format!("Unknown salt image {name}"))?;
            return Ok((Rc::new(WrappedElement(found.clone())), item.newcol));
        }
        if !t.starts_with('{') && !t.starts_with('}') && !trin(t).is_empty() {
            let item = self.ds.next()?;
            return Ok((
                Rc::new(ElementText::new(&[item.text], font)),
                item.newcol,
            ));
        }
        if self.pyramid_ready()? {
            return self.pyramid();
        }
        if self.scroll_ready()? {
            return self.scroll();
        }
        if self.border_ready()? {
            return self.border();
        }
        Err(format!("Unexpected salt item {t}"))
    }

    fn check_radio(&mut self, radio: bool, checked: bool, close: char) -> Res<(Elmt, bool)> {
        let item = self.ds.next()?;
        let text = after(&item.text, close);
        Ok((
            Rc::new(ElementRadioCheckbox::new(
                &text,
                default_font(12),
                radio,
                checked,
            )),
            item.newcol,
        ))
    }

    fn image(&mut self) -> Res<(Elmt, bool)> {
        let header = self.ds.next()?.text;
        let name = (header.len() > 2).then(|| header[2..].to_string());
        let mut img = Vec::new();
        while self.ds.peek_text(0)? != ">>" {
            img.push(self.ds.next()?.text);
        }
        let end = self.ds.next()?;
        let element: Elmt = Rc::new(ElementImage::new(&img));
        if let Some(n) = name {
            self.dictionary.insert(n, element.clone());
        }
        Ok((element, end.newcol))
    }

    fn menu(&mut self) -> Res<(Elmt, bool)> {
        self.ds.next()?;
        let mut bar = ElementMenuBar::new(default_font(12));
        let mut subentry: Option<String> = None;
        while self.ds.peek_text(0)? != "}" {
            let t = self.ds.next()?;
            match subentry.as_deref() {
                None => bar.add_entry(&t.text),
                Some("") => subentry = Some(t.text.clone()),
                Some(parent) => bar.add_sub_entry(parent, &t.text)?,
            }
            if !t.newcol {
                subentry = Some(String::new());
            }
        }
        let end = self.ds.next()?;
        Ok((Rc::new(bar), end.newcol))
    }

    fn tab(&mut self) -> Res<(Elmt, bool)> {
        self.ds.next()?;
        let mut bar = ElementTabBar::new(default_font(12));
        while self.ds.peek_text(0)? != "}" {
            let t = self.ds.next()?;
            bar.add_tab(&t.text);
            if !t.newcol {
                bar.vertical = true;
            }
        }
        let end = self.ds.next()?;
        Ok((Rc::new(bar), end.newcol))
    }

    fn tree_ready(&self) -> Res<bool> {
        if self.ds.peek_text(0)? == "{" {
            return Ok(is_tree_marker(self.ds.peek_text(1)?));
        }
        Ok(false)
    }

    fn tree(&mut self) -> Res<(Elmt, bool)> {
        self.ds.next()?;
        let text_t = self.ds.next()?.text;
        let mut strategy = TableStrategy::DrawNone;
        if text_t.chars().count() == 2 {
            strategy = text_t
                .chars()
                .nth(1)
                .and_then(TableStrategy::from_char)
                .ok_or("Bad tree marker")?;
        }
        let mut tree = ElementTree::new(default_font(12), strategy);
        let mut take_me = true;
        let mut depth = 0;
        while depth > 0 || self.ds.peek_text(0)? != "}" {
            if take_me {
                let t = self.ds.next()?;
                if t.text == "}" {
                    depth -= 1;
                } else if t.text.starts_with('{') {
                    depth += 1;
                }
                tree.add_entry(&t.text);
                take_me = !t.newcol;
            } else {
                let (e, newcol) = self.next_element()?;
                tree.add_cell_to_entry(e);
                take_me = !newcol;
            }
        }
        let end = self.ds.next()?;
        Ok((Rc::new(tree), end.newcol))
    }

    fn pyramid_ready(&self) -> Res<bool> {
        let text = self.ds.peek_text(0)?;
        if ["{", "{+", "{^", "{#", "{!", "{-"].contains(&text) {
            let t1 = self.ds.peek_text(1)?;
            return Ok(!(is_border_marker(t1) || is_tree_marker(t1)));
        }
        Ok(false)
    }

    fn pyramid(&mut self) -> Res<(Elmt, bool)> {
        let tmp = self.ds.next()?;
        let mut title = None;
        let mut strategy = TableStrategy::DrawNone;
        if tmp.text.chars().count() == 2 {
            strategy = tmp
                .text
                .chars()
                .nth(1)
                .and_then(TableStrategy::from_char)
                .ok_or("Bad table marker")?;
        }
        if strategy == TableStrategy::DrawOutsideWithTitle && tmp.newcol {
            title = Some(eventually_remove_starting_and_ending_double_quote_with(
                &self.ds.next()?.text,
                "\"",
            ));
        }
        let mut positionner = Positionner::default();
        while self.ds.peek_text(0)? != "}" {
            let (e, newcol) = self.next_element()?;
            if e.text() == Some("*") {
                positionner.merge_left(newcol);
            } else {
                positionner.add(e, newcol);
            }
        }
        let end = self.ds.next()?;
        Ok((
            Rc::new(ElementPyramid::new(positionner, strategy, title.as_deref())),
            end.newcol,
        ))
    }

    fn scroll_ready(&self) -> Res<bool> {
        Ok(["{S", "{S-", "{SI"].contains(&self.ds.peek_text(0)?))
    }

    fn scroll(&mut self) -> Res<(Elmt, bool)> {
        let header = self.ds.next()?.text;
        let mut positionner = Positionner::default();
        while self.ds.peek_text(0)? != "}" {
            let (e, newcol) = self.next_element()?;
            positionner.add(e, newcol);
        }
        let end = self.ds.next()?;
        Ok((
            Rc::new(ElementPyramidScrolled::new(
                positionner,
                ScrollStrategy::from_desc(&header),
            )),
            end.newcol,
        ))
    }

    fn border_ready(&self) -> Res<bool> {
        let text = self.ds.peek_text(0)?;
        if ["{", "{+", "{#", "{!", "{-"].contains(&text) {
            let t1 = self.ds.peek_text(1)?;
            return Ok(is_border_marker(t1));
        }
        Ok(false)
    }

    fn border(&mut self) -> Res<(Elmt, bool)> {
        self.ds.next()?;
        let mut result = ElementBorder::new();
        while self.ds.peek_text(0)? != "}" {
            let pos = self.ds.next()?.text;
            let slot = match pos.chars().next() {
                Some('N') => &mut result.north,
                Some('S') => &mut result.south,
                Some('E') => &mut result.east,
                Some('W') => &mut result.west,
                Some('C') => &mut result.center,
                _ => return Err(format!("Bad border position {pos}")),
            };
            let (e, _) = self.next_element()?;
            *slot = e;
        }
        let end = self.ds.next()?;
        Ok((Rc::new(result), end.newcol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(lines: &[&str]) -> Vec<(String, bool)> {
        let l: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        DataSource::new(&l)
            .data
            .into_iter()
            .map(|i| (i.text, i.newcol))
            .collect()
    }

    #[test]
    fn splits_columns_and_blocks() {
        let r = items(&["Modifiers: | { (X) public | () default }"]);
        let want = [
            ("Modifiers:", true),
            ("{", true),
            ("(X) public", true),
            ("() default", true),
            ("}", false),
        ];
        assert_eq!(r.len(), want.len());
        for (a, b) in r.iter().zip(want) {
            assert_eq!((a.0.as_str(), a.1), b);
        }
    }

    #[test]
    fn escapes() {
        let r = items(&["3\\|4 | a\\{b\\}"]);
        assert_eq!(r[0], ("3|4".to_string(), true));
        assert_eq!(r[1], ("a{b}".to_string(), false));
    }

    #[test]
    fn rejects_unbalanced() {
        let l = vec!["{".to_string(), "a".to_string()];
        assert!(Parser::new(&l).create_root().is_err());
        let l = vec!["a".to_string()];
        assert!(Parser::new(&l).create_root().is_err());
    }
}
