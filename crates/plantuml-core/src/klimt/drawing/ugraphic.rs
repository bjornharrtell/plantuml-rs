//! Shapes and the `UGraphic` drawing context, ported from `klimt/UGraphic`, `UShape` and the SVG drivers.

use super::svg::{PathSeg, SvgGraphics, SvgOption, TextAttrs};
use crate::klimt::color::{HColor, TransparentFillBehavior};
use crate::klimt::font::{StringBounder, UFont};

/// Translation applied to subsequent shapes (`UTranslate`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UTranslate {
    pub dx: f64,
    pub dy: f64,
}

impl UTranslate {
    pub const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }
    pub const fn dx(dx: f64) -> Self {
        Self::new(dx, 0.0)
    }
    pub const fn dy(dy: f64) -> Self {
        Self::new(0.0, dy)
    }
    pub fn compose(self, o: Self) -> Self {
        Self::new(self.dx + o.dx, self.dy + o.dy)
    }
    pub fn reverse(self) -> Self {
        Self::new(-self.dx, -self.dy)
    }
    pub fn scaled(self, f: f64) -> Self {
        Self::new(self.dx * f, self.dy * f)
    }
}

/// Stroke dash and thickness (`UStroke`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UStroke {
    pub dash_visible: f64,
    pub dash_space: f64,
    pub thickness: f64,
}

impl UStroke {
    pub const fn new(dash_visible: f64, dash_space: f64, thickness: f64) -> Self {
        Self {
            dash_visible,
            dash_space,
            thickness,
        }
    }
    pub const fn with_thickness(thickness: f64) -> Self {
        Self::new(0.0, 0.0, thickness)
    }
    pub const fn simple() -> Self {
        Self::with_thickness(1.0)
    }
    pub fn dasharray(&self) -> Option<(f64, f64)> {
        (self.dash_visible != 0.0).then_some((self.dash_visible, self.dash_space))
    }
}

/// Clipping rectangle (`UClip`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl UClip {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn translate(&self, t: UTranslate) -> Self {
        Self::new(self.x + t.dx, self.y + t.dy, self.width, self.height)
    }
    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
    /// Intersection of a rectangle with the clip.
    pub fn clipped_rectangle(&self, x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let x1 = x.max(self.x);
        let y1 = y.max(self.y);
        let x2 = (x + w).min(self.x + self.width);
        let y2 = (y + h).min(self.y + self.height);
        (x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0))
    }
    /// Liang-Barsky clipping; `None` when the line is entirely outside.
    pub fn clipped_line(&self, l: (f64, f64, f64, f64)) -> Option<(f64, f64, f64, f64)> {
        let (x1, y1, x2, y2) = l;
        let (dx, dy) = (x2 - x1, y2 - y1);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q) in [
            (-dx, x1 - self.x),
            (dx, self.x + self.width - x1),
            (-dy, y1 - self.y),
            (dy, self.y + self.height - y1),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    if r > t1 {
                        return None;
                    }
                    t0 = t0.max(r);
                } else {
                    if r < t0 {
                        return None;
                    }
                    t1 = t1.min(r);
                }
            }
        }
        Some((x1 + t0 * dx, y1 + t0 * dy, x1 + t1 * dx, y1 + t1 * dy))
    }
}

/// Text decoration flags of a font configuration (`FontStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FontStyles {
    pub underline: bool,
    pub strike: bool,
    pub wave: bool,
    pub backcolor: bool,
}

/// Text style (`FontConfiguration`, reduced to what the SVG driver needs).
#[derive(Debug, Clone, PartialEq)]
pub struct FontConfiguration {
    pub font: UFont,
    pub color: HColor,
    /// Color for underline/strike lines or text background.
    pub extended_color: Option<HColor>,
    pub styles: FontStyles,
    pub attributes: Vec<(String, String)>,
}

impl FontConfiguration {
    pub fn new(font: UFont, color: HColor) -> Self {
        Self {
            font,
            color,
            extended_color: None,
            styles: FontStyles::default(),
            attributes: Vec::new(),
        }
    }
}

/// Fill and stroke shared by sized shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct URectangle {
    pub width: f64,
    pub height: f64,
    pub rx: f64,
    pub ry: f64,
    pub delta_shadow: f64,
}

impl URectangle {
    pub const fn build(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            rx: 0.0,
            ry: 0.0,
            delta_shadow: 0.0,
        }
    }
    pub fn rounded(mut self, round: f64) -> Self {
        self.rx = round;
        self.ry = round;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ULine {
    pub dx: f64,
    pub dy: f64,
    pub delta_shadow: f64,
}

impl ULine {
    pub const fn new(dx: f64, dy: f64) -> Self {
        Self {
            dx,
            dy,
            delta_shadow: 0.0,
        }
    }
    pub const fn hline(dx: f64) -> Self {
        Self::new(dx, 0.0)
    }
    pub const fn vline(dy: f64) -> Self {
        Self::new(0.0, dy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UEllipse {
    pub width: f64,
    pub height: f64,
    pub start: f64,
    pub extend: f64,
    pub delta_shadow: f64,
}

impl UEllipse {
    pub const fn build(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            start: 0.0,
            extend: 0.0,
            delta_shadow: 0.0,
        }
    }
    pub const fn arc(width: f64, height: f64, start: f64, extend: f64) -> Self {
        Self {
            width,
            height,
            start,
            extend,
            delta_shadow: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UPolygon {
    pub points: Vec<(f64, f64)>,
    pub delta_shadow: f64,
}

impl UPolygon {
    pub fn new(points: Vec<(f64, f64)>) -> Self {
        Self {
            points,
            delta_shadow: 0.0,
        }
    }
    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push((x, y));
    }
    pub fn translate(&self, dx: f64, dy: f64) -> Self {
        Self {
            points: self.points.iter().map(|&(x, y)| (x + dx, y + dy)).collect(),
            delta_shadow: self.delta_shadow,
        }
    }
    pub fn min_max(&self) -> Option<(f64, f64, f64, f64)> {
        let mut it = self.points.iter();
        let &(x, y) = it.next()?;
        Some(it.fold((x, y, x, y), |(a, b, c, d), &(x, y)| {
            (a.min(x), b.min(y), c.max(x), d.max(y))
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UPath {
    pub segments: Vec<PathSeg>,
    pub delta_shadow: f64,
}

impl UPath {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn move_to(&mut self, x: f64, y: f64) {
        if let Some(PathSeg::MoveTo(a, b) | PathSeg::LineTo(a, b)) = self.segments.last() {
            if matches!(self.segments.last(), Some(PathSeg::MoveTo(..))) && *a == x && *b == y {
                return;
            }
        }
        self.segments.push(PathSeg::MoveTo(x, y));
    }
    pub fn line_to(&mut self, x: f64, y: f64) {
        self.segments.push(PathSeg::LineTo(x, y));
    }
    pub fn cubic_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        self.segments
            .push(PathSeg::CubicTo(c1x, c1y, c2x, c2y, x, y));
    }
    /// Quadratic curves are stored as cubics with a doubled control point, like Java.
    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) {
        self.cubic_to(cx, cy, cx, cy, x, y);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn arc_to(&mut self, rx: f64, ry: f64, rot: f64, large: bool, sweep: bool, x: f64, y: f64) {
        self.segments
            .push(PathSeg::ArcTo(rx, ry, rot, large, sweep, x, y));
    }
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
    pub fn translate(&self, dx: f64, dy: f64) -> Self {
        let segs = self
            .segments
            .iter()
            .map(|s| match *s {
                PathSeg::MoveTo(a, b) => PathSeg::MoveTo(a + dx, b + dy),
                PathSeg::LineTo(a, b) => PathSeg::LineTo(a + dx, b + dy),
                PathSeg::QuadTo(a, b, c, d) => PathSeg::QuadTo(a + dx, b + dy, c + dx, d + dy),
                PathSeg::CubicTo(a, b, c, d, e, f) => {
                    PathSeg::CubicTo(a + dx, b + dy, c + dx, d + dy, e + dx, f + dy)
                }
                PathSeg::ArcTo(rx, ry, r, l, sw, x, y) => {
                    PathSeg::ArcTo(rx, ry, r, l, sw, x + dx, y + dy)
                }
                PathSeg::Close => PathSeg::Close,
            })
            .collect();
        Self {
            segments: segs,
            delta_shadow: self.delta_shadow,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UText {
    pub text: String,
    pub font: FontConfiguration,
    pub orientation: i32,
}

impl UText {
    pub fn new(text: impl Into<String>, font: FontConfiguration) -> Self {
        Self {
            text: text.into(),
            font,
            orientation: 0,
        }
    }
}

/// Anything `UGraphic::draw` accepts.
#[derive(Debug, Clone, PartialEq)]
pub enum UShape {
    Rectangle(URectangle),
    Line(ULine),
    Ellipse(UEllipse),
    Polygon(UPolygon),
    Path(UPath),
    Text(UText),
    Comment(String),
}

macro_rules! impl_from {
    ($($t:ident => $v:ident),*) => {$(
        impl From<$t> for UShape { fn from(s: $t) -> Self { UShape::$v(s) } }
    )*};
}
impl_from!(URectangle => Rectangle, ULine => Line, UEllipse => Ellipse,
    UPolygon => Polygon, UPath => Path, UText => Text);

/// Current drawing state (`UParam`).
#[derive(Debug, Clone)]
struct Param {
    color: HColor,
    backcolor: HColor,
    stroke: UStroke,
    clip: Option<UClip>,
    hidden: bool,
}

/// Drawing context writing SVG (`UGraphicSvg`). `apply_*` methods return modified copies
/// sharing the same output through [`UGraphic::scope`].
pub struct UGraphic<'a> {
    svg: &'a mut SvgGraphics,
    bounder: &'a dyn StringBounder,
    translate: UTranslate,
    param: Param,
}

impl<'a> UGraphic<'a> {
    pub fn new(svg: &'a mut SvgGraphics, bounder: &'a dyn StringBounder) -> Self {
        Self {
            svg,
            bounder,
            translate: UTranslate::default(),
            param: Param {
                color: HColor::BLACK,
                backcolor: HColor::BLACK,
                stroke: UStroke::simple(),
                clip: None,
                hidden: false,
            },
        }
    }

    pub fn string_bounder(&self) -> &dyn StringBounder {
        self.bounder
    }

    /// Reborrow with independent state, like Java's immutable `apply`.
    pub fn scope(&mut self) -> UGraphic<'_> {
        UGraphic {
            svg: self.svg,
            bounder: self.bounder,
            translate: self.translate,
            param: self.param.clone(),
        }
    }

    pub fn apply_translate(&mut self, t: UTranslate) -> UGraphic<'_> {
        let mut u = self.scope();
        u.translate = u.translate.compose(t);
        u
    }
    pub fn apply_color(&mut self, c: HColor) -> UGraphic<'_> {
        let mut u = self.scope();
        u.param.color = c;
        u
    }
    pub fn apply_back_color(&mut self, c: HColor) -> UGraphic<'_> {
        let mut u = self.scope();
        u.param.backcolor = c;
        u
    }
    pub fn apply_stroke(&mut self, s: UStroke) -> UGraphic<'_> {
        let mut u = self.scope();
        u.param.stroke = s;
        u
    }
    /// Clip is given in the current (translated) coordinates.
    pub fn apply_clip(&mut self, c: UClip) -> UGraphic<'_> {
        let mut u = self.scope();
        u.param.clip = Some(c.translate(u.translate));
        u
    }
    pub fn apply_hidden(&mut self, hidden: bool) -> UGraphic<'_> {
        let mut u = self.scope();
        u.param.hidden = hidden;
        u
    }

    pub fn start_group(&mut self, attrs: &[(&str, String)]) {
        self.svg.start_group(attrs);
    }
    pub fn close_group(&mut self) {
        self.svg.close_group();
    }

    fn apply_fill(&mut self, color: &HColor) {
        match color {
            HColor::Gradient { c1, c2, policy } => {
                let id = self
                    .svg
                    .create_gradient(&c1.to_svg(), &c2.to_svg(), *policy);
                self.svg.set_fill_color(Some(&format!("url(#{id})")));
            }
            HColor::Simple {
                fill: TransparentFillBehavior::WithFillOpacity,
                ..
            } if color.is_transparent() => self.svg.set_fill_color_opacity("#00000000"),
            _ => self.svg.set_fill_color(Some(&color.to_svg())),
        }
    }

    fn apply_stroke_color(&mut self, color: &HColor) {
        match color {
            HColor::Gradient { c1, c2, policy } => {
                let id = self
                    .svg
                    .create_gradient(&c1.to_svg(), &c2.to_svg(), *policy);
                self.svg.set_stroke_color(Some(&format!("url(#{id})")));
            }
            _ => self.svg.set_stroke_color(Some(&color.to_svg())),
        }
    }

    fn apply_stroke_width(&mut self) {
        let s = self.param.stroke;
        self.svg.set_stroke_width(s.thickness, s.dasharray());
    }

    /// Draws a shape at the current translation.
    pub fn draw(&mut self, shape: impl Into<UShape>) {
        let (x, y) = (self.translate.dx, self.translate.dy);
        self.svg.set_hidden(self.param.hidden);
        match shape.into() {
            UShape::Rectangle(r) => self.draw_rectangle(&r, x, y),
            UShape::Line(l) => self.draw_line(&l, x, y),
            UShape::Ellipse(e) => self.draw_ellipse(&e, x, y),
            UShape::Polygon(p) => self.draw_polygon(&p, x, y),
            UShape::Path(p) => self.draw_path(&p, x, y),
            UShape::Text(t) => self.draw_text(&t, x, y),
            UShape::Comment(c) => self.svg.add_comment(&c),
        }
        self.svg.set_hidden(false);
    }

    fn draw_rectangle(&mut self, r: &URectangle, mut x: f64, mut y: f64) {
        let (mut w, mut h) = (r.width, r.height);
        let (c, b) = (self.param.color.clone(), self.param.backcolor.clone());
        self.apply_fill(&b);
        self.apply_stroke_color(&c);
        self.apply_stroke_width();
        if let Some(clip) = self.param.clip {
            (x, y, w, h) = clip.clipped_rectangle(x, y, w, h);
            if h <= 0.0 {
                return;
            }
        }
        self.svg
            .rectangle(x, y, w, h, r.rx / 2.0, r.ry / 2.0, r.delta_shadow);
    }

    fn draw_line(&mut self, l: &ULine, x: f64, y: f64) {
        let mut line = (x, y, x + l.dx, y + l.dy);
        if let Some(clip) = self.param.clip {
            match clip.clipped_line(line) {
                Some(c) => line = c,
                None => return,
            }
        }
        let c = self.param.color.clone();
        self.apply_stroke_color(&c);
        self.apply_stroke_width();
        self.svg
            .line(line.0, line.1, line.2, line.3, l.delta_shadow);
    }

    fn draw_ellipse(&mut self, e: &UEllipse, x: f64, y: f64) {
        let (w, h) = (e.width, e.height);
        if let Some(clip) = self.param.clip {
            if !clip.is_inside(x, y) || !clip.is_inside(x + w, y + h) {
                return;
            }
        }
        let (c, b) = (self.param.color.clone(), self.param.backcolor.clone());
        self.apply_stroke_color(&c);
        self.apply_fill(&b);
        self.apply_stroke_width();
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        if e.start == 0.0 && e.extend == 0.0 {
            self.svg.ellipse(cx, cy, w / 2.0, h / 2.0, e.delta_shadow);
            return;
        }
        let start = e.start + 90.0;
        let pt = |deg: f64| {
            let r = deg.to_radians();
            (cx + r.sin() * w / 2.0, cy + r.cos() * h / 2.0)
        };
        let (p1, p2) = if e.extend > 0.0 {
            (pt(start), pt(start + e.extend))
        } else {
            (pt(start + e.extend), pt(start))
        };
        self.svg
            .arc_ellipse(w / 2.0, h / 2.0, p1.0, p1.1, p2.0, p2.1);
    }

    fn draw_polygon(&mut self, p: &UPolygon, x: f64, y: f64) {
        let pts: Vec<f64> = p.points.iter().flat_map(|&(a, b)| [a + x, b + y]).collect();
        if let Some(clip) = self.param.clip {
            if pts.chunks_exact(2).any(|c| !clip.is_inside(c[0], c[1])) {
                return;
            }
        }
        let (c, b) = (self.param.color.clone(), self.param.backcolor.clone());
        self.apply_fill(&b);
        self.apply_stroke_color(&c);
        self.apply_stroke_width();
        self.svg.polygon(p.delta_shadow, &pts);
    }

    fn draw_path(&mut self, p: &UPath, x: f64, y: f64) {
        if let Some(clip) = self.param.clip {
            if !clip.is_inside(x, y) {
                return;
            }
        }
        let (c, b) = (self.param.color.clone(), self.param.backcolor.clone());
        if c == b && !matches!(c, HColor::Gradient { .. }) {
            self.svg.set_fill_color(Some(&c.to_svg()));
            self.svg.set_stroke_color(Some(""));
            self.svg.set_stroke_width(0.0, None);
        } else {
            self.apply_fill(&b);
            self.apply_stroke_color(&c);
            self.apply_stroke_width();
        }
        self.svg.path(x, y, &p.segments, p.delta_shadow);
    }

    fn draw_text(&mut self, t: &UText, mut x: f64, y: f64) {
        if let Some(clip) = self.param.clip {
            if !clip.is_inside(x, y) {
                return;
            }
        }
        let fc = &t.font;
        if fc.color.is_transparent() {
            return;
        }
        let font = &fc.font;
        let face = font.face;
        let bold = face.is_bold();
        let weight = bold.then(|| face.weight.max(700).to_string());
        let style = face.italic.then_some("italic");
        let mut text = t.text.clone();
        if text.trim().is_empty() {
            text = text.replace(' ', "\u{a0}");
        }
        while text.starts_with(' ') {
            x += self.bounder.calculate_dimension(font, " ").width;
            text.remove(0);
        }
        let text = text.trim_matches(|c: char| c.is_whitespace() && c != '\u{a0}');
        let dim = self.bounder.calculate_dimension(font, text);
        let mut deco = Vec::new();
        let mut extra_lines: Vec<(HColor, f64)> = Vec::new();
        if fc.styles.underline {
            match &fc.extended_color {
                None => deco.push("underline"),
                Some(c) => extra_lines.push((c.clone(), font.size2d() / 14.0)),
            }
        }
        if fc.styles.strike {
            match &fc.extended_color {
                None => deco.push("line-through"),
                Some(c) => extra_lines.push((c.clone(), -font.size2d() / 4.0)),
            }
        }
        if fc.styles.wave {
            deco.push("wavy underline");
        }
        let decoration = deco.join(" ");
        let mut back = None;
        if fc.styles.backcolor {
            if let Some(b) = &fc.extended_color {
                if let HColor::Gradient { c1, c2, policy } = b {
                    let id = self
                        .svg
                        .create_gradient(&c1.to_svg(), &c2.to_svg(), *policy);
                    self.svg.set_fill_color(Some(&format!("url(#{id})")));
                    self.svg.set_stroke_color(None);
                    self.svg.rectangle(
                        x,
                        y - dim.height + 2.0,
                        dim.width,
                        dim.height,
                        0.0,
                        0.0,
                        0.0,
                    );
                } else {
                    back = Some(b.to_svg());
                }
            }
        }
        self.svg.set_fill_color(Some(&fc.color.to_svg()));
        let family = font.svg_family();
        let attrs = TextAttrs {
            family: Some(&family),
            size: font.size,
            weight: weight.as_deref(),
            style,
            decoration: (!decoration.is_empty()).then_some(decoration.as_str()),
            text_length: dim.width,
            extra: fc.attributes.clone(),
            back_color: back.as_deref(),
            orientation: t.orientation,
        };
        self.svg.text(text, x, y, &attrs);
        for (c, dy) in extra_lines {
            self.svg.set_stroke_color(Some(&c.to_svg()));
            self.svg.set_stroke_width(font.size2d() / 28.0, None);
            self.svg.line(x, y + dy, x + dim.width, y + dy, 0.0);
        }
    }
}

/// Convenience: renders a drawing closure into an SVG string.
pub fn render_svg(
    option: SvgOption,
    seed: i64,
    bounder: &dyn StringBounder,
    f: impl FnOnce(&mut UGraphic<'_>),
) -> String {
    let mut svg = SvgGraphics::new(seed, option);
    {
        let mut ug = UGraphic::new(&mut svg, bounder);
        f(&mut ug);
    }
    svg.to_svg()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::color::Rgba;
    use crate::klimt::font::EmbeddedFontBounder;

    fn svg(f: impl FnOnce(&mut UGraphic<'_>)) -> String {
        render_svg(SvgOption::default(), 0, &EmbeddedFontBounder, f)
    }

    #[test]
    fn rectangle_with_translate_and_colors() {
        let out = svg(|ug| {
            let red = HColor::parse("#FF0000").unwrap();
            let mut u = ug.apply_color(HColor::BLACK);
            let mut u = u.apply_back_color(red);
            let mut u = u.apply_translate(UTranslate::new(5.0, 6.0));
            u.draw(URectangle::build(10.0, 20.0).rounded(10.0));
        });
        assert!(out.contains(r##"<rect fill="#F00" height="20""##), "{out}");
        assert!(
            out.contains(r#"x="5""#) && out.contains(r#"y="6""#),
            "{out}"
        );
        assert!(out.contains(r#"rx="5""#), "{out}");
    }

    #[test]
    fn line_ellipse_polygon_path() {
        let out = svg(|ug| {
            ug.draw(ULine::new(10.0, 10.0));
            ug.draw(UEllipse::build(10.0, 6.0));
            ug.draw(UEllipse::arc(10.0, 10.0, 0.0, 90.0));
            let mut p = UPolygon::default();
            p.add_point(0.0, 0.0);
            p.add_point(5.0, 0.0);
            p.add_point(5.0, 5.0);
            ug.draw(p);
            let mut path = UPath::new();
            path.move_to(0.0, 0.0);
            path.line_to(3.0, 3.0);
            path.quad_to(1.0, 1.0, 2.0, 2.0);
            ug.draw(path);
            ug.draw(UShape::Comment("hi".into()));
        });
        for s in [
            "<line",
            "<ellipse",
            "<polygon",
            "<path",
            "A5,5 0 0 0",
            "<!--hi-->",
        ] {
            assert!(out.contains(s), "{s} in {out}");
        }
    }

    #[test]
    fn text_and_decorations() {
        let out = svg(|ug| {
            let mut fc = FontConfiguration::new(UFont::sans_serif(14), HColor::BLACK);
            fc.styles.underline = true;
            fc.styles.strike = true;
            fc.extended_color = Some(HColor::parse("#00FF00").unwrap());
            ug.draw(UText::new("  Hello ", fc));
            let mut fc = FontConfiguration::new(UFont::sans_serif(14), HColor::BLACK);
            fc.styles.underline = true;
            ug.draw(UText::new("Hi", fc.clone()));
            let mut t = UText::new("T", fc);
            t.font.color = HColor::TRANSPARENT;
            ug.draw(t);
        });
        assert!(out.contains(">Hello</text>"), "{out}");
        assert!(out.contains("<line"), "{out}");
        assert!(out.contains("underline"), "{out}");
    }

    #[test]
    fn clipping_and_hidden() {
        let out = svg(|ug| {
            let mut u = ug.apply_clip(UClip::new(0.0, 0.0, 10.0, 10.0));
            u.draw(URectangle::build(100.0, 100.0));
            u.draw(URectangle::build(100.0, 0.0));
            u.draw(ULine::new(100.0, 0.0));
            u.apply_translate(UTranslate::new(50.0, 50.0))
                .draw(ULine::new(1.0, 1.0));
            u.draw(UEllipse::build(100.0, 100.0));
            let mut p = UPolygon::default();
            p.add_point(50.0, 50.0);
            u.draw(p);
            u.draw(UPath::new());
            let mut h = ug.apply_hidden(true);
            h.draw(URectangle::build(5.0, 5.0));
        });
        assert!(out.contains(r#"width="10""#), "{out}");
        assert!(!out.contains(r#"width="100""#), "{out}");
        assert!(!out.contains(r#"width="5""#), "{out}");
    }

    #[test]
    fn gradients_and_groups() {
        let out = svg(|ug| {
            let g = HColor::Gradient {
                c1: Rgba::rgb(255, 0, 0),
                c2: Rgba::rgb(0, 0, 255),
                policy: '|',
            };
            ug.start_group(&[("class", "x".into())]);
            let mut u = ug.apply_back_color(g.clone());
            let mut u = u.apply_color(g.clone());
            u.draw(URectangle::build(5.0, 5.0));
            u.draw(UEllipse::build(5.0, 5.0));
            u.draw(UPath::new());
            ug.close_group();
        });
        assert!(out.contains("linearGradient"), "{out}");
        assert!(out.contains(r#"class="x""#), "{out}");
    }

    #[test]
    fn helpers() {
        let t = UTranslate::dx(1.0).compose(UTranslate::dy(2.0));
        assert_eq!(t.reverse(), UTranslate::new(-1.0, -2.0));
        assert_eq!(t.scaled(2.0), UTranslate::new(2.0, 4.0));
        assert_eq!(UStroke::new(3.0, 2.0, 1.0).dasharray(), Some((3.0, 2.0)));
        assert_eq!(UStroke::simple().dasharray(), None);
        let c = UClip::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            c.clipped_line((-5.0, 5.0, 5.0, 5.0)),
            Some((0.0, 5.0, 5.0, 5.0))
        );
        assert_eq!(c.clipped_line((20.0, 5.0, 30.0, 5.0)), None);
        assert_eq!(c.clipped_line((5.0, 20.0, 5.0, 30.0)), None);
        let p = UPolygon::new(vec![(1.0, 2.0), (3.0, 1.0)]).translate(1.0, 1.0);
        assert_eq!(p.min_max(), Some((2.0, 2.0, 4.0, 3.0)));
        assert_eq!(UPath::new().translate(1.0, 1.0), UPath::new());
    }
}
