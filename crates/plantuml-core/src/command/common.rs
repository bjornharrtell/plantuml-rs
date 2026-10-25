//! Commands shared by all diagram types (`CommonCommands`).

use super::{
    BracketCommand, Command, CommandExecutionResult, SimpleMultiLineCommand, SingleLineCommand,
};
use crate::diagram::{DiagramBase, Scale};
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::regex::IRegex;
use crate::text::{BlocLines, LineLocation};
use regex::Regex;
use std::sync::LazyLock;

/// A diagram that has titles, header, footer and skin parameters.
pub trait HasBase {
    fn base_mut(&mut self) -> &mut DiagramBase;
}

type Cmd<D> = Box<dyn Command<D>>;

fn display_of(arg: Option<&str>) -> Display {
    Display::with_newlines(arg)
}

fn label_regex(keyword: &str, group1: &str, group2: &str) -> IRegex {
    IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf(keyword),
        IRegex::leaf("(?:[%s]*:[%s]*|[%s]+)"),
        IRegex::or(vec![
            IRegex::named(group1, "[%g](.*)[%g]"),
            IRegex::named(group2, "(.*[%pLN_.].*)"),
        ]),
        IRegex::end(),
    ])
}

fn titled_single<D: HasBase + 'static>(
    keyword: &str,
    g1: &'static str,
    g2: &'static str,
    prefix: &'static str,
    apply: impl Fn(&mut DiagramBase, Option<LineLocation>, Display) + 'static,
) -> Cmd<D> {
    Box::new(SingleLineCommand::new(
        label_regex(keyword, g1, g2),
        move |d: &mut D, loc, arg, _| {
            let text = arg.get_lazzy(prefix, 0);
            apply(d.base_mut(), loc, display_of(text));
            CommandExecutionResult::ok()
        },
    ))
}

fn multi_display(lines: &BlocLines, trim_first: bool) -> Display {
    let body = lines.sub_extract(1, 1);
    let body = if trim_first {
        body
    } else {
        body.remove_empty_columns()
    };
    Display::create(&body.as_strings())
}

fn position_and_label_single<D: HasBase + 'static>(
    keyword: &'static str,
    default_align: HorizontalAlignment,
    is_header: bool,
) -> Cmd<D> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::optional(IRegex::named("POSITION", "(left|right|center)")),
        IRegex::space_zero_or_more(),
        IRegex::leaf(keyword),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::space_zero_or_more(),
                IRegex::leaf(":"),
                IRegex::space_zero_or_more(),
            ]),
            IRegex::space_one_or_more(),
        ]),
        IRegex::or(vec![
            IRegex::named("LABEL1", "[%g](.*)[%g]"),
            IRegex::named("LABEL2", "(.*[%pLN_.].*)"),
        ]),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, move |d: &mut D, loc, arg, _| {
        let align = arg.get("POSITION", 0);
        let base = d.base_mut();
        let ha = header_alignment(base, align, default_align, is_header);
        let s = display_of(arg.get_lazzy("LABEL", 0));
        if is_header {
            base.update_header(loc, s, ha);
        } else {
            base.update_footer(loc, s, ha);
        }
        CommandExecutionResult::ok()
    }))
}

/// Alignment given or taken from the `header`/`footer` style.
fn header_alignment(
    base: &DiagramBase,
    align: Option<&str>,
    default: HorizontalAlignment,
    is_header: bool,
) -> HorizontalAlignment {
    match align {
        Some(a) => HorizontalAlignment::from_string(a).unwrap_or(default),
        None => {
            let q = if is_header {
                crate::style::queries::DOCUMENT_HEADER.clone()
            } else {
                crate::style::queries::DOCUMENT_FOOTER.clone()
            };
            base.skin_param
                .style_builder()
                .merged_style(&q)
                .map_or(default, |s| s.horizontal_alignment())
        }
    }
}

fn header_footer_multi<D: HasBase + 'static>(
    keyword: &'static str,
    default_align: HorizontalAlignment,
    is_header: bool,
) -> Cmd<D> {
    let start = format!("^(?:(left|right|center)?[%s]*){keyword}$");
    let end = format!("^end[%s]?{keyword}$");
    let start_re = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf(&format!("(?:(left|right|center)?[%s]*){keyword}")),
        IRegex::end(),
    ]);
    let start_for_exec = start_re.clone();
    let cmd = SimpleMultiLineCommand::new(&start, &end, move |d: &mut D, lines, _| {
        let lines = lines.trim();
        let m = start_for_exec
            .matcher(lines.first().expect("first").trimmed().string())
            .expect("starts");
        let align = m.get_lazzy("", 0).map(str::to_string);
        let _ = align;
        let first = lines.first().expect("first").trimmed();
        let align = LEADING_ALIGN
            .captures(first.string())
            .map(|c| c[1].to_string());
        let strings = multi_display(&lines, true);
        if strings.size() == 0 {
            return CommandExecutionResult::error(if is_header {
                "Empty header"
            } else {
                "Empty footer"
            });
        }
        let base = d.base_mut();
        let ha = header_alignment(base, align.as_deref(), default_align, is_header);
        let loc = lines.location();
        if is_header {
            base.update_header(loc, strings, ha);
        } else {
            base.update_footer(loc, strings, ha);
        }
        CommandExecutionResult::ok()
    });
    Box::new(cmd)
}

static LEADING_ALIGN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(left|right|center)").expect("valid"));

fn scale_commands<D: HasBase + 'static>() -> Vec<Cmd<D>> {
    let num = |name: &str| IRegex::named(name, "([0-9.]+)");
    let mut v: Vec<Cmd<D>> = Vec::new();
    let scale = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        num("SCALE"),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_zero_or_more(),
            IRegex::leaf("/"),
            IRegex::space_zero_or_more(),
            num("DIV"),
        ])),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        scale,
        |d: &mut D, _, arg, _| {
            let mut scale: f64 = arg
                .get("SCALE", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            if scale == 0.0 {
                return CommandExecutionResult::error("Scale cannot be zero");
            }
            if let Some(div) = arg.get("DIV", 0) {
                let div: f64 = div.parse().unwrap_or(0.0);
                if div == 0.0 {
                    return CommandExecutionResult::error("Scale cannot be zero");
                }
                scale /= div;
            }
            d.base_mut().scale = Some(Scale::Factor(scale));
            CommandExecutionResult::ok()
        },
    )));
    let wh = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        num("WIDTH"),
        IRegex::space_zero_or_more(),
        IRegex::leaf("[*x]"),
        IRegex::space_zero_or_more(),
        num("HEIGHT"),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        wh,
        |d: &mut D, _, arg, _| {
            let w = arg
                .get("WIDTH", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            let h = arg
                .get("HEIGHT", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            d.base_mut().scale = Some(Scale::WidthAndHeight(w, h));
            CommandExecutionResult::ok()
        },
    )));
    let one = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        num("VALUE"),
        IRegex::space_one_or_more(),
        IRegex::named("WIDTH", "(width|height)"),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        one,
        |d: &mut D, _, arg, _| {
            let size = arg
                .get("VALUE", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            let width = arg
                .get("WIDTH", 0)
                .is_some_and(|w| w.eq_ignore_ascii_case("width"));
            d.base_mut().scale = Some(if width {
                Scale::Width(size)
            } else {
                Scale::Height(size)
            });
            CommandExecutionResult::ok()
        },
    )));
    let max_w = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        IRegex::leaf("max"),
        IRegex::space_one_or_more(),
        num("WIDTH"),
        IRegex::space_one_or_more(),
        IRegex::leaf("width"),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        max_w,
        |d: &mut D, _, arg, _| {
            let w = arg
                .get("WIDTH", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            d.base_mut().scale = Some(Scale::MaxWidth(w));
            CommandExecutionResult::ok()
        },
    )));
    let max_h = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        IRegex::leaf("max"),
        IRegex::space_one_or_more(),
        num("HEIGHT"),
        IRegex::space_one_or_more(),
        IRegex::leaf("height"),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        max_h,
        |d: &mut D, _, arg, _| {
            let h = arg
                .get("HEIGHT", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            d.base_mut().scale = Some(Scale::MaxHeight(h));
            CommandExecutionResult::ok()
        },
    )));
    let max_wh = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("scale"),
        IRegex::space_one_or_more(),
        IRegex::leaf("max"),
        IRegex::space_one_or_more(),
        num("WIDTH"),
        IRegex::space_zero_or_more(),
        IRegex::leaf("[*x]"),
        IRegex::space_zero_or_more(),
        num("HEIGHT"),
        IRegex::end(),
    ]);
    v.push(Box::new(SingleLineCommand::new(
        max_wh,
        |d: &mut D, _, arg, _| {
            let w = arg
                .get("WIDTH", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            let h = arg
                .get("HEIGHT", 0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            d.base_mut().scale = Some(Scale::MaxWidthAndHeight(w, h));
            CommandExecutionResult::ok()
        },
    )));
    v
}

static SKIN_LINE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    crate::regex::compile(r"^([\w.]*(?:\<\<.*\>\>)?[\w.]*)[%s]+(?:(\{)|(.*))$|^\}?$")
});
static SKIN_COMMENT: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| crate::regex::compile(r"^[%s]*([%q].*|/[%q].*[%q]/[%s]*)$"));

fn full_match(re: &fancy_regex::Regex, s: &str) -> bool {
    re.find(s)
        .ok()
        .flatten()
        .is_some_and(|m| m.start() == 0 && m.end() == s.len())
}

/// Applies the lines of a `skinparam xxx {` block (`SkinLoader.execute`).
fn load_skin_block(base: &mut DiagramBase, lines: &BlocLines, group1: Option<&str>) {
    let mut context: Vec<String> = Vec::new();
    if let Some(g) = group1 {
        context.push(g.to_string());
    }
    let body = lines.sub_extract(1, 1).trim().remove_empty_lines();
    for s in body.iter() {
        let s = s.string();
        if s == "}" {
            context.pop();
            continue;
        }
        let Some(c) = SKIN_LINE.captures(s).ok().flatten() else {
            continue;
        };
        if c.get(2).is_some() {
            context.push(c.get(1).map_or(String::new(), |m| m.as_str().to_string()));
        } else if let Some(v) = c.get(3) {
            let key = format!(
                "{}{}",
                context.concat(),
                c.get(1).map_or("", |m| m.as_str())
            );
            base.set_param(&key, v.as_str());
        }
    }
}

/// Titles, scale, skinparam, pragma, inline styles (`addCommonCommands1`/`2`, partial).
pub fn common_commands<D: HasBase + 'static>() -> Vec<Cmd<D>> {
    let mut cmds: Vec<Cmd<D>> = Vec::new();
    cmds.push(titled_single(
        "title",
        "TITLE1",
        "TITLE2",
        "TITLE",
        |b, loc, s| {
            b.set_title(DisplayPositioned {
                location: loc,
                ..DisplayPositioned::single(
                    s,
                    Some(HorizontalAlignment::Center),
                    Some(VerticalAlignment::Top),
                )
            });
        },
    ));
    cmds.push(Box::new(SimpleMultiLineCommand::new(
        "^title$",
        "^end[%s]?title$",
        |d: &mut D, lines, _| {
            let strings = multi_display(lines, false);
            if strings.size() == 0 {
                return CommandExecutionResult::error("No title defined");
            }
            d.base_mut().set_title(DisplayPositioned {
                location: lines.location(),
                ..DisplayPositioned::single(
                    strings.replace_backslash_t(),
                    Some(HorizontalAlignment::Center),
                    Some(VerticalAlignment::Top),
                )
            });
            CommandExecutionResult::ok()
        },
    )));
    cmds.push(titled_single(
        "caption",
        "DISPLAY1",
        "DISPLAY2",
        "DISPLAY",
        |b, loc, s| {
            b.caption = DisplayPositioned {
                location: loc,
                ..DisplayPositioned::single(
                    s,
                    Some(HorizontalAlignment::Center),
                    Some(VerticalAlignment::Bottom),
                )
            };
        },
    ));
    cmds.push(Box::new(SimpleMultiLineCommand::new(
        "^caption$",
        "^end[%s]?caption$",
        |d: &mut D, lines, _| {
            let strings = multi_display(lines, false);
            if strings.size() == 0 {
                return CommandExecutionResult::error("No caption defined");
            }
            d.base_mut().caption = DisplayPositioned {
                location: lines.location(),
                ..DisplayPositioned::single(
                    strings.replace_backslash_t(),
                    Some(HorizontalAlignment::Center),
                    Some(VerticalAlignment::Bottom),
                )
            };
            CommandExecutionResult::ok()
        },
    )));
    let legend_start = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("legend"),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("VALIGN", "(top|bottom)"),
        ])),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("ALIGN", "(left|right|center)"),
        ])),
        IRegex::end(),
    ]);
    let legend_start2 = legend_start.clone();
    cmds.push(Box::new(super::MultiLineCommand::new(
        legend_start,
        crate::text::MultilinesStrategy::RemoveStartingQuote,
        super::Trim::Both,
        "^end[%s]?legend$",
        move |d: &mut D, lines: &BlocLines, _| {
            let lines = lines.trim_smart(1);
            let line0 = legend_start2
                .matcher(lines.first().expect("first").trimmed().string())
                .expect("starts");
            let align = line0.get("ALIGN", 0);
            let valign = line0.get("VALIGN", 0);
            let strings = multi_display(&lines, false);
            if strings.size() == 0 {
                return CommandExecutionResult::error("No legend defined");
            }
            let va = valign.map(VerticalAlignment::from_string);
            let ha = align
                .and_then(HorizontalAlignment::from_string)
                .unwrap_or(HorizontalAlignment::Center);
            d.base_mut().legend = DisplayPositioned {
                location: lines.location(),
                ..DisplayPositioned::single(strings.replace_backslash_t(), Some(ha), va)
            };
            CommandExecutionResult::ok()
        },
    )));
    cmds.push(titled_single(
        "legend",
        "LEGEND1",
        "LEGEND2",
        "LEGEND",
        |b, _, s| {
            b.legend = DisplayPositioned::single(
                s,
                Some(HorizontalAlignment::Center),
                Some(VerticalAlignment::Bottom),
            );
        },
    ));
    cmds.push(position_and_label_single(
        "footer",
        HorizontalAlignment::Center,
        false,
    ));
    cmds.push(header_footer_multi(
        "footer",
        HorizontalAlignment::Center,
        false,
    ));
    cmds.push(position_and_label_single(
        "header",
        HorizontalAlignment::Right,
        true,
    ));
    cmds.push(header_footer_multi(
        "header",
        HorizontalAlignment::Right,
        true,
    ));
    cmds.push(nope());
    cmds.extend(simple_commands());
    cmds.push(pragma());
    cmds.push(skinparam_single());
    cmds.push(skinparam_block());
    cmds.push(mainframe());
    cmds.extend(scale_commands());
    cmds.extend(style_commands());
    cmds
}

fn nope<D: 'static>() -> Cmd<D> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, |_: &mut D, _, _, _| {
        CommandExecutionResult::ok()
    }))
}

fn mainframe<D: HasBase + 'static>() -> Cmd<D> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("mainframe"),
        IRegex::or(vec![
            IRegex::group(vec![
                IRegex::space_zero_or_more(),
                IRegex::leaf(":"),
                IRegex::space_zero_or_more(),
            ]),
            IRegex::space_one_or_more(),
        ]),
        IRegex::named("LABEL", "(.*[%pLN_.].*)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, |d: &mut D, loc, arg, _| {
        let label = display_of(arg.get("LABEL", 0));
        d.base_mut().main_frame = DisplayPositioned {
            location: loc,
            ..DisplayPositioned::single(label, None, None)
        };
        CommandExecutionResult::ok()
    }))
}

fn pragma<D: HasBase + 'static>() -> Cmd<D> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("!pragma"),
        IRegex::space_one_or_more(),
        IRegex::named("NAME", "([A-Za-z_][A-Za-z_0-9]*)"),
        IRegex::optional(IRegex::group(vec![
            IRegex::space_one_or_more(),
            IRegex::named("VALUE", "(.*)"),
        ])),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, |d: &mut D, _, arg, _| {
        let name = arg.get("NAME", 0).unwrap_or("").to_lowercase();
        let value = arg.get("VALUE", 0).unwrap_or("").to_string();
        d.base_mut().pragma.insert(name, value);
        CommandExecutionResult::ok()
    }))
}

fn skinparam_single<D: HasBase + 'static>() -> Cmd<D> {
    let r = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", "(skinparam|skinparamlocked)"),
        IRegex::space_one_or_more(),
        IRegex::named("NAME", r"([\w.]*(?:\<\<[^<>]*\>\>)?[\w.]*)"),
        IRegex::space_one_or_more(),
        IRegex::named("VALUE", "([^{}]*)"),
        IRegex::end(),
    ]);
    Box::new(SingleLineCommand::new(r, |d: &mut D, _, arg, _| {
        let name = arg.get("NAME", 0).unwrap_or("");
        let value = arg.get("VALUE", 0).unwrap_or("");
        if name.eq_ignore_ascii_case("handwritten") {
            d.base_mut().add_warning(vec![format!(
                "'skinparam handwritten' is deprecated: write '!option handwritten {value}' instead"
            )]);
        }
        if name.eq_ignore_ascii_case("ParticipantPadding") {
            d.base_mut().add_warning(vec![
                "'skinparam ParticipantPadding' is ignored: use a style instead".to_string(),
                "<style>".to_string(),
                "participant {".to_string(),
                format!("  Padding {value}"),
                "}".to_string(),
                "</style>".to_string(),
            ]);
        }
        d.base_mut().set_param(name, value);
        CommandExecutionResult::ok()
    }))
}

fn skinparam_block<D: HasBase + 'static>() -> Cmd<D> {
    let starting = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf(r"skinparam[%s]*(?:[%s]+([\w.]*(?:\<\<.*\>\>)?[\w.]*))?[%s]*\{"),
        IRegex::end(),
    ]);
    let starting2 = starting.clone();
    Box::new(BracketCommand::new(
        starting,
        |line, _level| {
            let line = line.trim();
            full_match(&SKIN_COMMENT, line) || full_match(&SKIN_LINE, line)
        },
        move |d: &mut D, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let Some(m) = starting2.matcher(first.string()) else {
                return CommandExecutionResult::error("Bad syntax for skinparam");
            };
            let group1 = m.get_lazzy("", 0).map(str::to_string);
            let group1 = group1.or_else(|| {
                SKIN_GROUP
                    .captures(first.string())
                    .and_then(|c| c.get(1))
                    .map(|g| g.as_str().to_string())
            });
            load_skin_block(d.base_mut(), lines, group1.as_deref());
            CommandExecutionResult::ok()
        },
    ))
}

static SKIN_GROUP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^skinparam\s+([\w.]*(?:<<.*>>)?[\w.]*)\s*\{$").expect("valid")
});

fn style_commands<D: HasBase + 'static>() -> Vec<Cmd<D>> {
    let single = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf(r"\<style\>"),
        IRegex::named("STYLE", r"([-#.\w%s;:{}]+)"),
        IRegex::leaf(r"\</style\>"),
        IRegex::end(),
    ]);
    let multi_start = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf(r"\<style\>"),
        IRegex::end(),
    ]);
    vec![
        Box::new(SingleLineCommand::new(single, |d: &mut D, _, arg, _| {
            apply_style_text(d.base_mut(), arg.get("STYLE", 0).unwrap_or(""))
        })),
        Box::new(super::MultiLineCommand::new(
            multi_start,
            crate::text::MultilinesStrategy::RemoveStartingQuote,
            super::Trim::Both,
            r"^[%s]*\</?style\>[%s]*$",
            |d: &mut D, lines: &BlocLines, _| {
                apply_style_text(
                    d.base_mut(),
                    &lines.sub_extract(1, 1).as_strings().join("\n"),
                )
            },
        )),
    ]
}

fn apply_style_text(base: &mut DiagramBase, text: &str) -> CommandExecutionResult {
    let mut builder = base.skin_param.style_builder();
    match builder.parse_style_text(text) {
        Ok(styles) => {
            base.skin_param.mute_style(styles);
            CommandExecutionResult::ok()
        }
        Err(e) => CommandExecutionResult::error(format!("Error in style definition: {e}")),
    }
}

/// `skin`, `minwidth`, `page`, `rotate` and `!assume transparent`.
fn simple_commands<D: HasBase + 'static>() -> Vec<Cmd<D>> {
    let skin = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("skin"),
        IRegex::space_one_or_more(),
        IRegex::named("SKIN", r"([\w.]+)"),
        IRegex::end(),
    ]);
    let minwidth = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("minwidth"),
        IRegex::space_one_or_more(),
        IRegex::named("VALUE", r"(\d+)"),
        IRegex::end(),
    ]);
    let page = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("page"),
        IRegex::space_one_or_more(),
        IRegex::named("NB1", r"(\d+)"),
        IRegex::space_zero_or_more(),
        IRegex::leaf("x*"),
        IRegex::space_zero_or_more(),
        IRegex::named("NB2", r"(\d+)"),
        IRegex::end(),
    ]);
    let rotate = IRegex::concat(vec![IRegex::start(), IRegex::leaf("rotate"), IRegex::end()]);
    let assume = IRegex::concat(vec![
        IRegex::start(),
        IRegex::leaf("!assume"),
        IRegex::space_one_or_more(),
        IRegex::leaf("transparent"),
        IRegex::space_one_or_more(),
        IRegex::named("TYPE", "(dark|light)"),
        IRegex::end(),
    ]);
    vec![
        Box::new(SingleLineCommand::new(skin, |d: &mut D, _, arg, _| match d
            .base_mut()
            .skin_param
            .set_default_skin(arg.get("SKIN", 0).unwrap_or(""))
        {
            Ok(()) => CommandExecutionResult::ok(),
            Err(e) => CommandExecutionResult::error(e),
        })),
        Box::new(SingleLineCommand::new(minwidth, |_: &mut D, _, _, _| {
            CommandExecutionResult::ok()
        })),
        Box::new(SingleLineCommand::new(page, |d: &mut D, _, arg, _| {
            let parse = |k| {
                arg.get(k, 0)
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(0)
            };
            if parse("NB1") <= 0 || parse("NB2") <= 0 {
                return CommandExecutionResult::error("Argument must be positive");
            }
            let _ = d;
            CommandExecutionResult::ok()
        })),
        Box::new(SingleLineCommand::new(rotate, |_: &mut D, _, _, _| {
            CommandExecutionResult::ok()
        })),
        Box::new(SingleLineCommand::new(assume, |_: &mut D, _, _, _| {
            CommandExecutionResult::ok()
        })),
    ]
}
