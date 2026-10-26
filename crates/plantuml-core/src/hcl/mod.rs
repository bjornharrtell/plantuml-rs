//! HCL data diagrams (`hcl` package); drawn by [`crate::jsondiagram`].

use crate::json::JsonValue;

pub use crate::jsondiagram::HclDiagramFactory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Symbol {
    StringSimple,
    StringQuoted,
    FunctionName,
    SquareOpen,
    SquareClose,
    CurlyOpen,
    CurlyClose,
    ParenOpen,
    ParenClose,
    TwoPoints,
    Equals,
    Comma,
    Space,
}

#[derive(Debug, Clone)]
struct Term {
    symbol: Symbol,
    data: String,
}

type Res<T> = Result<T, String>;

/// `Character.isSpaceChar`: space separators only, not tabs or newlines.
fn is_space_char(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn symbol_of(c: char) -> Option<Symbol> {
    Some(match c {
        c if is_space_char(c) => Symbol::Space,
        '{' => Symbol::CurlyOpen,
        '}' => Symbol::CurlyClose,
        '[' => Symbol::SquareOpen,
        ']' => Symbol::SquareClose,
        '(' => Symbol::ParenOpen,
        ')' => Symbol::ParenClose,
        '=' => Symbol::Equals,
        ',' => Symbol::Comma,
        ':' => Symbol::TwoPoints,
        _ => return None,
    })
}

fn tokenize(source: &[String]) -> Res<Vec<Term>> {
    let mut chars = source
        .iter()
        .filter(|l| !l.trim().starts_with('#'))
        .flat_map(|l| l.chars())
        .peekable();
    let mut terms = Vec::new();
    let mut pending = String::new();
    let term = |symbol, data: &str| Term {
        symbol,
        data: data.to_string(),
    };
    while let Some(c) = chars.next() {
        let symbol = symbol_of(c);
        if symbol == Some(Symbol::ParenOpen) {
            if pending.is_empty() {
                return Err("function without a name".into());
            }
            terms.push(term(Symbol::FunctionName, &pending));
            pending.clear();
        } else if symbol.is_some() && !pending.is_empty() {
            terms.push(term(Symbol::StringSimple, &pending));
            pending.clear();
        }
        if symbol == Some(Symbol::Space) {
            continue;
        }
        if let Some(s) = symbol {
            terms.push(term(s, ""));
            continue;
        }
        if c == '"' {
            let mut sb = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    sb.extend(chars.next());
                    continue;
                }
                if c == '"' {
                    break;
                }
                sb.push(c);
            }
            terms.push(term(Symbol::StringQuoted, &sb));
            continue;
        }
        pending.push(c);
    }
    Ok(terms)
}

/// A value read after `=`: a JSON value, or a closing token for the caller.
enum Val {
    Term(Symbol),
    Json(JsonValue),
}

struct Parser {
    terms: std::vec::IntoIter<Term>,
}

impl Parser {
    fn next(&mut self) -> Res<Term> {
        self.terms
            .next()
            .ok_or_else(|| "unexpected end".to_string())
    }

    fn module_or_something(&mut self) -> Res<(String, JsonValue)> {
        let mut name = String::new();
        loop {
            let t = self.next()?;
            match t.symbol {
                Symbol::StringQuoted => name.push_str(&format!("\"{}\" ", t.data)),
                Symbol::StringSimple => name.push_str(&format!("{} ", t.data)),
                Symbol::CurlyOpen => return Ok((name.trim().to_string(), self.bracket_data()?)),
                s => return Err(format!("{s:?}")),
            }
        }
    }

    fn function_data(&mut self, name: &str) -> Res<JsonValue> {
        if self.next()?.symbol != Symbol::ParenOpen {
            return Err("( expected".into());
        }
        let mut args = Vec::new();
        loop {
            match self.value()? {
                Val::Term(Symbol::ParenClose) => {
                    if args.is_empty() {
                        return Ok(JsonValue::String(format!("{name}()")));
                    }
                    return Ok(JsonValue::Object(vec![(
                        format!("{name}()"),
                        JsonValue::Array(args),
                    )]));
                }
                Val::Term(Symbol::Comma) => {}
                Val::Term(s) => return Err(format!("{s:?}")),
                Val::Json(v) => args.push(v),
            }
        }
    }

    fn bracket_data(&mut self) -> Res<JsonValue> {
        let mut result = Vec::new();
        loop {
            let current = self.next()?;
            match current.symbol {
                Symbol::CurlyClose => return Ok(JsonValue::Object(result)),
                Symbol::StringSimple | Symbol::StringQuoted => {
                    let next = self.next()?;
                    if !matches!(next.symbol, Symbol::Equals | Symbol::TwoPoints) {
                        return Err(current.data);
                    }
                    match self.value()? {
                        Val::Json(v) => result.push((current.data, v)),
                        Val::Term(s) => return Err(format!("{s:?}")),
                    }
                }
                s => return Err(format!("{s:?}")),
            }
        }
    }

    fn value(&mut self) -> Res<Val> {
        let current = self.next()?;
        Ok(match current.symbol {
            Symbol::Comma | Symbol::ParenClose => Val::Term(current.symbol),
            Symbol::StringQuoted | Symbol::StringSimple => {
                Val::Json(JsonValue::String(current.data))
            }
            Symbol::SquareOpen => Val::Json(self.array()?),
            Symbol::CurlyOpen => Val::Json(self.bracket_data()?),
            Symbol::FunctionName => Val::Json(self.function_data(&current.data)?),
            s => return Err(format!("{s:?}")),
        })
    }

    fn array(&mut self) -> Res<JsonValue> {
        let mut result = Vec::new();
        loop {
            let current = self.next()?;
            match current.symbol {
                Symbol::CurlyOpen => result.push(self.bracket_data()?),
                Symbol::SquareClose => return Ok(JsonValue::Array(result)),
                Symbol::StringQuoted => result.push(JsonValue::String(current.data)),
                _ => {}
            }
        }
    }

    fn parse_me(mut self) -> Res<JsonValue> {
        let mut map: Vec<(String, JsonValue)> = Vec::new();
        while !self.terms.as_slice().is_empty() {
            let (name, value) = self.module_or_something()?;
            match map.iter_mut().find(|(k, _)| *k == name) {
                Some(e) => e.1 = value,
                None => map.push((name, value)),
            }
        }
        if map.len() == 1 {
            return Ok(map.remove(0).1);
        }
        Ok(JsonValue::Object(map))
    }
}

/// Parses HCL source lines into JSON (`HclParser.parseMe`).
pub fn parse_hcl(lines: &[String]) -> Result<Option<JsonValue>, String> {
    let terms = tokenize(lines)?;
    Parser {
        terms: terms.into_iter(),
    }
    .parse_me()
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Res<String> {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        parse_hcl(&lines).map(|v| v.unwrap().to_string())
    }

    #[test]
    fn module_with_values() {
        assert_eq!(
            parse(
                "# comment\nmodule \"vpc\" {\n  source = \"x\\\"y\"\n  n = three\n  list = [\"a\", \"b\", {k = \"v\"}]\n  f = max(\"a\", [\"b\"])\n  g = now()\n  o = { z : \"1\" }\n}\n"
            )
            .unwrap(),
            r#"{"source":"x\"y","n":"three","list":["a","b",{"k":"v"}],"f":{"max()":["a",["b"]]},"g":"now()","o":{"z":"1"}}"#
        );
    }

    #[test]
    fn several_modules() {
        assert_eq!(
            parse("a {\nx = \"1\"\n}\nb \"c\" {\n}\na {\nx = \"2\"\n}\n").unwrap(),
            r#"{"a":{"x":"2"},"b \"c\"":{}}"#
        );
        assert_eq!(parse("").unwrap(), "{}");
    }

    #[test]
    fn errors() {
        assert!(parse("a {").is_err());
        assert!(parse("a { x }").is_err());
        assert!(parse("a { x = } }").is_err());
        assert!(parse("a { x = ) }").is_err());
        assert!(parse("( x").is_err());
        assert!(parse("a = 1").is_err());
        assert!(parse("a { x = f(g").is_err());
        assert!(parse("a { x = f(=) }").is_err());
        assert!(parse("a { = }").is_err());
    }

    #[test]
    fn spaces() {
        assert!(is_space_char('\u{a0}'));
        assert!(!is_space_char('\t'));
    }
}
