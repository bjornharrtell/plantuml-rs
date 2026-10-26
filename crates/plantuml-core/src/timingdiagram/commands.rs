//! Commands of timing diagrams (`timingdiagram/command`).

use super::ruler::TimeAxisStrategy;
use super::tick::{Decimal, TimeTick, TimingFormat};
use super::{StateKind, TimingDiagram};
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Position;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereotype};
use crate::text::{BlocLines, MultilinesStrategy};

type Cmd = Box<dyn Command<TimingDiagram>>;

const PLAYER_CODE: &str = r"([\p{L}_][%pLN_.]*)";
const STATE_CODE: &str = "([-%pLN_][-%pLN_.]*)";
const LINE_STYLE: &str = r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*";

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut TimingDiagram, &RegexResult) -> CommandExecutionResult + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut TimingDiagram, _, arg, _| f(d, arg),
    ))
}

fn s0() -> IRegex {
    IRegex::space_zero_or_more()
}

fn s1() -> IRegex {
    IRegex::space_one_or_more()
}

fn lf(p: &str) -> IRegex {
    IRegex::leaf(p)
}

fn nm(n: &str, p: &str) -> IRegex {
    IRegex::named(n, p)
}

fn opt(parts: Vec<IRegex>) -> IRegex {
    IRegex::optional(IRegex::group(parts))
}

fn stereotype_of(arg: &RegexResult, a: &str, b: &str) -> Option<Stereotype> {
    arg.get(a, 0)
        .or_else(|| arg.get(b, 0))
        .map(Stereotype::build)
}

fn back() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn colors(arg: &RegexResult) -> Option<Colors> {
    back().colors(arg)
}

// ---- time ticks ------------------------------------------------------------------------

fn time_expr_without_at(name: &str) -> IRegex {
    IRegex::or(vec![
        nm(&format!("{name}CODE"), r":([%pLN_.]+)([-+][.\d]+)?"),
        nm(&format!("{name}DATE"), r"(\d+)/(\d+)/(\d+)"),
        nm(&format!("{name}HOUR"), r"(\d+):(\d+):(\d+)"),
        nm(&format!("{name}DIGIT"), r"(\+?)(-?\d+\.?\d*)"),
        nm(&format!("{name}CLOCK"), r"([%pLN_.@]+)\*(\d+)"),
    ])
}

fn time_expr_at(name: &str) -> IRegex {
    IRegex::group(vec![lf("@"), time_expr_without_at(name)])
}

fn time_expr_optional_at(name: &str) -> IRegex {
    IRegex::optional(time_expr_at(name))
}

fn parse_int(s: Option<&str>) -> i64 {
    s.and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// `TimeTickBuilder.parseTimeTick`; `None` is a null tick.
fn parse_time_tick(name: &str, arg: &RegexResult, d: &TimingDiagram) -> Option<TimeTick> {
    if let Some(code) = arg.get(&format!("{name}CODE"), 0) {
        let result = d.code_value(code);
        return match arg.get(&format!("{name}CODE"), 1) {
            None => result,
            Some(delta) => {
                let r = result?;
                let value = r.time().add(&Decimal::parse(delta)?);
                Some(TimeTick::new(value, TimingFormat::Decimal))
            }
        };
    }
    if let Some(clock) = arg.get(&format!("{name}CLOCK"), 0) {
        let n = parse_int(arg.get(&format!("{name}CLOCK"), 1));
        return d.clock_value(clock, n);
    }
    if arg.get(&format!("{name}HOUR"), 0).is_some() {
        let key = format!("{name}HOUR");
        let (h, m, s) = (
            parse_int(arg.get(&key, 0)),
            parse_int(arg.get(&key, 1)),
            parse_int(arg.get(&key, 2)),
        );
        return Some(TimeTick::new(
            Decimal::from_i64(3600 * h + 60 * m + s),
            TimingFormat::Hour,
        ));
    }
    if arg.get(&format!("{name}DATE"), 0).is_some() {
        let key = format!("{name}DATE");
        return Some(TimingFormat::create_date(
            parse_int(arg.get(&key, 0)),
            parse_int(arg.get(&key, 1)),
            parse_int(arg.get(&key, 2)),
            d.timing_format_date(),
        ));
    }
    let key = format!("{name}DIGIT");
    let Some(number) = arg.get(&key, 1) else {
        return d.now();
    };
    let relative = arg.get(&key, 0) == Some("+");
    let mut value = Decimal::parse(number)?;
    if relative {
        if let Some(now) = d.now() {
            value = now.time().add(&value);
        }
    }
    Some(TimeTick::new(value, TimingFormat::Decimal))
}

// ---- players ---------------------------------------------------------------------------

fn code_pattern() -> &'static str {
    "([%pLN_.@]+)"
}

fn full_pattern() -> &'static str {
    "[%g]([^%g]+)[%g]"
}

fn compact_prefix() -> IRegex {
    opt(vec![nm("COMPACT", "(compact)"), s1()])
}

fn robust_concise() -> Cmd {
    single(
        vec![
            IRegex::start(),
            compact_prefix(),
            nm("TYPE", "(robust|concise|rectangle)"),
            s1(),
            opt(vec![
                nm("FULL", full_pattern()),
                optional_pattern("STEREOTYPE"),
                lf("as"),
                s1(),
            ]),
            nm("CODE", code_pattern()),
            optional_pattern("STEREOTYPE2"),
            s0(),
            IRegex::or(vec![back().regex()]),
            IRegex::end(),
        ],
        |d, arg| {
            let compact = arg.get("COMPACT", 0).is_some();
            let code = arg.get("CODE", 0).unwrap_or("");
            let full = arg.get("FULL", 0).unwrap_or("");
            let stereo = stereotype_of(arg, "STEREOTYPE", "STEREOTYPE2");
            let Some(colors) = colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            let kind = match arg.get("TYPE", 0).unwrap_or("").to_lowercase().as_str() {
                "robust" => StateKind::Robust,
                "concise" => StateKind::Concise,
                _ => StateKind::Rectangle,
            };
            let back = colors.color(ColorType::Back).cloned();
            d.create_player_state(kind, code, full, compact, stereo, back);
            CommandExecutionResult::ok()
        },
    )
}

fn clock() -> Cmd {
    let number = r"([0-9]+(?:\.[0-9]+)?)";
    single(
        vec![
            IRegex::start(),
            compact_prefix(),
            nm("TYPE", "clock"),
            s1(),
            opt(vec![nm("FULL", full_pattern()), s1(), lf("as"), s1()]),
            nm("CODE", code_pattern()),
            s1(),
            lf("with"),
            s1(),
            lf("period"),
            s1(),
            nm("PERIOD", number),
            opt(vec![s1(), lf("pulse"), s1(), nm("PULSE", number)]),
            opt(vec![s1(), lf("offset"), s1(), nm("OFFSET", number)]),
            s0(),
            optional_pattern("STEREO"),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("CODE", 0).unwrap_or("");
            let dec = |k: &str| {
                arg.get(k, 0)
                    .and_then(Decimal::parse)
                    .unwrap_or(Decimal::ZERO)
            };
            let Some(period) = arg.get("PERIOD", 0).and_then(Decimal::parse) else {
                return CommandExecutionResult::error("Bad period");
            };
            let stereo = arg.get("STEREO", 0).map(Stereotype::build);
            d.create_player_clock(
                code,
                arg.get("FULL", 0).unwrap_or(""),
                period,
                dec("PULSE"),
                dec("OFFSET"),
                stereo,
            );
            CommandExecutionResult::ok()
        },
    )
}

fn analog() -> Cmd {
    let signed = r"(-?[0-9]*\.?[0-9]+)";
    single(
        vec![
            IRegex::start(),
            compact_prefix(),
            lf("analog"),
            s1(),
            nm("FULL", full_pattern()),
            optional_pattern("STEREOTYPE"),
            opt(vec![
                IRegex::or(vec![lf("between"), lf("from")]),
                s1(),
                nm("START", signed),
                s1(),
                IRegex::or(vec![lf("and"), lf("to")]),
                s1(),
                nm("END", signed),
                s1(),
            ]),
            lf("as"),
            s1(),
            nm("CODE", code_pattern()),
            optional_pattern("STEREOTYPE2"),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("CODE", 0).unwrap_or("");
            let stereo = stereotype_of(arg, "STEREOTYPE", "STEREOTYPE2");
            let idx = d.create_player_analog(code, arg.get("FULL", 0).unwrap_or(""), stereo);
            if let (Some(start), Some(end)) = (arg.get("START", 0), arg.get("END", 0)) {
                if let super::PlayerKind::Analog { series, .. } = &mut d.player_mut(idx).kind {
                    if let Err(e) = series.set_bounds(start, end) {
                        return CommandExecutionResult::error(e);
                    }
                }
            }
            CommandExecutionResult::ok()
        },
    )
}

fn binary() -> Cmd {
    single(
        vec![
            IRegex::start(),
            compact_prefix(),
            lf("binary"),
            s1(),
            opt(vec![
                nm("FULL", full_pattern()),
                optional_pattern("STEREOTYPE"),
                lf("as"),
                s1(),
            ]),
            nm("CODE", code_pattern()),
            optional_pattern("STEREOTYPE2"),
            IRegex::end(),
        ],
        |d, arg| {
            let stereo = stereotype_of(arg, "STEREOTYPE", "STEREOTYPE2");
            d.create_player_binary(
                arg.get("CODE", 0).unwrap_or(""),
                arg.get("FULL", 0).unwrap_or(""),
                stereo,
            );
            CommandExecutionResult::ok()
        },
    )
}

// ---- states ----------------------------------------------------------------------------

fn state_or_hidden() -> IRegex {
    IRegex::or(vec![
        nm("STATE1", "[%g]([^%g]*)[%g]"),
        nm("STATE2", STATE_CODE),
        nm("STATE3", r"(\{hidden\})"),
        nm("STATE4", r"(\{\.\.\.\})"),
        nm("STATE5", r"(\{-\})"),
        nm("STATE6", r"(\{\?\})"),
        nm("STATE7", &format!(r"(?:\{{{STATE_CODE},{STATE_CODE}\}})")),
    ])
}

fn states_of(arg: &RegexResult) -> Vec<String> {
    if let Some(s1) = arg.get("STATE7", 0) {
        return vec![
            s1.to_string(),
            arg.get("STATE7", 1).unwrap_or("").to_string(),
        ];
    }
    vec![arg.get_lazzy("STATE", 0).unwrap_or("").to_string()]
}

fn comment_part() -> IRegex {
    IRegex::group(vec![
        s0(),
        back().regex(),
        s0(),
        IRegex::optional(IRegex::group(vec![
            s0(),
            lf(":"),
            s0(),
            nm("COMMENT", "(.*?)"),
        ])),
        s0(),
    ])
}

fn add_state(
    d: &mut TimingDiagram,
    arg: &RegexResult,
    player: usize,
    now: Option<TimeTick>,
) -> CommandExecutionResult {
    let comment = arg.get("COMMENT", 0);
    let Some(colors) = colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    match d
        .player_mut(player)
        .set_state(now, comment, Some(colors), &states_of(arg))
    {
        Ok(()) => CommandExecutionResult::ok(),
        Err(e) => CommandExecutionResult::error(e),
    }
}

fn change_state_by_player() -> Cmd {
    let mut parts = vec![
        IRegex::start(),
        nm("CODE", PLAYER_CODE),
        s0(),
        lf("is"),
        s0(),
        state_or_hidden(),
    ];
    parts.push(comment_part());
    parts.push(IRegex::end());
    single(parts, |d, arg| {
        let code = arg.get("CODE", 0).unwrap_or("");
        let Some(idx) = d.player_index(code) else {
            return CommandExecutionResult::error(format!("Unknown \"{code}\""));
        };
        let now = d.now();
        add_state(d, arg, idx, now)
    })
}

fn change_state_by_time() -> Cmd {
    let parts = vec![
        IRegex::start(),
        s0(),
        time_expr_without_at("TIME"),
        s0(),
        lf("is"),
        s0(),
        state_or_hidden(),
        comment_part(),
        IRegex::end(),
    ];
    single(parts, |d, arg| {
        let Some(idx) = d.last_player() else {
            return CommandExecutionResult::error("Missing @ line before this");
        };
        let Some(tick) = parse_time_tick("TIME", arg, d) else {
            return CommandExecutionResult::error("What time?");
        };
        d.add_time(tick.clone(), None);
        add_state(d, arg, idx, Some(tick))
    })
}

fn define_state_long() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PLAYER", code_pattern()),
            s1(),
            lf("has"),
            s1(),
            nm("LABEL", full_pattern()),
            s1(),
            lf("as"),
            s1(),
            nm("STATE", code_pattern()),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("Unknown {code}"));
            };
            let r = d.player_mut(idx).define_state(
                arg.get("STATE", 0).unwrap_or(""),
                arg.get("LABEL", 0).unwrap_or(""),
            );
            r.map_or_else(CommandExecutionResult::error, |()| {
                CommandExecutionResult::ok()
            })
        },
    )
}

fn define_state_short() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PLAYER", code_pattern()),
            s1(),
            lf("has"),
            s1(),
            nm("STATE", "([-%pLN_.@]+)"),
            nm("STATES", "((,([-%pLN_.@]+))*)"),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("Unknown {code}"));
            };
            let mut all = vec![arg.get("STATE", 0).unwrap_or("").to_string()];
            if let Some(states) = arg.get("STATES", 0) {
                all.extend(
                    states
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                );
            }
            for s in all {
                if let Err(e) = d.player_mut(idx).define_state(&s, &s) {
                    return CommandExecutionResult::error(e);
                }
            }
            CommandExecutionResult::ok()
        },
    )
}

// ---- time and player selection ---------------------------------------------------------

fn at_time() -> Cmd {
    single(
        vec![
            IRegex::start(),
            time_expr_at("TIME"),
            opt(vec![
                s1(),
                lf("as"),
                s1(),
                lf(":"),
                nm("CODE", r"([%pLN_.]+)"),
            ]),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let Some(tick) = parse_time_tick("TIME", arg, d) else {
                return CommandExecutionResult::error("What time?");
            };
            d.add_time(tick, arg.get("CODE", 0));
            CommandExecutionResult::ok()
        },
    )
}

fn at_player() -> Cmd {
    single(
        vec![
            IRegex::start(),
            lf("@"),
            nm("PLAYER", PLAYER_CODE),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or("");
            match d.player_index(code) {
                None => CommandExecutionResult::error(format!("No such participant {code}")),
                Some(i) => {
                    d.set_last_player(i);
                    CommandExecutionResult::ok()
                }
            }
        },
    )
}

fn time_message() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PART1", PLAYER_CODE),
            time_expr_optional_at("TIME1"),
            s0(),
            nm("ARROW_BODY", "(-+)"),
            nm("ARROW_STYLE", &format!(r"(?:\[({LINE_STYLE})\])?")),
            nm("ARROW_HEAD", r"\>"),
            s0(),
            nm("PART2", PLAYER_CODE),
            time_expr_optional_at("TIME2"),
            s0(),
            IRegex::optional(IRegex::group(vec![lf(":"), s0(), nm("MESSAGE", "(.*)")])),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let p1 = arg.get("PART1", 0).unwrap_or("");
            let Some(i1) = d.player_index(p1) else {
                return CommandExecutionResult::error(format!("No such element: {p1}"));
            };
            let p2 = arg.get("PART2", 0).unwrap_or("");
            let Some(i2) = d.player_index(p2) else {
                return CommandExecutionResult::error(format!("No such element: {p2}"));
            };
            let t1 = parse_time_tick("TIME1", arg, d);
            let t2 = parse_time_tick("TIME2", arg, d);
            let style = arg.get_lazzy("ARROW_STYLE", 0).map(str::to_string);
            d.create_time_message(i1, t1, i2, t2, arg.get("MESSAGE", 0))
                .apply_style(style.as_deref());
            CommandExecutionResult::ok()
        },
    )
}

fn constraint() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PART1", &format!("({PLAYER_CODE})?")),
            time_expr_at("TIME1"),
            s0(),
            lf(r"\<"),
            lf("(-+)"),
            nm("ARROW_STYLE1", &format!(r"(?:\[({LINE_STYLE})\])?")),
            lf("(-*)"),
            lf(r"\>"),
            s0(),
            time_expr_at("TIME2"),
            s0(),
            IRegex::optional(IRegex::group(vec![lf(":"), s0(), nm("MESSAGE", "(.*)")])),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let player = match arg.get("PART1", 0) {
                None => match d.last_player() {
                    Some(p) => p,
                    None => {
                        return CommandExecutionResult::error("You have to provide a participant")
                    }
                },
                Some(code) => match d.player_index(code) {
                    Some(p) => p,
                    None => {
                        return CommandExecutionResult::error(format!("No such participant {code}"))
                    }
                },
            };
            let Some(tick1) = parse_time_tick("TIME1", arg, d) else {
                return CommandExecutionResult::error("Unknown time label");
            };
            let restore = d.now();
            d.update_now(Some(tick1.clone()));
            let tick2 = parse_time_tick("TIME2", arg, d);
            d.update_now(restore);
            let Some(tick2) = tick2 else {
                return CommandExecutionResult::error("Unknown time label");
            };
            let config = crate::skin::arrow::ArrowConfiguration::with_direction_both();
            let Some(config) = crate::sequencediagram::commands::apply_arrow_style(
                arg.get_lazzy("ARROW_STYLE", 0),
                config,
            ) else {
                return CommandExecutionResult::bad_color();
            };
            let color = config.color().cloned();
            d.player_mut(player)
                .create_constraint(tick1, tick2, arg.get("MESSAGE", 0), color)
                .map_or_else(CommandExecutionResult::error, |()| {
                    CommandExecutionResult::ok()
                })
        },
    )
}

// ---- notes -----------------------------------------------------------------------------

fn note() -> Cmd {
    single(
        vec![
            IRegex::start(),
            s0(),
            lf("note"),
            s1(),
            nm("POSITION", "(top|bottom)"),
            s1(),
            lf("of"),
            s1(),
            nm("CODE", PLAYER_CODE),
            s0(),
            nm("TAGS", &format!("{}?", crate::abel::Stereotag::pattern())),
            optional_pattern("STEREO"),
            s0(),
            lf(":"),
            s0(),
            nm("NOTE", "(.+)"),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("CODE", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("Unknown \"{code}\""));
            };
            let display = Display::with_newlines(arg.get("NOTE", 0));
            let now = d.now();
            let stereo = arg.get("STEREO", 0).map(Stereotype::build);
            let pos = Position::from_string(arg.get("POSITION", 0).unwrap_or("top"))
                .unwrap_or(Position::Top);
            d.player_mut(idx).add_note(now, display, pos, stereo);
            CommandExecutionResult::ok()
        },
    )
}

fn note_long() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        s0(),
        lf("note"),
        s1(),
        nm("POSITION", "(top|bottom)"),
        s1(),
        lf("of"),
        s1(),
        nm("CODE", PLAYER_CODE),
        s0(),
        optional_pattern("STEREO"),
        s0(),
        IRegex::end(),
    ]);
    let starting = regex.clone();
    Box::new(MultiLineCommand::new(
        regex,
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        "^end[%s]?note$",
        move |d: &mut TimingDiagram, lines: &BlocLines, _| {
            let Some(line0) = starting.matcher(lines.at(0).trimmed().string()) else {
                return CommandExecutionResult::error("Cannot parse the note");
            };
            let body = lines.sub_extract(1, 1).remove_empty_columns();
            let code = line0.get("CODE", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("Unknown \"{code}\""));
            };
            let display = Display::create(&body.as_strings());
            let now = d.now();
            let stereo = line0.get("STEREO", 0).map(Stereotype::build);
            let pos = Position::from_string(line0.get("POSITION", 0).unwrap_or("top"))
                .unwrap_or(Position::Top);
            d.player_mut(idx).add_note(now, display, pos, stereo);
            CommandExecutionResult::ok()
        },
    ))
}

// ---- settings --------------------------------------------------------------------------

fn scale_pixel() -> Cmd {
    single(
        vec![
            IRegex::start(),
            lf("scale"),
            s1(),
            nm("TICK", r"(\d+)"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_zero_or_one(),
                nm("UNIT", "([smhdDyY])"),
            ])),
            s1(),
            lf("as"),
            s1(),
            nm("PIXEL", r"(\d+)"),
            s1(),
            lf("pixels?"),
            s0(),
            IRegex::end(),
        ],
        |d, arg| {
            let factor = match arg.get("UNIT", 0) {
                Some("m") => 60,
                Some("h") => 3600,
                Some("D" | "d") => 3600 * 24,
                Some("Y" | "y") => 3600 * 8766,
                _ => 1,
            };
            let tick = parse_int(arg.get("TICK", 0)) * factor;
            let pixel = parse_int(arg.get("PIXEL", 0));
            if tick <= 0 || pixel <= 0 {
                return CommandExecutionResult::error("Bad value");
            }
            d.scale_in_pixels(tick, pixel);
            CommandExecutionResult::ok()
        },
    )
}

fn hide_time_axis() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("COMMAND", "(hide|manual)"),
            s1(),
            lf("time"),
            lf(".?"),
            lf("axis"),
            IRegex::end(),
        ],
        |d, arg| {
            if arg
                .get("COMMAND", 0)
                .is_some_and(|c| c.eq_ignore_ascii_case("manual"))
            {
                d.set_time_axis_strategy(TimeAxisStrategy::Manual)
            } else {
                d.set_time_axis_strategy(TimeAxisStrategy::Hidden)
            }
        },
    )
}

fn highlight() -> Cmd {
    single(
        vec![
            IRegex::start(),
            s0(),
            lf("highlight"),
            s1(),
            time_expr_without_at("FROM"),
            s1(),
            lf("to"),
            s1(),
            time_expr_without_at("TO"),
            s0(),
            back().regex(),
            s0(),
            IRegex::optional(IRegex::group(vec![lf(":"), s0(), nm("CAPTION", "(.*)")])),
            IRegex::end(),
        ],
        |d, arg| {
            let (Some(from), Some(to)) = (
                parse_time_tick("FROM", arg, d),
                parse_time_tick("TO", arg, d),
            ) else {
                return CommandExecutionResult::error("Unknown time label");
            };
            let Some(colors) = colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            d.highlight(
                from,
                to,
                Display::with_newlines(arg.get("CAPTION", 0)),
                colors,
            )
        },
    )
}

fn mode_compact() -> Cmd {
    single(
        vec![
            IRegex::start(),
            lf("mode"),
            s1(),
            lf("compact"),
            IRegex::end(),
        ],
        |d, _| {
            d.go_compact_mode();
            CommandExecutionResult::ok()
        },
    )
}

fn ticks() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PLAYER", code_pattern()),
            s1(),
            lf("ticks"),
            s1(),
            IRegex::or(vec![
                lf("every"),
                IRegex::group(vec![lf("num"), s1(), lf("on"), s1(), lf("multiple")]),
            ]),
            s1(),
            nm("NUM", "([0-9]+)"),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("No such participant {code}"));
            };
            if let super::PlayerKind::Analog { ticks_every, .. } = &mut d.player_mut(idx).kind {
                *ticks_every = arg.get("NUM", 0).and_then(|n| n.parse().ok());
            }
            CommandExecutionResult::ok()
        },
    )
}

fn pixel_height() -> Cmd {
    single(
        vec![
            IRegex::start(),
            nm("PLAYER", code_pattern()),
            s1(),
            lf("is"),
            s1(),
            nm("NUM", "([0-9]+)"),
            s1(),
            lf("pixels?"),
            s1(),
            lf("height"),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("PLAYER", 0).unwrap_or("");
            let Some(idx) = d.player_index(code) else {
                return CommandExecutionResult::error(format!("No such participant {code}"));
            };
            d.player_mut(idx).suggested_height = parse_int(arg.get("NUM", 0)) as i32;
            CommandExecutionResult::ok()
        },
    )
}

fn use_date_format() -> Cmd {
    single(
        vec![
            IRegex::start(),
            lf("use"),
            s0(),
            lf("date"),
            s0(),
            lf("format"),
            s0(),
            nm("FORMAT", full_pattern()),
            IRegex::end(),
        ],
        |d, arg| d.use_date_format(arg.get("FORMAT", 0).unwrap_or("")),
    )
}

fn footbox_ignored() -> Cmd {
    single(
        vec![
            IRegex::start(),
            lf("(hide|show)?"),
            s0(),
            lf("footbox"),
            IRegex::end(),
        ],
        |_, _| CommandExecutionResult::ok(),
    )
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = crate::command::common::common_commands();
    cmds.push(footbox_ignored());
    cmds.push(robust_concise());
    cmds.push(clock());
    cmds.push(analog());
    cmds.push(binary());
    cmds.push(define_state_short());
    cmds.push(define_state_long());
    cmds.push(change_state_by_player());
    cmds.push(change_state_by_time());
    cmds.push(at_time());
    cmds.push(at_player());
    cmds.push(time_message());
    cmds.push(note());
    cmds.push(note_long());
    cmds.push(constraint());
    cmds.push(scale_pixel());
    cmds.push(hide_time_axis());
    cmds.push(highlight());
    cmds.push(mode_compact());
    cmds.push(ticks());
    cmds.push(pixel_height());
    cmds.push(use_date_format());
    cmds
}
