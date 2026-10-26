//! Times of a timing diagram (`TimeTick`, `TimingFormat`, `DeduceFormat`) and the decimal
//! arithmetic behind them (`BigDecimal`).

use std::cmp::Ordering;
use std::rc::Rc;

/// Decimal number with a scale, like Java's `BigDecimal` (only what the diagram needs).
#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    unscaled: i128,
    scale: u32,
}

fn pow10(n: u32) -> Option<i128> {
    10i128.checked_pow(n)
}

impl Decimal {
    pub const ZERO: Decimal = Decimal {
        unscaled: 0,
        scale: 0,
    };

    pub fn from_i64(v: i64) -> Decimal {
        Decimal {
            unscaled: i128::from(v),
            scale: 0,
        }
    }

    /// Parses `[+-]digits[.digits]`.
    pub fn parse(s: &str) -> Option<Decimal> {
        let (neg, body) = match s.as_bytes().first()? {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        let (int, frac) = body.split_once('.').unwrap_or((body, ""));
        if int.is_empty() && frac.is_empty() {
            return None;
        }
        if !int.bytes().chain(frac.bytes()).all(|c| c.is_ascii_digit()) {
            return None;
        }
        let digits = format!("{int}{frac}");
        let unscaled: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse().ok()?
        };
        Some(Decimal {
            unscaled: if neg { -unscaled } else { unscaled },
            scale: u32::try_from(frac.len()).ok()?,
        })
    }

    pub fn signum(&self) -> i32 {
        self.unscaled.signum() as i32
    }

    pub fn to_f64(self) -> f64 {
        self.unscaled as f64 / 10f64.powi(self.scale as i32)
    }

    /// Integer part (`longValue`).
    pub fn to_i64(self) -> i64 {
        match pow10(self.scale) {
            Some(p) => (self.unscaled / p) as i64,
            None => 0,
        }
    }

    fn aligned(a: &Decimal, b: &Decimal) -> Option<(i128, i128)> {
        let scale = a.scale.max(b.scale);
        let x = a.unscaled.checked_mul(pow10(scale - a.scale)?)?;
        let y = b.unscaled.checked_mul(pow10(scale - b.scale)?)?;
        Some((x, y))
    }

    pub fn add(&self, o: &Decimal) -> Decimal {
        match Self::aligned(self, o) {
            Some((x, y)) => Decimal {
                unscaled: x + y,
                scale: self.scale.max(o.scale),
            },
            None => *self,
        }
    }

    pub fn mul_int(&self, n: i64) -> Decimal {
        Decimal {
            unscaled: self.unscaled * i128::from(n),
            scale: self.scale,
        }
    }

    /// `toPlainString`.
    pub fn to_plain_string(self) -> String {
        let neg = self.unscaled < 0;
        let mut digits = self.unscaled.unsigned_abs().to_string();
        let scale = self.scale as usize;
        if scale > 0 {
            if digits.len() <= scale {
                digits = format!("{}{digits}", "0".repeat(scale + 1 - digits.len()));
            }
            digits.insert(digits.len() - scale, '.');
        }
        if neg {
            format!("-{digits}")
        } else {
            digits
        }
    }

    /// `compareTo`: the scale does not matter.
    pub fn compare(&self, o: &Decimal) -> Ordering {
        match Self::aligned(self, o) {
            Some((x, y)) => x.cmp(&y),
            None => self
                .to_f64()
                .partial_cmp(&o.to_f64())
                .unwrap_or(Ordering::Equal),
        }
    }
}

/// A date pattern (`SimpleDateFormat`), printed in UTC.
#[derive(Debug)]
pub struct DateFormat {
    tokens: Vec<DateToken>,
}

#[derive(Debug)]
enum DateToken {
    Literal(String),
    Field(char, usize),
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl DateFormat {
    /// `None` for a pattern Java rejects.
    pub fn new(pattern: &str) -> Option<DateFormat> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        let mut literal = String::new();
        while i < chars.len() {
            let c = chars[i];
            if c == '\'' {
                if chars.get(i + 1) == Some(&'\'') {
                    literal.push('\'');
                    i += 2;
                    continue;
                }
                i += 1;
                loop {
                    match chars.get(i) {
                        None => return None,
                        Some('\'') if chars.get(i + 1) == Some(&'\'') => {
                            literal.push('\'');
                            i += 2;
                        }
                        Some('\'') => {
                            i += 1;
                            break;
                        }
                        Some(&ch) => {
                            literal.push(ch);
                            i += 1;
                        }
                    }
                }
            } else if c.is_ascii_alphabetic() {
                if !"GyYMLwWDdFEuaHkKhmsSzZX".contains(c) {
                    return None;
                }
                if !literal.is_empty() {
                    tokens.push(DateToken::Literal(std::mem::take(&mut literal)));
                }
                let mut n = 1;
                while chars.get(i + n) == Some(&c) {
                    n += 1;
                }
                tokens.push(DateToken::Field(c, n));
                i += n;
            } else {
                literal.push(c);
                i += 1;
            }
        }
        if !literal.is_empty() {
            tokens.push(DateToken::Literal(literal));
        }
        Some(DateFormat { tokens })
    }

    /// Formats `secs` seconds since the epoch.
    pub fn format(&self, secs: i64) -> String {
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        let (hh, mm, ss) = (rem / 3600, (rem / 60) % 60, rem % 60);
        let doy = days - days_from_civil(y, 1, 1) + 1;
        let dow = (days + 3).rem_euclid(7) as usize;
        let num = |v: i64, n: usize| format!("{v:0n$}");
        let mut out = String::new();
        for t in &self.tokens {
            match t {
                DateToken::Literal(s) => out.push_str(s),
                DateToken::Field(c, n) => {
                    let n = *n;
                    match c {
                        'G' => out.push_str(if y > 0 { "AD" } else { "BC" }),
                        'y' | 'Y' => {
                            if n == 2 {
                                out.push_str(&num(y.rem_euclid(100), 2));
                            } else {
                                out.push_str(&num(y, n));
                            }
                        }
                        'M' | 'L' => match n {
                            1 | 2 => out.push_str(&num(i64::from(m), n)),
                            3 => out.push_str(&MONTHS[m as usize - 1][..3]),
                            _ => out.push_str(MONTHS[m as usize - 1]),
                        },
                        'w' => out.push_str(&num((doy - 1) / 7 + 1, n)),
                        'W' | 'F' => out.push_str(&num(i64::from((d - 1) / 7 + 1), n)),
                        'D' => out.push_str(&num(doy, n)),
                        'd' => out.push_str(&num(i64::from(d), n)),
                        'E' => {
                            if n <= 3 {
                                out.push_str(&DAYS[dow][..3]);
                            } else {
                                out.push_str(DAYS[dow]);
                            }
                        }
                        'u' => out.push_str(&num(dow as i64 + 1, n)),
                        'a' => out.push_str(if hh < 12 { "AM" } else { "PM" }),
                        'H' => out.push_str(&num(hh, n)),
                        'k' => out.push_str(&num(if hh == 0 { 24 } else { hh }, n)),
                        'K' => out.push_str(&num(hh % 12, n)),
                        'h' => out.push_str(&num(if hh % 12 == 0 { 12 } else { hh % 12 }, n)),
                        'm' => out.push_str(&num(mm, n)),
                        's' => out.push_str(&num(ss, n)),
                        'S' => out.push_str(&num(0, n)),
                        'z' => out.push_str("UTC"),
                        'Z' => out.push_str("+0000"),
                        _ => out.push('Z'),
                    }
                }
            }
        }
        out
    }
}

/// Days since 1970-01-01 of a civil date; month and day may overflow like a lenient calendar.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y0 = year + (month - 1).div_euclid(12);
    let m = (month - 1).rem_euclid(12) + 1;
    let y = if m <= 2 { y0 - 1 } else { y0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + (day - 1)
}

pub fn civil_from_days(days: i64) -> (i64, i32, i32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// How a time is printed (`TimingFormat`).
#[derive(Debug, Clone)]
pub enum TimingFormat {
    Decimal,
    Hour,
    Date,
    Custom(Rc<DateFormat>),
}

impl PartialEq for TimingFormat {
    fn eq(&self, o: &Self) -> bool {
        match (self, o) {
            (TimingFormat::Decimal, TimingFormat::Decimal)
            | (TimingFormat::Hour, TimingFormat::Hour)
            | (TimingFormat::Date, TimingFormat::Date) => true,
            (TimingFormat::Custom(a), TimingFormat::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl TimingFormat {
    pub fn format_decimal(&self, time: &Decimal) -> String {
        match self {
            TimingFormat::Decimal => time.to_plain_string(),
            _ => self.format_time(time.to_i64()),
        }
    }

    pub fn format_time(&self, time: i64) -> String {
        match self {
            TimingFormat::Custom(f) => f.format(time),
            TimingFormat::Hour => {
                let s = (time % 60) as i32;
                let m = ((time / 60) % 60) as i32;
                let h = (time / 3600) as i32;
                format!("{h}:{m:02}:{s:02}")
            }
            TimingFormat::Date => {
                let (_, m, d) = civil_from_days(time.div_euclid(86400));
                format!("{m:02}/{d:02}")
            }
            TimingFormat::Decimal => time.to_string(),
        }
    }

    /// `createDate`: midnight GMT of the given day.
    pub fn create_date(yyyy: i64, mm: i64, dd: i64, format: TimingFormat) -> TimeTick {
        let secs = days_from_civil(yyyy, mm, dd) * 86400;
        TimeTick::new(Decimal::from_i64(secs), format)
    }
}

#[derive(Debug)]
struct TickData {
    time: Decimal,
    format: TimingFormat,
}

/// A point in time. Equality of two ticks as objects (`==`) is [`TimeTick::same`];
/// ordering uses the value.
#[derive(Debug, Clone)]
pub struct TimeTick(Rc<TickData>);

impl TimeTick {
    pub fn new(time: Decimal, format: TimingFormat) -> TimeTick {
        TimeTick(Rc::new(TickData { time, format }))
    }

    pub fn time(&self) -> &Decimal {
        &self.0.time
    }

    pub fn format(&self) -> &TimingFormat {
        &self.0.format
    }

    /// Same object, as Java's identity `equals`.
    pub fn same(&self, o: &TimeTick) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }

    pub fn compare(&self, o: &TimeTick) -> Ordering {
        self.0.time.compare(&o.0.time)
    }
}

/// Number format deduced from the bounds of an analog player (`DeduceFormat`).
#[derive(Debug, Clone)]
pub struct DeduceFormat {
    force_sign: bool,
    grouping: Option<char>,
    decimal: Option<char>,
    min_fraction: usize,
    max_fraction: usize,
}

impl DeduceFormat {
    pub fn from(number: &str) -> Option<DeduceFormat> {
        let mut chars = number.chars().peekable();
        let mut sign = None;
        if let Some(&c) = chars.peek() {
            if c == '+' || c == '-' {
                sign = Some(c);
                chars.next();
            }
        }
        let mut non1: Option<(char, usize)> = None;
        let mut non2: Option<(char, usize)> = None;
        let mut fraction = 0usize;
        for ch in chars {
            if ch.is_numeric() {
                if non1.is_some() {
                    fraction += 1;
                }
                continue;
            }
            match (&mut non1, &mut non2) {
                (None, _) => {
                    non1 = Some((ch, 1));
                    fraction = 0;
                }
                (Some((c, n)), _) if *c == ch => *n += 1,
                (Some(_), None) => {
                    non2 = Some((ch, 1));
                    fraction = 0;
                }
                (Some(_), Some((c, n))) if *c == ch => *n += 1,
                _ => return None,
            }
        }
        let force_sign = sign == Some('+');
        Some(match (non1, non2) {
            (Some((a, _)), Some((b, _))) => DeduceFormat {
                force_sign,
                grouping: Some(a),
                decimal: Some(b),
                min_fraction: fraction,
                max_fraction: fraction,
            },
            (a, _) => DeduceFormat {
                force_sign,
                grouping: None,
                decimal: a.map(|x| x.0),
                min_fraction: fraction,
                max_fraction: fraction,
            },
        })
    }

    pub fn merge_with(&self, other: Option<&DeduceFormat>) -> DeduceFormat {
        let Some(o) = other else {
            return self.clone();
        };
        let decimal = self.decimal.or(o.decimal);
        let mut grouping = match (self.grouping, o.grouping) {
            (None, g) => g,
            (g, None) => g,
            (Some(a), Some(b)) if a == b => Some(a),
            _ => None,
        };
        if grouping.is_some() && grouping == decimal {
            grouping = None;
        }
        DeduceFormat {
            force_sign: self.force_sign || o.force_sign,
            grouping,
            decimal,
            min_fraction: self.min_fraction.min(o.min_fraction),
            max_fraction: self.max_fraction.max(o.max_fraction),
        }
    }

    /// Prints `value` like the `DecimalFormat` built by `getDecimalFormat`.
    pub fn format(&self, value: f64) -> String {
        let neg = value < 0.0;
        let text = format!("{:.*}", self.max_fraction, value.abs());
        let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
        let mut frac = frac.to_string();
        while frac.len() > self.min_fraction && frac.ends_with('0') {
            frac.pop();
        }
        let mut int_part = String::new();
        if let Some(g) = self.grouping {
            for (i, c) in int.chars().enumerate() {
                if i > 0 && (int.len() - i) % 3 == 0 {
                    int_part.push(g);
                }
                int_part.push(c);
            }
        } else {
            int_part = int.to_string();
        }
        let mut out = String::new();
        if neg && (int != "0" || frac.chars().any(|c| c != '0')) {
            out.push('-');
        }
        if self.force_sign {
            out.push('+');
        }
        out.push_str(&int_part);
        if !frac.is_empty() {
            out.push(self.decimal.unwrap_or('.'));
            out.push_str(&frac);
        }
        out
    }
}

/// `Double.toString`.
pub fn java_double(v: f64) -> String {
    if v == 0.0 || (v.abs() >= 1e-3 && v.abs() < 1e7) {
        return format!("{v:?}");
    }
    let s = format!("{v:e}");
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let m = if m.contains('.') {
        m.to_string()
    } else {
        format!("{m}.0")
    };
    format!("{m}E{e}")
}
