//! Converting instants to horizontal positions (`gantt/timescale`).

use super::calendar::{OpenClose, TimeRange};
use super::time::{Day, Dow, TimePoint, MILLIS_PER_DAY, SECS_PER_DAY};
use super::PrintScale;
use std::cell::RefCell;
use std::collections::HashMap;

/// Working and closed hours of a day, as widths (`WorkingHours`).
#[derive(Debug, Clone)]
pub struct WorkingHours {
    pub ranges: Vec<TimeRange>,
    pub day_width: f64,
    pub close_factor: f64,
}

/// The seconds of `LocalTime.MAX`, as `Duration.between` counts them.
const END_OF_DAY: i64 = SECS_PER_DAY - 1;

impl WorkingHours {
    fn width_for_working(&self, seconds: i64) -> f64 {
        seconds as f64 * self.day_width / (24.0 * 3600.0)
    }

    fn width_for_closed(&self, seconds: i64) -> f64 {
        self.close_factor * self.width_for_working(seconds)
    }

    pub fn total_working(&self) -> i64 {
        self.ranges.iter().map(TimeRange::duration).sum()
    }

    pub fn working_duration_for_one_day(&self, is_closed: bool) -> f64 {
        if is_closed {
            return self.width_for_closed(SECS_PER_DAY);
        }
        let working = self.total_working();
        let pause = SECS_PER_DAY - working;
        self.width_for_working(working) + self.width_for_closed(pause)
    }

    /// Width of the day before time `pos` (seconds since midnight).
    pub fn partial_day_width(&self, pos: i64) -> f64 {
        let mut working = 0;
        let mut pause = 0;
        let mut t = 0;
        for r in &self.ranges {
            if pos < r.start {
                pause += pos - t;
                return self.width_for_working(working) + self.width_for_closed(pause);
            } else if pos < r.end {
                pause += r.start - t;
                working += pos - r.start;
                return self.width_for_working(working) + self.width_for_closed(pause);
            }
            pause += r.start - t;
            working += r.duration();
            t = r.end;
        }
        pause += pos - t;
        self.width_for_working(working) + self.width_for_closed(pause)
    }

    /// Rectangles (x, width) of the closed periods of a day.
    pub fn closed_rectangles(&self) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        let mut current = self.partial_day_width(0);
        for r in &self.ranges {
            let start = self.partial_day_width(r.start);
            out.push((current, start - current));
            current = self.partial_day_width(r.end);
        }
        let last = self.partial_day_width(END_OF_DAY);
        out.push((current, last - current));
        out
    }
}

pub enum TimeScale {
    /// Every day has the same width (`TimeScaleWink`).
    Wink {
        cell_width: f64,
        print_scale: PrintScale,
    },
    /// Wink shifted so that a day sits at zero (`TimeScaleDaily`, `TimeScaleCompressed`).
    Daily {
        cell_width: f64,
        delta: f64,
        compressed: bool,
    },
    HideClosed {
        cell_width: f64,
        starting_day: TimePoint,
        open_close: OpenClose,
        starting_int: RefCell<HashMap<TimePoint, i64>>,
    },
    Variant {
        min: Day,
        working_hours: WorkingHours,
        open_close: OpenClose,
    },
}

fn wink_position(cell_width: f64, instant: TimePoint) -> f64 {
    instant.millis() as f64 * cell_width / MILLIS_PER_DAY as f64
}

impl TimeScale {
    pub fn wink(size: f64, scale: f64, print_scale: PrintScale) -> TimeScale {
        TimeScale::Wink {
            cell_width: size * scale,
            print_scale,
        }
    }

    pub fn daily(
        size: f64,
        starting_day: TimePoint,
        scale: f64,
        zero_day: Option<Day>,
        compressed: bool,
    ) -> TimeScale {
        let cell_width = size * scale;
        let reference = zero_day.map_or(starting_day, TimePoint::of_start_of_day);
        TimeScale::Daily {
            cell_width,
            delta: wink_position(cell_width, reference),
            compressed,
        }
    }

    pub fn hide_closed(
        size: f64,
        starting_day: TimePoint,
        scale: f64,
        open_close: OpenClose,
    ) -> TimeScale {
        let mut map = HashMap::new();
        map.insert(starting_day, 0);
        TimeScale::HideClosed {
            cell_width: size * scale,
            starting_day,
            open_close,
            starting_int: RefCell::new(map),
        }
    }

    pub fn position(&self, instant: TimePoint) -> f64 {
        match self {
            TimeScale::Wink { cell_width, .. } => wink_position(*cell_width, instant),
            TimeScale::Daily {
                cell_width, delta, ..
            } => wink_position(*cell_width, instant) - delta,
            TimeScale::HideClosed {
                cell_width,
                starting_day,
                starting_int,
                ..
            } => {
                let instant = instant.floor_to_day().max(*starting_day);
                self.compute_up_to(instant);
                starting_int.borrow().get(&instant).copied().unwrap_or(0) as f64 * cell_width
            }
            TimeScale::Variant {
                min,
                working_hours,
                open_close,
            } => {
                let day = instant.to_day();
                let mut tmp = 0.0;
                let mut d = *min;
                while d < day {
                    tmp += working_hours.working_duration_for_one_day(open_close.is_closed(d));
                    d += 1;
                }
                tmp + working_hours.partial_day_width(instant.seconds_of_day())
            }
        }
    }

    fn compute_up_to(&self, dest: TimePoint) {
        let TimeScale::HideClosed {
            starting_day,
            starting_int,
            ..
        } = self
        else {
            return;
        };
        let mut i = *starting_day;
        let mut current = 0;
        loop {
            if dest <= i {
                return;
            }
            if self.width(i) > 0.0 {
                current += 1;
            }
            i = i.increment();
            starting_int.borrow_mut().insert(i, current);
            if i >= dest {
                return;
            }
        }
    }

    pub fn width(&self, instant: TimePoint) -> f64 {
        match self {
            TimeScale::Wink { cell_width, .. } | TimeScale::Daily { cell_width, .. } => *cell_width,
            TimeScale::HideClosed {
                cell_width,
                open_close,
                ..
            } => {
                if open_close.is_closed(instant.to_day()) {
                    0.0
                } else {
                    *cell_width
                }
            }
            TimeScale::Variant {
                working_hours,
                open_close,
                ..
            } => working_hours.working_duration_for_one_day(open_close.is_closed(instant.to_day())),
        }
    }

    pub fn is_breaking(&self, instant: TimePoint) -> bool {
        match self {
            TimeScale::Wink { print_scale, .. } => {
                if *print_scale == PrintScale::Weekly {
                    let num = instant.millis() / MILLIS_PER_DAY;
                    return num % 7 == 6;
                }
                true
            }
            TimeScale::Daily { compressed, .. } => {
                if *compressed {
                    instant.to_day_of_week() == Dow::Sunday
                } else {
                    true
                }
            }
            TimeScale::HideClosed { .. } => true,
            TimeScale::Variant { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gantt::time::days_from_civil;

    #[test]
    fn daily_positions() {
        let start = TimePoint::of_ymd(2020, 7, 1);
        let ts = TimeScale::daily(16.0, start, 1.0, None, false);
        assert_eq!(ts.position(start), 0.0);
        assert_eq!(ts.position(start.add_days(3)), 48.0);
        assert_eq!(ts.width(start), 16.0);
        assert!(ts.is_breaking(start));
        let weekly = TimeScale::daily(16.0, start, 0.25, Some(days_from_civil(2020, 6, 29)), true);
        assert!(weekly.position(start) > 0.0);
        assert!(weekly.is_breaking(TimePoint::of_ymd(2020, 7, 5)));
        assert!(!weekly.is_breaking(start));
        let wink = TimeScale::wink(16.0, 1.0, PrintScale::Weekly);
        assert!(wink.is_breaking(TimePoint::of_day_time(6, 0, 0, 0)));
        assert!(!wink.is_breaking(TimePoint::of_day_time(0, 0, 0, 0)));
    }

    #[test]
    fn closed_days_take_no_room() {
        let mut oc = OpenClose::new();
        oc.close_dow(Dow::Saturday);
        oc.close_dow(Dow::Sunday);
        let start = TimePoint::of_ymd(2020, 7, 3);
        let ts = TimeScale::hide_closed(10.0, start, 1.0, oc);
        assert_eq!(ts.position(start.add_days(1)), 10.0);
        // Friday to Monday: the weekend is skipped.
        assert_eq!(ts.position(start.add_days(3)), 10.0);
        assert_eq!(ts.width(start.add_days(1)), 0.0);
        assert!(ts.is_breaking(start));
    }

    #[test]
    fn working_hours_widths() {
        let wh = WorkingHours {
            ranges: vec![
                TimeRange::parse("8:00", "12:00").unwrap(),
                TimeRange::parse("14:00", "18:00").unwrap(),
            ],
            day_width: 120.0,
            close_factor: 0.2,
        };
        let full = wh.working_duration_for_one_day(false);
        assert!((wh.partial_day_width(86_399) - full).abs() < 0.01);
        assert_eq!(wh.closed_rectangles().len(), 3);
        let ts = TimeScale::Variant {
            min: 0,
            working_hours: wh.clone(),
            open_close: OpenClose::new(),
        };
        assert!((ts.position(TimePoint::of_start_of_day(1)) - full).abs() < 1e-9);
        assert_eq!(ts.width(TimePoint(0)), full);
        assert!(!ts.is_breaking(TimePoint(0)));
        assert!(wh.working_duration_for_one_day(true) < full);
    }
}
