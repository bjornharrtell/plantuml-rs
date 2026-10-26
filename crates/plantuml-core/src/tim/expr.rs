//! Expressions of the preprocessor: values, tokens, shunting-yard and RPN evaluation
//! (`TValue`, `Token`, `TokenType`, `TokenOperator`, `TokenStack`, `ShuntingYard`,
//! `ReversePolishInterpretor`).

use super::context::TContext;
use super::eater::Eater;
use super::functions::{TFunction, TFunctionSignature};
use super::line_type::TLineType;
use super::memory::TMemory;
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::text::StringLocated;
use std::collections::HashMap;
use std::rc::Rc;

/// Value of a variable or expression: an integer, a string or JSON data.
#[derive(Debug, Clone, PartialEq)]
pub enum TValue {
    Int(i32),
    Str(String),
    Json(JsonValue),
}

impl std::fmt::Display for TValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TValue::Json(j) => match j.as_str() {
                Some(s) => write!(f, "{s}"),
                None => write!(f, "{j}"),
            },
            TValue::Str(s) => write!(f, "{s}"),
            TValue::Int(i) => write!(f, "{i}"),
        }
    }
}

impl TValue {
    pub fn from_bool(b: bool) -> TValue {
        TValue::Int(i32::from(b))
    }

    pub fn text(s: impl Into<String>) -> TValue {
        TValue::Str(s.into())
    }

    pub fn is_number(&self) -> bool {
        matches!(self, TValue::Int(_))
    }

    pub fn is_json(&self) -> bool {
        matches!(self, TValue::Json(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, TValue::Str(_))
    }

    pub fn to_json(&self) -> Option<&JsonValue> {
        match self {
            TValue::Json(j) => Some(j),
            _ => None,
        }
    }

    /// `toInt`: zero unless the value is a number.
    pub fn to_int(&self) -> i32 {
        match self {
            TValue::Int(i) => *i,
            _ => 0,
        }
    }

    pub fn to_boolean(&self) -> bool {
        match self {
            TValue::Int(i) => *i != 0,
            other => !other.to_string().is_empty(),
        }
    }

    /// `toJsonValue`: numbers and strings become JSON.
    pub fn to_json_value(&self) -> JsonValue {
        match self {
            TValue::Int(i) => JsonValue::from_int(*i),
            TValue::Str(s) => JsonValue::from_str(s),
            TValue::Json(j) => j.clone(),
        }
    }

    pub fn to_token(&self) -> Token {
        match self {
            TValue::Int(_) => Token::new(self.to_string(), TokenType::Number, None),
            TValue::Json(j) => Token::new(self.to_string(), TokenType::JsonData, Some(j.clone())),
            TValue::Str(_) => Token::new(self.to_string(), TokenType::QuotedString, None),
        }
    }

    fn both_numbers(&self, other: &TValue) -> Option<(i32, i32)> {
        match (self, other) {
            (TValue::Int(a), TValue::Int(b)) => Some((*a, *b)),
            _ => None,
        }
    }

    fn compare(
        &self,
        other: &TValue,
        num: fn(i32, i32) -> bool,
        text: fn(std::cmp::Ordering) -> bool,
    ) -> TValue {
        match self.both_numbers(other) {
            Some((a, b)) => TValue::from_bool(num(a, b)),
            None => TValue::from_bool(text(java_compare(&self.to_string(), &other.to_string()))),
        }
    }
}

/// `String.compareTo` ordering (UTF-16 code units).
fn java_compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    QuotedString,
    JsonData,
    Operator,
    OpenParenMath,
    Comma,
    CloseParenMath,
    Number,
    PlainText,
    Spaces,
    FunctionName,
    OpenParenFunc,
    CloseParenFunc,
    Affectation,
}

pub const COMMERCIAL_MINUS_SIGN: char = '\u{2052}';

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub surface: String,
    pub token_type: TokenType,
    pub json: Option<JsonValue>,
}

impl Token {
    pub fn new(surface: impl Into<String>, token_type: TokenType, json: Option<JsonValue>) -> Self {
        Self {
            surface: surface.into(),
            token_type,
            json,
        }
    }

    pub fn operator(&self) -> Option<TokenOperator> {
        let mut it = self.surface.chars();
        let c1 = it.next()?;
        TokenOperator::of(c1, it.next().unwrap_or('\0'))
    }

    pub fn precedence(&self) -> i32 {
        if self.token_type == TokenType::Affectation {
            return TokenOperator::Equals.precedence();
        }
        self.operator().map_or(0, TokenOperator::precedence)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenOperator {
    Multiplication,
    Division,
    Addition,
    Substraction,
    LessThan,
    GreaterThan,
    LessThanOrEquals,
    GreaterThanOrEquals,
    Equals,
    NotEquals,
    LogicalAnd,
    LogicalOr,
}

impl TokenOperator {
    pub fn of(ch: char, ch2: char) -> Option<TokenOperator> {
        use TokenOperator::*;
        Some(match ch {
            '*' => Multiplication,
            '/' => Division,
            '+' => Addition,
            COMMERCIAL_MINUS_SIGN => Substraction,
            '<' => {
                if ch2 == '=' {
                    LessThanOrEquals
                } else {
                    LessThan
                }
            }
            '>' => {
                if ch2 == '=' {
                    GreaterThanOrEquals
                } else {
                    GreaterThan
                }
            }
            '=' if ch2 == '=' => Equals,
            '!' if ch2 == '=' => NotEquals,
            '&' if ch2 == '&' => LogicalAnd,
            '|' if ch2 == '|' => LogicalOr,
            _ => return None,
        })
    }

    pub fn precedence(self) -> i32 {
        use TokenOperator::*;
        match self {
            Multiplication | Division => 100 - 3,
            Addition | Substraction => 100 - 4,
            LessThan | GreaterThan | LessThanOrEquals | GreaterThanOrEquals => 100 - 6,
            Equals | NotEquals => 100 - 7,
            LogicalAnd => 100 - 11,
            LogicalOr => 100 - 12,
        }
    }

    pub fn display_len(self) -> usize {
        use TokenOperator::*;
        match self {
            LessThanOrEquals | GreaterThanOrEquals | Equals | NotEquals | LogicalAnd
            | LogicalOr => 2,
            _ => 1,
        }
    }

    pub fn operate(self, v1: &TValue, v2: &TValue) -> std::result::Result<TValue, String> {
        use std::cmp::Ordering as O;
        use TokenOperator::*;
        let nums = v1.both_numbers(v2);
        Ok(match self {
            Multiplication => match nums {
                Some((a, b)) => TValue::Int(a.wrapping_mul(b)),
                None => TValue::Str(format!("{v1}*{v2}")),
            },
            Division => match nums {
                Some((_, 0)) => return Err("/ by zero".to_string()),
                Some((a, b)) => TValue::Int(a.wrapping_div(b)),
                None => TValue::Str(format!("{v1}/{v2}")),
            },
            Addition => match nums {
                Some((a, b)) => TValue::Int(a.wrapping_add(b)),
                None => TValue::Str(format!("{v1}{v2}")),
            },
            Substraction => match nums {
                Some((a, b)) => TValue::Int(a.wrapping_sub(b)),
                None => TValue::Str(format!("{v1}{v2}")),
            },
            LessThan => v1.compare(v2, |a, b| a < b, |o| o == O::Less),
            GreaterThan => v1.compare(v2, |a, b| a > b, |o| o == O::Greater),
            LessThanOrEquals => v1.compare(v2, |a, b| a <= b, |o| o != O::Greater),
            GreaterThanOrEquals => v1.compare(v2, |a, b| a >= b, |o| o != O::Less),
            Equals => v1.compare(v2, |a, b| a == b, |o| o == O::Equal),
            NotEquals => v1.compare(v2, |a, b| a != b, |o| o != O::Equal),
            LogicalAnd => TValue::from_bool(v1.to_boolean() && v2.to_boolean()),
            LogicalOr => TValue::from_bool(v1.to_boolean() || v2.to_boolean()),
        })
    }
}

impl TokenType {
    fn is_single_char1(self) -> bool {
        matches!(
            self,
            TokenType::OpenParenMath | TokenType::Comma | TokenType::CloseParenMath
        )
    }

    fn from_char(ch: char, ch2: char) -> TokenType {
        if TLineType::is_quote(ch) {
            TokenType::QuotedString
        } else if ch == '=' {
            TokenType::Affectation
        } else if ch == '(' {
            TokenType::OpenParenMath
        } else if ch == ')' {
            TokenType::CloseParenMath
        } else if ch == ',' {
            TokenType::Comma
        } else if TLineType::is_latin_digit(ch) {
            TokenType::Number
        } else if TLineType::is_space_char(ch) {
            TokenType::Spaces
        } else if ch == '-' || TokenOperator::of(ch, ch2).is_some() {
            TokenType::Operator
        } else {
            TokenType::PlainText
        }
    }

    fn is_plain_text_break(ch: char, ch2: char) -> bool {
        let t = Self::from_char(ch, ch2);
        t.is_single_char1()
            || t == TokenType::Operator
            || t == TokenType::Spaces
            || t == TokenType::Affectation
    }

    fn is_subtraction_operator(last: Option<&Token>) -> bool {
        match last {
            None => false,
            Some(t) => !matches!(
                t.token_type,
                TokenType::Operator
                    | TokenType::OpenParenMath
                    | TokenType::Comma
                    | TokenType::Affectation
            ),
        }
    }

    /// Reads the next token of `eater` (`eatOneToken`).
    pub fn eat_one_token(
        last: Option<&Token>,
        eater: &mut Eater,
        manage_colon: bool,
    ) -> Result<Option<Token>> {
        let mut ch = eater.peek_char();
        if ch == '\0' {
            return Ok(None);
        }
        if manage_colon && ch == ':' {
            return Ok(None);
        }
        if ch == '-' && Self::is_subtraction_operator(last) {
            ch = COMMERCIAL_MINUS_SIGN;
        }
        let op = TokenOperator::of(ch, eater.peek_char_n2());
        if TLineType::is_quote(ch) {
            return Ok(Some(Token::new(
                eater.eat_and_get_quoted_string()?,
                TokenType::QuotedString,
                None,
            )));
        }
        if let Some(op) = op {
            if op.display_len() == 1 {
                eater.eat_one_char();
                return Ok(Some(Token::new(ch.to_string(), TokenType::Operator, None)));
            }
            let a = eater.eat_one_char();
            let b = eater.eat_one_char();
            return Ok(Some(Token::new(
                format!("{a}{b}"),
                TokenType::Operator,
                None,
            )));
        }
        let single = |eater: &mut Eater, t| Token::new(eater.eat_one_char().to_string(), t, None);
        Ok(Some(if ch == '=' {
            single(eater, TokenType::Affectation)
        } else if ch == '(' {
            single(eater, TokenType::OpenParenMath)
        } else if ch == ')' {
            single(eater, TokenType::CloseParenMath)
        } else if ch == ',' {
            single(eater, TokenType::Comma)
        } else if TLineType::is_latin_digit(ch) || ch == '-' {
            Token::new(eater.eat_and_get_number()?, TokenType::Number, None)
        } else if TLineType::is_space_char(ch) {
            Token::new(eater.eat_and_get_spaces(), TokenType::Spaces, None)
        } else {
            let mut result = String::new();
            loop {
                let c = eater.peek_char();
                if c == '\0' || Self::is_plain_text_break(c, eater.peek_char_n2()) {
                    break;
                }
                result.push(eater.eat_one_char());
            }
            Token::new(result, TokenType::PlainText, None)
        }))
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokenStack {
    tokens: Vec<Token>,
}

impl TokenStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(&self) -> usize {
        self.tokens.len()
    }

    pub fn add(&mut self, t: Token) {
        self.tokens.push(t);
    }

    pub fn without_space(&self) -> TokenStack {
        TokenStack {
            tokens: self
                .tokens
                .iter()
                .filter(|t| t.token_type != TokenType::Spaces)
                .cloned()
                .collect(),
        }
    }

    fn sub_token_stack(&self, i: usize) -> TokenStack {
        TokenStack {
            tokens: self.tokens[i..].to_vec(),
        }
    }

    /// Reads tokens up to a `,` or `)` that is not nested (`eatUntilCloseParenthesisOrComma`).
    pub fn eat_until_close_parenthesis_or_comma(eater: &mut Eater) -> Result<TokenStack> {
        let mut result = TokenStack::new();
        let mut level = 0;
        let mut last: Option<Token> = None;
        loop {
            eater.skip_spaces();
            let ch = eater.peek_char();
            if ch == '\0' {
                return Err(EaterException::new("until001", eater.string_located()));
            }
            if level == 0 && (ch == ',' || ch == ')') {
                return Ok(result);
            }
            let token = TokenType::eat_one_token(last.as_ref(), eater, false)?.expect("a token");
            match token.token_type {
                TokenType::OpenParenMath => level += 1,
                TokenType::CloseParenMath => level -= 1,
                _ => {}
            }
            if token.token_type != TokenType::Spaces {
                last = Some(token.clone());
            }
            result.add(token);
        }
    }

    fn eat_until_close_in_iterator(&self, pos: &mut usize, location: &StringLocated) -> Result<()> {
        let mut level = 0;
        loop {
            let Some(ch) = self.tokens.get(*pos) else {
                return Err(EaterException::new("until002", location));
            };
            let t = ch.token_type;
            if (level == 0 && (t == TokenType::Comma || t == TokenType::CloseParenMath))
                || t == TokenType::CloseParenFunc
            {
                return Ok(());
            }
            *pos += 1;
            match t {
                TokenType::OpenParenMath | TokenType::OpenParenFunc => level += 1,
                TokenType::CloseParenMath | TokenType::CloseParenFunc => level -= 1,
                _ => {}
            }
        }
    }

    fn count_function_arg(&self, mut pos: usize, location: &StringLocated) -> Result<usize> {
        let Some(first) = self.tokens.get(pos) else {
            return Err(EaterException::new("count12", location));
        };
        if matches!(
            first.token_type,
            TokenType::CloseParenMath | TokenType::CloseParenFunc
        ) {
            return Ok(0);
        }
        let mut result = 1;
        while pos < self.tokens.len() {
            self.eat_until_close_in_iterator(&mut pos, location)?;
            let t = self.tokens[pos].token_type;
            pos += 1;
            match t {
                TokenType::CloseParenMath | TokenType::CloseParenFunc => return Ok(result),
                TokenType::Comma => result += 1,
                _ => return Err(EaterException::new("count13", location)),
            }
        }
        Err(EaterException::new("count12", location))
    }

    /// Turns `name(` ... `)` into function calls (`guessFunctions`).
    pub fn guess_functions(&mut self, location: &StringLocated) -> Result<()> {
        let mut open: Vec<usize> = Vec::new();
        let mut parens: Vec<(usize, usize)> = Vec::new();
        for (i, t) in self.tokens.iter().enumerate() {
            match t.token_type {
                TokenType::OpenParenMath => open.push(i),
                TokenType::CloseParenMath => {
                    if let Some(o) = open.pop() {
                        parens.push((o, i));
                    }
                }
                _ => {}
            }
        }
        // Outermost pairs first: the argument count of a call must be taken before the nested calls
        // inside it are rewritten to OPEN_PAREN_FUNC/CLOSE_PAREN_FUNC.
        parens.sort_by_key(|(open, _)| *open);
        for (iopen, iclose) in parens {
            if iopen > 0 && self.tokens[iopen - 1].token_type == TokenType::PlainText {
                let surface = self.tokens[iopen - 1].surface.clone();
                self.tokens[iopen - 1] = Token::new(surface, TokenType::FunctionName, None);
                let nb = self
                    .sub_token_stack(iopen + 1)
                    .count_function_arg(0, location)?;
                self.tokens[iopen] = Token::new(nb.to_string(), TokenType::OpenParenFunc, None);
                self.tokens[iclose] = Token::new(")", TokenType::CloseParenFunc, None);
            }
        }
        Ok(())
    }

    /// Evaluates the expression (`getResult`).
    pub fn get_result(
        &self,
        location: &StringLocated,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<TValue> {
        let mut tmp = self.without_space();
        tmp.guess_functions(location)?;
        let queue = shunting_yard(&tmp, cx, mem, location)?;
        reverse_polish(location, &queue, cx, mem)
    }
}

fn is_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '$' | '_'))
}

fn is_operator_or_affectation(t: &Token) -> bool {
    matches!(t.token_type, TokenType::Operator | TokenType::Affectation)
}

fn shunting_yard(
    input: &TokenStack,
    cx: &mut TContext,
    mem: &mut TMemory,
    location: &StringLocated,
) -> Result<TokenStack> {
    let mut output = TokenStack::new();
    let mut ops: Vec<Token> = Vec::new(); // top is the end of the vector
    for token in &input.tokens {
        match token.token_type {
            TokenType::Number | TokenType::QuotedString => output.add(token.clone()),
            TokenType::FunctionName => ops.push(token.clone()),
            TokenType::PlainText => {
                let name = &token.surface;
                match cx.knowledge_variable(mem, name, location)? {
                    None => {
                        if !is_variable_name(name) {
                            return Err(EaterException::new(
                                format!("Parsing syntax error about {name}"),
                                location,
                            ));
                        }
                        output.add(Token::new(name.clone(), TokenType::QuotedString, None));
                    }
                    Some(v) => output.add(v.to_token()),
                }
            }
            TokenType::Operator | TokenType::Affectation => {
                loop {
                    let top_is_function = ops
                        .last()
                        .is_some_and(|t| t.token_type == TokenType::FunctionName);
                    let greater = ops.last().is_some_and(|t| {
                        is_operator_or_affectation(t) && t.precedence() > token.precedence()
                    });
                    let equal = ops.last().is_some_and(|t| {
                        is_operator_or_affectation(t) && t.precedence() == token.precedence()
                    });
                    if top_is_function || greater || equal {
                        output.add(ops.pop().expect("top"));
                    } else {
                        break;
                    }
                }
                ops.push(token.clone());
            }
            TokenType::OpenParenFunc | TokenType::OpenParenMath => ops.push(token.clone()),
            TokenType::CloseParenFunc => {
                while ops
                    .last()
                    .is_some_and(|t| t.token_type != TokenType::OpenParenFunc)
                {
                    output.add(ops.pop().expect("top"));
                }
                let first = ops
                    .pop()
                    .ok_or_else(|| EaterException::new("Parsing syntax error", location))?;
                output.add(first);
            }
            TokenType::CloseParenMath => {
                while ops
                    .last()
                    .is_some_and(|t| t.token_type != TokenType::OpenParenMath)
                {
                    output.add(ops.pop().expect("top"));
                }
                if ops.is_empty() {
                    return Err(EaterException::new("Parsing syntax error", location));
                }
                ops.pop();
            }
            TokenType::Comma => {
                while ops
                    .last()
                    .is_some_and(|t| t.token_type != TokenType::OpenParenFunc)
                {
                    output.add(ops.pop().expect("top"));
                }
            }
            _ => {
                return Err(EaterException::new(
                    format!("unsupported token {token:?}"),
                    location,
                ))
            }
        }
    }
    while let Some(t) = ops.pop() {
        output.add(t);
    }
    Ok(output)
}

fn reverse_polish(
    location: &StringLocated,
    queue: &TokenStack,
    cx: &mut TContext,
    mem: &mut TMemory,
) -> Result<TValue> {
    let mut named: HashMap<String, TValue> = HashMap::new();
    let mut stack: Vec<TValue> = Vec::new();
    let pop = |stack: &mut Vec<TValue>| {
        stack
            .pop()
            .ok_or_else(|| EaterException::new("rpn41", location))
    };
    let mut i = 0;
    while i < queue.tokens.len() {
        let token = &queue.tokens[i];
        i += 1;
        match token.token_type {
            TokenType::Number => {
                stack.push(TValue::Int(token.surface.parse::<i32>().map_err(|_| {
                    EaterException::new("Fatal parsing error", location)
                })?))
            }
            TokenType::QuotedString => stack.push(TValue::Str(token.surface.clone())),
            TokenType::JsonData => stack.push(TValue::Json(token.json.clone().expect("json"))),
            TokenType::Affectation => {
                let v2 = pop(&mut stack)?;
                let v1 = pop(&mut stack)?;
                named.insert(v1.to_string(), v2);
            }
            TokenType::Operator => {
                let v2 = pop(&mut stack)?;
                let v1 = pop(&mut stack)?;
                let op = token
                    .operator()
                    .ok_or_else(|| EaterException::new("bad op", location))?;
                let r = op
                    .operate(&v1, &v2)
                    .map_err(|_| EaterException::new("Fatal parsing error", location))?;
                stack.push(r);
            }
            TokenType::OpenParenFunc => {
                let declared: i64 = token.surface.parse().unwrap_or(0);
                let nb = usize::try_from(declared - named.len() as i64)
                    .map_err(|_| EaterException::new("rpn45", location))?;
                let Some(token2) = queue.tokens.get(i) else {
                    return Err(EaterException::new("rpn43", location));
                };
                i += 1;
                if token2.token_type != TokenType::FunctionName {
                    return Err(EaterException::new("rpn43", location));
                }
                let function: Rc<dyn TFunction> = cx
                    .functions
                    .function_smart(&TFunctionSignature::new(&token2.surface, nb))
                    .ok_or_else(|| {
                        EaterException::new(
                            format!("Unknown built-in function {}", token2.surface),
                            location,
                        )
                    })?;
                if !function.can_cover(nb, &Default::default()) {
                    return Err(EaterException::new(
                        format!(
                            "Bad number of arguments for {}",
                            function.signature().function_name
                        ),
                        location,
                    ));
                }
                let mut args = Vec::with_capacity(nb);
                for _ in 0..nb {
                    args.insert(0, pop(&mut stack)?);
                }
                let r = function.execute_return(cx, mem, location, &args, &named)?;
                named.clear();
                stack.push(r);
            }
            _ => return Err(EaterException::new("rpn41", location)),
        }
    }
    pop(&mut stack)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operators() {
        let two = TValue::Int(2);
        let three = TValue::Int(3);
        let s = TValue::text("a");
        assert_eq!(
            TokenOperator::Addition.operate(&two, &three).unwrap(),
            TValue::Int(5)
        );
        assert_eq!(
            TokenOperator::Addition.operate(&s, &three).unwrap(),
            TValue::text("a3")
        );
        assert_eq!(
            TokenOperator::Multiplication.operate(&s, &three).unwrap(),
            TValue::text("a*3")
        );
        assert!(TokenOperator::Division
            .operate(&two, &TValue::Int(0))
            .is_err());
        assert_eq!(
            TokenOperator::LessThan.operate(&two, &three).unwrap(),
            TValue::Int(1)
        );
        assert_eq!(
            TokenOperator::Equals.operate(&s, &s).unwrap(),
            TValue::Int(1)
        );
        assert_eq!(
            TokenOperator::NotEquals.operate(&s, &three).unwrap(),
            TValue::Int(1)
        );
        assert_eq!(
            TokenOperator::LogicalAnd
                .operate(&two, &TValue::Int(0))
                .unwrap(),
            TValue::Int(0)
        );
        assert_eq!(
            TokenOperator::LogicalOr
                .operate(&two, &TValue::Int(0))
                .unwrap(),
            TValue::Int(1)
        );
        assert!(!TValue::text("").to_boolean() && TValue::text("x").to_boolean());
    }
}
