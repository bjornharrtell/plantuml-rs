//! Commands of WBS diagrams (`wbs/Command*`).

use super::WbsDiagram;
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::parser::ColorParser;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::mindmap::IdeaShape;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::text::{BlocLines, MultilinesStrategy};
use regex::Regex;
use std::sync::LazyLock;

type Cmd = Box<dyn Command<WbsDiagram>>;

const TYPE: &str = "TYPE";
const DIRECTION: &str = "DIRECTION";

fn wbs_type() -> IRegex {
    IRegex::named(TYPE, r"([ \t]*[*+-]+)")
}

fn shape_direction() -> IRegex {
    IRegex::or(vec![
        IRegex::group(vec![
            IRegex::named("SHAPE_1", "(_)?"),
            IRegex::named("DIRECTION_1", "([<>])?"),
        ]),
        IRegex::group(vec![IRegex::named("DIRECTION_2", "([<>])?")]),
        IRegex::named("SHAPE_2", "(_)?"),
    ])
}

fn color_code() -> IRegex {
    let color = |n: &str| IRegex::optional(IRegex::named(n, r"\[(#\w+)\]"));
    let code = |n: &str| IRegex::optional(IRegex::named(n, r"\(([%pLN_]+)\)"));
    IRegex::or(vec![
        IRegex::group(vec![color("BACKCOLOR_1"), code("CODE_1")]),
        IRegex::group(vec![code("CODE_2"), color("BACKCOLOR_2")]),
    ])
}

/// The two orders of shape/direction and color/code: `old` puts the color first.
fn item_prefix(old: bool) -> Vec<IRegex> {
    if old {
        vec![wbs_type(), color_code(), shape_direction()]
    } else {
        vec![wbs_type(), shape_direction(), color_code()]
    }
}

fn item_regex(old: bool, with_code: bool) -> IRegex {
    let mut parts = vec![IRegex::start()];
    if with_code {
        parts.extend(item_prefix(old));
        parts.push(IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("LABEL", "([^%s].*)"),
        ])));
    } else {
        parts.push(wbs_type());
        if old {
            parts.push(IRegex::optional(IRegex::named("BACKCOLOR", r"\[(#\w+)\]")));
            parts.push(shape_direction());
        } else {
            parts.push(shape_direction());
            parts.push(IRegex::optional(IRegex::named("BACKCOLOR", r"\[(#\w+)\]")));
        }
        parts.extend([
            IRegex::space_one_or_more(),
            IRegex::named("LABEL", "[%g](.*)[%g]"),
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
            IRegex::named("CODE", "([%pLN_]+)"),
        ]);
    }
    parts.push(IRegex::end());
    IRegex::concat(parts)
}

fn back_color(arg: &RegexResult) -> Result<Option<HColor>, CommandExecutionResult> {
    match arg.get_lazzy("BACKCOLOR", 0) {
        None => Ok(None),
        Some(c) => HColor::parse(c)
            .map(Some)
            .ok_or_else(CommandExecutionResult::bad_color),
    }
}

/// `Direction.getWBSDirection`: true when going left.
fn goes_left(arg: &RegexResult) -> bool {
    let mut left = arg.get(TYPE, 0).unwrap_or("").contains('-');
    match arg.get_lazzy(DIRECTION, 0) {
        Some("<") => left = true,
        Some(">") => left = false,
        _ => {}
    }
    left
}

fn order_warning(arg: &RegexResult, with_code: bool) -> Vec<String> {
    let mut sb = arg.get(TYPE, 0).unwrap_or("").trim().to_string();
    let mut append = |before: &str, value: Option<&str>, after: &str| {
        if let Some(v) = value.filter(|v| !v.is_empty()) {
            sb.push_str(before);
            sb.push_str(v);
            sb.push_str(after);
        }
    };
    append("", arg.get_lazzy("SHAPE", 0), "");
    append("", arg.get_lazzy(DIRECTION, 0), "");
    append("[", arg.get_lazzy("BACKCOLOR", 0), "]");
    if with_code {
        append("(", arg.get_lazzy("CODE", 0), ")");
    }
    vec![format!(
        "Shape (_) and direction (< >) must come before color and id: start the line with '{sb}'"
    )]
}

fn item(old: bool, with_code: bool) -> Cmd {
    Box::new(
        SingleLineCommand::new(
            item_regex(old, with_code),
            move |d: &mut WbsDiagram, _, arg, _| {
                let color = match back_color(arg) {
                    Ok(c) => c,
                    Err(e) => return e,
                };
                let label = arg.get("LABEL", 0);
                let mut shape = IdeaShape::from_desc(arg.get_lazzy("SHAPE", 0));
                if label.is_none() && shape == IdeaShape::None {
                    shape = IdeaShape::Pseudo;
                } else if label.is_none() {
                    return CommandExecutionResult::error("Missing label for WBS node.");
                }
                if old {
                    d.base.add_warning(order_warning(arg, with_code));
                }
                let level = match d.get_smart_level(arg.get(TYPE, 0).unwrap_or("")) {
                    Ok(l) => l,
                    Err(e) => return CommandExecutionResult::error(e),
                };
                d.add_idea_text(
                    arg.get_lazzy("CODE", 0),
                    color,
                    level,
                    label.unwrap_or(""),
                    goes_left(arg),
                    shape,
                )
            },
        )
        .no_trim(),
    )
}

static END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(.*);\s*(<<(.+)>>)?$").expect("valid"));

fn multiline(old: bool) -> Cmd {
    let mut parts = vec![IRegex::start()];
    parts.extend(item_prefix(old));
    parts.extend([
        IRegex::leaf(":"),
        IRegex::named("DATA", "(.*)"),
        IRegex::end(),
    ]);
    let regex = IRegex::concat(parts);
    let starting = regex.clone();
    Box::new(MultiLineCommand::new(
        regex,
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        r"^(.*);\s*(\<\<(.+)\>\>)?$",
        move |d: &mut WbsDiagram, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let Some(line0) = starting.matcher(first.string()) else {
                return CommandExecutionResult::error("Cannot parse the WBS node");
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
            let color = match back_color(&line0) {
                Ok(c) => c,
                Err(e) => return e,
            };
            if old {
                d.base.add_warning(order_warning(&line0, true));
            }
            let level = match d.get_smart_level(line0.get(TYPE, 0).unwrap_or("")) {
                Ok(l) => l,
                Err(e) => return CommandExecutionResult::error(e),
            };
            d.add_idea(
                line0.get_lazzy("CODE", 0),
                color,
                level,
                Display::create(&lines.as_strings()),
                stereotype.map(|s| Stereotype::build(&s)),
                goes_left(&line0),
                IdeaShape::from_desc(line0.get_lazzy("SHAPE", 0)),
            )
        },
    ))
}

fn link() -> Cmd {
    let color = ColorParser::simple_color(ColorType::Line);
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("CODE1", "([%pLN_]+)"),
        IRegex::space_zero_or_more(),
        IRegex::named("LINK", r"[.-]+\>"),
        IRegex::space_zero_or_more(),
        IRegex::named("CODE2", "([%pLN_]+)"),
        IRegex::space_zero_or_more(),
        color.regex(),
        crate::stereo::optional_pattern("STEREOTYPE"),
        IRegex::named("LABEL_LINK", "(?::[%s]*(.+))?"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        move |d: &mut WbsDiagram, _, arg, _| {
            let Some(colors) = color.colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            let stereotype = arg.get("STEREOTYPE", 0).map(Stereotype::build);
            d.link(
                arg.get("CODE1", 0).unwrap_or(""),
                arg.get("CODE2", 0).unwrap_or(""),
                colors.color(ColorType::Line).cloned(),
                stereotype.as_ref(),
            )
        },
    ))
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.push(item(false, false));
    cmds.push(item(false, true));
    cmds.push(multiline(false));
    cmds.push(item(true, false));
    cmds.push(item(true, true));
    cmds.push(multiline(true));
    cmds.push(link());
    cmds
}
