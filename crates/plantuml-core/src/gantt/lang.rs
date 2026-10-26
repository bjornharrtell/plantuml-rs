//! The natural language of Gantt commands (`gantt/lang`, `gantt/ulang`): a subject, then verb
//! phrases made of a verb and a complement, joined by `and`.

use super::calendar::TimeRange;
use super::model::*;
use super::time::{date_of, month_from_string, Dow, TimePoint};
use super::ubrex::{self as ub, Capture, TextCursor, UMatcher, Ub};
use super::GanttDiagram;
use crate::command::{Command, CommandControl, CommandExecutionResult, ParserPass};
use crate::klimt::color::HColor;
use crate::stereo::Stereotype;
use crate::text::BlocLines;

type Day = super::time::Day;

/// What a subject or a complement stands for once parsed.
#[derive(Clone)]
pub enum Val {
    Gantt,
    Today,
    Task(TaskId),
    Resource(String),
    /// First and last day, inclusive.
    Days(Day, Day),
    Dow(Dow),
    Date(Day),
    TimeRange(TimeRange),
    Load(i64),
    Time(TimePoint),
    Instant(TaskInstant),
    Colors(CenterBorderColor),
    ColorPair(Vec<CenterBorderColor>),
    Completion(i32),
    Str(String),
    TwoNames(String, String),
    Pair(Box<Val>, Box<Val>),
    Unit,
}

type Res<T> = Result<T, String>;

fn bad<T>() -> Res<T> {
    Err("internal error: unexpected value".to_string())
}

impl Val {
    fn task(&self) -> Option<TaskId> {
        match self {
            Val::Task(t) => Some(*t),
            _ => None,
        }
    }
    fn date(&self) -> Option<Day> {
        match self {
            Val::Date(d) => Some(*d),
            _ => None,
        }
    }
    fn days(&self) -> Option<(Day, Day)> {
        match self {
            Val::Days(a, b) => Some((*a, *b)),
            _ => None,
        }
    }
    fn str(&self) -> &str {
        match self {
            Val::Str(s) => s,
            _ => "",
        }
    }
    fn center(&self) -> Option<HColor> {
        match self {
            Val::Colors(c) => c.center.clone(),
            _ => None,
        }
    }
}

// ---- words ----

fn leaf(s: &str) -> Ub {
    ub::leaf(s)
}

fn sp1() -> Ub {
    ub::space_one_or_more()
}

fn sp0() -> Ub {
    ub::space_zero_or_more()
}

fn u_zero_or_more(words: &[&str]) -> Ub {
    let alts = ub::or_vec(words.iter().map(|w| leaf(w)).collect());
    ub::zero_or_more(&ub::concat(&[&alts, &sp1()]))
}

fn u_one_of(words: &[&str]) -> Ub {
    ub::or_vec(words.iter().map(|w| leaf(w)).collect())
}

fn u_exactly(words: &[&str]) -> Ub {
    let mut parts = Vec::new();
    for w in words {
        parts.push(sp1());
        parts.push(leaf(w));
    }
    ub::concat(&parts.iter().collect::<Vec<_>>())
}

fn u_exactly2(words: &[&str]) -> Ub {
    let mut parts = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            parts.push(sp1());
        }
        parts.push(leaf(w));
    }
    ub::concat(&parts.iter().collect::<Vec<_>>())
}

fn task_code(name: &str) -> Ub {
    ub::concat(&[
        &leaf("["),
        &ub::named(name, &leaf("〇+「〤[]」")),
        &leaf("]"),
    ])
}

fn stereotype_optional(name: &str) -> Ub {
    let m = ub::named(name, &leaf("<< 〇+「〤<>」>>"));
    ub::optional(&ub::concat(&[&sp0(), &m, &sp0()]))
}

fn month_ub() -> Ub {
    let alts = super::time::MONTH_NAMES
        .iter()
        .map(|m| prefix_ub(&m[..3], " 〇*〴le"))
        .collect();
    ub::or_vec(alts)
}

fn dow_ub() -> Ub {
    let alts = Dow::ALL
        .iter()
        .map(|d| prefix_ub(&d.name_upper()[..3], "〇*〴le"))
        .collect();
    ub::or_vec(alts)
}

fn prefix_ub(prefix: &str, tail: &str) -> Ub {
    let mut def = String::new();
    for c in prefix.chars() {
        def.push('「');
        def.push(c.to_ascii_uppercase());
        def.push(c.to_ascii_lowercase());
        def.push('」');
    }
    def.push_str(tail);
    leaf(&def)
}

/// `Dow` from the text of `dow_ub`.
fn dow_of(s: &str) -> Res<Dow> {
    Dow::from_string(s).ok_or_else(|| format!("bad day of week {s}"))
}

// ---- dates ----

fn digits(max: &str) -> Ub {
    leaf(max)
}

fn ubrex_a(year: &str, month: &str, day: &str) -> Ub {
    ub::concat(&[
        &ub::named(day, &digits("〇{1-2}〴d")),
        &ub::upto(&leaf("〴D"), &ub::named(month, &month_ub())),
        &leaf("〇+〴D"),
        &ub::named(year, &digits("〇{1-4}〴d")),
    ])
}

fn ubrex_b(year: &str, month: &str, day: &str) -> Ub {
    ub::concat(&[
        &ub::named(year, &digits("〇{1-4}〴d")),
        &leaf("〇+〴D"),
        &ub::named(month, &digits("〇{1-2}〴d")),
        &leaf("〇+〴D"),
        &ub::named(day, &digits("〇{1-2}〴d")),
    ])
}

fn ubrex_c(year: &str, month: &str, day: &str) -> Ub {
    ub::concat(&[
        &ub::named(month, &month_ub()),
        &leaf("〇+〴D"),
        &ub::named(day, &digits("〇{1-2}〴d")),
        &leaf("〇+〴D"),
        &ub::named(year, &digits("〇{1-4}〴d")),
    ])
}

/// The three ways to write a date (`DayPattern`).
struct DayPattern {
    id: String,
}

impl DayPattern {
    fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }

    fn key(&self, base: &str) -> String {
        format!("{base}{}", self.id)
    }

    fn to_ubrex(&self) -> Ub {
        ub::or(&[
            &ubrex_a(&self.key("AYEAR"), &self.key("AMONTH"), &self.key("ADAY")),
            &ubrex_b(&self.key("BYEAR"), &self.key("BMONTH"), &self.key("BDAY")),
            &ubrex_c(&self.key("CYEAR"), &self.key("CMONTH"), &self.key("CDAY")),
        ])
    }

    fn get_day(&self, cap: &Capture) -> Res<Option<Day>> {
        for letter in ['A', 'B', 'C'] {
            let get =
                |what: &str| cap.find_first_value_by_key(&self.key(&format!("{letter}{what}")));
            if let Some(day) = get("DAY") {
                let month = get("MONTH").unwrap_or_default();
                let year = get("YEAR").unwrap_or_default();
                let month = if letter == 'B' {
                    month.parse::<u32>().ok()
                } else {
                    month_from_string(&month)
                };
                let (Ok(day), Ok(year), Some(month)) =
                    (day.parse::<u32>(), year.parse::<i64>(), month)
                else {
                    return Err("Invalid date".into());
                };
                return date_of(year, month, day)
                    .map(Some)
                    .ok_or_else(|| "Invalid date".to_string());
            }
        }
        Ok(None)
    }
}

// ---- complements ----

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DateType {
    Any,
    OnlyRelative,
    OnlyAbsolute,
}

#[derive(Clone)]
pub enum Comp {
    Anything,
    BeforeOrAfter,
    Close,
    Completed,
    Date(DateType),
    DayOfWeek,
    Deleted,
    Duration,
    FromTo,
    InColors,
    InColors2,
    InColorsFromTo,
    Intervals,
    IntervalsSmart,
    Named,
    Open,
    Task,
    TimePoint,
    Url,
    WithColorLink,
    WorkingHours,
    Pair(Box<Comp>, Box<Comp>),
}

fn duration_before_or_after() -> Ub {
    ub::concat(&[
        &ub::named("COMPLEMENT_NB1", &leaf("〇+〴d")),
        &sp1(),
        &ub::optional(&ub::concat(&[
            &ub::named("COMPLEMENT_WORKING1", &leaf("working")),
            &sp1(),
        ])),
        &ub::named("COMPLEMENT_DAY_OR_WEEK1", &leaf("【day┇week】")),
        &leaf("〇?s"),
        &ub::optional(&ub::concat(&[
            &sp1(),
            &leaf("and"),
            &sp1(),
            &ub::named("COMPLEMENT_NB2", &leaf("〇+〴d")),
            &sp1(),
            &ub::optional(&ub::concat(&[
                &ub::named("COMPLEMENT_WORKING2", &leaf("working")),
                &sp1(),
            ])),
            &ub::named("COMPLEMENT_DAY_OR_WEEK2", &leaf("【day┇week】")),
            &leaf("〇?s"),
        ])),
        &sp1(),
        &ub::named(
            "COMPLEMENT_BEFORE_OR_AFTER",
            &u_one_of(&["before", "after"]),
        ),
    ])
}

fn color_pair(prefix_a: &str, prefix_b: &str) -> Ub {
    ub::concat(&[
        &ub::named(prefix_a, &leaf("〇?# 〇+〴w")),
        &ub::optional(&ub::concat(&[
            &leaf("/"),
            &ub::named(prefix_b, &leaf("〇?# 〇+〴w")),
        ])),
    ])
}

fn color_of(name: Option<String>) -> Option<HColor> {
    name.map(|n| HColor::parse_or_white(&n))
}

/// `AbstractComplementTaskInstant.getComplementTaskInstant`.
fn task_instant(g: &GanttDiagram, cap: &Capture) -> Res<TaskInstant> {
    let get = |k: &str| cap.find_first_value_by_key(k);
    let code = get("COMPLEMENT_CODE_OTHER").unwrap_or_default();
    let start_or_end = get("COMPLEMENT_START_OR_END").unwrap_or_default();
    let moment = g
        .existing_moment(&code)
        .ok_or_else(|| format!("No such task {code}"))?;
    let attr = if start_or_end.eq_ignore_ascii_case("start") {
        Attr::Start
    } else {
        Attr::End
    };
    let mut result = TaskInstant::new(moment, attr);
    if let Some(nb1) = get("COMPLEMENT_NB1") {
        let days_of = |nb: &str, unit: Option<String>| -> Res<i64> {
            let n: i64 = nb.parse().map_err(|_| "Bad number".to_string())?;
            let factor = if unit.is_some_and(|u| u.starts_with('w')) {
                g.days_in_week()
            } else {
                1
            };
            Ok(n * factor)
        };
        let days1 = days_of(&nb1, get("COMPLEMENT_DAY_OR_WEEK1"))?;
        let days2 = match get("COMPLEMENT_NB2") {
            Some(nb2) => days_of(&nb2, get("COMPLEMENT_DAY_OR_WEEK2"))?,
            None => 0,
        };
        let mut delta = (days1 + days2) as i32;
        if get("COMPLEMENT_BEFORE_OR_AFTER").is_some_and(|s| s.eq_ignore_ascii_case("before")) {
            delta = -delta;
        }
        let working = get("COMPLEMENT_WORKING1").is_some() || get("COMPLEMENT_WORKING2").is_some();
        let mode = if working {
            ConstraintMode::DoNotCountCloseDay
        } else {
            ConstraintMode::IgnoreCalendar
        };
        result = result.with_delta(delta, mode, g.default_plan());
    }
    Ok(result)
}

fn for_task(value: Option<String>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    match (value.find('['), value.rfind(']')) {
        (Some(x), Some(y)) if x > 0 => value[x + 1..y].to_string(),
        _ => String::new(),
    }
}

impl Comp {
    pub fn ubrex(&self) -> Ub {
        match self {
            Comp::Anything => ub::named("ANYTHING", &leaf("〇*〴.")),
            Comp::BeforeOrAfter => {
                let before = ub::optional(&ub::or(&[
                    &leaf("at"),
                    &leaf("with"),
                    &leaf("after"),
                    &duration_before_or_after(),
                ]));
                let code = ub::concat(&[&task_code("COMPLEMENT_CODE_OTHER"), &leaf("〴.s")]);
                ub::concat(&[
                    &before,
                    &sp0(),
                    &code,
                    &sp1(),
                    &ub::named("COMPLEMENT_START_OR_END", &u_one_of(&["start", "end"])),
                ])
            }
            Comp::Close => ub::named(
                "CLOSED",
                &ub::concat(&[
                    &leaf("close〇?d"),
                    &ub::optional(&ub::concat(&[&leaf("∙for∙"), &task_code("FOO")])),
                ]),
            ),
            Comp::Open => ub::named(
                "OPEN",
                &ub::concat(&[
                    &leaf("open〇?e〇?d"),
                    &ub::optional(&ub::concat(&[&leaf("∙for∙"), &task_code("FOO")])),
                ]),
            ),
            Comp::Completed => ub::concat(&[
                &ub::named("COMPLEMENT", &leaf("〇+〴d")),
                &leaf("〇+〴W complete〇?d"),
            ]),
            Comp::Date(t) => {
                let abs = DayPattern::new("").to_ubrex();
                let d = ub::concat(&[
                    &ub::named("DCOUNT", &leaf("〇+〴d")),
                    &sp1(),
                    &leaf("day〇?s"),
                    &sp1(),
                    &leaf("after"),
                    &sp1(),
                    &leaf("start"),
                ]);
                let e = ub::concat(&[&leaf("「dD」+"), &ub::named("ECOUNT", &leaf("〇+〴d"))]);
                match t {
                    DateType::OnlyAbsolute => abs,
                    DateType::OnlyRelative => ub::or(&[&d, &e]),
                    DateType::Any => ub::or(&[&abs, &d, &e]),
                }
            }
            Comp::DayOfWeek => ub::named("COMPLEMENT", &dow_ub()),
            Comp::Deleted => leaf("deleted"),
            Comp::Duration => {
                let one = |p: &str| {
                    ub::concat(&[
                        &ub::named(&format!("CNUM{p}"), &leaf("〇+〴d")),
                        &sp1(),
                        &ub::named(
                            &format!("CUNIT{p}"),
                            &leaf("【hour┇minute┇second┇day┇week┇month】"),
                        ),
                        &leaf("〇?s"),
                    ])
                };
                let sep = leaf("【〇+「∙,」and〇+「∙,」┇〇+「∙,」】");
                ub::concat(&[
                    &one("0"),
                    &ub::zero_or_more(&ub::concat(&[&sep, &one("1")])),
                ])
            }
            Comp::FromTo => ub::concat(&[
                &leaf("from"),
                &sp1(),
                &task_code("COMPLEMENT1"),
                &sp1(),
                &leaf("to"),
                &sp1(),
                &task_code("COMPLEMENT2"),
            ]),
            Comp::InColors => ub::concat(&[
                &leaf("in"),
                &sp1(),
                &color_pair("COMPLEMENT1", "COMPLEMENT2"),
            ]),
            Comp::InColors2 => ub::concat(&[
                &leaf("colo〇?ured"),
                &sp1(),
                &ub::optional(&leaf("in〇+〴s")),
                &color_pair("COMPLEMENT1", "COMPLEMENT2"),
            ]),
            Comp::InColorsFromTo => ub::concat(&[
                &leaf("from"),
                &sp1(),
                &color_pair("FROM1", "FROM2"),
                &sp1(),
                &leaf("to"),
                &sp1(),
                &color_pair("TO1", "TO2"),
            ]),
            Comp::Intervals => {
                let p1 = DayPattern::new("1");
                let p2 = DayPattern::new("2");
                let b = ub::concat(&[
                    &p1.to_ubrex(),
                    &u_exactly(&["to"]),
                    &u_zero_or_more(&["the"]),
                    &sp1(),
                    &p2.to_ubrex(),
                ]);
                let e = ub::concat(&[
                    &leaf("「dD」+"),
                    &ub::named("ECOUNT1", &leaf("〇+〴d")),
                    &u_exactly(&["to"]),
                    &u_zero_or_more(&["the"]),
                    &sp1(),
                    &leaf("「dD」+"),
                    &ub::named("ECOUNT2", &leaf("〇+〴d")),
                ]);
                ub::or(&[&b, &e])
            }
            Comp::IntervalsSmart => {
                let p1 = DayPattern::new("1");
                ub::concat(&[
                    &p1.to_ubrex(),
                    &u_exactly(&["to"]),
                    &sp0(),
                    &ub::optional(&duration_before_or_after()),
                    &sp0(),
                    &ub::concat(&[&task_code("COMPLEMENT_CODE_OTHER"), &leaf("〴.s")]),
                    &sp1(),
                    &ub::named("COMPLEMENT_START_OR_END", &u_one_of(&["start", "end"])),
                ])
            }
            Comp::Named => ub::concat(&[
                &leaf("["),
                &ub::named("COMPLEMENT", &leaf("〇+「〤[]」")),
                &leaf("]"),
            ]),
            Comp::Task => ub::named("COMPLEMENT", &task_code("FOO")),
            Comp::TimePoint => {
                let d12 = leaf("〇{1-2}〴d");
                let seconds = ub::optional(&ub::concat(&[&leaf(":"), &ub::named("SECOND", &d12)]));
                ub::concat(&[
                    &DayPattern::new("").to_ubrex(),
                    &leaf("「Tt∙」"),
                    &ub::named("HOUR", &d12),
                    &leaf(":"),
                    &ub::named("MINUTE", &d12),
                    &seconds,
                ])
            }
            Comp::Url => ub::named("COMPLEMENT", &leaf("[[〄>]]")),
            Comp::WithColorLink => {
                let style = ub::optional(&ub::concat(&[
                    &ub::named(
                        "STYLE",
                        &ub::or(&[&leaf("dotted"), &leaf("bold"), &leaf("dashed")]),
                    ),
                    &sp1(),
                ]));
                ub::concat(&[
                    &leaf("with"),
                    &sp1(),
                    &style,
                    &ub::named("COLOR", &leaf("〇?# 〇+〴w")),
                    &sp1(),
                    &style,
                    &leaf("link"),
                ])
            }
            Comp::WorkingHours => ub::concat(&[&leaf("working"), &sp1(), &leaf("hours")]),
            Comp::Pair(a, b) => ub::concat(&[&a.ubrex(), &sp1(), &b.ubrex()]),
        }
    }

    pub fn get_me(&self, g: &mut GanttDiagram, cap: &Capture) -> Res<Val> {
        let get = |k: &str| cap.find_first_value_by_key(k);
        match self {
            Comp::Anything => Ok(Val::Str(get("ANYTHING").unwrap_or_default())),
            Comp::BeforeOrAfter => Ok(Val::Instant(task_instant(g, cap)?)),
            Comp::Close => Ok(Val::Str(for_task(get("CLOSED")))),
            Comp::Open => Ok(Val::Str(for_task(get("OPEN")))),
            Comp::Completed => {
                let v = get("COMPLEMENT").unwrap_or_default();
                Ok(Val::Completion(
                    v.parse().map_err(|_| "Bad number".to_string())?,
                ))
            }
            Comp::Date(_) => {
                if let Some(d) = DayPattern::new("").get_day(cap)? {
                    return Ok(Val::Date(d));
                }
                for key in ["DCOUNT", "ECOUNT"] {
                    if let Some(n) = get(key) {
                        let n: i64 = n.parse().map_err(|_| "Bad number".to_string())?;
                        return Ok(Val::Date(g.min_day + n));
                    }
                }
                bad()
            }
            Comp::DayOfWeek => Ok(Val::Dow(dow_of(&get("COMPLEMENT").unwrap_or_default())?)),
            Comp::Deleted => Ok(Val::Unit),
            Comp::Duration => {
                let mut totals = [0i64; 4];
                let mut acc = |num: &str, unit: &str| -> Res<()> {
                    let v: i64 = num.parse().map_err(|_| "Bad number".to_string())?;
                    let mut chars = unit.chars();
                    match chars.next().map(|c| c.to_ascii_lowercase()) {
                        Some('h') => totals[1] += v,
                        Some('d') => totals[0] += v,
                        Some('w') => totals[0] += v * g.days_in_week(),
                        Some('s') => totals[3] += v,
                        Some('m') => {
                            if chars.next().is_some_and(|c| c.eq_ignore_ascii_case(&'i')) {
                                totals[2] += v;
                            } else {
                                totals[0] += v * g.days_in_month();
                            }
                        }
                        _ => return Err(format!("unknown time unit: {unit}")),
                    }
                    Ok(())
                };
                acc(
                    &get("CNUM0").unwrap_or_default(),
                    &get("CUNIT0").unwrap_or_default(),
                )?;
                let nums = cap.find_values_by_key("CNUM1");
                let units = cap.find_values_by_key("CUNIT1");
                for (n, u) in nums.iter().zip(units.iter()) {
                    acc(n, u)?;
                }
                let seconds = g.cal.duration_of_days(totals[0])
                    + totals[1] * 3600
                    + totals[2] * 60
                    + totals[3];
                Ok(Val::Load(seconds))
            }
            Comp::FromTo => Ok(Val::TwoNames(
                get("COMPLEMENT1").unwrap_or_default(),
                get("COMPLEMENT2").unwrap_or_default(),
            )),
            Comp::InColors | Comp::InColors2 => Ok(Val::Colors(CenterBorderColor::new(
                color_of(get("COMPLEMENT1")),
                color_of(get("COMPLEMENT2")),
            ))),
            Comp::InColorsFromTo => Ok(Val::ColorPair(vec![
                CenterBorderColor::new(color_of(get("FROM1")), color_of(get("FROM2"))),
                CenterBorderColor::new(color_of(get("TO1")), color_of(get("TO2"))),
            ])),
            Comp::Intervals => {
                if let Some(d1) = DayPattern::new("1").get_day(cap)? {
                    let d2 = DayPattern::new("2").get_day(cap)?.ok_or("Invalid date")?;
                    return Ok(Val::Days(d1, d2));
                }
                if let (Some(a), Some(b)) = (get("ECOUNT1"), get("ECOUNT2")) {
                    let a: i64 = a.parse().map_err(|_| "Bad number".to_string())?;
                    let b: i64 = b.parse().map_err(|_| "Bad number".to_string())?;
                    return Ok(Val::Days(g.min_day + a, g.min_day + b));
                }
                bad()
            }
            Comp::IntervalsSmart => {
                let d1 = DayPattern::new("1").get_day(cap)?.ok_or("Invalid date")?;
                let end = task_instant(g, cap)?;
                let mut precise = g.instant_precise(&end);
                if end.attribute == Attr::End {
                    precise = precise.decrement();
                }
                Ok(Val::Days(d1, precise.to_day()))
            }
            Comp::Named => Ok(Val::Str(get("COMPLEMENT").unwrap_or_default())),
            Comp::Task => {
                let code = get("COMPLEMENT").unwrap_or_default();
                g.existing_task(&code)
                    .map(Val::Task)
                    .ok_or_else(|| format!("No such task {code}"))
            }
            Comp::TimePoint => {
                let day = DayPattern::new("").get_day(cap)?.ok_or("Invalid date")?;
                let num = |k: &str| -> Res<i64> {
                    get(k).map_or(Ok(0), |v| v.parse().map_err(|_| "Bad number".to_string()))
                };
                let (h, m, s) = (num("HOUR")?, num("MINUTE")?, num("SECOND")?);
                if h > 23 || m > 59 || s > 59 {
                    return Err("Invalid time".into());
                }
                Ok(Val::Time(TimePoint::of_day_time(day, h, m, s)))
            }
            Comp::Url => Ok(Val::Unit),
            Comp::WithColorLink => {
                let c = color_of(get("COLOR"));
                Ok(Val::Colors(CenterBorderColor {
                    center: c.clone(),
                    border: c,
                    style: get("STYLE"),
                }))
            }
            Comp::WorkingHours => Ok(Val::Str(String::new())),
            Comp::Pair(a, b) => {
                let r1 = a.get_me(g, cap);
                let r2 = b.get_me(g, cap);
                Ok(Val::Pair(Box::new(r1?), Box::new(r2?)))
            }
        }
    }
}

// ---- verbs ----

type Exec = fn(&mut GanttDiagram, &Val, &Val) -> CommandExecutionResult;

pub struct VerbPhrase {
    verb: Ub,
    ignored: Option<Ub>,
    comp: Comp,
    comp_ub: Ub,
    exec: Exec,
}

fn vp(verb: &Ub, ignored: Option<Ub>, comp: Comp, exec: Exec) -> VerbPhrase {
    VerbPhrase {
        verb: verb.clone(),
        ignored,
        comp_ub: comp.ubrex(),
        comp,
        exec,
    }
}

struct Verbs {
    are: Ub,
    display_on_same_row_as: Ub,
    ends: Ub,
    happens: Ub,
    is: Ub,
    is_colored: Ub,
    is_displayed_as: Ub,
    is_off: Ub,
    is_on: Ub,
    is_or_are: Ub,
    is_or_are_named: Ub,
    just: Ub,
    links_to: Ub,
    occurs: Ub,
    pauses: Ub,
    requires: Ub,
    starts: Ub,
    works_on: Ub,
}

fn verbs() -> Verbs {
    Verbs {
        are: leaf("are"),
        display_on_same_row_as: leaf("display〇?s 〇+〴s on 〇+〴s same 〇+〴s row 〇+〴s as"),
        ends: leaf("ends"),
        happens: leaf("happens"),
        is: leaf("is"),
        is_colored: leaf("is 〇+〴s colo〇?ured"),
        is_displayed_as: leaf("is 〇+〴s displayed 〇+〴s as"),
        is_off: leaf("is 〇+〴s off"),
        is_on: leaf("is 〇+〴s on"),
        is_or_are: leaf("【is┇are】"),
        is_or_are_named: leaf("【is┇are】〇+〴s named"),
        just: leaf("just"),
        links_to: leaf("links 〇+〴s to"),
        occurs: leaf("occurs"),
        pauses: leaf("pauses"),
        requires: leaf("【lasts┇requires】"),
        starts: leaf("starts"),
        works_on: leaf("works 〇+〴s on"),
    }
}

fn ok() -> CommandExecutionResult {
    CommandExecutionResult::ok()
}

fn err_bad() -> CommandExecutionResult {
    CommandExecutionResult::error("internal error: unexpected value")
}

fn no_start(g: &GanttDiagram) -> bool {
    g.is_relative_mode()
}

const NO_START: &str = "No starting date for the project";

// ---- subjects ----

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Gantt,
    Today,
    Task,
    Resource,
    DaysAsDates,
    DayOfWeek,
    DayAsDate,
    Separator,
    WorkingHours,
}

impl Subject {
    fn ubrex(self) -> Ub {
        match self {
            Subject::Gantt => ub::or(&[&leaf("project"), &leaf("gantt")]),
            Subject::Today => leaf("today"),
            Subject::Task => ub::or(&[
                &ub::named("IT", &leaf("it")),
                &ub::concat(&[
                    &ub::optional(&ub::concat(&[&ub::named("THEN", &leaf("then")), &sp1()])),
                    &task_code("SUBJECT"),
                    &stereotype_optional("STEREOTYPE"),
                    &ub::optional(&ub::concat(&[
                        &u_exactly(&["as"]),
                        &sp1(),
                        &task_code("SHORTNAME"),
                    ])),
                    &ub::optional(&ub::concat(&[
                        &u_exactly(&["on"]),
                        &sp1(),
                        &ub::named(
                            "RESOURCE",
                            &ub::one_or_more(&ub::concat(&[&leaf("{〇+「〤{}」}"), &sp0()])),
                        ),
                    ])),
                ]),
            ]),
            Subject::Resource => ub::or(&[
                &ub::named("THEY", &leaf("【she┇he┇they】")),
                &ub::concat(&[
                    &leaf("{"),
                    &ub::named("RESOURCE", &leaf("〇+「〤{}」")),
                    &leaf("}"),
                ]),
            ]),
            Subject::DaysAsDates => {
                let b = ub::concat(&[
                    &ubrex_b("BYEAR1", "BMONTH1", "BDAY1"),
                    &u_exactly(&["to"]),
                    &sp1(),
                    &ubrex_b("BYEAR2", "BMONTH2", "BDAY2"),
                ]);
                let e = ub::concat(&[
                    &leaf("「dD」+"),
                    &ub::named("ECOUNT1", &leaf("〇+〴d")),
                    &u_exactly(&["to"]),
                    &sp1(),
                    &leaf("「dD」+"),
                    &ub::named("ECOUNT2", &leaf("〇+〴d")),
                ]);
                let and = ub::concat(&[
                    &ubrex_b("BYEAR3", "BMONTH3", "BDAY3"),
                    &u_exactly(&["and"]),
                    &sp1(),
                    &ub::named("COUNT_AND", &leaf("〇+〴d")),
                    &sp1(),
                    &leaf("day〇?s"),
                ]);
                let then = ub::concat(&[
                    &leaf("then"),
                    &sp1(),
                    &ub::named("COUNT_THEN", &leaf("〇+〴d")),
                    &sp1(),
                    &leaf("day〇?s"),
                ]);
                ub::or(&[&b, &e, &and, &then])
            }
            Subject::DayOfWeek => ub::named("SUBJECT", &dow_ub()),
            Subject::DayAsDate => {
                let e = ub::concat(&[
                    &ub::named("ETYPE", &leaf("「dDtTeE」")),
                    &ub::named("EOPERATION", &leaf("「-+」")),
                    &ub::named("ECOUNT", &leaf("〇+〴d")),
                ]);
                ub::or(&[&ubrex_b("BYEAR", "BMONTH", "BDAY"), &e])
            }
            Subject::Separator => leaf("separator"),
            Subject::WorkingHours => ub::concat(&[
                &leaf("from"),
                &sp1(),
                &ub::named("START", &leaf("〇+〴d:〇+〴d")),
                &sp1(),
                &leaf("to"),
                &sp1(),
                &ub::named("END", &leaf("〇+〴d:〇+〴d")),
            ]),
        }
    }

    fn get_me(self, g: &mut GanttDiagram, m: &UMatcher) -> Res<Val> {
        let get = |k: &str| m.find_first_value_by_key(k);
        match self {
            Subject::Gantt | Subject::Separator => Ok(Val::Gantt),
            Subject::Today => Ok(Val::Today),
            Subject::Task => {
                let id = if get("IT").is_some() {
                    g.it().ok_or("Not sure what are you refering to?")?
                } else {
                    let subject = get("SUBJECT");
                    let short = get("SHORTNAME");
                    let code = TaskCode::from_id_and_display(short.as_deref(), subject.as_deref())
                        .ok_or("Missing task name")?;
                    let id = g.get_or_create_task(code, get("THEN").is_some());
                    if let Some(st) = get("STEREOTYPE") {
                        g.tasks[id].stereotype = Some(Stereotype::build(&st));
                    }
                    g.set_it(id);
                    id
                };
                if let Some(resource) = get("RESOURCE") {
                    for part in resource.split(['{', '}']) {
                        let part = part.trim();
                        if !part.is_empty() && !g.affect_resource(id, part) {
                            return Err("Bad argument for resource".into());
                        }
                    }
                }
                Ok(Val::Task(id))
            }
            Subject::Resource => {
                if get("THEY").is_some() {
                    let they = g.they().ok_or("Not sure who are you refering to?")?;
                    return Ok(Val::Resource(they.to_string()));
                }
                let name = get("RESOURCE").unwrap_or_default();
                g.get_resource(&name);
                g.set_they(&name);
                Ok(Val::Resource(name))
            }
            Subject::DaysAsDates => {
                if let Some(count) = get("COUNT_AND") {
                    let d3 = subject_date(g, m, "3")?;
                    let n: i64 = count.parse().map_err(|_| "Bad number".to_string())?;
                    return Ok(days_with_count(g, d3, n));
                }
                if let Some(count) = get("COUNT_THEN") {
                    let d3 = g.then_date().to_day();
                    let n: i64 = count.parse().map_err(|_| "Bad number".to_string())?;
                    return Ok(days_with_count(g, d3, n));
                }
                let d1 = subject_date(g, m, "1")?;
                let d2 = subject_date(g, m, "2")?;
                Ok(Val::Days(d1, d2))
            }
            Subject::DayOfWeek => Ok(Val::Dow(dow_of(&get("SUBJECT").unwrap_or_default())?)),
            Subject::DayAsDate => {
                if let Some(day) = get("BDAY") {
                    let num = |v: String| v.parse::<i64>().map_err(|_| "Bad number".to_string());
                    let d = num(day)?;
                    let mo = num(get("BMONTH").unwrap_or_default())?;
                    let y = num(get("BYEAR").unwrap_or_default())?;
                    return date_of(y, mo as u32, d as u32)
                        .map(Val::Date)
                        .ok_or_else(|| "Invalid date".to_string());
                }
                if let Some(count) = get("ECOUNT") {
                    let kind = get("ETYPE").unwrap_or_default().to_uppercase();
                    let mut day: i64 = count.parse().map_err(|_| "Bad number".to_string())?;
                    if get("EOPERATION").as_deref() == Some("-") {
                        day = -day;
                    }
                    return match kind.as_str() {
                        "D" => Ok(Val::Date(g.min_day + day)),
                        "T" => Ok(Val::Date(g.get_today() + day)),
                        "E" => Ok(Val::Date(g.computed_max_day() + day)),
                        _ => bad(),
                    };
                }
                bad()
            }
            Subject::WorkingHours => {
                let start = get("START").unwrap_or_default();
                let end = get("END").unwrap_or_default();
                TimeRange::parse(&start, &end)
                    .map(Val::TimeRange)
                    .ok_or_else(|| "Invalid time".to_string())
            }
        }
    }

    fn verb_phrases(self) -> Vec<VerbPhrase> {
        let v = verbs();
        match self {
            Subject::Gantt => vec![vp(
                &v.starts,
                Some(u_zero_or_more(&["on", "for", "the", "at"])),
                Comp::Date(DateType::OnlyAbsolute),
                |g, _, c| match c.date() {
                    Some(d) => g.update_starting_point(d),
                    None => err_bad(),
                },
            )],
            Subject::Today => vec![
                vp(&v.is_colored, None, Comp::InColors, |g, _, c| {
                    g.set_today_colors(c.center());
                    ok()
                }),
                vp(&v.is, None, Comp::Date(DateType::Any), |g, _, c| {
                    match c.date() {
                        Some(d) => g.set_today(d),
                        None => err_bad(),
                    }
                }),
            ],
            Subject::Task => task_verbs(&v),
            Subject::Resource => resource_verbs(&v),
            Subject::DaysAsDates => vec![
                vp(&v.is_or_are, None, Comp::Close, |g, s, c| {
                    if let Some((a, b)) = s.days() {
                        for d in a..=b {
                            g.close_day_as_date(d, c.str());
                        }
                    }
                    ok()
                }),
                vp(&v.is_or_are, None, Comp::Open, |g, s, c| {
                    if let Some((a, b)) = s.days() {
                        for d in a..=b {
                            g.open_day_as_date(d, c.str());
                        }
                    }
                    ok()
                }),
                vp(&v.is_or_are, None, Comp::InColors2, |g, s, c| {
                    if let (Some((a, b)), Some(color)) = (s.days(), c.center()) {
                        for d in a..=b {
                            g.color_day(d, color.clone());
                        }
                    }
                    ok()
                }),
                vp(&v.is_or_are_named, None, Comp::Named, |g, s, c| {
                    if let Some((a, b)) = s.days() {
                        for d in a..=b {
                            g.name_day(d, c.str());
                        }
                    }
                    ok()
                }),
            ],
            Subject::DayOfWeek => vec![
                vp(&v.are, None, Comp::Open, |g, s, c| {
                    if let Val::Dow(d) = s {
                        g.open_day_of_week(*d, c.str());
                    }
                    ok()
                }),
                vp(&v.are, None, Comp::Close, |g, s, _| {
                    if let Val::Dow(d) = s {
                        g.close_day_of_week(*d);
                    }
                    ok()
                }),
                vp(&v.is_or_are, None, Comp::InColors2, |g, s, c| {
                    if let (Val::Dow(d), Some(color)) = (s, c.center()) {
                        g.color_day_of_week(*d, color);
                    }
                    ok()
                }),
            ],
            Subject::DayAsDate => vec![
                vp(&v.is_or_are, None, Comp::Open, |g, s, c| {
                    if let Some(d) = s.date() {
                        g.open_day_as_date(d, c.str());
                    }
                    ok()
                }),
                vp(&v.is_or_are, None, Comp::Close, |g, s, c| {
                    if let Some(d) = s.date() {
                        g.close_day_as_date(d, c.str());
                    }
                    ok()
                }),
                vp(&v.is_or_are, None, Comp::InColors2, |g, s, c| {
                    if let (Some(d), Some(color)) = (s.date(), c.center()) {
                        g.color_day(d, color);
                    }
                    ok()
                }),
            ],
            Subject::Separator => vec![
                vp(
                    &v.just,
                    Some(leaf("before")),
                    Comp::Date(DateType::Any),
                    |g, _, c| {
                        if let Some(d) = c.date() {
                            g.add_vertical_separator_before(d);
                        }
                        ok()
                    },
                ),
                vp(
                    &v.just,
                    Some(leaf("after")),
                    Comp::Date(DateType::Any),
                    |g, _, c| {
                        if let Some(d) = c.date() {
                            g.add_vertical_separator_before(d + 1);
                        }
                        ok()
                    },
                ),
                vp(&v.just, None, Comp::BeforeOrAfter, |g, _, c| {
                    if let Val::Instant(i) = c {
                        let d = g.instant_precise(i).to_day();
                        g.add_vertical_separator_before(d);
                    }
                    ok()
                }),
            ],
            Subject::WorkingHours => {
                vec![vp(&v.are, None, Comp::WorkingHours, |g, s, _| match s {
                    Val::TimeRange(r) => g.add_working_time_range(*r),
                    _ => err_bad(),
                })]
            }
        }
    }
}

fn subject_date(g: &GanttDiagram, m: &UMatcher, suffix: &str) -> Res<Day> {
    if let Some(day) = m.find_first_value_by_key(&format!("BDAY{suffix}")) {
        let num = |v: Option<String>| {
            v.unwrap_or_default()
                .parse::<i64>()
                .map_err(|_| "Bad number".to_string())
        };
        let (y, mo) = (
            num(m.find_first_value_by_key(&format!("BYEAR{suffix}")))?,
            num(m.find_first_value_by_key(&format!("BMONTH{suffix}")))?,
        );
        let d: i64 = day.parse().map_err(|_| "Bad number".to_string())?;
        return date_of(y, mo as u32, d as u32).ok_or_else(|| "Invalid date".to_string());
    }
    if let Some(n) = m.find_first_value_by_key(&format!("ECOUNT{suffix}")) {
        let n: i64 = n.parse().map_err(|_| "Bad number".to_string())?;
        return Ok(g.min_day + n);
    }
    bad()
}

/// `DaysAsDates(gantt, date, count)`: `count` open days starting at `date`.
fn days_with_count(g: &GanttDiagram, date: Day, count: i64) -> Val {
    let mut count = count;
    let mut tmp = date;
    let mut guard = 0;
    while count > 0 && guard < 100_000 {
        if g.cal.is_open(tmp) {
            count -= 1;
        }
        tmp += 1;
        guard += 1;
    }
    Val::Days(date, tmp)
}

fn task_verbs(v: &Verbs) -> Vec<VerbPhrase> {
    let on_for = || Some(u_zero_or_more(&["on", "for", "the", "at"]));
    let the_on_at = || Some(u_zero_or_more(&["the", "on", "at"]));
    vec![
        vp(&v.requires, on_for(), Comp::Duration, |g, s, c| {
            if let (Some(t), Val::Load(l)) = (s.task(), c) {
                g.set_task_load(t, *l);
            }
            ok()
        }),
        vp(&v.starts, the_on_at(), Comp::TimePoint, |g, s, c| {
            if no_start(g) {
                return CommandExecutionResult::error(NO_START);
            }
            if let (Some(t), Val::Time(p)) = (s.task(), c) {
                g.set_task_start(t, *p);
            }
            ok()
        }),
        vp(
            &v.starts,
            None,
            Comp::Pair(Box::new(Comp::BeforeOrAfter), Box::new(Comp::WithColorLink)),
            |g, s, c| {
                let (Some(t), Val::Pair(a, b)) = (s.task(), c) else {
                    return err_bad();
                };
                let (Val::Instant(when), Val::Colors(col)) = (&**a, &**b) else {
                    return err_bad();
                };
                let p = g.instant_precise(when);
                g.set_task_start(t, p);
                if when.is_task() {
                    let mut link = GanttConstraint::new(
                        when.clone(),
                        TaskInstant::new(Moment::Task(t), Attr::Start),
                        col.center.clone(),
                    );
                    link.apply_style(col.style.as_deref());
                    g.add_constraint(link);
                }
                ok()
            },
        ),
        vp(&v.starts, None, Comp::BeforeOrAfter, |g, s, c| {
            let (Some(t), Val::Instant(when)) = (s.task(), c) else {
                return err_bad();
            };
            let p = g.instant_precise(when);
            g.set_task_start(t, p);
            if when.is_task() {
                g.add_constraint(GanttConstraint::new(
                    when.clone(),
                    TaskInstant::new(Moment::Task(t), Attr::Start),
                    None,
                ));
            }
            ok()
        }),
        vp(
            &v.starts,
            the_on_at(),
            Comp::Date(DateType::OnlyRelative),
            |g, s, c| {
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    g.set_task_start(t, TimePoint::of_start_of_day(d));
                }
                ok()
            },
        ),
        vp(
            &v.starts,
            the_on_at(),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if no_start(g) {
                    return CommandExecutionResult::error(NO_START);
                }
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    g.set_task_start(t, TimePoint::of_start_of_day(d));
                }
                ok()
            },
        ),
        vp(&v.is_colored, None, Comp::InColors, |g, s, c| {
            if let (Some(t), Val::Colors(col)) = (s.task(), c) {
                if let Some(imp) = g.impl_mut(t) {
                    imp.colors = vec![col.clone()];
                }
            }
            ok()
        }),
        vp(
            &v.is_colored,
            Some(u_exactly2(&["for", "completion"])),
            Comp::InColorsFromTo,
            |g, s, c| {
                if let (Some(t), Val::ColorPair(cols)) = (s.task(), c) {
                    if let Some(imp) = g.impl_mut(t) {
                        imp.colors = cols.clone();
                    }
                }
                ok()
            },
        ),
        vp(
            &v.happens,
            the_on_at(),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    let day_load = g.cal.duration_of_days(1);
                    g.set_task_load(t, day_load);
                    g.set_task_start(t, TimePoint::of_start_of_day(d));
                    if let Some(imp) = g.impl_mut(t) {
                        imp.diamond = true;
                    }
                }
                ok()
            },
        ),
        vp(&v.happens, the_on_at(), Comp::BeforeOrAfter, |g, s, c| {
            let (Some(t), Val::Instant(when)) = (s.task(), c) else {
                return err_bad();
            };
            let day_load = g.cal.duration_of_days(1);
            g.set_task_load(t, day_load);
            if let Some(imp) = g.impl_mut(t) {
                imp.diamond = true;
            }
            let p = g.instant_precise(when);
            g.set_task_start(
                t,
                if when.attribute == Attr::End {
                    p.decrement()
                } else {
                    p
                },
            );
            ok()
        }),
        vp(&v.occurs, None, Comp::FromTo, |g, s, c| {
            let (Some(t), Val::TwoNames(n1, n2)) = (s.task(), c) else {
                return err_bad();
            };
            let Some(from) = g.existing_task(n1) else {
                return CommandExecutionResult::error(format!("No such {n1} task"));
            };
            let Some(to) = g.existing_task(n2) else {
                return CommandExecutionResult::error(format!("No such {n2} task"));
            };
            let (fs, te) = (g.task_start(from), g.task_end(to));
            g.set_task_start(t, fs);
            g.set_task_end(t, te);
            g.add_constraint(GanttConstraint::new(
                TaskInstant::new(Moment::Task(from), Attr::Start),
                TaskInstant::new(Moment::Task(t), Attr::Start),
                None,
            ));
            g.add_constraint(GanttConstraint::new(
                TaskInstant::new(Moment::Task(to), Attr::End),
                TaskInstant::new(Moment::Task(t), Attr::End),
                None,
            ));
            ok()
        }),
        vp(&v.ends, the_on_at(), Comp::TimePoint, |g, s, c| {
            if no_start(g) {
                return CommandExecutionResult::error(NO_START);
            }
            if let (Some(t), Val::Time(p)) = (s.task(), c) {
                g.set_task_end(t, *p);
            }
            ok()
        }),
        vp(&v.ends, None, Comp::BeforeOrAfter, |g, s, c| {
            let (Some(t), Val::Instant(when)) = (s.task(), c) else {
                return err_bad();
            };
            let p = g.instant_precise(when);
            g.set_task_end(t, p);
            g.add_constraint(GanttConstraint::new(
                when.clone(),
                TaskInstant::new(Moment::Task(t), Attr::End),
                None,
            ));
            ok()
        }),
        vp(
            &v.ends,
            the_on_at(),
            Comp::Date(DateType::OnlyRelative),
            |g, s, c| {
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    g.set_task_end(t, TimePoint::of_start_of_day(d).increment());
                }
                ok()
            },
        ),
        vp(
            &v.ends,
            the_on_at(),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if no_start(g) {
                    return CommandExecutionResult::error(NO_START);
                }
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    g.set_task_end(t, TimePoint::of_start_of_day(d).increment());
                }
                ok()
            },
        ),
        vp(&v.display_on_same_row_as, None, Comp::Named, |g, s, c| {
            let Some(t1) = s.task() else { return err_bad() };
            let Some(t2) = g.existing_task(c.str()) else {
                return CommandExecutionResult::error(format!("No such task {}", c.str()));
            };
            if t1 != t2 {
                g.tasks[t1].row = Some(t2);
            }
            ok()
        }),
        vp(&v.is, None, Comp::Deleted, |g, s, _| match s.task() {
            Some(t) => g.delete_task(t),
            None => err_bad(),
        }),
        vp(&v.is, None, Comp::Completed, |g, s, c| {
            if let (Some(t), Val::Completion(n)) = (s.task(), c) {
                if let Some(imp) = g.impl_mut(t) {
                    imp.completion = *n;
                }
            }
            ok()
        }),
        vp(
            &v.pauses,
            Some(u_zero_or_more(&["the", "on", "at", "from"])),
            Comp::Intervals,
            pause_days,
        ),
        vp(
            &v.pauses,
            Some(u_zero_or_more(&["the", "on", "at", "from"])),
            Comp::IntervalsSmart,
            pause_days,
        ),
        vp(
            &v.pauses,
            Some(u_zero_or_more(&["the", "on", "at", "from"])),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if let (Some(t), Some(d)) = (s.task(), c.date()) {
                    if let Some(imp) = g.impl_mut(t) {
                        imp.paused_days.insert(d);
                    }
                }
                ok()
            },
        ),
        vp(
            &v.pauses,
            Some(u_zero_or_more(&["the", "on", "at", "from"])),
            Comp::DayOfWeek,
            |g, s, c| {
                if let (Some(t), Val::Dow(d)) = (s.task(), c) {
                    if let Some(imp) = g.impl_mut(t) {
                        imp.paused_dow.insert(*d);
                    }
                }
                ok()
            },
        ),
        vp(&v.links_to, None, Comp::Url, |_, _, _| ok()),
        vp(&v.is_displayed_as, None, Comp::Anything, |g, s, c| {
            if let Some(t) = s.task() {
                g.tasks[t].display = Some(c.str().to_string());
            }
            ok()
        }),
    ]
}

fn pause_days(g: &mut GanttDiagram, s: &Val, c: &Val) -> CommandExecutionResult {
    if let (Some(t), Some((a, b))) = (s.task(), c.days()) {
        if let Some(imp) = g.impl_mut(t) {
            for d in a..=b {
                imp.paused_days.insert(d);
            }
        }
    }
    ok()
}

fn resource_verbs(v: &Verbs) -> Vec<VerbPhrase> {
    let from_on = || Some(u_zero_or_more(&["from", "on", "for", "the", "at"]));
    let res_mut = |g: &mut GanttDiagram, s: &Val| -> Option<usize> {
        match s {
            Val::Resource(name) => Some(g.get_resource(name)),
            _ => None,
        }
    };
    let _ = res_mut;
    fn with_res(g: &mut GanttDiagram, s: &Val, f: impl FnOnce(&mut super::calendar::OpenClose)) {
        if let Val::Resource(name) = s {
            let i = g.get_resource(name);
            f(&mut g.resources[i].open_close);
        }
    }
    vec![
        vp(&v.works_on, None, Comp::Task, |g, s, c| {
            if let (Val::Resource(name), Some(t)) = (s, c.task()) {
                if let Some(imp) = g.impl_mut(t) {
                    match imp.resources.iter_mut().find(|(r, _)| r == name) {
                        Some(e) => e.1 = 100,
                        None => imp.resources.push((name.clone(), 100)),
                    }
                }
            }
            ok()
        }),
        vp(
            &v.is_off,
            Some(ub::concat(&[&leaf("before"), &u_zero_or_more(&["the"])])),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if let Some(d) = c.date() {
                    with_res(g, s, |oc| oc.set_off_before(d));
                }
                ok()
            },
        ),
        vp(
            &v.is_off,
            Some(ub::concat(&[&leaf("after"), &u_zero_or_more(&["the"])])),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if let Some(d) = c.date() {
                    with_res(g, s, |oc| oc.set_off_after(d));
                }
                ok()
            },
        ),
        vp(&v.is_off, from_on(), Comp::Intervals, |g, s, c| {
            if let Some((a, b)) = c.days() {
                with_res(g, s, |oc| {
                    for d in a..=b {
                        oc.close_day(d);
                    }
                });
            }
            ok()
        }),
        vp(&v.is_off, from_on(), Comp::DayOfWeek, |g, s, c| {
            if let Val::Dow(d) = c {
                with_res(g, s, |oc| oc.close_dow(*d));
            }
            ok()
        }),
        vp(
            &v.is_off,
            from_on(),
            Comp::Date(DateType::Any),
            |g, s, c| {
                if let Some(d) = c.date() {
                    with_res(g, s, |oc| oc.close_day(d));
                }
                ok()
            },
        ),
        vp(&v.is_on, from_on(), Comp::Intervals, |g, s, c| {
            if let Some((a, b)) = c.days() {
                with_res(g, s, |oc| {
                    for d in a..=b {
                        oc.open_day(d);
                    }
                });
            }
            ok()
        }),
        vp(&v.is_on, from_on(), Comp::Date(DateType::Any), |g, s, c| {
            if let Some(d) = c.date() {
                with_res(g, s, |oc| oc.open_day(d));
            }
            ok()
        }),
    ]
}

// ---- the command ----

fn skip_spaces(tn: &mut TextCursor, space: &Ub) {
    let m = tn.match_ub(space);
    tn.jump(m.accepted_len());
}

impl VerbPhrase {
    /// Whether the verb and a complement follow (`check`).
    fn check(&self, tn: &TextCursor, space: &Ub) -> bool {
        self.parse(tn.clone(), space).is_some()
    }

    /// Consumes the verb phrase; returns the complement matcher and the advanced cursor.
    fn parse(&self, mut tn: TextCursor, space: &Ub) -> Option<(UMatcher, TextCursor)> {
        skip_spaces(&mut tn, space);
        let verb = tn.match_ub(&self.verb);
        if verb.accepted_len() == 0 {
            return None;
        }
        tn.jump(verb.accepted_len());
        skip_spaces(&mut tn, space);
        if let Some(ignored) = &self.ignored {
            tn.jump(tn.match_ub(ignored).accepted_len());
            skip_spaces(&mut tn, space);
        }
        let comp = tn.match_ub(&self.comp_ub);
        if comp.accepted_len() == 0 {
            return None;
        }
        tn.jump(comp.accepted_len());
        Some((comp, tn))
    }
}

/// A command made of a subject and verb phrases (`NaturalGanttCommand`).
pub struct NaturalGanttCommand {
    subject: Subject,
    subject_ub: Ub,
    space: Ub,
    and: Ub,
    verbs: Vec<VerbPhrase>,
}

impl NaturalGanttCommand {
    pub fn new(subject: Subject) -> Self {
        Self {
            subject,
            subject_ub: subject.ubrex(),
            space: sp1(),
            and: leaf("〇+〴s and 〇+〴s"),
            verbs: subject.verb_phrases(),
        }
    }
}

impl Command<GanttDiagram> for NaturalGanttCommand {
    fn execute(
        &self,
        diagram: &mut GanttDiagram,
        lines: &BlocLines,
        _pass: ParserPass,
    ) -> CommandExecutionResult {
        let mut tn = TextCursor::new(lines.at(0).string());
        skip_spaces(&mut tn, &self.space);
        let subject_match = tn.match_ub(&self.subject_ub);
        tn.jump(subject_match.accepted_len());
        skip_spaces(&mut tn, &self.space);
        let subject = match self.subject.get_me(diagram, &subject_match) {
            Ok(v) => v,
            Err(e) => return CommandExecutionResult::error(e),
        };
        let mut candidates: std::collections::VecDeque<&VerbPhrase> = self.verbs.iter().collect();
        while let Some(phrase) = candidates.pop_front() {
            if !phrase.check(&tn, &self.space) {
                continue;
            }
            let Some((comp_match, next)) = phrase.parse(tn.clone(), &self.space) else {
                continue;
            };
            tn = next;
            let complement = match phrase.comp.get_me(diagram, comp_match.capture()) {
                Ok(v) => v,
                Err(e) => return CommandExecutionResult::error(e),
            };
            let result = (phrase.exec)(diagram, &subject, &complement);
            if !result.is_ok() {
                return result;
            }
            let and = tn.match_ub(&self.and);
            if and.accepted_len() == 0 {
                return result;
            }
            tn.jump(and.accepted_len());
            candidates = self.verbs.iter().collect();
        }
        CommandExecutionResult::error("No matching verb phrase")
    }

    fn is_valid(&self, lines: &BlocLines) -> CommandControl {
        let mut tn = TextCursor::new(lines.at(0).string());
        skip_spaces(&mut tn, &self.space);
        let m = tn.match_ub(&self.subject_ub);
        if m.accepted_len() == 0 {
            return CommandControl::NotOk;
        }
        tn.jump(m.accepted_len());
        skip_spaces(&mut tn, &self.space);
        if self.verbs.iter().any(|v| v.check(&tn, &self.space)) {
            CommandControl::Ok
        } else {
            CommandControl::NotOk
        }
    }
}
