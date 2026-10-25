//! Variables and the stacks of open `!if`, `!while` and `!foreach` (`TMemory`).

use super::expr::{TValue, TokenStack};
use super::trie::Trie;
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::text::StringLocated;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TVariableScope {
    Local,
    Global,
}

impl TVariableScope {
    pub fn lazy_parse(value: &str) -> Option<TVariableScope> {
        if value.eq_ignore_ascii_case("local") {
            Some(TVariableScope::Local)
        } else if value.eq_ignore_ascii_case("global") {
            Some(TVariableScope::Global)
        } else {
            None
        }
    }
}

/// State of an open `!if` (`ExecutionContextIf`).
#[derive(Debug, Clone)]
pub struct ExecutionContextIf {
    is_true: bool,
    has_been_burn: bool,
}

impl ExecutionContextIf {
    pub fn from_value(is_true: bool) -> Self {
        Self {
            is_true,
            has_been_burn: is_true,
        }
    }

    pub fn condition_is_ok_here(&self) -> bool {
        self.is_true
    }

    pub fn entering_else_if(&mut self) {
        self.is_true = false;
    }

    pub fn now_in_else(&mut self) {
        self.is_true = !self.has_been_burn;
    }

    pub fn now_in_some_else_if(&mut self) {
        self.is_true = true;
        self.has_been_burn = true;
    }

    pub fn has_been_burn(&self) -> bool {
        self.has_been_burn
    }
}

/// State of an open `!while` (`ExecutionContextWhile`).
#[derive(Debug, Clone)]
pub struct ExecutionContextWhile {
    pub expression: TokenStack,
    pub start: usize,
    pub skip_me: bool,
}

/// State of an open `!foreach` (`ExecutionContextForeach`).
#[derive(Debug, Clone)]
pub struct ExecutionContextForeach {
    pub varname: String,
    pub json: JsonValue,
    pub start: usize,
    pub skip_me: bool,
    current_index: usize,
}

impl ExecutionContextForeach {
    pub fn new(varname: String, json: JsonValue, start: usize) -> Self {
        Self {
            varname,
            json,
            start,
            skip_me: false,
            current_index: 0,
        }
    }

    pub fn current_value(&self) -> JsonValue {
        match &self.json {
            JsonValue::Array(a) => a[self.current_index].clone(),
            JsonValue::Object(o) => JsonValue::from_str(&o[self.current_index].0),
            _ => panic!("foreach over a scalar"),
        }
    }

    pub fn inc(&mut self) {
        self.current_index += 1;
        if self.current_index >= json_size(&self.json) {
            self.skip_me = true;
        }
    }
}

pub fn json_size(j: &JsonValue) -> usize {
    match j {
        JsonValue::Array(a) => a.len(),
        JsonValue::Object(o) => o.len(),
        _ => 0,
    }
}

#[derive(Default)]
struct Global {
    vars: HashMap<String, TValue>,
    trie: Trie,
}

#[derive(Default)]
struct Local {
    overridden: HashMap<String, TValue>,
    overridden_trie: Trie,
    locals: HashMap<String, TValue>,
    locals_trie: Trie,
}

#[derive(Default)]
struct Contexts {
    ifs: Vec<ExecutionContextIf>,
    whiles: Vec<ExecutionContextWhile>,
    foreachs: Vec<ExecutionContextForeach>,
}

/// Variables visible to a piece of code, plus its control-flow stacks.
pub struct TMemory {
    global: Rc<RefCell<Global>>,
    local: Option<Local>,
    ctx: Contexts,
}

impl Default for TMemory {
    fn default() -> Self {
        Self::new_global()
    }
}

impl TMemory {
    pub fn new_global() -> Self {
        Self {
            global: Rc::new(RefCell::new(Global::default())),
            local: None,
            ctx: Contexts::default(),
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<TValue> {
        if let Some(l) = &self.local {
            if let Some(v) = l.overridden.get(name) {
                return Some(v.clone());
            }
            if let Some(v) = self.global.borrow().vars.get(name) {
                return Some(v.clone());
            }
            return l.locals.get(name).cloned();
        }
        self.global.borrow().vars.get(name).cloned()
    }

    fn put_global(&self, name: &str, value: TValue) {
        let mut g = self.global.borrow_mut();
        g.vars.insert(name.to_string(), value);
        g.trie.add(name);
    }

    pub fn put_variable(
        &mut self,
        name: &str,
        value: TValue,
        scope: Option<TVariableScope>,
        location: &StringLocated,
    ) -> Result<()> {
        log::debug!("[Mem] setting {name}");
        let Some(local) = &mut self.local else {
            if scope == Some(TVariableScope::Local) {
                return Err(EaterException::new(
                    "Cannot use local variable here",
                    location,
                ));
            }
            self.put_global(name, value);
            return Ok(());
        };
        if scope == Some(TVariableScope::Global) {
            self.put_global(name, value);
            return Ok(());
        }
        if scope == Some(TVariableScope::Local) || local.overridden.contains_key(name) {
            local.overridden.insert(name.to_string(), value);
            local.overridden_trie.add(name);
        } else if self.global.borrow().vars.contains_key(name) {
            self.put_global(name, value);
        } else {
            local.locals.insert(name.to_string(), value);
            local.locals_trie.add(name);
        }
        Ok(())
    }

    pub fn remove_variable(&mut self, name: &str) {
        if let Some(local) = &mut self.local {
            if local.overridden.remove(name).is_some() {
                local.overridden_trie.remove(name);
                return;
            }
            if !self.global.borrow().vars.contains_key(name) {
                local.locals.remove(name);
                local.locals_trie.remove(name);
                return;
            }
        }
        let mut g = self.global.borrow_mut();
        g.vars.remove(name);
        g.trie.remove(name);
    }

    /// Longest variable name at `pos` of `s` (`variablesNames3().getLonguestMatchStartingIn`).
    pub fn longest_variable_at(&self, s: &[char], pos: usize) -> String {
        let s1 = self.global.borrow().trie.longest_match_starting_in(s, pos);
        let Some(l) = &self.local else {
            return s1;
        };
        let s2 = l.overridden_trie.longest_match_starting_in(s, pos);
        let s3 = l.locals_trie.longest_match_starting_in(s, pos);
        let (n1, n2, n3) = (s1.chars().count(), s2.chars().count(), s3.chars().count());
        if n1 >= n2 && n1 >= n3 {
            s1
        } else if n2 >= n3 && n2 >= n1 {
            s2
        } else {
            s3
        }
    }

    /// A memory for a function call: global variables plus the given arguments.
    pub fn fork_from_global(&self, input: HashMap<String, TValue>) -> TMemory {
        let mut local = Local::default();
        for name in input.keys() {
            local.overridden_trie.add(name);
        }
        local.overridden = input;
        TMemory {
            global: self.global.clone(),
            local: Some(local),
            ctx: Contexts::default(),
        }
    }

    pub fn dump_debug(&self, message: &str) {
        log::info!("[Mem] Start of memory_dump {message}");
        let g = self.global.borrow();
        let mut names: Vec<_> = g.vars.iter().collect();
        names.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in names {
            log::info!("[Mem] {k} = {v}");
        }
        log::info!("[Mem] End of memory_dump");
    }

    pub fn peek_if(&self) -> Option<&ExecutionContextIf> {
        self.ctx.ifs.last()
    }

    pub fn peek_if_mut(&mut self) -> Option<&mut ExecutionContextIf> {
        self.ctx.ifs.last_mut()
    }

    pub fn add_if(&mut self, c: ExecutionContextIf) {
        self.ctx.ifs.push(c);
    }

    pub fn poll_if(&mut self) -> Option<ExecutionContextIf> {
        self.ctx.ifs.pop()
    }

    pub fn are_all_if_ok(&self) -> bool {
        self.ctx
            .ifs
            .iter()
            .all(ExecutionContextIf::condition_is_ok_here)
    }

    pub fn add_while(&mut self, c: ExecutionContextWhile) {
        self.ctx.whiles.push(c);
    }

    pub fn peek_while(&self) -> Option<&ExecutionContextWhile> {
        self.ctx.whiles.last()
    }

    pub fn poll_while(&mut self) -> Option<ExecutionContextWhile> {
        self.ctx.whiles.pop()
    }

    pub fn add_foreach(&mut self, c: ExecutionContextForeach) {
        self.ctx.foreachs.push(c);
    }

    pub fn peek_foreach(&self) -> Option<&ExecutionContextForeach> {
        self.ctx.foreachs.last()
    }

    pub fn peek_foreach_mut(&mut self) -> Option<&mut ExecutionContextForeach> {
        self.ctx.foreachs.last_mut()
    }

    pub fn poll_foreach(&mut self) -> Option<ExecutionContextForeach> {
        self.ctx.foreachs.pop()
    }
}
