//! Commands of activity diagrams (`activitydiagram3/command`), in the order of
//! `ActivityDiagramFactory3.initCommandsList`.

use super::rainbow::Rainbow;
use super::stereogroup::Stereogroup;
use super::ActivityDiagram3;
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand};
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo;
use crate::text::{BlocLines, MultilinesStrategy};
use crate::url::{self, UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ActivityDiagram3>>;

/// `-[...]->`: a color or line style list (`CommandLinkElement.STYLE_COLORS_MULTIPLES`).
const STYLE_COLORS_MULTIPLES: &str = concat!(
    r"-\[(",
    r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)",
    r"(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*",
    r"(?:(?:;",
    r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)",
    r"(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*",
    r")*)",
    r"*)\]->"
);

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut ActivityDiagram3, &RegexResult) -> CommandExecutionResult + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut ActivityDiagram3, _, arg, _| f(d, arg),
    ))
}

fn url_of(d: &ActivityDiagram3, text: Option<&str>) -> Option<crate::url::Url> {
    let text = text?;
    UrlBuilder::new(d.skin().get_value("topurl"), UrlMode::Strict).url(text)
}

fn color_regex() -> IRegex {
    ColorParser::simple_color(ColorType::Back).regex()
}

/// `start`, `stop` and `end` share their shape.
fn node_command(
    word: &'static str,
    f: fn(&mut ActivityDiagram3, crate::klimt::color::colors::Colors),
) -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf(word),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        move |d, arg| {
            let colors = Stereogroup::build(arg).inner_colors();
            f(d, colors);
            CommandExecutionResult::ok()
        },
    )
}

fn arrow_regex(label: &str) -> Vec<IRegex> {
    vec![
        IRegex::start(),
        IRegex::or(vec![
            IRegex::leaf("->"),
            IRegex::named("COLOR", STYLE_COLORS_MULTIPLES),
        ]),
        IRegex::space_zero_or_more(),
        IRegex::or(vec![IRegex::named("LABEL", label), IRegex::leaf("")]),
        IRegex::end(),
    ]
}

fn apply_arrow_color(d: &mut ActivityDiagram3, arg: &RegexResult) {
    if let Some(color) = arg.get("COLOR", 0) {
        let space = d.skin().as_int("colorarrowseparationspace", 0);
        let rainbow = Rainbow::build(d.skin(), color, space);
        d.set_color_next_arrow(rainbow);
    }
}

fn activity_end_parts() -> Vec<IRegex> {
    vec![
        IRegex::space_zero_or_more(),
        Stereogroup::optional_stereogroup(),
        IRegex::space_zero_or_more(),
        url::optional(),
        IRegex::end(),
    ]
}

fn multiline_end() -> IRegex {
    let mut parts = vec![
        IRegex::start(),
        IRegex::named("TEXT", "(.*)"),
        IRegex::leaf(";"),
    ];
    parts.extend(activity_end_parts());
    IRegex::concat(parts)
}

fn add_long_activity(d: &mut ActivityDiagram3, lines: &BlocLines) -> CommandExecutionResult {
    let lines = lines.remove_empty_columns();
    let first = lines.first().expect("first line").trimmed();
    let start = IRegex::concat(vec![
        IRegex::start(),
        color_regex(),
        IRegex::leaf(":"),
        IRegex::named("DATA", "(.*)"),
        IRegex::end(),
    ]);
    let line0 = start.matcher(first.string()).expect("start line");
    let last = multiline_end()
        .matcher(lines.last().string())
        .expect("end line");
    if let Some(color) = line0.get("COLOR", 0) {
        d.base.add_warning(vec![format!(
            "This syntax is deprecated, you must add <<{color}>> at the end of the line, after the ';'"
        )]);
    }
    let url = url_of(d, last.get(url::URL_KEY, 0));
    let stereogroup = Stereogroup::build(&last);
    let style = stereogroup.box_style();
    let lines = lines.remove_starting_and_ending(line0.get("DATA", 0).unwrap_or(""), 0);
    let lines = lines.override_last_line(last.get("TEXT", 0).unwrap_or(""));
    d.add_activity(
        Display::create(&lines.as_strings()),
        style,
        url,
        &stereogroup,
    )
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    // CommandFootboxIgnored
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("(?:hide|show)?"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("footbox"),
            IRegex::end(),
        ],
        |_, _| CommandExecutionResult::ok(),
    ));
    cmds.extend(crate::command::common::common_commands());
    // CommandSwimlane, CommandSwimlane2, CommandPartition3, CommandCloseGroup3,
    // CommandCloseGroupLegacy3: with swimlanes and groups.

    // CommandArrow3
    cmds.push(single(arrow_regex("(.*);"), |d, arg| {
        apply_arrow_color(d, arg);
        if let Some(label) = arg.get("LABEL", 0).filter(|l| !l.is_empty()) {
            d.set_label_next_arrow(Display::with_newlines(Some(label)));
        }
        CommandExecutionResult::ok()
    }));
    // CommandArrowLong3
    cmds.push(Box::new(MultiLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::or(vec![
                IRegex::leaf("->"),
                IRegex::named("COLOR", STYLE_COLORS_MULTIPLES),
            ]),
            IRegex::space_zero_or_more(),
            IRegex::named("LABEL", "(.*)"),
            IRegex::end(),
        ]),
        MultilinesStrategy::RemoveStartingQuote,
        crate::command::Trim::Both,
        "^(.*);$",
        |d: &mut ActivityDiagram3, lines, _| {
            let lines = lines.remove_empty_columns();
            let start = IRegex::concat(vec![
                IRegex::start(),
                IRegex::or(vec![
                    IRegex::leaf("->"),
                    IRegex::named("COLOR", STYLE_COLORS_MULTIPLES),
                ]),
                IRegex::space_zero_or_more(),
                IRegex::named("LABEL", "(.*)"),
                IRegex::end(),
            ]);
            let first = lines.first().expect("first line").trimmed();
            let line0 = start.matcher(first.string()).expect("start line");
            apply_arrow_color(d, &line0);
            let lines = lines.remove_starting_and_ending(line0.get("LABEL", 0).unwrap_or(""), 1);
            d.set_label_next_arrow(Display::create(&lines.as_strings()));
            CommandExecutionResult::ok()
        },
    )));
    // CommandRepeat3 comes here.

    // CommandActivity3
    cmds.push(single(
        {
            let mut parts = vec![
                IRegex::start(),
                color_regex(),
                stereo::optional_pattern("IGNORED"),
                IRegex::leaf(":"),
                IRegex::named("LABEL", "(.*?)"),
                IRegex::leaf(";"),
            ];
            parts.extend(activity_end_parts());
            parts
        },
        |d, arg| {
            let url = url_of(d, arg.get(url::URL_KEY, 0));
            let stereogroup = Stereogroup::build(arg);
            let label = arg.get("LABEL", 0).unwrap_or("");
            if let Some(ignored) = arg.get("IGNORED", 0) {
                d.base.add_warning(vec![format!(
                    "The stereotype {ignored} before the ':' is ignored: write ':{label}; {ignored}' instead"
                )]);
            }
            if let Some(color) = arg.get("COLOR", 0) {
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated and the color is ignored: write ':{label}; <<{color}>>' instead of '{color}:{label};'"
                )]);
            }
            d.add_activity(
                Display::with_newlines(Some(label)),
                stereogroup.box_style(),
                url,
                &stereogroup,
            )
        },
    ));
    // CommandIf4, CommandIf2, CommandIfLegacy1, CommandElseIf3, CommandElseIf2, CommandElse3,
    // CommandElseLegacy1, CommandEndif3, CommandSwitch, CommandCase, CommandEndSwitch,
    // CommandRepeatWhile3, CommandBackward3, CommandWhile3, CommandWhileEnd3, CommandFork3,
    // CommandForkAgain3, CommandForkEnd3, CommandSplit3, CommandSplitAgain3, CommandSplitEnd3.

    cmds.push(node_command("start", ActivityDiagram3::start));
    cmds.push(node_command("stop", ActivityDiagram3::stop));
    // CommandCircleSpot3, CommandBreak
    cmds.push(node_command("end", ActivityDiagram3::end));
    // CommandKill3

    // CommandLink3
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("link"),
            IRegex::space_one_or_more(),
            IRegex::named("COLOR", r"(#\w+)"),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(c) = arg
                .get("COLOR", 0)
                .and_then(crate::klimt::color::HColor::parse)
            {
                d.set_color_next_arrow(Rainbow::from_color(Some(c), None));
            }
            CommandExecutionResult::ok()
        },
    ));
    // CommandNote3, CommandNoteLong3

    // CommandActivityLong3
    cmds.push(Box::new(MultiLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            color_regex(),
            IRegex::leaf(":"),
            IRegex::named("DATA", "(.*)"),
            IRegex::end(),
        ]),
        MultilinesStrategy::RemoveStartingQuote,
        crate::command::Trim::None,
        &multiline_end().pattern(),
        |d: &mut ActivityDiagram3, lines, _| add_long_activity(d, lines),
    )));
    // CommandActivityList
    cmds.push(single(
        {
            let mut parts = vec![
                IRegex::start(),
                IRegex::leaf("[-*]"),
                IRegex::space_zero_or_one(),
                IRegex::named("LABEL", "(.*?)"),
            ];
            parts.extend(activity_end_parts());
            parts
        },
        |d, arg| {
            let stereogroup = Stereogroup::build(arg);
            let url = url_of(d, arg.get(url::URL_KEY, 0));
            d.add_activity(
                Display::with_newlines(arg.get("LABEL", 0)),
                super::ftile::box_style::BoxStyle::Plain,
                url,
                &stereogroup,
            )
        },
    ));
    // CommandLabel, CommandGoto
    cmds
}
