//! Style values and their cascade priority (`style/value`).

use crate::klimt::color::HColor;
use crate::klimt::font::FontFace;
use crate::klimt::geom::HorizontalAlignment;

/// Priority of a value; compared field by field in declaration order (`Specificity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Specificity {
    forced: bool,
    ancestor_cascade: bool,
    ancestor_rank: i32,
    stereotype_count: usize,
    order: i32,
}

impl Specificity {
    pub fn at_order(order: i32) -> Self {
        Self {
            forced: false,
            ancestor_cascade: false,
            ancestor_rank: 0,
            stereotype_count: 0,
            order,
        }
    }
    pub fn forced_override() -> Self {
        Self {
            forced: true,
            ..Self::at_order(0)
        }
    }
    pub fn with_stereotype_count(mut self, count: usize) -> Self {
        self.stereotype_count = count;
        self
    }
    pub fn with_ancestor_rank(mut self, rank: i32) -> Self {
        self.ancestor_cascade = true;
        self.ancestor_rank = rank;
        self
    }
    pub fn has_stereotype(&self) -> bool {
        self.stereotype_count > 0
    }
    pub fn is_bigger_than(&self, other: &Specificity) -> bool {
        self > other
    }
}

/// A property value (`Value`): a light/dark string pair, a resolved color, or nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Str {
        light: Option<String>,
        dark: Option<String>,
        specificity: Specificity,
    },
    Color(HColor, Specificity),
}

impl Value {
    pub fn regular(s: &str, specificity: Specificity) -> Value {
        Value::Str {
            light: Some(s.to_string()),
            dark: None,
            specificity,
        }
    }

    pub fn of(light: Option<String>, dark: Option<String>, specificity: Specificity) -> Value {
        Value::Str {
            light,
            dark,
            specificity,
        }
    }

    pub fn specificity(&self) -> Specificity {
        match self {
            Value::Null => Specificity::at_order(0),
            Value::Str { specificity, .. } | Value::Color(_, specificity) => *specificity,
        }
    }

    pub fn with_ancestor_rank(&self, rank: i32) -> Value {
        match self {
            Value::Str {
                light,
                dark,
                specificity,
            } => Value::of(
                light.clone(),
                dark.clone(),
                specificity.with_ancestor_rank(rank),
            ),
            other => other.clone(),
        }
    }

    /// `ValueImpl.mergeWith` / `DarkString.mergeWith`: `previous` is the value being overridden.
    pub fn merge_with(&self, previous: Option<&Value>) -> Value {
        let Some(prev) = previous else {
            return self.clone();
        };
        match (self, prev) {
            (
                Value::Str {
                    light: l1,
                    dark: d1,
                    specificity: s1,
                },
                Value::Str {
                    light: l2,
                    dark: d2,
                    specificity: s2,
                },
            ) => {
                // `this` is the newer value, `other` (prev) the existing one.
                if (d1.is_none() && d2.is_none()) || (l1.is_none() && l2.is_none()) {
                    return if s1.is_bigger_than(s2) {
                        self.clone()
                    } else {
                        prev.clone()
                    };
                }
                if d1.is_none() && l2.is_none() {
                    return Value::of(l1.clone(), d2.clone(), *s1);
                }
                if d2.is_none() && l1.is_none() {
                    return Value::of(l2.clone(), d1.clone(), *s2);
                }
                if s1.is_bigger_than(s2) {
                    self.clone()
                } else {
                    prev.clone()
                }
            }
            _ => {
                if prev.specificity().is_bigger_than(&self.specificity()) {
                    prev.clone()
                } else {
                    self.clone()
                }
            }
        }
    }

    fn light(&self) -> Option<&str> {
        match self {
            Value::Str { light, .. } => light.as_deref(),
            _ => None,
        }
    }

    pub fn as_string(&self) -> String {
        self.light().unwrap_or_default().to_string()
    }

    pub fn as_color(&self) -> HColor {
        match self {
            Value::Null => HColor::BLACK,
            Value::Color(c, _) => c.clone(),
            Value::Str { light, dark, .. } => {
                let Some(light) = light else {
                    panic!("no light value: {self:?}");
                };
                if light.eq_ignore_ascii_case("none") || light.eq_ignore_ascii_case("transparent") {
                    return HColor::TRANSPARENT;
                }
                let result = HColor::parse_or_white(light);
                match dark {
                    Some(d) => result.with_dark(HColor::parse_or_white(d)),
                    None => result,
                }
            }
        }
    }

    pub fn as_boolean(&self) -> bool {
        self.light().is_some_and(|s| s.eq_ignore_ascii_case("true"))
    }

    fn digits(&self) -> String {
        self.light()
            .unwrap_or_default()
            .chars()
            .filter(char::is_ascii_digit)
            .collect()
    }

    pub fn as_int(&self) -> i32 {
        self.digits().parse().unwrap_or(0)
    }

    pub fn as_int_or_minus_one(&self) -> i32 {
        if matches!(self, Value::Null) {
            return 0;
        }
        self.digits().parse().unwrap_or(-1)
    }

    /// Digits and dots only; `NaN` when there are none (zero for [`Value::Null`]).
    pub fn as_double(&self) -> f64 {
        if matches!(self, Value::Null) {
            return 0.0;
        }
        let s: String = self
            .light()
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        s.parse().unwrap_or(f64::NAN)
    }

    pub fn as_double_default_to(&self, default: f64) -> f64 {
        if matches!(self, Value::Null) {
            return default;
        }
        let v = self.as_double();
        if v.is_nan() {
            default
        } else {
            v
        }
    }

    pub fn as_font_face(&self) -> FontFace {
        let raw = self.light().unwrap_or_default();
        if raw.is_empty() {
            return FontFace::NORMAL;
        }
        let s = raw.trim().to_lowercase();
        match s.as_str() {
            "bold" => FontFace::BOLD,
            "italic" => FontFace::ITALIC,
            "plain" | "normal" => FontFace::NORMAL,
            _ => FontFace::from_css_weight(&s).unwrap_or(FontFace::NORMAL),
        }
    }

    pub fn as_horizontal_alignment(&self) -> HorizontalAlignment {
        match self {
            Value::Null => HorizontalAlignment::Left,
            _ => HorizontalAlignment::from_string(&self.as_string()).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(order: i32) -> Specificity {
        Specificity::at_order(order)
    }

    #[test]
    fn specificity_order() {
        assert!(s(2).is_bigger_than(&s(1)));
        assert!(!s(1).is_bigger_than(&s(1)));
        assert!(s(1).with_stereotype_count(1).is_bigger_than(&s(5)));
        assert!(s(1)
            .with_ancestor_rank(0)
            .is_bigger_than(&s(5).with_stereotype_count(3)));
        assert!(Specificity::forced_override().is_bigger_than(&s(100).with_ancestor_rank(9)));
        assert!(s(1).with_stereotype_count(1).has_stereotype());
    }

    #[test]
    fn merge_light_dark() {
        let a = Value::regular("a", s(1));
        let b = Value::regular("b", s(2));
        assert_eq!(b.merge_with(Some(&a)), b);
        assert_eq!(a.merge_with(Some(&b)), b);
        assert_eq!(a.merge_with(None), a);
        let dark = Value::of(None, Some("d".into()), s(3));
        let merged = a.merge_with(Some(&dark));
        assert_eq!(merged, Value::of(Some("a".into()), Some("d".into()), s(1)));
        let merged = dark.merge_with(Some(&a));
        assert_eq!(merged, Value::of(Some("a".into()), Some("d".into()), s(1)));
        let full = Value::of(Some("x".into()), Some("y".into()), s(1));
        assert_eq!(
            full.merge_with(Some(&Value::of(Some("p".into()), Some("q".into()), s(2))))
                .as_string(),
            "p"
        );
        let c = Value::Color(HColor::WHITE, s(9));
        assert_eq!(a.merge_with(Some(&c)), c);
        assert_eq!(c.merge_with(Some(&a)), c);
    }

    #[test]
    fn conversions() {
        let v = |x: &str| Value::regular(x, s(1));
        assert_eq!(v("14px").as_int(), 14);
        assert_eq!(v("").as_int(), 0);
        assert_eq!(v("x").as_int_or_minus_one(), -1);
        assert_eq!(v("1.5").as_double(), 1.5);
        assert!(v("x").as_double().is_nan());
        assert_eq!(v("x").as_double_default_to(1.5), 1.5);
        assert!(v("TRUE").as_boolean());
        assert_eq!(v("bold").as_font_face(), FontFace::BOLD);
        assert_eq!(v("600").as_font_face().weight, 600);
        assert_eq!(v("zzz").as_font_face(), FontFace::NORMAL);
        assert_eq!(v("").as_font_face(), FontFace::NORMAL);
        assert_eq!(
            v("center").as_horizontal_alignment(),
            HorizontalAlignment::Center
        );
        assert_eq!(v("none").as_color(), HColor::TRANSPARENT);
        assert_eq!(v("white").as_color(), HColor::WHITE);
        let dk = Value::of(Some("white".into()), Some("black".into()), s(1)).as_color();
        assert_eq!(dk.dark_scheme_theme(), HColor::BLACK);
        assert_eq!(Value::Null.as_int(), 0);
        assert_eq!(Value::Null.as_double(), 0.0);
        assert_eq!(Value::Null.as_double_default_to(2.0), 2.0);
        assert_eq!(Value::Null.as_color(), HColor::BLACK);
        assert_eq!(Value::Null.as_string(), "");
        assert!(!Value::Null.as_boolean());
        assert_eq!(Value::Null.as_font_face(), FontFace::NORMAL);
        assert_eq!(
            Value::Null.as_horizontal_alignment(),
            HorizontalAlignment::Left
        );
        assert_eq!(Value::Null.as_int_or_minus_one(), 0);
        assert_eq!(Value::Color(HColor::WHITE, s(1)).as_color(), HColor::WHITE);
        let r = v("a").with_ancestor_rank(2);
        assert!(r.specificity() > s(1).with_ancestor_rank(1));
        assert_eq!(Value::Null.with_ancestor_rank(1), Value::Null);
    }
}
