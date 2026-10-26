//! Working calendars as piecewise constant functions of time (`gantt/ngm/math`, `OpenClose`).
//!
//! Instants are seconds since 1970-01-01T00:00, like [`TimePoint`](super::time::TimePoint).

use super::time::{Day, Dow, SECS_PER_DAY};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// `LocalDateTime.MIN` and `MAX`, far enough to never be reached but safe from overflow.
pub const MIN_TIME: i64 = -(1 << 52);
pub const MAX_TIME: i64 = 1 << 52;

/// An exact rational number (`Fraction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fraction {
    num: i128,
    den: i128,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Fraction {
    pub const ZERO: Fraction = Fraction { num: 0, den: 1 };
    pub const ONE: Fraction = Fraction { num: 1, den: 1 };

    pub fn new(num: i64, den: i64) -> Fraction {
        let (mut n, mut d) = (i128::from(num), i128::from(den));
        if d == 0 {
            d = 1;
        }
        if d < 0 {
            n = -n;
            d = -d;
        }
        let g = gcd(n.abs(), d).max(1);
        Fraction {
            num: n / g,
            den: d / g,
        }
    }

    fn from_128(n: i128, d: i128) -> Fraction {
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd(n.abs(), d).max(1);
        Fraction {
            num: n / g,
            den: d / g,
        }
    }

    pub fn add(self, o: Fraction) -> Fraction {
        Self::from_128(self.num * o.den + o.num * self.den, self.den * o.den)
    }

    pub fn subtract(self, o: Fraction) -> Fraction {
        Self::from_128(self.num * o.den - o.num * self.den, self.den * o.den)
    }

    pub fn multiply(self, o: Fraction) -> Fraction {
        Self::from_128(self.num * o.num, self.den * o.den)
    }

    pub fn multiply_by_long(self, k: i64) -> Fraction {
        Self::from_128(self.num * i128::from(k), self.den)
    }

    pub fn divide(self, o: Fraction) -> Fraction {
        Self::from_128(self.num * o.den, self.den * o.num)
    }

    pub fn whole_part(self) -> i64 {
        (self.num / self.den) as i64
    }

    pub fn is_zero(self) -> bool {
        self.num == 0
    }

    fn compare(self, o: Fraction) -> std::cmp::Ordering {
        (self.num * o.den).cmp(&(o.num * self.den))
    }
}

/// How two values are combined (`Fraction.PRODUCT` and friends).
#[derive(Debug, Clone, Copy)]
pub enum Op {
    Product,
    Sum,

    Max,
}

impl Op {
    fn apply(self, a: Fraction, b: Fraction) -> Fraction {
        match self {
            Op::Product => a.multiply(b),
            Op::Sum => a.add(b),
            Op::Max => {
                if a.compare(b).is_ge() {
                    a
                } else {
                    b
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Forward,
    Backward,
}

/// A stretch of time where a function is constant (`Segment`). Going backward, `start` is
/// after `end`.
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub start: i64,
    pub end: i64,
    pub value: Fraction,
    pub dir: Dir,
}

impl Segment {
    pub fn forward(start: i64, end: i64, value: Fraction) -> Segment {
        Segment {
            start,
            end,
            value,
            dir: Dir::Forward,
        }
    }

    pub fn backward(start: i64, end: i64, value: Fraction) -> Segment {
        Segment {
            start,
            end,
            value,
            dir: Dir::Backward,
        }
    }

    fn intersection(segments: &[Segment], op: Op) -> Segment {
        let dir = segments[0].dir;
        let (mut start, mut end, mut value) =
            (segments[0].start, segments[0].end, segments[0].value);
        for s in &segments[1..] {
            if dir == Dir::Forward {
                start = start.max(s.start);
                end = end.min(s.end);
            } else {
                start = start.min(s.start);
                end = end.max(s.end);
            }
            value = op.apply(value, s.value);
        }
        Segment {
            start,
            end,
            value,
            dir,
        }
    }

    fn clamped_start(&self, current: i64) -> i64 {
        if self.dir == Dir::Forward {
            current.max(self.start)
        } else {
            current.min(self.start)
        }
    }
}

/// A function of time that is constant on segments (`PiecewiseConstant`).
pub trait PiecewiseConstant {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment;
}

pub type Pc = Rc<dyn PiecewiseConstant>;

/// Upper bound on the segments walked in a row (`Infinite loop detected`).
const MAX_SEGMENTS: usize = 9999;

/// Walks the segments from `instant`, calling `f` until it returns false.
pub fn iterate_segments(
    pc: &dyn PiecewiseConstant,
    instant: i64,
    dir: Dir,
    mut f: impl FnMut(&Segment) -> bool,
) {
    let mut current = instant;
    for _ in 0..MAX_SEGMENTS {
        let seg = pc.segment_at(current, dir);
        current = seg.end;
        if !f(&seg) {
            return;
        }
    }
    log::warn!("infinite loop detected in piecewise constant iteration");
}

/// `PiecewiseConstantUtils.isZeroOnDay`.
pub fn is_zero_on_day(pc: &dyn PiecewiseConstant, day: Day) -> bool {
    let next = (day + 1) * SECS_PER_DAY;
    let mut result = true;
    iterate_segments(pc, day * SECS_PER_DAY, Dir::Forward, |seg| {
        if seg.start >= next {
            return false;
        }
        if !seg.value.is_zero() {
            result = false;
            return false;
        }
        true
    });
    result
}

/// One value per day of the week (`PiecewiseConstantWeekday`).
#[derive(Debug, Clone)]
pub struct ByWeekday([Fraction; 7]);

impl ByWeekday {
    pub fn of(same: Fraction) -> ByWeekday {
        ByWeekday([same; 7])
    }

    pub fn with(mut self, day: Dow, value: Fraction) -> ByWeekday {
        self.0[day.index()] = value;
        self
    }
}

impl PiecewiseConstant for ByWeekday {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment {
        let midnight = instant.div_euclid(SECS_PER_DAY) * SECS_PER_DAY;
        if dir == Dir::Forward {
            let value = self.0[Dow::of_day(midnight / SECS_PER_DAY).index()];
            return Segment::forward(midnight, midnight + SECS_PER_DAY, value);
        }
        let end = if instant == midnight {
            midnight - SECS_PER_DAY
        } else {
            midnight
        };
        let value = self.0[Dow::of_day(end / SECS_PER_DAY).index()];
        Segment::backward(end + SECS_PER_DAY, end, value)
    }
}

/// A default value, with exceptions on some dates (`PiecewiseConstantSpecificDays`).
#[derive(Debug, Clone)]
pub struct SpecificDays {
    default: Fraction,
    days: HashMap<Day, Fraction>,
}

impl SpecificDays {
    pub fn of(default: Fraction) -> SpecificDays {
        SpecificDays {
            default,
            days: HashMap::new(),
        }
    }

    pub fn with_days(default: Fraction, days: HashMap<Day, Fraction>) -> SpecificDays {
        SpecificDays { default, days }
    }

    fn value(&self, day: Day) -> Fraction {
        self.days.get(&day).copied().unwrap_or(self.default)
    }
}

impl PiecewiseConstant for SpecificDays {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment {
        if dir == Dir::Forward {
            let day = instant.div_euclid(SECS_PER_DAY);
            let start = day * SECS_PER_DAY;
            return Segment::forward(start, start + SECS_PER_DAY, self.value(day));
        }
        let mut day = instant.div_euclid(SECS_PER_DAY);
        if instant == day * SECS_PER_DAY {
            day -= 1;
        }
        let end = day * SECS_PER_DAY;
        Segment::backward(end + SECS_PER_DAY, end, self.value(day))
    }
}

/// One for a time window, zero outside (`PiecewiseConstantTimeWindow`).
#[derive(Debug, Clone)]
pub struct TimeWindow {
    off_before: i64,
    off_after: i64,
}

impl TimeWindow {
    pub fn new(off_before: i64, off_after: i64) -> TimeWindow {
        TimeWindow {
            off_before,
            off_after,
        }
    }
}

impl PiecewiseConstant for TimeWindow {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment {
        if dir == Dir::Forward {
            if instant < self.off_before {
                return Segment::forward(instant, self.off_before, Fraction::ZERO);
            }
            if instant >= self.off_after {
                return Segment::forward(instant, MAX_TIME, Fraction::ZERO);
            }
            return Segment::forward(instant, self.off_after, Fraction::ONE);
        }
        if instant > self.off_after {
            return Segment::backward(instant, self.off_after, Fraction::ZERO);
        }
        if instant <= self.off_before {
            return Segment::backward(instant, MIN_TIME, Fraction::ZERO);
        }
        Segment::backward(instant, self.off_before, Fraction::ONE)
    }
}

/// A start and an end time in a day, in seconds (`TimeRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

impl TimeRange {
    /// From `h:mm` texts; `None` when they are not times.
    pub fn parse(start: &str, end: &str) -> Option<TimeRange> {
        Some(TimeRange {
            start: parse_time(start)?,
            end: parse_time(end)?,
        })
    }

    pub fn duration(&self) -> i64 {
        self.end - self.start
    }
}

fn parse_time(s: &str) -> Option<i64> {
    let (h, m) = s.split_once(':')?;
    let (h, m) = (h.trim().parse::<i64>().ok()?, m.trim().parse::<i64>().ok()?);
    ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 3600 + m * 60)
}

/// Working hours of a day (`PiecewiseConstantHours`).
#[derive(Debug, Clone)]
pub struct Hours {
    default: Fraction,
    boundaries: BTreeMap<i64, Fraction>,
}

impl Hours {
    pub fn from_working_hours(ranges: &[TimeRange]) -> Hours {
        let mut result = Hours {
            default: Fraction::ZERO,
            boundaries: BTreeMap::new(),
        };
        for r in ranges {
            result.boundaries.insert(r.start, Fraction::ONE);
            if r.end != 0 {
                result.boundaries.insert(r.end, result.default);
            }
        }
        result
    }
}

impl PiecewiseConstant for Hours {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment {
        let day_start = instant.div_euclid(SECS_PER_DAY) * SECS_PER_DAY;
        let t = instant - day_start;
        let higher = |k: i64| self.boundaries.range(k + 1..).next().map(|(k, _)| *k);
        if dir == Dir::Forward {
            let (start_key, value) = match self.boundaries.range(..=t).next_back() {
                Some((k, v)) => (*k, *v),
                None => (0, self.default),
            };
            let end = higher(start_key).map_or(day_start + SECS_PER_DAY, |k| day_start + k);
            return Segment::forward(day_start + start_key, end, value);
        }
        if t == 0 {
            let prev = day_start - SECS_PER_DAY;
            let (start_key, value) = match self.boundaries.iter().next_back() {
                Some((k, v)) => (*k, *v),
                None => (0, self.default),
            };
            return Segment::backward(prev + SECS_PER_DAY, prev + start_key, value);
        }
        let (start_key, value) = match self.boundaries.range(..t).next_back() {
            Some((k, v)) => (*k, *v),
            None => (0, self.default),
        };
        let start = higher(start_key).map_or(day_start + SECS_PER_DAY, |k| day_start + k);
        Segment::backward(start, day_start + start_key, value)
    }
}

/// Several functions combined value by value (`Combiner`).
pub struct Combiner {
    elements: Vec<Pc>,
    op: Op,
}

impl Combiner {
    fn build(op: Op, elements: Vec<Pc>) -> Pc {
        if elements.len() == 1 {
            return elements[0].clone();
        }
        Rc::new(Combiner { elements, op })
    }

    pub fn sum(elements: Vec<Pc>) -> Pc {
        Self::build(Op::Sum, elements)
    }

    pub fn max(elements: Vec<Pc>) -> Pc {
        Self::build(Op::Max, elements)
    }

    pub fn product(elements: Vec<Pc>) -> Pc {
        Self::build(Op::Product, elements)
    }
}

impl PiecewiseConstant for Combiner {
    fn segment_at(&self, instant: i64, dir: Dir) -> Segment {
        let segments: Vec<Segment> = self
            .elements
            .iter()
            .map(|e| e.segment_at(instant, dir))
            .collect();
        Segment::intersection(&segments, self.op)
    }
}

/// Where a task of a given effort (in seconds) ends or starts (`LoadIntegrator`).
pub fn integrate(
    load_function: &dyn PiecewiseConstant,
    total_load: i64,
    start: i64,
    dir: Dir,
) -> i64 {
    let mut remaining = Fraction::new(total_load, 1);
    let mut current = start;
    let mut result = None;
    iterate_segments(load_function, start, dir, |seg| {
        if remaining == Fraction::ZERO {
            return false;
        }
        let rate = seg.value;
        if rate == Fraction::ZERO {
            current = seg.end;
            return true;
        }
        let effective = seg.clamped_start(current);
        let seconds = (seg.end - effective).abs();
        let load = rate.multiply_by_long(seconds);
        if load.compare(remaining).is_ge() {
            let needed = remaining.divide(rate).whole_part();
            result = Some(if dir == Dir::Forward {
                effective + needed
            } else {
                effective - needed
            });
            return false;
        }
        remaining = remaining.subtract(load);
        current = seg.end;
        true
    });
    result.unwrap_or(current)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DayStatus {
    Open,
    Close,
}

/// Open and closed days (`OpenClose`).
#[derive(Clone, Default)]
pub struct OpenClose {
    weekday: BTreeMap<Dow, DayStatus>,
    days: HashMap<Day, DayStatus>,
    working_time_ranges: Vec<TimeRange>,
    off_before: Option<Day>,
    off_after: Option<Day>,
    cached: RefCell<Option<Pc>>,
}

impl std::fmt::Debug for OpenClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenClose").finish_non_exhaustive()
    }
}

impl OpenClose {
    pub fn new() -> OpenClose {
        OpenClose::default()
    }

    /// A copy of this calendar overridden by `except` (`mutateMe`).
    pub fn mutate_me(&self, except: Option<&OpenClose>) -> OpenClose {
        let mut r = OpenClose {
            weekday: self.weekday.clone(),
            days: self.days.clone(),
            working_time_ranges: self.working_time_ranges.clone(),
            off_before: self.off_before,
            off_after: self.off_after,
            cached: RefCell::new(None),
        };
        if let Some(e) = except {
            r.weekday.extend(e.weekday.iter().map(|(k, v)| (*k, *v)));
            r.days.extend(e.days.iter().map(|(k, v)| (*k, *v)));
        }
        r
    }

    pub fn days_in_week(&self) -> i64 {
        7 - self
            .weekday
            .values()
            .filter(|s| **s == DayStatus::Close)
            .count() as i64
    }

    fn local_status(&self, day: Day) -> Option<DayStatus> {
        if self.off_before.is_some_and(|b| day < b) {
            return Some(DayStatus::Close);
        }
        if self.off_after.is_some_and(|a| day > a) {
            return Some(DayStatus::Close);
        }
        self.days
            .get(&day)
            .or_else(|| self.weekday.get(&Dow::of_day(day)))
            .copied()
    }

    pub fn is_closed(&self, day: Day) -> bool {
        self.local_status(day) == Some(DayStatus::Close)
    }

    pub fn close_dow(&mut self, d: Dow) {
        self.weekday.insert(d, DayStatus::Close);
        *self.cached.borrow_mut() = None;
    }

    pub fn open_dow(&mut self, d: Dow) {
        self.weekday.insert(d, DayStatus::Open);
        *self.cached.borrow_mut() = None;
    }

    pub fn close_day(&mut self, d: Day) {
        self.days.insert(d, DayStatus::Close);
        *self.cached.borrow_mut() = None;
    }

    pub fn open_day(&mut self, d: Day) {
        self.days.insert(d, DayStatus::Open);
        *self.cached.borrow_mut() = None;
    }

    pub fn set_off_before(&mut self, d: Day) {
        self.off_before = Some(d);
        *self.cached.borrow_mut() = None;
    }

    pub fn set_off_after(&mut self, d: Day) {
        self.off_after = Some(d);
        *self.cached.borrow_mut() = None;
    }

    pub fn last_day_if_any(&self) -> Option<Day> {
        self.off_after
    }

    pub fn add_working_time_range(&mut self, r: TimeRange) {
        self.working_time_ranges.push(r);
    }

    pub fn as_piecewise_constant(&self) -> Pc {
        if let Some(c) = self.cached.borrow().as_ref() {
            return c.clone();
        }
        let mut week = ByWeekday::of(Fraction::ONE);
        for (d, s) in &self.weekday {
            if *s == DayStatus::Close {
                week = week.with(*d, Fraction::ZERO);
            }
        }
        let week: Pc = Rc::new(week);
        let mut result = week.clone();
        if !self.days.is_empty() {
            let mut closed = HashMap::new();
            let mut open = HashMap::new();
            for (d, s) in &self.days {
                if *s == DayStatus::Close {
                    closed.insert(*d, Fraction::ZERO);
                } else {
                    open.insert(*d, Fraction::ONE);
                }
            }
            let specific_closed: Pc = Rc::new(SpecificDays::with_days(Fraction::ONE, closed));
            let specific_open: Pc = Rc::new(SpecificDays::with_days(Fraction::ZERO, open));
            result = Combiner::max(vec![
                specific_open,
                Combiner::product(vec![week, specific_closed]),
            ]);
        }
        if !self.working_time_ranges.is_empty() {
            result = Combiner::product(vec![
                result,
                Rc::new(Hours::from_working_hours(&self.working_time_ranges)),
            ]);
        }
        let start_of = |d: Day| d * SECS_PER_DAY;
        let result = match (self.off_before, self.off_after) {
            (None, None) => result,
            (None, Some(a)) => Combiner::product(vec![
                result,
                Rc::new(TimeWindow::new(MIN_TIME, start_of(a + 1))),
            ]),
            (Some(b), None) => Combiner::product(vec![
                result,
                Rc::new(TimeWindow::new(start_of(b), MAX_TIME)),
            ]),
            (Some(b), Some(a)) => Combiner::product(vec![
                result,
                Rc::new(TimeWindow::new(start_of(b), start_of(a + 1))),
            ]),
        };
        *self.cached.borrow_mut() = Some(result.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gantt::time::days_from_civil;

    fn d(y: i64, m: u32, dd: u32) -> Day {
        days_from_civil(y, m, dd)
    }

    #[test]
    fn fractions() {
        let a = Fraction::new(1, 2);
        assert_eq!(a.add(Fraction::new(1, 3)), Fraction::new(5, 6));
        assert_eq!(a.subtract(a), Fraction::ZERO);
        assert_eq!(a.divide(Fraction::new(1, 4)).whole_part(), 2);
        assert_eq!(a.multiply_by_long(10).whole_part(), 5);
        assert_eq!(Fraction::new(3, -6), Fraction::new(-1, 2));
        assert!(
            Op::Max.apply(a, Fraction::ZERO) == a
                && Op::Max.apply(a, Fraction::ONE) == Fraction::ONE
        );
        assert_eq!(Op::Sum.apply(a, a), Fraction::ONE);
    }

    #[test]
    fn weekend_is_skipped() {
        let mut oc = OpenClose::new();
        oc.close_dow(Dow::Saturday);
        oc.close_dow(Dow::Sunday);
        assert_eq!(oc.days_in_week(), 5);
        let pc = oc.as_piecewise_constant();
        // Friday 2020-07-03, three days of work ends Wednesday 2020-07-08 at midnight.
        let start = d(2020, 7, 3) * SECS_PER_DAY;
        let end = integrate(pc.as_ref(), 3 * SECS_PER_DAY, start, Dir::Forward);
        assert_eq!(end, d(2020, 7, 8) * SECS_PER_DAY);
        let back = integrate(pc.as_ref(), 3 * SECS_PER_DAY, end, Dir::Backward);
        assert_eq!(back, start);
        assert!(is_zero_on_day(pc.as_ref(), d(2020, 7, 4)));
        assert!(!is_zero_on_day(pc.as_ref(), d(2020, 7, 6)));
        assert!(oc.is_closed(d(2020, 7, 4)));
    }

    #[test]
    fn specific_days_and_windows() {
        let mut oc = OpenClose::new();
        oc.close_dow(Dow::Saturday);
        oc.open_day(d(2018, 12, 29));
        oc.close_day(d(2018, 12, 25));
        oc.set_off_before(d(2018, 12, 1));
        oc.set_off_after(d(2019, 1, 31));
        assert_eq!(oc.last_day_if_any(), Some(d(2019, 1, 31)));
        let pc = oc.as_piecewise_constant();
        assert!(!is_zero_on_day(pc.as_ref(), d(2018, 12, 29)));
        assert!(is_zero_on_day(pc.as_ref(), d(2018, 12, 25)));
        assert!(is_zero_on_day(pc.as_ref(), d(2018, 11, 20)));
        assert!(is_zero_on_day(pc.as_ref(), d(2019, 2, 20)));
        let copy = oc.mutate_me(Some(&OpenClose::new()));
        assert!(copy.is_closed(d(2018, 12, 25)));
    }

    #[test]
    fn working_hours() {
        let mut oc = OpenClose::new();
        oc.add_working_time_range(TimeRange::parse("8:00", "12:00").unwrap());
        oc.add_working_time_range(TimeRange::parse("14:00", "18:00").unwrap());
        let pc = oc.as_piecewise_constant();
        let start = d(2024, 9, 23) * SECS_PER_DAY + 10 * 3600;
        // 4 hours of work from 10:00: 2 hours until 12:00 then 2 more from 14:00.
        let end = integrate(pc.as_ref(), 4 * 3600, start, Dir::Forward);
        assert_eq!(end, d(2024, 9, 23) * SECS_PER_DAY + 16 * 3600);
        let back = integrate(pc.as_ref(), 4 * 3600, end, Dir::Backward);
        assert_eq!(back, start);
        assert!(TimeRange::parse("x", "1:00").is_none());
        assert_eq!(TimeRange::parse("1:00", "3:30").unwrap().duration(), 9000);
    }

    #[test]
    fn sums_and_percentages() {
        let half: Pc = Rc::new(SpecificDays::of(Fraction::new(1, 2)));
        let full: Pc = Rc::new(ByWeekday::of(Fraction::ONE));
        let both = Combiner::sum(vec![half.clone(), full.clone()]);
        let seg = both.segment_at(0, Dir::Forward);
        assert_eq!(seg.value, Fraction::new(3, 2));
        let one = Combiner::product(vec![half.clone()]);
        assert_eq!(one.segment_at(0, Dir::Forward).value, Fraction::new(1, 2));
        let w = ByWeekday::of(Fraction::ONE).with(Dow::Thursday, Fraction::ZERO);
        assert!(w.segment_at(0, Dir::Forward).value.is_zero());
        assert!(w.segment_at(SECS_PER_DAY, Dir::Backward).value.is_zero());
        let sd = SpecificDays::of(Fraction::ONE);
        assert_eq!(sd.segment_at(SECS_PER_DAY, Dir::Backward).end, 0);
    }
}
