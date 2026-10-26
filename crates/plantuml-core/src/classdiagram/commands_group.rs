//! Packages and namespaces (`CommandPackage`, `CommandPackageEmpty`, `CommandNamespace*`, `CommandEndPackage`).

use super::ClassDiagram;
use crate::abel::GroupType;
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;
use crate::string_utils::eventually_remove_starting_and_ending_double_quote as unquote;
use crate::url::{UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ClassDiagram>>;

const NAMESPACE_REGEX: &str = r"([%pLN_][-%pLN_.:\\/]*)";

fn apply_url(d: &ClassDiagram, arg: &RegexResult, entity: &crate::abel::Entity) {
    if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
        let top = d.cuca.base.skin_param.get_value("topurl");
        if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
            entity.add_url(url);
        }
    }
}

fn package_empty() -> Cmd {
    let regex = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("package"),
        IRegex::space_one_or_more(),
        IRegex::named("DISPLAY", "([%g][^%g]+[%g]|[^#%s{}]*)"),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
            IRegex::named("CODE", "([%pLN_.]+)"),
        ])),
        IRegex::space_zero_or_more(),
        IRegex::named("COLOR", r"(#[0-9a-fA-F]{6}|#?\w+)?"),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\{"),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\}"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(
        regex,
        |d: &mut ClassDiagram, loc, arg, _| {
            let display_raw = unquote(arg.get("DISPLAY", 0).unwrap_or(""));
            let (id_short, display) = match arg.get("CODE", 0) {
                None => {
                    if display_raw.is_empty() {
                        (d.cuca.unique_sequence("##"), None)
                    } else {
                        (display_raw.clone(), Some(display_raw))
                    }
                }
                Some(code) => (code.to_string(), Some(display_raw)),
            };
            let id = d.cuca.clean_id(Some(&id_short)).unwrap_or_default();
            let quark = match d.cuca.quark_in_context(false, &id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let status = d.cuca.goto_group(
                loc,
                quark,
                Display::with_newlines(display.as_deref()),
                GroupType::Package,
                None,
            );
            if !status.is_ok() {
                return status;
            }
            let p = d.cuca.current_group();
            if let Some(c) = arg.get("COLOR", 0) {
                if let Some(color) = HColor::parse(c) {
                    let colors = p.colors().add(ColorType::Back, Some(color));
                    p.set_colors(colors);
                }
            }
            d.cuca.end_group();
            CommandExecutionResult::ok()
        },
    ))
}

fn namespace_regex(display_first: bool, empty: bool) -> IRegex {
    let mut parts = vec![
        IRegex::start(),
        IRegex::leaf("namespace"),
        IRegex::space_one_or_more(),
    ];
    if display_first {
        parts.extend([
            IRegex::leaf("[%g]"),
            IRegex::named("DISPLAY", "([^%g]+)"),
            IRegex::leaf("[%g]"),
            IRegex::space_one_or_more(),
            IRegex::leaf("as"),
            IRegex::space_one_or_more(),
        ]);
    }
    parts.extend([
        IRegex::named("NAME", NAMESPACE_REGEX),
        crate::stereo::optional_pattern("STEREOTYPE"),
        crate::url::optional(),
        IRegex::space_zero_or_more(),
        crate::klimt::color::parser::exp1(),
        IRegex::space_zero_or_more(),
        IRegex::leaf(r"\{"),
    ]);
    if empty {
        parts.extend([IRegex::space_zero_or_more(), IRegex::leaf(r"\}")]);
    }
    parts.push(IRegex::end());
    IRegex::concat(parts)
}

fn namespace_apply(
    d: &mut ClassDiagram,
    arg: &RegexResult,
    p: &crate::abel::Entity,
    usymbol_set: bool,
) {
    if let Some(s) = arg.get("STEREOTYPE", 0) {
        if !usymbol_set {
            p.set_stereotype(Some(Stereotype::build(s)));
        }
    }
    apply_url(d, arg, p);
    if let Some(c) = arg.get("COLOR", 0) {
        if let Some(color) = HColor::parse(c) {
            let colors = p.colors().add(ColorType::Back, Some(color));
            p.set_colors(colors);
        }
    }
}

fn namespace(display_first: bool, empty: bool) -> Cmd {
    Box::new(
        SingleLineCommand::new(
            namespace_regex(display_first, empty),
            move |d: &mut ClassDiagram, loc, arg, _| {
                let id_short = arg.get("NAME", 0).unwrap_or("");
                let id = d.cuca.clean_id(Some(id_short)).unwrap_or_default();
                let quark = match d.cuca.quark_in_context(false, &id) {
                    Ok(q) => q,
                    Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
                };
                if empty && quark.data().is_some() {
                    return CommandExecutionResult::error(format!(
                        "Already exists {}",
                        quark.name()
                    ));
                }
                let (display, usymbol) = if display_first {
                    (arg.get_lazzy("DISPLAY", 0).unwrap_or("").to_string(), None)
                } else if empty {
                    (quark.qualified_name().to_string(), None)
                } else {
                    let st = arg.get("STEREOTYPE", 0);
                    (quark.name().to_string(), st.and_then(|s| d.usymbol_for(s)))
                };
                let status = d.cuca.goto_group(
                    loc,
                    quark,
                    Display::with_newlines(Some(&display)),
                    GroupType::Package,
                    usymbol.clone(),
                );
                if !status.is_ok() {
                    return status;
                }
                let p = d.cuca.current_group();
                namespace_apply(d, arg, &p, usymbol.is_some());
                if empty {
                    d.cuca.end_group();
                }
                CommandExecutionResult::ok()
            },
        )
        .with_final_bracket(),
    )
}

pub fn commands() -> Vec<Cmd> {
    vec![package_empty(), namespace(false, true)]
}
