//! Reading of a preprocessor line (`Eater`, `EaterFunctionCall`).

use super::context::TContext;
use super::expr::{TValue, Token, TokenStack, TokenType};
use super::functions::{TFunctionArgument, TFunctionImpl, TFunctionType};
use super::line_type::TLineType;
use super::memory::TMemory;
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::text::StringLocated;
use std::collections::HashMap;

pub struct Eater {
    chars: Vec<char>,
    i: usize,
    sl: StringLocated,
}

impl Eater {
    pub fn new(sl: StringLocated) -> Self {
        Self {
            chars: sl.string().chars().collect(),
            i: 0,
            sl,
        }
    }

    pub fn from_text(s: &str) -> Self {
        Self::new(StringLocated::new(s, None))
    }

    pub fn string_located(&self) -> &StringLocated {
        &self.sl
    }

    pub fn current_position(&self) -> usize {
        self.i
    }

    pub fn eat_all_to_end(&mut self) -> String {
        let r: String = self.chars[self.i.min(self.chars.len())..].iter().collect();
        self.i = self.chars.len();
        r
    }

    pub fn eat_expression(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<TValue> {
        let ch = self.peek_char();
        if ch == '{' || ch == '[' {
            let data = self.eat_all_to_end();
            return match JsonValue::parse(&data) {
                Ok(j) => Ok(TValue::Json(j)),
                Err(e) => {
                    let mut ex = EaterException::new("Fatal parsing error", &self.sl);
                    ex.json_column = Some(e.column);
                    Err(ex)
                }
            };
        }
        let stack = self.eat_token_stack()?;
        stack.get_result(&self.sl.clone(), cx, mem)
    }

    pub fn eat_token_stack(&mut self) -> Result<TokenStack> {
        let mut stack = TokenStack::new();
        self.add_into_token_stack(&mut stack, false)?;
        if stack.size() == 0 {
            return Err(EaterException::new("Missing expression", &self.sl));
        }
        Ok(stack)
    }

    pub fn eat_expression_stop_at_colon(
        &mut self,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<TValue> {
        let mut stack = TokenStack::new();
        self.add_into_token_stack(&mut stack, true)?;
        stack.get_result(&self.sl.clone(), cx, mem)
    }

    fn add_into_token_stack(&mut self, stack: &mut TokenStack, stop_at_colon: bool) -> Result<()> {
        let mut last: Option<Token> = None;
        loop {
            let Some(token) = TokenType::eat_one_token(last.as_ref(), self, stop_at_colon)? else {
                return Ok(());
            };
            stack.add(token.clone());
            if token.token_type != TokenType::Spaces {
                last = Some(token);
            }
        }
    }

    pub fn eat_and_get_quoted_string(&mut self) -> Result<String> {
        let separator = self.peek_char();
        if !TLineType::is_quote(separator) {
            return Err(EaterException::new("quote10", &self.sl));
        }
        self.check_and_eat_char(separator)?;
        let mut value = String::new();
        self.add_up_to(separator, &mut value);
        self.check_and_eat_char(separator)?;
        Ok(value)
    }

    pub fn eat_and_get_optional_quoted_string(&mut self) -> Result<String> {
        if TLineType::is_quote(self.peek_char()) {
            return self.eat_and_get_quoted_string();
        }
        let mut value = String::new();
        let mut level = 0;
        loop {
            let ch = self.peek_char();
            if ch == '\0' {
                return Err(EaterException::new("until001", &self.sl));
            }
            if level == 0 && (ch == ',' || ch == ')') {
                return Ok(value.trim().to_string());
            }
            let ch = self.eat_one_char();
            if ch == '(' {
                level += 1;
            } else if ch == ')' {
                level -= 1;
            }
            value.push(ch);
        }
    }

    pub fn eat_and_get_number(&mut self) -> Result<String> {
        let mut result = String::new();
        loop {
            let ch = self.peek_char();
            if result.is_empty() && ch == '-' {
                result.push(self.eat_one_char());
                continue;
            }
            if ch == '\0' || !TLineType::is_latin_digit(ch) {
                return Ok(result);
            }
            result.push(self.eat_one_char());
        }
    }

    pub fn eat_and_get_spaces(&mut self) -> String {
        let mut result = String::new();
        loop {
            let ch = self.peek_char();
            if ch == '\0' || !TLineType::is_space_char(ch) {
                return result;
            }
            result.push(self.eat_one_char());
        }
    }

    pub fn eat_and_get_varname(&mut self) -> Result<String> {
        self.eat_name("a002")
    }

    pub fn eat_and_get_function_name(&mut self) -> Result<String> {
        self.eat_name("a003")
    }

    fn eat_name(&mut self, err: &str) -> Result<String> {
        if self.i >= self.chars.len() {
            return Err(EaterException::new(err, &self.sl));
        }
        let first = self.eat_one_char();
        if !TLineType::is_letter_or_emoji_or_underscore_or_dollar(first) {
            return Err(EaterException::new(err, &self.sl));
        }
        let mut name = first.to_string();
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            if !TLineType::is_letter_or_emoji_or_underscore_or_digit(ch) {
                break;
            }
            self.i += 1;
            name.push(ch);
        }
        Ok(name)
    }

    pub fn skip_spaces(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    pub fn skip_until_char(&mut self, ch: char) {
        while self.i < self.chars.len() && self.chars[self.i] != ch {
            self.i += 1;
        }
    }

    pub fn peek_char(&self) -> char {
        self.chars.get(self.i).copied().unwrap_or('\0')
    }

    pub fn peek_char_n2(&self) -> char {
        self.chars.get(self.i + 1).copied().unwrap_or('\0')
    }

    /// Whether the rest looks like `name = ...` (`matchAffectation`).
    pub fn match_affectation(&self) -> bool {
        let rest = &self.chars[self.i.min(self.chars.len())..];
        let mut k = 0;
        if rest.first() == Some(&'$') {
            k += 1;
        }
        match rest.get(k) {
            Some(c) if c.is_alphabetic() || *c == '_' => k += 1,
            _ => return false,
        }
        while rest
            .get(k)
            .is_some_and(|c| c.is_alphabetic() || *c == '_' || c.is_ascii_digit())
        {
            k += 1;
        }
        while rest.get(k).is_some_and(|c| c.is_whitespace()) {
            k += 1;
        }
        rest.get(k) == Some(&'=')
    }

    pub fn has_next_char(&self) -> bool {
        self.i < self.chars.len()
    }

    pub fn eat_one_char(&mut self) -> char {
        let c = self.chars[self.i];
        self.i += 1;
        c
    }

    pub fn check_and_eat_char(&mut self, ch: char) -> Result<()> {
        if self.i >= self.chars.len() || self.chars[self.i] != ch {
            return Err(EaterException::new("a001", &self.sl));
        }
        self.i += 1;
        Ok(())
    }

    pub fn safe_check_and_eat_char(&mut self, ch: char) -> bool {
        if self.i >= self.chars.len() || self.chars[self.i] != ch {
            return false;
        }
        self.i += 1;
        true
    }

    pub fn check_and_eat_str(&mut self, s: &str) -> Result<()> {
        for c in s.chars() {
            self.check_and_eat_char(c)?;
        }
        Ok(())
    }

    fn add_up_to(&mut self, separator: char, sb: &mut String) {
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            if ch == separator {
                return;
            }
            self.i += 1;
            sb.push(ch);
        }
    }

    pub fn eat_declare_function(
        &mut self,
        cx: &mut TContext,
        mem: &mut TMemory,
        unquoted: bool,
        location: &StringLocated,
        allow_no_parenthesis: bool,
        ftype: TFunctionType,
    ) -> Result<TFunctionImpl> {
        let mut args: Vec<TFunctionArgument> = Vec::new();
        let name = self.eat_and_get_function_name()?;
        self.skip_spaces();
        if !self.safe_check_and_eat_char('(') {
            if allow_no_parenthesis {
                return Ok(TFunctionImpl::new(&name, args, unquoted, ftype));
            }
            return Err(EaterException::new("Missing opening parenthesis", &self.sl));
        }
        loop {
            self.skip_spaces();
            let ch = self.peek_char();
            if TLineType::is_letter_or_emoji_or_underscore_or_dollar(ch) {
                let varname = self.eat_and_get_varname()?;
                self.skip_spaces();
                let def = if self.peek_char() == '=' {
                    self.eat_one_char();
                    let mut stack = TokenStack::eat_until_close_parenthesis_or_comma(self)?;
                    stack.guess_functions(location)?;
                    Some(stack.get_result(&self.sl.clone(), cx, mem)?)
                } else {
                    None
                };
                args.push(TFunctionArgument::new(&varname, def));
            } else if ch == ',' {
                self.check_and_eat_char(',')?;
            } else if ch == ')' {
                self.check_and_eat_char(')')?;
                break;
            } else {
                return Err(EaterException::new(
                    "Error in function definition",
                    &self.sl,
                ));
            }
        }
        self.skip_spaces();
        Ok(TFunctionImpl::new(&name, args, unquoted, ftype))
    }

    pub fn eat_declare_return_function_with_optional_return(
        &mut self,
        cx: &mut TContext,
        mem: &mut TMemory,
        unquoted: bool,
        location: &StringLocated,
    ) -> Result<TFunctionImpl> {
        let result = self.eat_declare_function(
            cx,
            mem,
            unquoted,
            location,
            false,
            TFunctionType::ReturnFunction,
        )?;
        if self.peek_char() == 'r' {
            self.check_and_eat_str("return")?;
            self.skip_spaces();
            let line = format!("!return {}", self.eat_all_to_end());
            result.add_body(StringLocated::new(line, location.location()))?;
        } else if self.peek_char() == '!' {
            self.check_and_eat_str("!return")?;
            self.skip_spaces();
            let line = format!("!return {}", self.eat_all_to_end());
            result.add_body(StringLocated::new(line, location.location()))?;
        }
        Ok(result)
    }
}

/// Arguments of a call `name(a, b=c)` (`EaterFunctionCall`).
#[derive(Default)]
pub struct FunctionCall {
    pub values: Vec<TValue>,
    pub named: HashMap<String, TValue>,
    pub position: usize,
}

impl FunctionCall {
    pub fn analyze(
        s: StringLocated,
        is_legacy_define: bool,
        unquoted: bool,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<FunctionCall> {
        let mut e = Eater::new(s);
        let mut call = FunctionCall::default();
        let result = call.run(&mut e, is_legacy_define, unquoted, cx, mem);
        call.position = e.current_position();
        result.map(|()| call)
    }

    fn run(
        &mut self,
        e: &mut Eater,
        is_legacy_define: bool,
        unquoted: bool,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<()> {
        e.skip_until_char('(');
        e.check_and_eat_char('(')?;
        e.skip_spaces();
        if e.peek_char() == ')' {
            e.check_and_eat_char(')')?;
            return Ok(());
        }
        let loc = e.sl.location();
        loop {
            e.skip_spaces();
            if is_legacy_define {
                let read = e.eat_and_get_optional_quoted_string()?;
                let value =
                    cx.apply_functions_and_variables(mem, &StringLocated::new(read, loc))?;
                self.values.push(TValue::Str(value.unwrap_or_default()));
            } else if unquoted {
                if e.match_affectation() {
                    let varname = e.eat_and_get_varname()?;
                    e.skip_spaces();
                    e.check_and_eat_char('=')?;
                    e.skip_spaces();
                    let read = e.eat_and_get_optional_quoted_string()?;
                    let value =
                        cx.apply_functions_and_variables(mem, &StringLocated::new(read, loc))?;
                    self.named
                        .insert(varname, TValue::Str(value.unwrap_or_default()));
                } else {
                    let read = e.eat_and_get_optional_quoted_string()?;
                    let value =
                        cx.apply_functions_and_variables(mem, &StringLocated::new(read, loc))?;
                    self.values.push(TValue::Str(value.unwrap_or_default()));
                }
            } else if e.match_affectation() {
                let varname = e.eat_and_get_varname()?;
                e.skip_spaces();
                e.check_and_eat_char('=')?;
                e.skip_spaces();
                let mut tokens =
                    TokenStack::eat_until_close_parenthesis_or_comma(e)?.without_space();
                tokens.guess_functions(&e.sl)?;
                let result = tokens.get_result(&e.sl.clone(), cx, mem)?;
                self.named.insert(varname, result);
            } else {
                let mut tokens =
                    TokenStack::eat_until_close_parenthesis_or_comma(e)?.without_space();
                tokens.guess_functions(&e.sl)?;
                let result = tokens.get_result(&e.sl.clone(), cx, mem)?;
                self.values.push(result);
            }
            e.skip_spaces();
            if !e.has_next_char() {
                return Err(EaterException::new("call001", &e.sl));
            }
            let ch = e.eat_one_char();
            if ch == ',' {
                continue;
            }
            if ch == ')' {
                return Ok(());
            }
            if unquoted {
                return Err(EaterException::new(
                    "unquoted function/procedure cannot use expression.",
                    &e.sl,
                ));
            }
            return Err(EaterException::new("call001", &e.sl));
        }
    }
}
