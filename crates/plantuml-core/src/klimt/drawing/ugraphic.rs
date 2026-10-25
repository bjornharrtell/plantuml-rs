//! Shapes and the `UGraphic` drawing context, ported from `klimt/UGraphic`, `UShape` and the SVG drivers.

use super::svg::{PathSeg, SvgGraphics, SvgOption, TextAttrs};
use crate::klimt::color::{HColor, TransparentFillBehavior};
use crate::klimt::font::config::{FontConfiguration, FontStyle};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::MinMax;
use crate::klimt::shape::{Stencil, UHorizontalLine};
use std::cell::RefCell;
use std::rc::Rc;

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
    /// Rotation of the vector around its start (`rotate`).
    pub fn rotate(&self, theta: f64) -> ULine {
        if theta == 0.0 {
            return self.clone();
        }
        let (sin, cos) = theta.sin_cos();
        ULine {
            dx: self.dx * cos - self.dy * sin,
            dy: self.dx * sin + self.dy * cos,
            delta_shadow: self.delta_shadow,
        }
    }

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
    /// Rotates every point around the origin.
    pub fn rotate(&mut self, theta: f64) {
        if theta == 0.0 {
            return;
        }
        let (sin, cos) = theta.sin_cos();
        for p in &mut self.points {
            *p = (p.0 * cos - p.1 * sin, p.0 * sin + p.1 * cos);
        }
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
    pub fn close_path(&mut self) {
        self.segments.push(PathSeg::Close);
    }
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
    /// Bounding box of all control points (arcs count by their end point).
    pub fn min_max(&self) -> Option<(f64, f64, f64, f64)> {
        let mut r: Option<(f64, f64, f64, f64)> = None;
        let mut add = |x: f64, y: f64| {
            r = Some(match r {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            });
        };
        for s in &self.segments {
            match *s {
                PathSeg::MoveTo(a, b) | PathSeg::LineTo(a, b) => add(a, b),
                PathSeg::QuadTo(a, b, c, d) => {
                    add(a, b);
                    add(c, d);
                }
                PathSeg::CubicTo(a, b, c, d, e, f) => {
                    add(a, b);
                    add(c, d);
                    add(e, f);
                }
                PathSeg::ArcTo(.., x, y) => add(x, y),
                PathSeg::Close => {}
            }
        }
        r
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
    Text(Box<UText>),
    HLine(UHorizontalLine),
    /// Invisible shape only extending the bounds (`UEmpty`).
    Empty(f64, f64),
    Comment(String),
}

macro_rules! impl_from {
    ($($t:ident => $v:ident),*) => {$(
        impl From<$t> for UShape { fn from(s: $t) -> Self { UShape::$v(s) } }
    )*};
}
impl_from!(URectangle => Rectangle, ULine => Line, UEllipse => Ellipse,
    UPolygon => Polygon, UPath => Path, UHorizontalLine => HLine);

impl From<UText> for UShape {
    fn from(t: UText) -> Self {
        UShape::Text(Box::new(t))
    }
}

/// Current drawing state (`UParam`).
#[derive(Debug, Clone)]
struct Param {
    color: HColor,
    backcolor: HColor,
    stroke: UStroke,
    clip: Option<UClip>,
    hidden: bool,
}

/// Shared handle on the SVG output; every drawing context created by `apply_*` writes to it.
#[derive(Clone)]
struct SvgHandle(Rc<RefCell<SvgGraphics>>);

macro_rules! svg_forward {
    ($($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)*) => {
        #[allow(clippy::too_many_arguments)]
        impl SvgHandle {
            $(fn $name(&self, $($arg: $ty),*) $(-> $ret)? {
                self.0.borrow_mut().$name($($arg),*)
            })*
        }
    };
}

svg_forward! {
    start_group(attrs: &[(&str, String)]);
    close_group();
    create_gradient(c1: &str, c2: &str, policy: char) -> String;
    set_fill_color(fill: Option<&str>);
    set_fill_color_opacity(fill: &str);
    set_stroke_color(stroke: Option<&str>);
    set_stroke_width(width: f64, dash: Option<(f64, f64)>);
    set_hidden(hidden: bool);
    add_comment(comment: &str);
    rectangle(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64, shadow: f64);
    ellipse(cx: f64, cy: f64, rx: f64, ry: f64, shadow: f64);
    arc_ellipse(rx: f64, ry: f64, x1: f64, y1: f64, x2: f64, y2: f64);
    line(x1: f64, y1: f64, x2: f64, y2: f64, shadow: f64);
    polygon(shadow: f64, points: &[f64]);
    path(x: f64, y: f64, segs: &[PathSeg], shadow: f64);
    text(text: &str, x: f64, y: f64, attrs: &TextAttrs);
}

/// Drawing context writing SVG (`UGraphicSvg`). `apply_*` methods return modified copies
/// sharing the same output through [`UGraphic::scope`].
#[derive(Clone)]
pub struct UGraphic {
    svg: SvgHandle,
    bounder: Rc<dyn StringBounder>,
    translate: UTranslate,
    param: Param,
    stencil: Option<Rc<StencilLayer>>,
    default_background: HColor,
    /// When set, drawing only measures bounds (`LimitFinder`).
    limit: Option<Rc<RefCell<MinMax>>>,
    /// Whether tiles draw their background layer (`Context2D.isBackground`).
    background_layer: bool,
}

/// Stencil installed by [`UGraphic::apply_stencil`] (`UGraphicStencil`).
struct StencilLayer {
    stencil: Rc<dyn Stencil>,
    default_stroke: UStroke,
    /// Position of the drawing context when the stencil was installed.
    origin: UTranslate,
    parent: Option<Rc<StencilLayer>>,
}

impl UGraphic {
    pub fn new(svg: Rc<RefCell<SvgGraphics>>, bounder: Rc<dyn StringBounder>) -> Self {
        Self {
            svg: SvgHandle(svg),
            bounder,
            translate: UTranslate::default(),
            param: Param {
                color: HColor::TRANSPARENT,
                backcolor: HColor::TRANSPARENT,
                stroke: UStroke::simple(),
                clip: None,
                hidden: false,
            },
            stencil: None,
            default_background: HColor::WHITE,
            limit: None,
            background_layer: false,
        }
    }

    pub fn is_background_layer(&self) -> bool {
        self.background_layer
    }

    pub fn apply_background_layer(&self, background: bool) -> UGraphic {
        let mut u = self.clone();
        u.background_layer = background;
        u
    }

    /// Background assumed when the current back color is transparent.
    /// Whether shapes only feed a bounding box (`LimitFinder`).
    pub fn is_finding_limits(&self) -> bool {
        self.limit.is_some()
    }

    pub fn default_background(&self) -> &HColor {
        &self.default_background
    }

    pub fn with_default_background(mut self, bg: HColor) -> Self {
        self.default_background = bg;
        self
    }

    pub fn back_color(&self) -> &HColor {
        &self.param.backcolor
    }

    pub fn string_bounder(&self) -> &dyn StringBounder {
        self.bounder.as_ref()
    }

    pub fn apply_translate(&self, t: UTranslate) -> UGraphic {
        let mut u = self.clone();
        u.translate = u.translate.compose(t);
        u
    }
    pub fn apply_color(&self, c: HColor) -> UGraphic {
        let mut u = self.clone();
        u.param.color = c;
        u
    }
    pub fn apply_back_color(&self, c: HColor) -> UGraphic {
        let mut u = self.clone();
        u.param.backcolor = c;
        u
    }
    pub fn apply_stroke(&self, s: UStroke) -> UGraphic {
        let mut u = self.clone();
        u.param.stroke = s;
        u
    }
    /// Clip is given in the current (translated) coordinates.
    pub fn apply_clip(&self, c: UClip) -> UGraphic {
        let mut u = self.clone();
        u.param.clip = Some(c.translate(u.translate));
        u
    }
    pub fn apply_hidden(&self, hidden: bool) -> UGraphic {
        let mut u = self.clone();
        u.param.hidden = hidden;
        u
    }

    /// Makes horizontal lines span `stencil` from the current position on.
    pub fn apply_stencil(&self, stencil: Rc<dyn Stencil>, default_stroke: UStroke) -> UGraphic {
        let parent = self.stencil.clone();
        let mut u = self.clone();
        u.stencil = Some(Rc::new(StencilLayer {
            stencil,
            default_stroke,
            origin: u.translate,
            parent,
        }));
        u
    }

    pub fn start_group(&self, attrs: &[(&str, String)]) {
        if self.limit.is_some() {
            return;
        }
        self.svg.start_group(attrs);
    }
    pub fn close_group(&self) {
        if self.limit.is_some() {
            return;
        }
        self.svg.close_group();
    }

    fn apply_fill(&self, color: &HColor) {
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

    fn apply_stroke_color(&self, color: &HColor) {
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

    fn apply_stroke_width(&self) {
        let s = self.param.stroke;
        self.svg.set_stroke_width(s.thickness, s.dasharray());
    }

    /// Draws a shape at the current translation.
    pub fn draw(&self, shape: impl Into<UShape>) {
        let (x, y) = (self.translate.dx, self.translate.dy);
        if let Some(limit) = self.limit.clone() {
            self.limit_shape(&shape.into(), x, y, &limit);
            return;
        }
        self.svg.set_hidden(self.param.hidden);
        match shape.into() {
            UShape::Empty(..) => {}
            UShape::Rectangle(r) => self.draw_rectangle(&r, x, y),
            UShape::Line(l) => self.draw_line(&l, x, y),
            UShape::Ellipse(e) => self.draw_ellipse(&e, x, y),
            UShape::Polygon(p) => self.draw_polygon(&p, x, y),
            UShape::Path(p) => self.draw_path(&p, x, y),
            UShape::Text(t) => self.draw_text(&t, x, y),
            UShape::HLine(l) => self.draw_hline(&l),
            UShape::Comment(c) => self.svg.add_comment(&c),
        }
        self.svg.set_hidden(false);
    }

    fn limit_shape(&self, shape: &UShape, x: f64, y: f64, limit: &RefCell<MinMax>) {
        let clip = self.param.clip;
        let add = |px: f64, py: f64| {
            if clip.is_none_or(|c| c.is_inside(px, py)) {
                let mut m = limit.borrow_mut();
                *m = m.add_point(px, py);
            }
        };
        match shape {
            UShape::Text(t) => {
                let dim = self.bounder.calculate_dimension(&t.font.font(), &t.text);
                let y = y - (dim.height - 1.5);
                add(x, y);
                add(x, y + dim.height);
                add(x + dim.width, y);
                add(x + dim.width, y + dim.height);
            }
            UShape::Line(l) => {
                add(x, y);
                add(x + l.dx, y + l.dy);
            }
            UShape::Ellipse(e) => {
                add(x, y);
                add(
                    x + e.width - 1.0 + e.delta_shadow * 2.0,
                    y + e.height - 1.0 + e.delta_shadow * 2.0,
                );
            }
            UShape::Polygon(p) => {
                if let Some((x1, y1, x2, y2)) = p.min_max() {
                    add(x + x1 - 10.0, y + y1);
                    add(x + x2 + 10.0, y + y2);
                }
            }
            UShape::Path(p) => {
                if let Some((x1, y1, x2, y2)) = p.min_max() {
                    add(x + x1, y + y1);
                    add(x + x2, y + y2);
                }
            }
            UShape::Rectangle(r) => {
                add(x - 1.0, y - 1.0);
                add(
                    x + r.width - 1.0 + r.delta_shadow * 2.0,
                    y + r.height - 1.0 + r.delta_shadow * 2.0,
                );
            }
            UShape::Empty(w, h) => {
                add(x, y);
                add(x + w, y + h);
            }
            UShape::HLine(l) => self.draw_hline(l),
            UShape::Comment(_) => {}
        }
    }

    fn draw_hline(&self, l: &UHorizontalLine) {
        let Some(layer) = self.stencil.clone() else {
            return;
        };
        let y = self.translate.dy - layer.origin.dy;
        let mut u = self.clone();
        u.translate = layer.origin;
        u.stencil = layer.parent.clone();
        l.draw_line_internal(&u, layer.stencil.as_ref(), y, layer.default_stroke);
    }

    fn draw_rectangle(&self, r: &URectangle, mut x: f64, mut y: f64) {
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

    fn draw_line(&self, l: &ULine, x: f64, y: f64) {
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

    fn draw_ellipse(&self, e: &UEllipse, x: f64, y: f64) {
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

    fn draw_polygon(&self, p: &UPolygon, x: f64, y: f64) {
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

    fn draw_path(&self, p: &UPath, x: f64, y: f64) {
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

    fn draw_text(&self, t: &UText, mut x: f64, y: f64) {
        if let Some(clip) = self.param.clip {
            if !clip.is_inside(x, y) {
                return;
            }
        }
        let fc = &t.font;
        if fc.color().is_transparent() {
            return;
        }
        let font = fc.font();
        let face = fc.font_face();
        // Explicit bold keeps a heavier face weight; otherwise only non-default weights show.
        let weight = if fc.contains_style(FontStyle::Bold) {
            Some(if face.weight >= 700 {
                face.weight.to_string()
            } else {
                "700".to_string()
            })
        } else if face.weight != 400 {
            Some(face.weight.to_string())
        } else {
            None
        };
        let style = (fc.contains_style(FontStyle::Italic) || face.italic).then_some("italic");
        let mut text = t.text.clone();
        if text.trim().is_empty() {
            text = text.replace(' ', "\u{a0}");
        }
        while text.starts_with(' ') {
            x += self.bounder.calculate_dimension(&font, " ").width;
            text.remove(0);
        }
        let text = text.trim_matches(|c: char| c.is_whitespace() && c != '\u{a0}');
        let dim = self.bounder.calculate_dimension(&font, text);
        let mut deco = Vec::new();
        let mut extra_lines: Vec<(HColor, f64)> = Vec::new();
        let ext = fc.extended_color();
        if fc.contains_style(FontStyle::Underline)
            && fc.underline_stroke().is_none_or(|s| s.thickness > 0.0)
        {
            match ext {
                None => deco.push("underline"),
                Some(c) => extra_lines.push((c.clone(), font.size2d() / 14.0)),
            }
        }
        if fc.contains_style(FontStyle::Strike) {
            match ext {
                None => deco.push("line-through"),
                Some(c) => extra_lines.push((c.clone(), -font.size2d() / 4.0)),
            }
        }
        if fc.contains_style(FontStyle::Wave) {
            deco.push("wavy underline");
        }
        let decoration = deco.join(" ");
        let mut back = None;
        if fc.contains_style(FontStyle::BackColor) {
            if let Some(b) = ext {
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
        self.svg.set_fill_color(Some(&fc.color().to_svg()));
        let family = font.svg_family();
        let attrs = TextAttrs {
            family: Some(&family),
            size: font.size,
            weight: weight.as_deref(),
            style,
            decoration: (!decoration.is_empty()).then_some(decoration.as_str()),
            text_length: dim.width,
            extra: fc.attributes().to_vec(),
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

/// Bounds of what `f` draws (`LimitFinder`); `init_to_zero` starts the box at the origin.
pub fn find_limits(
    bounder: Rc<dyn StringBounder>,
    init_to_zero: bool,
    f: impl FnOnce(&UGraphic),
) -> MinMax {
    let scratch = Rc::new(RefCell::new(SvgGraphics::new(0, SvgOption::default())));
    let limit = Rc::new(RefCell::new(MinMax::empty(init_to_zero)));
    {
        let mut ug = UGraphic::new(scratch, bounder);
        ug.limit = Some(limit.clone());
        f(&ug);
    }
    let m = *limit.borrow();
    if m.is_infinity() {
        MinMax::empty(true)
    } else {
        m
    }
}

/// Convenience: renders a drawing closure into an SVG string.
pub fn render_svg(
    option: SvgOption,
    seed: i64,
    bounder: Rc<dyn StringBounder>,
    f: impl FnOnce(&UGraphic),
) -> String {
    let svg = Rc::new(RefCell::new(SvgGraphics::new(seed, option)));
    f(&UGraphic::new(svg.clone(), bounder));
    let svg = Rc::try_unwrap(svg)
        .unwrap_or_else(|_| panic!("drawing context still in use"))
        .into_inner();
    svg.to_svg()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::color::Rgba;
    use crate::klimt::font::{EmbeddedFontBounder, UFont};

    fn svg(f: impl FnOnce(&UGraphic)) -> String {
        render_svg(SvgOption::default(), 0, Rc::new(EmbeddedFontBounder), f)
    }

    #[test]
    fn rectangle_with_translate_and_colors() {
        let out = svg(|ug| {
            let red = HColor::parse("#FF0000").unwrap();
            let u = ug.apply_color(HColor::BLACK);
            let u = u.apply_back_color(red);
            let u = u.apply_translate(UTranslate::new(5.0, 6.0));
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
            let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14))
                .underline()
                .add(FontStyle::Strike)
                .change_extended_color(Some(HColor::parse("#00FF00").unwrap()));
            ug.draw(UText::new("  Hello ", fc));
            let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14))
                .underline()
                .bold()
                .italic();
            ug.draw(UText::new("Hi", fc.clone()));
            let t = UText::new("T", fc.change_color(HColor::TRANSPARENT));
            ug.draw(t);
            let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14))
                .add(FontStyle::BackColor)
                .change_extended_color(Some(HColor::parse("#00FF00").unwrap()));
            ug.draw(UText::new("B", fc));
            let g = HColor::Gradient {
                c1: Rgba::rgb(255, 0, 0),
                c2: Rgba::rgb(0, 0, 255),
                policy: '-',
            };
            let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14))
                .add(FontStyle::BackColor)
                .change_extended_color(Some(g));
            ug.draw(UText::new("G", fc));
            ug.draw(UText::new(
                "   ",
                FontConfiguration::black_blue_true(UFont::sans_serif(14)),
            ));
        });
        assert!(out.contains(">Hello</text>"), "{out}");
        assert!(out.contains("<line"), "{out}");
        assert!(out.contains("underline"), "{out}");
        assert!(
            out.contains("font-weight=\"700\"") && out.contains("italic"),
            "{out}"
        );
        assert!(out.contains("linearGradient"), "{out}");
    }

    #[test]
    fn clipping_and_hidden() {
        let out = svg(|ug| {
            let u = ug.apply_clip(UClip::new(0.0, 0.0, 10.0, 10.0));
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
            let h = ug.apply_hidden(true);
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
            let u = ug.apply_back_color(g.clone());
            let u = u.apply_color(g.clone());
            u.draw(URectangle::build(5.0, 5.0));
            u.draw(UEllipse::build(5.0, 5.0));
            u.draw(UPath::new());
            ug.close_group();
        });
        assert!(out.contains("linearGradient"), "{out}");
        assert!(out.contains(r#"class="x""#), "{out}");
    }

    #[test]
    fn limits() {
        let m = find_limits(Rc::new(EmbeddedFontBounder), true, |ug| {
            ug.apply_translate(UTranslate::new(5.0, 6.0))
                .draw(URectangle::build(10.0, 20.0));
            ug.draw(UShape::Empty(30.0, 40.0));
            let mut p = UPath::new();
            p.move_to(1.0, 1.0);
            p.line_to(50.0, 2.0);
            ug.apply_translate(UTranslate::dy(100.0)).draw(p);
            ug.start_group(&[]);
            ug.close_group();
            let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14));
            ug.apply_translate(UTranslate::new(0.0, 200.0))
                .draw(UText::new("abc", fc));
            let c = ug.apply_clip(UClip::new(0.0, 0.0, 10.0, 10.0));
            c.draw(ULine::new(500.0, 500.0));
        });
        assert_eq!(m.min_x, 0.0);
        assert!(m.max_y >= 200.0 && m.max_y < 220.0, "{m:?}");
        assert!(m.max_x >= 50.0);
        let none = find_limits(Rc::new(EmbeddedFontBounder), false, |_| {});
        assert_eq!(none, MinMax::empty(true));
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
