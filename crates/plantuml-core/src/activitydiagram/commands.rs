//! Commands of legacy activity diagrams (`activitydiagram.command`).

use super::ActivityDiagram;
use crate::abel::{Entity, LeafType, Link, LinkArg};
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::decoration::link::{LinkDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::{exp2, exp3, ColorParser};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Direction;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereotype};
use crate::text::LineLocation;
use crate::url::{self, UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ActivityDiagram>>;

const LINE_STYLE: &str = r"dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+";

/// `StringUtils.manageArrowForCuca`: the arrow reduced to its dashes.
fn manage_arrow(arrow: &str, dir: Direction) -> String {
    let mut s: String = arrow
        .replace('=', "-")
        .chars()
        .filter(|c| !c.is_alphanumeric() && *c != '_')
        .collect();
    if matches!(dir, Direction::Left | Direction::Right) {
        let mut out = String::new();
        for c in s.chars() {
            if !(c == '-' && out.ends_with('-')) {
                out.push(c);
            }
        }
        s = out;
    }
    if s.chars().count() == 2 && matches!(dir, Direction::Up | Direction::Down) {
        s = s.replacen('-', "--", 1);
    }
    s
}

/// `StringUtils.getArrowDirection` of an arrow ending with `>`.
fn arrow_direction(arrow: &str) -> Direction {
    crate::string_utils::get_queue_direction(arrow.trim_end_matches('>'))
}

fn arrow_regex() -> Vec<IRegex> {
    vec![
        IRegex::named("ARROW_BODY1", "([-.]+)"),
        IRegex::named("ARROW_STYLE1", &format!(r"(?:\[({LINE_STYLE})\])?")),
        IRegex::named(
            "ARROW_DIRECTION",
            r"(\*|left|right|up|down|le?|ri?|up?|do?)?",
        ),
        IRegex::named("ARROW_STYLE2", &format!(r"(?:\[({LINE_STYLE})\])?")),
        IRegex::named("ARROW_BODY2", "([-.]*)"),
        IRegex::leaf(r"\>"),
    ]
}

fn link_activity_regex() -> IRegex {
    let mut parts = vec![
        IRegex::start(),
        IRegex::optional(IRegex::or_named(
            "FIRST",
            vec![
                IRegex::named("STAR", r"(\(\*(top)?\))"),
                IRegex::named("CODE", "([%pLN][%pLN_.]*)"),
                IRegex::named("BAR", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                IRegex::named("QUOTED", "[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN_.]+))?"),
            ],
        )),
        optional_pattern("STEREOTYPE"),
        exp2(),
        IRegex::space_zero_or_more(),
        url::optional(),
    ];
    parts.extend(arrow_regex());
    parts.extend([
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named("BRACKET", r"\[([^\]*]+[^\]]*)\]")),
        IRegex::space_zero_or_more(),
        IRegex::or_named(
            "FIRST2",
            vec![
                IRegex::named("STAR2", r"(\(\*(top|\d+)?\))"),
                IRegex::named("OPENBRACKET2", r"(\{)"),
                IRegex::named("CODE2", "([%pLN][%pLN_.]*)"),
                IRegex::named("BAR2", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
                IRegex::named(
                    "QUOTED2",
                    "[%g]([^%g]+)[%g](?:[%s]+as[%s]+([%pLN][%pLN_.]*))?",
                ),
                IRegex::named("QUOTED_INVISIBLE2", r"(\w.*?)"),
            ],
        ),
        optional_pattern("STEREOTYPE2"),
        IRegex::optional(IRegex::group(vec![
            IRegex::leaf("in"),
            IRegex::space_one_or_more(),
            IRegex::named("PARTITION2", r"([%g][^%g]+[%g]|\S+)"),
        ])),
        IRegex::space_zero_or_more(),
        exp3(),
        IRegex::end(),
    ]);
    IRegex::concat(parts)
}

fn back_color(arg: &RegexResult, key: &str, entity: &Entity) {
    if let Some(c) = arg.get(key, 0).and_then(HColor::parse) {
        entity.set_colors(Colors::empty().add(ColorType::Back, Some(c)));
    }
}

fn clean(s: &str) -> String {
    crate::string_utils::eventually_remove_starting_and_ending_double_quote(s)
}

/// `CommandLinkActivity.getEntity`: one end of a link; the first one defaults to the last activity.
fn get_entity(
    d: &mut ActivityDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    start: bool,
) -> Option<Entity> {
    let suf = if start { "" } else { "2" };
    let key = |name: &str| format!("{name}{suf}");
    if arg.get(&key("OPENBRACKET"), 0).is_some() {
        return None;
    }
    if arg.get(&key("STAR"), 0).is_some() {
        return Some(if start {
            d.get_start(loc)
        } else {
            d.get_end(loc, arg.get(&key("STAR"), 1))
        });
    }
    let partition = arg.get(&key("PARTITION"), 0).map(clean);
    let in_partition = |d: &mut ActivityDiagram| {
        if let Some(p) = &partition {
            d.goto_package(loc, &clean(p), p);
        }
    };
    let leave = |d: &mut ActivityDiagram| {
        if partition.is_some() {
            d.cuca.end_group();
        }
    };
    let create = |d: &mut ActivityDiagram, id: &str, display: &str, leaf: LeafType| {
        let quark = d.cuca.quark_in_context(true, &clean(id)).expect("name");
        match quark.data() {
            Some(e) => e,
            None => d.create_leaf(loc, quark, Display::with_newlines(Some(display)), leaf),
        }
    };
    if let Some(code) = arg.get(&key("CODE"), 0) {
        in_partition(d);
        let result = create(d, code, code, LeafType::Activity);
        leave(d);
        return Some(result);
    }
    if let Some(bar) = arg.get(&key("BAR"), 0) {
        return Some(create(d, bar, bar, LeafType::SynchroBar));
    }
    if let Some(text) = arg.get(&key("QUOTED"), 0) {
        let id = arg.get(&key("QUOTED"), 1).unwrap_or(text);
        in_partition(d);
        let result = create(d, id, text, LeafType::Activity);
        leave(d);
        return Some(result);
    }
    if let Some(text) = arg.get(&key("QUOTED_INVISIBLE"), 0) {
        in_partition(d);
        let result = create(d, text, text, LeafType::Activity);
        leave(d);
        return Some(result);
    }
    if arg.get(&key("FIRST"), 0).is_none() {
        return d.last_entity_consulted();
    }
    None
}

fn link_activity() -> Cmd {
    Box::new(SingleLineCommand::new(
        link_activity_regex(),
        |d: &mut ActivityDiagram, loc, arg, _| {
            let Some(entity1) = get_entity(d, loc, arg, true) else {
                return CommandExecutionResult::error("No such activity");
            };
            if let Some(s) = arg.get("STEREOTYPE", 0) {
                entity1.set_stereotype(Some(Stereotype::build(s)));
            }
            back_color(arg, "BACKCOLOR", &entity1);
            let Some(entity2) = get_entity(d, loc, arg, false) else {
                return CommandExecutionResult::error("No such activity");
            };
            d.set_last_entity_consulted(Some(entity2.clone()));
            back_color(arg, "BACKCOLOR2", &entity2);
            if let Some(s) = arg.get("STEREOTYPE2", 0) {
                entity2.set_stereotype(Some(Stereotype::build(s)));
            }
            let label = Display::with_newlines(arg.get("BRACKET", 0));
            let body1 = arg.get("ARROW_BODY1", 0).unwrap_or("");
            let body2 = arg.get("ARROW_BODY2", 0).unwrap_or("");
            let direction = arg.get("ARROW_DIRECTION", 0).unwrap_or("");
            let full = format!("{body1}{direction}{body2}>");
            let dir = arrow_direction(&full);
            let arrow = manage_arrow(&full, dir);
            let mut length = arrow.chars().count() as i32 - 1;
            if direction.contains('*') {
                length = 2;
            }
            let mut link_type = LinkType::new(LinkDecor::Arrow, LinkDecor::None);
            if format!("{body1}{body2}").contains('.') {
                link_type = link_type.go_dotted();
            }
            let icon = d.cuca.base.skin_param.class_attribute_icon_size() > 0;
            let link_arg = LinkArg::build_with(label, length, icon);
            let uid = d.cuca.unique_sequence("lnk");
            let mut link = Link::new(loc, uid, entity1, entity2, link_type, link_arg);
            if direction.contains('*') {
                link.set_constraint(false);
            }
            if matches!(dir, Direction::Left | Direction::Up) {
                let uid = d.cuca.unique_sequence("lnk");
                link = link.inv(uid);
            }
            if let Some(u) = arg.get(url::URL_KEY, 0) {
                let top = d.cuca.base.skin_param.get_value("topurl");
                link.set_url(UrlBuilder::new(top, UrlMode::Strict).url(u));
            }
            link.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
            d.cuca.add_link(link);
            CommandExecutionResult::ok()
        },
    ))
}

fn if_regex() -> IRegex {
    let source = IRegex::optional(IRegex::or_named(
        "FIRST",
        vec![
            IRegex::named("STAR", r"(\(\*(top)?\))"),
            IRegex::named("BAR", r"(?:==+)[%s]*([%pLN_.]+)[%s]*(?:==+)"),
            IRegex::named("QUOTED", "[%g]([^%g]*)[%g](?:[%s]+as[%s]+([%pLN_.]+))?"),
            IRegex::named("CODE", "([%pLN_.]+)"),
        ],
    ));
    let arrow = {
        let mut a = vec![IRegex::space_zero_or_more()];
        a.extend(arrow_regex());
        IRegex::optional(IRegex::group(a))
    };
    let quoted = |label: &str, code: &str, then: bool| {
        let mut p = vec![
            IRegex::leaf("if[%s]*"),
            IRegex::named(label, "[%g]([^%g]*)[%g]"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named(code, "([%pLN_.]+)"),
                IRegex::space_one_or_more(),
            ])),
        ];
        if then {
            p.push(IRegex::leaf("then"));
        }
        IRegex::group(p)
    };
    IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        source,
        arrow,
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named("BRACKET", r"\[([^\]*]+[^\]]*)\]")),
        IRegex::space_zero_or_more(),
        IRegex::or(vec![
            quoted("IF1", "ASIF1", true),
            IRegex::group(vec![
                IRegex::leaf("if[%s]+"),
                IRegex::named("IF2", "(.+?)"),
                IRegex::leaf("then"),
            ]),
            quoted("IF3", "ASIF3", false),
            IRegex::group(vec![IRegex::leaf("if[%s]+"), IRegex::named("IF4", "(.+)")]),
        ]),
        IRegex::end(),
    ])
}

fn if_command() -> Cmd {
    Box::new(SingleLineCommand::new(
        if_regex(),
        |d: &mut ActivityDiagram, loc, arg, _| {
            let Some(entity1) = get_entity(d, loc, arg, true) else {
                return CommandExecutionResult::error("No if possible at this point");
            };
            let (code, label) = if let Some(l) = arg.get("IF2", 0).or(arg.get("IF4", 0)) {
                (None, l)
            } else if arg.get("IF1", 0).is_some() {
                (arg.get("ASIF1", 0), arg.get("IF1", 0).unwrap_or(""))
            } else {
                (arg.get("ASIF3", 0), arg.get("IF3", 0).unwrap_or(""))
            };
            d.start_if(loc, code);
            let mut length = 2;
            if arg.get("ARROW_BODY1", 0).is_some() {
                let body1 = arg.get("ARROW_BODY1", 0).unwrap_or("");
                let body2 = arg.get("ARROW_BODY2", 0).unwrap_or("");
                let direction = arg.get("ARROW_DIRECTION", 0).unwrap_or("");
                let full = format!("{body1}{direction}{body2}>");
                length = manage_arrow(&full, arrow_direction(&full)).chars().count() as i32 - 1;
            }
            let branch = d.current_branch().expect("branch");
            let link_arg = LinkArg::build(Display::with_newlines(arg.get("BRACKET", 0)), length)
                .with_quantifier(None, Some(label.to_string()))
                .with_distance_angle(Some(d.cuca.label_distance()), Some(d.cuca.label_angle()));
            let uid = d.cuca.unique_sequence("lnk");
            let link = Link::new(
                loc,
                uid,
                entity1,
                branch,
                LinkType::new(LinkDecor::Arrow, LinkDecor::None),
                link_arg,
            );
            link.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
            d.cuca.add_link(link);
            CommandExecutionResult::ok()
        },
    ))
}

fn else_command() -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![IRegex::start(), IRegex::leaf("else"), IRegex::end()]),
        |d: &mut ActivityDiagram, _, _, _| {
            if d.last_entity_consulted().is_none() {
                return CommandExecutionResult::error("No if for this else");
            }
            let Some(branch) = d.current_branch() else {
                return CommandExecutionResult::error("No if for this else");
            };
            d.set_last_entity_consulted(Some(branch));
            CommandExecutionResult::ok()
        },
    ))
}

fn endif_command() -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("end"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("if"),
            IRegex::end(),
        ]),
        |d: &mut ActivityDiagram, _, _, _| {
            if d.last_entity_consulted().is_none() || d.current_branch().is_none() {
                return CommandExecutionResult::error("No if for this endif");
            }
            d.endif();
            CommandExecutionResult::ok()
        },
    ))
}

fn partition_command() -> Cmd {
    let color = ColorParser::simple_color(ColorType::Back);
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("partition"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::named("NAME1", "[%g]([^%g]+)[%g]"),
            IRegex::named("NAME2", r"(\S+)"),
        ]),
        IRegex::space_zero_or_more(),
        color.regex(),
        optional_pattern("STEREOTYPE"),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::leaf(r"\{")),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        move |d: &mut ActivityDiagram, loc, arg, _| {
            let name = arg.get("NAME1", 0).or(arg.get("NAME2", 0)).unwrap_or("");
            let id = clean(name);
            d.goto_package(loc, &id, &id);
            let group = d.cuca.current_group();
            let Some(colors) = color.colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            if !colors.is_empty() {
                group.set_colors(colors);
            }
            group.set_stereotype(arg.get("STEREOTYPE", 0).map(Stereotype::build));
            CommandExecutionResult::ok()
        },
    ))
}

fn end_partition_command() -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::or(vec![
                IRegex::group(vec![
                    IRegex::leaf("end"),
                    IRegex::space_zero_or_more(),
                    IRegex::leaf("partition"),
                ]),
                IRegex::leaf(r"\}"),
            ]),
            IRegex::end(),
        ]),
        |d: &mut ActivityDiagram, _, _, _| {
            if d.cuca.end_group() {
                CommandExecutionResult::ok()
            } else {
                CommandExecutionResult::error("No partition defined")
            }
        },
    ))
}

pub fn commands() -> Vec<Cmd> {
    use crate::classdiagram::commands_misc as misc;
    let mut cmds: Vec<Cmd> = vec![misc::rank_dir()];
    cmds.extend(crate::command::common::common_commands());
    cmds.extend([
        partition_command(),
        end_partition_command(),
        if_command(),
        else_command(),
        endif_command(),
        link_activity(),
    ]);
    cmds
}
