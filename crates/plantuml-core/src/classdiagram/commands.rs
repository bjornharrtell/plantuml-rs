//! Commands of class diagrams (`classdiagram/command`).

use super::ClassDiagram;
use crate::abel::{LeafType, Link, LinkArg, LinkArrow, Stereotag};
use crate::command::{
    Command, CommandExecutionResult, MultiLineCommand, ParserPass, SingleLineCommand, Trim,
};
use crate::decoration::link::{LinkDecor, LinkMiddleDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Direction;
use crate::regex::{IRegex, RegexResult};
use crate::skin::visibility::VisibilityModifier;
use crate::stereo::Stereotype;
use crate::string_utils::{eventually_remove_starting_and_ending_double_quote as unquote, trin};
use crate::text::LineLocation;
use crate::text::{BlocLines, MultilinesStrategy};
use crate::url::{UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ClassDiagram>>;

const GENERIC_LEVELS: usize = 4;

fn generic_regex(level: usize) -> String {
    let part1 = r"(?:[^\<\>/]|\<";
    let part2 = r"\>)*";
    if level == 0 {
        format!("{part1}{part2}")
    } else {
        format!("{part1}{}{part2}", generic_regex(level - 1))
    }
}

fn generic_pattern() -> String {
    format!(r"[^\<\>/]{}", generic_regex(GENERIC_LEVELS))
}

const SEPARATOR_CHAR_SINGLE: &str = r"[^%pLN%s_$#\:{}<>%g]";
const SEPARATOR_CHAR_DOUBLE: &str = r"[\\]{2}|::";

fn separator() -> String {
    format!("(?:{SEPARATOR_CHAR_SINGLE}|{SEPARATOR_CHAR_DOUBLE})")
}

fn single() -> String {
    let s = separator();
    format!("{s}?[%pLN_]+(?:{s}[%pLN_]+)*")
}

fn single_guillemet() -> String {
    let s = separator();
    format!("[%g]{s}?[%pLN_]+(?:{s}[%pLN_]+)*[%g]")
}

fn single2() -> String {
    format!("(?:{}|{})", single(), single_guillemet())
}

fn couple() -> String {
    let s2 = single2();
    format!(r"\([%s]*({s2})[%s]*,[%s]*({s2})[%s]*\)")
}

fn class_identifier() -> String {
    let s = separator();
    format!("({s}?[%pLN_$]+(?:{s}[%pLN_$]+)*|[%g][^%g]+[%g])")
}

fn code() -> String {
    let s = separator();
    format!("{s}?[%pLN_$]+(?:{s}[%pLN_$]+)*")
}

fn codes() -> String {
    let c = code();
    format!(r"{c}(?:\s*,\s*{c})*")
}

const LINE_STYLE_KEY1: &str = r"dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+";
const LINE_STYLE_KEY2: &str =
    r",dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+";

pub fn line_style() -> String {
    format!(r"(?:#\w+|{LINE_STYLE_KEY1})(?:,#\w+|{LINE_STYLE_KEY2})*")
}

fn display_with_generic() -> String {
    format!(r"[%g](.+?)(?:\<({})\>)?[%g]", generic_pattern())
}

fn name_and_code_for_class_with_generic() -> IRegex {
    let code = "[^%s{}%g<>]+";
    IRegex::or(vec![
        IRegex::group(vec![
            IRegex::named("DISPLAY1", &display_with_generic()),
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
            IRegex::named("CODE1", &format!("({code})")),
        ]),
        IRegex::group(vec![
            IRegex::named("CODE2", &format!("({code})")),
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
            IRegex::named("DISPLAY2", &display_with_generic()),
        ]),
        IRegex::named("CODE3", &format!("({code})")),
        IRegex::named("CODE4", "[%g]([^%g]+)[%g]"),
    ])
}

fn tags_pattern() -> String {
    Stereotag::pattern()
}

fn back_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn line_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Line)
}

fn add_tags(entity: &crate::abel::Entity, tags: Option<&str>) {
    let Some(tags) = tags else { return };
    for tag in tags.split(' ').filter(|t| !t.is_empty()) {
        if let Some(t) = tag.strip_prefix('$') {
            entity.add_stereotag(Stereotag(t.to_string()));
        }
    }
}

fn manage_extends(
    location: Option<LineLocation>,
    keyword: &str,
    d: &mut ClassDiagram,
    arg: &RegexResult,
    entity: &crate::abel::Entity,
) -> Result<(), CommandExecutionResult> {
    let Some(kw) = arg.get(keyword, 0) else {
        return Ok(());
    };
    let extends = kw.eq_ignore_ascii_case("extends");
    let mut type2 = LeafType::Class;
    if !extends || entity.leaf_type() == Some(LeafType::Interface) {
        type2 = LeafType::Interface;
    }
    let codes = unquote(arg.get(keyword, 1).unwrap_or(""));
    for s in codes.split(',') {
        let id_short = trin(s);
        let id = d.cuca.clean_id(Some(id_short)).expect("some");
        let quark = d
            .cuca
            .quark_in_context(false, &id)
            .map_err(|(m, score)| CommandExecutionResult::error_with_score(m, score))?;
        let cl2 = match quark.data() {
            Some(e) => e,
            None => d.cuca.really_create_leaf(
                location,
                quark.clone(),
                Display::with_newlines(Some(quark.name())),
                type2,
            ),
        };
        let mut type_link = LinkType::new(LinkDecor::None, LinkDecor::Extends);
        if type2 == LeafType::Interface && entity.leaf_type() != Some(LeafType::Interface) {
            type_link = type_link.go_dashed();
        }
        let uid = d.cuca.unique_sequence("lnk");
        let link_arg = LinkArg::no_display(2)
            .with_quantifier(None, None)
            .with_distance_angle(Some(d.cuca.label_distance()), Some(d.cuca.label_angle()));
        let link = Link::new(location, uid, cl2, entity.clone(), type_link, link_arg);
        d.cuca.add_link(link);
    }
    Ok(())
}

/// Fields shared by the single line and the multi lines class declaration.
struct ClassDecl {
    location: Option<LineLocation>,
    arg: RegexResult,
}

fn create_class(
    d: &mut ClassDiagram,
    decl: &ClassDecl,
    multi: bool,
) -> Result<crate::abel::Entity, CommandExecutionResult> {
    let arg = &decl.arg;
    let location = decl.location;
    let type_string = arg.get("TYPE", 0).unwrap_or("class").to_uppercase();
    let leaf_type = LeafType::from_keyword(&type_string)
        .ok_or_else(|| CommandExecutionResult::error("Unknown type"))?;
    let visibility = arg
        .get("VISIBILITY", 0)
        .and_then(|v| VisibilityModifier::from_text(&format!("{v}FOO"), false));
    let id_short = d.cuca.clean_id(arg.get_lazzy("CODE", 0));
    let id_short = id_short.unwrap_or_default();
    let display_string = arg.get_lazzy("DISPLAY", 0);
    let generic = arg
        .get_lazzy("DISPLAY", 1)
        .or_else(|| arg.get("GENERIC", 0))
        .map(str::to_string);
    let stereo = arg.get("STEREO", 0);
    let quark = d
        .cuca
        .quark_in_context(false, &id_short)
        .map_err(|(m, s)| CommandExecutionResult::error_with_score(m, s))?;
    let mut display = Display::with_newlines(display_string);
    if display_string.is_none() {
        display = Display::with_newlines(Some(quark.name()))
            .with_creole_mode(crate::klimt::creole::CreoleMode::SimpleLine);
    }
    let entity = match quark.data() {
        None => d
            .cuca
            .really_create_leaf(location, quark.clone(), display, leaf_type),
        Some(e) => {
            if !e.mute_to_type(leaf_type) {
                return Err(CommandExecutionResult::error(if multi {
                    format!("Cannot create {id_short} because it already exists")
                } else {
                    "Bad name".to_string()
                }));
            }
            if display_string.is_some() {
                e.set_display(display);
            }
            e
        }
    };
    let check = d.check_if_package_hierarchy_is_ok(&entity);
    if !check.is_ok() {
        return Err(check);
    }
    d.cuca.set_last_entity(Some(entity.clone()));
    entity.set_visibility_modifier(visibility);
    if let Some(st) = stereo {
        let radius = d.cuca.base.skin_param.circled_character_radius();
        entity.set_stereotype(Some(Stereotype::build_complex(st, radius)));
        entity.set_stereostyle(st);
    }
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            entity.add_url(url);
        }
    }
    let Some(mut colors) = back_color().colors(arg) else {
        return Err(CommandExecutionResult::bad_color());
    };
    if let Some(s) = arg.get("LINECOLOR", 1) {
        if let Some(c) = HColor::parse(s) {
            colors = colors.add(ColorType::Line, Some(c));
        }
    }
    if let Some(s) = arg.get("LINECOLOR", 0) {
        colors = colors.add_legacy_stroke(s);
    }
    entity.set_colors(colors);
    if let Some(g) = generic {
        entity.set_generic(Some(g));
    }
    if type_string.contains("STATIC") {
        entity.set_static(true);
    }
    Ok(entity)
}

fn class_regex(with_map_and_circle: bool, multi: bool) -> IRegex {
    let types = if with_map_and_circle {
        "interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|circle|diamond|protocol|struct|exception|metaclass|stereotype|dataclass|record|map"
    } else {
        "interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record"
    };
    let codes = codes();
    let g = generic_pattern();
    let mut parts = vec![
        IRegex::start(),
        IRegex::named(
            "VISIBILITY",
            &format!(
                "({})?",
                VisibilityModifier::regex_for_visibility_character()
            ),
        ),
        IRegex::space_zero_or_more(),
        IRegex::named("TYPE", &format!("({types})")),
        IRegex::space_one_or_more(),
        name_and_code_for_class_with_generic(),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_zero_or_more(),
            IRegex::named("GENERIC", &format!(r"\<({g})\>")),
        ])),
        IRegex::space_zero_or_more(),
        IRegex::named("TAGS1", &format!("{}?", tags_pattern())),
        crate::stereo::optional_pattern("STEREO"),
        IRegex::named("TAGS2", &format!("{}?", tags_pattern())),
        IRegex::space_zero_or_more(),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        back_color().regex(),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::group(vec![
            IRegex::leaf("##"),
            IRegex::named("LINECOLOR", r"(?:\[(dotted|dashed|bold)\])?(\w+)?"),
        ])),
    ];
    let ext_tail = |name: &str, kw: &str| {
        let mut p = vec![
            IRegex::space_one_or_more(),
            IRegex::named(name, &format!("({kw})[%s]+({codes}|[%g]([^%g]+)[%g])")),
        ];
        if multi {
            p.push(IRegex::optional(IRegex::group(vec![
                IRegex::space_zero_or_more(),
                IRegex::leaf(&format!(r"\<({g})\>")),
            ])));
        }
        IRegex::optional(IRegex::group(p))
    };
    parts.push(ext_tail("EXTENDS", "extends"));
    parts.push(ext_tail("IMPLEMENTS", "implements"));
    if multi {
        parts.extend([
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ]);
    } else {
        parts.push(IRegex::optional(IRegex::group(vec![
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\}"),
        ])));
        parts.push(IRegex::end());
    }
    IRegex::concat(parts)
}

fn create_class_single() -> Cmd {
    Box::new(SingleLineCommand::new(
        class_regex(true, false),
        |d: &mut ClassDiagram, loc, arg, _| {
            let decl = ClassDecl {
                location: loc,
                arg: arg.clone(),
            };
            let entity = match create_class(d, &decl, false) {
                Ok(e) => e,
                Err(r) => return r,
            };
            for kw in ["EXTENDS", "IMPLEMENTS"] {
                if let Err(r) = manage_extends(loc, kw, d, arg, &entity) {
                    return r;
                }
            }
            add_tags(&entity, arg.get_lazzy("TAGS", 0));
            CommandExecutionResult::ok()
        },
    ))
}

fn create_class_multi() -> Cmd {
    let starting = class_regex(false, true);
    let re = starting.clone();
    Box::new(
        MultiLineCommand::new(
            starting,
            MultilinesStrategy::RemoveStartingQuote,
            Trim::Both,
            r"^[%s]*\}[%s]*$",
            move |d: &mut ClassDiagram, lines: &BlocLines, _: ParserPass| {
                let first = lines.first().expect("first").trimmed();
                let arg = re.matcher(first.string()).expect("starts");
                let loc = lines.location();
                let decl = ClassDecl {
                    location: loc,
                    arg: arg.clone(),
                };
                let entity = match create_class(d, &decl, true) {
                    Ok(e) => e,
                    Err(r) => return r,
                };
                if lines.size() > 1 {
                    let body = lines.sub_extract(1, 1);
                    for s in body.as_strings() {
                        if !s.is_empty() && VisibilityModifier::is_visibility_character(&s) {
                            d.cuca.set_visibility_modifier_present(true);
                        }
                        entity.add_field_or_method(&s);
                    }
                }
                for kw in ["EXTENDS", "IMPLEMENTS"] {
                    if let Err(r) = manage_extends(loc, kw, d, &arg, &entity) {
                        return r;
                    }
                }
                add_tags(&entity, arg.get_lazzy("TAGS", 0));
                CommandExecutionResult::ok()
            },
        )
        .with_final_bracket(),
    )
}

// links

fn not_null(s: Option<&str>) -> &str {
    s.unwrap_or("")
}

fn arrow_head(arg: &RegexResult, key: &str) -> String {
    not_null(arg.get(key, 0)).replace('_', "")
}

fn full_arrow(arg: &RegexResult) -> String {
    format!(
        "{}{}{}{}{}",
        arrow_head(arg, "ARROW_HEAD1"),
        not_null(arg.get("ARROW_BODY1", 0)),
        not_null(arg.get("ARROW_DIRECTION", 0)),
        not_null(arg.get("ARROW_BODY2", 0)),
        arrow_head(arg, "ARROW_HEAD2"),
    )
}

fn queue_direction(s: &str) -> Direction {
    let s = s.to_lowercase();
    if s.contains("left") {
        return Direction::Left;
    }
    if s.contains("right") {
        return Direction::Right;
    }
    if s.contains("up") {
        return Direction::Up;
    }
    if s.contains("down") {
        return Direction::Down;
    }
    if s.contains('l') {
        return Direction::Left;
    }
    if s.contains('r') {
        return Direction::Right;
    }
    if s.contains('u') {
        return Direction::Up;
    }
    if s.contains('d') {
        return Direction::Down;
    }
    if s.chars().count() == 1 {
        return Direction::Right;
    }
    Direction::Down
}

fn get_direction(arg: &RegexResult) -> Direction {
    let s: String = full_arrow(arg)
        .chars()
        .filter(|c| matches!(c, '-' | '.' | '=') || c.is_alphanumeric() || *c == '_')
        .collect();
    let mut s = s.as_str();
    if let Some(r) = s.strip_prefix('o') {
        s = r;
    }
    if let Some(r) = s.strip_suffix('o') {
        s = r;
    }
    queue_direction(s)
}

fn get_queue_length(arg: &RegexResult) -> i32 {
    full_arrow(arg)
        .chars()
        .filter(|c| matches!(c, '-' | '.' | '='))
        .count() as i32
}

fn get_link_type(arg: &RegexResult) -> LinkType {
    let decors1 = LinkDecor::lookup1(
        Some(&arrow_head(arg, "ARROW_HEAD1"))
            .filter(|s| !s.is_empty())
            .map(String::as_str),
    );
    let decors2 = LinkDecor::lookup2(
        Some(&arrow_head(arg, "ARROW_HEAD2"))
            .filter(|s| !s.is_empty())
            .map(String::as_str),
    );
    let mut result = LinkType::new(decors2, decors1);
    if not_null(arg.get("ARROW_BODY1", 0)).contains('.')
        || not_null(arg.get("ARROW_BODY2", 0)).contains('.')
    {
        result = result.go_dashed();
    }
    match arg.get("INSIDE", 0) {
        Some("0") => result = result.with_middle(LinkMiddleDecor::Circle),
        Some("0)") => result = result.with_middle(LinkMiddleDecor::CircleCircled1),
        Some("(0") => result = result.with_middle(LinkMiddleDecor::CircleCircled2),
        Some("(0)") => result = result.with_middle(LinkMiddleDecor::CircleCircled),
        _ => {}
    }
    result
}

/// Cardinalities, roles and arrow of a link label (`Labels`, `StringWithArrow`).
struct Labels {
    first_label: Option<String>,
    second_label: Option<String>,
    first_role: Option<String>,
    second_role: Option<String>,
    label: Option<String>,
    link_arrow: LinkArrow,
}

impl Labels {
    fn new(arg: &RegexResult) -> Labels {
        let clean_role = |r: Option<&str>| {
            r.map(|r| {
                crate::string_utils::eventually_remove_starting_and_ending_double_quote_with(
                    r, "\"",
                )
            })
        };
        let mut l = Labels {
            first_label: arg.get("FIRST_LABEL", 0).map(str::to_string),
            second_label: arg.get("SECOND_LABEL", 0).map(str::to_string),
            first_role: clean_role(arg.get("FIRST_ROLE", 0)),
            second_role: clean_role(arg.get("SECOND_ROLE", 0)),
            label: None,
            link_arrow: LinkArrow::NoneOrSeveral,
        };
        let label_link = arg.get("LABEL_LINK", 0).map(|s| l.init(s));
        let (label, arrow) = string_with_arrow(label_link.as_deref());
        l.label = label;
        l.link_arrow = arrow;
        l
    }

    fn init(&mut self, label_link: &str) -> String {
        use std::sync::LazyLock;
        static BOTH: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
            crate::regex::compile(r#"^[%g]([^%g]+)[%g]([^%g]+)[%g]([^%g]+)[%g]$"#)
        });
        static FIRST: LazyLock<fancy_regex::Regex> =
            LazyLock::new(|| crate::regex::compile(r#"^[%g]([^%g]+)[%g]([^%g]+)$"#));
        static SECOND: LazyLock<fancy_regex::Regex> =
            LazyLock::new(|| crate::regex::compile(r#"^([^%g]+)[%g]([^%g]+)[%g]$"#));
        if self.first_label.is_none() && self.second_label.is_none() {
            let clean = |s: &str| trin(&unquote(trin(s))).to_string();
            if let Ok(Some(c)) = BOTH.captures(label_link) {
                self.first_label = Some(c[1].to_string());
                self.second_label = Some(c[3].to_string());
                return clean(&c[2]);
            }
            if let Ok(Some(c)) = FIRST.captures(label_link) {
                self.first_label = Some(c[1].to_string());
                self.second_label = None;
                return clean(&c[2]);
            }
            if let Ok(Some(c)) = SECOND.captures(label_link) {
                self.first_label = None;
                self.second_label = Some(c[2].to_string());
                return clean(&c[1]);
            }
        }
        crate::string_utils::eventually_remove_starting_and_ending_double_quote_with(
            label_link, "\"",
        )
    }
}

fn has_several_guide_lines(s: &str) -> bool {
    let all: Vec<&str> = s.split("\\n").collect();
    if all.len() <= 1 {
        return false;
    }
    all.iter().any(|s| {
        s.starts_with("< ") || s.starts_with("> ") || s.ends_with(" <") || s.ends_with(" >")
    })
}

fn string_with_arrow(complete: Option<&str>) -> (Option<String>, LinkArrow) {
    let Some(c) = complete else {
        return (None, LinkArrow::NoneOrSeveral);
    };
    let c = crate::string_utils::eventually_remove_starting_and_ending_double_quote_with(c, "\"");
    if has_several_guide_lines(&c) {
        return (Some(c), LinkArrow::NoneOrSeveral);
    }
    if c == "<" {
        (None, LinkArrow::Backward)
    } else if c == ">" {
        (None, LinkArrow::DirectNormal)
    } else if let Some(r) = c.strip_prefix("< ") {
        (Some(trin(r).to_string()), LinkArrow::Backward)
    } else if let Some(r) = c.strip_prefix("> ") {
        (Some(trin(r).to_string()), LinkArrow::DirectNormal)
    } else if let Some(r) = c.strip_suffix(" >") {
        (Some(trin(r).to_string()), LinkArrow::DirectNormal)
    } else if let Some(r) = c.strip_suffix(" <") {
        (Some(trin(r).to_string()), LinkArrow::Backward)
    } else {
        (Some(c), LinkArrow::NoneOrSeveral)
    }
}

fn link_regex() -> IRegex {
    let quals = |name: &str| {
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::leaf(r"[\[]"),
            IRegex::named(name, r"([^\[\]]+)"),
            IRegex::leaf(r"[\]]"),
            IRegex::space_one_or_more(),
        ]))
    };
    let ls = line_style();
    IRegex::concat(vec![
        IRegex::start(),
        IRegex::optional(IRegex::group(vec![
            IRegex::named("HEADER", r"@([\d.]+)"),
            IRegex::space_one_or_more(),
        ])),
        IRegex::or(vec![
            IRegex::named("ENT1", &class_identifier()),
            IRegex::named("COUPLE1", &couple()),
        ]),
        IRegex::space_zero_or_more(),
        quals("QUALIFIER1"),
        IRegex::optional(IRegex::named("FIRST_LABEL", "[%g]([^%g]+)[%g]")),
        IRegex::optional(IRegex::named("FIRST_ROLE", "/([^%s]+|[%g][^%g]+[%g])")),
        IRegex::space_zero_or_more(),
        IRegex::group(vec![
            IRegex::named("ARROW_HEAD1", &LinkDecor::regex_decors1()),
            IRegex::named("ARROW_BODY1", "([-=.]+)"),
            IRegex::named("ARROW_STYLE1", &format!(r"(?:\[({ls})\])?")),
            IRegex::named("ARROW_DIRECTION", "(left|right|up|down|le?|ri?|up?|do?)?"),
            IRegex::optional(IRegex::named("INSIDE", r"(0|\(0\)|\(0|0\))(?=[-=.~])")),
            IRegex::named("ARROW_STYLE2", &format!(r"(?:\[({ls})\])?")),
            IRegex::named("ARROW_BODY2", "([-=.]*)"),
            IRegex::named("ARROW_HEAD2", &LinkDecor::regex_decors2()),
        ]),
        IRegex::space_zero_or_more(),
        IRegex::optional(IRegex::named("SECOND_LABEL", "[%g]([^%g]+)[%g]")),
        IRegex::optional(IRegex::named("SECOND_ROLE", "/([^%s]+|[%g][^%g]+[%g])")),
        quals("QUALIFIER2"),
        IRegex::space_zero_or_more(),
        IRegex::or(vec![
            IRegex::named("ENT2", &class_identifier()),
            IRegex::named("COUPLE2", &couple()),
        ]),
        IRegex::space_zero_or_more(),
        line_color().regex(),
        IRegex::space_zero_or_more(),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        crate::stereo::optional_pattern("STEREOTYPE"),
        IRegex::optional(IRegex::group(vec![
            IRegex::leaf(":"),
            IRegex::space_zero_or_more(),
            IRegex::named("LABEL_LINK", "(.+)"),
        ])),
        IRegex::end(),
    ])
}

fn link_class() -> Cmd {
    Box::new(SingleLineCommand::new(
        link_regex(),
        |d: &mut ClassDiagram, loc, arg, _| execute_link(d, loc, arg),
    ))
}

fn get_or_create_class(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    quark: crate::plasma::Quark<crate::abel::Entity>,
) -> crate::abel::Entity {
    match quark.data() {
        Some(e) => e,
        None => d.cuca.really_create_leaf(
            loc,
            quark.clone(),
            Display::with_newlines(Some(quark.name())),
            LeafType::Class,
        ),
    }
}

fn execute_link(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let mut ent1 = d.cuca.clean_id(arg.get("ENT1", 0));
    let mut ent2 = d.cuca.clean_id(arg.get("ENT2", 0));
    let (Some(e1), Some(e2)) = (ent1.as_mut(), ent2.as_mut()) else {
        return CommandExecutionResult::error("Association classes are not supported yet");
    };
    let (mut port1, mut port2) = (None, None);
    let link_type = get_link_type(arg);
    if e1.contains("::") && d.cuca.first_with_name(e1).is_none() {
        port1 = d.cuca.port_id(e1);
        *e1 = d.cuca.remove_port_id(e1);
    }
    if e2.contains("::") && d.cuca.first_with_name(e2).is_none() {
        port2 = d.cuca.port_id(e2);
        *e2 = d.cuca.remove_port_id(e2);
    }
    let q1 = match d.cuca.quark_in_context(true, e1) {
        Ok(q) => q,
        Err((m, _)) => return CommandExecutionResult::error(m),
    };
    let q2 = match d.cuca.quark_in_context(true, e2) {
        Ok(q) => q,
        Err((m, _)) => return CommandExecutionResult::error(m),
    };
    let cl1 = get_or_create_class(d, loc, q1);
    let cl2 = get_or_create_class(d, loc, q2);
    let dir = get_direction(arg);
    let queue = if matches!(dir, Direction::Left | Direction::Right) {
        1
    } else {
        get_queue_length(arg)
    };
    let labels = Labels::new(arg);
    let kal1 = arg.get("QUALIFIER1", 0).map(str::to_string);
    let kal2 = arg.get("QUALIFIER2", 0).map(str::to_string);
    let display = Display::with_newlines(labels.label.as_deref());
    let link_arg = LinkArg::build_with(
        display,
        queue,
        d.cuca.base.skin_param.class_attribute_icon_size() > 0,
    )
    .with_quantifier(labels.first_label.clone(), labels.second_label.clone())
    .with_role(labels.first_role.clone(), labels.second_role.clone())
    .with_distance_angle(Some(d.cuca.label_distance()), Some(d.cuca.label_angle()))
    .with_kal(kal1, kal2);
    let uid = d.cuca.unique_sequence("lnk");
    let mut link = Link::new(loc, uid, cl1, cl2, link_type, link_arg);
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        link.set_url(UrlBuilder::new(top, UrlMode::Strict).url(u));
    }
    link.set_port_members(port1, port2);
    if matches!(dir, Direction::Left | Direction::Up) {
        let uid = d.cuca.unique_sequence("lnk");
        link = link.inv(uid);
    }
    link.set_link_arrow(labels.link_arrow);
    let Some(colors) = line_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    link.set_colors(colors);
    link.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    if let Some(st) = arg.get("STEREOTYPE", 0) {
        link.set_stereotype(Some(Stereotype::build(st)));
    }
    link.set_code_line(loc);
    d.cuca.add_link(link.clone());
    if let Some(w) = arg.get("HEADER", 0) {
        if let Ok(w) = w.parse::<f64>() {
            link.get_mut().weight = w;
        }
    }
    CommandExecutionResult::ok()
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.push(create_class_multi());
    cmds.push(create_class_single());
    cmds.push(link_class());
    cmds
}

#[allow(dead_code)]
fn _unused(_: Colors) {}
