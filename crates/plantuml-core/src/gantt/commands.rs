//! The commands of Gantt diagrams that are not sentences (`gantt/command`).

use super::lang::{Comp, DateType, NaturalGanttCommand, Subject};
use super::model::{CenterBorderColor, TaskCode};
use super::ubrex::{self as ub, Capture, Ub};
use super::{Column, GanttDiagram, PrintScale, WeeklyHeaderStrategy};
use crate::command::{
    Command, CommandControl, CommandExecutionResult, MultiLineCommand, ParserPass,
    SingleLineCommand, Trim,
};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereotype};
use crate::text::{BlocLines, LineLocation, MultilinesStrategy};

type Cmd = Box<dyn Command<GanttDiagram>>;

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut GanttDiagram, &RegexResult) -> CommandExecutionResult + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut GanttDiagram, _: Option<LineLocation>, arg, _| f(d, arg),
    ))
}

const KEY1: &str = r"dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+";
const KEY2: &str = r",dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+";

fn line_style() -> String {
    format!(r"(?:#\w+|{KEY1})(?:,#\w+|{KEY2})*")
}

fn arrow_parts(first: IRegex, second: IRegex) -> Vec<IRegex> {
    vec![
        IRegex::start(),
        first,
        IRegex::space_zero_or_more(),
        IRegex::leaf("(-+)"),
        IRegex::named("ARROW_STYLE", &format!(r"(?:\[({})\])?", line_style())),
        IRegex::leaf("(-*)"),
        IRegex::leaf(r"\>"),
        IRegex::space_zero_or_more(),
        second,
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]
}

/// A command matched by a [`Ub`] pattern on the whole trimmed line (`UBrexSingleLineCommand2`).
struct UbrexLine {
    pattern: Ub,
    exec: fn(&mut GanttDiagram, &Capture) -> CommandExecutionResult,
}

impl Command<GanttDiagram> for UbrexLine {
    fn execute(
        &self,
        diagram: &mut GanttDiagram,
        lines: &BlocLines,
        _pass: ParserPass,
    ) -> CommandExecutionResult {
        let line = lines.at(0).trimmed();
        let m = ub::match_str(&self.pattern, line.string());
        if !m.exact_match() {
            return CommandExecutionResult::error(format!("Cannot parse line {}", line.string()));
        }
        (self.exec)(diagram, m.capture())
    }

    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        if lines.size() != 1 {
            return CommandControl::NotOk;
        }
        let line = lines.at(0).trimmed();
        if ub::match_str(&self.pattern, line.string()).exact_match() {
            CommandControl::Ok
        } else {
            CommandControl::NotOk
        }
    }
}

fn arrow_style(arg: &RegexResult) -> Option<String> {
    arg.get("ARROW_STYLE", 0).map(str::to_string)
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    for s in [
        Subject::Gantt,
        Subject::Today,
        Subject::Task,
        Subject::Resource,
        Subject::DaysAsDates,
        Subject::DayOfWeek,
        Subject::DayAsDate,
        Subject::Separator,
        Subject::WorkingHours,
    ] {
        cmds.push(Box::new(NaturalGanttCommand::new(s)));
    }
    // [A]->[B] between existing tasks, with plain codes
    cmds.push(single(
        arrow_parts(
            IRegex::named("CODE1", r"([%pLN_.]+)"),
            IRegex::named("CODE2", r"([%pLN_.]+)"),
        ),
        |d, arg| {
            let c1 = arg.get("CODE1", 0).unwrap_or("");
            let c2 = arg.get("CODE2", 0).unwrap_or("");
            let Some(t1) = d.existing_task(c1) else {
                return CommandExecutionResult::error(format!("No such task {c1}"));
            };
            let Some(t2) = d.existing_task(c2) else {
                return CommandExecutionResult::error(format!("No such task {c2}"));
            };
            let link = d.force_task_order(t1, t2);
            d.constraints[link].apply_style(arrow_style(arg).as_deref());
            CommandExecutionResult::ok()
        },
    ));
    // [A]->[B] creating the tasks
    let bracketed = |name: &str| {
        IRegex::group(vec![
            IRegex::leaf(r"\["),
            IRegex::named(name, r"([^\[\]]+?)"),
            IRegex::leaf(r"\]"),
        ])
    };
    let parts = arrow_parts(bracketed("TASK1"), bracketed("TASK2"));
    cmds.push(single(parts, |d, arg| {
        let n1 = arg.get("TASK1", 0).unwrap_or("");
        let n2 = arg.get("TASK2", 0).unwrap_or("");
        let t1 = d.get_or_create_task(TaskCode::from_id(n1), false);
        let t2 = d.get_or_create_task(TaskCode::from_id(n2), false);
        let link = d.force_task_order(t1, t2);
        d.constraints[link].apply_style(arrow_style(arg).as_deref());
        CommandExecutionResult::ok()
    }));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::named("CODE", r"\[([%pLN_.]+)\]"),
            IRegex::space_one_or_more(),
            IRegex::named("COLORS", r"#(\w+)(?:/(#?\w+))?"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let code = arg.get("CODE", 0).unwrap_or("");
            let Some(t) = d.existing_task(code) else {
                return CommandExecutionResult::error(format!("No such task {code}"));
            };
            let color = |s: Option<&str>| s.and_then(HColor::parse);
            let (c1, c2) = (arg.get("COLORS", 0), arg.get("COLORS", 1));
            if c1.is_some_and(|c| HColor::parse(c).is_none())
                || c2.is_some_and(|c| HColor::parse(c).is_none())
            {
                return CommandExecutionResult::bad_color();
            }
            if let Some(imp) = d.impl_mut(t) {
                imp.colors = vec![CenterBorderColor::new(color(c1), color(c2))];
            }
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("--"),
            IRegex::space_zero_or_more(),
            IRegex::named("COMMENT", "((.+?)[%s]*--)?"),
            IRegex::end(),
        ],
        |d, arg| {
            d.add_separator(arg.get("COMMENT", 1).map(str::to_string));
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("weeks?"),
            IRegex::space_one_or_more(),
            IRegex::leaf("starts?"),
            IRegex::leaf("[^0-9]*?"),
            IRegex::space_one_or_more(),
            IRegex::named("WEEKDAY", r"((?:MON|TUE|WED|THU|FRI|SAT|SUN)[a-z]*)"),
            IRegex::space_one_or_more(),
            IRegex::leaf("[^0-9]*?"),
            IRegex::named("NUM", "([0-9]+)"),
            IRegex::leaf("[^0-9]*?"),
            IRegex::end(),
        ],
        |d, arg| {
            let Some(day) = super::time::Dow::from_string(arg.get("WEEKDAY", 0).unwrap_or(""))
            else {
                return CommandExecutionResult::error("Bad day of week");
            };
            let num = arg.get("NUM", 0).and_then(|n| n.parse().ok()).unwrap_or(4);
            d.week_number_strategy = super::time::WeekNumberStrategy {
                first_day_of_week: day,
                minimal_days_in_first_week: num,
            };
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("group"),
            IRegex::space_one_or_more(),
            IRegex::leaf(r"\["),
            IRegex::named("NAME", r"([^\[\]]+)"),
            IRegex::leaf(r"\]"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| d.add_group(TaskCode::from_id(arg.get("NAME", 0).unwrap_or(""))),
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("end"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("group"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, _| d.end_group(),
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("language"),
            IRegex::space_one_or_more(),
            IRegex::named("LANG", r"(\w+)"),
            IRegex::end(),
        ],
        |d, arg| d.change_language(arg.get("LANG", 0).unwrap_or("en")),
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::or(vec![
                IRegex::leaf("projectscale"),
                IRegex::leaf("ganttscale"),
                IRegex::leaf("printscale"),
            ]),
            IRegex::space_one_or_more(),
            IRegex::or_named(
                "SCALE",
                vec![
                    IRegex::leaf("yearly"),
                    IRegex::leaf("quarterly"),
                    IRegex::leaf("monthly"),
                    IRegex::leaf("daily"),
                    IRegex::leaf("weekly"),
                ],
            ),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::or_named(
                    "OPTION",
                    vec![
                        IRegex::leaf(r"(with\s+calendar\s+date)"),
                        IRegex::named("NUMBER", r"(?:with\s+week\s+numbering\s+from\s+(-?\d+))"),
                    ],
                ),
            ])),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_one_or_more(),
                IRegex::leaf("zoom"),
                IRegex::space_one_or_more(),
                IRegex::named("ZOOM", r"([.\d]+)"),
            ])),
            IRegex::end(),
        ],
        |d, arg| {
            d.print_scale =
                PrintScale::from_string(&arg.get("SCALE", 0).unwrap_or("").to_lowercase());
            if let Some(z) = arg.get("ZOOM", 0) {
                d.factor_scale = z.parse().unwrap_or(1.0);
            }
            if let Some(option) = arg.get_lazzy("OPTION", 0) {
                if option.contains("date") {
                    d.weekly_header_strategy = Some(WeeklyHeaderStrategy::DayOfMonth);
                    d.week_starting_number = 0;
                } else if option.contains("numbering") {
                    d.weekly_header_strategy = Some(WeeklyHeaderStrategy::FromN);
                    d.week_starting_number = arg
                        .get("NUMBER", 0)
                        .and_then(|n| n.parse().ok())
                        .unwrap_or(0);
                }
            }
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(Box::new(UbrexLine {
        pattern: ub::concat(&[
            &ub::leaf("print"),
            &ub::space_one_or_more(),
            &ub::leaf("between"),
            &ub::space_one_or_more(),
            &ub::named("START", &Comp::Date(DateType::Any).ubrex()),
            &ub::space_one_or_more(),
            &ub::leaf("and"),
            &ub::space_one_or_more(),
            &ub::named("END", &Comp::Date(DateType::Any).ubrex()),
            &ub::end(),
        ]),
        exec: |d, cap| {
            let comp = Comp::Date(DateType::Any);
            let start = comp.get_me(d, &cap.extract_by_prefix("START"));
            let end = comp.get_me(d, &cap.extract_by_prefix("END"));
            match (start, end) {
                (Ok(super::lang::Val::Date(s)), Ok(super::lang::Val::Date(e))) => {
                    d.print_start = Some(s);
                    d.print_end = Some(e);
                    CommandExecutionResult::ok()
                }
                (Err(e), _) | (_, Err(e)) => CommandExecutionResult::error(e),
                _ => CommandExecutionResult::error("Invalid date"),
            }
        },
    }));
    cmds.push(note_bottom());
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::named("TYPE", "(hide|show)?"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("footbox"),
            IRegex::end(),
        ],
        |d, arg| {
            d.show_footbox = arg
                .get("TYPE", 0)
                .is_some_and(|t| t.eq_ignore_ascii_case("show"));
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("labels?"),
            IRegex::space_one_or_more(),
            IRegex::leaf("on"),
            IRegex::space_zero_or_more(),
            IRegex::named("POSITION", "(first|last)"),
            IRegex::space_zero_or_more(),
            IRegex::leaf("column"),
            IRegex::optional(IRegex::group(vec![
                IRegex::space_zero_or_more(),
                IRegex::leaf("and"),
                IRegex::space_zero_or_more(),
                IRegex::named("ALIGNED", "(left|right)"),
                IRegex::space_zero_or_more(),
                IRegex::leaf("aligned"),
            ])),
            IRegex::end(),
        ],
        |d, arg| {
            d.base.add_warning(vec![format!(
                "'labels on {} column' is no longer supported: this line is ignored",
                arg.get("POSITION", 0).unwrap_or("")
            )]);
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("hide"),
            IRegex::space_one_or_more(),
            IRegex::leaf("ress?ources?"),
            IRegex::space_one_or_more(),
            IRegex::leaf("names?"),
            IRegex::end(),
        ],
        |d, _| {
            d.hide_resource_name = true;
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("hide"),
            IRegex::space_one_or_more(),
            IRegex::leaf("ress?ources?"),
            IRegex::space_one_or_more(),
            IRegex::leaf("footbox"),
            IRegex::end(),
        ],
        |d, _| {
            d.hide_resource_footbox = true;
            CommandExecutionResult::ok()
        },
    ));
    let column = ub::or_vec(
        ["TASK", "START", "END", "DURATION"]
            .iter()
            .flat_map(|c| [ub::leaf(c), ub::leaf(&c.to_lowercase())])
            .collect(),
    );
    cmds.push(Box::new(UbrexLine {
        pattern: ub::concat(&[
            &ub::named("COMMAND", &ub::or(&[&ub::leaf("hide"), &ub::leaf("show")])),
            &ub::space_one_or_more(),
            &ub::leaf("column"),
            &ub::space_one_or_more(),
            &ub::named("WHAT", &column),
            &ub::end(),
        ]),
        exec: |d, cap| {
            let show = cap
                .find_first_value_by_key("COMMAND")
                .is_some_and(|c| c.starts_with(['s', 'S']));
            let what = cap.find_first_value_by_key("WHAT").unwrap_or_default();
            d.hide_or_show_column(show, Column::of(&what));
            CommandExecutionResult::ok()
        },
    }));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("hide"),
            IRegex::space_one_or_more(),
            IRegex::leaf("closed"),
            IRegex::end(),
        ],
        |d, _| {
            d.hide_closed = true;
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::leaf("task"),
            IRegex::space_one_or_more(),
            IRegex::or(vec![
                IRegex::leaf("default[%s]+completion"),
                IRegex::leaf("completion[%s]+default"),
            ]),
            IRegex::space_one_or_more(),
            IRegex::leaf("to"),
            IRegex::space_one_or_more(),
            IRegex::named("VALUE", r"(\d+)"),
            IRegex::leaf(".*"),
            IRegex::end(),
        ],
        |d, arg| {
            let value: i32 = arg
                .get("VALUE", 0)
                .and_then(|v| v.parse().ok())
                .unwrap_or(100);
            if value > 100 {
                return CommandExecutionResult::error("Completetion must between 0 and 100");
            }
            d.set_task_default_completion(value);
            CommandExecutionResult::ok()
        },
    ));
    cmds
}

fn note_bottom() -> Cmd {
    let start = IRegex::concat(vec![
        IRegex::start(),
        IRegex::named("TYPE", "(note)"),
        IRegex::space_zero_or_more(),
        IRegex::named("POSITION", "(bottom)"),
        IRegex::space_zero_or_more(),
        IRegex::named("TAGS", r"((?:#\w+)(?:[%s]+#\w+)*)?"),
        optional_pattern("STEREO"),
        IRegex::end(),
    ]);
    let again = start.clone();
    Box::new(MultiLineCommand::new(
        start,
        MultilinesStrategy::RemoveStartingQuote,
        Trim::Both,
        "^end[%s]*note$",
        move |d: &mut GanttDiagram, lines: &BlocLines, _| {
            let line0 = lines.at(0).trimmed();
            let Some(arg) = again.matcher(line0.string()) else {
                return CommandExecutionResult::error("Syntax error");
            };
            let body = lines.sub_extract(1, 1).remove_empty_columns();
            if body.size() == 0 {
                return CommandExecutionResult::error("No note defined");
            }
            let stereotype = arg.get("STEREO", 0).map(Stereotype::build);
            d.add_note(Display::create(&body.as_strings()), stereotype)
        },
    ))
}
