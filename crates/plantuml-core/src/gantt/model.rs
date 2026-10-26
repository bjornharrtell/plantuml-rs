//! Tasks, resources and constraints of a Gantt diagram (`gantt/core`, `gantt/solver`).

use super::calendar::{OpenClose, Pc};
use super::time::{Day, Dow, TimePoint};
use crate::klimt::color::{HColor, TransparentFillBehavior};
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::UStroke;
use crate::stereo::Stereotype;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

pub type TaskId = usize;

/// Identifier and displayed name of a task (`TaskCode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCode {
    pub id: String,
    pub display: String,
}

impl TaskCode {
    pub fn from_id(id: &str) -> TaskCode {
        TaskCode {
            id: id.to_string(),
            display: id.to_string(),
        }
    }

    pub fn from_id_and_display(id: Option<&str>, display: Option<&str>) -> Option<TaskCode> {
        let id = id.or(display)?;
        match display {
            None => Some(TaskCode::from_id(id)),
            Some(d) => Some(TaskCode {
                id: id.to_string(),
                display: d.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Start,
    End,
    Load,
}

/// A value held by the solver (`PValue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PVal {
    Time(TimePoint),
    /// An effort in seconds (`Load`).
    Load(i64),
}

/// Keeps the last two things said about a task (`Solver`): two of start, end and load.
#[derive(Debug, Clone, Default)]
pub struct Solver {
    values: Vec<(Attr, PVal)>,
}

impl Solver {
    pub fn set(&mut self, attr: Attr, mut value: PVal) {
        let previous = self
            .values
            .iter()
            .position(|(a, _)| *a == attr)
            .map(|i| self.values.remove(i).1);
        if let (Some(PVal::Time(p)), PVal::Time(v), Attr::Start) = (previous, value, attr) {
            if p > v {
                value = PVal::Time(p);
            }
        }
        self.values.push((attr, value));
        if self.values.len() > 2 {
            self.values.remove(0);
        }
    }

    pub fn get(&self, attr: Attr) -> Option<PVal> {
        self.values
            .iter()
            .find(|(a, _)| *a == attr)
            .map(|(_, v)| *v)
    }
}

/// A fill and a border color, with an optional line style (`CenterBorderColor`).
#[derive(Debug, Clone, PartialEq)]
pub struct CenterBorderColor {
    pub center: Option<HColor>,
    pub border: Option<HColor>,
    pub style: Option<String>,
}

impl CenterBorderColor {
    pub fn new(center: Option<HColor>, border: Option<HColor>) -> Self {
        Self {
            center,
            border,
            style: None,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.center.is_some()
    }

    pub fn unlinear_to(&self, other: &CenterBorderColor, completion: i32) -> CenterBorderColor {
        CenterBorderColor {
            center: unlinear_opt(&self.center, &other.center, completion),
            border: unlinear_opt(&self.border, &other.border, completion),
            style: self.style.clone(),
        }
    }
}

fn unlinear_opt(a: &Option<HColor>, b: &Option<HColor>, completion: i32) -> Option<HColor> {
    match (a, b) {
        (Some(a), Some(b)) => Some(unlinear(a, b, completion)),
        _ => a.clone(),
    }
}

/// A color between two others, with a cubic ease (`HColors.unlinear`).
pub fn unlinear(c1: &HColor, c2: &HColor, completion: i32) -> HColor {
    if completion == 0 {
        return c1.clone();
    }
    if completion == 100 {
        return c2.clone();
    }
    match (c1, c2) {
        (HColor::Simple { color: a, .. }, HColor::Simple { color: b, .. }) => {
            let completion = completion.min(100) as f32 / 100.0;
            let f = completion * completion * completion;
            let (h1, h2) = (a.to_hsl(), b.to_hsl());
            let lin = |x: f32, y: f32| x + (y - x) * f;
            let rgb = crate::klimt::color::Rgba::from_hsl([
                lin(h1[0], h2[0]),
                lin(h1[1], h2[1]),
                lin(h1[2], h2[2]),
            ]);
            HColor::Simple {
                color: rgb,
                dark: None,
                fill: TransparentFillBehavior::WithFillNone,
            }
        }
        _ => c1.clone(),
    }
}

/// Plain, dotted, dashed or bold arrows of the links between tasks (`LinkStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LinkStyle {
    pub kind: LinkKind,
    pub thickness: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkKind {
    #[default]
    Normal,
    Dashed,
    Dotted,
    Bold,
}

impl LinkStyle {
    fn stroke3(&self) -> UStroke {
        let t = self.thickness.unwrap_or(1.0);
        match self.kind {
            LinkKind::Dashed => UStroke::new(7.0, 7.0, t),
            LinkKind::Dotted => UStroke::new(1.0, 3.0, t),
            LinkKind::Bold => UStroke::with_thickness(2.0),
            LinkKind::Normal => UStroke::with_thickness(t),
        }
    }

    /// `LinkType.getStroke3`: the stroke when the style has the default one.
    pub fn stroke_over(&self, default: UStroke) -> UStroke {
        if self.thickness.is_some() {
            return self.stroke3();
        }
        if default.dash_visible == 0.0 && default.dash_space == 0.0 {
            return LinkStyle {
                thickness: Some(default.thickness),
                ..*self
            }
            .stroke3();
        }
        default
    }
}

/// Which side of a bar an arrow starts or ends on (`GSide`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GSide {
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl GSide {
    pub fn is_bottom(self) -> bool {
        matches!(self, GSide::BottomLeft | GSide::BottomRight)
    }

    pub fn is_top(self) -> bool {
        matches!(self, GSide::TopLeft | GSide::TopRight)
    }

    pub fn reverse_bottom_top(self) -> GSide {
        match self {
            GSide::BottomLeft => GSide::TopLeft,
            _ => GSide::TopRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowType {
    Incoming,
    Outgoing,
}

/// A task start/end or a named period (`Moment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    Task(TaskId),
    Range(TimePoint, TimePoint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintMode {
    IgnoreCalendar,
    DoNotCountCloseDay,
}

/// The start or the end of a moment, possibly shifted by some days (`TaskInstant`).
#[derive(Clone)]
pub struct TaskInstant {
    pub moment: Moment,
    pub attribute: Attr,
    pub delta: i32,
    pub mode: ConstraintMode,
    pub calendar: Option<Pc>,
}

impl TaskInstant {
    pub fn new(moment: Moment, attribute: Attr) -> TaskInstant {
        TaskInstant {
            moment,
            attribute,
            delta: 0,
            mode: ConstraintMode::IgnoreCalendar,
            calendar: None,
        }
    }

    pub fn with_delta(&self, delta: i32, mode: ConstraintMode, calendar: Pc) -> TaskInstant {
        TaskInstant {
            moment: self.moment,
            attribute: self.attribute,
            delta,
            mode,
            calendar: Some(calendar),
        }
    }

    pub fn is_task(&self) -> bool {
        matches!(self.moment, Moment::Task(_))
    }

    pub fn task(&self) -> Option<TaskId> {
        match self.moment {
            Moment::Task(t) => Some(t),
            Moment::Range(..) => None,
        }
    }
}

/// An arrow between two task instants (`GanttConstraint`).
#[derive(Clone)]
pub struct GanttConstraint {
    pub source: TaskInstant,
    pub dest: TaskInstant,
    pub forced_color: Option<HColor>,
    pub style: LinkStyle,
}

impl GanttConstraint {
    pub fn new(source: TaskInstant, dest: TaskInstant, forced_color: Option<HColor>) -> Self {
        Self {
            source,
            dest,
            forced_color,
            style: LinkStyle::default(),
        }
    }

    pub fn is_on(&self, task: TaskId) -> bool {
        self.source.task() == Some(task) || self.dest.task() == Some(task)
    }

    pub fn is_there_right_arrow(&self, task: TaskId) -> bool {
        (self.dest.task() == Some(task) && self.dest.attribute == Attr::End)
            || (self.source.task() == Some(task)
                && self.dest.attribute == Attr::End
                && self.source.attribute == Attr::End)
    }

    /// `WithLinkType.applyStyle`: `dashed`, `bold`, `dotted`, colors and `thickness=n`.
    pub fn apply_style(&mut self, arrow_style: Option<&str>) {
        let Some(style) = arrow_style else { return };
        for (i, part) in style.split(';').filter(|p| !p.is_empty()).enumerate() {
            for s in part.split(',').filter(|p| !p.is_empty()) {
                match s.to_ascii_lowercase().as_str() {
                    "dashed" => self.style.kind = LinkKind::Dashed,
                    "bold" => self.style.kind = LinkKind::Bold,
                    "dotted" => self.style.kind = LinkKind::Dotted,
                    "hidden" | "single" | "plain" | "node" | "norank" => {}
                    other => {
                        if let Some(t) = other.strip_prefix("thickness=") {
                            self.style.thickness = t.parse().ok();
                        } else if i == 0 {
                            self.forced_color = Some(HColor::parse_or_white(s));
                        }
                    }
                }
            }
        }
    }
}

/// A person or machine tasks can be assigned to (`Resource`).
#[derive(Clone)]
pub struct Resource {
    pub name: String,
    pub open_close: OpenClose,
}

impl Resource {
    pub fn new(name: &str) -> Resource {
        Resource {
            name: name.to_string(),
            open_close: OpenClose::new(),
        }
    }
}

/// The data specific to an ordinary task (`TaskImpl`).
#[derive(Default)]
pub struct TaskImpl {
    pub paused_days: BTreeSet<Day>,
    pub paused_dow: BTreeSet<Dow>,
    pub solver: Solver,
    /// Resources and the percentage of their time (insertion ordered).
    pub resources: Vec<(String, i32)>,
    pub diamond: bool,
    pub completion: i32,
    pub note: Option<Display>,
    pub note_stereotype: Option<Stereotype>,
    pub colors: Vec<CenterBorderColor>,
    pub cached_pc: RefCell<Option<Pc>>,
    pub cached_start: Cell<Option<TimePoint>>,
    pub cached_end: Cell<Option<TimePoint>>,
}

impl TaskImpl {
    pub fn invalidate_cache(&self) {
        *self.cached_pc.borrow_mut() = None;
        self.cached_start.set(None);
        self.cached_end.set(None);
    }

    pub fn is_assigned_to(&self, res: &str) -> bool {
        self.resources.iter().any(|(r, _)| r == res)
    }

    pub fn colors(&self) -> Option<CenterBorderColor> {
        match self.colors.len() {
            0 => None,
            1 => Some(self.colors[0].clone()),
            _ => Some(self.colors[0].unlinear_to(&self.colors[1], self.completion)),
        }
    }
}

pub enum Kind {
    Impl(Box<TaskImpl>),
    Group {
        parent: Option<TaskId>,
        children: Vec<TaskId>,
    },
    Separator {
        comment: Option<String>,
    },
}

/// Anything on a row of the chart (`Task`).
pub struct Task {
    pub code: TaskCode,
    pub row: Option<TaskId>,
    pub display: Option<String>,
    pub stereotype: Option<Stereotype>,
    pub kind: Kind,
}

impl Task {
    pub fn as_impl(&self) -> Option<&TaskImpl> {
        match &self.kind {
            Kind::Impl(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_impl_mut(&mut self) -> Option<&mut TaskImpl> {
        match &mut self.kind {
            Kind::Impl(t) => Some(t),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_keeps_two_values() {
        let mut s = Solver::default();
        s.set(Attr::Start, PVal::Time(TimePoint(100)));
        s.set(Attr::Load, PVal::Load(10));
        s.set(Attr::End, PVal::Time(TimePoint(200)));
        assert_eq!(s.get(Attr::Start), None);
        assert_eq!(s.get(Attr::End), Some(PVal::Time(TimePoint(200))));
        s.set(Attr::Start, PVal::Time(TimePoint(50)));
        s.set(Attr::Start, PVal::Time(TimePoint(20)));
        assert_eq!(s.get(Attr::Start), Some(PVal::Time(TimePoint(50))));
    }

    #[test]
    fn colors_blend_with_completion() {
        let a = HColor::parse_or_white("#000000");
        let b = HColor::parse_or_white("#ffffff");
        assert_eq!(unlinear(&a, &b, 0), a);
        assert_eq!(unlinear(&a, &b, 100), b);
        let mid = unlinear(&a, &b, 50);
        assert_ne!(mid, a);
        let c1 = CenterBorderColor::new(Some(a.clone()), None);
        let c2 = CenterBorderColor::new(Some(b), None);
        assert!(c1.unlinear_to(&c2, 50).is_ok());
        let mut t = TaskImpl {
            completion: 100,
            colors: vec![c1, c2.clone()],
            ..TaskImpl::default()
        };
        assert_eq!(t.colors().unwrap().center, c2.center);
        t.colors.truncate(1);
        assert!(t.colors().is_some());
        t.invalidate_cache();
        assert!(!t.is_assigned_to("x"));
    }

    #[test]
    fn link_style_strokes() {
        let mut c = GanttConstraint::new(
            TaskInstant::new(Moment::Task(0), Attr::End),
            TaskInstant::new(Moment::Task(1), Attr::Start),
            None,
        );
        c.apply_style(Some("dashed,#red;thickness=3"));
        assert_eq!(c.style.kind, LinkKind::Dashed);
        assert!(c.forced_color.is_some());
        let s = c.style.stroke_over(UStroke::with_thickness(1.5));
        assert_eq!(s.thickness, 3.0);
        let plain = LinkStyle::default().stroke_over(UStroke::with_thickness(1.5));
        assert_eq!(plain.thickness, 1.5);
        assert!(c.is_on(0) && !c.is_there_right_arrow(1));
        assert_eq!(GSide::BottomRight.reverse_bottom_top(), GSide::TopRight);
        assert!(GSide::TopLeft.is_top() && GSide::BottomLeft.is_bottom());
        assert_eq!(
            TaskCode::from_id_and_display(None, Some("a")).unwrap().id,
            "a"
        );
        assert!(TaskCode::from_id_and_display(None, None).is_none());
    }
}
