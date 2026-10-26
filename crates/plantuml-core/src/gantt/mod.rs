//! Gantt diagrams (`gantt` package): tasks, milestones, resources and dependencies laid out on
//! a timeline.

mod calendar;
mod commands;
mod ctx;
mod header;
mod i18n;
mod i18n_data;
mod lang;
mod model;
mod render;
mod taskdraw;
mod time;
mod timescale;
mod ubrex;

#[cfg(test)]
mod tests;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::Margins;
use calendar::{
    integrate, is_zero_on_day, ByWeekday, Combiner, Dir, Fraction, OpenClose, Pc, SpecificDays,
    TimeRange,
};
use model::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use time::{Day, Dow, TimePoint, WeekNumberStrategy, SECS_PER_DAY};

/// How wide a day is drawn (`PrintScale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintScale {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl PrintScale {
    pub fn default_scale(self) -> f64 {
        1.0 / match self {
            PrintScale::Daily => 1.0,
            PrintScale::Weekly => 4.0,
            PrintScale::Monthly => 15.0,
            PrintScale::Quarterly => 40.0,
            PrintScale::Yearly => 60.0,
        }
    }

    pub fn from_string(value: &str) -> PrintScale {
        match value.chars().next() {
            Some('w') => PrintScale::Weekly,
            Some('m') => PrintScale::Monthly,
            Some('q') => PrintScale::Quarterly,
            Some('y') => PrintScale::Yearly,
            _ => PrintScale::Daily,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeeklyHeaderStrategy {
    WeekOfYear,
    DayOfMonth,
    FromN,
}

/// Columns of the table of tasks (`GanttTaskTableColumn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Column {
    Task,
    Start,
    End,
    Duration,
}

impl Column {
    pub fn of(what: &str) -> Option<Column> {
        match what.to_uppercase().as_str() {
            "TASK" => Some(Column::Task),
            "START" => Some(Column::Start),
            "END" => Some(Column::End),
            "DURATION" => Some(Column::Duration),
            _ => None,
        }
    }
}

/// Everything known about days (`DayCalendarData`).
#[derive(Default)]
pub struct DayCalendar {
    pub open_close: OpenClose,
    pub name_days: HashMap<TimePoint, String>,
    pub color_days_today: HashMap<TimePoint, HColor>,
    pub color_days: HashMap<TimePoint, HColor>,
    pub color_days_of_week: HashMap<Dow, HColor>,
    pub vertical_separator_before: HashSet<Day>,
    pub open_close_for_task: HashMap<String, OpenClose>,
    pub working_time_ranges: Vec<TimeRange>,
}

impl DayCalendar {
    pub fn is_open(&self, day: Day) -> bool {
        !self.open_close.is_closed(day)
    }

    pub fn day_color(&self, day: TimePoint) -> Option<HColor> {
        self.color_days_today
            .get(&day)
            .or_else(|| self.color_days.get(&day))
            .cloned()
    }

    pub fn has_working_time_ranges(&self) -> bool {
        !self.working_time_ranges.is_empty()
    }

    /// `durationOfDays`: seconds of effort in some days.
    pub fn duration_of_days(&self, total_days: i64) -> i64 {
        if self.has_working_time_ranges() {
            self.working_time_ranges
                .iter()
                .map(TimeRange::duration)
                .sum::<i64>()
                * total_days
        } else {
            total_days * SECS_PER_DAY
        }
    }
}

/// The diagram being built by commands (`GanttDiagram`).
pub struct GanttDiagram {
    pub base: DiagramBase,
    pub tasks: Vec<Task>,
    pub code_index: HashMap<String, TaskId>,
    pub resources: Vec<Resource>,
    pub constraints: Vec<GanttConstraint>,
    pub cal: DayCalendar,
    pub min_day: Day,
    pub print_start: Option<Day>,
    pub print_end: Option<Day>,
    pub print_scale: PrintScale,
    pub factor_scale: f64,
    pub hide_closed: bool,
    pub lang: String,
    pub week_number_strategy: WeekNumberStrategy,
    pub weekly_header_strategy: Option<WeeklyHeaderStrategy>,
    pub week_starting_number: i32,
    pub show_footbox: bool,
    pub hide_resource_name: bool,
    pub hide_resource_footbox: bool,
    pub displayed_columns: Vec<Column>,
    today: Option<TimePoint>,
    default_completion: i32,
    it: Option<TaskId>,
    they: Option<String>,
    current_group: Option<TaskId>,
}

impl GanttDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Gantt),
            tasks: Vec::new(),
            code_index: HashMap::new(),
            resources: Vec::new(),
            constraints: Vec::new(),
            cal: DayCalendar::default(),
            min_day: 0,
            print_start: None,
            print_end: None,
            print_scale: PrintScale::Daily,
            factor_scale: 1.0,
            hide_closed: false,
            lang: "en".into(),
            week_number_strategy: WeekNumberStrategy::default(),
            weekly_header_strategy: None,
            week_starting_number: 0,
            show_footbox: true,
            hide_resource_name: false,
            hide_resource_footbox: false,
            displayed_columns: vec![Column::Start, Column::End, Column::Duration],
            today: None,
            default_completion: 100,
            it: None,
            they: None,
            current_group: None,
        }
    }

    // ---- simple settings ----

    pub fn change_language(&mut self, lang: &str) -> CommandExecutionResult {
        self.lang = lang.to_lowercase();
        CommandExecutionResult::ok()
    }

    pub fn effective_scale(&self) -> f64 {
        self.print_scale.default_scale() * self.factor_scale
    }

    pub fn is_relative_mode(&self) -> bool {
        self.min_day == 0
    }

    pub fn hide_or_show_column(&mut self, show: bool, column: Option<Column>) {
        if let Some(c) = column {
            if show {
                if !self.displayed_columns.contains(&c) {
                    self.displayed_columns.push(c);
                }
            } else {
                self.displayed_columns.retain(|x| *x != c);
            }
            self.displayed_columns.sort();
        }
    }

    // ---- calendar commands ----

    pub fn close_day_of_week(&mut self, day: Dow) {
        self.cal.open_close.close_dow(day);
    }

    pub fn open_day_of_week(&mut self, day: Dow, task: &str) {
        if task.is_empty() {
            self.cal.open_close.open_dow(day);
        } else {
            self.cal
                .open_close_for_task
                .entry(task.to_string())
                .or_default()
                .open_dow(day);
        }
    }

    pub fn close_day_as_date(&mut self, day: Day, task: &str) {
        if task.is_empty() {
            self.cal.open_close.close_day(day);
        } else {
            self.cal
                .open_close_for_task
                .entry(task.to_string())
                .or_default()
                .close_day(day);
        }
    }

    pub fn open_day_as_date(&mut self, day: Day, task: &str) {
        if task.is_empty() {
            self.cal.open_close.open_day(day);
        } else {
            self.cal
                .open_close_for_task
                .entry(task.to_string())
                .or_default()
                .open_day(day);
        }
    }

    pub fn color_day(&mut self, day: Day, color: HColor) {
        self.cal
            .color_days
            .insert(TimePoint::of_start_of_day(day), color);
    }

    pub fn color_day_of_week(&mut self, day: Dow, color: HColor) {
        self.cal.color_days_of_week.insert(day, color);
    }

    pub fn name_day(&mut self, day: Day, name: &str) {
        self.cal
            .name_days
            .insert(TimePoint::of_start_of_day(day), name.to_string());
    }

    pub fn add_vertical_separator_before(&mut self, day: Day) {
        self.cal.vertical_separator_before.insert(day);
    }

    pub fn add_working_time_range(&mut self, r: TimeRange) -> CommandExecutionResult {
        self.cal.working_time_ranges.push(r);
        self.cal.open_close.add_working_time_range(r);
        CommandExecutionResult::ok()
    }

    pub fn get_today(&mut self) -> Day {
        if self.today.is_none() {
            self.today = Some(todays_midnight());
        }
        self.today.map_or(0, TimePoint::to_day)
    }

    pub fn set_today_colors(&mut self, center: Option<HColor>) {
        let today = TimePoint::of_start_of_day(self.get_today());
        if let Some(c) = center {
            self.cal.color_days_today.insert(today, c);
        }
    }

    pub fn set_today(&mut self, date: Day) -> CommandExecutionResult {
        self.today = Some(TimePoint::of_start_of_day(date));
        CommandExecutionResult::ok()
    }

    pub fn update_starting_point(&mut self, start: Day) -> CommandExecutionResult {
        if !self.tasks.is_empty() {
            return CommandExecutionResult::error(
                "Starting point must be set before task definition",
            );
        }
        self.min_day = start;
        CommandExecutionResult::ok()
    }

    pub fn days_in_week(&self) -> i64 {
        self.cal.open_close.days_in_week()
    }

    pub fn days_in_month(&self) -> i64 {
        30
    }

    /// The date after the last colored or named day (`getThenDate`).
    pub fn then_date(&self) -> TimePoint {
        let mut result = TimePoint::of_start_of_day(self.min_day);
        for d in self.cal.color_days.keys().chain(self.cal.name_days.keys()) {
            if *d > result {
                result = *d;
            }
        }
        result
    }

    // ---- tasks ----

    pub fn existing_task(&self, id: &str) -> Option<TaskId> {
        self.code_index.get(id).copied()
    }

    fn put_task(&mut self, task: Task) -> TaskId {
        let id = task.code.id.clone();
        if let Some(&existing) = self.code_index.get(&id) {
            self.tasks[existing] = task;
            return existing;
        }
        self.tasks.push(task);
        self.code_index.insert(id, self.tasks.len() - 1);
        self.tasks.len() - 1
    }

    fn last_created_task(&self) -> Option<TaskId> {
        self.tasks
            .iter()
            .rposition(|t| matches!(t.kind, Kind::Impl(_)))
    }

    pub fn get_or_create_task(&mut self, code: TaskCode, linked_to_previous: bool) -> TaskId {
        if let Some(id) = self.existing_task(&code.id) {
            return id;
        }
        let previous = if linked_to_previous {
            self.last_created_task()
        } else {
            None
        };
        let mut imp = TaskImpl {
            completion: self.default_completion,
            ..TaskImpl::default()
        };
        imp.solver.set(
            Attr::Start,
            PVal::Time(TimePoint::of_start_of_day(self.min_day)),
        );
        imp.solver
            .set(Attr::Load, PVal::Load(self.cal.duration_of_days(1)));
        let id = self.put_task(Task {
            code,
            row: None,
            display: None,
            stereotype: None,
            kind: Kind::Impl(Box::new(imp)),
        });
        if let Some(Kind::Group { children, .. }) =
            self.current_group.map(|g| &mut self.tasks[g].kind)
        {
            children.push(id);
        }
        if let Some(prev) = previous {
            self.force_task_order(prev, id);
        }
        id
    }

    fn impl_of(&self, id: TaskId) -> Option<&TaskImpl> {
        self.tasks[id].as_impl()
    }

    pub fn impl_mut(&mut self, id: TaskId) -> Option<&mut TaskImpl> {
        let t = self.tasks[id].as_impl_mut()?;
        t.invalidate_cache();
        Some(t)
    }

    pub fn set_task_start(&mut self, id: TaskId, start: TimePoint) {
        if let Some(t) = self.impl_mut(id) {
            t.solver.set(Attr::Start, PVal::Time(start));
        }
    }

    pub fn set_task_end(&mut self, id: TaskId, end: TimePoint) {
        if let Some(t) = self.impl_mut(id) {
            t.solver.set(Attr::End, PVal::Time(end));
        }
    }

    pub fn set_task_load(&mut self, id: TaskId, seconds: i64) {
        if let Some(t) = self.impl_mut(id) {
            t.solver.set(Attr::Load, PVal::Load(seconds));
        }
    }

    pub fn force_task_order(&mut self, task1: TaskId, task2: TaskId) -> usize {
        let end1 = TaskInstant::new(Moment::Task(task1), Attr::End);
        let when = self.instant_precise(&end1);
        self.set_task_start(task2, when);
        self.constraints.push(GanttConstraint::new(
            end1,
            TaskInstant::new(Moment::Task(task2), Attr::Start),
            None,
        ));
        self.constraints.len() - 1
    }

    pub fn add_constraint(&mut self, c: GanttConstraint) {
        self.constraints.push(c);
    }

    pub fn add_separator(&mut self, comment: Option<String>) {
        let id = format!("##{}", self.tasks.len());
        self.put_task(Task {
            code: TaskCode::from_id(&id),
            row: None,
            display: None,
            stereotype: None,
            kind: Kind::Separator { comment },
        });
    }

    pub fn add_group(&mut self, code: TaskCode) -> CommandExecutionResult {
        let parent = self.current_group;
        let id = self.put_task(Task {
            code,
            row: None,
            display: None,
            stereotype: None,
            kind: Kind::Group {
                parent,
                children: Vec::new(),
            },
        });
        if let Some(Kind::Group { children, .. }) = parent.map(|g| &mut self.tasks[g].kind) {
            children.push(id);
        }
        self.current_group = Some(id);
        CommandExecutionResult::ok()
    }

    pub fn end_group(&mut self) -> CommandExecutionResult {
        let Some(g) = self.current_group else {
            return CommandExecutionResult::error("No group to be closed");
        };
        self.current_group = match &self.tasks[g].kind {
            Kind::Group { parent, .. } => *parent,
            _ => None,
        };
        CommandExecutionResult::ok()
    }

    pub fn delete_task(&mut self, id: TaskId) -> CommandExecutionResult {
        if let Some(t) = self.impl_mut(id) {
            t.colors = vec![CenterBorderColor::new(
                Some(HColor::WHITE),
                Some(HColor::BLACK),
            )];
        }
        CommandExecutionResult::ok()
    }

    pub fn add_note(
        &mut self,
        note: Display,
        stereotype: Option<Stereotype>,
    ) -> CommandExecutionResult {
        let Some(last) = self.tasks.len().checked_sub(1) else {
            return CommandExecutionResult::error("No task defined");
        };
        if let Some(t) = self.tasks[last].as_impl_mut() {
            t.note = Some(note);
            t.note_stereotype = stereotype;
        }
        CommandExecutionResult::ok()
    }

    pub fn set_task_default_completion(&mut self, v: i32) {
        self.default_completion = v;
    }

    pub fn set_it(&mut self, id: TaskId) {
        self.it = Some(id);
    }

    pub fn it(&self) -> Option<TaskId> {
        self.it
    }

    pub fn they(&self) -> Option<&str> {
        self.they.as_deref()
    }

    pub fn set_they(&mut self, name: &str) {
        self.they = Some(name.to_string());
    }

    // ---- resources ----

    pub fn get_resource(&mut self, name: &str) -> usize {
        if let Some(i) = self.resources.iter().position(|r| r.name == name) {
            return i;
        }
        self.resources.push(Resource::new(name));
        self.resources.len() - 1
    }

    /// `affectResource`: `name` or `name:percentage`; false when the share is zero.
    pub fn affect_resource(&mut self, task: TaskId, description: &str) -> bool {
        let (name, percentage) = match description.find(':') {
            Some(i) if i > 0 => {
                let rest = &description[i + 1..];
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if digits.is_empty() {
                    (description, 100)
                } else {
                    (&description[..i], digits.parse().unwrap_or(100))
                }
            }
            _ => (description, 100),
        };
        self.get_resource(name);
        if percentage == 0 {
            return false;
        }
        if let Some(t) = self.impl_mut(task) {
            match t.resources.iter_mut().find(|(r, _)| r == name) {
                Some(entry) => entry.1 = percentage,
                None => t.resources.push((name.to_string(), percentage)),
            }
        }
        true
    }

    fn resource_by_name(&self, name: &str) -> Option<&Resource> {
        self.resources.iter().find(|r| r.name == name)
    }

    // ---- moments and instants ----

    pub fn existing_moment(&self, id: &str) -> Option<Moment> {
        if let Some(t) = self.existing_task(id) {
            return Some(Moment::Task(t));
        }
        let mut start: Option<TimePoint> = None;
        let mut end: Option<TimePoint> = None;
        for (d, name) in &self.cal.name_days {
            if !name.eq_ignore_ascii_case(id) {
                continue;
            }
            start = Some(start.map_or(*d, |s| s.min(*d)));
            end = Some(end.map_or(*d, |e| e.max(*d)));
        }
        Some(Moment::Range(start?, end?.increment()))
    }

    pub fn moment_start(&self, m: Moment) -> TimePoint {
        match m {
            Moment::Task(t) => self.task_start(t),
            Moment::Range(s, _) => s,
        }
    }

    pub fn moment_end(&self, m: Moment) -> TimePoint {
        match m {
            Moment::Task(t) => self.task_end(t),
            Moment::Range(_, e) => e,
        }
    }

    pub fn instant_precise(&self, ti: &TaskInstant) -> TimePoint {
        let mut value = match ti.attribute {
            Attr::Start => self.moment_start(ti.moment),
            _ => self.moment_end(ti.moment),
        };
        if ti.delta > 0 {
            let mut added = 0;
            while added < ti.delta {
                value = value.increment();
                if ti.mode == ConstraintMode::DoNotCountCloseDay {
                    if let Some(cal) = &ti.calendar {
                        if is_zero_on_day(cal.as_ref(), value.to_day()) {
                            continue;
                        }
                    }
                }
                added += 1;
            }
        }
        if ti.delta < 0 {
            for _ in 0..-ti.delta {
                value = value.decrement();
            }
        }
        value
    }

    pub fn same_row_as(&self, a: &TaskInstant, b: &TaskInstant) -> bool {
        match (a.task(), b.task()) {
            (Some(t1), Some(t2)) => {
                self.tasks[t2].row == Some(t1) || self.tasks[t1].row == Some(t2)
            }
            _ => false,
        }
    }

    pub fn default_plan(&self) -> Pc {
        self.cal.open_close.as_piecewise_constant()
    }

    // ---- solving ----

    /// The calendar a task works with, before its own pauses (`getLoadPlanableForTask`).
    fn load_planable_for_task(&self, id: &str) -> Pc {
        self.cal
            .open_close
            .mutate_me(self.cal.open_close_for_task.get(id))
            .as_piecewise_constant()
    }

    fn local_pause(&self, t: &TaskImpl) -> Pc {
        let mut week = ByWeekday::of(Fraction::ONE);
        for d in &t.paused_dow {
            week = week.with(*d, Fraction::ZERO);
        }
        let week: Pc = Rc::new(week);
        if t.paused_days.is_empty() {
            return week;
        }
        let closed: HashMap<Day, Fraction> =
            t.paused_days.iter().map(|d| (*d, Fraction::ZERO)).collect();
        Combiner::product(vec![
            week,
            Rc::new(SpecificDays::with_days(Fraction::ONE, closed)),
        ])
    }

    pub fn task_pc(&self, id: TaskId) -> Pc {
        let Some(t) = self.impl_of(id) else {
            return self.default_plan();
        };
        if let Some(c) = t.cached_pc.borrow().as_ref() {
            return c.clone();
        }
        let default_plan = self.load_planable_for_task(&self.tasks[id].code.id);
        let result = if t.resources.is_empty() {
            Combiner::product(vec![default_plan, self.local_pause(t)])
        } else {
            let contributions: Vec<Pc> = t
                .resources
                .iter()
                .map(|(name, pct)| {
                    let availability = self.resource_by_name(name).map_or_else(
                        || OpenClose::new().as_piecewise_constant(),
                        |r| r.open_close.as_piecewise_constant(),
                    );
                    let share: Pc = Rc::new(SpecificDays::of(Fraction::new(i64::from(*pct), 100)));
                    Combiner::product(vec![availability, share])
                })
                .collect();
            Combiner::product(vec![
                default_plan,
                Combiner::product(vec![self.local_pause(t), Combiner::sum(contributions)]),
            ])
        };
        *t.cached_pc.borrow_mut() = Some(result.clone());
        result
    }

    /// The calendar a task is drawn with (`getDefaultPlan` of `TaskImpl`).
    pub fn task_default_plan(&self, id: TaskId) -> Pc {
        self.load_planable_for_task(&self.tasks[id].code.id)
    }

    fn solved(&self, id: TaskId, attr: Attr) -> Option<PVal> {
        self.impl_of(id)?.solver.get(attr)
    }

    pub fn task_load(&self, id: TaskId) -> i64 {
        match self.solved(id, Attr::Load) {
            Some(PVal::Load(l)) => l,
            _ => SECS_PER_DAY,
        }
    }

    pub fn task_start(&self, id: TaskId) -> TimePoint {
        match &self.tasks[id].kind {
            Kind::Group { children, .. } => children
                .iter()
                .map(|c| self.task_start(*c))
                .min()
                .unwrap_or_else(|| TimePoint::of_start_of_day(self.min_day)),
            Kind::Separator { .. } => TimePoint::of_start_of_day(self.min_day),
            Kind::Impl(t) => {
                if let Some(s) = t.cached_start.get() {
                    return s;
                }
                let pc = self.task_pc(id);
                let mut result = match self.solved(id, Attr::Start) {
                    Some(PVal::Time(s)) => s,
                    _ => {
                        let end = match self.solved(id, Attr::End) {
                            Some(PVal::Time(e)) => e,
                            _ => TimePoint::of_start_of_day(self.min_day),
                        };
                        TimePoint(integrate(
                            pc.as_ref(),
                            self.task_load(id),
                            end.secs(),
                            Dir::Backward,
                        ))
                    }
                };
                if !t.diamond {
                    let mut guard = 0;
                    while is_zero_on_day(pc.as_ref(), result.to_day()) && guard < 100_000 {
                        result = result.increment();
                        guard += 1;
                    }
                }
                t.cached_start.set(Some(result));
                result
            }
        }
    }

    pub fn task_end(&self, id: TaskId) -> TimePoint {
        match &self.tasks[id].kind {
            Kind::Group { children, .. } => children
                .iter()
                .map(|c| self.task_end(*c))
                .max()
                .unwrap_or_else(|| TimePoint::of_start_of_day(self.min_day)),
            Kind::Separator { .. } => TimePoint::of_start_of_day(self.min_day),
            Kind::Impl(t) => {
                if let Some(e) = t.cached_end.get() {
                    return e;
                }
                let result = match self.solved(id, Attr::End) {
                    Some(PVal::Time(e)) => e,
                    _ => {
                        let start = match self.solved(id, Attr::Start) {
                            Some(PVal::Time(s)) => s,
                            _ => TimePoint::of_start_of_day(self.min_day),
                        };
                        TimePoint(integrate(
                            self.task_pc(id).as_ref(),
                            self.task_load(id),
                            start.secs(),
                            Dir::Forward,
                        ))
                    }
                };
                t.cached_end.set(Some(result));
                result
            }
        }
    }

    pub fn task_end_minus_one_day(&self, id: TaskId) -> TimePoint {
        self.task_end(id).decrement()
    }

    /// Days a task pauses, by date or by day of the week (`getAllPaused`).
    pub fn all_paused(&self, id: TaskId) -> Vec<Day> {
        let Some(t) = self.impl_of(id) else {
            return Vec::new();
        };
        let mut result: std::collections::BTreeSet<Day> = t.paused_days.clone();
        if !t.paused_dow.is_empty() {
            let end = self.task_end_minus_one_day(id);
            let mut cur = self.task_start(id);
            while cur <= end {
                if t.paused_dow.contains(&cur.to_day_of_week()) {
                    result.insert(cur.to_day());
                }
                cur = cur.increment();
            }
        }
        result.into_iter().collect()
    }

    /// Load of a resource on an instant (`loadForResource`).
    pub fn load_for_resource(&self, id: TaskId, res: &str, instant: TimePoint) -> i32 {
        let Some(t) = self.impl_of(id) else {
            return 0;
        };
        let Some((_, pct)) = t.resources.iter().find(|(r, _)| r == res) else {
            return 0;
        };
        if instant < self.task_start(id) || instant > self.task_end_minus_one_day(id) {
            return 0;
        }
        if t.paused_days.contains(&instant.to_day())
            || t.paused_dow.contains(&instant.to_day_of_week())
        {
            return 0;
        }
        if self
            .resource_by_name(res)
            .is_some_and(|r| r.open_close.is_closed(instant.to_day()))
        {
            return 0;
        }
        *pct
    }

    pub fn load_for_resource_total(&self, res: &str, instant: TimePoint) -> i32 {
        (0..self.tasks.len())
            .map(|i| self.load_for_resource(i, res, instant))
            .sum()
    }

    pub fn constraints_for_task(&self, id: TaskId) -> Vec<&GanttConstraint> {
        self.constraints.iter().filter(|c| c.is_on(id)).collect()
    }

    // ---- bounds ----

    /// `TimeBoundsData.initMinMax`: the last day any task or colored day reaches.
    pub fn computed_max_day(&self) -> Day {
        let mut max: Option<Day> = if self.tasks.is_empty() {
            Some(self.min_day)
        } else {
            (0..self.tasks.len())
                .filter(|i| matches!(self.tasks[*i].kind, Kind::Impl(_)))
                .map(|i| self.task_end(i).minus_one_second().to_day())
                .max()
        };
        for d in self.cal.color_days.keys().chain(self.cal.name_days.keys()) {
            if max.is_none_or(|m| d.to_day() > m) {
                max = Some(d.to_day());
            }
        }
        max.unwrap_or(self.min_day)
    }

    /// The displayed days: the printed interval or everything (`getTextBlock`).
    pub fn bounds(&self) -> Bounds {
        match (self.print_start, self.print_end) {
            (Some(s), Some(e)) => Bounds {
                min_day: s,
                max_day: e,
                printing: true,
            },
            _ => Bounds {
                min_day: self.min_day,
                max_day: self.computed_max_day(),
                printing: false,
            },
        }
    }

    pub fn start_for_drawing(&self, id: TaskId, b: &Bounds) -> TimePoint {
        let start = self.task_start(id);
        if b.printing {
            start.max(TimePoint::of_start_of_day(b.min_day))
        } else {
            start
        }
    }

    pub fn end_for_drawing(&self, id: TaskId, b: &Bounds) -> TimePoint {
        let end = self.task_end(id);
        if b.printing {
            end.min(TimePoint::of_start_of_day(b.max_day + 1))
        } else {
            end
        }
    }

    pub fn is_hidden(&self, id: TaskId, b: &Bounds) -> bool {
        if !b.printing || matches!(self.tasks[id].kind, Kind::Separator { .. }) {
            return false;
        }
        if self.task_end_minus_one_day(id) < TimePoint::of_start_of_day(b.min_day) {
            return true;
        }
        self.task_start(id) > TimePoint::of_end_of_day_minus_one_second(b.max_day)
    }
}

/// First and last displayed days (`TimeBoundsData`).
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_day: Day,
    pub max_day: Day,
    pub printing: bool,
}

fn todays_midnight() -> TimePoint {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        TimePoint(secs).floor_to_day()
    }
    #[cfg(target_arch = "wasm32")]
    {
        TimePoint(0)
    }
}

impl Diagram for GanttDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        render::main_block(self, sb, skin)
    }

    fn default_margins(&self) -> Margins {
        Margins::same(0.0)
    }
}

impl HasBase for GanttDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for GanttDiagram {
    fn check_final_error(&self) -> Option<String> {
        if self.computed_max_day() - self.min_day > 365 * 20 {
            return Some("Gantt diagrams cannot last more than 20 years".into());
        }
        None
    }
}

pub struct GanttDiagramFactory;

impl DiagramFactory for GanttDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Gantt
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            GanttDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
