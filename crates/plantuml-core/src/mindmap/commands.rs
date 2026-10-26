//! Commands of mind map diagrams (`mindmap/Command*`).

use super::{IdeaShape, MindMapDiagram};
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::text::{BlocLines, MultilinesStrategy};
use regex::Regex;
use std::sync::LazyLock;

type Cmd = Box<dyn Command<MindMapDiagram>>;

/// Optional `[#color]` of a node.
fn back_color(arg: &RegexResult) -> Result<Option<HColor>, CommandExecutionResult> {
    match arg.get("BACKCOLOR", 0) {
        None => Ok(None),
        Some(c) => HColor::parse(c)
            .map(Some)
            .ok_or_else(CommandExecutionResult::bad_color),
    }
}

fn type_color_shape() -> Vec<IRegex> {
    vec![
        IRegex::optional(IRegex::named("BACKCOLOR", r"\[(#\w+)\]")),
        IRegex::named("SHAPE", "(_)?"),
    ]
}

fn root() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", "(0)"),
        IRegex::space_zero_or_more(),
        IRegex::named("LABEL", "(.*)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        |d: &mut MindMapDiagram, _, arg, _| {
            let label = Display::with_newlines(arg.get("LABEL", 0));
            d.add_idea(None, 0, label, IdeaShape::Box, true)
        },
    ))
}

fn orgmode() -> Cmd {
    let mut parts = vec![IRegex::start(), IRegex::named("TYPE", r"([ \t]*[*#]+)")];
    parts.extend(type_color_shape());
    parts.extend([
        IRegex::space_zero_or_more(),
        IRegex::named("LABEL", "(.*)"),
        IRegex::end(),
    ]);
    Box::new(
        SingleLineCommand::new(
            IRegex::concat(parts),
            |d: &mut MindMapDiagram, _, arg, _| {
                let color = match back_color(arg) {
                    Ok(c) => c,
                    Err(e) => return e,
                };
                let level = match d.get_smart_level(arg.get("TYPE", 0).unwrap_or("")) {
                    Ok(l) => l,
                    Err(e) => return CommandExecutionResult::error(e),
                };
                let label = Display::with_newlines(arg.get("LABEL", 0));
                let shape = IdeaShape::from_desc(arg.get("SHAPE", 0));
                d.add_idea_in_default_direction(color, level, label, shape)
            },
        )
        .no_trim(),
    )
}

fn plus() -> Cmd {
    let mut parts = vec![IRegex::start(), IRegex::named("TYPE", "([+-]+)")];
    parts.extend(type_color_shape());
    parts.extend([
        IRegex::space_zero_or_more(),
        IRegex::named("LABEL", "(.*)"),
        IRegex::end(),
    ]);
    Box::new(
        SingleLineCommand::new(
            IRegex::concat(parts),
            |d: &mut MindMapDiagram, _, arg, _| {
                let color = match back_color(arg) {
                    Ok(c) => c,
                    Err(e) => return e,
                };
                let marker = arg.get("TYPE", 0).unwrap_or("");
                let direction = !marker.contains('-');
                let label = Display::with_newlines(arg.get("LABEL", 0));
                let shape = IdeaShape::from_desc(arg.get("SHAPE", 0));
                d.add_idea(
                    color,
                    marker.chars().count() as i32 - 1,
                    label,
                    shape,
                    direction,
                )
            },
        )
        .no_trim(),
    )
}

fn direction() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("[^*#]*"),
        IRegex::leaf(r"\b"),
        IRegex::named("DIRECTION", "(left|right|top|bottom)"),
        IRegex::leaf(r"\b"),
        IRegex::leaf("[^*#]*"),
        IRegex::leaf("(side|direction)"),
        IRegex::leaf("[^*#]*"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        |d: &mut MindMapDiagram, _, arg, _| {
            let dir = arg.get("DIRECTION", 0).unwrap_or("").to_lowercase();
            d.set_default_direction(dir == "right" || dir == "bottom");
            CommandExecutionResult::ok()
        },
    ))
}

static END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(.*);\s*(<<(.+)>>)?$").expect("valid"));

fn orgmode_multiline() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", "([*#]+)"),
        IRegex::optional(IRegex::named("BACKCOLOR", r"\[(#\w+)\]")),
        IRegex::named("SHAPE", "(_)?"),
        IRegex::leaf(":"),
        IRegex::named("DATA", "(.*)"),
        IRegex::end(),
    ]);
    let starting = regex.clone();
    Box::new(MultiLineCommand::new(
        regex,
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        r"^(.*);\s*(\<\<(.+)\>\>)?$",
        move |d: &mut MindMapDiagram, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let Some(line0) = starting.matcher(first.string()) else {
                return CommandExecutionResult::error("Cannot parse the mind map node");
            };
            let color = match back_color(&line0) {
                Ok(c) => c,
                Err(e) => return e,
            };
            let last = END.captures(lines.last().string());
            let stereotype = last
                .as_ref()
                .and_then(|c| c.get(2))
                .map(|m| m.as_str().to_string());
            let mut lines = lines.remove_starting_and_ending(line0.get("DATA", 0).unwrap_or(""), 1);
            if let (Some(_), Some(c)) = (&stereotype, &last) {
                lines = lines.override_last_line(c.get(1).map_or("", |m| m.as_str()));
            }
            let level = line0.get("TYPE", 0).unwrap_or("").chars().count() as i32 - 1;
            let label = Display::create(&lines.as_strings());
            let shape = IdeaShape::from_desc(line0.get("SHAPE", 0));
            match stereotype {
                None => d.add_idea_in_default_direction(color, level, label, shape),
                Some(s) => {
                    d.add_idea_default(color, level, label, shape, Some(Stereotype::build(&s)))
                }
            }
        },
    ))
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.push(orgmode_multiline());
    cmds.push(orgmode());
    cmds.push(root());
    cmds.push(plus());
    cmds.push(direction());
    cmds
}
