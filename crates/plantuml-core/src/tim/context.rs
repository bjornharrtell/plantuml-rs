//! The preprocessor engine (`TContext`).

use super::eater::{Eater, FunctionCall};
use super::expr::TValue;
use super::functions::{FunctionsSet, TFunction, TFunctionSignature, TFunctionType};
use super::import::ImportedArchive;
use super::iterator::build_code_iterator;
use super::line_type::TLineType;
use super::memory::{TMemory, TVariableScope};
use super::resolver::{NoResolver, ResolvedFile, Resolver};
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::text::StringLocated;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Options set by `!option` (`OptionKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionKey {
    Language,
    UseDescriptiveNames,
    Handwritten,
    Debug,
    SvgDesc,
    SvgTitle,
}

impl OptionKey {
    pub const ALL: [OptionKey; 6] = [
        OptionKey::Language,
        OptionKey::UseDescriptiveNames,
        OptionKey::Handwritten,
        OptionKey::Debug,
        OptionKey::SvgDesc,
        OptionKey::SvgTitle,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OptionKey::Language => "LANGUAGE",
            OptionKey::UseDescriptiveNames => "USE_DESCRIPTIVE_NAMES",
            OptionKey::Handwritten => "HANDWRITTEN",
            OptionKey::Debug => "DEBUG",
            OptionKey::SvgDesc => "SVG_DESC",
            OptionKey::SvgTitle => "SVG_TITLE",
        }
    }

    pub fn default_value(self) -> Option<&'static str> {
        match self {
            OptionKey::Handwritten | OptionKey::Debug => Some("true"),
            _ => None,
        }
    }

    fn simplify(s: &str) -> String {
        s.chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    pub fn lazy_from(s: &str) -> Option<OptionKey> {
        let wanted = Self::simplify(s);
        Self::ALL
            .into_iter()
            .find(|k| Self::simplify(k.name()) == wanted)
    }
}

/// What the preprocessor leaves for the diagram: warnings and options.
#[derive(Debug, Clone, Default)]
pub struct PreprocessingArtifact {
    pub warnings: Vec<Vec<String>>,
    pub options: HashMap<OptionKey, String>,
}

impl PreprocessingArtifact {
    pub fn add_warning(&mut self, w: Vec<String>) {
        if !self.warnings.contains(&w) {
            self.warnings.push(w);
        }
    }
}

/// Lines of a `!startsub` block (`Sub`).
#[derive(Debug, Clone, Default)]
pub struct Sub {
    pub name: String,
    pub lines: Vec<StringLocated>,
}

impl Sub {
    /// Extracts the block `blocname` of a file (`Sub.fromFile`).
    pub fn from_file(
        lines: &[StringLocated],
        blocname: &str,
        cx: &mut TContext,
        mem: &mut TMemory,
    ) -> Result<Option<Sub>> {
        let mut result: Option<Sub> = None;
        let mut skip = false;
        for s in lines {
            let trimmed = s.trimmed();
            let ty = TLineType::of(&trimmed);
            if ty == TLineType::StartSub {
                let name = analyze_startsub(&trimmed, cx, mem)?;
                if name == blocname {
                    skip = false;
                    if result.is_none() {
                        result = Some(Sub {
                            name: blocname.to_string(),
                            lines: Vec::new(),
                        });
                    }
                }
                continue;
            }
            if ty == TLineType::EndSub && result.is_some() {
                skip = true;
            }
            if let (Some(r), false) = (&mut result, skip) {
                r.lines.push(s.clone());
            }
        }
        Ok(result)
    }
}

pub fn analyze_startsub(
    s: &StringLocated,
    _cx: &mut TContext,
    _mem: &mut TMemory,
) -> Result<String> {
    let mut e = Eater::new(s.clone());
    e.skip_spaces();
    e.check_and_eat_str("!startsub")?;
    e.skip_spaces();
    let name = e.eat_all_to_end();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(EaterException::new("Bad sub name", s));
    }
    Ok(name)
}

pub struct TContext {
    pub result_list: Vec<StringLocated>,
    pub debug: Vec<StringLocated>,
    pub functions: FunctionsSet,
    pub environment: HashMap<String, String>,
    pub subs: HashMap<String, Sub>,
    pub resolver: Rc<dyn Resolver>,
    pub current_dir: Option<String>,
    /// Index in `imports` of the archive the current file came from.
    current_import: Option<usize>,
    /// Archives made available by `!import` (`PathSystem`'s imported folders).
    imports: Vec<ImportedArchive>,
    included_current: HashSet<(String, Option<String>)>,
    pub artifact: PreprocessingArtifact,
    pending_add: Option<String>,
    pub theme_metadata: JsonValue,
    /// Definitions of `@startdef` blocks, by name (`DefinitionsContainer`).
    pub definitions: HashMap<String, Vec<String>>,
    pub rng_state: u64,
}

impl TContext {
    pub fn new(environment: HashMap<String, String>, resolver: Rc<dyn Resolver>) -> Self {
        Self {
            result_list: Vec::new(),
            debug: Vec::new(),
            functions: FunctionsSet::new(Some(super::builtin::standard_functions())),
            environment,
            subs: HashMap::new(),
            resolver,
            current_dir: None,
            current_import: None,
            imports: Vec::new(),
            included_current: HashSet::new(),
            artifact: PreprocessingArtifact::default(),
            pending_add: None,
            theme_metadata: JsonValue::Object(Vec::new()),
            definitions: HashMap::new(),
            rng_state: 0,
        }
    }

    pub fn without_resolver(environment: HashMap<String, String>) -> Self {
        Self::new(environment, Rc::new(NoResolver))
    }

    pub fn environment_value(&self, key: &str) -> Option<&str> {
        self.environment.get(key).map(String::as_str)
    }

    /// Value of a variable name that may contain `.` or `[` (JSON access) (`asKnowledge`).
    pub fn knowledge_variable(
        &mut self,
        mem: &mut TMemory,
        name: &str,
        location: &StringLocated,
    ) -> Result<Option<TValue>> {
        if name.contains('.') || name.contains('[') {
            let r = self
                .apply_functions_and_variables(mem, &StringLocated::new(name, location.location()))?
                .unwrap_or_default();
            return Ok(Some(match JsonValue::parse(&r) {
                Ok(j) => TValue::Json(j),
                Err(_) => TValue::Str(r),
            }));
        }
        Ok(mem.get_variable(name))
    }

    pub fn execute_lines(
        &mut self,
        mem: &mut TMemory,
        body: &[StringLocated],
        ftype: Option<TFunctionType>,
        mode_special: bool,
    ) -> Result<Option<TValue>> {
        let mut it = build_code_iterator(body.to_vec());
        while let Some(s) = it.peek(self, mem)? {
            if let Some(result) = self.execute_one_line(mem, &s, ftype, mode_special)? {
                return Ok(Some(result));
            }
            it.next(self, mem)?;
        }
        Ok(None)
    }

    fn execute_lines_internal(
        &mut self,
        mem: &mut TMemory,
        body: &[StringLocated],
        ftype: Option<TFunctionType>,
    ) -> Result<()> {
        let mut it = build_code_iterator(body.to_vec());
        while let Some(s) = it.peek(self, mem)? {
            self.execute_one_line(mem, &s, ftype, false)?;
            it.next(self, mem)?;
        }
        Ok(())
    }

    fn execute_one_line(
        &mut self,
        mem: &mut TMemory,
        s: &StringLocated,
        ftype: Option<TFunctionType>,
        mode_special: bool,
    ) -> Result<Option<TValue>> {
        self.debug.push(s.clone());
        let ty = TLineType::of(s);
        let is_return_fn = ftype == Some(TFunctionType::ReturnFunction);
        match ty {
            TLineType::IncludeSub => self.execute_includesub(mem, s)?,
            TLineType::Theme => self.execute_theme(mem, s)?,
            TLineType::Include => self.execute_include(mem, s)?,
            TLineType::IncludeDef => self.execute_include_def(mem, s)?,
            TLineType::Import => self.execute_import(mem, s)?,
            TLineType::DumpMemory => {
                let mut e = Eater::new(s.trimmed());
                e.skip_spaces();
                e.check_and_eat_str("!dump_memory")?;
                e.skip_spaces();
                mem.dump_debug(&e.eat_all_to_end());
            }
            TLineType::Assert => self.execute_assert(mem, &s.trimmed())?,
            TLineType::Option => self.execute_option(mem, &s.trimmed())?,
            TLineType::Undef => {
                let mut e = Eater::new(s.trimmed());
                e.skip_spaces();
                e.check_and_eat_str("!undef")?;
                e.skip_spaces();
                let name = e.eat_and_get_varname()?;
                mem.remove_variable(&name);
            }
            TLineType::Plain if !is_return_fn => self.add_plain(mem, s)?,
            TLineType::Return if is_return_fn => {
                if mode_special {
                    let mut e = Eater::new(s.clone());
                    e.skip_spaces();
                    e.check_and_eat_str("!return")?;
                    e.skip_spaces();
                    return Ok(Some(e.eat_expression(self, mem)?));
                }
                return Ok(None);
            }
            TLineType::Plain if is_return_fn => {
                self.apply_functions_and_variables_internal(mem, s)?;
            }
            TLineType::AffectationDefine => self.execute_affectation_define(mem, s)?,
            TLineType::EndFunction if ftype.is_none() => {}
            TLineType::Log => {
                let mut e = Eater::new(s.trimmed());
                e.skip_spaces();
                e.check_and_eat_str("!log")?;
                e.skip_spaces();
                let data = self
                    .apply_functions_and_variables(
                        mem,
                        &StringLocated::new(e.eat_all_to_end(), s.location()),
                    )?
                    .unwrap_or_default();
                log::info!("[Log] {data}");
            }
            _ if !s.string().is_empty() && s.string().chars().all(char::is_whitespace) => {}
            _ => {
                return Err(EaterException::new(
                    format!("Compile Error {ftype:?} {ty:?}"),
                    s,
                ))
            }
        }
        Ok(None)
    }

    fn add_plain(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        if let Some(mut tmp) = self.apply_functions_and_variables_internal(mem, s)? {
            if let Some(pending) = self.pending_add.take() {
                tmp[0] =
                    StringLocated::new(format!("{pending}{}", tmp[0].string()), tmp[0].location());
            }
            self.result_list.extend(tmp);
        }
        Ok(())
    }

    fn execute_affectation_define(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let t = s.trimmed();
        let mut e = Eater::new(t.clone());
        e.skip_spaces();
        e.check_and_eat_str("!define")?;
        e.skip_spaces();
        let varname = e.eat_and_get_varname()?;
        e.skip_spaces();
        let rest = e.eat_all_to_end();
        let value = self
            .apply_functions_and_variables(mem, &StringLocated::new(rest, t.location()))?
            .unwrap_or_default();
        mem.put_variable(
            &varname,
            TValue::Str(value),
            Some(TVariableScope::Global),
            &t,
        )
    }

    fn execute_assert(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let mut e = Eater::new(s.clone());
        e.skip_spaces();
        e.check_and_eat_str("!assert")?;
        e.skip_spaces();
        let value = e.eat_expression_stop_at_colon(self, mem)?;
        e.skip_spaces();
        if !value.to_boolean() {
            if e.peek_char() == ':' {
                e.check_and_eat_char(':')?;
                let message = e.eat_expression(self, mem)?;
                return Err(EaterException::new(
                    format!("Assertion error : {message}"),
                    s,
                ));
            }
            return Err(EaterException::new("Assertion error", s));
        }
        Ok(())
    }

    fn execute_option(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let mut e = Eater::new(s.clone());
        e.skip_spaces();
        e.check_and_eat_str("!option")?;
        e.skip_spaces();
        let key = e.eat_and_get_varname()?;
        e.skip_spaces();
        let value = e.eat_expression(self, mem).ok();
        let option = OptionKey::lazy_from(&key);
        match option {
            None => {
                let known: Vec<String> = OptionKey::ALL
                    .iter()
                    .map(|k| k.name().to_lowercase())
                    .collect();
                self.artifact.add_warning(vec![
                    format!("Unknown '!option {key}': this line is ignored"),
                    format!("Known options are: {}", known.join(", ")),
                ]);
            }
            Some(k) => match (value, k.default_value()) {
                (None, None) => self.artifact.add_warning(vec![format!(
                    "'!option {key}' needs a value: write '!option {key} <value>'"
                )]),
                (None, Some(d)) => {
                    self.artifact.options.insert(k, d.to_string());
                }
                (Some(v), _) => {
                    self.artifact.options.insert(k, v.to_string());
                }
            },
        }
        Ok(())
    }

    fn apply_functions_and_variables_internal(
        &mut self,
        mem: &mut TMemory,
        located: &StringLocated,
    ) -> Result<Option<Vec<StringLocated>>> {
        let Some(result) = self.apply_functions_and_variables(mem, located)? else {
            return Ok(None);
        };
        Ok(Some(
            result
                .split('\n')
                .map(|p| StringLocated::new(p, located.location()))
                .collect(),
        ))
    }

    /// Replaces variables and calls in a line; `None` when a procedure took over the line.
    pub fn apply_functions_and_variables(
        &mut self,
        mem: &mut TMemory,
        text: &StringLocated,
    ) -> Result<Option<String>> {
        let chars: Vec<char> = text.string().chars().collect();
        let mut result = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if let Some(present) = self.function_name_at(&chars, i) {
                let sub: String = chars[i..].iter().collect();
                let call = FunctionCall::analyze(
                    StringLocated::new(sub, text.location()),
                    self.is_legacy_define(&present),
                    self.is_unquoted(&present),
                    self,
                    mem,
                )?;
                let signature = TFunctionSignature::with_named(
                    &present,
                    call.values.len(),
                    call.named.keys().cloned().collect(),
                );
                let function = self.functions.function_smart(&signature).ok_or_else(|| {
                    EaterException::new(format!("Function not found {present}"), text)
                })?;
                match function.function_type() {
                    TFunctionType::Procedure => {
                        self.pending_add = Some(result.clone());
                        function.execute_procedure(self, mem, text, &call.values, &call.named)?;
                        i += call.position;
                        let remaining: String = chars[i.min(chars.len())..].iter().collect();
                        if !remaining.is_empty() {
                            self.append_to_last_result(&remaining);
                        }
                        return Ok(None);
                    }
                    TFunctionType::LegacyDefineLong => {
                        self.pending_add = Some(chars[..i].iter().collect());
                        function.execute_procedure(self, mem, text, &call.values, &call.named)?;
                        return Ok(None);
                    }
                    _ => {
                        let r =
                            function.execute_return(self, mem, text, &call.values, &call.named)?;
                        result.push_str(&r.to_string());
                        i += call.position.saturating_sub(1);
                    }
                }
            } else if let Some(var) = self.varname_at(mem, &chars, i) {
                i = self.replace_variables(mem, &chars, i, &var, &mut result, text)?;
            } else {
                result.push(c);
            }
            i += 1;
        }
        Ok(Some(result))
    }

    fn append_to_last_result(&mut self, remaining: &str) {
        if let Some(last) = self.result_list.last_mut() {
            *last = last.append(remaining);
        }
    }

    fn just_after_backslash_n(s: &[char], pos: usize) -> bool {
        pos > 1 && s[pos - 2] == '\\' && s[pos - 1] == 'n'
    }

    fn function_name_at(&self, s: &[char], pos: usize) -> Option<String> {
        let just_after_letter = pos > 0
            && TLineType::is_letter_or_emoji_or_underscore_or_digit(s[pos - 1])
            && !Self::just_after_backslash_n(s, pos);
        if just_after_letter && s[pos] != '%' && s[pos] != '$' {
            return None;
        }
        let fname = self.functions.longest_match_starting_in(s, pos);
        if fname.is_empty() {
            return None;
        }
        let mut n = fname.chars();
        n.next_back();
        Some(n.as_str().to_string())
    }

    fn varname_at(&self, mem: &TMemory, s: &[char], pos: usize) -> Option<String> {
        let just_after_letter = pos > 0
            && TLineType::is_letter_or_emoji_or_underscore_or_digit(s[pos - 1])
            && !Self::just_after_backslash_n(s, pos);
        if just_after_letter && s[pos] != '$' {
            return None;
        }
        let varname = mem.longest_variable_at(s, pos);
        if varname.is_empty() {
            return None;
        }
        let end = pos + varname.chars().count();
        if end == s.len() || !TLineType::is_letter_or_emoji_or_underscore_or_digit(s[end]) {
            return Some(varname);
        }
        None
    }

    fn replace_variables(
        &mut self,
        mem: &mut TMemory,
        s: &[char],
        mut i: usize,
        var: &str,
        result: &mut String,
        location: &StringLocated,
    ) -> Result<usize> {
        if result.ends_with("##") {
            result.truncate(result.len() - 2);
        }
        let value = mem.get_variable(var).expect("variable");
        i += var.chars().count() - 1;
        match &value {
            TValue::Json(j) => {
                if let Some(text) = j.as_str() {
                    result.push_str(text);
                } else if j.is_number() {
                    result.push_str(&j.to_string());
                } else {
                    i += 1;
                    i = self.replace_json(mem, j.clone(), s, i, result, location)? - 1;
                }
            }
            other => result.push_str(&other.to_string()),
        }
        if i + 2 < s.len() && s[i + 1] == '#' && s[i + 2] == '#' {
            i += 2;
        }
        Ok(i)
    }

    fn replace_json(
        &mut self,
        mem: &mut TMemory,
        mut json: JsonValue,
        s: &[char],
        mut i: usize,
        result: &mut String,
        location: &StringLocated,
    ) -> Result<usize> {
        let mut missing = false;
        while i < s.len() {
            let n = s[i];
            if n == '.' {
                i += 1;
                let mut field = String::new();
                while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_' || s[i] == '$') {
                    field.push(s[i]);
                    i += 1;
                }
                match json.get(field.as_str()) {
                    Some(v) => json = v.clone(),
                    None => {
                        missing = true;
                        break;
                    }
                }
            } else if n == '[' {
                i += 1;
                let mut in_bracket = String::new();
                let mut level = 0;
                loop {
                    let Some(&c) = s.get(i) else {
                        return Err(EaterException::new("Major parsing error", location));
                    };
                    if c == '[' {
                        level += 1;
                    }
                    if c == ']' {
                        if level == 0 {
                            break;
                        }
                        level -= 1;
                    }
                    in_bracket.push(c);
                    i += 1;
                }
                let nb = self
                    .apply_functions_and_variables(
                        mem,
                        &StringLocated::new(in_bracket, location.location()),
                    )?
                    .unwrap_or_default();
                let next = match &json {
                    JsonValue::Array(_) => {
                        let idx: usize = nb
                            .parse()
                            .map_err(|_| EaterException::new("Data parsing error", location))?;
                        json.get(idx).cloned()
                    }
                    JsonValue::Object(_) => json.get(nb.as_str()).cloned(),
                    _ => return Err(EaterException::new("Major parsing error", location)),
                };
                match next {
                    Some(v) => json = v,
                    None => return Err(EaterException::new("Data parsing error", location)),
                }
                i += 1;
            } else {
                break;
            }
        }
        if !missing {
            match json.as_str() {
                Some(t) => result.push_str(t),
                None => result.push_str(&json.to_string()),
            }
        }
        Ok(i)
    }

    pub fn is_legacy_define(&self, name: &str) -> bool {
        self.functions
            .functions_by_name(name)
            .iter()
            .any(|f| f.function_type().is_legacy())
    }

    pub fn is_unquoted(&self, name: &str) -> bool {
        self.functions
            .functions_by_name(name)
            .iter()
            .any(|f| f.is_unquoted())
    }

    pub fn does_function_exist(&self, name: &str) -> bool {
        self.functions.does_function_exist(name)
    }

    pub fn function_smart(&self, sig: &TFunctionSignature) -> Option<Rc<dyn TFunction>> {
        self.functions.function_smart(sig)
    }

    /// Joins and removes the lines from index `n1` (`extractFromResultList`).
    pub fn extract_from_result_list(&mut self, n1: usize) -> String {
        let mut sb = String::new();
        while self.result_list.len() > n1 {
            let l = self.result_list.remove(n1);
            sb.push_str(l.string());
            if self.result_list.len() > n1 {
                sb.push('\u{e100}');
            }
        }
        sb
    }

    pub fn xargs(&self) -> Option<String> {
        let first = self.result_list.first()?.string();
        let idx = first.find(' ')?;
        Some(first[idx + 1..].trim().to_string())
    }

    // ---- declarations of functions -------------------------------------------------------

    pub fn execute_legacy_define(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        if self.functions.pending.is_some() {
            return Err(EaterException::new("already0048", s));
        }
        let t = s.trimmed();
        let mut e = Eater::new(t.clone());
        e.skip_spaces();
        e.check_and_eat_str("!define")?;
        e.skip_spaces();
        let f = e.eat_declare_function(self, mem, true, &t, false, TFunctionType::LegacyDefine)?;
        f.set_legacy_definition(e.eat_all_to_end());
        self.functions.add_function(Rc::new(f));
        Ok(())
    }

    pub fn execute_legacy_define_long(
        &mut self,
        mem: &mut TMemory,
        s: &StringLocated,
    ) -> Result<()> {
        if self.functions.pending.is_some() {
            return Err(EaterException::new("already0068", s));
        }
        let t = s.trimmed();
        let mut e = Eater::new(t.clone());
        e.skip_spaces();
        e.check_and_eat_str("!definelong")?;
        e.skip_spaces();
        let f =
            e.eat_declare_function(self, mem, true, &t, true, TFunctionType::LegacyDefineLong)?;
        self.functions.pending = Some(Rc::new(f));
        Ok(())
    }

    pub fn execute_end_function(&mut self) {
        if let Some(p) = self.functions.pending.take() {
            if p.function_type() == TFunctionType::LegacyDefineLong {
                p.finalize_end_definelong();
            }
            self.functions.add_function(p);
        }
    }

    pub fn execute_declare_function(
        &mut self,
        mem: &mut TMemory,
        s: &StringLocated,
        procedure: bool,
    ) -> Result<()> {
        if self.functions.pending.is_some() {
            return Err(EaterException::new("already0068", s));
        }
        let t = s.trimmed();
        let mut e = Eater::new(t);
        e.skip_spaces();
        e.check_and_eat_char('!')?;
        let mut unquoted = false;
        let mut final_flag = false;
        loop {
            let unq = e.peek_char() == 'u';
            let fin = e.peek_char() == 'f' && e.peek_char_n2() == 'i';
            if !(unq || fin) {
                break;
            }
            if unq {
                e.check_and_eat_str("unquoted")?;
                e.skip_spaces();
                unquoted = true;
            } else {
                e.check_and_eat_str("final")?;
                e.skip_spaces();
                final_flag = true;
            }
        }
        let function = if procedure {
            e.check_and_eat_str("procedure")?;
            e.skip_spaces();
            e.eat_declare_function(self, mem, unquoted, s, false, TFunctionType::Procedure)?
        } else {
            e.check_and_eat_str("function")?;
            e.skip_spaces();
            e.eat_declare_return_function_with_optional_return(self, mem, unquoted, s)?
        };
        let declared = function.signature().clone();
        let previous = self.functions.get_exact(&declared);
        if previous.is_some() && (final_flag || self.functions.functions_final.contains(&declared))
        {
            return Err(EaterException::new("This function is already defined", s));
        }
        if final_flag {
            self.functions.functions_final.insert(declared);
        }
        let function = Rc::new(function);
        if function.has_body() {
            self.functions.add_function(function);
        } else {
            self.functions.pending = Some(function);
        }
        Ok(())
    }

    // ---- includes --------------------------------------------------------------------------

    fn eat_include_target(
        &mut self,
        mem: &mut TMemory,
        s: &StringLocated,
        keyword: &str,
    ) -> Result<String> {
        let mut e = Eater::new(s.trimmed());
        e.skip_spaces();
        e.check_and_eat_str(keyword)?;
        e.skip_spaces();
        let rest = e.eat_all_to_end();
        Ok(self
            .apply_functions_and_variables(mem, &StringLocated::new(rest, s.location()))?
            .unwrap_or_default())
    }

    fn read_lines(
        content: &str,
        location: Option<crate::text::LineLocation>,
    ) -> Vec<StringLocated> {
        let _ = location;
        crate::preproc::read_lines(content)
    }

    fn execute_includesub(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let what = self.eat_include_target(mem, s, "!includesub")?;
        let mut sub: Option<Sub> = None;
        let saved_dir = self.current_dir.clone();
        let saved_import = self.current_import;
        if let Some(idx) = what.find('!') {
            let filename = &what[..idx];
            let blocname = &what[idx + 1..];
            if let Some((f, import)) = self.resolve_include_file(filename) {
                self.current_dir = f.dir.clone();
                self.current_import = import;
                let lines = Self::read_lines(&f.content, s.location());
                let r = Sub::from_file(&lines, blocname, self, mem);
                self.current_dir = saved_dir.clone();
                self.current_import = saved_import;
                sub = r?;
            }
        }
        if sub.is_none() {
            sub = self.subs.get(&what).cloned();
        }
        let Some(sub) = sub else {
            return Err(EaterException::new(format!("cannot include {what}"), s));
        };
        let r = self.execute_lines_internal(mem, &sub.lines, None);
        self.current_dir = saved_dir;
        self.current_import = saved_import;
        r
    }

    fn execute_include_def(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let name = self.eat_include_target(mem, s, "!includedef")?;
        let definition = self.definitions.get(&name).cloned().unwrap_or_default();
        let body: Vec<StringLocated> = definition
            .iter()
            .map(|l| StringLocated::new(l.as_str(), s.location()))
            .collect();
        self.execute_lines_internal(mem, &body, None)
    }

    fn execute_theme(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let t = s.trimmed();
        let mut e = Eater::new(t.clone());
        e.skip_spaces();
        e.check_and_eat_str("!theme")?;
        e.skip_spaces();
        let mut name = e.eat_all_to_end();
        let mut from: Option<String> = None;
        if let Some(x) = name.to_lowercase().find(" from ") {
            let from_tmp = name[x + " from ".len()..].trim().to_string();
            from = self
                .apply_functions_and_variables(mem, &StringLocated::new(from_tmp, t.location()))?;
            name = name[..x].trim().to_string();
        }
        let real_name = self
            .apply_functions_and_variables(mem, &StringLocated::new(name.clone(), t.location()))?
            .unwrap_or_default();
        let resolver = self.resolver.clone();
        let Some(content) =
            resolver.read_theme(&real_name, from.as_deref(), self.current_dir.as_deref())
        else {
            let location = from.map(|f| format!(" in {f}")).unwrap_or_default();
            return Err(EaterException::new(
                format!("No such theme {name}{location}"),
                s,
            ));
        };
        let lines = crate::preproc::read_lines(&content);
        // A leading `---` block is the metadata of the theme (`ReadLineWithYamlHeader`).
        let mut body = lines.clone();
        if lines.first().is_some_and(|l| l.string() == "---") {
            let mut metadata = Vec::new();
            let mut end = lines.len();
            for (i, l) in lines.iter().enumerate().skip(1) {
                if l.string() == "---" {
                    end = i + 1;
                    break;
                }
                let tmp = l.string();
                if let Some(idx) = tmp.find(':') {
                    if idx > 0 {
                        metadata.push((
                            tmp[..idx].trim().to_string(),
                            JsonValue::String(tmp[idx + 1..].trim().to_string()),
                        ));
                    }
                }
                end = i + 1;
            }
            body = lines[end.min(lines.len())..].to_vec();
            self.theme_metadata = JsonValue::Object(metadata);
        }
        self.execute_lines(mem, &body, None, false)?;
        Ok(())
    }

    fn execute_include(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let t = s.trimmed();
        let mut e = Eater::new(t.clone());
        e.skip_spaces();
        e.check_and_eat_str("!include")?;
        let mut strategy = IncludeStrategy::Default;
        match e.peek_char() {
            'u' => e.check_and_eat_str("url")?,
            '_' => {
                e.check_and_eat_char('_')?;
                if e.peek_char() == 'm' {
                    e.check_and_eat_str("many")?;
                    strategy = IncludeStrategy::Many;
                } else {
                    e.check_and_eat_str("once")?;
                    strategy = IncludeStrategy::Once;
                }
            }
            _ => {}
        }
        e.skip_spaces();
        let rest = e.eat_all_to_end();
        let mut what = self
            .apply_functions_and_variables(mem, &StringLocated::new(rest, t.location()))?
            .unwrap_or_default();
        let mut suffix: Option<String> = None;
        if let Some(idx) = what.rfind('!') {
            suffix = Some(what[idx + 1..].to_string());
            what.truncate(idx);
        }
        let resolver = self.resolver.clone();
        let saved_dir = self.current_dir.clone();
        let saved_import = self.current_import;
        let content: String;
        if what.starts_with('<') && what.ends_with('>') {
            let path = &what[1..what.len() - 1];
            let Some(c) = resolver.read_stdlib(path) else {
                return Err(EaterException::new(format!("cannot include {what}"), s));
            };
            content = c;
        } else if what.starts_with("http://") || what.starts_with("https://") {
            let Some(c) = resolver.read_url(&what) else {
                return Err(EaterException::new("Cannot open URL", s));
            };
            content = extract_block(&c, suffix.as_deref()).ok_or_else(|| {
                EaterException::new(
                    format!(
                        "cannot include {what}!{}",
                        suffix.clone().unwrap_or_default()
                    ),
                    s,
                )
            })?;
        } else if what.starts_with('[') && what.ends_with(']') {
            return Err(EaterException::new("cannot include To be finished", s));
        } else {
            let Some((f, import)) = self.resolve_include_file(&what) else {
                return Err(EaterException::new(format!("cannot include {what}"), s));
            };
            let key = (f.identity.clone(), suffix.clone());
            let seen = self.included_current.contains(&key);
            if strategy == IncludeStrategy::Default && seen {
                return Ok(());
            }
            if strategy == IncludeStrategy::Once && seen {
                return Err(EaterException::new(
                    "This file has already been included",
                    s,
                ));
            }
            content = match extract_block(&f.content, suffix.as_deref()) {
                Some(c) => c,
                None => {
                    return Err(EaterException::new(
                        format!(
                            "cannot include {what}!{}",
                            suffix.clone().unwrap_or_default()
                        ),
                        s,
                    ))
                }
            };
            self.current_dir = f.dir.clone();
            self.current_import = import;
            self.included_current.insert(key);
        }
        let lines = crate::preproc::read_lines(&content);
        let lines = crate::preproc::remove_yaml_header_lines(lines);
        let r = self.execute_lines(mem, &lines, None, false);
        self.current_dir = saved_dir;
        self.current_import = saved_import;
        r.map(|_| ())
    }

    /// Reads `what` from the filesystem, the current archive, or an imported
    /// archive's root (`PathSystem.getInputFile`).
    fn resolve_include_file(&self, what: &str) -> Option<(ResolvedFile, Option<usize>)> {
        if let Some(idx) = self.current_import {
            if let Some((path, content, dir)) =
                self.imports[idx].read(what, self.current_dir.as_deref())
            {
                return Some((
                    ResolvedFile {
                        content,
                        dir: Some(dir),
                        identity: format!("import:{idx}:{path}"),
                    },
                    Some(idx),
                ));
            }
        }
        if let Some(f) = self.resolver.read_file(what, self.current_dir.as_deref()) {
            return Some((f, None));
        }
        for (idx, archive) in self.imports.iter().enumerate() {
            if let Some((path, content, dir)) = archive.read(what, None) {
                return Some((
                    ResolvedFile {
                        content,
                        dir: Some(dir),
                        identity: format!("import:{idx}:{path}"),
                    },
                    Some(idx),
                ));
            }
        }
        None
    }

    /// `!import archive.zip` adds the archive's entries for later includes.
    fn execute_import(&mut self, mem: &mut TMemory, s: &StringLocated) -> Result<()> {
        let mut e = Eater::new(s.trimmed());
        e.skip_spaces();
        e.check_and_eat_str("!import")?;
        e.skip_spaces();
        let what = self
            .apply_functions_and_variables(
                mem,
                &StringLocated::new(e.eat_all_to_end(), s.location()),
            )?
            .unwrap_or_default();
        let resolver = self.resolver.clone();
        let Some(bytes) = resolver.read_binary(&what, self.current_dir.as_deref()) else {
            return Err(EaterException::new("Cannot import", s));
        };
        let Some(archive) = ImportedArchive::parse(&bytes) else {
            return Err(EaterException::new("Cannot import", s));
        };
        self.imports.push(archive);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncludeStrategy {
    Default,
    Many,
    Once,
}

/// Selects the diagram block asked by `file!suffix` (`DiagramDetector.extractFromFile`).
///
/// Without suffix the whole file is returned (including its `@startuml` lines, which the
/// preprocessor drops); a numeric suffix selects the n-th block and an identifier the block
/// declared with `@startuml(id=name)`.
pub fn extract_block(content: &str, suffix: Option<&str>) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let is_start = |l: &str| crate::command::is_start_directive(l);
    let is_end = |l: &str| crate::command::is_end_directive(l);
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, l) in lines.iter().enumerate() {
        if is_start(l) {
            start = Some(i);
        } else if is_end(l) {
            if let Some(s) = start.take() {
                blocks.push((s, i));
            }
        }
    }
    // A plain include of a file without a diagram block is used as is.
    if blocks.is_empty() {
        return Some(content.to_string());
    }
    let chosen = match suffix {
        None => blocks.first(),
        Some(suffix) => {
            if let Ok(n) = suffix.parse::<usize>() {
                blocks.get(n)
            } else {
                let id = format!("id={suffix}");
                blocks.iter().find(|(s, _)| {
                    let l = lines[*s].to_lowercase();
                    l.contains(&format!("({})", id.to_lowercase()))
                        || l.contains(&format!("({id},").to_lowercase())
                        || l.contains(&format!(",{id})").to_lowercase())
                })
            }
        }
    }?;
    // The start and end directives are not part of the included body.
    Some(lines[chosen.0 + 1..chosen.1].join("\n"))
}
