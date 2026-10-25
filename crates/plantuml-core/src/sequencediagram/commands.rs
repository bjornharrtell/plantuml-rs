//! Parsing of sequence diagram lines (`sequencediagram/command`).

use super::model::*;
use super::{new_message, SequenceDiagram};
use crate::command::{
    Command, CommandExecutionResult, MultiLineCommand, ParserPass, SingleLineCommand, Trim,
};
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::parser as color_parser;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::skin::arrow::{ArrowBody, ArrowConfiguration, ArrowDecoration, ArrowHead, ArrowPart};
use crate::stereo::{optional_pattern, Stereotype};
use crate::text::{BlocLines, LineLocation, MultilinesStrategy};
use crate::url::{self, UrlBuilder, UrlMode, URL_KEY};

type Cmd = Box<dyn Command<SequenceDiagram>>;

const CODE: &str = "([%pLN_.@]+)";

fn order_regex() -> IRegex {
    IRegex::optional(IRegex::group(vec![
        IRegex::leaf("order"),
        IRegex::space_one_or_more(),
        IRegex::named("ORDER", r"(-?\d{1,7})"),
    ]))
}

fn type_regex() -> IRegex {
    IRegex::or(vec![
        IRegex::named(
            "TYPE",
            "(participant|actor|create|boundary|control|entity|queue|database|collections)",
        ),
        IRegex::named(
            "CREATE",
            "create[%s](participant|actor|boundary|control|entity|queue|database|collections)",
        ),
    ])
}

fn participant_tail() -> Vec<IRegex> {
    vec![
        optional_pattern("STEREO"),
        order_regex(),
        IRegex::space_zero_or_more(),
        url::optional(),
        IRegex::space_zero_or_more(),
        color_parser::exp1(),
        IRegex::end(),
    ]
}

fn participant_regex(middle: Vec<IRegex>) -> IRegex {
    let mut v = vec![IRegex::start(), type_regex(), IRegex::space_one_or_more()];
    v.extend(middle);
    v.extend(participant_tail());
    IRegex::concat(v)
}

fn quoted_full() -> IRegex {
    IRegex::named("FULL", "[%g]([^%g]+)[%g]")
}

/// Creates the participant described by the groups (`CommandParticipant.executeArg`).
fn execute_participant(
    d: &mut SequenceDiagram,
    location: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let code = arg.get("CODE", 0).expect("code").to_string();
    if d.participants_contains_key(&code) {
        d.put_participant_in_last(&code);
        return CommandExecutionResult::ok();
    }
    let strings = arg.get("FULL", 0).map(|f| Display::with_newlines(Some(f)));
    let (ptype, create) = match (arg.get("CREATE", 0), arg.get("TYPE", 0)) {
        (Some(c), _) => (ParticipantType::parse(c).expect("type"), true),
        (None, Some(t)) if t.eq_ignore_ascii_case("create") => (ParticipantType::Participant, true),
        (None, Some(t)) => (ParticipantType::parse(t).expect("type"), false),
        _ => unreachable!("type or create"),
    };
    let order = arg.get("ORDER", 0).map_or(0, |o| o.parse().unwrap_or(0));
    let id = d.create_new_participant(location, ptype, &code, strings, order);
    finish_participant(d, id, arg);
    if create {
        if let Some(err) = d.activate(id, LifeEventType::Create, None, None) {
            return CommandExecutionResult::error(err);
        }
    }
    CommandExecutionResult::ok()
}

fn finish_participant(d: &mut SequenceDiagram, id: usize, arg: &RegexResult) {
    if let Some(st) = arg.get("STEREO", 0) {
        let top = !d.base.skin_param.value_is("stereotypePosition", "bottom");
        let stereotype = Stereotype::build_complex(st, circled_radius(d));
        d.configure_participant(id, |p| p.set_stereotype(stereotype, top));
    }
    let back = arg
        .get("COLOR", 0)
        .and_then(crate::klimt::color::HColor::parse);
    let url = arg.get(URL_KEY, 0).and_then(|u| {
        UrlBuilder::new(d.base.skin_param.get_value("topurl"), UrlMode::Strict).url(u)
    });
    d.configure_participant(id, |p| {
        p.set_specific_back_color(back);
        if let Some(u) = url {
            p.set_url(u);
        }
    });
}

/// `circledCharacterRadius` default: font size of the spot / 3 + 6.
fn circled_radius(d: &SequenceDiagram) -> f64 {
    let v = d.base.skin_param.as_int("circledCharacterRadius", -1);
    if v != -1 {
        return f64::from(v);
    }
    let size = d.base.skin_param.as_int("circledcharacterfontsize", 17);
    f64::from(size / 3 + 6)
}

fn participant_commands() -> Vec<Cmd> {
    let a = participant_regex(vec![
        IRegex::optional(IRegex::group(vec![
            quoted_full(),
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
        ])),
        IRegex::named("CODE", CODE),
    ]);
    let a2 = participant_regex(vec![
        IRegex::named("CODE", CODE),
        IRegex::space_one_or_more(),
        IRegex::leaf("as"),
        IRegex::space_one_or_more(),
        quoted_full(),
    ]);
    let a3 = participant_regex(vec![
        IRegex::named("FULL", CODE),
        IRegex::space_one_or_more(),
        IRegex::leaf("as"),
        IRegex::space_one_or_more(),
        IRegex::named("CODE", CODE),
    ]);
    let a4 = participant_regex(vec![IRegex::named("CODE", "[%g]([^%g]+)[%g]")]);
    let mut cmds: Vec<Cmd> = Vec::new();
    for r in [a, a2, a3, a4] {
        cmds.push(Box::new(SingleLineCommand::new(r, |d, loc, arg, _| {
            execute_participant(d, loc, arg)
        })));
    }
    let starting = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", "(participant)"),
        IRegex::space_one_or_more(),
        IRegex::named("CODE", CODE),
        optional_pattern("STEREO"),
        order_regex(),
        IRegex::space_zero_or_more(),
        url::optional(),
        IRegex::space_zero_or_more(),
        color_parser::exp1(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\["),
        IRegex::end(),
    ]);
    cmds.push(Box::new(MultiLineCommand::new(
        starting.clone(),
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        r"^([^\[\]]*)\]$",
        move |d: &mut SequenceDiagram, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let arg = starting.matcher(first.string()).expect("matches");
            let code = arg.get("CODE", 0).expect("code").to_string();
            if d.participants_contains_key(&code) {
                d.put_participant_in_last(&code);
                return CommandExecutionResult::ok();
            }
            let body = lines.sub_extract(1, 1).remove_empty_columns();
            let strings = Display::create(&body.as_strings());
            let order = arg.get("ORDER", 0).map_or(0, |o| o.parse().unwrap_or(0));
            let id = d.create_new_participant(
                lines.location(),
                ParticipantType::Participant,
                &code,
                Some(strings),
                order,
            );
            if let Some(st) = arg.get("STEREO", 0) {
                let top = !d.base.skin_param.value_is("stereotypePosition", "bottom");
                let stereotype = Stereotype::build_complex(st, circled_radius(d));
                d.configure_participant(id, |p| p.set_stereotype(stereotype, top));
            }
            if let Some(u) = arg.get(URL_KEY, 0) {
                let url =
                    UrlBuilder::new(d.base.skin_param.get_value("topurl"), UrlMode::Strict).url(u);
                if let Some(url) = url {
                    d.configure_participant(id, |p| p.set_url(url));
                }
            }
            CommandExecutionResult::ok()
        },
    )));
    cmds
}

const ANCHOR: &str = r"(\{([%pLN_]+)\}[%s]+)?";
const LINE_STYLE_KEYS: &str = r"dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+";
const LINE_STYLE_KEYS2: &str =
    r",dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+";

fn color_or_style_pattern() -> String {
    format!(r"(?:\[((?:#\w+|{LINE_STYLE_KEYS})(?:,#\w+|{LINE_STYLE_KEYS2})*)\])?")
}

fn part_regex(n: &str) -> IRegex {
    IRegex::or_named(
        n,
        vec![
            IRegex::named(&format!("{n}CODE"), "([%pLN_.@]+)"),
            IRegex::named(&format!("{n}LONG"), "[%g]([^%g]+)[%g]"),
            IRegex::named(
                &format!("{n}LONGCODE"),
                "[%g]([^%g]+)[%g][%s]*as[%s]+([%pLN_.@]+)",
            ),
            IRegex::named(
                &format!("{n}CODELONG"),
                "([%pLN_.@]+)[%s]+as[%s]*[%g]([^%g]+)[%g]",
            ),
        ],
    )
}

fn arrow_regex() -> IRegex {
    IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("PARALLEL", "(&[%s]*)?"),
        IRegex::named("ANCHOR", ANCHOR),
        part_regex("PART1"),
        IRegex::named("PART1ANCHOR", ANCHOR),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::or_named(
            "ARROW_DRESSING1",
            vec![
                IRegex::leaf("[%s][ox]"),
                IRegex::leaf(r"(?:[%s][ox]|\(\d+\))?<<?_?"),
                IRegex::leaf("(?:[%s][ox])?//?"),
                IRegex::leaf(r"(?:[%s][ox])?\\\\?"),
            ],
        )),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("ARROW_BODYA1", "(-+)"),
                IRegex::named("ARROW_STYLE1", &color_or_style_pattern()),
                IRegex::named("ARROW_BODYB1", "(-*)"),
            ]),
            IRegex::group(vec![
                IRegex::named("ARROW_BODYA2", "(-*)"),
                IRegex::named("ARROW_STYLE2", &color_or_style_pattern()),
                IRegex::named("ARROW_BODYB2", "(-+)"),
            ]),
        ]),
        IRegex::optional(IRegex::or_named(
            "ARROW_DRESSING2",
            vec![
                IRegex::leaf(r"_?>>?(?:[ox][%s]|\(\d+\))?"),
                IRegex::leaf("//?(?:[ox][%s])?"),
                IRegex::leaf(r"\\\\?(?:[ox][%s])?"),
                IRegex::leaf("[ox][%s]"),
            ],
        )),
        IRegex::space_zero_or_more(),
        part_regex("PART2"),
        IRegex::named("MULTICAST", r"((?:\s&\s[%pLN_.@]+)*)"),
        IRegex::named("PART2ANCHOR", ANCHOR),
        IRegex::space_zero_or_more(),
        IRegex::named("ACTIVATION", r"(?:(\+\+|\*\*|!!|--|--\+\+|\+\+--)?)"),
        IRegex::space_zero_or_more(),
        IRegex::named("LIFECOLOR", r"(?:(#\w+)?)"),
        optional_pattern("STEREOTYPE"),
        url::optional(),
        IRegex::space_zero_or_more(),
        IRegex::named("MESSAGE", "(?::[%s]*(.*))?"),
        IRegex::end(),
    ])
    .protect_size(2000)
}

fn endpoint(
    d: &mut SequenceDiagram,
    location: Option<LineLocation>,
    arg: &RegexResult,
    n: &str,
) -> usize {
    let (code, display) = if let Some(c) = arg.get(&format!("{n}CODE"), 0) {
        (c.to_string(), Display::with_newlines(Some(c)))
    } else if let Some(c) = arg.get(&format!("{n}LONG"), 0) {
        (c.to_string(), Display::with_newlines(Some(c)))
    } else if let Some(c) = arg.get(&format!("{n}LONGCODE"), 0) {
        let code = arg
            .get(&format!("{n}LONGCODE"), 1)
            .expect("code")
            .to_string();
        (code, Display::with_newlines(Some(c)))
    } else if let Some(c) = arg.get(&format!("{n}CODELONG"), 0) {
        let long = arg.get(&format!("{n}CODELONG"), 1).expect("long");
        (c.to_string(), Display::with_newlines(Some(long)))
    } else {
        unreachable!("an endpoint is always present");
    };
    d.get_or_create_participant(location, &code, Some(display))
}

fn dressing(arg: &RegexResult, key: &str) -> String {
    arg.get(key, 0)
        .unwrap_or("")
        .replace('_', "")
        .to_lowercase()
}

fn inclination(key: Option<&str>) -> i32 {
    let Some(key) = key else { return 0 };
    let (Some(x1), Some(x2)) = (key.find('('), key.find(')')) else {
        return 0;
    };
    key[x1 + 1..x2].parse().unwrap_or(0)
}

fn contains(s: &str, any: &[&str]) -> bool {
    any.iter().any(|t| s.contains(t))
}

fn body_length(arg: &RegexResult) -> usize {
    let a = arg.get_lazzy("ARROW_BODYA", 0).unwrap_or("");
    let b = arg.get_lazzy("ARROW_BODYB", 0).unwrap_or("");
    a.chars().count() + b.chars().count()
}

/// Applies `[#red,dashed]` style specs to an arrow (`CommandArrow.applyStyle`).
pub fn apply_arrow_style(
    style: Option<&str>,
    mut config: ArrowConfiguration,
) -> Option<ArrowConfiguration> {
    let Some(style) = style else {
        return Some(config);
    };
    for s in style.split(',').filter(|s| !s.is_empty()) {
        if s.eq_ignore_ascii_case("dashed") || s.eq_ignore_ascii_case("dotted") {
            config = config.with_body(ArrowBody::Dotted);
        } else if s.eq_ignore_ascii_case("bold") {
        } else if s.eq_ignore_ascii_case("hidden") {
            config = config.with_body(ArrowBody::Hidden);
        } else {
            config = config.with_color(Some(crate::klimt::color::HColor::parse(s)?));
        }
    }
    Some(config)
}

fn execute_arrow(
    d: &mut SequenceDiagram,
    location: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let d1 = dressing(arg, "ARROW_DRESSING1");
    let d2 = dressing(arg, "ARROW_DRESSING2");
    let inclination1 = inclination(arg.get("ARROW_DRESSING1", 0));
    let inclination2 = inclination(arg.get("ARROW_DRESSING2", 0));
    let has_d1_but_x = contains(&d1, &["<", "\\", "/"]);
    let x_in_d1 = d1.contains('x');
    let has_d2_but_x = contains(&d2, &[">", "\\", "/"]);
    let x_in_d2 = d2.contains('x');
    let reverse_define = if has_d2_but_x || (x_in_d1 && x_in_d2) {
        false
    } else if has_d1_but_x {
        true
    } else if x_in_d1 || x_in_d2 {
        false
    } else {
        return CommandExecutionResult::error("Illegal sequence arrow");
    };
    let (p1, p2, circle_at_start, circle_at_end, sync1, sync2);
    if reverse_define {
        p2 = endpoint(d, location, arg, "PART1");
        p1 = endpoint(d, location, arg, "PART2");
        circle_at_start = d2.contains('o');
        circle_at_end = d1.contains('o');
        sync2 = contains(&d1, &["<<", "\\\\", "//"]);
        sync1 = contains(&d2, &[">>", "\\\\", "//"]);
    } else {
        p1 = endpoint(d, location, arg, "PART1");
        p2 = endpoint(d, location, arg, "PART2");
        circle_at_start = d1.contains('o');
        circle_at_end = d2.contains('o');
        sync1 = contains(&d1, &["<<", "\\\\", "//"]);
        sync2 = contains(&d2, &[">>", "\\\\", "//"]);
    }
    let dotted = body_length(arg) > 1;
    let labels = match arg.get("MESSAGE", 0) {
        None => Display::create(&[""]),
        Some(m) => Display::with_newlines(Some(m)),
    };
    let mut config = if has_d1_but_x && has_d2_but_x {
        ArrowConfiguration::with_direction_both()
    } else {
        ArrowConfiguration::with_direction_normal()
    };
    if dotted {
        config = config.with_body(ArrowBody::Dotted);
    }
    if sync1 {
        config = config.with_head1(ArrowHead::Async);
    }
    if sync2 {
        config = config.with_head2(ArrowHead::Async);
    }
    if d2.contains('\\') || d1.contains('/') {
        config = config.with_part(ArrowPart::TopPart);
    }
    if d2.contains('/') || d1.contains('\\') {
        config = config.with_part(ArrowPart::BottomPart);
    }
    if circle_at_end {
        config = config.with_decoration2(ArrowDecoration::Circle);
    }
    if circle_at_start {
        config = config.with_decoration1(ArrowDecoration::Circle);
    }
    if reverse_define {
        if x_in_d1 {
            config = config.with_head2(ArrowHead::CrossX);
        }
        if x_in_d2 {
            config = config.with_head1(ArrowHead::CrossX);
        }
    } else {
        if x_in_d1 {
            config = config.with_head1(ArrowHead::CrossX);
        }
        if x_in_d2 {
            config = config.with_head2(ArrowHead::CrossX);
        }
    }
    if reverse_define {
        config = config.reverse_define();
    }
    let Some(styled) = apply_arrow_style(arg.get_lazzy("ARROW_STYLE", 0), config) else {
        return CommandExecutionResult::bad_color();
    };
    config = styled.with_inclination(inclination1 + inclination2);
    let activation = arg.get("ACTIVATION", 0).filter(|a| !a.is_empty());
    if activation.is_some_and(|a| a.starts_with('*')) {
        d.activate(p2, LifeEventType::Create, None, None);
    }
    let number = d.next_message_number();
    let label = d.manage_variable(&labels);
    let mut msg = new_message(d, p1, p2, label, config.clone(), number, location);
    let mut multicast = Vec::new();
    if let Some(mc) = arg.get("MULTICAST", 0) {
        for s in mc.split('&') {
            let s = s.trim();
            if !s.is_empty() {
                multicast.push(d.get_or_create_participant(location, s, None));
            }
        }
    }
    msg.multicast = multicast;
    if let Some(u) = arg.get(URL_KEY, 0) {
        msg.core.url =
            UrlBuilder::new(d.base.skin_param.get_value("topurl"), UrlMode::Strict).url(u);
    }
    if let Some(st) = arg.get("STEREOTYPE", 0) {
        msg.core.stereotype = Some(Stereotype::build(st));
    }
    if arg.get("PARALLEL", 0).is_some() {
        msg.core.parallel = true;
    }
    msg.core.anchor = arg.get("ANCHOR", 1).map(str::to_string);
    msg.core.part1_anchor = arg.get("PART1ANCHOR", 1).map(str::to_string);
    msg.core.part2_anchor = arg.get("PART2ANCHOR", 1).map(str::to_string);
    let status = d.add_message(Event::Message(msg));
    if !status.is_ok() {
        return status;
    }
    let life_color = arg
        .get("LIFECOLOR", 0)
        .and_then(crate::klimt::color::HColor::parse);
    if let Some(spec) = activation {
        manage_activations(spec, d, p1, p2, life_color);
        return CommandExecutionResult::ok();
    }
    if d.autoactivate && matches!(config.head(), ArrowHead::Normal | ArrowHead::Async) {
        if config.is_dotted() {
            d.activate(p1, LifeEventType::Deactivate, None, None);
        } else {
            d.activate(p2, LifeEventType::Activate, life_color, None);
        }
    }
    CommandExecutionResult::ok()
}

fn manage_activations(
    spec: &str,
    d: &mut SequenceDiagram,
    p1: usize,
    p2: usize,
    color: Option<crate::klimt::color::HColor>,
) {
    let chars: Vec<char> = spec.chars().collect();
    match chars[0] {
        '+' => {
            d.activate(p2, LifeEventType::Activate, color.clone(), None);
        }
        '-' => {
            d.activate(p1, LifeEventType::Deactivate, None, None);
        }
        '!' => {
            d.activate(p2, LifeEventType::Destroy, None, None);
        }
        _ => {}
    }
    if chars.len() == 4 {
        match chars[2] {
            '+' => {
                d.activate(p2, LifeEventType::Activate, color, None);
            }
            '-' => {
                d.activate(p1, LifeEventType::Deactivate, None, None);
            }
            _ => {}
        }
    }
}

fn autonumber_command() -> Cmd {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("autonumber"),
        IRegex::space_zero_or_more(),
        IRegex::named("START", r"(\d(?:(?:[^%pLN%s]+|\d+)*\d)?)?"),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("STEP", r"(\d+)"),
        ])),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("FORMAT", "[%g]([^%g]+)[%g]"),
        ])),
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        r,
        |d: &mut SequenceDiagram, _, arg, _| {
            let start = DottedNumber::create(arg.get("START", 0).unwrap_or("1"));
            let inc = arg.get("STEP", 0).map_or(1, |s| s.parse().unwrap_or(1));
            let df = arg.get("FORMAT", 0).unwrap_or("<b>0</b>");
            d.autonumber_go(start, inc, df);
            CommandExecutionResult::ok()
        },
    ))
}

fn who_regex() -> &'static str {
    "([%pLN_.@]+|[%g][^%g]+[%g])"
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
}

fn result_of(error: Option<String>) -> CommandExecutionResult {
    match error {
        None => CommandExecutionResult::ok(),
        Some(e) => CommandExecutionResult::error(e),
    }
}

fn simple<F>(r: IRegex, f: F) -> Cmd
where
    F: Fn(&mut SequenceDiagram, Option<LineLocation>, &RegexResult) -> CommandExecutionResult
        + 'static,
{
    Box::new(SingleLineCommand::new(
        r,
        move |d: &mut SequenceDiagram, loc, arg, _| f(d, loc, arg),
    ))
}

fn lifeline_commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("HIDE", "(hide|show)"),
            IRegex::space_one_or_more(),
            IRegex::leaf("@?unlinked"),
            IRegex::end(),
        ]),
        |d, _, arg| {
            d.hide_unlinked_data = arg
                .get("HIDE", 0)
                .is_some_and(|h| h.eq_ignore_ascii_case("hide"));
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("TYPE", "(activate|deactivate|destroy|create)"),
            IRegex::space_one_or_more(),
            IRegex::named("WHO", who_regex()),
            IRegex::space_zero_or_more(),
            IRegex::named("BACK", r"(#\w+)?"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::named("LINE", r"(#\w+)"),
            ])),
            IRegex::end(),
        ]),
        |d, loc, arg| {
            let kind = match arg.get("TYPE", 0).unwrap_or("").to_uppercase().as_str() {
                "ACTIVATE" => LifeEventType::Activate,
                "DEACTIVATE" => LifeEventType::Deactivate,
                "DESTROY" => LifeEventType::Destroy,
                _ => LifeEventType::Create,
            };
            let who = strip_quotes(arg.get("WHO", 0).unwrap_or(""));
            let p = d.get_or_create_participant(loc, who, None);
            let back = arg
                .get("BACK", 0)
                .and_then(crate::klimt::color::HColor::parse);
            let line = arg
                .get("LINE", 0)
                .and_then(crate::klimt::color::HColor::parse);
            result_of(d.activate(p, kind, back, line))
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("deactivate"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ]),
        |d, _, _| {
            let Some(m) = d.activating_message() else {
                return CommandExecutionResult::error("Nothing to deactivate.");
            };
            let p2 = match &d.events()[m] {
                Event::Message(m) => m.p2,
                Event::MessageExo(m) => m.participant,
                _ => unreachable!(),
            };
            result_of(d.activate(p2, LifeEventType::Deactivate, None, None))
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("NAME", CODE),
            IRegex::space_zero_or_more(),
            IRegex::named("TYPE", r"(\+\+|--)"),
            IRegex::space_zero_or_more(),
            IRegex::named("COLOR", r"(#\w+)?"),
            IRegex::end(),
        ]),
        |d, loc, arg| {
            let kind = if arg.get("TYPE", 0) == Some("++") {
                LifeEventType::Activate
            } else {
                LifeEventType::Deactivate
            };
            let p = d.get_or_create_participant(loc, arg.get("NAME", 0).unwrap_or(""), None);
            let color = arg
                .get("COLOR", 0)
                .and_then(crate::klimt::color::HColor::parse);
            result_of(d.activate(p, kind, color, None))
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("PARALLEL", "(&[%s]*)?"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("return"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::named("COLOR", r"(#\w+)"),
                IRegex::space_one_or_more(),
            ])),
            IRegex::named("MESSAGE", "(.*)"),
            IRegex::end(),
        ]),
        execute_return,
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("autoactivate"),
            IRegex::space_one_or_more(),
            IRegex::named("ON", "(off|on)"),
            IRegex::end(),
        ]),
        |d, _, arg| {
            d.autoactivate = arg
                .get("ON", 0)
                .is_some_and(|v| v.eq_ignore_ascii_case("on"));
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::named("TYPE", "(hide|show)?"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("footbox"),
            IRegex::end(),
        ]),
        |d, _, arg| {
            d.set_show_footbox(
                arg.get("TYPE", 0)
                    .is_some_and(|v| v.eq_ignore_ascii_case("show")),
            );
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(simple(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("footbox"),
            IRegex::space_zero_or_more(),
            IRegex::named("TYPE", "(on|off)?"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ]),
        |d, _, arg| {
            d.set_show_footbox(
                arg.get("TYPE", 0)
                    .is_some_and(|v| v.eq_ignore_ascii_case("on")),
            );
            CommandExecutionResult::ok()
        },
    ));
    cmds
}

fn execute_return(
    d: &mut SequenceDiagram,
    location: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let mut do_deactivation = true;
    let mut message1 = d.activating_message();
    if message1.is_none() {
        match d.last_event_with_deactivate() {
            Some(i) if matches!(d.events()[i], Event::Message(_)) => {
                message1 = Some(i);
                do_deactivation = false;
            }
            _ => return CommandExecutionResult::error("Nowhere to return to."),
        }
    }
    let m1 = message1.expect("set above");
    let core = d.events()[m1].message_core().expect("message").clone();
    let mut arrow = core.arrow_configuration.with_body(ArrowBody::Dotted);
    if let Some(c) = arg.get("COLOR", 0) {
        match crate::klimt::color::HColor::parse(c) {
            Some(c) => arrow = arrow.with_color(Some(c)),
            None => return CommandExecutionResult::bad_color(),
        }
    }
    let display = Display::with_newlines(arg.get("MESSAGE", 0));
    let number = d.next_message_number();
    let (message2, p2_of_1) = match d.events()[m1].clone() {
        Event::MessageExo(exo1) => {
            let msg = MessageExo {
                core: MessageCore::new(d.style_builder(), display, arrow, number, location),
                participant: exo1.participant,
                exo_type: exo1.exo_type.reverse(),
                short_arrow: false,
            };
            (Event::MessageExo(msg), exo1.participant)
        }
        Event::Message(m) => {
            let mut msg = Message {
                core: MessageCore::new(d.style_builder(), display, arrow, number, location),
                p1: m.p2,
                p2: m.p1,
                multicast: Vec::new(),
            };
            if arg.get("PARALLEL", 0).is_some() {
                msg.core.parallel = true;
            }
            (Event::Message(msg), m.p2)
        }
        _ => unreachable!(),
    };
    let status = d.add_message(message2);
    if !status.is_ok() {
        return status;
    }
    if do_deactivation {
        return result_of(d.activate(p2_of_1, LifeEventType::Deactivate, None, None));
    }
    CommandExecutionResult::ok()
}

fn layout_commands() -> Vec<Cmd> {
    vec![
        simple(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf("=="),
                IRegex::space_zero_or_more(),
                IRegex::named("LABEL", "(.*)"),
                IRegex::space_zero_or_more(),
                IRegex::leaf("=="),
                IRegex::end(),
            ]),
            |d, _, arg| {
                d.divider(Display::with_newlines(arg.get("LABEL", 0)));
                CommandExecutionResult::ok()
            },
        ),
        simple(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf(r"\|\|"),
                IRegex::named("VALUE", r"(\d+)?"),
                IRegex::leaf(r"\|+"),
                IRegex::end(),
            ]),
            |d, _, arg| {
                match arg.get("VALUE", 0).filter(|v| !v.is_empty()) {
                    Some(v) => d.hspace(v.parse().unwrap_or(25)),
                    None => d.hspace(25),
                }
                CommandExecutionResult::ok()
            },
        ),
        simple(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf("(?:\\.{3}|\u{2026})"),
                IRegex::optional(IRegex::named("LABEL", "(.*)(?:\\.{3}|\u{2026})")),
                IRegex::end(),
            ]),
            |d, _, arg| {
                let strings = match arg.get("LABEL", 0) {
                    None => Display::empty(),
                    Some(l) => Display::with_newlines(Some(l)),
                };
                d.delay(strings);
                CommandExecutionResult::ok()
            },
        ),
        simple(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf("ignore"),
                IRegex::space_zero_or_more(),
                IRegex::leaf("newpage"),
                IRegex::end(),
            ]),
            |d, _, _| {
                d.ignore_newpage();
                CommandExecutionResult::ok()
            },
        ),
        simple(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::leaf("@?"),
                IRegex::leaf("newpage"),
                IRegex::optional(IRegex::group(vec![
                    IRegex::leaf("(?:[%s]*:[%s]*|[%s]+)"),
                    IRegex::named("LABEL", "(.*[%pLN_.].*)"),
                ])),
                IRegex::end(),
            ]),
            |d, loc, arg| {
                let strings = match arg.get("LABEL", 0) {
                    None => Display::null(),
                    Some(l) => Display::with_newlines(Some(l)),
                };
                d.newpage(crate::klimt::creole::display::DisplayPositioned {
                    location: loc,
                    ..crate::klimt::creole::display::DisplayPositioned::single(
                        strings,
                        Some(crate::klimt::geom::HorizontalAlignment::Center),
                        Some(crate::klimt::geom::VerticalAlignment::Top),
                    )
                });
                CommandExecutionResult::ok()
            },
        ),
    ]
}

fn grouping_command() -> Cmd {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("PARALLEL", "(&[%s]*)?"),
        IRegex::named(
            "TYPE",
            "(opt|alt|loop|par|par2|break|critical|else|end|also|group|partition)",
        ),
        IRegex::named(
            "COLORS",
            r"((?<!else)(?<!also)(?<!end)#\w+)?(?:[%s]+(#\w+))?",
        ),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("COMMENT", "(.*?)"),
        ])),
        IRegex::end(),
    ]);
    simple(r, |d, _, arg| {
        let color = |i: usize| -> Result<Option<crate::klimt::color::HColor>, ()> {
            match arg.get("COLORS", i) {
                None => Ok(None),
                Some(c) => crate::klimt::color::HColor::parse(c).map(Some).ok_or(()),
            }
        };
        let (Ok(back_element), Ok(back_general)) = (color(0), color(1)) else {
            return CommandExecutionResult::bad_color();
        };
        let mut ty = arg.get("TYPE", 0).unwrap_or("").to_lowercase();
        let mut comment = arg.get("COMMENT", 0).map(str::to_string);
        let grouping_type = GroupingType::parse(&ty);
        if ty == "group" {
            match comment.as_deref() {
                None | Some("") => comment = Some("group".to_string()),
                Some(c) => {
                    if let Some((header, label)) = split_trailing_bracket(c) {
                        ty = header;
                        comment = Some(label);
                    }
                }
            }
        }
        let parallel = arg.get("PARALLEL", 0).is_some();
        if d.grouping(
            &ty,
            comment.as_deref(),
            grouping_type,
            back_general,
            back_element,
            parallel,
        ) {
            CommandExecutionResult::ok()
        } else {
            CommandExecutionResult::error("Cannot create group")
        }
    })
}

/// Splits `header [label]` of a `group` (`TRAILING_BRACKET_CONTENT_PATTERN`).
fn split_trailing_bracket(comment: &str) -> Option<(String, String)> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        fancy_regex::Regex::new(r"^(.*\[\[.*\]\].*?|.*?)\[(.*)\]$").expect("bracket pattern")
    });
    let caps = re.captures(comment).ok().flatten()?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

fn box_commands() -> Vec<Cmd> {
    let start = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("box"),
        IRegex::optional(IRegex::or(vec![
            IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::named("NAME1", "[%g]([^%g]+)[%g]"),
            ]),
            IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::named("NAME2", "([^#]+)"),
            ]),
        ])),
        optional_pattern("STEREO"),
        color_parser::ColorParser::simple_color(ColorType::Back).regex(),
        IRegex::end(),
    ]);
    let end = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("end"),
        IRegex::space_zero_or_more(),
        IRegex::leaf("box"),
        IRegex::end(),
    ]);
    vec![
        simple(start, |d, _, arg| {
            let stereotype = arg.get("STEREO", 0).map(Stereotype::build);
            let Some(colors) = color_parser::ColorParser::simple_color(ColorType::Back).colors(arg)
            else {
                return CommandExecutionResult::bad_color();
            };
            let title = arg.get_lazzy("NAME", 0).unwrap_or("");
            d.box_start(
                Display::with_newlines(Some(title)),
                colors.color(ColorType::Back).cloned(),
                stereotype,
            );
            CommandExecutionResult::ok()
        }),
        simple(end, |d, _, _| {
            if !d.is_box_pending() {
                return CommandExecutionResult::error("Missing starting box");
            }
            d.end_box();
            CommandExecutionResult::ok()
        }),
    ]
}

fn reference_participants(
    d: &mut SequenceDiagram,
    location: Option<LineLocation>,
    parts: &str,
) -> Vec<usize> {
    crate::string_utils::split_comma(parts)
        .iter()
        .map(|s| {
            d.get_or_create_participant(
                location,
                &crate::string_utils::eventually_remove_starting_and_ending_double_quote(s),
                None,
            )
        })
        .collect()
}

fn reference_url(d: &SequenceDiagram, arg: &RegexResult) -> Option<crate::url::Url> {
    arg.get(URL_KEY, 0).and_then(|u| {
        UrlBuilder::new(d.base.skin_param.get_value("topurl"), UrlMode::Strict).url(u)
    })
}

fn reference_commands() -> Vec<Cmd> {
    const PART: &str = "[%pLN_.@]+|[%g][^%g]+[%g]";
    let single = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("ref"),
        IRegex::named("REF", r"(#\w+)?"),
        IRegex::space_one_or_more(),
        IRegex::leaf("over"),
        IRegex::space_one_or_more(),
        IRegex::named("PARTS", &format!("(({PART})([%s]*,[%s]*({PART}))*)")),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named(URL_KEY, r"(\[\[.*?\]\])")),
        IRegex::space_zero_or_more(),
        IRegex::leaf(":"),
        IRegex::space_zero_or_more(),
        IRegex::named("TEXT", "(.*)"),
        IRegex::end(),
    ]);
    let multi = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("ref"),
        IRegex::named("REF", r"(#\w+)?"),
        IRegex::space_one_or_more(),
        IRegex::leaf("over"),
        IRegex::space_one_or_more(),
        IRegex::named("PARTS", &format!("((?:{PART})(?:[%s]*,[%s]*(?:{PART}))*)")),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named(URL_KEY, r"(\[\[.*?\]\])")),
        IRegex::space_zero_or_more(),
        IRegex::named("UNUSED", r"(#\w+)?"),
        IRegex::end(),
    ]);
    let back_color = |arg: &RegexResult| -> Result<Option<crate::klimt::color::HColor>, ()> {
        match arg.get("REF", 0) {
            None => Ok(None),
            Some(c) => crate::klimt::color::HColor::parse(c).map(Some).ok_or(()),
        }
    };
    let single_cmd = simple(single, move |d, loc, arg| {
        let Ok(back_element) = back_color(arg) else {
            return CommandExecutionResult::bad_color();
        };
        let participants = reference_participants(d, loc, arg.get("PARTS", 0).unwrap_or(""));
        let text = crate::string_utils::trin(arg.get("TEXT", 0).unwrap_or(""));
        let strings = Display::with_newlines(Some(text));
        let url = reference_url(d, arg);
        let reference = Reference::new(
            &participants,
            url,
            strings,
            None,
            back_element,
            d.style_builder(),
        );
        d.add_reference(reference);
        CommandExecutionResult::ok()
    });
    let starting = multi.clone();
    let multi_cmd: Cmd = Box::new(MultiLineCommand::new(
        multi,
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        "^end[%s]?(ref)?$",
        move |d: &mut SequenceDiagram, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let Some(arg) = starting.matcher(first.string()) else {
                return CommandExecutionResult::error(format!(
                    "Cannot parse line {}",
                    first.string()
                ));
            };
            let Ok(back_element) = back_color(&arg) else {
                return CommandExecutionResult::bad_color();
            };
            let participants =
                reference_participants(d, lines.location(), arg.get("PARTS", 0).unwrap_or(""));
            let body = lines.sub_extract(1, 1).remove_empty_columns();
            let strings = Display::create(&body.as_strings());
            let url = reference_url(d, &arg);
            let reference = Reference::new(
                &participants,
                url,
                strings,
                None,
                back_element,
                d.style_builder(),
            );
            d.add_reference(reference);
            CommandExecutionResult::ok()
        },
    ));
    vec![single_cmd, multi_cmd]
}

/// All commands of the sequence diagram, in matching order (`initCommandsList`, partial).
pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.extend(participant_commands());
    cmds.push(Box::new(SingleLineCommand::new(
        arrow_regex(),
        |d, loc, arg, _: ParserPass| execute_arrow(d, loc, arg),
    )));
    cmds.extend(lifeline_commands());
    cmds.extend(layout_commands());
    cmds.extend(box_commands());
    cmds.extend(reference_commands());
    cmds.extend(super::note_commands::note_commands());
    cmds.push(grouping_command());
    cmds.push(autonumber_command());
    cmds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{create_system, UmlSource};

    fn parse(text: &str) -> SequenceDiagram {
        let source = UmlSource::from_text(text);
        create_system(
            &commands(),
            SequenceDiagram::from_source(source.clone()),
            &source,
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn simple_message() {
        let d = parse("@startuml\nAlice -> Bob: hi\n@enduml");
        assert_eq!(d.participants().len(), 2);
        assert_eq!(d.participants()[0].code, "Alice");
        let Event::Message(m) = &d.events()[0] else {
            panic!()
        };
        assert_eq!((m.p1, m.p2), (0, 1));
        assert_eq!(m.core.label.text_at(0), "hi");
        assert!(!m.core.arrow_configuration.is_dotted());
    }

    #[test]
    fn declarations() {
        let d = parse(
            "@startuml\nparticipant \"Long name\" as L <<st>> #red\nactor A order 5\nactor B as \"B B\"\nentity E\nparticipant L\nL -> A\n@enduml",
        );
        let codes: Vec<_> = d.participants().iter().map(|p| p.code.as_str()).collect();
        assert!(codes.contains(&"L") && codes.contains(&"A") && codes.contains(&"B"));
        assert_eq!(d.participants().last().unwrap().code, "A");
        let l = d.participants().iter().find(|p| p.code == "L").unwrap();
        assert!(l.stereotype().is_some());
        assert_eq!(l.display(false).text_at(1), "Long name");
    }

    #[test]
    fn arrow_variants() {
        let d = parse(
            "@startuml\nA -> B\nB --> A: back\nA ->> B\nA -\\ B\nA ->x B\nA <- B\nA ->o B\nB <-> A\nA -[#red]> B\nA -[hidden]> B\nA ->(10) B\nA -> A: self\nautonumber\nA -> B: n\nautonumber 5 2 \"<b>00</b>\"\nA -> B: m\n@enduml",
        );
        let msgs: Vec<_> = d
            .events()
            .iter()
            .filter_map(|e| {
                if let Event::Message(m) = e {
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(msgs.len(), 14);
        assert!(msgs[1].core.arrow_configuration.is_dotted());
        assert_eq!(msgs[12].core.message_number.as_deref(), Some("<b>1</b>"));
        assert_eq!(msgs[13].core.message_number.as_deref(), Some("<b>05</b>"));
        assert!(msgs[11].is_self_message());
        assert!(msgs[9].core.arrow_configuration.is_hidden());
    }

    #[test]
    fn activation_shortcuts() {
        let d = parse("@startuml\nA -> B ++ : x\nB -> A --\nA -> B !!\n@enduml");
        let kinds: Vec<_> = d
            .events()
            .iter()
            .filter_map(|e| {
                if let Event::LifeEvent(l) = e {
                    Some(l.kind)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            kinds,
            [
                LifeEventType::Activate,
                LifeEventType::Deactivate,
                LifeEventType::Destroy
            ]
        );
    }

    #[test]
    fn illegal_arrow_is_an_error() {
        let source = UmlSource::from_text("@startuml\nA -- B\n@enduml");
        let r = create_system(
            &commands(),
            SequenceDiagram::from_source(source.clone()),
            &source,
        );
        assert!(r.is_err());
    }
}
