//! Fonts and text measurement, ported from `klimt/font` and
//! `klimt/drawing/font/StringBounderEmbeddedFont`.

pub mod config;
mod metrics;

/// Font weight and style (`UFontFace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontFace {
    pub italic: bool,
    /// CSS weight, 100..=900.
    pub weight: u16,
}

impl FontFace {
    pub const NORMAL: FontFace = FontFace {
        italic: false,
        weight: 400,
    };
    pub const BOLD: FontFace = FontFace {
        italic: false,
        weight: 700,
    };
    pub const ITALIC: FontFace = FontFace {
        italic: true,
        weight: 400,
    };
    pub const BOLD_ITALIC: FontFace = FontFace {
        italic: true,
        weight: 700,
    };

    /// CSS keyword or number to a face (`UFontFace.fromCssWeight`).
    pub fn from_css_weight(s: &str) -> Option<FontFace> {
        let w = match s.trim().to_lowercase().as_str() {
            "normal" => 400,
            "bold" => 700,
            "lighter" => 300,
            "bolder" => 800,
            n => n.parse().ok()?,
        };
        Some(FontFace {
            italic: false,
            weight: w,
        })
    }

    pub fn is_bold(self) -> bool {
        self.weight >= 700
    }

    fn style_index(self) -> usize {
        usize::from(self.is_bold()) + 2 * usize::from(self.italic)
    }
}

impl Default for FontFace {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// A font: family definition, face and integer size (`UFont`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UFont {
    family: String,
    pub face: FontFace,
    pub size: i32,
}

impl UFont {
    pub fn new(family: &str, face: FontFace, size: i32) -> Self {
        Self {
            family: family.to_string(),
            face,
            size,
        }
    }

    pub fn sans_serif(size: i32) -> Self {
        Self::new("SansSerif", FontFace::NORMAL, size)
    }

    pub fn serif(size: i32) -> Self {
        Self::new("Serif", FontFace::NORMAL, size)
    }

    pub fn monospace(size: i32) -> Self {
        Self::new("Monospaced", FontFace::NORMAL, size)
    }

    pub fn with_size(&self, size: f32) -> Self {
        Self {
            size: size as i32,
            ..self.clone()
        }
    }

    pub fn with_face(&self, face: FontFace) -> Self {
        Self {
            face,
            ..self.clone()
        }
    }

    pub fn size2d(&self) -> f64 {
        f64::from(self.size)
    }

    /// Full definition as given, e.g. `SansSerif` or `"Verdana, Arial"`.
    pub fn definition(&self) -> &str {
        &self.family
    }

    /// Family as written in SVG `font-family` (`FontStack.getSvgFamily`).
    pub fn svg_family(&self) -> String {
        match self.family.as_str() {
            "Serif" => "serif".into(),
            "SansSerif" => "sans-serif".into(),
            "Monospaced" => "monospace".into(),
            other => other.replace('"', "'"),
        }
    }

    fn first_family(&self) -> String {
        let first = self.family.split(',').next().unwrap_or("");
        first.replace(['"', '\''], " ").trim().to_lowercase()
    }

    /// Java uses a logical (composite) font for anything but an explicit Arial.
    fn is_logical(&self) -> bool {
        let f = self.first_family();
        f != "arial" && f != "liberation sans"
    }
}

/// Width and height of a text.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimension {
    pub width: f64,
    pub height: f64,
}

impl Dimension {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn delta(self, dw: f64, dh: f64) -> Self {
        Self::new(self.width + dw, self.height + dh)
    }

    /// Stacks `bottom` under this one.
    pub fn merge_tb(self, bottom: Dimension) -> Self {
        Self::new(self.width.max(bottom.width), self.height + bottom.height)
    }

    /// Puts `right` beside this one.
    pub fn merge_lr(self, right: Dimension) -> Self {
        Self::new(self.width + right.width, self.height.max(right.height))
    }

    pub fn at_least(self, min_width: f64, min_height: f64) -> Self {
        Self::new(self.width.max(min_width), self.height.max(min_height))
    }
}

/// Measures text (`StringBounder`).
pub trait StringBounder {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> Dimension;

    fn descent(&self, font: &UFont, _text: &str) -> f64 {
        font.size2d() / 4.5
    }
}

/// Measures with Liberation Sans metrics, reproducing Java on Windows
/// (`StringBounderEmbeddedFont`). Other families are measured as `SansSerif`.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmbeddedFontBounder;

const LOGICAL_MIN_ASCENT: f64 = 2059.0 / 2048.0;
const LOGICAL_MIN_DESCENT: f64 = 450.0 / 2048.0;

fn is_invisible(cp: u32) -> bool {
    matches!(cp, 9 | 10 | 13 | 0x2028 | 0x2029)
}

impl EmbeddedFontBounder {
    fn relative_advance(font: &UFont, logical: bool, cp: u32) -> f64 {
        let t = metrics::tables();
        let style = font.face.style_index();
        let face = &t.faces[style];
        if is_invisible(cp) {
            return 0.0;
        }
        if logical {
            if let Some(w) = t.measured.width(cp, style) {
                return f64::from(w) / 2048.0;
            }
        }
        if let Some(a) = face.advance(cp) {
            return f64::from(a) / f64::from(face.units_per_em);
        }
        if logical {
            return metrics::table_width(cp) / 16.0;
        }
        f64::from(face.missing_advance) / f64::from(face.units_per_em)
    }
}

impl StringBounder for EmbeddedFontBounder {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> Dimension {
        if text.is_empty() {
            return Dimension::default();
        }
        let t = metrics::tables();
        let face = &t.faces[font.face.style_index()];
        let upm = f64::from(face.units_per_em);
        let logical = font.is_logical();
        let size = font.size2d();
        let width: f64 = text
            .chars()
            .map(|c| Self::relative_advance(font, logical, c as u32))
            .sum();
        let mut ascent = f64::from(face.ascent) / upm;
        let mut descent = f64::from(face.descent) / upm;
        if logical {
            ascent = ascent.max(LOGICAL_MIN_ASCENT);
            descent = descent.max(LOGICAL_MIN_DESCENT);
        }
        let leading = f64::from(face.line_gap) / upm;
        Dimension::new(width * size, (ascent + descent + leading) * size)
    }

    fn descent(&self, font: &UFont, _text: &str) -> f64 {
        let face = &metrics::tables().faces[font.face.style_index()];
        let mut d = f64::from(face.descent) / f64::from(face.units_per_em);
        if font.is_logical() {
            d = d.max(LOGICAL_MIN_DESCENT);
        }
        d * font.size2d()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sans_serif_width_matches_java_reference() {
        let b = EmbeddedFontBounder;
        let f = UFont::sans_serif(14);
        // textLength from Vega reference SVGs (27.213; Java AWT differs by ~0.05%).
        let d = b.calculate_dimension(&f, "Test");
        assert!((d.width - 27.213).abs() < 0.05, "{}", d.width);
    }

    #[test]
    fn heights_and_descent() {
        let b = EmbeddedFontBounder;
        let f = UFont::sans_serif(12);
        let d = b.calculate_dimension(&f, "x");
        assert!((d.height - 15.09375).abs() < 1e-6, "{}", d.height);
        assert!((b.descent(&f, "x") - 450.0 / 2048.0 * 12.0).abs() < 1e-9);
        let arial = UFont::new("Arial", FontFace::NORMAL, 12);
        assert!(b.calculate_dimension(&arial, "x").height < d.height + 1.0);
        assert!(b.descent(&arial, "x") < b.descent(&f, "x"));
    }

    #[test]
    fn empty_and_invisible() {
        let b = EmbeddedFontBounder;
        let f = UFont::sans_serif(12);
        assert_eq!(b.calculate_dimension(&f, ""), Dimension::default());
        assert_eq!(b.calculate_dimension(&f, "\t").width, 0.0);
    }

    #[test]
    fn bold_italic_and_fallbacks() {
        let b = EmbeddedFontBounder;
        let reg = UFont::sans_serif(12);
        let bold = reg.with_face(FontFace::BOLD);
        assert!(
            b.calculate_dimension(&bold, "Hello").width
                > b.calculate_dimension(&reg, "Hello").width
        );
        let it = reg.with_face(FontFace::BOLD_ITALIC);
        assert!(b.calculate_dimension(&it, "Hello").width > 0.0);
        // beyond BMP and CJK use the generic table
        assert!(b.calculate_dimension(&reg, "\u{1F600}").width > 0.0);
        assert!(b.calculate_dimension(&reg, "漢").width > 0.0);
        let arial = UFont::new("\"Liberation Sans\", x", FontFace::NORMAL, 12);
        assert!(b.calculate_dimension(&arial, "\u{3042}").width > 0.0);
    }

    #[test]
    fn font_helpers() {
        assert_eq!(UFont::serif(10).svg_family(), "serif");
        assert_eq!(UFont::monospace(10).svg_family(), "monospace");
        assert_eq!(UFont::sans_serif(10).svg_family(), "sans-serif");
        let f = UFont::new("\"Foo Bar\", Baz", FontFace::NORMAL, 9);
        assert_eq!(f.svg_family(), "'Foo Bar', Baz");
        assert_eq!(f.definition(), "\"Foo Bar\", Baz");
        assert_eq!(f.with_size(11.7).size, 11);
        assert!(FontFace::BOLD.is_bold() && !FontFace::ITALIC.is_bold());
        assert_eq!(FontFace::default(), FontFace::NORMAL);
    }
}
