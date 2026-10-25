//! Port of the minimal-json library bundled with PlantUML
//! (`net.sourceforge.plantuml.json`).
//!
//! The parser mirrors the Java one: whitespace is `space`, `\t`, `\n`, `\r`;
//! `//` comments are accepted between object members; nesting is limited to
//! 1000 levels; numbers keep their raw text. Error offsets and columns count
//! UTF-16 code units like the Java parser does. Strings containing lone
//! surrogates are decoded lossily (U+FFFD).
//!
//! Java overloads that differ only by argument type are split by key type:
//! [`JsonValue::get`], [`JsonValue::set`] and [`JsonValue::remove`] take a
//! `&str` member name on objects or a `usize` index on arrays, and
//! `JsonValue::add` (object member) is `push` for arrays.

use std::fmt::{self, Write};

const MAX_NESTING_LEVEL: usize = 1000;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    True,
    False,
    /// Raw number text as parsed, e.g. `1e3` or `-0.5`.
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    /// Members in insertion order. Names may repeat.
    Object(Vec<(String, JsonValue)>),
}

/// A parse failure. `offset` is the UTF-16 index of the offending position,
/// `line` and `column` are 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.line, self.column)
    }
}

impl std::error::Error for ParseError {}

/// Key used to address a member of an object (`&str`) or an element of an
/// array (`usize`).
pub trait JsonKey {
    /// Looks the key up; `None` if the value has no such member or element.
    fn get_in<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue>;
    /// Mutable variant of [`JsonKey::get_in`].
    fn get_in_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue>;
    /// Replaces the last matching member, or appends it when absent (objects);
    /// replaces the element (arrays). Panics on a wrong type or bad index.
    fn set_in(&self, value: &mut JsonValue, new: JsonValue);
    /// Removes the last matching member (no-op when absent); removes the
    /// element for arrays. Panics on a wrong type or bad index.
    fn remove_in(&self, value: &mut JsonValue);
}

impl JsonKey for &str {
    fn get_in<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(members) => members
                .iter()
                .rev()
                .find(|(n, _)| n.as_str() == *self)
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn get_in_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        match value {
            JsonValue::Object(members) => members
                .iter_mut()
                .rev()
                .find(|(n, _)| n.as_str() == *self)
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn set_in(&self, value: &mut JsonValue, new: JsonValue) {
        let members = object_mut(value, "set");
        match members.iter().rposition(|(n, _)| n.as_str() == *self) {
            Some(i) => members[i].1 = new,
            None => members.push((self.to_string(), new)),
        }
    }

    fn remove_in(&self, value: &mut JsonValue) {
        let members = object_mut(value, "remove");
        if let Some(i) = members.iter().rposition(|(n, _)| n.as_str() == *self) {
            members.remove(i);
        }
    }
}

impl JsonKey for usize {
    fn get_in<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Array(items) => items.get(*self),
            _ => None,
        }
    }

    fn get_in_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        match value {
            JsonValue::Array(items) => items.get_mut(*self),
            _ => None,
        }
    }

    fn set_in(&self, value: &mut JsonValue, new: JsonValue) {
        array_mut(value, "set")[*self] = new;
    }

    fn remove_in(&self, value: &mut JsonValue) {
        array_mut(value, "remove").remove(*self);
    }
}

fn object_mut<'a>(value: &'a mut JsonValue, op: &str) -> &'a mut Vec<(String, JsonValue)> {
    match value {
        JsonValue::Object(members) => members,
        _ => panic!("{op} requires a JSON object"),
    }
}

fn array_mut<'a>(value: &'a mut JsonValue, op: &str) -> &'a mut Vec<JsonValue> {
    match value {
        JsonValue::Array(items) => items,
        _ => panic!("{op} requires a JSON array"),
    }
}

impl JsonValue {
    /// Parses a complete JSON text. Leading and trailing whitespace is allowed;
    /// any other trailing character is an error.
    pub fn parse(s: &str) -> Result<JsonValue, ParseError> {
        let mut parser = Parser {
            src: s.encode_utf16().collect(),
            pos: 0,
            depth: 0,
        };
        parser.document()
    }

    /// A number value from an `i32`.
    pub fn from_int(value: i32) -> JsonValue {
        JsonValue::Number(value.to_string())
    }

    /// A string value. Named after the Java factory; not `FromStr`, which
    /// would parse JSON text instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> JsonValue {
        JsonValue::String(value.to_string())
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JsonValue::Object(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JsonValue::Array(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, JsonValue::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    /// `true` for both `true` and `false`.
    pub fn is_boolean(&self) -> bool {
        matches!(self, JsonValue::True | JsonValue::False)
    }

    /// The text of a string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Looks up a member by name (objects; the last match wins) or an element
    /// by index (arrays).
    pub fn get<K: JsonKey>(&self, key: K) -> Option<&JsonValue> {
        key.get_in(self)
    }

    /// Replaces the last member with this name or appends it (objects);
    /// replaces the element at this index (arrays).
    pub fn set<K: JsonKey>(&mut self, key: K, value: JsonValue) {
        key.set_in(self, value)
    }

    /// Removes the last member with this name, if any (objects); removes the
    /// element at this index (arrays).
    pub fn remove<K: JsonKey>(&mut self, key: K) {
        key.remove_in(self)
    }

    /// Appends a member; duplicate names are kept. Objects only.
    pub fn add(&mut self, name: &str, value: JsonValue) {
        object_mut(self, "add").push((name.to_string(), value));
    }

    /// Appends an element. Arrays only.
    pub fn push(&mut self, value: JsonValue) {
        array_mut(self, "push").push(value);
    }

    /// Member names in insertion order, duplicates included. Empty for
    /// non-objects.
    pub fn names(&self) -> Vec<&str> {
        match self {
            JsonValue::Object(members) => members.iter().map(|(n, _)| n.as_str()).collect(),
            _ => Vec::new(),
        }
    }

    /// Whether an object has a member with this name.
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Member count for objects, element count for arrays, 0 otherwise.
    pub fn len(&self) -> usize {
        match self {
            JsonValue::Object(members) => members.len(),
            JsonValue::Array(items) => items.len(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets every member of `other` on this object, in order (Java
    /// `JsonObject.merge`). Panics if either side is not an object.
    pub fn merge(&mut self, other: &JsonValue) {
        let JsonValue::Object(source) = other else {
            panic!("merge requires a JSON object argument");
        };
        for (name, value) in source {
            self.set(name.as_str(), value.clone());
        }
    }

    /// Like [`JsonValue::merge`], but object members present on both sides
    /// are merged recursively (Java `JsonObject.deepMerge`). Panics if either
    /// side is not an object.
    pub fn deep_merge(&mut self, other: &JsonValue) {
        let JsonValue::Object(source) = other else {
            panic!("deep_merge requires a JSON object argument");
        };
        for (name, value) in source {
            if value.is_object() {
                let key = name.as_str();
                if let Some(existing) = key.get_in_mut(self) {
                    if existing.is_object() {
                        existing.deep_merge(value);
                        continue;
                    }
                }
            }
            self.set(name.as_str(), value.clone());
        }
    }
}

impl fmt::Display for JsonValue {
    /// Compact JSON text, identical to minimal-json `toString()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(self, f)
    }
}

fn write_value<W: Write>(value: &JsonValue, out: &mut W) -> fmt::Result {
    match value {
        JsonValue::Null => out.write_str("null"),
        JsonValue::True => out.write_str("true"),
        JsonValue::False => out.write_str("false"),
        JsonValue::Number(raw) => out.write_str(raw),
        JsonValue::String(s) => write_string(s, out),
        JsonValue::Array(items) => {
            out.write_char('[')?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.write_char(',')?;
                }
                write_value(item, out)?;
            }
            out.write_char(']')
        }
        JsonValue::Object(members) => {
            out.write_char('{')?;
            for (i, (name, item)) in members.iter().enumerate() {
                if i > 0 {
                    out.write_char(',')?;
                }
                write_string(name, out)?;
                out.write_char(':')?;
                write_value(item, out)?;
            }
            out.write_char('}')
        }
    }
}

fn write_string<W: Write>(s: &str, out: &mut W) -> fmt::Result {
    out.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            '\u{2028}' => out.write_str("\\u2028")?,
            '\u{2029}' => out.write_str("\\u2029")?,
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32)?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

struct Parser {
    src: Vec<u16>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn document(&mut self) -> Result<JsonValue, ParseError> {
        self.skip_ws();
        let value = self.value()?;
        self.skip_ws();
        if self.pos < self.src.len() {
            return Err(self.error("Unexpected character"));
        }
        Ok(value)
    }

    /// The current unit if it is ASCII; any non-ASCII unit maps to `0xFF`,
    /// which matches no structural character.
    fn peek(&self) -> Option<u8> {
        self.src
            .get(self.pos)
            .map(|&c| if c < 0x80 { c as u8 } else { 0xFF })
    }

    fn digit(&self) -> bool {
        matches!(self.peek(), Some(b'0'..=b'9'))
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<JsonValue, ParseError> {
        match self.peek() {
            Some(b'n') => {
                self.literal("null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.literal("true")?;
                Ok(JsonValue::True)
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(JsonValue::False)
            }
            Some(b'"') => Ok(JsonValue::String(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.expected("value")),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), ParseError> {
        // The first byte was already matched by `value`.
        self.pos += 1;
        for byte in word.bytes().skip(1) {
            if !self.eat(byte) {
                return Err(self.expected(&format!("'{}'", byte as char)));
            }
        }
        Ok(())
    }

    fn number(&mut self) -> Result<JsonValue, ParseError> {
        let start = self.pos;
        self.eat(b'-');
        let first = self.peek();
        if !self.digit() {
            return Err(self.expected("digit"));
        }
        self.pos += 1;
        if first != Some(b'0') {
            while self.digit() {
                self.pos += 1;
            }
        }
        if self.eat(b'.') {
            if !self.digit() {
                return Err(self.expected("digit"));
            }
            while self.digit() {
                self.pos += 1;
            }
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if !self.digit() {
                return Err(self.expected("digit"));
            }
            while self.digit() {
                self.pos += 1;
            }
        }
        let text: String = self.src[start..self.pos]
            .iter()
            .map(|&c| c as u8 as char)
            .collect();
        Ok(JsonValue::Number(text))
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.pos += 1; // opening quote
        let mut out: Vec<u16> = Vec::new();
        loop {
            match self.src.get(self.pos).copied() {
                None => return Err(self.expected("valid string character")),
                Some(0x22) => {
                    self.pos += 1;
                    break;
                }
                Some(0x5c) => {
                    self.pos += 1;
                    self.escape(&mut out)?;
                }
                Some(c) if c < 0x20 => return Err(self.expected("valid string character")),
                Some(c) => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
        Ok(String::from_utf16_lossy(&out))
    }

    /// Called with `pos` on the character after a backslash.
    fn escape(&mut self, out: &mut Vec<u16>) -> Result<(), ParseError> {
        let unit: u16 = match self.peek() {
            Some(b @ (b'"' | b'/' | b'\\')) => u16::from(b),
            Some(b'b') => 0x08,
            Some(b'f') => 0x0c,
            Some(b'n') => 0x0a,
            Some(b'r') => 0x0d,
            Some(b't') => 0x09,
            Some(b'u') => {
                let mut code: u16 = 0;
                for _ in 0..4 {
                    self.pos += 1;
                    let digit = self
                        .peek()
                        .and_then(hex_value)
                        .ok_or_else(|| self.expected("hexadecimal digit"))?;
                    code = code * 16 + u16::from(digit);
                }
                code
            }
            _ => return Err(self.expected("valid escape sequence")),
        };
        out.push(unit);
        self.pos += 1;
        Ok(())
    }

    fn array(&mut self) -> Result<JsonValue, ParseError> {
        self.pos += 1;
        self.enter()?;
        let mut items = Vec::new();
        self.skip_ws();
        if !self.eat(b']') {
            loop {
                self.skip_ws();
                items.push(self.value()?);
                self.skip_ws();
                if !self.eat(b',') {
                    break;
                }
            }
            if !self.eat(b']') {
                return Err(self.expected("',' or ']'"));
            }
        }
        self.depth -= 1;
        Ok(JsonValue::Array(items))
    }

    fn object(&mut self) -> Result<JsonValue, ParseError> {
        self.pos += 1;
        self.enter()?;
        let mut members = Vec::new();
        self.skip_ws();
        if !self.eat(b'}') {
            loop {
                self.skip_ws();
                if self.eat(b'/') {
                    self.comment()?;
                }
                if self.peek() != Some(b'"') {
                    return Err(self.expected("name"));
                }
                let name = self.string()?;
                self.skip_ws();
                if !self.eat(b':') {
                    return Err(self.expected("':'"));
                }
                self.skip_ws();
                let value = self.value()?;
                members.push((name, value));
                self.skip_ws();
                if self.eat(b'/') {
                    self.comment()?;
                }
                if !self.eat(b',') {
                    break;
                }
            }
            if !self.eat(b'}') {
                return Err(self.expected("',' or '}'"));
            }
        }
        self.depth -= 1;
        Ok(JsonValue::Object(members))
    }

    /// Called after the first `/` of a comment has been consumed.
    fn comment(&mut self) -> Result<(), ParseError> {
        if !self.eat(b'/') {
            return Err(self.expected("Error in comment"));
        }
        while self.pos < self.src.len() && !matches!(self.peek(), Some(b'\n' | b'\r')) {
            self.pos += 1;
        }
        self.skip_ws();
        Ok(())
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_LEVEL {
            return Err(self.error("Nesting too deep"));
        }
        Ok(())
    }

    fn expected(&self, what: &str) -> ParseError {
        if self.pos >= self.src.len() {
            self.error("Unexpected end of input")
        } else {
            self.error(&format!("Expected {what}"))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        let (line, column) = self.line_column();
        ParseError {
            message: message.to_string(),
            offset: self.pos,
            line,
            column,
        }
    }

    /// Line and column of `pos`, reproducing the Java parser: a newline that
    /// is the last unit of the input does not start a new line.
    fn line_column(&self) -> (usize, usize) {
        let len = self.src.len();
        let end = if self.pos >= len {
            len.saturating_sub(1)
        } else {
            self.pos
        };
        let mut line = 1;
        let mut line_start = 0;
        for (i, &c) in self.src[..end].iter().enumerate() {
            if c == u16::from(b'\n') {
                line += 1;
                line_start = i + 1;
            }
        }
        (line, self.pos - line_start + 1)
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> JsonValue {
        JsonValue::parse(s).unwrap()
    }

    fn err(s: &str) -> ParseError {
        JsonValue::parse(s).unwrap_err()
    }

    fn assert_err(s: &str, message: &str, offset: usize, line: usize, column: usize) {
        let e = err(s);
        assert_eq!(
            (e.message.as_str(), e.offset, e.line, e.column),
            (message, offset, line, column),
            "input {s:?}"
        );
    }

    fn num(s: &str) -> JsonValue {
        JsonValue::Number(s.to_string())
    }

    #[test]
    fn json_round_trip_compact() {
        let s = r#"{"a":[1,-0.5,1e3,true,false,null,"x"],"b":{},"c":[],"d":{"k":[{"a":null},[true,false]]}}"#;
        assert_eq!(ok(s).to_string(), s);
        assert_eq!(ok(&ok(s).to_string()), ok(s));
    }

    #[test]
    fn json_parse_strips_whitespace() {
        let v = ok(" \t\r\n{ \"a\" : [ 1 , 2 ] }\n ");
        assert_eq!(v.to_string(), r#"{"a":[1,2]}"#);
    }

    #[test]
    fn json_numbers_kept_raw() {
        for s in [
            "0",
            "-0",
            "1e3",
            "1E+2",
            "-0.5",
            "12.50e-3",
            "123456789012345678901234567890",
        ] {
            assert_eq!(ok(s), num(s));
            assert_eq!(ok(s).to_string(), s);
        }
    }

    #[test]
    fn json_scalars() {
        assert_eq!(ok("null"), JsonValue::Null);
        assert_eq!(ok("true"), JsonValue::True);
        assert_eq!(ok("false"), JsonValue::False);
        assert_eq!(ok("\"x\""), JsonValue::String("x".into()));
        assert_eq!(ok("  7  "), num("7"));
    }

    #[test]
    fn json_type_predicates_and_as_str() {
        assert!(ok("true").is_boolean());
        assert!(ok("false").is_boolean());
        assert!(!ok("null").is_boolean());
        assert!(ok("1").is_number());
        assert!(ok("\"s\"").is_string());
        assert!(ok("[]").is_array());
        assert!(ok("{}").is_object());
        assert_eq!(ok("\"s\"").as_str(), Some("s"));
        assert_eq!(ok("1").as_str(), None);
    }

    #[test]
    fn json_leading_zero_rejected() {
        assert_err("01", "Unexpected character", 1, 1, 2);
        assert_err("[01]", "Expected ',' or ']'", 2, 1, 3);
    }

    #[test]
    fn json_number_errors() {
        assert_err("-", "Unexpected end of input", 1, 1, 2);
        assert_err("-x", "Expected digit", 1, 1, 2);
        assert_err("1.", "Unexpected end of input", 2, 1, 3);
        assert_err("1.x", "Expected digit", 2, 1, 3);
        assert_err("1e", "Unexpected end of input", 2, 1, 3);
        assert_err("1e+", "Unexpected end of input", 3, 1, 4);
    }

    #[test]
    fn json_literal_errors() {
        assert_err("tru", "Unexpected end of input", 3, 1, 4);
        assert_err("trux", "Expected 'e'", 3, 1, 4);
        assert_err("nul1", "Expected 'l'", 3, 1, 4);
        assert_err("nope", "Expected 'u'", 1, 1, 2);
    }

    #[test]
    fn json_string_escapes_parse() {
        let v = ok(r#""a\"b\\c\/d\n\r\t\b\f\u0041\u00e9""#);
        assert_eq!(v, JsonValue::String("a\"b\\c/d\n\r\t\u{8}\u{c}Aé".into()));
        assert_eq!(v.to_string(), "\"a\\\"b\\\\c/d\\n\\r\\t\\u0008\\u000cAé\"");
    }

    #[test]
    fn json_string_escapes_output() {
        let v = JsonValue::String("q\"b\\n\n\r\t\u{1}\u{7f}\u{2028}\u{2029}/é".into());
        assert_eq!(
            v.to_string(),
            "\"q\\\"b\\\\n\\n\\r\\t\\u0001\u{7f}\\u2028\\u2029/é\""
        );
        assert_eq!(ok(&v.to_string()), v);
    }

    #[test]
    fn json_string_errors() {
        assert_err("\"abc", "Unexpected end of input", 4, 1, 5);
        assert_err("\"a\u{1}\"", "Expected valid string character", 2, 1, 3);
        assert_err("\"\\x\"", "Expected valid escape sequence", 2, 1, 3);
        assert_err("\"\\", "Unexpected end of input", 2, 1, 3);
        assert_err("\"\\u12G4\"", "Expected hexadecimal digit", 5, 1, 6);
        assert_err("\"\\u12", "Unexpected end of input", 5, 1, 6);
    }

    #[test]
    fn json_lone_surrogate_is_lossy() {
        assert_eq!(ok("\"\\ud800\""), JsonValue::String("\u{fffd}".into()));
    }

    #[test]
    fn json_offsets_count_utf16_units() {
        // The emoji is two UTF-16 units, so the stray 'x' sits at offset 5.
        assert_err("\"😀\" x", "Unexpected character", 5, 1, 6);
    }

    #[test]
    fn json_error_locations_multiline() {
        assert_err("{\n  \"a\": 1,\n  x\n}", "Expected name", 14, 3, 3);
    }

    #[test]
    fn json_eof_after_trailing_newline_matches_java() {
        // Java does not count a newline that is the last input unit.
        assert_err("{\n", "Unexpected end of input", 2, 1, 3);
    }

    #[test]
    fn json_trailing_garbage_rejected() {
        assert_err("{} x", "Unexpected character", 3, 1, 4);
        assert_err("1 2", "Unexpected character", 2, 1, 3);
        assert_err("\"a\" \"b\"", "Unexpected character", 4, 1, 5);
        assert_err("[1]]", "Unexpected character", 3, 1, 4);
    }

    #[test]
    fn json_empty_input_rejected() {
        assert_err("", "Unexpected end of input", 0, 1, 1);
        assert_err("  ", "Unexpected end of input", 2, 1, 3);
    }

    #[test]
    fn json_structure_errors() {
        assert_err("[1,]", "Expected value", 3, 1, 4);
        assert_err("[1 2]", "Expected ',' or ']'", 3, 1, 4);
        assert_err("[1", "Unexpected end of input", 2, 1, 3);
        assert_err("{\"a\" 1}", "Expected ':'", 5, 1, 6);
        assert_err("{\"a\"", "Unexpected end of input", 4, 1, 5);
        assert_err("{\"a\":1,}", "Expected name", 7, 1, 8);
        assert_err("{\"a\":1 \"b\":2}", "Expected ',' or '}'", 7, 1, 8);
    }

    #[test]
    fn json_comments_in_objects() {
        assert_eq!(
            ok("{\"a\":1 // one\n,\"b\":2}").to_string(),
            r#"{"a":1,"b":2}"#
        );
        assert_eq!(ok("{ // head\n \"a\":1}").to_string(), r#"{"a":1}"#);
        assert_err("{\"a\":1 /x}", "Expected Error in comment", 8, 1, 9);
        // Comments are not allowed in arrays.
        assert_err("[1 // no\n]", "Expected ',' or ']'", 3, 1, 4);
    }

    #[test]
    fn json_nesting_limit() {
        let deep = format!("{}{}", "[".repeat(1000), "]".repeat(1000));
        assert!(JsonValue::parse(&deep).is_ok());
        assert_err(&"[".repeat(1001), "Nesting too deep", 1001, 1, 1002);
    }

    #[test]
    fn json_parse_error_display() {
        assert_eq!(err("[1 2]").to_string(), "Expected ',' or ']' at 1:4");
    }

    #[test]
    fn json_duplicates_kept() {
        let v = ok(r#"{"a":1,"b":2,"a":3}"#);
        assert_eq!(v.names(), vec!["a", "b", "a"]);
        assert_eq!(v.get("a"), Some(&num("3")));
        assert_eq!(v.len(), 3);
        assert_eq!(v.to_string(), r#"{"a":1,"b":2,"a":3}"#);
    }

    #[test]
    fn json_object_get_set_remove() {
        let mut v = ok(r#"{"a":1,"a":2}"#);
        v.set("a", JsonValue::from_int(9));
        assert_eq!(v.to_string(), r#"{"a":1,"a":9}"#);
        v.set("c", JsonValue::from_int(4));
        assert_eq!(v.to_string(), r#"{"a":1,"a":9,"c":4}"#);
        v.remove("a");
        assert_eq!(v.to_string(), r#"{"a":1,"c":4}"#);
        v.remove("missing");
        assert_eq!(v.to_string(), r#"{"a":1,"c":4}"#);
        assert_eq!(v.get("missing"), None);
        assert!(v.contains("c"));
        assert!(!v.contains("missing"));
        assert_eq!(v.get("a"), Some(&num("1")));
    }

    #[test]
    fn json_add_appends_duplicates() {
        let mut v = JsonValue::Object(Vec::new());
        v.add("x", JsonValue::from_int(1));
        v.add("x", JsonValue::from_str("y"));
        assert_eq!(v.to_string(), r#"{"x":1,"x":"y"}"#);
        assert_eq!(v.len(), 2);
        assert!(!v.is_empty());
    }

    #[test]
    fn json_array_ops() {
        let mut a = ok("[1,2,3]");
        assert_eq!(a.len(), 3);
        assert_eq!(a.get(1), Some(&num("2")));
        assert_eq!(a.get(9), None);
        assert_eq!(a.get("k"), None);
        a.set(0, JsonValue::from_str("x"));
        a.push(JsonValue::from_int(4));
        a.remove(1);
        assert_eq!(a.to_string(), r#"["x",3,4]"#);
    }

    #[test]
    fn json_from_helpers() {
        assert_eq!(JsonValue::from_int(-42).to_string(), "-42");
        assert_eq!(JsonValue::from_str("a\"b").to_string(), r#""a\"b""#);
    }

    #[test]
    fn json_merge_replaces_members() {
        let mut a = ok(r#"{"a":1,"b":{"x":1}}"#);
        a.merge(&ok(r#"{"b":{"y":2},"c":3,"a":5}"#));
        assert_eq!(a.to_string(), r#"{"a":5,"b":{"y":2},"c":3}"#);
    }

    #[test]
    fn json_deep_merge_recurses_into_objects() {
        let mut a = ok(r#"{"b":{"x":1,"z":{"k":1}},"s":1}"#);
        a.deep_merge(&ok(r#"{"b":{"y":2,"z":{"j":2}},"s":{"n":1},"c":3}"#));
        assert_eq!(
            a.to_string(),
            r#"{"b":{"x":1,"z":{"k":1,"j":2},"y":2},"s":{"n":1},"c":3}"#
        );
    }

    #[test]
    #[should_panic(expected = "add requires a JSON object")]
    fn json_add_on_array_panics() {
        ok("[]").add("a", JsonValue::Null);
    }

    #[test]
    #[should_panic(expected = "merge requires a JSON object argument")]
    fn json_merge_non_object_panics() {
        ok("{}").merge(&ok("[]"));
    }
}
