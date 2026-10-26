//! Commands of description diagrams (`descdiagram/command`).

use crate::abel::{GroupType, LeafType};
use crate::classdiagram::commands::{add_tags, back_color, execute_link_description};
use crate::classdiagram::ClassDiagram;
use crate::command::{
    Command, CommandExecutionResult, MultiLineCommand, ParserPass, SingleLineCommand, Trim,
};
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::parser::ColorParser;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::string_utils::{eventually_remove_starting_and_ending_double_quote as unquote, trin};
use crate::text::{BlocLines, MultilinesStrategy};
use crate::url::{UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ClassDiagram>>;

/// Declaration keywords of a description element (`CommandCreateElementFull.ALL_TYPES`).
const ALL_TYPES: &str = "person|artifact|actor/|actor|folder|card|file|package|rectangle|hexagon|label|node|frame|cloud|action|process|database|queue|stack|storage|agent|usecase/|usecase|component|boundary|control|entity|interface|circle|collections|port|portin|portout";

const CODE: &str =
    r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])";
const CODE_WITH_QUOTE: &str = r"([%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%g].+?[%g])";
const DISPLAY: &str = r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\])";
const DISPLAY_WITHOUT_QUOTE: &str = r"([%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]|[%pLN_.]+)";

fn code_char(s: &str) -> char {
    if s.chars().count() > 2 {
        s.chars().next().unwrap_or('\0')
    } else {
        '\0'
    }
}

fn is_guillemet(c: char) -> bool {
    matches!(c, '"' | '\u{201c}' | '\u{201d}' | '\u{e121}')
}

fn color2() -> ColorParser {
    ColorParser::simple_color_id(ColorType::Back, "COLOR2")
}

fn tags_pattern() -> String {
    crate::abel::Stereotag::pattern()
}

fn forbidden_line(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_alphabetic() || c.is_ascii_digit() || c == '_' || c == '.')
}

// create element, single line

fn create_element_full_pattern() -> IRegex {
    let mut parts = vec![
        IRegex::start(),
        IRegex::named("SYMBOL", &format!("(?:((?:{ALL_TYPES})|\\(\\))[%s]+)?")),
        color2().regex(),
        IRegex::space_zero_or_more(),
        IRegex::or(vec![
            IRegex::named("CODE1", CODE_WITH_QUOTE),
            IRegex::group(vec![
                IRegex::named("DISPLAY2", DISPLAY),
                crate::stereo::optional_pattern("STEREOTYPE2"),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE2", CODE),
            ]),
            IRegex::group(vec![
                IRegex::named("CODE3", CODE),
                crate::stereo::optional_pattern("STEREOTYPE3"),
                IRegex::leaf("as"),
                IRegex::space_zero_or_more(),
                IRegex::named("DISPLAY3", DISPLAY),
            ]),
            IRegex::group(vec![
                IRegex::named("DISPLAY4", DISPLAY_WITHOUT_QUOTE),
                crate::stereo::optional_pattern("STEREOTYPE4"),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE4", CODE),
            ]),
        ]),
        IRegex::space_zero_or_more(),
        IRegex::named("TAGS1", &format!("{}?", tags_pattern())),
        crate::stereo::optional_pattern("STEREOTYPE"),
        IRegex::named("TAGS2", &format!("{}?", tags_pattern())),
        IRegex::space_zero_or_more(),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        back_color().regex(),
        IRegex::end(),
    ];
    parts.shrink_to_fit();
    IRegex::concat(parts)
}

/// The type and symbol chosen by a declaration (`CommandCreateElementFull.executeArg`).
fn full_symbol(
    d: &ClassDiagram,
    symbol: Option<&str>,
) -> (
    LeafType,
    Option<std::rc::Rc<dyn crate::decoration::usymbol::USymbol>>,
) {
    let skin = &d.cuca.base.skin_param;
    let actor = skin.actor_style().to_usymbol();
    match symbol {
        None => (LeafType::Description, Some(actor)),
        Some(s) if s.eq_ignore_ascii_case("portin") => (LeafType::Portin, None),
        Some(s) if s.eq_ignore_ascii_case("portout") => (LeafType::Portout, None),
        Some(s) if s.eq_ignore_ascii_case("port") => (LeafType::Portin, None),
        Some(s) if s.eq_ignore_ascii_case("usecase") => (LeafType::Usecase, None),
        Some(s) if s.eq_ignore_ascii_case("usecase/") => (LeafType::UsecaseBusiness, None),
        Some(s) if s.eq_ignore_ascii_case("circle") => (LeafType::Circle, None),
        Some(s) => (
            LeafType::Description,
            crate::decoration::usymbol::from_keyword(s, skin.actor_style(), skin.component_style()),
        ),
    }
}

fn same_symbol(
    a: &Option<std::rc::Rc<dyn crate::decoration::usymbol::USymbol>>,
    b: &Option<std::rc::Rc<dyn crate::decoration::usymbol::USymbol>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.snames() == b.snames(),
        _ => false,
    }
}

fn execute_create_element_full(
    d: &mut ClassDiagram,
    loc: Option<crate::text::LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let mut code_raw = arg.get_lazzy("CODE", 0).unwrap_or("").to_string();
    let mut display_raw = arg.get_lazzy("DISPLAY", 0).map(str::to_string);
    let code_first = code_char(&code_raw);
    let display_first = display_raw.as_deref().map(code_char).unwrap_or('\0');
    let mut symbol = arg.get("SYMBOL", 0).map(str::to_string);

    if let Some(rest) = code_raw.strip_prefix("()") {
        symbol = Some("interface".to_string());
        code_raw = trin(rest).to_string();
    } else if code_first == '(' || display_first == '(' {
        if symbol.as_deref().is_some_and(|s| s.ends_with('/')) {
            symbol = Some("usecase/".to_string());
        } else if display_raw.as_deref().is_some_and(|s| s.ends_with(")/")) {
            let mut s = display_raw.take().expect("display");
            s.truncate(s.len() - 1);
            display_raw = Some(s);
            symbol = Some("usecase/".to_string());
        } else if code_raw.ends_with(")/") {
            code_raw.truncate(code_raw.len() - 1);
            symbol = Some("usecase/".to_string());
        } else {
            symbol = Some("usecase".to_string());
        }
    } else if code_first == ':' || display_first == ':' {
        if symbol.as_deref().is_some_and(|s| s.ends_with('/')) {
            symbol = Some("actor/".to_string());
        } else if display_raw.as_deref().is_some_and(|s| s.ends_with(":/")) {
            let mut s = display_raw.take().expect("display");
            s.truncate(s.len() - 1);
            display_raw = Some(s);
            symbol = Some("actor/".to_string());
        } else if code_raw.ends_with(":/") {
            code_raw.truncate(code_raw.len() - 1);
            symbol = Some("actor/".to_string());
        } else {
            symbol = Some("actor".to_string());
        }
    } else if code_first == '[' || display_first == '[' {
        symbol = Some("component".to_string());
    }

    let (leaf_type, usymbol) = full_symbol(d, symbol.as_deref());

    let id = match d.clean_id(Some(&code_raw)) {
        Some(id) => id,
        None => return CommandExecutionResult::error("Bad name"),
    };
    let quark = match d.cuca.quark_in_context(false, &id) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    if quark.data().is_some_and(|e| e.is_group()) {
        return CommandExecutionResult::error(format!(
            "This element ({}) is already defined",
            quark.name()
        ));
    }
    if let Some(other) = quark.data() {
        if other.leaf_type() != Some(leaf_type) || !same_symbol(&other.usymbol(), &usymbol) {
            return CommandExecutionResult::error(format!(
                "This element ({}) is already defined",
                quark.name()
            ));
        }
    }
    if matches!(leaf_type, LeafType::Portin | LeafType::Portout) && d.cuca.current_group().is_root()
    {
        return CommandExecutionResult::error(
            "Port can only be used inside an element and not at root level",
        );
    }

    let display = display_raw.unwrap_or_else(|| quark.name().to_string());
    let entity = match quark.data() {
        Some(e) => e,
        None => d.cuca.really_create_leaf_with(
            loc,
            quark.clone(),
            Display::with_newlines(Some(&unquote(&display))),
            leaf_type,
            usymbol,
        ),
    };
    entity.set_display(Display::with_newlines(Some(&unquote(&display))));

    if let Some(st) = arg.get_lazzy("STEREOTYPE", 0) {
        let radius = d.cuca.base.skin_param.circled_character_radius();
        entity.set_stereotype(Some(Stereotype::build_complex(st, radius)));
    }
    add_tags(&entity, arg.get_lazzy("TAGS", 0));
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            entity.add_url(url);
        }
    }
    let Some(colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    entity.set_colors(colors);
    CommandExecutionResult::ok()
}

fn create_element_full() -> Cmd {
    Box::new(
        SingleLineCommand::new(
            create_element_full_pattern(),
            |d: &mut ClassDiagram, loc, arg, _| execute_create_element_full(d, loc, arg),
        )
        .forbid_text(forbidden_line),
    )
}

// create element, multilines

fn create_element_multilines_pattern(multiline_type: usize) -> IRegex {
    let mut parts = vec![
        IRegex::start(),
        IRegex::named("TYPE", &format!("({ALL_TYPES})[%s]+")),
        IRegex::named("CODE", "([%pLN_.]+)"),
        crate::stereo::optional_pattern("STEREO"),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        crate::klimt::color::parser::exp1(),
        IRegex::space_zero_or_more(),
    ];
    if multiline_type == 0 {
        parts.extend([
            IRegex::leaf("as"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("[%g]"),
            IRegex::named("DESC", "([^%g]*)"),
        ]);
    } else {
        parts.extend([IRegex::leaf(r"\["), IRegex::named("DESC", "(.*)")]);
    }
    parts.push(IRegex::end());
    IRegex::concat(parts)
}

fn multiline_type_usymbol(
    d: &ClassDiagram,
    symbol: &str,
) -> (
    LeafType,
    Option<std::rc::Rc<dyn crate::decoration::usymbol::USymbol>>,
) {
    let skin = &d.cuca.base.skin_param;
    if symbol.eq_ignore_ascii_case("usecase") {
        (
            LeafType::Usecase,
            Some(crate::decoration::usymbol::usymbols::usecase()),
        )
    } else if symbol.eq_ignore_ascii_case("usecase/") {
        (
            LeafType::UsecaseBusiness,
            Some(crate::decoration::usymbol::usymbols::usecase_business()),
        )
    } else {
        (
            LeafType::Description,
            crate::decoration::usymbol::from_string(
                symbol,
                skin.actor_style(),
                skin.component_style(),
                skin.package_style(),
            ),
        )
    }
}

fn execute_create_element_multilines(
    d: &mut ClassDiagram,
    lines: &BlocLines,
    multiline_type: usize,
) -> CommandExecutionResult {
    let location = lines.location();
    let lines = lines.trim_smart(1);
    let re = create_element_multilines_pattern(multiline_type);
    let first = lines.first().expect("first").trimmed();
    let Some(line0) = re.matcher(first.string()) else {
        return CommandExecutionResult::error("Cannot parse element");
    };
    let symbol = line0.get("TYPE", 0).unwrap_or("").to_string();
    let (leaf_type, usymbol) = multiline_type_usymbol(d, &symbol);
    let id_short = line0.get("CODE", 0).unwrap_or("").to_string();
    let last = lines.last().trimmed();
    let last_string = last.string();
    let tail = if multiline_type == 0 {
        if last_string.chars().last().is_some_and(is_guillemet) {
            &last_string[..last_string.len() - 1]
        } else {
            last_string
        }
    } else {
        last_string.strip_suffix(']').unwrap_or(last_string)
    };

    let mut body: Vec<String> = Vec::new();
    if let Some(desc_start) = line0.get("DESC", 0) {
        if !desc_start.is_empty() {
            body.push(desc_start.to_string());
        }
    }
    if lines.size() > 2 {
        body.extend(lines.sub_extract(1, 1).as_strings());
    }
    if !tail.is_empty() {
        body.push(tail.to_string());
    }
    let display = Display::create(&body);

    let id = match d.clean_id(Some(&id_short)) {
        Some(id) => id,
        None => return CommandExecutionResult::error("Bad name"),
    };
    let quark = match d.cuca.quark_in_context(true, &id) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    if let Some(other) = quark.data() {
        if other.leaf_type() != Some(leaf_type) || !same_symbol(&other.usymbol(), &usymbol) {
            return CommandExecutionResult::error(format!(
                "This element ({}) is already defined",
                quark.name()
            ));
        }
    }
    let result = match quark.data() {
        Some(e) => e,
        None => {
            d.cuca
                .really_create_leaf_with(location, quark.clone(), display, leaf_type, usymbol)
        }
    };

    if let Some(st) = line0.get("STEREO", 0) {
        let radius = d.cuca.base.skin_param.circled_character_radius();
        result.set_stereotype(Some(Stereotype::build_complex(st, radius)));
    }
    if let Some(u) = line0.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            result.add_url(url);
        }
    }
    let Some(colors) = back_color().colors(&line0) else {
        return CommandExecutionResult::bad_color();
    };
    result.set_colors(colors);
    CommandExecutionResult::ok()
}

pub(crate) fn create_element_multilines(multiline_type: usize) -> Cmd {
    let end = if multiline_type == 0 {
        r"^(.*)[%g]$"
    } else {
        r"^([^\[\]]*)\]$"
    };
    Box::new(MultiLineCommand::new(
        create_element_multilines_pattern(multiline_type),
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        end,
        move |d: &mut ClassDiagram, lines: &BlocLines, _: ParserPass| {
            execute_create_element_multilines(d, lines, multiline_type)
        },
    ))
}

// package with a symbol

const PACKAGE_TYPES: &str = "package|rectangle|hexagon|node|artifact|folder|file|frame|cloud|action|process|database|storage|component|card|queue|stack";

fn package_with_usymbol_pattern() -> IRegex {
    IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("SYMBOL", &format!("({PACKAGE_TYPES})")),
        IRegex::space_one_or_more(),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::named("DISPLAY1", "([%g].+?[%g])"),
                IRegex::optional(IRegex::group(vec![
                    IRegex::space_one_or_more(),
                    IRegex::named("STEREOTYPE1", r"(\<\<.+\>\>)"),
                ])),
                IRegex::space_zero_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE1", "([^#%s{}]+)"),
            ]),
            IRegex::group(vec![
                IRegex::named("CODE2", "([^#%s{}%g]+)"),
                IRegex::optional(IRegex::group(vec![
                    IRegex::space_one_or_more(),
                    IRegex::named("STEREOTYPE2", r"(\<\<.+\>\>)"),
                ])),
                IRegex::space_zero_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("DISPLAY2", "([%g].+?[%g])"),
            ]),
            IRegex::group(vec![
                IRegex::named("DISPLAY3", "([^#%s{}%g]+)"),
                IRegex::optional(IRegex::group(vec![
                    IRegex::space_one_or_more(),
                    IRegex::named("STEREOTYPE3", r"(\<\<.+\>\>)"),
                ])),
                IRegex::space_zero_or_more(),
                IRegex::leaf("as"),
                IRegex::space_one_or_more(),
                IRegex::named("CODE3", "([^#%s{}%g]+)"),
            ]),
            IRegex::named("CODE8", "([%g][^%g]+[%g])"),
            IRegex::named("CODE9", "([^#%s{}%g]*)"),
        ]),
        IRegex::space_zero_or_more(),
        IRegex::named("TAGS1", &format!("{}?", tags_pattern())),
        crate::stereo::optional_pattern("STEREOTYPE"),
        IRegex::named("TAGS2", &format!("{}?", tags_pattern())),
        IRegex::space_zero_or_more(),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        back_color().regex(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\{"),
        IRegex::end(),
    ])
}

fn execute_package_with_usymbol(
    d: &mut ClassDiagram,
    loc: Option<crate::text::LineLocation>,
    arg: &RegexResult,
) -> CommandExecutionResult {
    let code_arg = unquote(arg.get_lazzy("CODE", 0).unwrap_or(""));
    let code_or_unique = if code_arg.is_empty() {
        d.cuca.unique_sequence("##")
    } else {
        code_arg.clone()
    };
    let id_short = d.cuca.clean_id(Some(&code_or_unique)).unwrap_or_default();
    let quark = match d.cuca.quark_in_context(false, &id_short) {
        Ok(q) => q,
        Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
    };
    let display = match &code_arg {
        c if c.is_empty() => None,
        _ => Some(unquote(
            arg.get_lazzy("DISPLAY", 0).unwrap_or_else(|| quark.name()),
        )),
    };
    let symbol = arg.get("SYMBOL", 0).unwrap_or("package");
    let skin = &d.cuca.base.skin_param;
    let usymbol = crate::decoration::usymbol::from_string(
        symbol,
        skin.actor_style(),
        skin.component_style(),
        skin.package_style(),
    );
    let status = d.cuca.goto_group(
        loc,
        quark,
        Display::with_newlines(display.as_deref()),
        GroupType::Package,
        usymbol,
    );
    if !status.is_ok() {
        return status;
    }
    let p = d.cuca.current_group();
    if let Some(st) = arg.get_lazzy("STEREOTYPE", 0) {
        p.set_stereotype(Some(Stereotype::build(st)));
    }
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            p.add_url(url);
        }
    }
    add_tags(&p, arg.get_lazzy("TAGS", 0));
    let Some(colors) = back_color().colors(arg) else {
        return CommandExecutionResult::bad_color();
    };
    p.set_colors(colors);
    CommandExecutionResult::ok()
}

pub(crate) fn package_with_usymbol() -> Cmd {
    Box::new(SingleLineCommand::new(
        package_with_usymbol_pattern(),
        |d: &mut ClassDiagram, loc, arg, _| execute_package_with_usymbol(d, loc, arg),
    ))
}

// together, end of group

pub(crate) fn together() -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("together"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{"),
            IRegex::end(),
        ]),
        |d: &mut ClassDiagram, _, _, _| d.cuca.goto_together(),
    ))
}

fn end_group() -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\}"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ]),
        |d: &mut ClassDiagram, _, _, _| {
            if d.cuca.end_group() {
                CommandExecutionResult::ok()
            } else {
                CommandExecutionResult::error("No package or namespace defined")
            }
        },
    ))
}

// links

fn link_element() -> Cmd {
    let code = r"([%pLN_.]+|[%g][^%g]+[%g]|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|(?!\[\*\])\[[^\[\]]+\]|\((?!\*\))[^)]+\)/?)";
    let group = |name: &str| IRegex::named(name, code);
    let rgb = crate::classdiagram::commands::line_style();
    Box::new(SingleLineCommand::new(
        IRegex::concat(vec![
            IRegex::start(),
            group("ENT1"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("FIRST_LABEL", "[%g]([^%g]+)[%g]")),
            IRegex::space_zero_or_more(),
            IRegex::named(
                "ARROW_HEAD1",
                &crate::decoration::link::LinkDecor::regex_decors1(),
            ),
            IRegex::named("ARROW_BODY1", "([-=.\u{7e}]+)"),
            IRegex::named("ARROW_STYLE1", &format!(r"(?:\[({rgb})\])?")),
            IRegex::optional(IRegex::named(
                "ARROW_DIRECTION",
                r"(left|right|up|down|le?|ri?|up?|do?)(?=[-=.~0()\[\]])",
            )),
            IRegex::optional(IRegex::named("INSIDE", r"(0|\(0\)|\(0|0\))(?=[-=.~])")),
            IRegex::named("ARROW_STYLE2", &format!(r"(?:\[({rgb})\])?")),
            IRegex::named("ARROW_BODY2", "([-=.\u{7e}]*)"),
            IRegex::named(
                "ARROW_HEAD2",
                &crate::decoration::link::LinkDecor::regex_decors2(),
            ),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("SECOND_LABEL", "[%g]([^%g]+)[%g]")),
            IRegex::space_zero_or_more(),
            group("ENT2"),
            IRegex::space_zero_or_more(),
            ColorParser::simple_color(ColorType::Line).regex(),
            crate::stereo::optional_pattern("STEREOTYPE"),
            IRegex::named("LABEL_LINK", r"(?::[%s]*(.+))?"),
            IRegex::end(),
        ]),
        |d: &mut ClassDiagram, loc, arg, _| execute_link_description(d, loc, arg),
    ))
}

// commands

/// All commands of a description diagram (`DescriptionDiagramFactory.initCommandsList`).
pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.push(crate::classdiagram::commands_misc::rank_dir::<ClassDiagram>());
    cmds.push(crate::classdiagram::commands_misc::remove_restore::<
        ClassDiagram,
    >());
    cmds.push(crate::classdiagram::commands_misc::url_command::<
        ClassDiagram,
    >());
    cmds.push(crate::classdiagram::commands::hide_show2::<ClassDiagram>());
    cmds.push(package_with_usymbol());
    cmds.push(together());
    cmds.push(end_group());
    cmds.extend(crate::classdiagram::commands_note::floating_commands::<
        ClassDiagram,
    >());
    cmds.push(create_element_full());
    cmds.push(create_element_multilines(0));
    cmds.push(create_element_multilines(1));
    cmds.extend(
        crate::classdiagram::commands_note::on_entity_commands_for::<ClassDiagram>(
            CODE_NOTE,
            ParserPass::One,
        ),
    );
    cmds.push(link_element());
    cmds.extend(crate::classdiagram::commands_note::multi_commands::<
        ClassDiagram,
    >());
    cmds
}

const CODE_NOTE: &str = r"[%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|\[[^\]]*\][^\]]*\]|\((?!\*\))[^)]+\)|:[^:]+:|[%g][^%g]+[%g]";

#[allow(dead_code)]
fn _unused(_: HColor) {}
