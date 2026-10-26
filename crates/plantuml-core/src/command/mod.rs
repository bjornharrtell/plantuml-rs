//! Command framework, ported from PlantUML's `command` package: single-line and multi-line
//! commands matched with [`IRegex`] and the loop that feeds source lines to them.

pub mod common;

use crate::regex::{compile, IRegex, RegexResult};
use crate::text::{BlocLines, LineLocation, MultilinesStrategy, StringLocated};

/// Outcome of asking a command whether some lines are for it (`CommandControl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandControl {
    Ok,
    NotOk,
    /// More lines are needed.
    OkPartial,
}

/// Passes a diagram may need over the source (`ParserPass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserPass {
    One,
    Two,
    Three,
}

/// Result of running a command (`CommandExecutionResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandExecutionResult {
    error: Option<String>,
    score: i32,
}

impl CommandExecutionResult {
    pub fn ok() -> Self {
        Self {
            error: None,
            score: 0,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            score: 0,
        }
    }

    pub fn error_with_score(message: impl Into<String>, score: i32) -> Self {
        Self {
            error: Some(message.into()),
            score,
        }
    }

    pub fn bad_color() -> Self {
        Self::error_with_score("No such color", 10)
    }

    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn score(&self) -> i32 {
        self.score
    }
}

/// A parsing error with the line where it occurred (`ErrorUml`).
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorUml {
    pub kind: ErrorUmlType,
    pub message: String,
    pub score: i32,
    pub line: Option<StringLocated>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorUmlType {
    SyntaxError,
    ExecutionError,
}

impl ErrorUml {
    /// 1-based line number of the error, 0 when unknown.
    pub fn position(&self) -> usize {
        self.line
            .as_ref()
            .and_then(StringLocated::location)
            .map_or(0, |l| l.position)
    }
}

impl std::fmt::Display for ErrorUml {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (line {})", self.message, self.position())
    }
}

impl std::error::Error for ErrorUml {}

/// A command working on a diagram of type `D`.
pub trait Command<D> {
    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult;

    fn is_valid(&self, lines: &BlocLines) -> CommandControl;

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        pass == ParserPass::One
    }
}

/// Restricts a command to some passes (`isEligibleFor`).
pub struct OnPasses<D> {
    inner: Box<dyn Command<D>>,
    passes: Vec<ParserPass>,
}

impl<D> Command<D> for OnPasses<D> {
    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult {
        self.inner.execute(diagram, lines, pass)
    }

    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        self.inner.is_valid(lines)
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        self.passes.contains(&pass)
    }
}

/// Wraps `inner` so that it only runs on `passes`.
pub fn on_passes<D: 'static>(
    inner: Box<dyn Command<D>>,
    passes: &[ParserPass],
) -> Box<dyn Command<D>> {
    Box::new(OnPasses {
        inner,
        passes: passes.to_vec(),
    })
}

/// How whitespace around the last line of a multiline command is handled (`Trim`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trim {
    Both,
    LeftOnly,
    None,
}

impl Trim {
    pub fn trim(self, s: &StringLocated) -> String {
        match self {
            Trim::None => s.string().to_string(),
            Trim::Both => s.trimmed().string().to_string(),
            Trim::LeftOnly => s.string().trim_start().to_string(),
        }
    }
}

type TextPredicate = Box<dyn Fn(&str) -> bool>;
type LinesPredicate = Box<dyn Fn(&BlocLines) -> bool>;
type LinesVerification = Box<dyn Fn(&BlocLines) -> CommandControl>;

type SingleExec<D> =
    dyn Fn(&mut D, Option<LineLocation>, &RegexResult, ParserPass) -> CommandExecutionResult;

/// One-line command (`SingleLineCommand2`).
pub struct SingleLineCommand<D> {
    pattern: IRegex,
    do_trim: bool,
    with_final_bracket: bool,
    forbidden_lines: Option<LinesPredicate>,
    forbidden_text: Option<TextPredicate>,
    final_verification: Option<Box<dyn Fn() -> CommandControl>>,
    exec: Box<SingleExec<D>>,
}

impl<D> SingleLineCommand<D> {
    pub fn new(
        pattern: IRegex,
        exec: impl Fn(&mut D, Option<LineLocation>, &RegexResult, ParserPass) -> CommandExecutionResult
            + 'static,
    ) -> Self {
        Self {
            pattern,
            do_trim: true,
            with_final_bracket: false,
            forbidden_lines: None,
            forbidden_text: None,
            final_verification: None,
            exec: Box::new(exec),
        }
    }

    pub fn no_trim(mut self) -> Self {
        self.do_trim = false;
        self
    }

    /// The command may be written `x` newline `{` or `x {`.
    pub fn with_final_bracket(mut self) -> Self {
        self.with_final_bracket = true;
        self
    }

    pub fn forbid_lines(mut self, f: impl Fn(&BlocLines) -> bool + 'static) -> Self {
        self.forbidden_lines = Some(Box::new(f));
        self
    }

    pub fn forbid_text(mut self, f: impl Fn(&str) -> bool + 'static) -> Self {
        self.forbidden_text = Some(Box::new(f));
        self
    }

    pub fn final_verification(mut self, f: impl Fn() -> CommandControl + 'static) -> Self {
        self.final_verification = Some(Box::new(f));
        self
    }

    fn my_trim(&self, s: &StringLocated) -> String {
        if self.do_trim {
            s.trimmed().string().to_string()
        } else {
            s.string().to_string()
        }
    }

    fn is_valid_bracket(&self, lines: &BlocLines) -> CommandControl {
        if self.my_trim(lines.at(1)) != "{" {
            return CommandControl::NotOk;
        }
        let v = format!("{} {{", lines.at(0).string());
        self.is_valid(&BlocLines::single_string(&v))
    }
}

impl<D> Command<D> for SingleLineCommand<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        if lines.size() == 2 && self.with_final_bracket {
            return self.is_valid_bracket(lines);
        }
        if lines.size() != 1 {
            return CommandControl::NotOk;
        }
        if self.forbidden_lines.as_ref().is_some_and(|f| f(lines)) {
            return CommandControl::NotOk;
        }
        let line = self.my_trim(lines.at(0));
        if self.with_final_bracket && !line.ends_with('{') {
            let v = format!("{} {{", lines.at(0).string());
            return if self.is_valid(&BlocLines::single_string(&v)) == CommandControl::Ok {
                CommandControl::OkPartial
            } else {
                CommandControl::NotOk
            };
        }
        if self.pattern.is_match(&line) {
            self.final_verification
                .as_ref()
                .map_or(CommandControl::Ok, |f| f())
        } else {
            CommandControl::NotOk
        }
    }

    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult {
        let lines = if self.with_final_bracket && lines.size() == 2 {
            BlocLines::single_string(&format!("{} {{", lines.at(0).string()))
        } else {
            lines.clone()
        };
        assert_eq!(lines.size(), 1, "single line command");
        let first = lines.at(0);
        let line = self.my_trim(first);
        if self.forbidden_text.as_ref().is_some_and(|f| f(&line)) {
            return CommandExecutionResult::error(format!("Syntax error: {line}"));
        }
        let Some(arg) = self.pattern.matcher(&line) else {
            return CommandExecutionResult::error(format!("Cannot parse line {line}"));
        };
        (self.exec)(diagram, first.location(), &arg, pass)
    }
}

type MultiExec<D> = dyn Fn(&mut D, &BlocLines, ParserPass) -> CommandExecutionResult;

/// Command spanning several lines up to an end marker (`CommandMultilines2`).
pub struct MultiLineCommand<D> {
    starting: IRegex,
    strategy: MultilinesStrategy,
    trim_end: Trim,
    end: fancy_regex::Regex,
    with_final_bracket: bool,
    forbidden: Option<LinesPredicate>,
    final_verification: Option<LinesVerification>,
    exec: Box<MultiExec<D>>,
}

impl<D> MultiLineCommand<D> {
    /// `end` is a pattern that must match the whole (trimmed) last line.
    pub fn new(
        starting: IRegex,
        strategy: MultilinesStrategy,
        trim_end: Trim,
        end: &str,
        exec: impl Fn(&mut D, &BlocLines, ParserPass) -> CommandExecutionResult + 'static,
    ) -> Self {
        Self {
            starting,
            strategy,
            trim_end,
            end: compile(end),
            with_final_bracket: false,
            forbidden: None,
            final_verification: None,
            exec: Box::new(exec),
        }
    }

    pub fn with_final_bracket(mut self) -> Self {
        self.with_final_bracket = true;
        self
    }

    pub fn forbid(mut self, f: impl Fn(&BlocLines) -> bool + 'static) -> Self {
        self.forbidden = Some(Box::new(f));
        self
    }

    pub fn final_verification(
        mut self,
        f: impl Fn(&BlocLines) -> CommandControl + 'static,
    ) -> Self {
        self.final_verification = Some(Box::new(f));
        self
    }

    fn end_matches(&self, s: &str) -> bool {
        self.end
            .find(s)
            .ok()
            .flatten()
            .is_some_and(|m| m.start() == 0 && m.end() == s.len())
    }
}

impl<D> Command<D> for MultiLineCommand<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let mut lines = lines.clean_list(self.strategy);
        if self.forbidden.as_ref().is_some_and(|f| f(&lines)) {
            return CommandControl::NotOk;
        }
        if self.with_final_bracket {
            if lines.size() == 1 && !lines.at(0).trimmed().string().ends_with('{') {
                let v = format!("{} {{", lines.at(0).string());
                return if self.is_valid(&BlocLines::single_string(&v)) == CommandControl::OkPartial
                {
                    CommandControl::OkPartial
                } else {
                    CommandControl::NotOk
                };
            }
            lines = lines.eventually_move_bracket();
        }
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !self.starting.is_match(first.trimmed().string()) {
            return CommandControl::NotOk;
        }
        if lines.size() == 1 {
            return CommandControl::OkPartial;
        }
        if !self.end_matches(&self.trim_end.trim(lines.last())) {
            return CommandControl::OkPartial;
        }
        self.final_verification
            .as_ref()
            .map_or(CommandControl::Ok, |f| f(&lines))
    }

    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult {
        let mut lines = lines.clean_list(self.strategy);
        if self.with_final_bracket {
            lines = lines.eventually_move_bracket();
        }
        (self.exec)(diagram, &lines, pass)
    }
}

type LineConsistency = Box<dyn Fn(&str, i32) -> bool>;

/// Multi-line command whose body ends when braces balance (`CommandMultilinesBracket`).
pub struct BracketCommand<D> {
    starting: IRegex,
    consistent: LineConsistency,
    exec: Box<MultiExec<D>>,
}

impl<D> BracketCommand<D> {
    pub fn new(
        starting: IRegex,
        consistent: impl Fn(&str, i32) -> bool + 'static,
        exec: impl Fn(&mut D, &BlocLines, ParserPass) -> CommandExecutionResult + 'static,
    ) -> Self {
        Self {
            starting,
            consistent: Box::new(consistent),
            exec: Box::new(exec),
        }
    }
}

impl<D> Command<D> for BracketCommand<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !self.starting.is_match(first.trimmed().string()) {
            return CommandControl::NotOk;
        }
        if lines.size() == 1 {
            return CommandControl::OkPartial;
        }
        let mut level = 1;
        for cs in lines.sub_extract(1, 0).iter() {
            let t = cs.trimmed();
            let s = t.string();
            if !(self.consistent)(s, level) {
                return CommandControl::NotOk;
            }
            if s.ends_with('{') {
                level += 1;
            }
            if s.ends_with('}') {
                level -= 1;
            }
            if level < 0 {
                return CommandControl::NotOk;
            }
        }
        if level != 0 {
            CommandControl::OkPartial
        } else {
            CommandControl::Ok
        }
    }

    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult {
        (self.exec)(diagram, lines, pass)
    }
}

/// Multi-line command from a start pattern and an end pattern, both matching whole trimmed
/// lines (`CommandMultilines`).
pub struct SimpleMultiLineCommand<D> {
    starting: IRegex,
    ending: fancy_regex::Regex,
    exec: Box<MultiExec<D>>,
}

impl<D> SimpleMultiLineCommand<D> {
    /// `start` and `end` are pattern sources like `^title$`.
    pub fn new(
        start: &str,
        end: &str,
        exec: impl Fn(&mut D, &BlocLines, ParserPass) -> CommandExecutionResult + 'static,
    ) -> Self {
        let body = start
            .strip_prefix('^')
            .and_then(|s| s.strip_suffix('$'))
            .expect("anchored");
        Self {
            starting: IRegex::concat(vec![IRegex::start(), IRegex::leaf(body), IRegex::end()]),
            ending: compile(end),
            exec: Box::new(exec),
        }
    }

    pub fn starting(&self) -> &IRegex {
        &self.starting
    }
}

impl<D> Command<D> for SimpleMultiLineCommand<D> {
    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let Some(first) = lines.first() else {
            return CommandControl::NotOk;
        };
        if !self.starting.is_match(first.trimmed().string()) {
            return CommandControl::NotOk;
        }
        if lines.size() == 1 {
            return CommandControl::OkPartial;
        }
        let last = lines.last().trimmed();
        let whole = self
            .ending
            .find(last.string())
            .ok()
            .flatten()
            .is_some_and(|m| m.start() == 0 && m.end() == last.string().len());
        if whole {
            CommandControl::Ok
        } else {
            CommandControl::OkPartial
        }
    }

    fn execute(
        &self,
        diagram: &mut D,
        lines: &BlocLines,
        pass: ParserPass,
    ) -> CommandExecutionResult {
        (self.exec)(diagram, lines, pass)
    }
}

/// A diagram that can be built by commands (the parts of `AbstractDiagram` the loop needs).
pub trait CommandDiagram {
    fn required_passes(&self) -> Vec<ParserPass> {
        vec![ParserPass::One]
    }

    fn starting_pass(&mut self, _pass: ParserPass) {}

    /// Error found once all lines were read (`checkFinalError`).
    fn check_final_error(&self) -> Option<String> {
        None
    }

    fn make_diagram_ready(&mut self) {}
}

/// The lines of one diagram, `@startxxx` first and `@endxxx` last (`UmlSource`).
#[derive(Debug, Clone, Default)]
pub struct UmlSource {
    pub lines: Vec<StringLocated>,
}

impl UmlSource {
    /// Builds a source from already located lines.
    pub fn from_lines(lines: Vec<StringLocated>) -> Self {
        Self { lines }
    }

    /// Builds a source from text; lines are numbered from 0 like Java's locations.
    pub fn from_text(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .enumerate()
                .map(|(i, l)| StringLocated::new(l, Some(LineLocation { position: i })))
                .collect(),
        }
    }

    pub fn total_line_count(&self) -> usize {
        self.lines.len()
    }

    /// Whether only directives, comments and blank lines are present.
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|sl| {
            let l = sl.string();
            is_start_directive(l)
                || is_end_directive(l)
                || l.trim_start().starts_with('\'')
                || l.trim().is_empty()
        })
    }
}

fn directive_keyword(s: &str, keyword: &str) -> bool {
    let t = s.trim_start();
    t.strip_prefix(['@', '\\'])
        .is_some_and(|r| r.starts_with(keyword))
}

pub fn is_start_directive(s: &str) -> bool {
    let t = s.trim_start();
    directive_keyword(s, "start") && t.chars().count() > 6
}

pub fn is_end_directive(s: &str) -> bool {
    directive_keyword(s, "end")
}

fn embedded_type(s: &str) -> Option<String> {
    let t = s.trim();
    let rest = t.strip_prefix("{{")?;
    let rest = rest.trim_end();
    if rest.is_empty() || rest.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(rest.to_string())
    } else {
        None
    }
}

fn is_embedded_end(s: &str) -> bool {
    s.trim() == "}}"
}

struct Cursor<'a> {
    lines: &'a [StringLocated],
    pos: usize,
}

impl Cursor<'_> {
    fn has_next(&self) -> bool {
        self.pos < self.lines.len()
    }
    fn peek(&self) -> &StringLocated {
        &self.lines[self.pos]
    }
    fn next(&mut self) -> &StringLocated {
        self.pos += 1;
        &self.lines[self.pos - 1]
    }
}

fn add_embedded(cursor: &mut Cursor<'_>, mut lines: BlocLines) -> BlocLines {
    let line = cursor.next().clone();
    let is_embedded = embedded_type(line.string()).is_some();
    lines = lines.add(line);
    if is_embedded {
        let mut nested = 1;
        while cursor.has_next() {
            let s = cursor.next().clone();
            lines = lines.add(s.clone());
            if embedded_type(s.string()).is_some() {
                nested += 1;
            } else if is_embedded_end(s.string()) {
                nested -= 1;
                if nested == 0 {
                    return lines;
                }
            }
        }
    }
    lines
}

/// Finds the command for the next lines and advances the cursor.
fn get_candidate<'c, D>(
    commands: &'c [Box<dyn Command<D>>],
    cursor: &mut Cursor<'_>,
) -> Option<(&'c dyn Command<D>, BlocLines)> {
    let single = BlocLines::single(cursor.peek().clone());
    for cmd in commands {
        match cmd.is_valid(&single) {
            CommandControl::Ok => {
                cursor.pos += 1;
                return Some((cmd.as_ref(), single));
            }
            CommandControl::OkPartial => {
                let mut cloned = Cursor {
                    lines: cursor.lines,
                    pos: cursor.pos,
                };
                if let Some(lines) = multiline_ok(&mut cloned, cmd.as_ref()) {
                    cursor.pos = cloned.pos;
                    return Some((cmd.as_ref(), lines));
                }
            }
            CommandControl::NotOk => {}
        }
    }
    None
}

fn multiline_ok<D>(cursor: &mut Cursor<'_>, cmd: &dyn Command<D>) -> Option<BlocLines> {
    let mut lines = BlocLines::create();
    while cursor.has_next() {
        lines = add_embedded(cursor, lines);
        match cmd.is_valid(&lines) {
            CommandControl::NotOk => return None,
            CommandControl::Ok => return Some(lines),
            CommandControl::OkPartial => {}
        }
    }
    None
}

/// Runs all commands over the source (`PSystemCommandFactory.createSystem`).
pub fn create_system<D: CommandDiagram>(
    commands: &[Box<dyn Command<D>>],
    mut diagram: D,
    source: &UmlSource,
) -> Result<D, ErrorUml> {
    let lines = &source.lines;
    assert!(
        lines
            .first()
            .is_some_and(|l| is_start_directive(l.string())),
        "source must begin with @start"
    );
    if source.is_empty() {
        return Err(ErrorUml {
            kind: ErrorUmlType::SyntaxError,
            message: "Empty description".into(),
            score: 0,
            line: lines.get(1).cloned(),
        });
    }
    let mut cursor = Cursor { lines, pos: 1 };
    for pass in diagram.required_passes() {
        diagram.starting_pass(pass);
        while cursor.has_next() {
            if is_end_directive(cursor.peek().string()) {
                cursor.pos = 1;
                break;
            }
            let Some((cmd, bloc)) = get_candidate(commands, &mut cursor) else {
                let line = cursor.peek().clone();
                return Err(ErrorUml {
                    kind: ErrorUmlType::SyntaxError,
                    message: "Syntax Error?".into(),
                    score: 0,
                    line: Some(line),
                });
            };
            if !cmd.is_eligible_for(pass) {
                continue;
            }
            let result = cmd.execute(&mut diagram, &bloc, pass);
            if let Some(message) = result.error_message() {
                return Err(ErrorUml {
                    kind: ErrorUmlType::ExecutionError,
                    message: message.to_string(),
                    score: result.score(),
                    line: bloc.first().cloned(),
                });
            }
        }
    }
    if let Some(err) = diagram.check_final_error() {
        return Err(ErrorUml {
            kind: ErrorUmlType::ExecutionError,
            message: err,
            score: 0,
            line: lines.last().cloned(),
        });
    }
    diagram.make_diagram_ready();
    Ok(diagram)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug)]
    struct Diag {
        words: Vec<String>,
        blocks: Vec<Vec<String>>,
        finished: bool,
        fail_final: bool,
    }

    impl CommandDiagram for Diag {
        fn check_final_error(&self) -> Option<String> {
            self.fail_final.then(|| "final".to_string())
        }
        fn make_diagram_ready(&mut self) {
            self.finished = true;
        }
    }

    fn commands() -> Vec<Box<dyn Command<Diag>>> {
        let word = IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("word"),
            IRegex::space_one_or_more(),
            IRegex::named("W", "(\\w+)"),
            IRegex::end(),
        ]);
        let fail = IRegex::concat(vec![IRegex::start(), IRegex::leaf("fail"), IRegex::end()]);
        let block = IRegex::concat(vec![IRegex::start(), IRegex::leaf("begin"), IRegex::end()]);
        vec![
            Box::new(SingleLineCommand::new(word, |d: &mut Diag, _, arg, _| {
                d.words.push(arg.get("W", 0).unwrap().to_string());
                CommandExecutionResult::ok()
            })),
            Box::new(SingleLineCommand::new(fail, |_: &mut Diag, _, _, _| {
                CommandExecutionResult::error_with_score("boom", 3)
            })),
            Box::new(MultiLineCommand::new(
                block,
                MultilinesStrategy::RemoveStartingQuote,
                Trim::Both,
                "end",
                |d: &mut Diag, lines, _| {
                    d.blocks.push(lines.sub_extract(1, 1).as_strings());
                    CommandExecutionResult::ok()
                },
            )),
        ]
    }

    fn run(text: &str) -> Result<Diag, ErrorUml> {
        create_system(&commands(), Diag::default(), &UmlSource::from_text(text))
    }

    #[test]
    fn runs_single_and_multi_line_commands() {
        let d =
            run("@startuml\nword a\n  WORD b  \nbegin\n x\n' comment\n y\nend\n@enduml").unwrap();
        assert_eq!(d.words, ["a", "b"]);
        assert_eq!(d.blocks, [vec![" x", " y"]]);
        assert!(d.finished);
    }

    #[test]
    fn errors() {
        let e = run("@startuml\nword a\nnope\n@enduml").unwrap_err();
        assert_eq!(e.kind, ErrorUmlType::SyntaxError);
        assert_eq!(e.position(), 2);
        assert_eq!(e.to_string(), "Syntax Error? (line 2)");
        let e = run("@startuml\nfail\n@enduml").unwrap_err();
        assert_eq!(
            (e.kind, e.score, e.message.as_str()),
            (ErrorUmlType::ExecutionError, 3, "boom")
        );
        let e = run("@startuml\n' only a comment\n@enduml").unwrap_err();
        assert_eq!(e.message, "Empty description");
        let e = run("@startuml\nbegin\nx\n@enduml").unwrap_err();
        assert_eq!(e.kind, ErrorUmlType::SyntaxError);
        let d = Diag {
            fail_final: true,
            ..Diag::default()
        };
        let e = create_system(
            &commands(),
            d,
            &UmlSource::from_text("@startuml\nword a\n@enduml"),
        )
        .unwrap_err();
        assert_eq!(e.message, "final");
    }

    #[test]
    fn bracket_and_trim_variants() {
        let pat = || {
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf("pkg"),
                IRegex::space_one_or_more(),
                IRegex::named("N", "(\\w+)"),
                IRegex::space_zero_or_more(),
                IRegex::leaf("\\{"),
                IRegex::end(),
            ])
        };
        let c = SingleLineCommand::new(pat(), |_: &mut Diag, _, a, _| {
            assert_eq!(a.get("N", 0), Some("p"));
            CommandExecutionResult::ok()
        })
        .with_final_bracket();
        assert_eq!(
            c.is_valid(&BlocLines::from_strings(&["pkg p", "{"])),
            CommandControl::Ok
        );
        assert_eq!(
            c.is_valid(&BlocLines::from_strings(&["pkg p", "x"])),
            CommandControl::NotOk
        );
        assert_eq!(
            c.is_valid(&BlocLines::single_string("pkg p")),
            CommandControl::OkPartial
        );
        assert_eq!(
            c.is_valid(&BlocLines::single_string("zzz")),
            CommandControl::NotOk
        );
        assert_eq!(
            c.is_valid(&BlocLines::from_strings(&["a", "b", "c"])),
            CommandControl::NotOk
        );
        let mut d = Diag::default();
        assert!(c
            .execute(
                &mut d,
                &BlocLines::from_strings(&["pkg p", "{"]),
                ParserPass::One
            )
            .is_ok());
        let nt =
            SingleLineCommand::new(pat(), |_: &mut Diag, _, _, _| CommandExecutionResult::ok())
                .no_trim()
                .forbid_text(|l| l.contains("bad"))
                .forbid_lines(|l| l.at(0).string().contains("never"))
                .final_verification(|| CommandControl::OkPartial);
        assert_eq!(
            nt.is_valid(&BlocLines::single_string("pkg p {")),
            CommandControl::OkPartial
        );
        assert_eq!(
            nt.is_valid(&BlocLines::single_string("pkg never {")),
            CommandControl::NotOk
        );
        assert!(!nt
            .execute(
                &mut d,
                &BlocLines::single_string("pkg bad {"),
                ParserPass::One
            )
            .is_ok());
        assert!(!nt
            .execute(&mut d, &BlocLines::single_string("zzz"), ParserPass::One)
            .is_ok());
        assert!(!nt.is_eligible_for(ParserPass::Two));
        let ml = MultiLineCommand::new(
            IRegex::concat(vec![IRegex::start(), IRegex::leaf("note"), IRegex::end()]),
            MultilinesStrategy::KeepStartingQuote,
            Trim::LeftOnly,
            "end note",
            |_: &mut Diag, _, _| CommandExecutionResult::ok(),
        )
        .forbid(|l| l.size() > 5)
        .final_verification(|_| CommandControl::Ok)
        .with_final_bracket();
        assert_eq!(
            ml.is_valid(&BlocLines::from_strings(&["note"])),
            CommandControl::NotOk
        );
        assert_eq!(Trim::None.trim(&StringLocated::new(" a ", None)), " a ");
        assert_eq!(Trim::Both.trim(&StringLocated::new(" a ", None)), "a");
        assert_eq!(Trim::LeftOnly.trim(&StringLocated::new(" a ", None)), "a ");
        let _ = CommandExecutionResult::bad_color();
    }

    #[test]
    fn directives_and_embedded() {
        assert!(is_start_directive("  @startuml"));
        assert!(!is_start_directive("@start"));
        assert!(is_end_directive("@enduml"));
        assert!(!is_end_directive("end"));
        assert_eq!(embedded_type("{{ditaa"), Some("ditaa".into()));
        assert_eq!(embedded_type("{{"), Some(String::new()));
        assert_eq!(embedded_type("x"), None);
        assert!(is_embedded_end(" }} "));
        let d = run("@startuml\nbegin\n{{\nend\n}}\nend\n@enduml").unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(UmlSource::from_text("a\nb").total_line_count(), 2);
    }
}
