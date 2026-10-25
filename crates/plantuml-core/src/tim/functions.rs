//! Functions and procedures (`TFunction`, `TFunctionImpl`, `FunctionsSet`).

use super::context::TContext;
use super::expr::TValue;
use super::line_type::TLineType;
use super::memory::TMemory;
use super::trie::Trie;
use super::{EaterException, Result};
use crate::text::StringLocated;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TFunctionType {
    Procedure,
    ReturnFunction,
    LegacyDefine,
    LegacyDefineLong,
}

impl TFunctionType {
    pub fn is_legacy(self) -> bool {
        matches!(
            self,
            TFunctionType::LegacyDefine | TFunctionType::LegacyDefineLong
        )
    }
}

/// Name and number of arguments; named arguments are not part of the identity.
#[derive(Debug, Clone)]
pub struct TFunctionSignature {
    pub function_name: String,
    pub nb_arg: usize,
    pub named_arguments: HashSet<String>,
}

impl TFunctionSignature {
    pub fn new(name: &str, nb_arg: usize) -> Self {
        Self::with_named(name, nb_arg, HashSet::new())
    }

    pub fn with_named(name: &str, nb_arg: usize, named: HashSet<String>) -> Self {
        Self {
            function_name: name.to_string(),
            nb_arg,
            named_arguments: named,
        }
    }

    pub fn same_function_name_as(&self, other: &TFunctionSignature) -> bool {
        self.function_name == other.function_name
    }
}

impl PartialEq for TFunctionSignature {
    fn eq(&self, other: &Self) -> bool {
        self.function_name == other.function_name && self.nb_arg == other.nb_arg
    }
}

impl Eq for TFunctionSignature {}

impl Hash for TFunctionSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.function_name.hash(state);
        self.nb_arg.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct TFunctionArgument {
    pub name: String,
    pub default: Option<TValue>,
}

impl TFunctionArgument {
    pub fn new(name: &str, default: Option<TValue>) -> Self {
        Self {
            name: name.to_string(),
            default,
        }
    }
}

pub trait TFunction {
    fn signature(&self) -> &TFunctionSignature;
    fn can_cover(&self, nb_arg: usize, named: &HashSet<String>) -> bool;
    fn function_type(&self) -> TFunctionType;
    fn execute_return(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        location: &StringLocated,
        args: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> Result<TValue>;
    fn execute_procedure(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        location: &StringLocated,
        args: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> Result<()>;
    fn is_unquoted(&self) -> bool;
}

/// A function declared in the source (`TFunctionImpl`).
pub struct TFunctionImpl {
    signature: TFunctionSignature,
    args: Vec<TFunctionArgument>,
    body: RefCell<Vec<StringLocated>>,
    unquoted: bool,
    ftype: Cell<TFunctionType>,
    legacy_definition: RefCell<Option<String>>,
    contains_return: Cell<bool>,
}

impl TFunctionImpl {
    pub fn new(
        name: &str,
        args: Vec<TFunctionArgument>,
        unquoted: bool,
        ftype: TFunctionType,
    ) -> Self {
        let names: HashSet<String> = args.iter().map(|a| a.name.clone()).collect();
        Self {
            signature: TFunctionSignature::with_named(name, args.len(), names),
            args,
            body: RefCell::new(Vec::new()),
            unquoted,
            ftype: Cell::new(ftype),
            legacy_definition: RefCell::new(None),
            contains_return: Cell::new(false),
        }
    }

    pub fn add_body(&self, s: StringLocated) -> Result<()> {
        if TLineType::of(&s) == TLineType::Return {
            self.contains_return.set(true);
            if self.ftype.get() == TFunctionType::Procedure {
                return Err(EaterException::new(
                    "A procedure cannot have !return directive. Declare it as a function instead ?",
                    &s,
                ));
            }
        }
        self.body.borrow_mut().push(s);
        Ok(())
    }

    pub fn set_legacy_definition(&self, def: String) {
        *self.legacy_definition.borrow_mut() = Some(def);
    }

    pub fn has_body(&self) -> bool {
        !self.body.borrow().is_empty()
    }

    pub fn does_contain_return(&self) -> bool {
        self.contains_return.get()
    }

    pub fn finalize_end_definelong(&self) {
        assert_eq!(self.ftype.get(), TFunctionType::LegacyDefineLong);
        let body = self.body.borrow();
        if body.len() == 1 {
            self.ftype.set(TFunctionType::LegacyDefine);
            *self.legacy_definition.borrow_mut() = Some(body[0].string().to_string());
        }
    }

    fn new_memory(
        &self,
        mem: &TMemory,
        values: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> TMemory {
        let mut result = HashMap::new();
        let mut ivalue = 0;
        for arg in &self.args {
            let value = if let Some(v) = named.get(&arg.name) {
                v.clone()
            } else if ivalue < values.len() {
                ivalue += 1;
                values[ivalue - 1].clone()
            } else {
                arg.default.clone().expect("argument value")
            };
            result.insert(arg.name.clone(), value);
        }
        mem.fork_from_global(result)
    }
}

impl TFunction for TFunctionImpl {
    fn signature(&self) -> &TFunctionSignature {
        &self.signature
    }

    fn can_cover(&self, nb_arg: usize, named: &HashSet<String>) -> bool {
        if named
            .iter()
            .any(|n| !self.signature.named_arguments.contains(n))
        {
            return false;
        }
        if nb_arg > self.args.len() {
            return false;
        }
        let needed = self
            .args
            .iter()
            .filter(|a| !named.contains(&a.name) && a.default.is_none())
            .count();
        nb_arg >= needed
    }

    fn function_type(&self) -> TFunctionType {
        self.ftype.get()
    }

    fn execute_return(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        location: &StringLocated,
        args: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> Result<TValue> {
        if self.ftype.get() == TFunctionType::LegacyDefine {
            let definition = self
                .legacy_definition
                .borrow()
                .clone()
                .expect("legacy definition");
            let mut copy = self.new_memory(mem, args, &HashMap::new());
            let tmp = cx.apply_functions_and_variables(
                &mut copy,
                &StringLocated::new(definition, location.location()),
            )?;
            return Ok(TValue::Str(tmp.unwrap_or_default()));
        }
        if self.ftype.get() != TFunctionType::ReturnFunction {
            return Err(EaterException::new(
                "Illegal call here. Is there a return directive in your function?",
                location,
            ));
        }
        let mut copy = self.new_memory(mem, args, named);
        let body = self.body.borrow().clone();
        match cx.execute_lines(&mut copy, &body, Some(TFunctionType::ReturnFunction), true)? {
            Some(v) => Ok(v),
            None => Err(EaterException::new(
                "No return directive found in your function",
                location,
            )),
        }
    }

    fn execute_procedure(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        _location: &StringLocated,
        args: &[TValue],
        named: &HashMap<String, TValue>,
    ) -> Result<()> {
        assert!(matches!(
            self.ftype.get(),
            TFunctionType::Procedure | TFunctionType::LegacyDefineLong
        ));
        let mut copy = self.new_memory(mem, args, named);
        let body = self.body.borrow().clone();
        cx.execute_lines(&mut copy, &body, Some(TFunctionType::Procedure), false)?;
        Ok(())
    }

    fn is_unquoted(&self) -> bool {
        self.unquoted
    }
}

/// All known functions, with a fallback set of built-ins (`FunctionsSet`).
#[derive(Default)]
pub struct FunctionsSet {
    base: Option<Rc<FunctionsSet>>,
    functions: Vec<(TFunctionSignature, Rc<dyn TFunction>)>,
    by_name: HashMap<String, Vec<Rc<dyn TFunction>>>,
    pub functions_final: HashSet<TFunctionSignature>,
    trie: Trie,
    pub pending: Option<Rc<TFunctionImpl>>,
}

impl FunctionsSet {
    pub fn new(base: Option<Rc<FunctionsSet>>) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn get_exact(&self, searched: &TFunctionSignature) -> Option<Rc<dyn TFunction>> {
        if let Some((_, f)) = self.functions.iter().find(|(s, _)| s == searched) {
            return Some(f.clone());
        }
        self.base.as_ref().and_then(|b| b.get_exact(searched))
    }

    pub fn function_smart(&self, searched: &TFunctionSignature) -> Option<Rc<dyn TFunction>> {
        self.get_exact(searched)
            .or_else(|| self.search_covering(searched))
    }

    fn is_covering(candidate: &Rc<dyn TFunction>, searched: &TFunctionSignature) -> bool {
        candidate.signature().same_function_name_as(searched)
            && candidate.can_cover(searched.nb_arg, &searched.named_arguments)
    }

    fn search_covering(&self, searched: &TFunctionSignature) -> Option<Rc<dyn TFunction>> {
        for (_, c) in &self.functions {
            if Self::is_covering(c, searched) {
                return Some(c.clone());
            }
        }
        let base = self.base.as_ref()?;
        for (sig, c) in &base.functions {
            if !self.functions.iter().any(|(s, _)| s == sig) && Self::is_covering(c, searched) {
                return Some(c.clone());
            }
        }
        None
    }

    pub fn longest_match_starting_in(&self, s: &[char], pos: usize) -> String {
        let result = self.trie.longest_match_starting_in(s, pos);
        match &self.base {
            Some(b) if result.is_empty() => b.longest_match_starting_in(s, pos),
            _ => result,
        }
    }

    pub fn add_function(&mut self, func: Rc<dyn TFunction>) {
        let sig = func.signature().clone();
        match self.functions.iter_mut().find(|(s, _)| *s == sig) {
            Some(slot) => slot.1 = func.clone(),
            None => self.functions.push((sig.clone(), func.clone())),
        }
        self.trie.add(&format!("{}(", sig.function_name));
        let list = self.by_name.entry(sig.function_name.clone()).or_default();
        list.retain(|f| f.signature() != &sig);
        list.push(func);
    }

    pub fn does_function_exist(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
            || self
                .base
                .as_ref()
                .is_some_and(|b| b.does_function_exist(name))
    }

    pub fn functions_by_name(&self, name: &str) -> Vec<Rc<dyn TFunction>> {
        let mut result: Vec<Rc<dyn TFunction>> =
            self.by_name.get(name).cloned().unwrap_or_default();
        if let Some(b) = &self.base {
            for f in b.functions_by_name(name) {
                if !result.iter().any(|r| r.signature() == f.signature()) {
                    result.push(f);
                }
            }
        }
        result
    }
}
