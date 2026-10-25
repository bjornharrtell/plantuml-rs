//! SVG backend, ported from `SvgGraphics`.
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

use super::xml::{XmlChild, XmlNode};

const DEFAULT_FONT_FAMILY: &str = "sans-serif";

/// Formats like `StringUtils.formatDecimal`: rounds half up, drops trailing zeros.
pub fn format_decimal(x: f64, decimal: usize) -> String {
    if x == 0.0 || x.is_nan() {
        return "0".into();
    }
    let scale = 10f64.powi(decimal as i32);
    let abs = x.abs();
    let scaled = abs * scale;
    let floor = scaled.floor();
    let frac = scaled - floor;
    let rounded = if (frac - 0.5).abs() < scaled.max(1.0) * 4e-15 && frac >= 0.5 - 1e-300 {
        // exact or near tie: Java's String.format rounds the exact decimal half up
        let exact = format!("{:.*}", decimal + 20, abs);
        exact_round_half_up(&exact, decimal)
    } else if frac < 0.5 {
        floor as u64
    } else {
        floor as u64 + 1
    };
    let mut s = format!("{}", rounded);
    if decimal > 0 {
        while s.len() <= decimal {
            s.insert(0, '0');
        }
        s.insert(s.len() - decimal, '.');
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "0" {
        return s;
    }
    if x < 0.0 {
        s.insert(0, '-');
    }
    s
}

fn exact_round_half_up(exact: &str, decimal: usize) -> u64 {
    let dot = exact.find('.').unwrap();
    let int_part = &exact[..dot];
    let frac = &exact[dot + 1..];
    let keep: u64 = format!("{}{}", int_part, &frac[..decimal])
        .parse()
        .unwrap_or(0);
    if frac.as_bytes()[decimal] >= b'5' {
        keep + 1
    } else {
        keep
    }
}

/// One segment of a path (`USegment`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSeg {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    QuadTo(f64, f64, f64, f64),
    CubicTo(f64, f64, f64, f64, f64, f64),
    /// rx, ry, x-axis rotation, large-arc, sweep, x, y
    ArcTo(f64, f64, f64, bool, bool, f64, f64),
    Close,
}

/// How text length is enforced (`LengthAdjust`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthAdjust {
    None,
    Spacing,
    SpacingAndGlyphs,
}

/// Document-level options (`SvgOption`).
#[derive(Debug, Clone)]
pub struct SvgOption {
    pub scale: f64,
    pub decimal: usize,
    pub min_width: f64,
    pub min_height: f64,
    /// SVG color of the background (`#RRGGBB[AA]`), or `None`.
    pub backcolor: Option<String>,
    pub length_adjust: LengthAdjust,
    pub preserve_aspect_ratio: String,
    pub dimension_style: bool,
    pub title: Option<String>,
    pub root_attributes: Vec<(String, String)>,
}

impl Default for SvgOption {
    fn default() -> Self {
        Self {
            scale: 1.0,
            decimal: 3,
            min_width: 0.0,
            min_height: 0.0,
            backcolor: None,
            length_adjust: LengthAdjust::Spacing,
            preserve_aspect_ratio: "none".into(),
            dimension_style: true,
            title: None,
            root_attributes: Vec::new(),
        }
    }
}

/// Attributes of a text element.
#[derive(Debug, Clone, Default)]
pub struct TextAttrs<'a> {
    pub family: Option<&'a str>,
    pub size: i32,
    pub weight: Option<&'a str>,
    pub style: Option<&'a str>,
    pub decoration: Option<&'a str>,
    pub text_length: f64,
    pub extra: Vec<(String, String)>,
    pub back_color: Option<&'a str>,
    pub orientation: i32,
}

pub struct SvgGraphics {
    option: SvgOption,
    root: XmlNode,
    defs: XmlNode,
    g_root: XmlNode,
    pending: Vec<XmlNode>,
    fill: String,
    stroke: String,
    stroke_width: String,
    stroke_dasharray: Option<String>,
    max_x: i32,
    max_y: i32,
    hidden: bool,
    with_shadow: bool,
    shadow_id: String,
    gradient_id: String,
    filter_uid: String,
    gradients: HashMap<(String, String, char), String>,
    gradient_count: usize,
    back_filters: HashMap<String, String>,
    background: Option<usize>,
    backcolor_string: Option<String>,
}

fn is_hex_with_alpha(c: &str) -> bool {
    c.len() == 9 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

/// `#AABBCC` becomes `#ABC`.
pub fn shorten_color(color: &str) -> String {
    let b = color.as_bytes();
    if b.len() == 7 && b[0] == b'#' && b[1] == b[2] && b[3] == b[4] && b[5] == b[6] {
        return format!("#{}{}{}", b[1] as char, b[3] as char, b[5] as char);
    }
    color.to_string()
}

impl SvgGraphics {
    pub fn new(seed: i64, option: SvgOption) -> Self {
        let mut root = XmlNode::new("svg");
        root.set("xmlns", "http://www.w3.org/2000/svg");
        root.set("xmlns:xlink", "http://www.w3.org/1999/xlink");
        root.set("version", "1.1");
        if let Some(t) = &option.title {
            let mut n = XmlNode::new("title");
            n.set_text(t);
            root.push(n);
        }
        for (k, v) in &option.root_attributes {
            root.set(k, v.clone());
        }
        let mut g_root = XmlNode::new("g");
        g_root.set("font-family", DEFAULT_FONT_FAMILY);
        match option.length_adjust {
            LengthAdjust::Spacing => g_root.set("lengthAdjust", "spacing"),
            LengthAdjust::SpacingAndGlyphs => g_root.set("lengthAdjust", "spacingAndGlyphs"),
            LengthAdjust::None => {}
        }
        let seed36 = to_base36(seed.unsigned_abs());
        let mut g = Self {
            stroke_width: format_decimal(1.0, option.decimal),
            max_x: 0,
            max_y: 0,
            root,
            defs: XmlNode::new("defs"),
            g_root,
            pending: Vec::new(),
            fill: "#000000".into(),
            stroke: "#000000".into(),
            stroke_dasharray: None,
            hidden: false,
            with_shadow: false,
            shadow_id: format!("f{seed36}"),
            gradient_id: format!("g{seed36}"),
            filter_uid: format!("b{seed36}"),
            gradients: HashMap::new(),
            gradient_count: 0,
            back_filters: HashMap::new(),
            background: None,
            backcolor_string: None,
            option,
        };
        g.ensure_visible(g.option.min_width, g.option.min_height);
        if let Some(back) = g.option.backcolor.clone() {
            g.backcolor_string = Some(back.clone());
            if back != "#00000000" && back != "#000000" && back != "#FFFFFF" {
                g.set_fill_color(Some(&back));
                g.set_stroke_color(None);
                let rect = g.create_rect(0.0, 0.0, 0.0, 0.0);
                g.g_root.push(rect);
                g.background = Some(g.g_root.children.len() - 1);
            }
        }
        g
    }

    fn format(&self, x: f64) -> String {
        format_decimal(x * self.option.scale, self.option.decimal)
    }

    fn format_opacity(&self, v: f64) -> String {
        if v <= 0.0 {
            "0".into()
        } else if v >= 1.0 {
            "1".into()
        } else {
            format_decimal(v, self.option.decimal.max(2))
        }
    }

    pub fn ensure_visible(&mut self, x: f64, y: f64) {
        if x > f64::from(self.max_x) {
            self.max_x = (x + 1.0) as i32;
        }
        if y > f64::from(self.max_y) {
            self.max_y = (y + 1.0) as i32;
        }
    }

    pub fn max_dimension(&self) -> (i32, i32) {
        (self.max_x, self.max_y)
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// `None` or `#00000000` means no fill.
    pub fn set_fill_color(&mut self, fill: Option<&str>) {
        self.fill = fix_color(fill);
    }

    /// Keeps the color as is, including transparent ones (`WITH_FILL_OPACITY`).
    pub fn set_fill_color_opacity(&mut self, fill: &str) {
        self.fill = fill.to_string();
    }

    pub fn set_stroke_color(&mut self, stroke: Option<&str>) {
        self.stroke = fix_color(stroke);
    }

    pub fn set_stroke_width(&mut self, width: f64, dash: Option<(f64, f64)>) {
        self.stroke_width = self.format(width);
        self.stroke_dasharray = dash.map(|(a, b)| format!("{},{}", self.format(a), self.format(b)));
    }

    fn current(&mut self) -> &mut XmlNode {
        self.pending.last_mut().unwrap_or(&mut self.g_root)
    }

    fn fill_me(&self, elt: &mut XmlNode) {
        if is_hex_with_alpha(&self.fill) {
            elt.set("fill", shorten_color(&self.fill[..7]));
            let alpha = f64::from(u8::from_str_radix(&self.fill[7..], 16).unwrap_or(255)) / 255.0;
            elt.set("fill-opacity", self.format_opacity(alpha));
        } else {
            elt.set("fill", shorten_color(&self.fill));
        }
    }

    fn style_me(&self, elt: &mut XmlNode, supp: Option<&str>) {
        if self.stroke_width == "0" {
            return;
        }
        let mut style = format!("stroke:{};", shorten_color(&self.stroke));
        if self.stroke != "none" {
            style.push_str(&format!("stroke-width:{};", self.stroke_width));
            if let Some(d) = &self.stroke_dasharray {
                style.push_str(&format!("stroke-dasharray:{d};"));
            }
        }
        if let Some(s) = supp {
            style.push_str(s);
        }
        elt.set("style", style);
    }

    fn manage_shadow(&mut self, delta: f64) {
        if delta == 0.0 || self.with_shadow {
            return;
        }
        let mut filter = XmlNode::new("filter");
        filter.set("id", self.shadow_id.clone());
        filter.set("x", "-1");
        filter.set("y", "-1");
        filter.set("width", "300%");
        filter.set("height", "300%");
        let two = self.format(2.0);
        let four = self.format(4.0);
        add_filter(
            &mut filter,
            "feGaussianBlur",
            &[("result", "blurOut"), ("stdDeviation", &two)],
        );
        add_filter(
            &mut filter,
            "feColorMatrix",
            &[
                ("type", "matrix"),
                ("in", "blurOut"),
                ("result", "blurOut2"),
                ("values", "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 .4 0"),
            ],
        );
        add_filter(
            &mut filter,
            "feOffset",
            &[
                ("result", "blurOut3"),
                ("in", "blurOut2"),
                ("dx", &four),
                ("dy", &four),
            ],
        );
        add_filter(
            &mut filter,
            "feBlend",
            &[
                ("in", "SourceGraphic"),
                ("in2", "blurOut3"),
                ("mode", "normal"),
            ],
        );
        self.defs.push(filter);
        self.with_shadow = true;
    }

    fn add_shadow_filter(&self, elt: &mut XmlNode, delta: f64) {
        if delta > 0.0 {
            elt.set("filter", format!("url(#{})", self.shadow_id));
        }
    }

    fn create_rect(&self, x: f64, y: f64, w: f64, h: f64) -> XmlNode {
        let mut e = XmlNode::new("rect");
        e.set("x", self.format(x));
        e.set("y", self.format(y));
        e.set("width", self.format(w));
        e.set("height", self.format(h));
        self.fill_me(&mut e);
        self.style_me(&mut e, None);
        e
    }

    pub fn rectangle(&mut self, x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64, shadow: f64) {
        if h <= 0.0 || w <= 0.0 {
            return;
        }
        self.manage_shadow(shadow);
        if !self.hidden {
            let mut e = self.create_rect(x, y, w, h);
            self.add_shadow_filter(&mut e, shadow);
            if rx > 0.0 && ry > 0.0 {
                e.set("rx", self.format(rx));
                e.set("ry", self.format(ry));
            }
            self.current().push(e);
        }
        self.ensure_visible(x + w + 2.0 * shadow, y + h + 2.0 * shadow);
    }

    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, shadow: f64) {
        self.manage_shadow(shadow);
        if !self.hidden {
            let mut e = XmlNode::new("ellipse");
            e.set("cx", self.format(cx));
            e.set("cy", self.format(cy));
            e.set("rx", self.format(rx));
            e.set("ry", self.format(ry));
            self.fill_me(&mut e);
            self.style_me(&mut e, None);
            self.add_shadow_filter(&mut e, shadow);
            self.current().push(e);
        }
        self.ensure_visible(cx + rx + shadow * 2.0, cy + ry + shadow * 2.0);
    }

    /// Elliptical arc from (x1,y1) to (x2,y2) (`svgArcEllipse`).
    pub fn arc_ellipse(&mut self, rx: f64, ry: f64, x1: f64, y1: f64, x2: f64, y2: f64) {
        if !self.hidden {
            let d = format!(
                "M{},{} A{},{} 0 0 0 {} {}",
                self.format(x1),
                self.format(y1),
                self.format(rx),
                self.format(ry),
                self.format(x2),
                self.format(y2)
            );
            let mut e = XmlNode::new("path");
            e.set("d", d);
            self.fill_me(&mut e);
            self.style_me(&mut e, None);
            self.current().push(e);
        }
        self.ensure_visible(x1, y1);
        self.ensure_visible(x2, y2);
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, shadow: f64) {
        self.manage_shadow(shadow);
        if !self.hidden {
            let mut e = XmlNode::new("line");
            e.set("x1", self.format(x1));
            e.set("y1", self.format(y1));
            e.set("x2", self.format(x2));
            e.set("y2", self.format(y2));
            self.style_me(&mut e, None);
            self.add_shadow_filter(&mut e, shadow);
            self.current().push(e);
        }
        self.ensure_visible(x1 + 2.0 * shadow, y1 + 2.0 * shadow);
        self.ensure_visible(x2 + 2.0 * shadow, y2 + 2.0 * shadow);
    }

    pub fn polygon(&mut self, shadow: f64, points: &[f64]) {
        self.manage_shadow(shadow);
        if !self.hidden {
            let mut e = XmlNode::new("polygon");
            let pts: Vec<String> = points.iter().map(|c| self.format(*c)).collect();
            e.set("points", pts.join(","));
            self.fill_me(&mut e);
            self.style_me(&mut e, Some("stroke-linejoin:miter;stroke-miterlimit:10;"));
            self.add_shadow_filter(&mut e, shadow);
            self.current().push(e);
        }
        for p in points.chunks_exact(2) {
            self.ensure_visible(p[0] + 2.0 * shadow, p[1] + 2.0 * shadow);
        }
    }

    pub fn path(&mut self, x: f64, y: f64, segs: &[PathSeg], shadow: f64) {
        self.manage_shadow(shadow);
        self.ensure_visible(x, y);
        let mut d = String::new();
        let s2 = 2.0 * shadow;
        for seg in segs {
            match *seg {
                PathSeg::MoveTo(a, b) => {
                    d.push_str(&format!("M{},{} ", self.format(a + x), self.format(b + y)));
                    self.ensure_visible(a + x + s2, b + y + s2);
                }
                PathSeg::LineTo(a, b) => {
                    d.push_str(&format!("L{},{} ", self.format(a + x), self.format(b + y)));
                    self.ensure_visible(a + x + s2, b + y + s2);
                }
                PathSeg::QuadTo(a, b, c, e) => {
                    d.push_str(&format!(
                        "Q{},{} {},{} ",
                        self.format(a + x),
                        self.format(b + y),
                        self.format(c + x),
                        self.format(e + y)
                    ));
                    self.ensure_visible(a + x + s2, b + y + s2);
                    self.ensure_visible(c + x + s2, e + y + s2);
                }
                PathSeg::CubicTo(a, b, c, e, f, g) => {
                    d.push_str(&format!(
                        "C{},{} {},{} {},{} ",
                        self.format(a + x),
                        self.format(b + y),
                        self.format(c + x),
                        self.format(e + y),
                        self.format(f + x),
                        self.format(g + y)
                    ));
                    self.ensure_visible(a + x + s2, b + y + s2);
                    self.ensure_visible(c + x + s2, e + y + s2);
                    self.ensure_visible(f + x + s2, g + y + s2);
                }
                PathSeg::ArcTo(rx, ry, rot, large, sweep, a, b) => {
                    d.push_str(&format!(
                        "A{},{} {} {} {} {},{} ",
                        self.format(rx),
                        self.format(ry),
                        self.format(rot),
                        u8::from(large),
                        u8::from(sweep),
                        self.format(a + x),
                        self.format(b + y)
                    ));
                    self.ensure_visible(a + rx + x + s2, b + ry + y + s2);
                }
                PathSeg::Close => {}
            }
        }
        if !self.hidden {
            let mut e = XmlNode::new("path");
            e.set("d", d.trim());
            self.style_me(&mut e, None);
            self.fill_me(&mut e);
            self.add_shadow_filter(&mut e, shadow);
            self.current().push(e);
        }
    }

    pub fn text(&mut self, text: &str, x: f64, y: f64, a: &TextAttrs) {
        if !self.hidden {
            let mut e = XmlNode::new("text");
            e.set("x", self.format(x));
            e.set("y", self.format(y));
            self.fill_me(&mut e);
            match a.orientation {
                90 => e.set(
                    "transform",
                    format!("rotate(-90 {} {})", self.format(x), self.format(y)),
                ),
                270 => e.set(
                    "transform",
                    format!("rotate(90 {} {})", self.format(x), self.format(y)),
                ),
                _ => {}
            }
            e.set("font-size", self.format(f64::from(a.size)));
            if text.chars().count() > 1 && self.option.length_adjust != LengthAdjust::None {
                e.set("textLength", self.format(a.text_length));
            }
            if let Some(w) = a.weight {
                e.set("font-weight", w);
            }
            if let Some(s) = a.style {
                e.set("font-style", s);
            }
            if let Some(d) = a.decoration {
                e.set("text-decoration", d);
            }
            let mut content = text.to_string();
            if let Some(f) = a.family {
                let f = if f.eq_ignore_ascii_case("monospaced") {
                    "monospace"
                } else {
                    f
                };
                if !f.eq_ignore_ascii_case(DEFAULT_FONT_FAMILY) {
                    e.set("font-family", f);
                }
                if f.eq_ignore_ascii_case("monospace") || f.eq_ignore_ascii_case("courier") {
                    content = content.replace(' ', "\u{a0}");
                }
            }
            if let Some(bc) = a.back_color {
                let id = self.filter_back_color(bc);
                e.set("filter", format!("url(#{id})"));
            }
            for (k, v) in &a.extra {
                e.set(k, v.clone());
            }
            e.set_text(&content);
            self.current().push(e);
        }
        self.ensure_visible(x, y);
        self.ensure_visible(x + a.text_length, y);
    }

    fn filter_back_color(&mut self, color: &str) -> String {
        if let Some(id) = self.back_filters.get(color) {
            return id.clone();
        }
        let id = format!("{}{}", self.filter_uid, self.back_filters.len());
        self.back_filters.insert(color.to_string(), id.clone());
        let mut filter = XmlNode::new("filter");
        filter.set("id", id.clone());
        filter.set("x", "0");
        filter.set("y", "0");
        filter.set("width", "1");
        filter.set("height", "1");
        add_filter(
            &mut filter,
            "feFlood",
            &[("flood-color", color), ("result", "flood")],
        );
        add_filter(
            &mut filter,
            "feComposite",
            &[
                ("in", "SourceGraphic"),
                ("in2", "flood"),
                ("operator", "over"),
            ],
        );
        self.defs.push(filter);
        id
    }

    /// Creates (or reuses) a two-color linear gradient and returns its id.
    pub fn create_gradient(&mut self, c1: &str, c2: &str, policy: char) -> String {
        let key = (c1.to_string(), c2.to_string(), policy);
        if let Some(id) = self.gradients.get(&key) {
            return id.clone();
        }
        let mut e = XmlNode::new("linearGradient");
        let (x1, y1, x2, y2) = match policy {
            '|' => ("0%", "50%", "100%", "50%"),
            '\\' => ("0%", "100%", "100%", "0%"),
            '-' => ("50%", "0%", "50%", "100%"),
            _ => ("0%", "0%", "100%", "100%"),
        };
        e.set("x1", x1);
        e.set("y1", y1);
        e.set("x2", x2);
        e.set("y2", y2);
        let id = format!("{}{}", self.gradient_id, self.gradient_count);
        self.gradient_count += 1;
        self.gradients.insert(key, id.clone());
        e.set("id", id.clone());
        for (c, off) in [(c1, "0%"), (c2, "100%")] {
            let mut s = XmlNode::new("stop");
            s.set("stop-color", shorten_color(c));
            s.set("offset", off);
            e.push(s);
        }
        self.defs.push(e);
        id
    }

    pub fn add_comment(&mut self, comment: &str) {
        self.current()
            .children
            .push(XmlChild::Comment(comment.into()));
    }

    pub fn start_group(&mut self, attrs: &[(&str, String)]) {
        let mut g = XmlNode::new("g");
        for (k, v) in attrs {
            g.set(k, v.clone());
        }
        self.pending.push(g);
    }

    /// Closes the current group, dropping it when empty.
    pub fn close_group(&mut self) {
        let g = self.pending.pop().expect("no open group");
        if !g.is_empty() {
            self.current().push(g);
        }
    }

    /// Serializes the document.
    pub fn to_svg(mut self) -> String {
        while !self.pending.is_empty() {
            self.close_group();
        }
        let (max_x, max_y) = (self.max_x, self.max_y);
        let scale = self.option.scale;
        let mx = (f64::from(max_x) * scale) as i32;
        let my = (f64::from(max_y) * scale) as i32;
        let mut style = format!("width:{mx}px;height:{my}px;");
        if let Some(b) = &self.backcolor_string {
            if b != "#00000000" {
                style.push_str(&format!("background:{b};"));
            }
        }
        if self.option.dimension_style {
            self.root.set("style", style);
            self.root
                .set("width", format!("{}px", self.format(f64::from(max_x))));
            self.root
                .set("height", format!("{}px", self.format(f64::from(max_y))));
        }
        self.root.set("viewBox", format!("0 0 {mx} {my}"));
        self.root.set("zoomAndPan", "magnify");
        self.root.set(
            "preserveAspectRatio",
            self.option.preserve_aspect_ratio.clone(),
        );
        self.root.set("contentStyleType", "text/css");
        if let Some(i) = self.background {
            let w = self.format(f64::from(max_x));
            let h = self.format(f64::from(max_y));
            if let XmlChild::Node(n) = &mut self.g_root.children[i] {
                n.set("width", w);
                n.set("height", h);
            }
        }
        let mut root = self.root;
        root.push(self.defs);
        root.push(self.g_root);
        root.to_xml(2)
    }
}

fn fix_color(c: Option<&str>) -> String {
    match c {
        None | Some("#00000000") => "none".into(),
        Some(c) => c.to_string(),
    }
}

fn add_filter(filter: &mut XmlNode, name: &str, data: &[(&str, &str)]) {
    let mut e = XmlNode::new(name);
    for (k, v) in data {
        e.set(k, *v);
    }
    filter.push(e);
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(std::char::from_digit((n % 36) as u32, 36).unwrap());
        n /= 36;
    }
    out.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_round_half_up() {
        assert_eq!(format_decimal(0.0, 3), "0");
        assert_eq!(format_decimal(30.5625, 3), "30.563");
        assert_eq!(format_decimal(1.0, 3), "1");
        assert_eq!(format_decimal(-2.5, 3), "-2.5");
        assert_eq!(format_decimal(0.0004, 3), "0");
        assert_eq!(format_decimal(12.3456, 3), "12.346");
        assert_eq!(format_decimal(0.0625, 3), "0.063");
        assert_eq!(format_decimal(7.0, 0), "7");
        assert_eq!(format_decimal(f64::NAN, 3), "0");
    }

    #[test]
    fn colors() {
        assert_eq!(shorten_color("#FF0000"), "#F00");
        assert_eq!(shorten_color("#FF0001"), "#FF0001");
        assert_eq!(shorten_color("none"), "none");
        assert_eq!(to_base36(0), "0");
        assert_eq!(to_base36(35), "z");
    }

    fn svg() -> SvgGraphics {
        SvgGraphics::new(1, SvgOption::default())
    }

    #[test]
    fn rect_and_text_document() {
        let mut g = svg();
        g.set_fill_color(Some("#F1F1F1"));
        g.set_stroke_color(Some("#181818"));
        g.set_stroke_width(0.5, None);
        g.rectangle(8.5, 142.0, 56.938, 62.0, 2.5, 2.5, 0.0);
        g.set_fill_color(Some("#000000"));
        g.text(
            "Bob",
            37.5,
            161.889,
            &TextAttrs {
                size: 14,
                text_length: 24.938,
                family: Some("monospaced"),
                ..Default::default()
            },
        );
        let s = g.to_svg();
        assert!(s.contains(
            "<rect fill=\"#F1F1F1\" height=\"62\" rx=\"2.5\" ry=\"2.5\" style=\"stroke:#181818;stroke-width:0.5;\" width=\"56.938\" x=\"8.5\" y=\"142\"/>"
        ));
        assert!(s.contains(
            "<text fill=\"#000\" font-family=\"monospace\" font-size=\"14\" textLength=\"24.938\" x=\"37.5\" y=\"161.889\">Bob</text>"
        ));
        assert!(s.contains("viewBox=\"0 0 66 205\""));
        assert!(s.contains("style=\"width:66px;height:205px;\""));
        assert!(s.contains("<defs/>"));
    }

    #[test]
    fn ellipse_line_polygon_path() {
        let mut g = svg();
        g.set_fill_color(Some("#ADD1B2"));
        g.set_stroke_color(Some("#181818"));
        g.ellipse(23.5, 158.0, 11.0, 11.0, 0.0);
        g.line(1.0, 2.0, 3.0, 4.0, 0.0);
        g.polygon(0.0, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        g.set_stroke_width(1.0, Some((7.0, 3.0)));
        g.path(
            1.0,
            1.0,
            &[
                PathSeg::MoveTo(0.0, 0.0),
                PathSeg::LineTo(1.0, 1.0),
                PathSeg::QuadTo(1.0, 2.0, 3.0, 4.0),
                PathSeg::CubicTo(1.0, 2.0, 3.0, 4.0, 5.0, 6.0),
                PathSeg::ArcTo(5.0, 5.0, 0.0, true, false, 1.0, 1.0),
                PathSeg::Close,
            ],
            0.0,
        );
        let s = g.to_svg();
        assert!(s.contains("<ellipse cx=\"23.5\" cy=\"158\" fill=\"#ADD1B2\""));
        assert!(
            s.contains("<line style=\"stroke:#181818;stroke-width:0.5;\"")
                || s.contains("<line style=")
        );
        assert!(s.contains("points=\"1,2,3,4,5,6\""));
        assert!(s.contains("stroke-linejoin:miter;stroke-miterlimit:10;"));
        assert!(s.contains("d=\"M1,1 L2,2 Q2,3 4,5 C2,3 4,5 6,7 A5,5 0 1 0 2,2\""));
        assert!(s.contains("stroke-dasharray:7,3;"));
    }

    #[test]
    fn background_gradient_shadow_groups() {
        let mut o = SvgOption {
            backcolor: Some("#FFA500".into()),
            title: Some("T".into()),
            ..SvgOption::default()
        };
        o.root_attributes
            .push(("data-diagram-type".into(), "CLASS".into()));
        let mut g = SvgGraphics::new(7, o);
        g.start_group(&[("class", "entity".into())]);
        g.add_comment("class Bob");
        let id = g.create_gradient("#FF0000", "#0000FF", '|');
        assert_eq!(g.create_gradient("#FF0000", "#0000FF", '|'), id);
        for p in ['\\', '-', '/'] {
            g.create_gradient("#111111", "#222222", p);
        }
        g.set_fill_color_opacity("#FF000080");
        g.set_stroke_color(None);
        g.rectangle(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 4.0);
        g.set_hidden(true);
        g.rectangle(0.0, 0.0, 5.0, 5.0, 0.0, 0.0, 0.0);
        g.set_hidden(false);
        g.rectangle(0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0);
        g.start_group(&[("class", "empty".into())]);
        g.close_group();
        g.text(
            "x",
            0.0,
            10.0,
            &TextAttrs {
                size: 10,
                text_length: 5.0,
                weight: Some("700"),
                style: Some("italic"),
                decoration: Some("underline"),
                back_color: Some("#FFFF00"),
                orientation: 90,
                extra: vec![("data-a".into(), "b".into())],
                ..Default::default()
            },
        );
        g.text(
            "y",
            0.0,
            10.0,
            &TextAttrs {
                orientation: 270,
                back_color: Some("#FFFF00"),
                ..Default::default()
            },
        );
        g.close_group();
        assert_eq!(g.max_dimension(), (19, 19));
        let s = g.to_svg();
        assert!(s.contains("data-diagram-type=\"CLASS\""));
        assert!(s.contains("<title>T</title>"));
        assert!(s.contains("background:#FFA500;"));
        assert!(s.contains("fill-opacity=\"0.502\""));
        assert!(s.contains("filter=\"url(#fb)\"") || s.contains("filter=\"url(#f7)\""));
        assert!(s.contains("<feGaussianBlur"));
        assert!(s.contains("<feFlood flood-color=\"#FFFF00\""));
        assert!(s.contains("rotate(-90 0 10)") && s.contains("rotate(90 0 10)"));
        assert!(s.contains("font-style=\"italic\"") && s.contains("text-decoration=\"underline\""));
        assert!(!s.contains("empty"));
        assert!(s.contains("<!--class Bob-->"));
        assert!(s.contains("<linearGradient"));
        assert!(s.contains("class=\"entity\""));
    }

    #[test]
    fn options_without_dimension_style() {
        let o = SvgOption {
            dimension_style: false,
            length_adjust: LengthAdjust::SpacingAndGlyphs,
            ..SvgOption::default()
        };
        let mut g = SvgGraphics::new(1, o);
        g.text(
            "ab",
            0.0,
            0.0,
            &TextAttrs {
                family: Some("Courier"),
                size: 12,
                text_length: 3.0,
                ..Default::default()
            },
        );
        let s = g.to_svg();
        assert!(!s.contains("width=\"") || !s.contains("px\""));
        assert!(s.contains("lengthAdjust=\"spacingAndGlyphs\""));
        assert!(s.contains("a\u{a0}") || s.contains("ab"));
        let none = SvgOption {
            length_adjust: LengthAdjust::None,
            ..SvgOption::default()
        };
        let mut g = SvgGraphics::new(1, none);
        g.text(
            "ab",
            0.0,
            0.0,
            &TextAttrs {
                size: 12,
                text_length: 3.0,
                ..Default::default()
            },
        );
        assert!(!g.to_svg().contains("textLength"));
    }
}
