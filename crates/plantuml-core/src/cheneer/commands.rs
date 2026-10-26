//! Commands of Chen EER diagrams (`cheneer/command`).

use super::ChenDiagram;
use crate::abel::{Entity, LeafType, Link, LinkArg};
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::decoration::link::{LinkDecor, LinkMiddleDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::text::LineLocation;

type Cmd = Box<dyn Command<ChenDiagram>>;

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut ChenDiagram, Option<LineLocation>, &RegexResult) -> CommandExecutionResult
        + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut ChenDiagram, loc, arg, _| f(d, loc, arg),
    ))
}

fn back_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn line_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Line)
}

fn code_pattern() -> &'static str {
    "([%pLN_.-]+)"
}

fn display_as() -> IRegex {
    IRegex::optional(IRegex::group(vec![
        IRegex::named("DISPLAY", "[%g]([^%g]+)[%g]"),
        IRegex::space_one_or_more(),
        IRegex::leaf("as"),
        IRegex::space_one_or_more(),
    ]))
}

fn stereotype() -> IRegex {
    IRegex::named("STEREO", "(<<.+>>)?")
}

fn set_stereotype(entity: &Entity, stereo: Option<&str>) {
    if let Some(st) = stereo {
        entity.set_stereotype(Some(Stereotype::build(st)));
        entity.set_stereostyle(st);
    }
}

fn add_link(
    d: &mut ChenDiagram,
    location: Option<LineLocation>,
    entity1: &Entity,
    entity2: &Entity,
    type_: LinkType,
    arg: LinkArg,
    colors: Colors,
) {
    let uid = d.cuca.unique_sequence("lnk");
    let link = Link::new(
        location,
        uid,
        entity1.clone(),
        entity2.clone(),
        type_,
        arg,
    );
    link.set_colors(colors);
    d.cuca.add_link(link);
}

fn entity_type(arg: &RegexResult) -> Option<LeafType> {
    match arg.get("TYPE", 0)? {
        "entity" => Some(LeafType::ChenEntity),
        "relationship" => Some(LeafType::ChenRelationship),
        _ => None,
    }
}

fn create_entity() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("TYPE", "(entity|relationship)"),
            IRegex::space_one_or_more(),
            display_as(),
            IRegex::named("CODE", code_pattern()),
            IRegex::space_zero_or_more(),
            stereotype(),
            IRegex::space_zero_or_more(),
            back_color().regex(),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{"),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let Some(type_) = entity_type(arg) else {
                return CommandExecutionResult::error("Unknown entity type");
            };
            let id = d.clean_id(arg.get("CODE", 0)).unwrap_or_default();
            let quark = match d.cuca.quark_in_context(true, &id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let display_text = arg
                .get("DISPLAY", 0)
                .map(str::to_string)
                .unwrap_or_else(|| quark.name().to_string());
            let entity = match quark.data() {
                None => d.cuca.really_create_leaf(
                    loc,
                    quark.clone(),
                    Display::with_newlines(Some(&display_text)),
                    type_,
                ),
                Some(e) => {
                    if !e.mute_to_type(type_) {
                        return CommandExecutionResult::error("Bad name");
                    }
                    e
                }
            };
            set_stereotype(&entity, arg.get("STEREO", 0));
            let Some(colors) = back_color().colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            entity.set_colors(colors);
            d.push_owner(entity);
            CommandExecutionResult::ok()
        },
    )
}

fn create_attribute() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            display_as(),
            IRegex::named("CODE", "([%pLN%s_.:]+)"),
            IRegex::space_zero_or_more(),
            stereotype(),
            IRegex::space_zero_or_more(),
            line_color().regex(),
            IRegex::space_zero_or_more(),
            IRegex::named("COMPOSITE", r"(\{)?"),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let Some(owner) = d.peek_owner() else {
                return CommandExecutionResult::error(
                    "Attribute must be inside an entity, relationship or another attribute",
                );
            };
            let id_short = d
                .clean_id(arg.get("CODE", 0).map(str::trim))
                .unwrap_or_default();
            let id = format!("{}/{}", owner.name(), id_short);
            let quark = match d.cuca.quark_in_context(true, &id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let display_text = arg.get("DISPLAY", 0).unwrap_or(&id_short);
            let entity = match quark.data() {
                None => d.cuca.really_create_leaf(
                    loc,
                    quark.clone(),
                    Display::with_newlines(Some(display_text)),
                    LeafType::ChenAttribute,
                ),
                Some(_) => return CommandExecutionResult::error("Attribute already exists"),
            };
            set_stereotype(&entity, arg.get("STEREO", 0));
            let Some(colors) = line_color().colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            entity.set_colors(colors.clone());
            add_link(
                d,
                loc,
                &entity,
                &owner,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::build(Display::null(), 2),
                colors,
            );
            if arg.get("COMPOSITE", 0).is_some() {
                d.push_owner(entity);
            }
            CommandExecutionResult::ok()
        },
    )
}

fn associate() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("NAME1", code_pattern()),
            IRegex::space_zero_or_more(),
            IRegex::named("PARTICIPATION", "([-=])"),
            IRegex::optional(IRegex::named(
                "CARDINALITY",
                "([%pLN]+|\\([%pLN]+,[%s]*[%pLN]+\\))",
            )),
            IRegex::named("PARTICIPATION2", "([-=])"),
            IRegex::space_zero_or_more(),
            IRegex::named("NAME2", code_pattern()),
            IRegex::space_zero_or_more(),
            line_color().regex(),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let name1 = d.clean_id(arg.get("NAME1", 0)).unwrap_or_default();
            let name2 = d.clean_id(arg.get("NAME2", 0)).unwrap_or_default();
            let Some(entity1) = d
                .cuca
                .quark_in_context(true, &name1)
                .ok()
                .and_then(|q| q.data())
            else {
                return CommandExecutionResult::error(format!("No such entity: {name1}"));
            };
            let Some(entity2) = d
                .cuca
                .quark_in_context(true, &name2)
                .ok()
                .and_then(|q| q.data())
            else {
                return CommandExecutionResult::error(format!("No such entity: {name2}"));
            };
            let is_double = arg.get("PARTICIPATION", 0) == Some("=");
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double {
                link_type = link_type.go_bold();
            }
            let cardinality = arg.get("CARDINALITY", 0);
            let link_arg = LinkArg::build(Display::with_newlines(cardinality), 3);
            let Some(colors) = line_color().colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            add_link(
                d,
                loc,
                &entity1,
                &entity2,
                link_type,
                link_arg,
                colors,
            );
            if let Some(link) = d.cuca.links().last() {
                link.set_port_members(
                    d.cuca.port_id(&entity1.name()),
                    d.cuca.port_id(&entity2.name()),
                );
            }
            CommandExecutionResult::ok()
        },
    )
}

fn end_group() -> Cmd {
    single(
        vec![IRegex::start(), IRegex::leaf(r"\}"), IRegex::end()],
        |d, _, _| {
            if d.pop_owner() {
                CommandExecutionResult::ok()
            } else {
                CommandExecutionResult::error("Unbalanced brackets")
            }
        },
    )
}

fn simple_subclass() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("NAME1", code_pattern()),
            IRegex::space_zero_or_more(),
            IRegex::named("PARTICIPATION", "([-=])"),
            IRegex::named("DIRECTION", "([<>])"),
            IRegex::named("PARTICIPATION2", "([-=])"),
            IRegex::space_zero_or_more(),
            IRegex::named("NAME2", code_pattern()),
            IRegex::space_zero_or_more(),
            line_color().regex(),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let name1 = d.clean_id(arg.get("NAME1", 0)).unwrap_or_default();
            let name2 = d.clean_id(arg.get("NAME2", 0)).unwrap_or_default();
            let Some(entity1) = d
                .cuca
                .quark_in_context(true, &name1)
                .ok()
                .and_then(|q| q.data())
            else {
                return CommandExecutionResult::error(format!("No such entity: {name1}"));
            };
            let Some(entity2) = d
                .cuca
                .quark_in_context(true, &name2)
                .ok()
                .and_then(|q| q.data())
            else {
                return CommandExecutionResult::error(format!("No such entity: {name2}"));
            };
            let is_double = arg.get("PARTICIPATION", 0) == Some("=");
            let is_superset = arg.get("DIRECTION", 0) == Some(">");
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double {
                link_type = link_type.go_bold();
            }
            link_type = link_type.with_middle(if is_superset {
                LinkMiddleDecor::Superset
            } else {
                LinkMiddleDecor::Subset
            });
            let Some(colors) = line_color().colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            add_link(
                d,
                loc,
                &entity1,
                &entity2,
                link_type,
                LinkArg::build(Display::null(), 3),
                colors,
            );
            if let Some(link) = d.cuca.links().last() {
                link.set_port_members(
                    d.cuca.port_id(&entity1.name()),
                    d.cuca.port_id(&entity2.name()),
                );
            }
            CommandExecutionResult::ok()
        },
    )
}

fn multi_subclass() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::named("SUPERCLASS", code_pattern()),
            IRegex::space_zero_or_more(),
            IRegex::named("PARTICIPATION", "([-=])"),
            IRegex::leaf(">"),
            IRegex::named("PARTICIPATION2", "([-=])"),
            IRegex::space_zero_or_more(),
            IRegex::named("SYMBOL", "([doU])"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{"),
            IRegex::named("SUBCLASSES", "(.+)"),
            IRegex::leaf(r"\}"),
            IRegex::space_zero_or_more(),
            back_color().regex(),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let superclass = d.clean_id(arg.get("SUPERCLASS", 0)).unwrap_or_default();
            let subclasses = arg.get("SUBCLASSES", 0).unwrap_or("");
            let symbol = arg.get("SYMBOL", 0).unwrap_or("o");
            let Some(colors) = back_color().colors(arg) else {
                return CommandExecutionResult::bad_color();
            };
            let center_id = format!("{superclass}/{symbol}{subclasses}/center");
            let center_quark = match d.cuca.quark_in_context(false, &center_id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let center = d.cuca.really_create_leaf(
                loc,
                center_quark,
                Display::create(&[symbol]),
                LeafType::ChenCircle,
            );
            center.set_colors(colors.clone());
            let Some(superclass_entity) = d
                .cuca
                .quark_in_context(true, &superclass)
                .ok()
                .and_then(|q| q.data())
            else {
                return CommandExecutionResult::error(format!("No such entity: {superclass}"));
            };
            let is_double = arg.get("PARTICIPATION", 0) == Some("=");
            let mut link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
            if is_double {
                link_type = link_type.go_bold();
            }
            if symbol == "U" {
                link_type = link_type.with_middle(LinkMiddleDecor::Superset);
            }
            add_link(
                d,
                loc,
                &superclass_entity,
                &center,
                link_type,
                LinkArg::build(Display::null(), 2),
                colors.clone(),
            );
            if let Some(link) = d.cuca.links().last() {
                link.set_port_members(
                    d.cuca.port_id(&superclass_entity.name()),
                    d.cuca.port_id(&center.name()),
                );
            }
            for subclass in subclasses.split(',') {
                let subclass = d.clean_id(Some(subclass.trim())).unwrap_or_default();
                let Some(subclass_entity) = d
                    .cuca
                    .quark_in_context(true, &subclass)
                    .ok()
                    .and_then(|q| q.data())
                else {
                    return CommandExecutionResult::error(format!("No such entity: {subclass}"));
                };
                let mut subclass_link_type = LinkType::new(LinkDecor::None, LinkDecor::None);
                if symbol != "U" {
                    subclass_link_type =
                        subclass_link_type.with_middle(LinkMiddleDecor::Superset);
                }
                add_link(
                    d,
                    loc,
                    &center,
                    &subclass_entity,
                    subclass_link_type,
                    LinkArg::build(Display::null(), 3),
                    colors.clone(),
                );
            }
            CommandExecutionResult::ok()
        },
    )
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = crate::command::common::common_commands();
    cmds.push(crate::classdiagram::commands_misc::rank_dir::<ChenDiagram>());
    cmds.push(create_entity());
    cmds.push(create_attribute());
    cmds.push(associate());
    cmds.push(end_group());
    cmds.push(simple_subclass());
    cmds.push(multi_subclass());
    cmds
}
