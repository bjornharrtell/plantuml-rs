//! Commands accepted inside `@startebnf`
//! (`CommandEBnfSingleLine`, `UBrexCommandEbnfMultilines` and the comment commands).

use super::PSystemEbnf;
use crate::command::common::common_commands;
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::regex::IRegex;
use crate::text::{BlocLines, MultilinesStrategy};

/// Every command of an EBNF diagram, in the order they are tried
/// (`PSystemEbnfFactory.initCommandsList`).
pub fn commands() -> Vec<Box<dyn Command<PSystemEbnf>>> {
    let mut cmds = common_commands();
    cmds.push(comment_single_line());
    cmds.push(comment_multilines2());
    cmds.push(comment_multilines());
    cmds.push(rule_single_line());
    cmds.push(rule_multilines());
    cmds
}

/// `(* comment *)` on one line (`UBrexCommandComment`).
fn comment_single_line() -> Box<dyn Command<PSystemEbnf>> {
    let pattern = IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\(\*"),
        IRegex::named("COMMENT", "(.*)"),
        IRegex::leaf(r"\*\)"),
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        pattern,
        |d: &mut PSystemEbnf, _, arg, _| {
            d.add_note(arg.get("COMMENT", 0).unwrap_or("").trim().to_string());
            CommandExecutionResult::ok()
        },
    ))
}

/// `(** comment **)` over several lines (`CommandCommentMultilines2`).
fn comment_multilines2() -> Box<dyn Command<PSystemEbnf>> {
    let starting = IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\(\*\*"),
        IRegex::leaf(".*"),
        IRegex::end(),
    ]);
    Box::new(MultiLineCommand::new(
        starting,
        MultilinesStrategy::KeepStartingQuote,
        Trim::Both,
        r"^.*\*\*\)[%s]*$",
        |d: &mut PSystemEbnf, lines, _| {
            d.add_note(remove_few_chars(lines, 3));
            CommandExecutionResult::ok()
        },
    ))
}

/// `(* comment *)` over several lines (`CommandCommentMultilines`).
fn comment_multilines() -> Box<dyn Command<PSystemEbnf>> {
    let starting = IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\(\*"),
        IRegex::leaf(".*"),
        IRegex::end(),
    ]);
    Box::new(MultiLineCommand::new(
        starting,
        MultilinesStrategy::KeepStartingQuote,
        Trim::Both,
        r"^.*\*\)[%s]*$",
        |d: &mut PSystemEbnf, lines, _| {
            d.add_note(remove_few_chars(lines, 2));
            CommandExecutionResult::ok()
        },
    ))
}

/// `name = expression ;` on one line, with optional comments around the name
/// (`CommandEBnfSingleLine`).
fn rule_single_line() -> Box<dyn Command<PSystemEbnf>> {
    let comment = |name: &'static str| {
        IRegex::optional(IRegex::group(vec![
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\(\*"),
            IRegex::named(name, "(.*[^%s].*)"),
            IRegex::leaf(r"\*\)"),
            IRegex::space_zero_or_more(),
        ]))
    };
    let pattern = IRegex::concat(vec![
        IRegex::start(),
        comment("COMMENTA"),
        IRegex::named("ID", "([%pLN_][-%pLN_]*)"),
        comment("COMMENTB"),
        IRegex::space_zero_or_more(),
        IRegex::named("EQUALS", "(=)"),
        IRegex::space_zero_or_more(),
        IRegex::named("LINE", "(.*;)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        pattern,
        |d: &mut PSystemEbnf, _, arg, _| {
            let id = arg.get("ID", 0).unwrap_or("");
            let equals = arg.get("EQUALS", 0).unwrap_or("");
            let line = arg.get("LINE", 0).unwrap_or("");
            let full = format!("{id}{equals}{line}");
            let above = arg.get("COMMENTA", 0).map(str::to_string);
            let below = arg.get("COMMENTB", 0).map(str::to_string);
            d.add_bloc_lines(&BlocLines::single_string(&full), above, below)
        },
    ))
}

/// `name = expression ;` spread over several lines (`UBrexCommandEbnfMultilines`).
fn rule_multilines() -> Box<dyn Command<PSystemEbnf>> {
    let starting = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("[%pLN_]"),
        IRegex::zero_or_more(IRegex::leaf("[-%pLN_]")),
        IRegex::space_zero_or_more(),
        IRegex::leaf("="),
        IRegex::leaf(".*"),
        IRegex::end(),
    ]);
    Box::new(MultiLineCommand::new(
        starting,
        MultilinesStrategy::KeepStartingQuote,
        Trim::Both,
        r"^.*;$",
        |d: &mut PSystemEbnf, lines, _| d.add_bloc_lines(lines, None, None),
    ))
}

/// Removes the `n` delimiters at the start of the first line and at the end of the last one
/// (`BlocLines.removeFewChars`).
fn remove_few_chars(lines: &BlocLines, n: usize) -> String {
    let lines = lines.trim();
    let mut strings = lines.as_strings();
    if strings.is_empty() {
        return String::new();
    }
    let first: String = strings[0].chars().skip(n).collect();
    strings[0] = first;
    let last = strings.len() - 1;
    let kept = strings[last].chars().count().saturating_sub(n);
    strings[last] = strings[last].chars().take(kept).collect();
    strings.join("\n")
}
