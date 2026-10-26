//! Placing the boxes and the links between them (`SmetanaForJson`, `JsonCurve`, `Arrow`).
//!
//! The layout is `dot` on nodes made of fixed-size cells, one cell per row, laid out sideways:
//! a node is as wide as its box is high, so that ranks run from left to right once the
//! coordinates are swapped back (`Mirror`).

use super::text_block::TextBlockJson;
use crate::json::JsonValue;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, UPath, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName, SName, Style};
use crate::yaml::Highlighted;
use std::fmt::Write;
use std::rc::Rc;

type Point = (f64, f64);

/// A link, with its points already in drawing coordinates.
struct Curve {
    points: Vec<Point>,
    ep: Option<Point>,
}

/// Where each box and link goes.
pub struct JsonLayout {
    nodes: Vec<(Rc<TextBlockJson>, UTranslate)>,
    curves: Vec<Curve>,
    skin: Rc<SkinParam>,
    name: SName,
}

struct Collected {
    blocks: Vec<Rc<TextBlockJson>>,
    dims: Vec<Dimension>,
    heights: Vec<Vec<f64>>,
    widths: Vec<(f64, f64)>,
    /// `(tail, head, tail port)` in creation order.
    edges: Vec<(usize, usize, usize)>,
}

fn up_one_level(key: &str, list: &[Highlighted]) -> Vec<Highlighted> {
    list.iter().filter_map(|h| h.up_one_level(key)).collect()
}

fn collect(
    c: &mut Collected,
    skin: &Rc<SkinParam>,
    name: SName,
    sb: &dyn StringBounder,
    current: &JsonValue,
    highlighted: &[Highlighted],
) -> usize {
    let block = Rc::new(TextBlockJson::new(skin, name, current, highlighted));
    let idx = c.blocks.len();
    c.dims.push(block.calculate_dimension(sb));
    c.heights.push(block.all_heights(sb));
    c.widths
        .push((block.width_col_a(sb), block.width_col_b(sb)));
    c.blocks.push(block.clone());
    let keys = block.keys();
    for (i, child) in block.children().into_iter().enumerate() {
        if let Some(child) = child {
            let sub = up_one_level(&keys[i], highlighted);
            let child_idx = collect(c, skin, name, sb, child, &sub);
            c.edges.push((idx, child_idx, i));
        }
    }
    idx
}

fn cell(n: f64) -> i64 {
    n.round() as i64
}

/// The DOT text; cell sizes are in points, swapped as explained in the module comment.
fn build_dot(c: &Collected) -> String {
    let mut dot = String::from("digraph g {\n");
    for (i, block) in c.blocks.iter().enumerate() {
        let dim = c.dims[i];
        let heights = &c.heights[i];
        if heights.is_empty() {
            let _ = writeln!(
                dot,
                "N{i} [shape=box fixedsize=true label=\"\" width=\"{}\" height=\"{}\"];",
                dim.height / 72.0,
                dim.width / 72.0
            );
            continue;
        }
        let (wa, wb) = c.widths[i];
        let table = "<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"0\">";
        let mut label = String::from(table);
        if block.is_array() {
            label.push_str("<TR>");
            for (k, h) in heights.iter().enumerate() {
                let _ = write!(
                    label,
                    "<TD PORT=\"P{k}\" FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\"></TD>",
                    cell(*h),
                    cell(wa)
                );
            }
            label.push_str("</TR>");
        } else {
            let total: i64 = heights.iter().map(|h| cell(*h)).sum();
            let _ = write!(
                label,
                "<TR><TD COLSPAN=\"{}\" FIXEDSIZE=\"TRUE\" WIDTH=\"{total}\" HEIGHT=\"{}\"></TD></TR><TR>",
                heights.len(),
                cell(wa)
            );
            for (k, h) in heights.iter().enumerate() {
                let _ = write!(
                    label,
                    "<TD PORT=\"P{k}\" FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\"></TD>",
                    cell(*h),
                    cell(wb)
                );
            }
            label.push_str("</TR>");
        }
        label.push_str("</TABLE>");
        let _ = writeln!(
            dot,
            "N{i} [shape=none margin=0 width=0 height=0 label=<{label}>];"
        );
    }
    for (tail, head, port) in &c.edges {
        let _ = writeln!(
            dot,
            "N{tail} -> N{head} [arrowsize=.75 arrowtail=none arrowhead=normal tailport=\"P{port}\"];"
        );
    }
    dot.push_str("}\n");
    dot
}

fn parse_point(s: &str) -> Option<Point> {
    let (x, y) = s.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// `pos` of an edge: optional `e,x,y` arrow tip, then the Bezier points.
fn parse_spline(s: &str) -> Option<(Vec<Point>, Option<Point>)> {
    let mut ep = None;
    let mut points = Vec::new();
    for tok in s.split_whitespace() {
        if let Some(t) = tok.strip_prefix("e,") {
            ep = Some(parse_point(t)?);
        } else if tok.starts_with("s,") {
            continue;
        } else {
            points.push(parse_point(tok)?);
        }
    }
    (points.len() >= 4 && (points.len() - 1) % 3 == 0).then_some((points, ep))
}

fn json_str<'a>(v: &'a JsonValue, key: &str) -> Option<&'a str> {
    v.get(key).and_then(JsonValue::as_str)
}

/// Lays `root` out; `None` if the layout fails.
pub fn layout(
    root: &JsonValue,
    highlighted: &[Highlighted],
    skin: &Rc<SkinParam>,
    name: SName,
    sb: &dyn StringBounder,
) -> Option<JsonLayout> {
    let mut c = Collected {
        blocks: Vec::new(),
        dims: Vec::new(),
        heights: Vec::new(),
        widths: Vec::new(),
        edges: Vec::new(),
    };
    collect(&mut c, skin, name, sb, root, highlighted);
    let dot = build_dot(&c);
    log::debug!("json dot input:\n{dot}");
    let json = match graphviz_pure::render_dot(&dot, "dot", "json") {
        Ok(j) => j,
        Err(e) => {
            log::error!("graphviz failed: {e}");
            return None;
        }
    };
    let doc = JsonValue::parse(&json).ok()?;
    let objects = doc.get("objects")?;
    let mut centers: Vec<Option<Point>> = vec![None; c.blocks.len()];
    for i in 0..objects.len() {
        let o = objects.get(i)?;
        let Some(idx) = json_str(o, "name")
            .and_then(|n| n.strip_prefix('N'))
            .and_then(|n| n.parse::<usize>().ok())
        else {
            continue;
        };
        if idx < centers.len() {
            centers[idx] = parse_point(json_str(o, "pos")?);
        }
    }
    // Layout width is the drawn height and the other way round.
    let extent = |i: usize| (c.dims[i].height, c.dims[i].width);
    let mut max: f64 = 0.0;
    for (i, center) in centers.iter().enumerate() {
        let (_, h) = extent(i);
        max = max.max(center.as_ref()?.1 + h / 2.0);
    }
    let inv = |v: f64| max - v;
    let switch = |p: Point| (inv(p.1), p.0);
    let mut nodes = Vec::new();
    for (i, block) in c.blocks.iter().enumerate() {
        let (w, h) = extent(i);
        let (x, y) = centers[i]?;
        nodes.push((
            block.clone(),
            UTranslate::new(inv(y + h / 2.0), x - w / 2.0),
        ));
    }
    let mut curves = Vec::new();
    let edges = doc.get("edges");
    for k in 0..c.edges.len() {
        let pos = edges
            .and_then(|e| e.get(k))
            .and_then(|e| json_str(e, "pos"))
            .and_then(parse_spline);
        let Some((points, ep)) = pos else {
            continue;
        };
        curves.push(Curve {
            points: points.into_iter().map(switch).collect(),
            ep: ep.map(switch),
        });
    }
    Some(JsonLayout {
        nodes,
        curves,
        skin: skin.clone(),
        name,
    })
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The point `len` further from `direction` than `center` (`JsonCurve.supp`).
fn supp(center: Point, direction: Point, len: f64) -> Point {
    let full = distance(center, direction);
    let dx = (center.0 - direction.0) / full;
    let dy = (center.1 - direction.1) / full;
    (center.0 + dx * len, center.1 + dy * len)
}

/// Length of the stub inside the box where a link starts.
const VERY_FIRST_LINE: f64 = 13.0;

impl Curve {
    fn very_first(&self) -> Point {
        // A repeated first point has no direction: look at the next distinct one.
        let first = self.points[0];
        let towards = self
            .points
            .iter()
            .find(|p| distance(first, **p) > 1e-9)
            .copied()
            .unwrap_or((first.0 + 1.0, first.1));
        supp(first, towards, VERY_FIRST_LINE)
    }

    fn draw_curve(&self, color: &HColor, ug: &UGraphic) {
        let mut path = UPath::new();
        let first = self.very_first();
        path.move_to(first.0, first.1);
        path.line_to(self.points[0].0, self.points[0].1);
        let mut i = 1;
        while i + 2 < self.points.len() {
            let (p2, p3, p4) = (self.points[i], self.points[i + 1], self.points[i + 2]);
            path.cubic_to(p2.0, p2.1, p3.0, p3.1, p4.0, p4.1);
            i += 3;
        }
        ug.draw(path);
        if let (Some(ep), Some(last)) = (self.ep, self.points.last()) {
            draw_arrow(*last, ep, &ug.apply_back_color(color.clone()));
        }
    }

    fn draw_spot(&self, ug: &UGraphic) {
        let size = 3.0;
        let p = self.very_first();
        let ug = ug.apply_translate(UTranslate::new(p.0 - size, p.1 - size));
        ug.apply_stroke(UStroke::simple())
            .draw(UEllipse::build(2.0 * size, 2.0 * size));
    }
}

fn point_at(center: Point, alpha: f64, len: f64) -> Point {
    (center.0 + len * alpha.sin(), center.1 + len * alpha.cos())
}

/// The arrow head from `p1` to the tip `p2` (`Arrow.drawArrow`).
fn draw_arrow(p1: Point, p2: Point, ug: &UGraphic) {
    let ug = ug.apply_stroke(UStroke::simple());
    let dist = distance(p1, p2);
    let alpha = (p2.0 - p1.0).atan2(p2.1 - p1.1);
    let (factor, factor2) = (0.4, 0.3);
    let p3 = point_at(p1, alpha + std::f64::consts::FRAC_PI_2, dist * factor);
    let p4 = point_at(p1, alpha - std::f64::consts::FRAC_PI_2, dist * factor);
    let p11 = point_at(p1, alpha, dist * factor2);
    let mut path = UPath::new();
    path.move_to(p4.0, p4.1);
    path.line_to(p11.0, p11.1);
    path.line_to(p3.0, p3.1);
    path.line_to(p2.0, p2.1);
    path.line_to(p4.0, p4.1);
    path.close_path();
    ug.draw(path);
}

impl JsonLayout {
    fn style(&self, query: &crate::style::StyleQuery) -> Style {
        self.skin
            .style_builder()
            .merged_style(query)
            .expect("json diagram style")
    }

    fn with_stroke_and_color(&self, ug: &UGraphic, style: &Style) -> UGraphic {
        ug.apply_stroke(style.stroke())
            .apply_color(style.value(PName::LineColor).as_color())
    }

    pub fn draw(&self, ug: &UGraphic) {
        let style_node = self.style(&queries::NODE.add(self.name));
        let node_ug = self.with_stroke_and_color(ug, &style_node);
        for (block, pos) in &self.nodes {
            block.draw(&node_ug.apply_translate(*pos));
        }
        let style_arrow = self.style(&queries::ARROW.add(self.name));
        let color = style_arrow.value(PName::LineColor).as_color();
        let arrow_ug = self.with_stroke_and_color(ug, &style_arrow);
        for curve in &self.curves {
            curve.draw_curve(&color, &arrow_ug);
            curve.draw_spot(&arrow_ug.apply_back_color(color.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splines() {
        let (pts, ep) = parse_spline("e,10,20 1,2 3,4 5,6 7,8").unwrap();
        assert_eq!(pts.len(), 4);
        assert_eq!(ep, Some((10.0, 20.0)));
        assert!(parse_spline("1,2 3,4").is_none());
        assert!(parse_spline("x").is_none());
        assert_eq!(parse_spline("s,1,1 1,2 3,4 5,6 7,8").unwrap().1, None);
    }

    #[test]
    fn geometry() {
        let p = supp((10.0, 0.0), (0.0, 0.0), 5.0);
        assert_eq!(p, (15.0, 0.0));
        assert_eq!(cell(17.5), 18);
    }
}
