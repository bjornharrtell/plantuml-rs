//! Dates and instants of Gantt diagrams (`gantt/time`), replacing `java.time` with plain
//! proleptic Gregorian arithmetic: a day is a number of days since 1970-01-01 and an instant a
//! number of seconds since 1970-01-01T00:00 (no time zone).

use super::i18n_data;

/// A `LocalDate`: days since 1970-01-01.
pub type Day = i64;

pub const SECS_PER_DAY: i64 = 86_400;
pub const MILLIS_PER_DAY: i64 = SECS_PER_DAY * 1000;

/// Days since the epoch of a civil date.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> Day {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date of a day number.
pub fn civil_from_days(z: Day) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
    }
}

/// `LocalDate.of`, `None` when the date does not exist.
pub fn date_of(y: i64, m: u32, d: u32) -> Option<Day> {
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// A day of the week, Monday first like `java.time.DayOfWeek`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Dow {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Dow {
    pub const ALL: [Dow; 7] = [
        Dow::Monday,
        Dow::Tuesday,
        Dow::Wednesday,
        Dow::Thursday,
        Dow::Friday,
        Dow::Saturday,
        Dow::Sunday,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// `DayOfWeek.getValue()`: 1 for Monday.
    pub fn value(self) -> i64 {
        self as i64 + 1
    }

    pub fn of_day(day: Day) -> Dow {
        Dow::ALL[(day + 3).rem_euclid(7) as usize]
    }

    pub fn name_upper(self) -> &'static str {
        [
            "MONDAY",
            "TUESDAY",
            "WEDNESDAY",
            "THURSDAY",
            "FRIDAY",
            "SATURDAY",
            "SUNDAY",
        ][self.index()]
    }

    /// `DayOfWeekUtils.fromString`: matched on the first three letters.
    pub fn from_string(value: &str) -> Option<Dow> {
        let up: String = value.to_uppercase().chars().take(3).collect();
        Dow::ALL
            .into_iter()
            .find(|d| d.name_upper().starts_with(&up))
    }
}

pub const MONTH_NAMES: [&str; 12] = [
    "JANUARY",
    "FEBRUARY",
    "MARCH",
    "APRIL",
    "MAY",
    "JUNE",
    "JULY",
    "AUGUST",
    "SEPTEMBER",
    "OCTOBER",
    "NOVEMBER",
    "DECEMBER",
];

/// `MonthUtils.fromString`: a month number (1 to 12) from its first three letters.
pub fn month_from_string(value: &str) -> Option<u32> {
    let up: String = value.to_uppercase().chars().take(3).collect();
    MONTH_NAMES
        .iter()
        .position(|m| m.starts_with(&up))
        .map(|i| i as u32 + 1)
}

/// How weeks are numbered (`WeekNumberStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekNumberStrategy {
    pub first_day_of_week: Dow,
    pub minimal_days_in_first_week: i64,
}

impl Default for WeekNumberStrategy {
    fn default() -> Self {
        Self {
            first_day_of_week: Dow::Monday,
            minimal_days_in_first_week: 4,
        }
    }
}

/// An instant (`TimePoint`), seconds since 1970-01-01T00:00.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimePoint(pub i64);

impl TimePoint {
    pub fn of_start_of_day(day: Day) -> TimePoint {
        TimePoint(day * SECS_PER_DAY)
    }

    pub fn of_end_of_day_minus_one_second(day: Day) -> TimePoint {
        TimePoint((day + 1) * SECS_PER_DAY - 1)
    }

    pub fn of_ymd(y: i64, m: u32, d: u32) -> TimePoint {
        TimePoint::of_start_of_day(days_from_civil(y, m, d))
    }

    pub fn of_day_time(day: Day, hour: i64, minute: i64, second: i64) -> TimePoint {
        TimePoint(day * SECS_PER_DAY + hour * 3600 + minute * 60 + second)
    }

    pub fn secs(self) -> i64 {
        self.0
    }

    pub fn millis(self) -> i64 {
        self.0 * 1000
    }

    pub fn to_day(self) -> Day {
        self.0.div_euclid(SECS_PER_DAY)
    }

    pub fn seconds_of_day(self) -> i64 {
        self.0.rem_euclid(SECS_PER_DAY)
    }

    pub fn hour(self) -> i64 {
        self.seconds_of_day() / 3600
    }

    pub fn minute(self) -> i64 {
        self.seconds_of_day() % 3600 / 60
    }

    pub fn second(self) -> i64 {
        self.seconds_of_day() % 60
    }

    pub fn ymd(self) -> (i64, u32, u32) {
        civil_from_days(self.to_day())
    }

    pub fn year(self) -> i64 {
        self.ymd().0
    }

    pub fn month(self) -> u32 {
        self.ymd().1
    }

    pub fn day_of_month(self) -> u32 {
        self.ymd().2
    }

    /// `YearMonth.from(time)`.
    pub fn month_year(self) -> (i64, u32) {
        let (y, m, _) = self.ymd();
        (y, m)
    }

    /// `"Q" + ((month().ordinal() + 3) / 3)`.
    pub fn quarter(self) -> String {
        format!("Q{}", (self.month() - 1 + 3) / 3)
    }

    pub fn to_day_of_week(self) -> Dow {
        Dow::of_day(self.to_day())
    }

    pub fn add_days(self, n: i64) -> TimePoint {
        TimePoint(self.0 + n * SECS_PER_DAY)
    }

    pub fn increment(self) -> TimePoint {
        self.add_days(1)
    }

    pub fn decrement(self) -> TimePoint {
        self.add_days(-1)
    }

    pub fn minus_one_second(self) -> TimePoint {
        TimePoint(self.0 - 1)
    }

    pub fn absolute_day_num(self) -> i64 {
        self.0.div_euclid(SECS_PER_DAY)
    }

    pub fn floor_to_day(self) -> TimePoint {
        TimePoint::of_start_of_day(self.to_day())
    }

    /// `LocalDate.get(WeekFields.of(first, minimalDays).weekOfYear())`.
    pub fn week_of_year(self, strategy: WeekNumberStrategy) -> i64 {
        let day = self.to_day();
        let (y, _, _) = civil_from_days(day);
        let doy = day - days_from_civil(y, 1, 1) + 1;
        let dow = (Dow::of_day(day).value() - strategy.first_day_of_week.value()).rem_euclid(7) + 1;
        let week_start = (doy - dow).rem_euclid(7);
        let mut offset = -week_start;
        if week_start + 1 > strategy.minimal_days_in_first_week {
            offset = 7 - week_start;
        }
        (7 + offset + (doy - 1)) / 7
    }
}

/// How the month and the day are written (`TimePointFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePointFormat {
    DayOfWeekShort,
    DayOfWeekLong,
    MonthYearShort,
    MonthYearLong,
    MonthLong,
    MonthShort,
    Year,
    Quarter,
    DayOfMonth,
    MonthAndDay,
}

fn lookup<const N: usize>(
    table: &[(&'static str, [&'static str; N])],
    lang: &str,
    i: usize,
) -> &'static str {
    let row = table
        .iter()
        .find(|(l, _)| *l == lang)
        .or_else(|| table.iter().find(|(l, _)| *l == "en"))
        .expect("english row");
    row.1[i]
}

pub fn day_of_week_long(d: Dow, lang: &str) -> &'static str {
    lookup(i18n_data::DAY_OF_WEEK_LONG, lang, d.index())
}

pub fn day_of_week_short(d: Dow, lang: &str) -> &'static str {
    lookup(i18n_data::DAY_OF_WEEK_SHORT, lang, d.index())
}

pub fn month_long(month: u32, lang: &str) -> &'static str {
    lookup(i18n_data::MONTH_LONG, lang, month as usize - 1)
}

pub fn month_short(month: u32, lang: &str) -> &'static str {
    lookup(i18n_data::MONTH_SHORT, lang, month as usize - 1)
}

impl TimePointFormat {
    pub fn format(self, when: TimePoint, lang: &str) -> String {
        match self {
            TimePointFormat::DayOfMonth => when.day_of_month().to_string(),
            TimePointFormat::Quarter => when.quarter(),
            TimePointFormat::Year => when.year().to_string(),
            TimePointFormat::DayOfWeekLong => day_of_week_long(when.to_day_of_week(), lang).into(),
            TimePointFormat::DayOfWeekShort => {
                day_of_week_short(when.to_day_of_week(), lang).into()
            }
            TimePointFormat::MonthLong => month_long(when.month(), lang).into(),
            TimePointFormat::MonthShort => month_short(when.month(), lang).into(),
            TimePointFormat::MonthYearShort => {
                format!("{} {}", month_short(when.month(), lang), when.year())
            }
            TimePointFormat::MonthYearLong => {
                format!("{} {}", month_long(when.month(), lang), when.year())
            }
            TimePointFormat::MonthAndDay => month_and_day(when, lang),
        }
    }
}

fn month_and_day(when: TimePoint, lang: &str) -> String {
    let d = when.day_of_month();
    let short = TimePointFormat::MonthShort.format(when, lang);
    match lang {
        "fr" => format!("{d} {short}"),
        "es" => format!("{d} de {short}"),
        "de" => format!("{d}. {short}"),
        "ja" => format!("{short}/{d}日"),
        "ko" => format!("{short} {d}일"),
        "ru" => format!(
            "{d} {}",
            TimePointFormat::MonthLong.format(when, lang).to_lowercase()
        ),
        "zh" => format!("{short}{d}日"),
        _ => format!("{short} {d}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2020, 2, 29)), (2020, 2, 29));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(date_of(2021, 2, 29), None);
        assert_eq!(date_of(2020, 2, 29), Some(days_from_civil(2020, 2, 29)));
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn day_of_week_and_names() {
        assert_eq!(Dow::of_day(0), Dow::Thursday);
        assert_eq!(Dow::of_day(days_from_civil(2018, 12, 5)), Dow::Wednesday);
        assert_eq!(Dow::from_string("satur"), Some(Dow::Saturday));
        assert_eq!(Dow::from_string("xx"), None);
        assert_eq!(month_from_string("December"), Some(12));
        assert_eq!(month_from_string("mar"), Some(3));
    }

    #[test]
    fn time_point_parts() {
        let t = TimePoint::of_day_time(days_from_civil(2024, 9, 22), 10, 30, 46);
        assert_eq!((t.hour(), t.minute(), t.second()), (10, 30, 46));
        assert_eq!(t.ymd(), (2024, 9, 22));
        assert_eq!(t.quarter(), "Q3");
        assert_eq!(t.to_day_of_week(), Dow::Sunday);
        assert_eq!(t.floor_to_day().seconds_of_day(), 0);
        assert_eq!(t.minus_one_second().second(), 45);
        assert_eq!(
            TimePoint::of_end_of_day_minus_one_second(0).seconds_of_day(),
            SECS_PER_DAY - 1
        );
    }

    #[test]
    fn week_numbers() {
        let s = WeekNumberStrategy::default();
        // ISO week of 2018-12-31 is week 1 of 2019 but weekOfYear counts inside the year.
        assert_eq!(TimePoint::of_ymd(2020, 7, 1).week_of_year(s), 27);
        assert_eq!(TimePoint::of_ymd(2021, 1, 1).week_of_year(s), 0);
        assert_eq!(TimePoint::of_ymd(2021, 1, 4).week_of_year(s), 1);
    }

    #[test]
    fn formats() {
        let t = TimePoint::of_ymd(2020, 7, 3);
        assert_eq!(TimePointFormat::MonthAndDay.format(t, "en"), "Jul 3");
        assert_eq!(TimePointFormat::MonthAndDay.format(t, "de"), "3. Jul");
        assert_eq!(TimePointFormat::MonthAndDay.format(t, "es"), "3 de jul");
        assert!(TimePointFormat::MonthAndDay.format(t, "ja").ends_with('日'));
        assert_eq!(TimePointFormat::DayOfWeekShort.format(t, "en"), "Fr");
        assert_eq!(TimePointFormat::DayOfWeekLong.format(t, "fr"), "vendredi");
        assert_eq!(TimePointFormat::MonthYearLong.format(t, "en"), "July 2020");
        assert_eq!(TimePointFormat::MonthYearShort.format(t, "xx"), "Jul 2020");
        assert_eq!(TimePointFormat::Quarter.format(t, "en"), "Q3");
        assert_eq!(TimePointFormat::Year.format(t, "en"), "2020");
        assert_eq!(TimePointFormat::DayOfMonth.format(t, "en"), "3");
        assert_eq!(TimePointFormat::MonthAndDay.format(t, "ru"), "3 июль");
    }
}
