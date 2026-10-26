//! Unicode bracketed expressions (`com.plantuml.ubrex`): the small, case-insensitive and
//! non-backtracking pattern language the Gantt commands are written in.
//!
//! A pattern is a sequence of atoms: a plain character, `〴s` (a character class, upper case
//! meaning "not"), `「abc」` (a character set, `〤` first meaning "not", `〜` for ranges),
//! `【a┇b】` (first matching alternative), `〇+x`, `〇*x`, `〇?x`, `〇{1-2}x` (repetitions),
//! `〄>x` (everything up to `x`), `〘...〙` (a group) and `〒$` (end of text). Spaces are
//! ignored and `∙` stands for a space.

use std::rc::Rc;

/// What a character class accepts (`CharClassRaw`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassRaw {
    Any,
    Space,
    Guillemet,
    Word,
    Digit,
    AlphaNumeric,
    Letter,
}

impl ClassRaw {
    fn matches(self, ch: char) -> bool {
        match self {
            ClassRaw::Any => true,
            ClassRaw::Space => ch == ' ',
            ClassRaw::Guillemet => ch == '"' || ch == '“' || ch == '”',
            ClassRaw::Word => ch.is_ascii_alphanumeric() || ch == '_',
            ClassRaw::Digit => ch.is_ascii_digit() || ch == '_',
            ClassRaw::AlphaNumeric => ch.is_alphanumeric(),
            ClassRaw::Letter => ch.is_alphabetic(),
        }
    }

    fn from_definition(ch: char) -> Result<ClassRaw, String> {
        Ok(match ch {
            'S' | 's' => ClassRaw::Space,
            'G' | 'g' => ClassRaw::Guillemet,
            'D' | 'd' => ClassRaw::Digit,
            'W' | 'w' => ClassRaw::Word,
            'A' | 'a' => ClassRaw::AlphaNumeric,
            'L' | 'l' => ClassRaw::Letter,
            '.' => ClassRaw::Any,
            _ => return Err(format!("unknown character class {ch}")),
        })
    }

    /// Length of the syntax of the class in the definition (`le` and `an` take two).
    fn definition_length(self) -> usize {
        match self {
            ClassRaw::AlphaNumeric | ClassRaw::Letter => 2,
            _ => 1,
        }
    }
}

/// The values a repetition may take (`Repetition`).
#[derive(Debug, Clone, Default)]
pub struct Repetition {
    values: Vec<usize>,
    min_inclusive: Option<usize>,
}

impl Repetition {
    fn matches(&self, nb: usize) -> bool {
        self.min_inclusive.is_some_and(|m| nb >= m) || self.values.contains(&nb)
    }

    fn add_token(&mut self, token: &str) -> Result<(), String> {
        let num = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| format!("bad repetition {token}"))
        };
        if token.is_empty() {
            return Err("empty repetition token".into());
        }
        if let Some(min) = token.strip_suffix('+') {
            self.min_inclusive = Some(num(min)?);
        } else if let Some((a, b)) = token.split_once('-') {
            for i in num(a)?..=num(b)? {
                self.values.push(i);
            }
        } else {
            self.values.push(num(token)?);
        }
        Ok(())
    }
}

/// The values captured so far, in order (`Capture`).
#[derive(Debug, Clone, Default)]
pub struct Capture(Vec<(String, String)>);

impl Capture {
    fn merge(mut self, other: Capture) -> Capture {
        if self.0.is_empty() {
            return other;
        }
        self.0.extend(other.0);
        self
    }

    fn with_prefixed_keys(mut self, prefix: &str) -> Capture {
        for e in &mut self.0 {
            e.0 = format!("{prefix}/{}", e.0);
        }
        self
    }

    fn with_entry(mut self, key: &str, value: String) -> Capture {
        self.0.push((key.to_string(), value));
        self
    }

    /// The values of all entries with this exact key.
    pub fn find_values_by_key(&self, key: &str) -> Vec<String> {
        self.0
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn find_first_value_by_key(&self, key: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// The entries under `prefix/`, with the prefix removed.
    pub fn extract_by_prefix(&self, prefix: &str) -> Capture {
        let p = format!("{prefix}/");
        Capture(
            self.0
                .iter()
                .filter_map(|(k, v)| k.strip_prefix(&p).map(|k| (k.to_string(), v.clone())))
                .collect(),
        )
    }
}

/// Text being matched, possibly seen backwards (`TextNavigator`).
struct Nav<'a> {
    chars: &'a [char],
    p1: usize,
    p2: usize,
    reversed: bool,
}

impl Nav<'_> {
    fn len(&self) -> usize {
        self.p2 - self.p1
    }

    fn at(&self, i: usize) -> char {
        if self.reversed {
            self.chars[self.p2 - i - 1]
        } else {
            self.chars[self.p1 + i]
        }
    }

    fn reverse(&self, pos: usize) -> Nav<'_> {
        Nav {
            chars: self.chars,
            p1: 0,
            p2: self.p1 + pos,
            reversed: true,
        }
    }

    fn sub(&self, from: usize, to: usize) -> String {
        (from..to).map(|i| self.at(i)).collect()
    }
}

/// A compiled pattern (`Challenge`).
#[derive(Debug)]
pub enum Ch {
    Single(char),
    Class(ClassRawPub, bool),
    Set {
        classes: Vec<ClassRawPub>,
        chars: Box<[bool; 128]>,
        reversed: bool,
    },
    List(Vec<Rc<Ch>>),
    Alternative(Vec<Rc<Ch>>),
    Named(String, Vec<Rc<Ch>>),
    OneOrMore(Rc<Ch>),
    ZeroOrMore(Rc<Ch>),
    Optional(Rc<Ch>),
    Repeat(Repetition, Rc<Ch>),
    UpTo(Rc<Ch>),
    OneOrMoreUpTo(Rc<Ch>, Rc<Ch>),
    Lazy(Rc<Ch>, Rc<Ch>),
    LookAhead(Rc<Ch>, bool),
    LookBehind(Rc<Ch>, bool),
    EndOfText,
}

/// Opaque wrapper so that `Ch` can be public while the class list is not.
#[derive(Debug, Clone, Copy)]
pub struct ClassRawPub(ClassRaw);

type Run = Option<(usize, Capture)>;

fn ok(len: usize) -> Run {
    Some((len, Capture::default()))
}

fn run_seq(list: &[Rc<Ch>], nav: &Nav, pos: usize) -> Run {
    let mut capture = Capture::default();
    let mut cur = pos;
    for c in list {
        let (len, cap) = c.run(nav, cur)?;
        cur += len;
        capture = capture.merge(cap);
    }
    Some((cur - pos, capture))
}

impl Ch {
    fn run(&self, nav: &Nav, pos: usize) -> Run {
        match self {
            Ch::Single(ch) => {
                if pos >= nav.len() {
                    return None;
                }
                (nav.at(pos).to_ascii_lowercase() == *ch).then(|| (1, Capture::default()))
            }
            Ch::Class(raw, negative) => {
                if pos >= nav.len() {
                    return None;
                }
                (raw.0.matches(nav.at(pos)) != *negative).then(|| (1, Capture::default()))
            }
            Ch::Set {
                classes,
                chars,
                reversed,
            } => {
                if pos >= nav.len() {
                    return None;
                }
                let ch = nav.at(pos);
                let lower = ch.to_ascii_lowercase();
                let in_set = (lower as u32 >= 32 && (lower as u32) < 128 && chars[lower as usize])
                    || classes.iter().any(|c| c.0.matches(ch));
                (in_set != *reversed).then(|| (1, Capture::default()))
            }
            Ch::List(list) => run_seq(list, nav, pos),
            Ch::Alternative(alts) => alts.iter().find_map(|a| a.run(nav, pos)),
            Ch::Named(name, list) => {
                let (len, capture) = run_seq(list, nav, pos)?;
                let value = nav.sub(pos, pos + len);
                Some((
                    len,
                    capture.with_prefixed_keys(name).with_entry(name, value),
                ))
            }
            Ch::OneOrMore(origin) => {
                let mut capture = Capture::default();
                let mut cur = pos;
                loop {
                    match origin.run(nav, cur) {
                        None => {
                            return (cur > pos).then_some((cur - pos, capture));
                        }
                        Some((0, _)) => return (cur > pos).then_some((cur - pos, capture)),
                        Some((len, cap)) => {
                            capture = capture.merge(cap);
                            cur += len;
                        }
                    }
                }
            }
            Ch::ZeroOrMore(origin) => {
                let mut capture = Capture::default();
                let mut cur = pos;
                loop {
                    match origin.run(nav, cur) {
                        None | Some((0, _)) => return Some((cur - pos, capture)),
                        Some((len, cap)) => {
                            capture = capture.merge(cap);
                            cur += len;
                        }
                    }
                }
            }
            Ch::Optional(origin) => origin.run(nav, pos).or_else(|| ok(0)),
            Ch::Repeat(rep, origin) => {
                let mut capture = Capture::default();
                let mut cur = pos;
                let mut count = 0;
                loop {
                    match origin.run(nav, cur) {
                        None | Some((0, _)) => {
                            return (cur > pos && rep.matches(count))
                                .then_some((cur - pos, capture));
                        }
                        Some((len, cap)) => {
                            capture = capture.merge(cap);
                            cur += len;
                            count += 1;
                        }
                    }
                }
            }
            Ch::UpTo(origin) => {
                let mut cur = pos;
                loop {
                    if origin.run(nav, cur).is_some() {
                        return ok(cur - pos);
                    }
                    cur += 1;
                    if cur > nav.len() {
                        return None;
                    }
                }
            }
            Ch::OneOrMoreUpTo(origin, end) => {
                let mut capture = Capture::default();
                let mut cur = pos;
                loop {
                    let (len, cap) = origin.run(nav, cur)?;
                    if len == 0 {
                        return None;
                    }
                    capture = capture.merge(cap);
                    cur += len;
                    if end.run(nav, cur).is_some() {
                        return Some((cur - pos, capture));
                    }
                }
            }
            Ch::Lazy(origin, stop) => {
                let mut capture = Capture::default();
                let mut cur = pos;
                loop {
                    let (len, cap) = origin.run(nav, cur)?;
                    if len == 0 {
                        return None;
                    }
                    capture = capture.merge(cap);
                    cur += len;
                    if stop.run(nav, cur).is_some() {
                        return Some((cur - pos, capture));
                    }
                }
            }
            Ch::LookAhead(origin, positive) => {
                (origin.run(nav, pos).is_some() == *positive).then(|| (0, Capture::default()))
            }
            Ch::LookBehind(origin, positive) => {
                let rev = nav.reverse(pos);
                (origin.run(&rev, 0).is_some() == *positive).then(|| (0, Capture::default()))
            }
            Ch::EndOfText => (pos >= nav.len()).then(|| (0, Capture::default())),
        }
    }
}

/// A compiled pattern shared between commands.
pub type Ub = Rc<Ch>;

struct Cur<'a> {
    s: &'a [char],
    p: usize,
}

impl Cur<'_> {
    fn len(&self) -> usize {
        self.s.len() - self.p
    }

    fn at(&self, i: usize) -> Result<char, String> {
        self.s
            .get(self.p + i)
            .copied()
            .ok_or_else(|| "unexpected end of definition".to_string())
    }

    fn jump(&mut self, n: usize) {
        self.p += n;
    }
}

fn closing_bracket(cur: &Cur, open: char, close: char) -> Option<usize> {
    let mut level = 0;
    for i in 1..cur.len() {
        let ch = cur.s[cur.p + i];
        if ch == open {
            level += 1;
        } else if ch == close {
            if level == 0 {
                return Some(i);
            }
            level -= 1;
        }
    }
    None
}

fn parse_list(chars: &[char]) -> Result<Vec<Rc<Ch>>, String> {
    if chars.is_empty() {
        return Err("empty definition".into());
    }
    let mut cur = Cur { s: chars, p: 0 };
    parse_all(&mut cur)
}

fn parse_all(cur: &mut Cur) -> Result<Vec<Rc<Ch>>, String> {
    let mut out = Vec::new();
    while cur.len() > 0 {
        if cur.at(0)? == ' ' {
            cur.jump(1);
        } else {
            out.extend(parse_atom(cur)?);
        }
    }
    Ok(out)
}

fn parse_single(cur: &mut Cur) -> Result<Rc<Ch>, String> {
    let mut v = parse_atom(cur)?;
    if v.len() != 1 {
        return Err("expected a single atom".into());
    }
    Ok(v.remove(0))
}

fn parse_atom(cur: &mut Cur) -> Result<Vec<Rc<Ch>>, String> {
    let c = cur.at(0)?;
    match c {
        '┇' => Err("unexpected ┇".into()),
        '〒' => {
            cur.jump(1);
            let c0 = cur.at(0)?;
            if c0 == '$' {
                cur.jump(1);
                return Ok(vec![Rc::new(Ch::EndOfText)]);
            }
            let (behind, positive, size) = match (c0, cur.at(1).ok()) {
                ('=', _) => (false, true, 1),
                ('!', _) => (false, false, 1),
                ('<', Some('=')) => (true, true, 2),
                ('<', Some('!')) => (true, false, 2),
                _ => return Err("syntax error in look around".into()),
            };
            cur.jump(size);
            let p1 = parse_single(cur)?;
            Ok(vec![Rc::new(if behind {
                Ch::LookBehind(p1, positive)
            } else {
                Ch::LookAhead(p1, positive)
            })])
        }
        '【' => {
            let mut start = 1;
            let mut level = 0;
            let mut alts = Vec::new();
            for i in 1..cur.len() {
                let ch = cur.s[cur.p + i];
                if ch == '【' {
                    level += 1;
                } else if level == 0 && ch == '┇' {
                    alts.push(Rc::new(Ch::List(parse_list(
                        &cur.s[cur.p + start..cur.p + i],
                    )?)));
                    start = i + 1;
                } else if ch == '】' {
                    if level == 0 {
                        alts.push(Rc::new(Ch::List(parse_list(
                            &cur.s[cur.p + start..cur.p + i],
                        )?)));
                        cur.jump(i + 1);
                        return Ok(vec![Rc::new(Ch::Alternative(alts))]);
                    }
                    level -= 1;
                }
            }
            Err("unterminated alternative".into())
        }
        '〇' => {
            let operator = cur.at(1)?;
            cur.jump(2);
            match operator {
                '{' => {
                    let mut rep = Repetition::default();
                    let mut token = String::new();
                    loop {
                        let ch = cur.at(0)?;
                        if ch == '}' {
                            rep.add_token(&token)?;
                            cur.jump(1);
                            break;
                        } else if ch == ';' {
                            rep.add_token(&token)?;
                            token.clear();
                        } else {
                            token.push(ch);
                        }
                        cur.jump(1);
                    }
                    let origin = parse_single(cur)?;
                    Ok(vec![Rc::new(Ch::Repeat(rep, origin))])
                }
                'l' => {
                    cur.jump(1);
                    let origin = parse_single(cur)?;
                    let remaining = parse_all(cur)?;
                    let mut out = vec![Rc::new(Ch::Lazy(
                        origin,
                        Rc::new(Ch::List(remaining.clone())),
                    ))];
                    out.extend(remaining);
                    Ok(out)
                }
                '+' | '*' | '?' => {
                    let origin = parse_single(cur)?;
                    Ok(vec![Rc::new(match operator {
                        '+' => Ch::OneOrMore(origin),
                        '*' => Ch::ZeroOrMore(origin),
                        _ => Ch::Optional(origin),
                    })])
                }
                _ => Err(format!("unknown quantifier {operator}")),
            }
        }
        '〄' => {
            let operator = cur.at(1)?;
            if operator != '>' {
                return Err("unsupported up to operator".into());
            }
            cur.jump(2);
            let p2 = parse_single(cur)?;
            Ok(vec![Rc::new(Ch::UpTo(p2.clone())), p2])
        }
        '〘' => {
            let end = closing_bracket(cur, '〘', '〙').ok_or("unterminated group")?;
            let list = parse_list(&cur.s[cur.p + 1..cur.p + end])?;
            cur.jump(end + 1);
            Ok(vec![Rc::new(Ch::List(list))])
        }
        '「' => {
            let end = (0..cur.len())
                .find(|i| cur.s[cur.p + i] == '」')
                .ok_or("unterminated character set")?;
            let set = build_set(&cur.s[cur.p + 1..cur.p + end])?;
            cur.jump(end + 1);
            Ok(vec![Rc::new(set)])
        }
        '〴' => {
            cur.jump(1);
            let first = cur.at(0)?;
            let raw = ClassRaw::from_definition(first)?;
            cur.jump(raw.definition_length());
            Ok(vec![Rc::new(Ch::Class(
                ClassRawPub(raw),
                first.is_uppercase(),
            ))])
        }
        '〃' => {
            cur.jump(1);
            Ok(vec![Rc::new(Ch::Single('"'))])
        }
        ' ' => Err("no space allowed".into()),
        _ => {
            let ch = if c == '∙' { ' ' } else { c };
            cur.jump(1);
            Ok(vec![Rc::new(Ch::Single(ch.to_ascii_lowercase()))])
        }
    }
}

fn build_set(pattern: &[char]) -> Result<Ch, String> {
    if pattern.is_empty() {
        return Err("empty character set".into());
    }
    let mut classes = Vec::new();
    let mut chars = Box::new([false; 128]);
    let mut reversed = false;
    let mut add = |ch: char| {
        let ch = ch.to_ascii_lowercase();
        if (32..128).contains(&(ch as u32)) {
            chars[ch as usize] = true;
        }
    };
    let mut i = 0;
    while i < pattern.len() {
        let ch = pattern[i];
        if ch == '〤' {
            reversed = true;
        } else if ch == ' ' {
        } else if ch == '〴' {
            i += 1;
            let c = *pattern.get(i).ok_or("class expected")?;
            let raw = ClassRaw::from_definition(c)?;
            i += raw.definition_length() - 1;
            classes.push(ClassRawPub(raw));
        } else if pattern.len() - i > 2 && pattern[i + 1] == '〜' {
            let end = pattern[i + 2];
            for c in (ch as u32)..=(end as u32) {
                if let Some(c) = char::from_u32(c) {
                    add(c);
                }
            }
            i += 2;
        } else if ch == '〃' {
            add('"');
        } else if ch == '∙' {
            add(' ');
        } else {
            add(ch);
        }
        i += 1;
    }
    Ok(Ch::Set {
        classes,
        chars,
        reversed,
    })
}

/// `UBrexLeaf`: a pattern from its definition.
pub fn leaf(definition: &str) -> Ub {
    let chars: Vec<char> = definition.chars().collect();
    match parse_list(&chars) {
        Ok(list) => Rc::new(Ch::List(list)),
        Err(e) => panic!("bad ubrex definition {definition:?}: {e}"),
    }
}

/// `UBrexConcat`.
pub fn concat(parts: &[&Ub]) -> Ub {
    Rc::new(Ch::List(parts.iter().map(|p| (*p).clone()).collect()))
}

/// `UBrexOr`.
pub fn or(parts: &[&Ub]) -> Ub {
    Rc::new(Ch::Alternative(
        parts.iter().map(|p| (*p).clone()).collect(),
    ))
}

pub fn or_vec(parts: Vec<Ub>) -> Ub {
    Rc::new(Ch::Alternative(parts))
}

/// `UBrexNamed`.
pub fn named(name: &str, part: &Ub) -> Ub {
    Rc::new(Ch::Named(name.to_string(), vec![part.clone()]))
}

pub fn optional(part: &Ub) -> Ub {
    Rc::new(Ch::Optional(part.clone()))
}

pub fn one_or_more(part: &Ub) -> Ub {
    Rc::new(Ch::OneOrMore(part.clone()))
}

pub fn zero_or_more(part: &Ub) -> Ub {
    Rc::new(Ch::ZeroOrMore(part.clone()))
}

/// `UBrexUpto`: `what` repeated until `stop` matches, then `stop`.
pub fn upto(what: &Ub, stop: &Ub) -> Ub {
    Rc::new(Ch::List(vec![
        Rc::new(Ch::OneOrMoreUpTo(what.clone(), stop.clone())),
        stop.clone(),
    ]))
}

pub fn end() -> Ub {
    leaf("〒$")
}

pub fn space_one_or_more() -> Ub {
    leaf("〇+〴s")
}

pub fn space_zero_or_more() -> Ub {
    leaf("〇*〴s")
}

/// The outcome of matching a pattern at a position (`UMatcher`).
#[derive(Debug, Clone)]
pub struct UMatcher {
    len: Option<usize>,
    total: usize,
    accepted: String,
    capture: Capture,
}

#[allow(dead_code)]
impl UMatcher {
    pub fn start_match(&self) -> bool {
        self.len.is_some()
    }

    pub fn exact_match(&self) -> bool {
        self.len.is_some_and(|l| l == self.total)
    }

    pub fn accepted_match(&self) -> &str {
        &self.accepted
    }

    /// Number of characters accepted.
    pub fn accepted_len(&self) -> usize {
        self.accepted.chars().count()
    }

    pub fn find_first_value_by_key(&self, key: &str) -> Option<String> {
        self.capture.find_first_value_by_key(key)
    }

    pub fn find_values_by_key(&self, key: &str) -> Vec<String> {
        self.capture.find_values_by_key(key)
    }

    pub fn extract_by_prefix(&self, prefix: &str) -> Capture {
        self.capture.extract_by_prefix(prefix)
    }

    pub fn capture(&self) -> &Capture {
        &self.capture
    }
}

/// Matches `ub` against `text` from character `position`.
pub fn match_at(ub: &Ub, text: &[char], position: usize) -> UMatcher {
    let nav = Nav {
        chars: text,
        p1: position,
        p2: text.len(),
        reversed: false,
    };
    match ub.run(&nav, 0) {
        Some((len, capture)) => UMatcher {
            len: Some(len),
            total: nav.len(),
            accepted: nav.sub(0, len),
            capture,
        },
        None => UMatcher {
            len: None,
            total: nav.len(),
            accepted: String::new(),
            capture: Capture::default(),
        },
    }
}

/// Matches the whole string.
pub fn match_str(ub: &Ub, text: &str) -> UMatcher {
    let chars: Vec<char> = text.chars().collect();
    match_at(ub, &chars, 0)
}

/// A cursor over the text being parsed (`TextNavigator` as used by the commands).
#[derive(Debug, Clone)]
pub struct TextCursor {
    chars: Vec<char>,
    pos: usize,
}

impl TextCursor {
    pub fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
        }
    }

    pub fn jump(&mut self, n: usize) {
        self.pos += n;
    }

    pub fn match_ub(&self, ub: &Ub) -> UMatcher {
        match_at(ub, &self.chars, self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_case_insensitive() {
        let p = leaf("hello");
        assert!(match_str(&p, "HeLLo").exact_match());
        assert!(!match_str(&p, "hellO!").exact_match());
        assert!(match_str(&p, "hello!").start_match());
        assert!(!match_str(&p, "help").start_match());
    }

    #[test]
    fn repetitions_and_classes() {
        let digits = leaf("〇+〴d");
        let m = match_str(&digits, "123abc");
        assert_eq!(m.accepted_match(), "123");
        assert!(!match_str(&digits, "abc").start_match());
        assert!(!match_str(&leaf("〇{1-2}〴d"), "123").start_match());
        assert_eq!(match_str(&leaf("〇{1-2}〴d"), "12/").accepted_match(), "12");
        assert_eq!(
            match_str(&leaf("colo〇?ured"), "colored").accepted_match(),
            "colored"
        );
        assert!(match_str(&leaf("colo〇?ured"), "coloured").exact_match());
        assert_eq!(match_str(&leaf("〇*〴s"), "x").accepted_match(), "");
        assert!(match_str(&leaf("【is┇are】"), "ARE").exact_match());
        assert!(match_str(&leaf("「dD」+"), "d+").exact_match());
        assert!(match_str(&leaf("a∙b"), "a b").exact_match());
    }

    #[test]
    fn sets_and_names() {
        let code = concat(&[&leaf("["), &named("N", &leaf("〇+「〤[]」")), &leaf("]")]);
        let m = match_str(&code, "[Prototype design] lasts");
        assert_eq!(m.find_first_value_by_key("N").unwrap(), "Prototype design");
        assert_eq!(m.accepted_match(), "[Prototype design]");
        let nested = named("OUT", &code);
        let m = match_str(&nested, "[a]");
        assert_eq!(m.find_first_value_by_key("OUT").unwrap(), "[a]");
        assert_eq!(m.find_first_value_by_key("OUT/N").unwrap(), "a");
        assert_eq!(
            m.extract_by_prefix("OUT")
                .find_first_value_by_key("N")
                .unwrap(),
            "a"
        );
        assert_eq!(match_str(&leaf("「a〜c」"), "B").accepted_len(), 1);
    }

    #[test]
    fn up_to_and_lookaround() {
        let u = leaf("[[〄>]]");
        assert!(match_str(&u, "[[http://x{tip} label]]").exact_match());
        let e = concat(&[&leaf("ab"), &end()]);
        assert!(match_str(&e, "ab").exact_match());
        assert!(!match_str(&e, "abc").start_match());
        let la = leaf("〒!x a");
        assert!(match_str(&la, "a").start_match());
        assert!(!match_str(&la, "xa").start_match());
        let upto_p = upto(&leaf("〴."), &leaf("z"));
        assert_eq!(match_str(&upto_p, "abzcd").accepted_match(), "abz");
        let lb = leaf("a 〒<=a b");
        assert!(match_str(&lb, "ab").start_match());
        let lazy = leaf("〇l 〴. z");
        assert!(match_str(&lazy, "abz").start_match());
    }

    #[test]
    fn cursor_walks_the_text() {
        let mut c = TextCursor::new("  hello world");
        let m = c.match_ub(&space_one_or_more());
        c.jump(m.accepted_len());
        assert_eq!(c.match_ub(&leaf("hello")).accepted_match(), "hello");
        assert_eq!(or_vec(vec![leaf("x"), leaf("h")]).run_len(), ());
    }

    trait RunLen {
        fn run_len(&self);
    }
    impl RunLen for Ub {
        fn run_len(&self) {}
    }

    #[test]
    fn repetition_tokens() {
        let mut r = Repetition::default();
        r.add_token("2+").unwrap();
        r.add_token("1").unwrap();
        assert!(r.matches(1) && r.matches(5) && !r.matches(0));
        assert!(r.add_token("").is_err());
        assert!(parse_list(&[]).is_err());
        assert!(zero_or_more(&leaf("a"))
            .run(
                &Nav {
                    chars: &['a', 'a'],
                    p1: 0,
                    p2: 2,
                    reversed: false
                },
                0
            )
            .is_some());
        assert!(optional(&one_or_more(&leaf("b")))
            .run(
                &Nav {
                    chars: &['a'],
                    p1: 0,
                    p2: 1,
                    reversed: false
                },
                0
            )
            .is_some());
        assert!(or(&[&leaf("q")])
            .run(
                &Nav {
                    chars: &['q'],
                    p1: 0,
                    p2: 1,
                    reversed: false
                },
                0
            )
            .is_some());
    }
}
