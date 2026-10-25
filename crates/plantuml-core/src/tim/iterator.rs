//! The chain of line filters that interprets directives (`CodeIterator` and its layers).

use super::context::{analyze_startsub, Sub, TContext};
use super::eater::Eater;
use super::expr::{TValue, TokenStack};
use super::functions::TFunctionType;
use super::line_type::TLineType;
use super::memory::{
    json_size, ExecutionContextForeach, ExecutionContextIf, ExecutionContextWhile, TMemory,
    TVariableScope,
};
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::text::StringLocated;

pub trait CodeIterator {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>>;
    fn next(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<()>;
    /// Position in the underlying list of lines (`getCodePosition`).
    fn position(&self) -> usize;
    fn jump(&mut self, pos: usize, location: &StringLocated) -> Result<()>;
}

/// Source of lines (`CodeIteratorImpl`).
pub struct CodeIteratorImpl {
    list: Vec<StringLocated>,
    current: usize,
    count_jump: usize,
}

impl CodeIteratorImpl {
    pub fn new(list: Vec<StringLocated>) -> Self {
        Self {
            list,
            current: 0,
            count_jump: 0,
        }
    }
}

impl CodeIterator for CodeIteratorImpl {
    fn peek(&mut self, _: &mut TContext, _: &mut TMemory) -> Result<Option<StringLocated>> {
        Ok(self.list.get(self.current).cloned())
    }

    fn next(&mut self, _: &mut TContext, _: &mut TMemory) -> Result<()> {
        assert!(self.current < self.list.len(), "no more lines");
        self.current += 1;
        Ok(())
    }

    fn position(&self) -> usize {
        self.current
    }

    fn jump(&mut self, pos: usize, location: &StringLocated) -> Result<()> {
        self.count_jump += 1;
        if self.count_jump > 999 {
            return Err(EaterException::new("Infinite loop?", location));
        }
        self.current = pos;
        Ok(())
    }
}

macro_rules! delegate {
    () => {
        fn next(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<()> {
            self.source.next(cx, mem)
        }

        fn position(&self) -> usize {
            self.source.position()
        }

        fn jump(&mut self, pos: usize, location: &StringLocated) -> Result<()> {
            self.source.jump(pos, location)
        }
    };
}

type Source = Box<dyn CodeIterator>;

struct LongComment {
    source: Source,
}

impl CodeIterator for LongComment {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(first) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            if TLineType::of(&first) != TLineType::CommentLongStart {
                return Ok(Some(first));
            }
            while let Some(s) = self.source.peek(cx, mem)? {
                if s.trimmed().string().ends_with("'/") {
                    break;
                }
                cx.debug.push(s);
                self.source.next(cx, mem)?;
            }
            if let Some(s) = self.source.peek(cx, mem)? {
                cx.debug.push(s);
                self.source.next(cx, mem)?;
            }
        }
    }

    delegate!();
}

struct ShortComment {
    source: Source,
}

impl CodeIterator for ShortComment {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(r) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            if TLineType::of(&r) == TLineType::CommentSimple {
                cx.debug.push(r);
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(r));
        }
    }

    delegate!();
}

fn remove_special_inner_comment(s: &str) -> String {
    if s.contains("/'''") && s.contains("'''/") {
        let re = regex::Regex::new(r"/'''[-\w]*'''/").expect("valid");
        return re.replace_all(s, "").into_owned();
    }
    s.to_string()
}

/// `StringLocated.removeInnerComment`.
pub fn remove_inner_comment(sl: &StringLocated) -> StringLocated {
    let s = sl.string();
    let trim = s.replace('\t', " ");
    let trim = trim.trim();
    if trim.starts_with("/'") {
        if let Some(idx) = s.find("'/") {
            return StringLocated::new(remove_special_inner_comment(&s[idx + 2..]), sl.location());
        }
    }
    if trim.ends_with("'/") {
        if let Some(idx) = s.rfind("/'") {
            return StringLocated::new(remove_special_inner_comment(&s[..idx]), sl.location());
        }
    }
    if trim.contains("/'''") && trim.contains("'''/") {
        return StringLocated::new(remove_special_inner_comment(s), sl.location());
    }
    sl.clone()
}

struct InnerComment {
    source: Source,
}

impl CodeIterator for InnerComment {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        Ok(self.source.peek(cx, mem)?.map(|r| remove_inner_comment(&r)))
    }

    delegate!();
}

struct SubLayer {
    source: Source,
    reading: Option<CodeIteratorImpl>,
}

impl CodeIterator for SubLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        if let Some(r) = &mut self.reading {
            return r.peek(cx, mem);
        }
        let Some(result) = self.source.peek(cx, mem)? else {
            return Ok(None);
        };
        if TLineType::of(&result) == TLineType::StartSub {
            let name = analyze_startsub(&result.trimmed(), cx, mem)?;
            let mut created = Sub {
                name: name.clone(),
                lines: Vec::new(),
            };
            self.source.next(cx, mem)?;
            while let Some(s) = self.source.peek(cx, mem)? {
                match TLineType::of(&s) {
                    TLineType::StartSub => {
                        return Err(EaterException::new("Cannot nest sub", &result))
                    }
                    TLineType::EndSub => {
                        self.source.next(cx, mem)?;
                        cx.subs.insert(name.clone(), created.clone());
                        self.reading = Some(CodeIteratorImpl::new(created.lines.clone()));
                        break;
                    }
                    _ => {
                        created.lines.push(s);
                        self.source.next(cx, mem)?;
                    }
                }
            }
            cx.subs.insert(name, created);
        }
        if let Some(r) = &mut self.reading {
            return r.peek(cx, mem);
        }
        Ok(Some(result))
    }

    fn next(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<()> {
        let Some(r) = &mut self.reading else {
            return self.source.next(cx, mem);
        };
        r.next(cx, mem)?;
        if r.peek(cx, mem)?.is_none() {
            self.reading = None;
        }
        Ok(())
    }

    fn position(&self) -> usize {
        self.source.position()
    }

    fn jump(&mut self, pos: usize, location: &StringLocated) -> Result<()> {
        self.source.jump(pos, location)
    }
}

struct ReturnFunctionLayer {
    source: Source,
}

impl CodeIterator for ReturnFunctionLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            if let Some(p) = cx.functions.pending.clone() {
                if super::functions::TFunction::function_type(p.as_ref())
                    == TFunctionType::ReturnFunction
                {
                    cx.debug.push(result.clone());
                    if TLineType::of(&result) == TLineType::EndFunction {
                        if !p.does_contain_return() {
                            return Err(EaterException::new(
                                "This function does not have any !return directive. Declare it as a procedure instead ?",
                                &result,
                            ));
                        }
                        cx.execute_end_function();
                    } else {
                        p.add_body(result)?;
                    }
                    self.next(cx, mem)?;
                    continue;
                }
            }
            if TLineType::of(&result) == TLineType::DeclareReturnFunction {
                cx.debug.push(result.clone());
                cx.execute_declare_function(mem, &result, false)?;
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

struct ProcedureLayer {
    source: Source,
}

impl CodeIterator for ProcedureLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            if let Some(p) = cx.functions.pending.clone() {
                let t = super::functions::TFunction::function_type(p.as_ref());
                if matches!(
                    t,
                    TFunctionType::Procedure | TFunctionType::LegacyDefineLong
                ) {
                    cx.debug.push(result.clone());
                    if TLineType::of(&result) == TLineType::EndFunction {
                        cx.execute_end_function();
                    } else {
                        p.add_body(result)?;
                    }
                    self.next(cx, mem)?;
                    continue;
                }
            }
            if TLineType::of(&result) == TLineType::DeclareProcedure {
                cx.debug.push(result.clone());
                cx.execute_declare_function(mem, &result, true)?;
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

/// `!ifdef` expressions: identifiers combined with `!`, `&&`, `||` and parentheses (`EvalBoolean`).
pub fn eval_boolean(expr: &str, truth: &dyn Fn(&str) -> bool) -> bool {
    struct P<'a> {
        s: Vec<char>,
        pos: isize,
        ch: char,
        truth: &'a dyn Fn(&str) -> bool,
    }
    impl P<'_> {
        fn next_char(&mut self) {
            self.pos += 1;
            self.ch = self.s.get(self.pos as usize).copied().unwrap_or('\0');
        }

        fn eat(&mut self, c: char) -> bool {
            while self.ch == ' ' {
                self.next_char();
            }
            if self.ch == c {
                self.next_char();
                return true;
            }
            false
        }

        fn expression(&mut self) -> bool {
            let mut x = self.term();
            while self.eat('|') {
                self.eat('|');
                x |= self.term();
            }
            x
        }

        fn term(&mut self) -> bool {
            let mut x = self.factor();
            while self.eat('&') {
                self.eat('&');
                x &= self.factor();
            }
            x
        }

        fn is_identifier(&self) -> bool {
            self.ch == '_' || self.ch == '$' || self.ch.is_alphanumeric()
        }

        fn factor(&mut self) -> bool {
            if self.eat('!') {
                return !self.factor();
            }
            let start = self.pos;
            if self.eat('(') {
                let x = self.expression();
                self.eat(')');
                x
            } else if self.is_identifier() {
                while self.is_identifier() {
                    self.next_char();
                }
                let name: String = self.s[start as usize..self.pos as usize].iter().collect();
                (self.truth)(&name)
            } else {
                false
            }
        }
    }
    let mut p = P {
        s: expr.chars().collect(),
        pos: -1,
        ch: '\0',
        truth,
    };
    p.next_char();
    p.expression()
}

struct IfLayer {
    source: Source,
}

impl IfLayer {
    fn condition(
        &self,
        line: &StringLocated,
        keyword: &str,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<bool> {
        let mut e = Eater::new(line.clone());
        e.skip_spaces();
        e.check_and_eat_str(keyword)?;
        e.skip_spaces();
        Ok(e.eat_expression(cx, mem)?.to_boolean())
    }

    fn process(
        &mut self,
        line: &StringLocated,
        ty: TLineType,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<()> {
        match ty {
            TLineType::If => {
                let is_true = if mem.are_all_if_ok() {
                    self.condition(line, "!if", cx, mem)?
                } else {
                    false
                };
                mem.add_if(ExecutionContextIf::from_value(is_true));
            }
            TLineType::Ifdef => {
                let mut e = Eater::new(line.clone());
                e.skip_spaces();
                e.check_and_eat_str("!ifdef")?;
                e.skip_spaces();
                let expr = e.eat_all_to_end();
                let is_true = eval_boolean(&expr, &|name| {
                    mem.get_variable(name).is_some() || cx.does_function_exist(name)
                });
                mem.add_if(ExecutionContextIf::from_value(is_true));
            }
            TLineType::Ifndef => {
                let mut e = Eater::new(line.clone());
                e.skip_spaces();
                e.check_and_eat_str("!ifndef")?;
                e.skip_spaces();
                let name = e.eat_and_get_varname()?;
                let is_true = mem.get_variable(&name).is_none() && !cx.does_function_exist(&name);
                mem.add_if(ExecutionContextIf::from_value(is_true));
            }
            TLineType::ElseIf => {
                let Some(top) = mem.peek_if_mut() else {
                    return Err(EaterException::new("No if related to this elseif", line));
                };
                top.entering_else_if();
                let burnt = top.has_been_burn();
                if !burnt && self.condition(line, "!elseif", cx, mem)? {
                    mem.peek_if_mut().expect("if").now_in_some_else_if();
                }
            }
            TLineType::Else => {
                let Some(top) = mem.peek_if_mut() else {
                    return Err(EaterException::new("No if related to this else", line));
                };
                top.now_in_else();
            }
            TLineType::EndIf => {
                if mem.poll_if().is_none() {
                    return Err(EaterException::new("No if related to this endif", line));
                }
            }
            _ => unreachable!("not a conditional directive"),
        }
        Ok(())
    }
}

impl CodeIterator for IfLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            let ty = TLineType::of(&result);
            if matches!(
                ty,
                TLineType::If
                    | TLineType::Ifdef
                    | TLineType::Ifndef
                    | TLineType::Else
                    | TLineType::ElseIf
                    | TLineType::EndIf
            ) {
                cx.debug.push(result.clone());
                self.process(&result, ty, cx, mem)?;
                self.next(cx, mem)?;
                continue;
            }
            if mem.peek_if().is_some() && !mem.are_all_if_ok() {
                cx.debug.push(result);
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

struct LegacyDefineLayer {
    source: Source,
}

impl CodeIterator for LegacyDefineLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            match TLineType::of(&result) {
                TLineType::LegacyDefine => {
                    cx.debug.push(result.clone());
                    cx.execute_legacy_define(mem, &result)?;
                }
                TLineType::LegacyDefineLong => {
                    cx.debug.push(result.clone());
                    cx.execute_legacy_define_long(mem, &result)?;
                }
                _ => return Ok(Some(result)),
            }
            self.next(cx, mem)?;
        }
    }

    delegate!();
}

struct WhileLayer {
    source: Source,
}

impl WhileLayer {
    fn condition_value(
        expr: &TokenStack,
        location: &StringLocated,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<TValue> {
        expr.get_result(location, cx, mem)
    }
}

impl CodeIterator for WhileLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        let mut level = 0;
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            let current = mem.peek_while().cloned();
            let ty = TLineType::of(&result);
            if current.as_ref().is_some_and(|c| c.skip_me) {
                if ty == TLineType::While {
                    level += 1;
                } else if ty == TLineType::EndWhile {
                    level -= 1;
                    if level == -1 {
                        mem.poll_while();
                        level = 0;
                    }
                }
                self.next(cx, mem)?;
                continue;
            }
            if ty == TLineType::While {
                cx.debug.push(result.clone());
                let t = result.trimmed();
                let mut e = Eater::new(t.clone());
                e.skip_spaces();
                e.check_and_eat_str("!while")?;
                e.skip_spaces();
                let expression = e.eat_token_stack()?;
                let value = Self::condition_value(&expression, &t, cx, mem)?;
                mem.add_while(ExecutionContextWhile {
                    expression,
                    start: self.source.position(),
                    skip_me: !value.to_boolean(),
                });
                self.next(cx, mem)?;
                continue;
            } else if ty == TLineType::EndWhile {
                cx.debug.push(result.clone());
                let Some(current) = current else {
                    return Err(EaterException::new(
                        "No while related to this endwhile",
                        &result,
                    ));
                };
                let value = Self::condition_value(&current.expression, &result, cx, mem)?;
                if value.to_boolean() {
                    self.source.jump(current.start, &result)?;
                } else {
                    mem.poll_while();
                }
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

struct ForeachLayer {
    source: Source,
}

impl ForeachLayer {
    fn set_loop_variable(
        mem: &mut TMemory,
        foreach: &ExecutionContextForeach,
        position: &StringLocated,
    ) -> Result<()> {
        let first = foreach.current_value();
        mem.put_variable(
            &foreach.varname,
            TValue::Json(first),
            Some(TVariableScope::Global),
            position,
        )
    }
}

impl CodeIterator for ForeachLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        let mut level = 0;
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            let skip = mem.peek_foreach().is_some_and(|f| f.skip_me);
            let ty = TLineType::of(&result);
            if skip {
                if ty == TLineType::Foreach {
                    level += 1;
                } else if ty == TLineType::EndForeach {
                    level -= 1;
                    if level == -1 {
                        mem.poll_foreach();
                        level = 0;
                    }
                }
                self.next(cx, mem)?;
                continue;
            }
            if ty == TLineType::Foreach {
                cx.debug.push(result.clone());
                let s = result.trimmed();
                let mut e = Eater::new(s.clone());
                e.skip_spaces();
                e.check_and_eat_str("!foreach")?;
                e.skip_spaces();
                let varname = e.eat_and_get_varname()?;
                e.skip_spaces();
                e.check_and_eat_str("in")?;
                e.skip_spaces();
                let value = e.eat_expression(cx, mem)?;
                let json = value.to_json().cloned();
                let mut foreach = ExecutionContextForeach::new(
                    varname,
                    json.clone().unwrap_or(JsonValue::Null),
                    self.source.position(),
                );
                if json.as_ref().is_none_or(|j| json_size(j) == 0) {
                    foreach.skip_me = true;
                } else {
                    Self::set_loop_variable(mem, &foreach, &s)?;
                }
                mem.add_foreach(foreach);
                self.next(cx, mem)?;
                continue;
            } else if ty == TLineType::EndForeach {
                cx.debug.push(result.clone());
                let Some(foreach) = mem.peek_foreach_mut() else {
                    return Err(EaterException::new(
                        "No foreach related to this endforeach",
                        &result,
                    ));
                };
                foreach.inc();
                if foreach.skip_me {
                    mem.poll_foreach();
                } else {
                    let f = foreach.clone();
                    Self::set_loop_variable(mem, &f, &result)?;
                    self.source.jump(f.start, &result)?;
                }
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

struct AffectationLayer {
    source: Source,
}

impl AffectationLayer {
    fn execute_affectation(cx: &mut TContext, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let s = s.trimmed();
        let mut e = Eater::new(s.clone());
        e.skip_spaces();
        e.check_and_eat_char('!')?;
        e.skip_spaces();
        let mut varname = e.eat_and_get_varname()?;
        let mut scope = TVariableScope::lazy_parse(&varname);
        if scope.is_some() {
            e.skip_spaces();
            if e.peek_char() == '?' || e.peek_char() == '=' {
                scope = None;
            } else {
                varname = e.eat_and_get_varname()?;
            }
        }
        e.skip_spaces();
        let mut conditional = false;
        if e.peek_char() == '?' {
            e.check_and_eat_char('?')?;
            conditional = true;
        }
        e.check_and_eat_char('=')?;
        if conditional && mem.get_variable(&varname).is_some() {
            return Ok(());
        }
        e.skip_spaces();
        let value = e.eat_expression(cx, mem)?;
        mem.put_variable(&varname, value, scope, &s)
    }

    fn do_affectation(
        &mut self,
        mut result: StringLocated,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<()> {
        let mut last_location: isize = -1;
        for _ in 0..9999 {
            match Self::execute_affectation(cx, mem, &result) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let Some(col) = e.json_column else {
                        return Err(e);
                    };
                    if col as isize <= last_location {
                        return Err(EaterException::new("Error in JSON format", &result));
                    }
                    last_location = col as isize;
                    self.next(cx, mem)?;
                    let Some(forward) = self.source.peek(cx, mem)? else {
                        return Err(EaterException::new("Error in JSON format", &result));
                    };
                    result = result.append(forward.string());
                }
            }
        }
        Ok(())
    }
}

impl CodeIterator for AffectationLayer {
    fn peek(&mut self, cx: &mut TContext, mem: &mut TMemory) -> Result<Option<StringLocated>> {
        loop {
            let Some(result) = self.source.peek(cx, mem)? else {
                return Ok(None);
            };
            if TLineType::of(&result) == TLineType::Affectation {
                cx.debug.push(result.clone());
                self.do_affectation(result, cx, mem)?;
                self.next(cx, mem)?;
                continue;
            }
            return Ok(Some(result));
        }
    }

    delegate!();
}

/// The complete chain (`TContext.buildCodeIterator`).
pub fn build_code_iterator(body: Vec<StringLocated>) -> Box<dyn CodeIterator> {
    let it: Source = Box::new(CodeIteratorImpl::new(body));
    let it: Source = Box::new(LongComment { source: it });
    let it: Source = Box::new(ShortComment { source: it });
    let it: Source = Box::new(InnerComment { source: it });
    let it: Source = Box::new(SubLayer {
        source: it,
        reading: None,
    });
    let it: Source = Box::new(ReturnFunctionLayer { source: it });
    let it: Source = Box::new(ProcedureLayer { source: it });
    let it: Source = Box::new(IfLayer { source: it });
    let it: Source = Box::new(LegacyDefineLayer { source: it });
    let it: Source = Box::new(WhileLayer { source: it });
    let it: Source = Box::new(ForeachLayer { source: it });
    Box::new(AffectationLayer { source: it })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_expressions() {
        let truth = |n: &str| n == "A" || n == "$b";
        assert!(eval_boolean("A", &truth));
        assert!(!eval_boolean("C", &truth));
        assert!(eval_boolean("A && $b", &truth));
        assert!(!eval_boolean("A && C", &truth));
        assert!(eval_boolean("A || C", &truth));
        assert!(eval_boolean("!C", &truth));
        assert!(eval_boolean("(C || A) && !C", &truth));
    }

    #[test]
    fn inner_comments() {
        let r = |s: &str| {
            remove_inner_comment(&StringLocated::new(s, None))
                .string()
                .to_string()
        };
        assert_eq!(r("/' c '/ A -> B"), " A -> B");
        assert_eq!(r("A -> B /' c '/"), "A -> B ");
        assert_eq!(r("A -> B"), "A -> B");
        assert_eq!(r("x /'''id'''/ y /'''-'''/ z"), "x  y  z");
    }
}
