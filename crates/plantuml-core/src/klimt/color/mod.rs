//! Colors, ported from `klimt/color` (`HColorSet`, `HColorSimple`, `HSLColor`, ...).

mod named;

/// 8-bit RGBA color (`XColor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    fn from_u24(v: u32) -> Self {
        Self::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// `#RRGGBB`, `#RRGGBBAA`, or `#00000000` when fully transparent.
    pub fn to_svg(self) -> String {
        if self.a == 0 {
            return "#00000000".into();
        }
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// YIQ gray level, 0..=255.
    pub fn gray_scale(self) -> u8 {
        ((self.r as u32 * 299 + self.g as u32 * 587 + self.b as u32 * 114) / 1000) as u8
    }

    /// Weighted channel distance (`ColorUtils.distance`).
    pub fn distance(self, o: Rgba) -> u32 {
        let d = |a: u8, b: u8| a.abs_diff(b) as u32;
        d(self.r, o.r) * 299 + d(self.g, o.g) * 587 + d(self.b, o.b) * 114
    }

    pub fn is_gray(self) -> bool {
        self.r == self.g && self.g == self.b
    }

    /// `[h (0..360), s (0..100), l (0..100)]`.
    pub fn to_hsl(self) -> [f32; 3] {
        let (r, g, b) = (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        );
        let min = r.min(g.min(b));
        let max = r.max(g.max(b));
        let h = if max == min {
            0.0
        } else if max == r {
            ((60.0 * (g - b) / (max - min)) + 360.0) % 360.0
        } else if max == g {
            (60.0 * (b - r) / (max - min)) + 120.0
        } else {
            (60.0 * (r - g) / (max - min)) + 240.0
        };
        let l = (max + min) / 2.0;
        let s = if max == min {
            0.0
        } else if l <= 0.5 {
            (max - min) / (max + min)
        } else {
            (max - min) / (2.0 - max - min)
        };
        [h, s * 100.0, l * 100.0]
    }

    pub fn from_hsl(hsl: [f32; 3]) -> Self {
        let s = hsl[1].clamp(0.0, 100.0) / 100.0;
        let l = hsl[2].clamp(0.0, 100.0) / 100.0;
        let h = (hsl[0] % 360.0) / 360.0;
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            (l + s) - (s * l)
        };
        let p = 2.0 * l - q;
        let ch = |t: f32| (hue_to_rgb(p, q, t).clamp(0.0, 1.0) * 255.0).round() as u8;
        Self::rgb(ch(h + 1.0 / 3.0), ch(h), ch(h - 1.0 / 3.0))
    }
}

fn hue_to_rgb(p: f32, q: f32, mut h: f32) -> f32 {
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }
    if 6.0 * h < 1.0 {
        p + ((q - p) * 6.0 * h)
    } else if 2.0 * h < 1.0 {
        q
    } else if 3.0 * h < 2.0 {
        p + ((q - p) * 6.0 * ((2.0 / 3.0) - h))
    } else {
        p
    }
}

/// How a fully transparent color fills shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransparentFillBehavior {
    #[default]
    WithFillNone,
    WithFillOpacity,
}

/// A PlantUML color (`HColor`).
#[derive(Debug, Clone, PartialEq)]
pub enum HColor {
    Simple {
        color: Rgba,
        dark: Option<Box<HColor>>,
        fill: TransparentFillBehavior,
    },
    /// Gradient with a policy char: `|`, `-`, `/` or `\`.
    Gradient { c1: Rgba, c2: Rgba, policy: char },
    /// `?light:dark[:transparent]`.
    Scheme {
        light: Rgba,
        dark: Rgba,
        transparent: Option<Rgba>,
    },
    /// `automatic`: picks black or white against the background.
    Automagic,
}

impl HColor {
    pub const BLACK: HColor = HColor::simple(Rgba::rgb(0, 0, 0));
    pub const WHITE: HColor = HColor::simple(Rgba::rgb(255, 255, 255));
    pub const TRANSPARENT: HColor = HColor::simple(Rgba::new(0, 0, 0, 0));

    pub const fn simple(color: Rgba) -> HColor {
        HColor::Simple {
            color,
            dark: None,
            fill: TransparentFillBehavior::WithFillNone,
        }
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, HColor::Simple { color, .. } if color.a == 0)
    }

    pub fn is_dark(&self) -> bool {
        match self {
            HColor::Simple { color, .. } => color.gray_scale() < 128,
            _ => true,
        }
    }

    /// Base RGBA color; gradients use their first color.
    pub fn rgba(&self) -> Rgba {
        match self {
            HColor::Simple { color, .. } => *color,
            HColor::Gradient { c1, .. } => *c1,
            HColor::Scheme { light, .. } => *light,
            HColor::Automagic => Rgba::rgb(0, 0, 0),
        }
    }

    pub fn to_svg(&self) -> String {
        if self.is_transparent() {
            return "#00000000".into();
        }
        self.rgba().to_svg()
    }

    /// Source-like representation (`HColor.asString`).
    pub fn as_string(&self) -> String {
        match self {
            HColor::Simple { color, .. } if color.a == 0 => "transparent".into(),
            HColor::Simple { color, .. } if color.a == 255 => {
                format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
            }
            HColor::Simple { color, .. } => {
                format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    color.a, color.r, color.g, color.b
                )
            }
            _ => String::new(),
        }
    }

    pub fn lighten(&self, ratio: i32) -> HColor {
        self.adjust_luminance(ratio)
    }

    pub fn darken(&self, ratio: i32) -> HColor {
        self.adjust_luminance(-ratio)
    }

    fn adjust_luminance(&self, ratio: i32) -> HColor {
        match self {
            HColor::Simple { color, .. } => {
                let mut hsl = color.to_hsl();
                hsl[2] += hsl[2] * (ratio as f32 / 100.0);
                HColor::simple(Rgba::from_hsl(hsl))
            }
            _ => self.clone(),
        }
    }

    /// Black or white, whichever contrasts with this color (`HColorSimple.opposite`).
    pub fn opposite(&self) -> HColor {
        let gray = self.rgba().gray_scale();
        let v = if 255 - gray > 127 { 255 } else { 0 };
        HColor::simple(Rgba::rgb(v, v, v))
    }

    /// Resolves scheme/automatic colors against `back` (`getAppropriateColor`).
    pub fn appropriate_for(&self, back: &HColor) -> HColor {
        match self {
            HColor::Automagic => back.opposite(),
            HColor::Scheme {
                light,
                dark,
                transparent,
            } => {
                if back.is_transparent() {
                    match transparent {
                        Some(t) => HColor::simple(*t),
                        None => HColor::Simple {
                            color: *light,
                            dark: Some(Box::new(HColor::simple(*dark))),
                            fill: TransparentFillBehavior::WithFillNone,
                        },
                    }
                } else if back.is_dark() {
                    HColor::simple(*dark)
                } else {
                    HColor::simple(*light)
                }
            }
            _ => self.clone(),
        }
    }

    /// Color for dark schemes (`darkSchemeTheme`).
    pub fn dark_scheme_theme(&self) -> HColor {
        match self {
            HColor::Simple { dark: Some(d), .. } => (**d).clone(),
            _ => self.clone(),
        }
    }

    /// Attaches the dark-scheme color of a simple color (`withDark`).
    pub fn with_dark(&self, dark: HColor) -> HColor {
        match self {
            HColor::Simple { color, fill, .. } => HColor::Simple {
                color: *color,
                dark: Some(Box::new(dark)),
                fill: *fill,
            },
            _ => self.clone(),
        }
    }

    pub fn with_fill_behavior(&self, behavior: TransparentFillBehavior) -> HColor {
        match self {
            HColor::Simple { color, dark, .. } => HColor::Simple {
                color: *color,
                dark: dark.clone(),
                fill: behavior,
            },
            _ => self.clone(),
        }
    }

    /// Parses `#RGB`, `#RRGGBB`, `#RRGGBBAA`, names, `transparent`, `automatic`,
    /// `?a:b[:c]` schemes and `a-b` gradients (`HColorSet.getColorOrNull`).
    pub fn parse(s: &str) -> Option<HColor> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.eq_ignore_ascii_case("transparent") || s.eq_ignore_ascii_case("background") {
            return Some(HColor::TRANSPARENT);
        }
        if s.eq_ignore_ascii_case("automatic") {
            return Some(HColor::Automagic);
        }
        if let Some(c) = parse_simple(s) {
            return Some(HColor::simple(c));
        }
        if let Some(rest) = s.strip_prefix('?') {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 || parts.len() == 3 {
                let c0 = parse_simple(parts[0]);
                let c1 = parse_simple(parts[1]);
                if let (Some(light), Some(dark)) = (c0, c1) {
                    if parts.len() == 2 {
                        return Some(HColor::Scheme {
                            light,
                            dark,
                            transparent: None,
                        });
                    }
                    if let Some(t) = parse_simple(parts[2]) {
                        return Some(HColor::Scheme {
                            light,
                            dark,
                            transparent: Some(t),
                        });
                    }
                }
            }
        }
        for (i, c) in s.char_indices() {
            if matches!(c, '-' | '\\' | '|' | '/') {
                if let (Some(c1), Some(c2)) = (parse_simple(&s[..i]), parse_simple(&s[i + 1..])) {
                    return Some(HColor::Gradient { c1, c2, policy: c });
                }
            }
        }
        None
    }

    /// Like [`parse`](Self::parse) but falls back to white.
    pub fn parse_or_white(s: &str) -> HColor {
        Self::parse(s).unwrap_or(HColor::WHITE)
    }
}

/// All registered color names, sorted (case-insensitive).
pub fn color_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = named::NAMES.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

fn nibble(c: u8) -> Option<u32> {
    (c as char).to_digit(16)
}

fn byte(s: &[u8], off: usize) -> Option<u8> {
    Some((nibble(s[off])? << 4 | nibble(s[off + 1])?) as u8)
}

fn parse_simple(s: &str) -> Option<Rgba> {
    let s = s.strip_prefix('#').unwrap_or(s);
    let b = s.as_bytes();
    match b.len() {
        1 => {
            let d = nibble(b[0])?;
            let v = ((d << 4) | d) as u8;
            return Some(Rgba::rgb(v, v, v));
        }
        3 => {
            if let (Some(r), Some(g), Some(bl)) = (nibble(b[0]), nibble(b[1]), nibble(b[2])) {
                let e = |d: u32| ((d << 4) | d) as u8;
                return Some(Rgba::rgb(e(r), e(g), e(bl)));
            }
        }
        6 => {
            if let (Some(r), Some(g), Some(bl)) = (byte(b, 0), byte(b, 2), byte(b, 4)) {
                return Some(Rgba::rgb(r, g, bl));
            }
        }
        8 => {
            if let (Some(r), Some(g), Some(bl), Some(a)) =
                (byte(b, 0), byte(b, 2), byte(b, 4), byte(b, 6))
            {
                return Some(Rgba::new(r, g, bl, a));
            }
        }
        _ => {}
    }
    let lower = s.to_ascii_lowercase();
    named::COLORS
        .binary_search_by(|(n, _)| (*n).cmp(lower.as_str()))
        .ok()
        .map(|i| Rgba::from_u24(named::COLORS[i].1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_forms() {
        assert_eq!(
            HColor::parse("#F00"),
            Some(HColor::simple(Rgba::rgb(255, 0, 0)))
        );
        assert_eq!(HColor::parse("#A80036").unwrap().to_svg(), "#A80036");
        assert_eq!(HColor::parse("#11223380").unwrap().to_svg(), "#11223380");
        assert_eq!(
            HColor::parse("8"),
            Some(HColor::simple(Rgba::rgb(0x88, 0x88, 0x88)))
        );
    }

    #[test]
    fn parses_names_case_insensitively() {
        assert_eq!(HColor::parse("LightGray").unwrap().to_svg(), "#D3D3D3");
        assert_eq!(HColor::parse("#aliceblue").unwrap().to_svg(), "#F0F8FF");
        assert_eq!(HColor::parse("BUSINESS").unwrap().to_svg(), "#FFFFCC");
        assert_eq!(HColor::parse("nosuchcolor"), None);
    }

    #[test]
    fn transparent_and_automatic() {
        assert!(HColor::parse("transparent").unwrap().is_transparent());
        assert!(HColor::parse("#background").unwrap().is_transparent());
        assert_eq!(HColor::TRANSPARENT.to_svg(), "#00000000");
        assert_eq!(HColor::TRANSPARENT.as_string(), "transparent");
        assert_eq!(HColor::parse("automatic"), Some(HColor::Automagic));
    }

    #[test]
    fn gradient_and_scheme() {
        match HColor::parse("red|blue") {
            Some(HColor::Gradient { c1, c2, policy }) => {
                assert_eq!(c1, Rgba::rgb(255, 0, 0));
                assert_eq!(c2, Rgba::rgb(0, 0, 255));
                assert_eq!(policy, '|');
            }
            other => panic!("{other:?}"),
        }
        let scheme = HColor::parse("?black:white").unwrap();
        assert_eq!(scheme.appropriate_for(&HColor::WHITE).to_svg(), "#000000");
        assert_eq!(scheme.appropriate_for(&HColor::BLACK).to_svg(), "#FFFFFF");
        assert!(HColor::parse("?black").is_none());
        assert!(HColor::parse("red-nosuch").is_none());
    }

    #[test]
    fn scheme_with_transparent_background() {
        let with_t = HColor::parse("?red:green:blue").unwrap();
        assert_eq!(
            with_t.appropriate_for(&HColor::TRANSPARENT).to_svg(),
            "#0000FF"
        );
        let without = HColor::parse("?red:green").unwrap();
        let r = without.appropriate_for(&HColor::TRANSPARENT);
        assert_eq!(r.to_svg(), "#FF0000");
        assert_eq!(r.dark_scheme_theme().to_svg(), "#008000");
    }

    #[test]
    fn automagic_uses_opposite() {
        assert_eq!(
            HColor::Automagic.appropriate_for(&HColor::WHITE),
            HColor::BLACK
        );
        assert_eq!(
            HColor::Automagic.appropriate_for(&HColor::BLACK),
            HColor::WHITE
        );
    }

    #[test]
    fn darkness() {
        assert!(HColor::BLACK.is_dark());
        assert!(!HColor::WHITE.is_dark());
        assert!(Rgba::rgb(5, 5, 5).is_gray());
        assert_eq!(
            Rgba::rgb(255, 255, 255).distance(Rgba::rgb(0, 0, 0)),
            255_000
        );
    }

    #[test]
    fn hsl_roundtrip_and_lighten() {
        let c = Rgba::rgb(200, 100, 50);
        let back = Rgba::from_hsl(c.to_hsl());
        assert!(
            c.r.abs_diff(back.r) <= 1 && c.g.abs_diff(back.g) <= 1 && c.b.abs_diff(back.b) <= 1
        );
        let base = HColor::parse("#808080").unwrap();
        assert!(base.lighten(20).rgba().r > 0x80);
        assert!(base.darken(20).rgba().r < 0x80);
        assert_eq!(HColor::Automagic.lighten(10), HColor::Automagic);
    }

    #[test]
    fn fill_behavior_and_names() {
        let c = HColor::TRANSPARENT.with_fill_behavior(TransparentFillBehavior::WithFillOpacity);
        assert!(c.is_transparent());
        assert!(color_names().contains(&"AliceBlue"));
        assert_eq!(HColor::parse_or_white("zzz"), HColor::WHITE);
        assert_eq!(HColor::parse("#80112233").unwrap().as_string(), "#33801122");
    }
}
