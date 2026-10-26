//! Smaller commands of class diagrams: members, hide/show, remove/restore, separators.

use super::ClassDiagram;
use crate::abel::LeafType;
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::cucadiagram::CucaHolder;
use crate::klimt::creole::display::Display;
use crate::regex::IRegex;
use crate::skin::visibility::VisibilityModifier;
use crate::skin::Rankdir;
use crate::stereo::Stereotype;
use crate::url::{UrlBuilder, UrlMode};

type Cmd = Box<dyn Command<ClassDiagram>>;

fn single<D: 'static>(
    parts: Vec<IRegex>,
    f: impl Fn(
            &mut D,
            Option<crate::text::LineLocation>,
            &crate::regex::RegexResult,
        ) -> CommandExecutionResult
        + 'static,
) -> Box<dyn Command<D>> {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut D, loc, arg, _| f(d, loc, arg),
    ))
}

fn add_method() -> Cmd {
    single::<ClassDiagram>(
        vec![
            IRegex::start(),
            IRegex::named("NAME", "([%pLN_.]+|[%g][^%g]+[%g])"),
            IRegex::space_one_or_more(),
            IRegex::leaf(":"),
            IRegex::space_one_or_more(),
            IRegex::named("DATA", "(.*)"),
            IRegex::end(),
        ],
        |d, loc, arg| {
            let id = d.cuca.clean_id(arg.get("NAME", 0)).unwrap_or_default();
            let quark = match d.cuca.quark_in_context(true, &id) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let entity = match quark.data() {
                Some(e) => e,
                None => d.cuca.really_create_leaf(
                    loc,
                    quark.clone(),
                    Display::with_newlines(Some(quark.name())),
                    LeafType::Class,
                ),
            };
            let field = arg.get("DATA", 0).unwrap_or("");
            if !field.is_empty() && VisibilityModifier::is_visibility_character(field) {
                d.cuca.set_visibility_modifier_present(true);
            }
            entity.add_field_or_method(field);
            CommandExecutionResult::ok()
        },
    )
}

pub fn remove_restore<D: CucaHolder + 'static>() -> Box<dyn Command<D>> {
    single::<D>(
        vec![
            IRegex::start(),
            IRegex::named("COMMAND", "(remove|restore)"),
            IRegex::space_one_or_more(),
            IRegex::named("WHAT", "(.+)"),
            IRegex::end(),
        ],
        |d, _, arg| {
            let show = arg
                .get("COMMAND", 0)
                .unwrap_or("")
                .eq_ignore_ascii_case("restore");
            let what = arg.get("WHAT", 0).unwrap_or("").trim();
            d.cuca_mut().remove_or_restore(what, show);
            CommandExecutionResult::ok()
        },
    )
}

fn allow_mixing() -> Cmd {
    single::<ClassDiagram>(
        vec![
            IRegex::start(),
            IRegex::leaf("allow"),
            IRegex::leaf("_?"),
            IRegex::leaf("mixing"),
            IRegex::end(),
        ],
        |d, _, _| {
            d.set_allow_mixing(true);
            CommandExecutionResult::ok()
        },
    )
}

fn layout_new_line() -> Cmd {
    single::<ClassDiagram>(
        vec![
            IRegex::start(),
            IRegex::leaf("layout_new_line"),
            IRegex::end(),
        ],
        |d, _, _| {
            d.layout_new_line();
            CommandExecutionResult::ok()
        },
    )
}

fn stereotype_command() -> Cmd {
    single::<ClassDiagram>(
        vec![
            IRegex::start(),
            IRegex::named("NAME", "([%pLN_.]+|[%g][^%g]+[%g])"),
            IRegex::space_zero_or_more(),
            IRegex::named("STEREO", r"(\<\<.+?\>\>)"),
            IRegex::end(),
        ],
        |d, _, arg| {
            let name = d.cuca.clean_id(arg.get("NAME", 0)).unwrap_or_default();
            let quark = match d.cuca.quark_in_context(true, &name) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let Some(entity) = quark.data() else {
                return CommandExecutionResult::error(format!("No such class {}", quark.name()));
            };
            let radius = d.cuca.base.skin_param.circled_character_radius();
            entity.set_stereotype(Some(Stereotype::build_complex(
                arg.get("STEREO", 0).unwrap_or(""),
                radius,
            )));
            CommandExecutionResult::ok()
        },
    )
}

pub fn url_command<D: CucaHolder + 'static>() -> Box<dyn Command<D>> {
    single::<D>(
        vec![
            IRegex::start(),
            IRegex::leaf("url"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::leaf("of|for")),
            IRegex::space_one_or_more(),
            IRegex::named("CODE", "([%pLN_.]+|[%g][^%g]+[%g])"),
            IRegex::space_one_or_more(),
            IRegex::optional(IRegex::leaf("is")),
            IRegex::space_zero_or_more(),
            crate::url::mandatory(),
            IRegex::end(),
        ],
        |d, _, arg| {
            let code = d.cuca().clean_id(arg.get("CODE", 0)).unwrap_or_default();
            let quark = match d.cuca().quark_in_context(true, &code) {
                Ok(q) => q,
                Err((m, s)) => return CommandExecutionResult::error_with_score(m, s),
            };
            let Some(entity) = quark.data() else {
                return CommandExecutionResult::error(format!("{} does not exist", quark.name()));
            };
            let top = d.cuca().base.skin_param.get_value("topurl");
            if let Some(u) = arg.get(crate::url::URL_KEY, 0) {
                if let Some(url) = UrlBuilder::new(top, UrlMode::Strict).url(u) {
                    entity.add_url(url);
                }
            }
            CommandExecutionResult::ok()
        },
    )
}

pub fn rank_dir<D: CucaHolder + 'static>() -> Box<dyn Command<D>> {
    single::<D>(
        vec![
            IRegex::start(),
            IRegex::named("DIRECTION", "(left[%s]to[%s]right|top[%s]to[%s]bottom)"),
            IRegex::space_one_or_more(),
            IRegex::leaf("direction"),
            IRegex::end(),
        ],
        |d, _, arg| {
            let s = arg.get("DIRECTION", 0).unwrap_or("").to_lowercase();
            d.cuca_mut().base.skin_param.rankdir = if s.starts_with("left") {
                Rankdir::LeftToRight
            } else {
                Rankdir::TopToBottom
            };
            CommandExecutionResult::ok()
        },
    )
}

pub fn commands() -> Vec<Cmd> {
    vec![
        rank_dir::<ClassDiagram>(),
        allow_mixing(),
        layout_new_line(),
        remove_restore::<ClassDiagram>(),
        url_command::<ClassDiagram>(),
        stereotype_command(),
        add_method(),
    ]
}
