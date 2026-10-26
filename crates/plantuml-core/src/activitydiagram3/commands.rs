//! Commands of activity diagrams (`activitydiagram3/command`), in the order of
//! `ActivityDiagramFactory3.initCommandsList`.

use super::rainbow::Rainbow;
use super::stereogroup::Stereogroup;
use super::ActivityDiagram3;
use crate::activitydiagram3::ftile::box_style::BoxStyle;
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand};
use crate::decoration::usymbol::USymbol;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo;
use crate::style::{queries, SName};
use crate::text::{BlocLines, MultilinesStrategy};
use crate::url::{self, UrlBuilder, UrlMode};
use std::rc::Rc;

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

fn group_symbol(kind: &str) -> Rc<dyn USymbol> {
    use crate::decoration::usymbol::usymbols;
    match kind.to_lowercase().as_str() {
        "partition" => usymbols::partition(),
        "package" => usymbols::package(),
        "rectangle" => usymbols::rectangle(),
        "card" => usymbols::card(),
        _ => usymbols::group(),
    }
}

/// `partition Name {`, `group G {`, `package`, `rectangle` or `card`.
fn open_group() -> Cmd {
    let back1 = ColorParser::simple_color_id(ColorType::Back, "BACK1");
    let back2 = ColorParser::simple_color_id(ColorType::Back, "BACK2");
    single(
        vec![
            IRegex::start(),
            IRegex::named("TYPE", "(partition|package|rectangle|card|group)"),
            IRegex::space_one_or_more(),
            IRegex::optional(IRegex::group(vec![
                back1.regex(),
                IRegex::space_one_or_more(),
            ])),
            IRegex::named("NAME", "([%g][^%g]+[%g]|.*?)"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_one_or_more(),
                back2.regex(),
            ])),
            crate::stereo::optional_pattern("STEREO"),
            IRegex::named("BRACKET", r"(\{?)"),
            IRegex::end(),
        ],
        move |d, arg| {
            let name = crate::string_utils::eventually_remove_starting_and_ending_double_quote(
                arg.get("NAME", 0).unwrap_or("").trim(),
            );
            let kind = arg.get("TYPE", 0).unwrap_or("group");
            let symbol = group_symbol(kind);
            let parser = if arg.get("BACK1", 0).is_some() {
                ColorParser::simple_color_id(ColorType::Back, "BACK1")
            } else {
                ColorParser::simple_color_id(ColorType::Back, "BACK2")
            };
            let Some(colors) = parser.colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            let stereotype = arg.get("STEREO", 0).map(crate::stereo::Stereotype::build);
            let query = queries::ACTIVITYDIAG
                .add_all(&symbol.snames())
                .add(SName::Composite)
                .with_stereotype_obj(stereotype.as_ref());
            let style = d
                .skin()
                .style_builder()
                .merged_style(&query)
                .expect("group style");
            let back = colors
                .color(ColorType::Back)
                .cloned()
                .unwrap_or_else(|| style.value(crate::style::PName::BackGroundColor).as_color());
            if arg.get("BRACKET", 0).unwrap_or("").is_empty() {
                let mut rebased = kind.to_string();
                if let Some(c) = arg.get("BACK1", 0) {
                    rebased.push(' ');
                    rebased.push_str(c);
                }
                rebased.push(' ');
                rebased.push_str(arg.get("NAME", 0).unwrap_or("").trim());
                if let Some(c) = arg.get("BACK2", 0) {
                    rebased.push(' ');
                    rebased.push_str(c);
                }
                if let Some(c) = arg.get("STEREO", 0) {
                    rebased.push(' ');
                    rebased.push_str(c);
                }
                d.base.add_warning(vec![format!(
                    "Missing '{{': write '{rebased} {{' and close it with '}}'"
                )]);
            }
            d.start_group(
                Display::with_newlines(Some(&name)),
                Some(back),
                symbol,
                style,
            );
            CommandExecutionResult::ok()
        },
    )
}

/// `}` closes the current group (`CommandCloseGroup3`).
fn close_group() -> Cmd {
    single(
        vec![IRegex::start(), IRegex::leaf(r"\}"), IRegex::end()],
        |d, _| d.close_group(),
    )
}

/// `end group` / `group end` closes the current group (`CommandCloseGroupLegacy3`).
fn close_group_legacy() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("CMD", "(end ?group|group ?end)"),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, arg| {
            let cmd = arg.get("CMD", 0).unwrap_or("end group");
            d.base.add_warning(vec![format!(
                "'{cmd}' is deprecated: close the group with '}}' instead"
            )]);
            d.close_group()
        },
    )
}

/// `fork`: start parallel branches.
fn fork() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("fork"),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            let colors = Stereogroup::build(arg).inner_colors();
            d.fork(colors);
            CommandExecutionResult::ok()
        },
    )
}

/// `fork again`: start the next parallel branch.
fn fork_again() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("fork"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("again"),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, _| d.fork_again(),
    )
}

/// `end fork` / `fork end` / `end merge`: close the parallel branches.
fn end_fork() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::or(vec![
                IRegex::leaf("end[%s]*fork"),
                IRegex::leaf("fork[%s]*end"),
                IRegex::leaf("end[%s]*merge"),
            ]),
            IRegex::space_zero_or_more(),
            IRegex::named("LABEL", r"(\{.+\})?"),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, _| d.end_fork(),
    )
}

/// The legacy `(*) --> Activity` start of a flow.
fn legacy_start() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf(r"\(\*\)"),
            IRegex::space_zero_or_more(),
            IRegex::named("ARROW", "(-->|->)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\["),
                IRegex::named("LABEL", r"([^]]*)"),
                IRegex::leaf(r"\]"),
                IRegex::space_zero_or_more(),
            ])),
            IRegex::named("NAME", "(.+)"),
            IRegex::end(),
        ],
        |d, arg| {
            d.start(crate::klimt::color::colors::Colors::empty());
            if let Some(label) = arg.get("LABEL", 0) {
                d.set_label_next_arrow(Display::with_newlines(Some(label)));
            }
            add_legacy_activity(d, arg.get("NAME", 0));
            CommandExecutionResult::ok()
        },
    )
}

fn add_legacy_activity(d: &mut ActivityDiagram3, name: Option<&str>) {
    if let Some(name) = name {
        d.add_activity(
            Display::with_newlines(Some(name.trim())),
            BoxStyle::Plain,
            None,
            &Stereogroup::default(),
        );
    }
}

/// A line that is only a spot name (`===Y1===`).
fn legacy_spot() -> Cmd {
    single(
        vec![IRegex::start(), IRegex::leaf(r"===.*==="), IRegex::end()],
        |_, _| CommandExecutionResult::ok(),
    )
}

/// `===Y1=== --> Activity`: the spot is transparent, the activity follows.
fn legacy_spot_arrow() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf(r"===.*==="),
            IRegex::space_zero_or_more(),
            IRegex::named("ARROW", "(-->|->)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\["),
                IRegex::named("LABEL", r"([^]]*)"),
                IRegex::leaf(r"\]"),
                IRegex::space_zero_or_more(),
            ])),
            IRegex::named("NAME", "(.+)"),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(label) = arg.get("LABEL", 0) {
                d.set_label_next_arrow(Display::with_newlines(Some(label)));
            }
            add_legacy_activity(d, arg.get("NAME", 0));
            CommandExecutionResult::ok()
        },
    )
}

/// `--> (*)`: the legacy end of a flow.
fn legacy_stop() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("ARROW", "(-->|->)"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\(\*\)"),
            IRegex::end(),
        ],
        |d, _| {
            d.stop(crate::klimt::color::colors::Colors::empty());
            CommandExecutionResult::ok()
        },
    )
}

/// `--> ===Y1===`: a trailing arrow to a transparent spot.
fn legacy_arrow_spot() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("ARROW", "(-->|->)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\["),
                IRegex::named("LABEL", r"([^]]*)"),
                IRegex::leaf(r"\]"),
                IRegex::space_zero_or_more(),
            ])),
            IRegex::leaf(r"===.*==="),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(label) = arg.get("LABEL", 0) {
                d.set_label_next_arrow(Display::with_newlines(Some(label)));
            }
            CommandExecutionResult::ok()
        },
    )
}

/// `--> Activity`: the legacy activity declaration after an arrow.
fn legacy_arrow_activity() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("ARROW", "(-->|->)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\["),
                IRegex::named("LABEL", r"([^]]*)"),
                IRegex::leaf(r"\]"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::named("NAME", "([^;]+)"),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(label) = arg.get("LABEL", 0) {
                d.set_label_next_arrow(Display::with_newlines(Some(label)));
            }
            add_legacy_activity(d, arg.get("NAME", 0));
            CommandExecutionResult::ok()
        },
    )
}

/// `switch (test)`: start a switch.
fn switch_start() -> Cmd {
    single(
        vec![
            IRegex::start(),
            crate::klimt::color::parser::exp4(),
            IRegex::leaf("switch"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::named("TEST", "(.*?)"),
            IRegex::leaf(r"\)"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(color) = arg.get("COLOR", 0) {
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated, you must add <<{color}>> at the end of the line"
                )]);
            }
            let colors = Stereogroup::build(arg).inner_colors();
            let back = colors.color(ColorType::Back).cloned();
            d.start_switch(Display::with_newlines(arg.get("TEST", 0)), back);
            CommandExecutionResult::ok()
        },
    )
}

/// `case (value)`: add a branch to the switch.
fn switch_case() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("case"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::named("TEST", "(.*?)"),
            IRegex::leaf(r"\)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let label = arg
                .get("TEST", 0)
                .filter(|t| !t.is_empty())
                .map(|t| Display::with_newlines(Some(t)));
            d.switch_case(label)
        },
    )
}

/// `endswitch`: close the switch.
fn end_switch() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("endswitch"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, _| d.end_switch(),
    )
}

/// `if (test) then (yes)` or `if (test) is (yes) then`.
fn if_start() -> Cmd {
    single(
        vec![
            IRegex::start(),
            crate::klimt::color::parser::exp4(),
            IRegex::leaf("if"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::named("TEST", "(.*?)"),
            IRegex::leaf(r"\)"),
            IRegex::space_zero_or_more(),
            IRegex::named("KEYWORD", "(then|is)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\("),
                IRegex::named("YES", "(.+?)"),
                IRegex::leaf(r"\)"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::leaf("then")),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(color) = arg.get("COLOR", 0) {
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated, you must add <<{color}>> at the end of the line"
                )]);
            }
            let colors = Stereogroup::build(arg).inner_colors();
            let back = colors.color(ColorType::Back).cloned();
            d.start_if(
                Display::with_newlines(arg.get("TEST", 0)),
                arg.get("YES", 0).map(|y| Display::with_newlines(Some(y))),
                back,
            );
            CommandExecutionResult::ok()
        },
    )
}

/// Legacy `if TEST then` without parentheses around the test.
fn if_legacy() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("if"),
            IRegex::space_one_or_more(),
            IRegex::named("TEST", "(.*?)"),
            IRegex::space_one_or_more(),
            IRegex::leaf("then"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\("),
                IRegex::named("YES", "(.+?)"),
                IRegex::leaf(r"\)"),
            ])),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            let colors = Stereogroup::build(arg).inner_colors();
            let back = colors.color(ColorType::Back).cloned();
            let test = match arg.get("TEST", 0) {
                Some("\"\"") | None => Display::with_newlines(None),
                other => Display::with_newlines(other),
            };
            d.start_if(
                test,
                arg.get("YES", 0).map(|y| Display::with_newlines(Some(y))),
                back,
            );
            CommandExecutionResult::ok()
        },
    )
}

/// `else (label)`: add a branch to the current `if`.
fn else_branch() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("else"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\("),
                IRegex::named("WHEN", "(.*?)"),
                IRegex::leaf(r"\)"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, arg| d.else_branch(arg.get("WHEN", 0).map(|w| Display::with_newlines(Some(w)))),
    )
}

/// `elseif (test) then (yes)`: add a branch to the current `if`.
fn elseif_branch() -> Cmd {
    single(
        vec![
            IRegex::start(),
            crate::klimt::color::parser::exp4(),
            IRegex::leaf("elseif"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::named("TEST", "(.*?)"),
            IRegex::leaf(r"\)"),
            IRegex::space_zero_or_more(),
            IRegex::named("KEYWORD", "(then|is)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(r"\("),
                IRegex::named("YES", "(.+?)"),
                IRegex::leaf(r"\)"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::leaf("then")),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(color) = arg.get("COLOR", 0) {
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated, you must add <<{color}>> at the end of the line"
                )]);
            }
            d.else_branch(arg.get("YES", 0).map(|y| Display::with_newlines(Some(y))))
        },
    )
}

/// `endif`: close the `if`.
fn end_if() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("end"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("if"),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, _| d.end_if(),
    )
}

/// `repeat [:label;]`: start a repeat loop.
fn repeat_start() -> Cmd {
    single(
        vec![
            IRegex::start(),
            crate::klimt::color::parser::exp4(),
            IRegex::leaf("repeat"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf(":"),
                IRegex::named("LABEL", "(.*?)"),
                IRegex::leaf(";"),
                IRegex::space_zero_or_more(),
            ])),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            let label = arg.get("LABEL", 0).map(|l| Display::with_newlines(Some(l)));
            if let Some(color) = arg.get("COLOR", 0) {
                let mut suggested = "repeat".to_string();
                if let Some(label) = &label {
                    let text = label.text_at(0);
                    suggested.push_str(" :");
                    suggested.push_str(&text);
                    suggested.push(';');
                }
                if let Some(stereo) = arg.get("STEREOGROUP", 0) {
                    suggested.push(' ');
                    suggested.push_str(stereo);
                }
                let mut original = format!("{color}:repeat");
                if let Some(label) = &label {
                    let text = label.text_at(0);
                    original.push_str(" :");
                    original.push_str(&text);
                    original.push(';');
                }
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated and the color is ignored: write '{suggested} <<{color}>>' instead of '{original}'"
                )]);
            }
            let stereogroup = Stereogroup::build(arg);
            let colors = stereogroup.inner_colors();
            let back = colors.color(ColorType::Back).cloned();
            d.do_repeat(label, colors, back);
            CommandExecutionResult::ok()
        },
    )
}

/// `repeat while (test)`: close the loop.
fn repeat_while() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::leaf("repeat"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("while"),
            IRegex::space_zero_or_more(),
            IRegex::named("TEST", r"\(([^)]*)\)"),
            IRegex::leaf(".*"),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| d.end_repeat(Display::with_newlines(arg.get("TEST", 0))),
    )
}

/// `while (test)`: start a loop.
fn while_start() -> Cmd {
    single(
        vec![
            IRegex::start(),
            crate::klimt::color::parser::exp4(),
            IRegex::leaf("while"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::named("TEST", "(.*?)"),
            IRegex::leaf(r"\)"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_zero_or_more(),
                IRegex::named("IS", "(is|equals?)"),
                IRegex::space_zero_or_more(),
                IRegex::named("YES", r"\((.+?)\)"),
            ])),
            IRegex::leaf(";?"),
            IRegex::space_zero_or_more(),
            Stereogroup::optional_stereogroup(),
            IRegex::end(),
        ],
        |d, arg| {
            if let Some(color) = arg.get("COLOR", 0) {
                d.base.add_warning(vec![format!(
                    "This syntax is deprecated, you must add <<{color}>> at the end of the line"
                )]);
            }
            let colors = Stereogroup::build(arg).inner_colors();
            let back = colors.color(ColorType::Back).cloned();
            d.do_while(Display::with_newlines(arg.get("TEST", 0)), back);
            CommandExecutionResult::ok()
        },
    )
}

/// `endwhile`: close the loop.
fn while_end() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::or(vec![
                IRegex::leaf("end[%s]*while"),
                IRegex::leaf("while[%s]*end"),
            ]),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("OUT", r"\((.+?)\)")),
            IRegex::leaf(";?"),
            IRegex::end(),
        ],
        |d, _| d.end_while(),
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
    // CommandSwimlane, CommandSwimlane2
    cmds.push(open_group());
    cmds.push(close_group());
    cmds.push(close_group_legacy());
    cmds.push(fork());
    cmds.push(fork_again());
    cmds.push(end_fork());
    cmds.push(while_start());
    cmds.push(while_end());
    cmds.push(repeat_start());
    cmds.push(repeat_while());
    cmds.push(if_start());
    cmds.push(if_legacy());
    cmds.push(else_branch());
    cmds.push(elseif_branch());
    cmds.push(end_if());
    cmds.push(switch_start());
    cmds.push(switch_case());
    cmds.push(end_switch());

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
    // Legacy arrows are tried last so that the multi-line arrow commands win.
    cmds.push(legacy_start());
    cmds.push(legacy_spot());
    cmds.push(legacy_spot_arrow());
    cmds.push(legacy_stop());
    cmds.push(legacy_arrow_spot());
    cmds.push(legacy_arrow_activity());
    cmds
}
