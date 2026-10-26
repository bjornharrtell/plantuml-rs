//! Commands of class diagrams (`classdiagram/command`).

use super::ClassDiagram;
use crate::abel::gender::EntityGender;
use crate::abel::{EntityPortion, LeafType, Link, LinkArg, LinkArrow, Stereotag};
use crate::command::{
    Command, CommandExecutionResult, MultiLineCommand, ParserPass, SingleLineCommand, Trim,
};
use crate::decoration::link::{LinkDecor, LinkMiddleDecor, LinkType};
use crate::klimt::color::colors::ColorType;
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

pub(crate) fn back_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn line_color() -> ColorParser {
    ColorParser::simple_color(ColorType::Line)
}

pub(crate) fn add_tags(entity: &crate::abel::Entity, tags: Option<&str>) {
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
        "interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|object|entity|circle|diamond|protocol|struct|exception|metaclass|stereotype|dataclass|record|map"
    } else {
        "interface|enum|annotation|abstract[%s]+class|static[%s]+class|abstract|class|object|entity|protocol|struct|exception|metaclass|stereotype|dataclass|record"
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

// set / hide / show

/// Portion named in a `hide`/`show` command (`getEntityPortion`).
fn entity_portion(s: &str) -> Option<EntityPortion> {
    let s = s.to_lowercase();
    let sub = &s[..s.len().min(3)];
    match sub {
        "met" => Some(EntityPortion::Method),
        "mem" => Some(EntityPortion::Member),
        "att" | "fie" => Some(EntityPortion::Field),
        "cir" => Some(EntityPortion::CircledCharacter),
        "ste" => Some(EntityPortion::Stereotype),
        _ => None,
    }
}

fn namespace_separator() -> Cmd {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("set"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::leaf("separator"),
            IRegex::leaf("namespaceseparator"),
        ]),
        IRegex::space_one_or_more(),
        IRegex::named(
            "SEPARATOR",
            r"((?:none|null)|[\\]{2}|::|[^%pLN%s_$#{}<>%g])",
        ),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        r,
        |d: &mut ClassDiagram, _, arg, _| {
            let s = arg.get("SEPARATOR", 0).unwrap_or("");
            if s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("null") {
                d.cuca.set_namespace_separator(None);
            } else {
                d.cuca.set_namespace_separator(Some(s));
            }
            CommandExecutionResult::ok()
        },
    ))
}

fn type_keyword_gender(word: &str) -> Option<EntityGender> {
    let t = LeafType::from_keyword(word)?;
    Some(EntityGender::by_entity_type(t))
}

const QUOTED_DISPLAY: &str = "[%g][^%g]+[%g]";

fn package_and_namespace_tail(parts: &mut Vec<IRegex>) {
    parts.push(IRegex::space_zero_or_more());
    parts.push(IRegex::named("TAGS1", &format!("{}?", tags_pattern())));
    parts.push(crate::stereo::optional_pattern("STEREO"));
    parts.push(IRegex::named("TAGS2", &format!("{}?", tags_pattern())));
    parts.push(IRegex::space_zero_or_more());
    parts.push(crate::url::optional());
    parts.push(IRegex::space_zero_or_more());
    parts.push(back_color().regex());
    parts.push(IRegex::space_zero_or_more());
    parts.push(IRegex::leaf(r"\{"));
    parts.push(IRegex::end());
}

/// `package [visibility] code [as "display"] [tags] [<<stereo>>] [#color] {` (`CommandPackage`).
fn create_package() -> Cmd {
    let code_bareword = "[%pLN_.$]*";
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
        IRegex::named("TYPE", "(package)"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("CODE1", &format!("({code_bareword}|{QUOTED_DISPLAY})")),
                IRegex::optional(IRegex::group(vec![
                    IRegex::space_one_or_more(),
                    IRegex::leaf("as"),
                    IRegex::space_one_or_more(),
                    IRegex::named("DISPLAY1", &format!("({QUOTED_DISPLAY})")),
                ])),
            ]),
            IRegex::group(vec![
                IRegex::named("DISPLAY2", &format!("({QUOTED_DISPLAY})")),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE2", "([%pLN_.]+)"),
            ]),
        ]),
    ];
    package_and_namespace_tail(&mut parts);
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        |d: &mut ClassDiagram, loc, arg, _| open_group(d, loc, arg),
    ))
}

/// `namespace name {` and `namespace "display" as name {` (`CommandNamespace`).
fn create_namespace() -> Cmd {
    let code = code();
    let mut parts = vec![
        IRegex::start(),
        IRegex::leaf("namespace"),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("DISPLAY1", &format!("({QUOTED_DISPLAY})")),
                IRegex::space_one_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE2", &format!("({code})")),
            ]),
            IRegex::named("CODE1", &format!("({code})")),
        ]),
    ];
    package_and_namespace_tail(&mut parts);
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        |d: &mut ClassDiagram, loc, arg, _| open_group(d, loc, arg),
    ))
}

fn open_group(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let id_raw = unquote(arg.get_lazzy("CODE", 0).unwrap_or(""));
    if id_raw.is_empty() {
        return CommandExecutionResult::error("Error in name");
    }
    let id = d.cuca.clean_id(Some(&id_raw)).unwrap_or_default();
    let quark = match d.cuca.quark_in_context(false, &id) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    let display = arg
        .get_lazzy("DISPLAY", 0)
        .map(|s| unquote(s).to_string())
        .unwrap_or_else(|| quark.name().to_string());
    let status = d.cuca.goto_group(
        loc,
        quark,
        Display::with_newlines(Some(&display)),
        crate::abel::GroupType::Package,
        None,
    );
    if !status.is_ok() {
        return status;
    }
    let group = d.cuca.current_group();
    if let Some(v) = arg.get("VISIBILITY", 0) {
        group.set_visibility_modifier(VisibilityModifier::from_text(&format!("{v}FOO"), false));
    }
    if let Some(st) = arg.get("STEREO", 0) {
        let skin = &d.cuca.base.skin_param;
        let inner = st
            .strip_prefix("<<")
            .and_then(|s| s.strip_suffix(">>"))
            .unwrap_or(st)
            .trim();
        let symbol = crate::decoration::usymbol::from_keyword(
            inner,
            skin.actor_style(),
            skin.component_style(),
        );
        if let Some(symbol) = symbol {
            group.set_usymbol(Some(symbol));
        } else {
            let radius = skin.circled_character_radius();
            group.set_stereotype(Some(Stereotype::build_complex(st, radius)));
            group.set_stereostyle(st);
        }
    }
    add_tags(&group, arg.get_lazzy("TAGS", 0));
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            group.add_url(url);
        }
    }
    let Some(colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    group.set_colors(colors);
    CommandExecutionResult::ok()
}

/// `}` closes the current package, namespace or together block (`CommandEndPackage`).
fn end_package() -> Cmd {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\}"),
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        r,
        |d: &mut ClassDiagram, _, _, _| {
            if d.cuca.end_group() {
                CommandExecutionResult::ok()
            } else {
                CommandExecutionResult::error("No package or namespace defined")
            }
        },
    ))
}

/// `hide`/`show` of a portion, optionally filtered by kind, stereotype or name (`CommandHideShowByGender`).
fn hide_show_by_gender() -> Cmd {
    const TYPE_KEYWORDS: &str = "class|object|interface|enum|annotation|dataclass|record|abstract|protocol|struct|exception|metaclass|stereotype";
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("COMMAND", "(hide|show)"),
        IRegex::space_one_or_more(),
        IRegex::named(
            "GENDER",
            &format!(r"(?:({TYPE_KEYWORDS}|[%pLN_.]+|[%g][^%g]+[%g]|\<\<.*\>\>)[%s]+)*?"),
        ),
        IRegex::optional(IRegex::group(vec![
            IRegex::named("EMPTY", "(empty)"),
            IRegex::space_one_or_more(),
        ])),
        IRegex::named(
            "PORTION",
            "(members?|attributes?|fields?|methods?|circles?|circled?|stereotypes?)",
        ),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        r,
        |d: &mut ClassDiagram, _, arg, _| {
            let Some(portion) = arg.get("PORTION", 0).and_then(entity_portion) else {
                return CommandExecutionResult::error("Unknown portion");
            };
            let show = arg
                .get("COMMAND", 0)
                .is_some_and(|c| c.eq_ignore_ascii_case("show"));
            let empty = arg.get("EMPTY", 0).is_some();
            let mut gender = match arg.get("GENDER", 0) {
                None => EntityGender::all(),
                Some(g) if g.starts_with("<<") => EntityGender::by_stereotype(g),
                Some(g) => match type_keyword_gender(g) {
                    Some(gender) => gender,
                    None => {
                        let name = unquote(g);
                        let id = d.cuca.clean_id(Some(&name)).unwrap_or_default();
                        let quark = match d.cuca.quark_in_context(true, &id) {
                            Ok(q) => q,
                            Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
                        };
                        if portion == EntityPortion::Method {
                            EntityGender::by_class_name(&name)
                        } else {
                            match quark.data() {
                                Some(e) => EntityGender::by_entity_alone(&e),
                                None => {
                                    return CommandExecutionResult::error(format!(
                                        "No such element {}",
                                        quark.name()
                                    ))
                                }
                            }
                        }
                    }
                },
            };
            let current = d.cuca.current_group();
            if !current.is_root() {
                gender = EntityGender::and(gender, EntityGender::by_package(&current));
            }
            let empty_by_gender = |p: EntityPortion| match p {
                EntityPortion::Method => EntityGender::empty_methods(),
                EntityPortion::Field => EntityGender::empty_fields(),
                _ => EntityGender::all(),
            };
            let empty_members = empty && portion == EntityPortion::Member;
            if empty && !empty_members {
                gender = EntityGender::and(gender, empty_by_gender(portion));
            }
            if empty_members {
                for p in [EntityPortion::Field, EntityPortion::Method] {
                    d.cuca.hide_or_show(
                        EntityGender::and(gender.clone(), empty_by_gender(p)),
                        p,
                        show,
                    );
                }
            } else {
                d.cuca.hide_or_show(gender, portion, show);
            }
            CommandExecutionResult::ok()
        },
    ))
}

/// `hide`/`show` of a portion filtered by visibility modifier (`CommandHideShowByVisibility`).
fn hide_show_by_visibility() -> Cmd {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("COMMAND", "(hide|show)"),
        IRegex::space_one_or_more(),
        IRegex::named(
            "VISIBILITY",
            "((?:public|private|protected|package)?(?:[,%s]+(?:public|private|protected|package))*)",
        ),
        IRegex::space_one_or_more(),
        IRegex::named("PORTION", "(members?|attributes?|fields?|methods?)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        r,
        |d: &mut ClassDiagram, _, arg, _| {
            let Some(portion) = arg.get("PORTION", 0).and_then(entity_portion) else {
                return CommandExecutionResult::error("Unknown portion");
            };
            let mut visibilities: Vec<VisibilityModifier> = Vec::new();
            let text = arg.get("VISIBILITY", 0).unwrap_or("").to_lowercase();
            for token in text.split([' ', ',']).filter(|t| !t.is_empty()) {
                for (word, field, method) in [
                    (
                        "public",
                        VisibilityModifier::PublicField,
                        VisibilityModifier::PublicMethod,
                    ),
                    (
                        "private",
                        VisibilityModifier::PrivateField,
                        VisibilityModifier::PrivateMethod,
                    ),
                    (
                        "protected",
                        VisibilityModifier::ProtectedField,
                        VisibilityModifier::ProtectedMethod,
                    ),
                    (
                        "package",
                        VisibilityModifier::PackagePrivateField,
                        VisibilityModifier::PackagePrivateMethod,
                    ),
                ] {
                    if token != word {
                        continue;
                    }
                    if matches!(portion, EntityPortion::Member | EntityPortion::Field) {
                        visibilities.push(field);
                    }
                    if matches!(portion, EntityPortion::Member | EntityPortion::Method) {
                        visibilities.push(method);
                    }
                }
            }
            let show = arg
                .get("COMMAND", 0)
                .is_some_and(|c| c.eq_ignore_ascii_case("show"));
            d.cuca.hide_or_show_visibility_modifier(&visibilities, show);
            CommandExecutionResult::ok()
        },
    ))
}

/// `hide`/`show` of whole elements by name or stereotype (`CommandHideShow2`).
pub(crate) fn hide_show2<D: crate::cucadiagram::CucaHolder + 'static>() -> Box<dyn Command<D>> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("COMMAND", "(hide|hide-class|show|show-class)"),
        IRegex::space_one_or_more(),
        IRegex::named("WHAT", r"([^%s]+|\<\<.*\>\>)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, |d: &mut D, _, arg, _| {
        let show = arg
            .get("COMMAND", 0)
            .is_some_and(|c| c.to_lowercase().starts_with('s'));
        let what = arg.get("WHAT", 0).unwrap_or("").trim();
        d.cuca_mut().hide_or_show2(what, show);
        CommandExecutionResult::ok()
    }))
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

use crate::string_utils::get_queue_direction as queue_direction;

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

pub(crate) fn link_regex() -> IRegex {
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
        None => {
            let leaf_type = if d.is_description() {
                LeafType::StillUnknown
            } else {
                LeafType::Class
            };
            d.cuca.really_create_leaf(
                loc,
                quark.clone(),
                Display::with_newlines(Some(quark.name())),
                leaf_type,
            )
        }
    }
}

/// `(a, b) .. c`, `c .. (a, b)` and `(a, b) .. (c, d)`.
fn execute_association(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    first_is_couple: bool,
    second_is_couple: bool,
) -> CommandExecutionResult {
    let existing =
        |d: &mut ClassDiagram, name: &str| -> Result<crate::abel::Entity, CommandExecutionResult> {
            let name = unquote(name);
            let q = d
                .cuca
                .quark_in_context(true, &name)
                .map_err(|(m, s)| CommandExecutionResult::error_with_score(m, s))?;
            q.data()
                .ok_or_else(|| CommandExecutionResult::error(format!("No class {name}")))
        };
    let single_entity = |d: &mut ClassDiagram, key: &str| {
        let id = unquote(arg.get(key, 0).unwrap_or(""));
        let q = d
            .cuca
            .quark_in_context(true, &id)
            .map_err(|(m, s)| CommandExecutionResult::error_with_score(m, s))?;
        Ok::<_, CommandExecutionResult>(match q.data() {
            Some(e) => e,
            None => d.cuca.really_create_leaf(
                loc,
                q.clone(),
                Display::with_newlines(Some(q.name())),
                LeafType::Class,
            ),
        })
    };
    let link_type = get_link_type(arg);
    let label = Display::with_newlines(arg.get("LABEL_LINK", 0));
    let couple =
        |d: &mut ClassDiagram,
         key: &str|
         -> Result<(crate::abel::Entity, crate::abel::Entity), CommandExecutionResult> {
            let a = existing(d, arg.get(key, 0).unwrap_or(""))?;
            let b = existing(d, arg.get(key, 1).unwrap_or(""))?;
            Ok((a, b))
        };
    let result = (|| -> Result<CommandExecutionResult, CommandExecutionResult> {
        if first_is_couple && second_is_couple {
            let (a1, b1) = couple(d, "COUPLE1")?;
            let (a2, b2) = couple(d, "COUPLE2")?;
            return Ok(d.association_between(loc, &a1, &b1, &a2, &b2, link_type, label));
        }
        let (mode, (a, b), c) = if first_is_couple {
            (1, couple(d, "COUPLE1")?, single_entity(d, "ENT2")?)
        } else {
            (2, couple(d, "COUPLE2")?, single_entity(d, "ENT1")?)
        };
        if d.association_class(loc, mode, &a, &b, &c, link_type, label) {
            Ok(CommandExecutionResult::ok())
        } else {
            Ok(CommandExecutionResult::error(
                "Cannot have more than 2 assocications",
            ))
        }
    })();
    result.unwrap_or_else(|e| e)
}

pub(crate) fn execute_link(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    execute_link_mode(d, loc, arg, false)
}

/// `CommandLinkElement` of a description diagram: notation-aware entity names.
pub(crate) fn execute_link_description(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    for key in ["ENT1", "ENT2"] {
        if let Some(raw) = arg.get(key, 0) {
            if let Err(e) = ensure_description_dummy(d, loc, raw) {
                return e;
            }
        }
    }
    execute_link_mode(d, loc, arg, true)
}

/// Creates an entity from the notation of a link endpoint (`getDummy`).
fn ensure_description_dummy(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    raw: &str,
) -> Result<(), CommandExecutionResult> {
    let raw = unquote(raw);
    let first = if raw.chars().count() > 2 {
        raw.chars().next().unwrap_or('\0')
    } else {
        '\0'
    };
    let end_slash = raw.ends_with('/');
    let id = d.clean_id(Some(&raw)).unwrap_or_default();
    if d.cuca.is_group_code(&id) {
        return Ok(());
    }
    let quark = d
        .cuca
        .quark_in_context(true, &id)
        .map_err(|(m, s)| CommandExecutionResult::error_with_score(m, s))?;
    if quark.data().is_some() {
        return Ok(());
    }
    let display = Display::with_newlines(Some(quark.name()));
    let skin = &d.cuca.base.skin_param;
    let (leaf_type, usymbol) = if raw.starts_with("()") {
        (
            LeafType::Description,
            Some(crate::decoration::usymbol::usymbols::interface()),
        )
    } else if first == '(' {
        if end_slash {
            (
                LeafType::UsecaseBusiness,
                Some(crate::decoration::usymbol::usymbols::usecase_business()),
            )
        } else {
            (
                LeafType::Usecase,
                Some(crate::decoration::usymbol::usymbols::usecase()),
            )
        }
    } else if first == ':' {
        (
            LeafType::Description,
            Some(if end_slash {
                crate::decoration::usymbol::usymbols::actor_stickman_business()
            } else {
                skin.actor_style().to_usymbol()
            }),
        )
    } else if first == '[' {
        (
            LeafType::Description,
            Some(skin.component_style().to_usymbol()),
        )
    } else {
        (LeafType::StillUnknown, None)
    };
    d.cuca
        .really_create_leaf_with(loc, quark, display, leaf_type, usymbol);
    Ok(())
}

fn execute_link_mode(
    d: &mut ClassDiagram,
    loc: Option<LineLocation>,
    arg: &RegexResult,
    description: bool,
) -> CommandExecutionResult {
    let clean = |d: &ClassDiagram, id: Option<&str>| {
        if description {
            d.clean_id(id)
        } else {
            d.cuca.clean_id(id)
        }
    };
    let mut ent1 = clean(d, arg.get("ENT1", 0));
    let mut ent2 = clean(d, arg.get("ENT2", 0));
    let (Some(e1), Some(e2)) = (ent1.as_mut(), ent2.as_mut()) else {
        return execute_association(d, loc, arg, ent1.is_none(), ent2.is_none());
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
    cmds.push(namespace_separator());
    cmds.push(hide_show_by_visibility());
    cmds.push(hide_show_by_gender());
    cmds.push(hide_show2::<ClassDiagram>());
    cmds.push(create_package());
    cmds.push(crate::descdiagram::commands::package_with_usymbol());
    cmds.push(crate::descdiagram::commands::together());
    cmds.push(create_namespace());
    cmds.extend(super::commands_misc::commands());
    cmds.extend(super::commands_group::commands());
    cmds.push(create_class_multi());
    cmds.push(create_class_single());
    cmds.extend(super::commands_note::floating_commands());
    cmds.push(end_package());
    cmds.push(link_class());
    cmds.extend(super::commands_note::on_entity_single_commands());
    cmds.extend(super::commands_note::multi_commands());
    cmds.push(crate::descdiagram::commands::create_element_multilines(0));
    cmds.push(crate::descdiagram::commands::create_element_multilines(1));
    cmds
}

#[allow(dead_code)]
fn _unused(_: ColorType) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{create_system, UmlSource};

    fn parse(text: &str) -> ClassDiagram {
        let source = UmlSource::from_text(text);
        create_system(&commands(), ClassDiagram::new(source.clone()), &source)
            .unwrap_or_else(|e| panic!("{e:?}"))
    }

    fn entity(d: &ClassDiagram, name: &str) -> crate::abel::Entity {
        d.cuca
            .first_with_name(name)
            .and_then(|q| q.data())
            .unwrap_or_else(|| panic!("no entity {name}"))
    }

    #[test]
    fn set_separator_changes_qualified_names() {
        let d = parse("@startuml\nset separator .\nclass App.Entity.A\n@enduml");
        assert_eq!(entity(&d, "A").qualified_name(), "App.Entity.A");
        let d = parse("@startuml\nset namespaceSeparator ::\nclass a::B\n@enduml");
        assert_eq!(entity(&d, "B").qualified_name(), "a::B");
        let d = parse("@startuml\nset separator none\nclass a.b\n@enduml");
        assert_eq!(entity(&d, "a.b").qualified_name(), "a.b");
    }

    #[test]
    fn hide_and_show_portions() {
        let d = parse("@startuml\nclass A {\n int x\n}\nhide members\n@enduml");
        let a = entity(&d, "A");
        assert!(!d.cuca.show_portion(EntityPortion::Field, &a));
        assert!(!d.cuca.show_portion(EntityPortion::Method, &a));
        let d = parse("@startuml\nhide circle\nclass A\n@enduml");
        let a = entity(&d, "A");
        assert!(!d.cuca.show_portion(EntityPortion::CircledCharacter, &a));
        assert!(d.cuca.show_portion(EntityPortion::Field, &a));
        let d = parse("@startuml\nhide members\nshow members\nclass A\n@enduml");
        assert!(d.cuca.show_portion(EntityPortion::Field, &entity(&d, "A")));
    }

    #[test]
    fn hide_stereotype_and_entity() {
        let d = parse("@startuml\nclass A <<focus>>\nhide <<focus>> stereotype\n@enduml");
        let a = entity(&d, "A");
        assert_eq!(d.cuca.visible_stereotype_labels(&a), Some(Vec::new()));
        let d = parse("@startuml\nclass A\nclass B\nhide B\n@enduml");
        assert!(d.cuca.is_hidden(&entity(&d, "B")));
        assert!(!d.cuca.is_hidden(&entity(&d, "A")));
    }

    #[test]
    fn objects_are_created_and_rendered() {
        let d = parse("@startuml\nobject A\nobject B\n@enduml");
        assert_eq!(entity(&d, "A").leaf_type(), Some(LeafType::Object));
        let svg = crate::render(
            "@startuml\nset separator \\ \nobject App\\Entity\\o1\nobject App\\Entity\\o2\nApp\\Entity\\o1 --> App\\Entity\\o2\n@enduml",
        )
        .expect("renders");
        assert!(svg.contains(">o1<"));
        assert!(svg.contains("data-qualified-name=\"App.Entity.o1\""));
    }

    #[test]
    fn intermediate_packages_are_packed() {
        let svg = crate::render(
            "@startuml\nset separator \\ \n!pragma useIntermediatePackages false\nclass App\\Entity\\ImageRequest\n@enduml",
        )
        .expect("renders");
        assert!(svg.contains("App\\Entity"));
        assert!(!svg.contains("data-qualified-name=\"App\""));
    }

    #[test]
    fn hide_by_visibility_and_empty_portions() {
        let d = parse("@startuml\nclass A {\n -int x\n +int y\n}\nhide private fields\n@enduml");
        let a = entity(&d, "A");
        assert_eq!(a.get().bodier.fields_to_display().len(), 1);
        let d = parse("@startuml\nhide empty members\nclass A\n@enduml");
        assert!(!d.cuca.show_portion(EntityPortion::Field, &entity(&d, "A")));
    }

    #[test]
    fn packages_and_namespaces_open_groups() {
        let d = parse("@startuml\npackage p {\nclass A\n}\n@enduml");
        assert_eq!(entity(&d, "A").qualified_name(), "p.A");
        let d = parse("@startuml\npackage \"Hello\" as uid {\nclass A\n}\n@enduml");
        assert_eq!(entity(&d, "A").qualified_name(), "uid.A");
        assert_eq!(d.cuca.groups().len(), 1);
        assert_eq!(d.cuca.groups()[0].display().text_at(0), "Hello");
        let d = parse("@startuml\nnamespace App.Entity {\nclass A\n}\n@enduml");
        assert_eq!(entity(&d, "A").qualified_name(), "App.Entity.A");
        let d = parse("@startuml\npackage a {\npackage b {\nclass C\n}\n}\n@enduml");
        assert_eq!(entity(&d, "C").qualified_name(), "a.b.C");
    }

    #[test]
    fn package_stereotype_may_name_a_symbol() {
        let d = parse("@startuml\npackage \"Hello\" as uid <<Frame>> {\nclass A\n}\n@enduml");
        let uid = &d.cuca.groups()[0];
        assert!(
            uid.usymbol().is_some(),
            "<<Frame>> selects the frame symbol"
        );
        assert!(
            uid.stereotype().is_none(),
            "a symbol is not also a stereotype"
        );
        let d = parse("@startuml\npackage p <<MyStereotype>> {\nclass A\n}\n@enduml");
        let p = &d.cuca.groups()[0];
        assert!(p.usymbol().is_none());
        assert!(
            p.stereotype().is_some(),
            "an unknown keyword stays a stereotype"
        );
    }

    #[test]
    fn stray_closing_bracket_is_an_error() {
        let source = UmlSource::from_text("@startuml\n}\n@enduml");
        assert!(create_system(&commands(), ClassDiagram::new(source.clone()), &source).is_err());
    }

    #[test]
    fn rankdir_from_direction_statement() {
        let d = parse("@startuml\nleft to right direction\nclass A\n@enduml");
        assert_eq!(
            d.cuca.base.skin_param.rankdir,
            crate::skin::Rankdir::LeftToRight
        );
        let d = parse("@startuml\ntop to bottom direction\nclass A\n@enduml");
        assert_eq!(
            d.cuca.base.skin_param.rankdir,
            crate::skin::Rankdir::TopToBottom
        );
    }
}
