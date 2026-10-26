//! Notes of class diagrams: floating, on an entity and on a link
//! (`CommandFactoryNote`, `CommandFactoryNoteOnEntity`, `CommandFactoryNoteOnLink`).

use super::ClassDiagram;
use crate::abel::{CucaNote, LeafType, Link, LinkArg, Stereotag};
use crate::command::{Command, CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::decoration::link::{LinkDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Position;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereotype};
use crate::string_utils::eventually_remove_starting_and_ending_double_quote as unquote;
use crate::text::{BlocLines, LineLocation, MultilinesStrategy};
use crate::url::{self, UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ClassDiagram>>;

const CODE: &str = r#"([^%s{}%g<>]+|[%g][^%g]+[%g])"#;
const END_NOTE: &str = "^[%s]*end[%s]?note$";
const END_BRACKET: &str = r"^(\})$";

fn back_color() -> ColorParser {
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

/// The text between the first and the last line of a multi-line command.
fn body_display(lines: &BlocLines) -> (Option<LineLocation>, Display) {
    let body = lines.sub_extract(1, 1).remove_empty_columns();
    (body.location(), Display::create(&body.as_strings()))
}

fn starting_of(lines: &BlocLines, re: &IRegex) -> Option<RegexResult> {
    re.matcher(lines.first()?.trimmed().string())
}

type Exec =
    fn(&mut ClassDiagram, Option<LineLocation>, &RegexResult, Display) -> CommandExecutionResult;

fn single_cmd(re: IRegex, key: &'static str, exec: Exec) -> Cmd {
    Box::new(SingleLineCommand::new(
        re,
        move |d: &mut ClassDiagram, loc, arg, _| {
            let display = Display::with_newlines(arg.get(key, 0));
            exec(d, loc, arg, display)
        },
    ))
}

fn multi_cmd(re: IRegex, end: &str, exec: Exec) -> Cmd {
    let starting = re.clone();
    Box::new(MultiLineCommand::new(
        re,
        MultilinesStrategy::KeepStartingQuote,
        Trim::Both,
        end,
        move |d: &mut ClassDiagram, lines: &BlocLines, _| {
            let Some(arg) = starting_of(lines, &starting) else {
                return CommandExecutionResult::error("Bad note");
            };
            let (loc, display) = body_display(lines);
            exec(d, loc, &arg, display)
        },
    ))
}

// floating note

fn floating_tail() -> Vec<IRegex> {
    vec![
        IRegex::named("CODE", "([%pLN_.]+)"),
        IRegex::space_zero_or_more(),
        tags("TAGS"),
        optional_pattern("STEREO"),
        back_color().regex(),
        IRegex::end(),
    ]
}

fn execute_floating(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandExecutionResult {
    let id = d.cuca.clean_id(arg.get("CODE", 0)).unwrap_or_default();
    let quark = match d.cuca.quark_in_context(false, &id) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    if quark.data().is_some() {
        return CommandExecutionResult::error(format!("Note already created: {}", quark.name()));
    }
    let entity = d
        .cuca
        .really_create_leaf(loc, quark, display, LeafType::Note);
    let Some(colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    entity.set_colors(colors);
    if let Some(s) = arg.get("STEREO", 0) {
        entity.set_stereotype(Some(Stereotype::build(s)));
    }
    add_tags(&entity, arg.get("TAGS", 0));
    CommandExecutionResult::ok()
}

fn floating_single() -> Cmd {
    let mut parts = vec![
        IRegex::start(),
        IRegex::leaf("note"),
        IRegex::space_one_or_more(),
        IRegex::leaf("[%g]"),
        IRegex::named("DISPLAY", "([^%g]+)"),
        IRegex::leaf("[%g]"),
        IRegex::space_one_or_more(),
        IRegex::leaf("as"),
        IRegex::space_one_or_more(),
    ];
    parts.extend(floating_tail());
    single_cmd(IRegex::concat(parts), "DISPLAY", execute_floating)
}

fn floating_multi() -> Cmd {
    let mut parts = vec![
        IRegex::start(),
        IRegex::leaf("note"),
        IRegex::space_one_or_more(),
        IRegex::leaf("as"),
        IRegex::space_one_or_more(),
    ];
    parts.extend(floating_tail());
    multi_cmd(IRegex::concat(parts), END_NOTE, execute_floating)
}

// note on entity

fn on_entity_regex(single: bool, multi_bracket: Option<bool>) -> IRegex {
    let mut v = vec![
        IRegex::start(),
        IRegex::leaf("note"),
        IRegex::space_one_or_more(),
        IRegex::named("POSITION", "(right|left|top|bottom)"),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::leaf("of"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE", CODE),
            ]),
            IRegex::leaf(""),
        ]),
        IRegex::space_zero_or_more(),
        tags("TAGS1"),
        optional_pattern("STEREO"),
        tags("TAGS2"),
        IRegex::space_zero_or_more(),
        back_color().regex(),
        IRegex::space_zero_or_more(),
        url::optional(),
    ];
    if single {
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::leaf(":"));
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::named("NOTE", "(.*)"));
    } else if multi_bracket == Some(true) {
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::leaf(r"\{"));
    }
    v.push(IRegex::end());
    IRegex::concat(v)
}

fn execute_on_entity(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandExecutionResult {
    let cl1 = match d.cuca.clean_id(arg.get("CODE", 0)) {
        None => match d.cuca.last_entity() {
            Some(e) => e,
            None => return CommandExecutionResult::error("Nothing to note to"),
        },
        Some(id) => {
            let quark = match d.cuca.quark_in_context(true, &id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            match quark.data() {
                Some(e) => e,
                None => return CommandExecutionResult::error(format!("Not known: {id}")),
            }
        }
    };
    let position = Position::from_string(arg.get("POSITION", 0).unwrap_or("right"))
        .unwrap_or(Position::Right)
        .with_rankdir(d.cuca.base.skin_param.rankdir);
    let Some(mut colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    let mut stereotype = None;
    if let Some(s) = arg.get("STEREO", 0) {
        let st = Stereotype::build(s);
        match d.cuca.base.skin_param.colors_for_note(&colors, &st) {
            Some(c) => colors = c,
            None => return CommandExecutionResult::bad_color(),
        }
        stereotype = Some(st);
    }
    let tmp = d.cuca.unique_sequence("GMN");
    let quark = match d.cuca.quark_in_context(true, &tmp) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    let note = d
        .cuca
        .really_create_leaf(loc, quark, display, LeafType::Note);
    if stereotype.is_some() {
        note.set_stereotype(stereotype);
    }
    note.set_colors(colors);
    if let Some(u) = arg.get(url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(u) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            note.add_url(u);
        }
    }
    add_tags(&note, arg.get_lazzy("TAGS", 0));
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).go_dashed();
    let (a, b, length) = match position {
        Position::Right => (cl1, note, 1),
        Position::Left => (note, cl1, 1),
        Position::Bottom => (cl1, note, 2),
        Position::Top => (note, cl1, 2),
    };
    let uid = d.cuca.unique_sequence("lnk");
    let link = Link::new(loc, uid, a, b, link_type, LinkArg::no_display(length));
    if length == 1 {
        link.get_mut().horizontal_solitary = true;
    }
    d.cuca.add_link(link);
    CommandExecutionResult::ok()
}

fn on_entity_single() -> Cmd {
    single_cmd(on_entity_regex(true, None), "NOTE", execute_on_entity)
}

fn on_entity_multi(bracket: bool) -> Cmd {
    multi_cmd(
        on_entity_regex(false, Some(bracket)),
        if bracket { END_BRACKET } else { END_NOTE },
        execute_on_entity,
    )
}

// note on a member (tip)

fn tip_regex(bracket: bool) -> IRegex {
    let mut v = vec![
        IRegex::start(),
        IRegex::leaf("note"),
        IRegex::space_one_or_more(),
        IRegex::named("POSITION", "(right|left)"),
        IRegex::space_one_or_more(),
        IRegex::leaf("of"),
        IRegex::space_one_or_more(),
        IRegex::named(
            "CODE",
            "([^%s{}%g<>:]+|[%g][^%g]+[%g])::([%g][^%g]+[%g]|[^%s]+)",
        ),
        IRegex::space_zero_or_more(),
        tags("TAGS1"),
        optional_pattern("STEREO"),
        tags("TAGS2"),
        IRegex::space_zero_or_more(),
        back_color().regex(),
        IRegex::space_zero_or_more(),
        url::optional(),
    ];
    if bracket {
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::leaf(r"\{"));
    }
    v.push(IRegex::end());
    IRegex::concat(v)
}

fn execute_tip(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandExecutionResult {
    let id_short = arg.get("CODE", 0).unwrap_or("").to_string();
    let member = unquote(arg.get("CODE", 1).unwrap_or(""));
    let quark = match d.cuca.quark_in_context(true, &id_short) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    let Some(cl1) = quark.data() else {
        return CommandExecutionResult::error("Nothing to note to");
    };
    let position = Position::from_string(arg.get("POSITION", 0).unwrap_or("right"))
        .unwrap_or(Position::Right)
        .with_rankdir(d.cuca.base.skin_param.rankdir);
    let pos_name = format!("{position:?}").to_uppercase();
    let tmp = unquote(&format!("{id_short}$$${pos_name}"));
    let ident_tip = match d.cuca.quark_in_context(true, &tmp) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    let tips = match ident_tip.data() {
        Some(t) => t,
        None => {
            let tips = d.cuca.really_create_leaf(
                loc,
                ident_tip,
                Display::with_newlines(Some("")),
                LeafType::Tips,
            );
            let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).invisible();
            let (a, b) = if position == Position::Right {
                (cl1, tips.clone())
            } else {
                (tips.clone(), cl1)
            };
            let uid = d.cuca.unique_sequence("lnk");
            d.cuca
                .add_link(Link::new(loc, uid, a, b, link_type, LinkArg::no_display(1)));
            tips
        }
    };
    let Some(mut colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    let mut stereotype = None;
    if let Some(s) = arg.get("STEREO", 0) {
        let st = Stereotype::build(s);
        match d.cuca.base.skin_param.colors_for_note(&colors, &st) {
            Some(c) => colors = c,
            None => return CommandExecutionResult::bad_color(),
        }
        stereotype = Some(st);
    }
    tips.put_tip(&member, display, colors, stereotype);
    CommandExecutionResult::ok()
}

fn tip_multi(bracket: bool) -> Cmd {
    multi_cmd(
        tip_regex(bracket),
        if bracket { END_BRACKET } else { END_NOTE },
        execute_tip,
    )
}

// note on link

fn on_link_regex(single: bool) -> IRegex {
    let mut v = vec![
        IRegex::start(),
        IRegex::leaf("note"),
        IRegex::space_one_or_more(),
        IRegex::named("POSITION", "(right|left|top|bottom)?"),
        IRegex::space_zero_or_more(),
        IRegex::named("ON", "(on|of)"),
        IRegex::space_one_or_more(),
        IRegex::leaf("link"),
        IRegex::space_zero_or_more(),
        back_color().regex(),
    ];
    if single {
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::leaf(":"));
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::named("NOTE", "(.*)"));
    }
    v.push(IRegex::end());
    IRegex::concat(v)
}

fn execute_on_link(
    d: &mut ClassDiagram,
    _loc: Option<LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandExecutionResult {
    let Some(link) = d.cuca.last_link() else {
        return CommandExecutionResult::error("No link defined");
    };
    if display.size() == 0 {
        return CommandExecutionResult::error("No note defined");
    }
    let position = arg
        .get("POSITION", 0)
        .filter(|p| !p.is_empty())
        .and_then(Position::from_string)
        .unwrap_or(Position::Bottom);
    let colors: Colors = match back_color().colors(arg) {
        Some(c) => c,
        None => return CommandExecutionResult::bad_color(),
    };
    link.add_note(CucaNote::build(display, position, colors));
    CommandExecutionResult::ok()
}

fn on_link_single() -> Cmd {
    single_cmd(on_link_regex(true), "NOTE", execute_on_link)
}

fn on_link_multi() -> Cmd {
    multi_cmd(on_link_regex(false), END_NOTE, execute_on_link)
}

/// Floating note, `Java order: after CommandCreateElementFull2`.
pub fn floating_commands() -> Vec<Cmd> {
    vec![floating_single()]
}

/// Note on an entity, single line (`after CommandLinkClass`).
pub fn on_entity_single_commands() -> Vec<Cmd> {
    vec![tip_multi(true), tip_multi(false), on_entity_single()]
}

/// Multi-line forms, in Java order.
pub fn multi_commands() -> Vec<Cmd> {
    vec![
        on_entity_multi(true),
        on_entity_multi(false),
        floating_multi(),
        on_link_single(),
        on_link_multi(),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::commands::commands;
    use super::*;
    use crate::command::{create_system, UmlSource};

    fn parse(text: &str) -> ClassDiagram {
        let source = UmlSource::from_text(text);
        create_system(&commands(), ClassDiagram::new(source.clone()), &source)
            .unwrap_or_else(|e| panic!("{e:?}"))
    }

    fn notes(d: &ClassDiagram) -> Vec<crate::abel::Entity> {
        d.cuca
            .leafs()
            .into_iter()
            .filter(|e| e.leaf_type() == Some(LeafType::Note))
            .collect()
    }

    #[test]
    fn floating_notes() {
        let d =
            parse("@startuml\nnote \"hello\" as N1 #red\nnote as N2\n a\n b\nend note\n@enduml");
        let n = notes(&d);
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].display().text_at(0), "hello");
        assert!(n[0].colors().color(ColorType::Back).is_some());
        assert_eq!(n[1].display().size(), 2);
    }

    #[test]
    fn note_on_entity_forms() {
        let d = parse(
            "@startuml\nclass A\nclass B\nnote right of A : one\nnote left of B\n x\n y\nend note\nnote top of A {\n z\n}\nnote bottom : last\n@enduml",
        );
        assert_eq!(notes(&d).len(), 4);
        let links = d.cuca.links();
        assert_eq!(links.len(), 4);
        assert_eq!(links[0].entity1().name(), "A");
        assert_eq!(links[1].entity2().name(), "B");
        assert_eq!(links[2].entity2().name(), "A");
        assert_eq!(links[3].entity2().leaf_type(), Some(LeafType::Note));
        assert!(links[0].get().horizontal_solitary);
        assert!(!links[2].get().horizontal_solitary);
    }

    #[test]
    fn note_on_link() {
        let d = parse(
            "@startuml\nclass A\nclass B\nA --> B\nnote on link : hi\nnote left on link\n a\nend note\n@enduml",
        );
        let link = d.cuca.last_link().expect("link");
        assert!(link.get().note.is_some());
        assert!(notes(&d).is_empty());
    }

    #[test]
    fn tips_on_members_share_one_entity() {
        let d = parse(
            "@startuml\nclass E {\n int f\n g()\n}\nnote right of E::f #yellow\n one\nend note\nnote right of E::g {\n two\n}\nnote left of E::f\n three\nend note\n@enduml",
        );
        let tips: Vec<_> = d
            .cuca
            .leafs()
            .into_iter()
            .filter(|e| e.leaf_type() == Some(LeafType::Tips))
            .collect();
        assert_eq!(tips.len(), 2);
        assert_eq!(tips[0].tips().len(), 2);
        assert!(d.cuca.links().iter().all(|l| l.is_invis()));
    }

    #[test]
    fn unknown_target_is_an_error() {
        let source = UmlSource::from_text("@startuml\nnote right of Zed : x\n@enduml");
        assert!(create_system(&commands(), ClassDiagram::new(source.clone()), &source).is_err());
    }
}
