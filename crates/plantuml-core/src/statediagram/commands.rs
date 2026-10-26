//! Commands of state diagrams (`statediagram.command`).

use super::StateDiagram;
use crate::abel::{GroupType, LeafType, Link, LinkArg, Stereotag};
use crate::command::{on_passes, Command, CommandExecutionResult, ParserPass, SingleLineCommand};
use crate::decoration::link::{LinkDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Direction;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereogroup, Stereotype};
use crate::text::LineLocation;
use crate::url::{self, UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<StateDiagram>>;

const ALL: [ParserPass; 3] = [ParserPass::One, ParserPass::Two, ParserPass::Three];
const LINE_STYLE: &str = r"dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+";
const STATE_PATTERN: &str =
    r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))";

fn color() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn tags(name: &str) -> IRegex {
    IRegex::named(name, &format!("{}?", Stereotag::pattern()))
}

fn add_tags(entity: &crate::abel::Entity, tags: Option<&str>) {
    for tag in tags.unwrap_or("").split(' ').filter(|t| !t.is_empty()) {
        if let Some(t) = tag.strip_prefix('$') {
            entity.add_stereotag(Stereotag(t.to_string()));
        }
    }
}

fn not_null<'a>(a: Option<&'a str>, b: Option<&'a str>) -> Option<&'a str> {
    a.or(b)
}

fn line_colors(arg: &RegexResult, mut colors: Colors) -> Colors {
    if let Some(c) = arg.get("LINECOLOR", 1).and_then(HColor::parse) {
        colors = colors.add(ColorType::Line, Some(c));
    }
    if let Some(s) = arg.get("LINECOLOR", 0) {
        colors = colors.add_legacy_stroke(s);
    }
    colors
}

fn apply_url(d: &StateDiagram, arg: &RegexResult, entity: &crate::abel::Entity) {
    if let Some(u) = arg.get(url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            entity.add_url(url);
        }
    }
}

fn quark_of(
    d: &StateDiagram,
    id: &str,
) -> Result<crate::plasma::Quark<crate::abel::Entity>, CommandExecutionResult> {
    d.cuca
        .quark_in_context(true, id)
        .map_err(|(m, s)| CommandExecutionResult::error_with_score(m, s))
}

fn create_state() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("state"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("CODE1", "([%pLN_.]+)"),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("DISPLAY1", "[%g]([^%g]+)[%g]"),
            ]),
            IRegex::group(vec![
                IRegex::named("DISPLAY2", "[%g]([^%g]+)[%g]"),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE2", "([%pLN_.]+)"),
            ]),
            IRegex::named("CODE3", "([%pLN_.]+)"),
            IRegex::named("CODE4", "[%g]([^%g]+)[%g]"),
        ]),
        IRegex::space_zero_or_more(),
        tags("TAGS1"),
        Stereogroup::optional_pattern(),
        tags("TAGS2"),
        IRegex::space_zero_or_more(),
        url::optional(),
        IRegex::space_zero_or_more(),
        color().regex(),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named(
            "LINECOLOR",
            r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
        )),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::group(vec![
            IRegex::leaf(":"),
            IRegex::space_zero_or_more(),
            IRegex::named("ADDFIELD", "(.*)"),
        ])),
        IRegex::end(),
    ]);
    on_passes(
        Box::new(SingleLineCommand::new(
            regex,
            |d: &mut StateDiagram, loc, arg, pass| {
                let id_short = arg.get_lazzy("CODE", 0).unwrap_or("").to_string();
                let id = d.cuca.clean_id(Some(&id_short)).unwrap_or_default();
                let quark = match quark_of(d, &id) {
                    Ok(q) => q,
                    Err(e) => return e,
                };
                let display = arg
                    .get_lazzy("DISPLAY", 0)
                    .map_or_else(|| quark.name().to_string(), str::to_string);
                let stereogroup = Stereogroup::build(arg);
                let leaf_type = stereogroup.leaf_type().unwrap_or(LeafType::State);
                if !d.check_concurrent_state_ok(&quark) {
                    return CommandExecutionResult::error(format!(
                        "The state {} has been created in a concurrent state : it cannot be used here.",
                        quark.name()
                    ));
                }
                let ent = match quark.data() {
                    Some(e) => {
                        d.cuca.set_last_entity(Some(e.clone()));
                        e
                    }
                    None => d.cuca.really_create_leaf(
                        loc,
                        quark.clone(),
                        Display::with_newlines(Some(&display)),
                        leaf_type,
                    ),
                };
                d.ensure_parent_state(loc, &quark);
                if pass == ParserPass::One {
                    ent.set_display(Display::with_newlines(Some(&display)));
                    ent.set_stereotype(stereogroup.build_stereotype());
                    apply_url(d, arg, &ent);
                    let Some(mut colors) = color().colors(arg) else {
                        return CommandExecutionResult::bad_color();
                    };
                    colors = line_colors(arg, colors);
                    colors = colors.merge_with(Some(&stereogroup.inner_colors()));
                    ent.set_colors(colors);
                    if let Some(f) = arg.get("ADDFIELD", 0) {
                        ent.add_field_or_method(f);
                    }
                    add_tags(&ent, arg.get_lazzy("TAGS", 0));
                    if ent.parent_container().is_some_and(|p| p.is_root())
                        && !ent.entity_position().is_normal()
                    {
                        return CommandExecutionResult::error(
                            "You cannot use this stereotype here",
                        );
                    }
                }
                CommandExecutionResult::ok()
            },
        )),
        &ALL,
    )
}

fn create_package_state() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("state"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("CODE1", "([%pLN_.]+)"),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("DISPLAY1", "[%g]([^%g]+)[%g]"),
            ]),
            IRegex::group(vec![
                IRegex::optional(IRegex::group(vec![
                    IRegex::named("DISPLAY2", "[%g]([^%g]+)[%g]"),
                    IRegex::space_one_or_more(),
                    IRegex::leaf("as"),
                    IRegex::space_one_or_more(),
                ])),
                IRegex::named("CODE2", "([%pLN_.]+)"),
            ]),
        ]),
        IRegex::space_zero_or_more(),
        tags("TAGS1"),
        Stereogroup::optional_pattern(),
        tags("TAGS2"),
        IRegex::space_zero_or_more(),
        url::optional(),
        IRegex::space_zero_or_more(),
        color().regex(),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named(
            "LINECOLOR",
            r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
        )),
        IRegex::leaf(r"(?:[%s]*\{|[%s]+begin)"),
        IRegex::end(),
    ]);
    on_passes(
        Box::new(SingleLineCommand::new(
            regex,
            |d: &mut StateDiagram, loc, arg, _| {
                let id_short = not_null(arg.get("CODE1", 0), arg.get("CODE2", 0))
                    .unwrap_or("")
                    .to_string();
                let quark = match quark_of(d, &id_short) {
                    Ok(q) => q,
                    Err(e) => return e,
                };
                let display = not_null(arg.get("DISPLAY1", 0), arg.get("DISPLAY2", 0));
                let shown = display.map_or_else(|| quark.name().to_string(), str::to_string);
                d.cuca.goto_group(
                    loc,
                    quark,
                    Display::with_newlines(Some(&shown)),
                    GroupType::State,
                    None,
                );
                let p = d.cuca.current_group();
                if let Some(disp) = display {
                    p.set_display(Display::with_newlines(Some(disp)));
                }
                let stereogroup = Stereogroup::build(arg);
                p.set_stereotype(stereogroup.build_stereotype());
                apply_url(d, arg, &p);
                let Some(mut colors) = color().colors(arg) else {
                    return CommandExecutionResult::bad_color();
                };
                colors = line_colors(arg, colors);
                colors = colors.merge_with(Some(&stereogroup.inner_colors()));
                p.set_colors(colors);
                add_tags(&p, arg.get_lazzy("TAGS", 0));
                CommandExecutionResult::ok()
            },
        )),
        &ALL,
    )
}

fn end_state() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("END", "(end[%s]?state|\\})"),
        IRegex::end(),
    ]);
    on_passes(
        Box::new(SingleLineCommand::new(
            regex,
            |d: &mut StateDiagram, _, _, _| {
                if d.cuca.current_group().is_root() {
                    return CommandExecutionResult::error("No inner state defined");
                }
                d.end_group();
                CommandExecutionResult::ok()
            },
        )),
        &ALL,
    )
}

fn concurrent() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", r"(--+|\|\|+)"),
        IRegex::end(),
    ]);
    on_passes(
        Box::new(SingleLineCommand::new(
            regex,
            |d: &mut StateDiagram, loc, arg, _| {
                let c = arg
                    .get("TYPE", 0)
                    .and_then(|t| t.chars().next())
                    .unwrap_or('-');
                d.concurrent_state(loc, c);
                CommandExecutionResult::ok()
            },
        )),
        &ALL,
    )
}

fn add_field() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::or(vec![
            IRegex::named("CODE3", "([%pLN_.]+)"),
            IRegex::named("CODE4", "[%g]([^%g]+)[%g]"),
        ]),
        IRegex::space_zero_or_more(),
        IRegex::leaf(":"),
        IRegex::space_zero_or_more(),
        IRegex::named("FIELD", "(.*)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        |d: &mut StateDiagram, loc, arg, _| {
            let code = arg.get_lazzy("CODE", 0).unwrap_or("").to_string();
            let quark = if d.cuca.current_group().name() == code {
                d.cuca.current_group().quark()
            } else {
                match quark_of(d, &d.cuca.clean_id(Some(&code)).unwrap_or_default()) {
                    Ok(q) => q,
                    Err(e) => return e,
                }
            };
            let entity = match quark.data() {
                Some(e) => e,
                None => d.cuca.really_create_leaf(
                    loc,
                    quark.clone(),
                    Display::with_newlines(Some(quark.name())),
                    LeafType::State,
                ),
            };
            entity.add_field_or_method(arg.get("FIELD", 0).unwrap_or(""));
            CommandExecutionResult::ok()
        },
    ))
}

fn link_regex(reverse: bool) -> IRegex {
    let body = if reverse {
        vec![
            IRegex::named("ARROW_CIRCLE_END", "(o[%s]+)?"),
            IRegex::leaf(r"\<"),
            IRegex::named("ARROW_BODY2", "(-*)"),
            IRegex::named("ARROW_STYLE2", &format!(r"(?:\[({LINE_STYLE})\])?")),
            IRegex::named("ARROW_DIRECTION", "(left|right|up|down|le?|ri?|up?|do?)?"),
            IRegex::named("ARROW_STYLE1", &format!(r"(?:\[({LINE_STYLE})\])?")),
            IRegex::named("ARROW_BODY1", "(-+)"),
            IRegex::named("ARROW_CROSS_START", "(x)?"),
        ]
    } else {
        vec![
            IRegex::named("ARROW_CROSS_START", "(x)?"),
            IRegex::named("ARROW_BODY1", "(-+)"),
            IRegex::named("ARROW_STYLE1", &format!(r"(?:\[({LINE_STYLE})\])?")),
            IRegex::named("ARROW_DIRECTION", "(left|right|up|down|le?|ri?|up?|do?)?"),
            IRegex::named("ARROW_STYLE2", &format!(r"(?:\[({LINE_STYLE})\])?")),
            IRegex::named("ARROW_BODY2", "(-*)"),
            IRegex::leaf(r"\>"),
            IRegex::named("ARROW_CIRCLE_END", "(o[%s]+)?"),
        ]
    };
    let (first, second) = if reverse {
        ("ENT2", "ENT1")
    } else {
        ("ENT1", "ENT2")
    };
    let mut parts = vec![
        IRegex::start(),
        IRegex::named(first, STATE_PATTERN),
        IRegex::space_zero_or_more(),
    ];
    parts.extend(body);
    parts.extend([
        IRegex::space_zero_or_more(),
        IRegex::named(second, STATE_PATTERN),
        IRegex::space_zero_or_more(),
        optional_pattern("STEREOTYPE"),
        IRegex::optional(IRegex::group(vec![
            IRegex::leaf(":"),
            IRegex::space_zero_or_more(),
            IRegex::named("LABEL", "(.+)"),
        ])),
        IRegex::end(),
    ]);
    IRegex::concat(parts)
}

fn remove_equals(code: &str) -> &str {
    code.trim_start_matches('=').trim_end_matches('=')
}

fn display_text(display: &Display) -> String {
    display
        .lines()
        .iter()
        .map(crate::klimt::creole::display::DisplayLine::text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_entity(
    d: &mut StateDiagram,
    loc: Option<LineLocation>,
    code: &str,
) -> Option<crate::abel::Entity> {
    if code.eq_ignore_ascii_case("[H]") {
        return Some(d.get_historical(loc));
    }
    if let Some(c) = code.strip_suffix("[H]") {
        return Some(d.get_historical_of(loc, c));
    }
    if code.eq_ignore_ascii_case("[H*]") {
        return Some(d.get_deep_history(loc));
    }
    if let Some(c) = code.strip_suffix("[H*]") {
        return Some(d.get_deep_history_of(loc, c));
    }
    if code.starts_with('=') && code.ends_with('=') {
        let id = remove_equals(code);
        let quark = d.cuca.quark_in_context(true, id).ok()?;
        if let Some(e) = quark.data() {
            return Some(e);
        }
        return Some(d.cuca.really_create_leaf(
            loc,
            quark.clone(),
            Display::with_newlines(Some(quark.name())),
            LeafType::SynchroBar,
        ));
    }
    if d.cuca.current_group().name() == code {
        return Some(d.cuca.current_group());
    }
    let quark = d.cuca.quark_in_context(true, code).ok()?;
    if !d.check_concurrent_state_ok(&quark) {
        return None;
    }
    if let Some(e) = quark.data() {
        return Some(e);
    }
    quark.parent()?.data()?;
    Some(d.cuca.really_create_leaf(
        loc,
        quark.clone(),
        Display::with_newlines(Some(quark.name())),
        LeafType::State,
    ))
}

fn use_node_style(d: &StateDiagram, arg: &RegexResult) -> bool {
    if arg
        .get_lazzy("ARROW_STYLE", 0)
        .is_some_and(|s| s.to_lowercase().contains("node"))
    {
        return true;
    }
    d.cuca
        .base
        .skin_param
        .get_value("statediagramedgelabelstyle")
        .is_some_and(|v| v.eq_ignore_ascii_case("node"))
}

fn execute_link(
    d: &mut StateDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    reverse: bool,
) -> CommandExecutionResult {
    let ent1 = arg.get("ENT1", 0).unwrap_or("").to_string();
    let ent2 = arg.get("ENT2", 0).unwrap_or("").to_string();
    let cl1 = if ent1.starts_with("[*]") {
        Some(d.get_start(loc))
    } else {
        get_entity(d, loc, &ent1)
    };
    let Some(cl1) = cl1 else {
        return CommandExecutionResult::error(format!("The state {ent1} cannot be used here."));
    };
    let cl2 = if ent2.starts_with("[*]") {
        Some(d.get_end(loc))
    } else {
        get_entity(d, loc, &ent2)
    };
    let Some(cl2) = cl2 else {
        return CommandExecutionResult::error(format!("The state {ent2} cannot be used here."));
    };
    let mut queue = format!(
        "{}{}",
        arg.get("ARROW_BODY1", 0).unwrap_or(""),
        arg.get("ARROW_BODY2", 0).unwrap_or("")
    );
    let dir = match arg.get("ARROW_DIRECTION", 0) {
        Some(s) => Some(crate::string_utils::get_queue_direction(s)),
        None if reverse => Some(Direction::Left),
        None => None,
    };
    if matches!(dir, Some(Direction::Left | Direction::Right)) {
        queue = "-".to_string();
    }
    let length = queue.chars().count() as i32;
    let cross = arg.get("ARROW_CROSS_START", 0).is_some();
    let circle_end = arg.get("ARROW_CIRCLE_END", 0).is_some();
    let link_type = LinkType::new(
        if circle_end {
            LinkDecor::ArrowAndCircle
        } else {
            LinkDecor::Arrow
        },
        if cross {
            LinkDecor::CircleCross
        } else {
            LinkDecor::None
        },
    );
    let label = Display::with_newlines(arg.get("LABEL", 0));
    let use_node = use_node_style(d, arg);
    let icon = d.cuca.base.skin_param.class_attribute_icon_size() > 0;
    let has_label = !label.is_null() && !display_text(&label).trim().is_empty();
    if use_node && has_label {
        return transition_with_node(d, loc, cl1, cl2, label, link_type, dir, arg, icon);
    }
    let link_arg = LinkArg::build_with(label, length, icon);
    let uid = d.cuca.unique_sequence("lnk");
    let mut link = Link::new(loc, uid, cl1, cl2, link_type, link_arg);
    if matches!(dir, Some(Direction::Left | Direction::Up)) {
        let uid = d.cuca.unique_sequence("lnk");
        link = link.inv(uid);
    }
    link.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    if let Some(st) = arg.get("STEREOTYPE", 0) {
        link.set_stereotype(Some(Stereotype::build(st)));
    }
    d.cuca.add_link(link);
    CommandExecutionResult::ok()
}

#[allow(clippy::too_many_arguments)]
fn transition_with_node(
    d: &mut StateDiagram,
    loc: Option<LineLocation>,
    source: crate::abel::Entity,
    target: crate::abel::Entity,
    label: Display,
    link_type: LinkType,
    dir: Option<Direction>,
    arg: &RegexResult,
    icon: bool,
) -> CommandExecutionResult {
    let clean: String = display_text(&label)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let id = format!(
        "transition_{}_{}_{}_{}",
        source.name(),
        target.name(),
        clean,
        d.cuca.unique_sequence_value()
    );
    let quark = match quark_of(d, &id) {
        Ok(q) => q,
        Err(e) => return e,
    };
    let node = d
        .cuca
        .really_create_leaf(loc, quark, label, LeafType::StateTransitionLabel);
    node.set_stereotype(Some(Stereotype::build("<<transition>>")));
    let plain = LinkType::new(LinkDecor::None, LinkDecor::None);
    let uid = d.cuca.unique_sequence("lnk");
    let mut first = Link::new(
        loc,
        uid,
        source,
        node.clone(),
        plain,
        LinkArg::build_with(Display::null(), 2, icon),
    );
    first.get_mut().weight = 10.0;
    let uid = d.cuca.unique_sequence("lnk");
    let mut second = Link::new(
        loc,
        uid,
        node,
        target,
        link_type,
        LinkArg::build_with(Display::null(), 2, icon),
    );
    second.get_mut().weight = 10.0;
    if matches!(dir, Some(Direction::Left | Direction::Up)) {
        let uid = d.cuca.unique_sequence("lnk");
        first = first.inv(uid);
        let uid = d.cuca.unique_sequence("lnk");
        second = second.inv(uid);
    }
    if let Some(style) = arg.get_lazzy("ARROW_STYLE", 0) {
        let filtered: Vec<&str> = style
            .split(',')
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("node"))
            .collect();
        if !filtered.is_empty() {
            let f = filtered.join(",");
            first.apply_style(Some(&f));
            second.apply_style(Some(&f));
        }
    }
    d.cuca.add_link(first);
    d.cuca.add_link(second);
    CommandExecutionResult::ok()
}

fn link_state(reverse: bool) -> Cmd {
    on_passes(
        Box::new(SingleLineCommand::new(
            link_regex(reverse),
            move |d: &mut StateDiagram, loc, arg, _| execute_link(d, loc, arg, reverse),
        )),
        &[ParserPass::Two],
    )
}

fn frame() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("frame"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("CODE1", "([%pLN_.]+)"),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("DISPLAY1", "[%g]([^%g]+)[%g]"),
            ]),
            IRegex::group(vec![
                IRegex::optional(IRegex::group(vec![
                    IRegex::named("DISPLAY2", "[%g]([^%g]+)[%g]"),
                    IRegex::space_one_or_more(),
                    IRegex::leaf("as"),
                    IRegex::space_one_or_more(),
                ])),
                IRegex::named("CODE2", "([%pLN_.]+)"),
            ]),
        ]),
        optional_pattern("STEREOTYPE"),
        url::optional(),
        IRegex::space_zero_or_more(),
        color().regex(),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named(
            "LINECOLOR",
            r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
        )),
        IRegex::leaf(r"(?:[%s]*\{|[%s]+begin)"),
        IRegex::end(),
    ]);
    on_passes(
        Box::new(SingleLineCommand::new(
            regex,
            |d: &mut StateDiagram, loc, arg, _| {
                let id = not_null(arg.get("CODE1", 0), arg.get("CODE2", 0))
                    .unwrap_or("")
                    .to_string();
                let quark = match quark_of(d, &id) {
                    Ok(q) => q,
                    Err(e) => return e,
                };
                let display = not_null(arg.get("DISPLAY1", 0), arg.get("DISPLAY2", 0))
                    .map_or_else(|| quark.name().to_string(), str::to_string);
                d.cuca.goto_group(
                    loc,
                    quark,
                    Display::with_newlines(Some(&display)),
                    GroupType::Package,
                    Some(crate::decoration::usymbol::usymbols::frame()),
                );
                let p = d.cuca.current_group();
                if let Some(s) = arg.get("STEREOTYPE", 0) {
                    p.set_stereotype(Some(Stereotype::build(s)));
                }
                apply_url(d, arg, &p);
                let Some(colors) = color().colors(arg) else {
                    return CommandExecutionResult::bad_color();
                };
                p.set_colors(line_colors(arg, colors));
                CommandExecutionResult::ok()
            },
        )),
        &ALL,
    )
}

pub fn commands() -> Vec<Cmd> {
    use crate::classdiagram::commands_misc as misc;
    use crate::classdiagram::commands_note as notes;
    let mut cmds: Vec<Cmd> = vec![
        misc::rank_dir(),
        misc::remove_restore(),
        create_state(),
        link_state(false),
        link_state(true),
        create_package_state(),
        frame(),
        end_state(),
        add_field(),
        concurrent(),
    ];
    cmds.extend(notes::on_entity_commands_for(
        r#"([%pLN_.]+|[%g][^%g]+[%g])"#,
        ParserPass::Three,
    ));
    cmds.extend(notes::on_link_commands_for(ParserPass::Two));
    cmds.push(misc::url_command());
    cmds.extend(notes::floating_all_commands());
    cmds.extend(crate::command::common::common_commands());
    cmds.push(crate::classdiagram::commands::hide_show2());
    cmds
}
