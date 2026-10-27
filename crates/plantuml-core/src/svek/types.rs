//! Small types of the Graphviz glue (`ShapeType`, `EntityPort`, `ColorSequence`, `SvgResult`).

use crate::klimt::dotpath::{Cubic, DotPath, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    Rectangle,
    RectanglePort,
    RectangleWithCircleInside,
    RectangleHtmlForPorts,
    RoundRectangle,
    Circle,
    Oval,
    Diamond,
    Octagon,
    Folder,
    Hexagon,
    Port,
}

/// Hands out the unique colors used to find shapes in the layout output (`ColorSequence`).
#[derive(Debug)]
pub struct ColorSequence {
    cpt: i32,
}

impl Default for ColorSequence {
    fn default() -> Self {
        ColorSequence { cpt: 1 }
    }
}

impl ColorSequence {
    pub fn get_value(&mut self) -> i32 {
        self.cpt += 1;
        self.cpt
    }
}

/// `#rrggbb` of a color number, lowercase (`XColor.toHexRGBColor`).
pub fn hex_rgb(color: i32) -> String {
    format!("#{:06x}", color & 0xFFFFFF)
}

/// A row of a node that an edge can be attached to (`PortGeometry`): the member of a class
/// addressed by `Foo::member`, written as a `PORT` row in the node's HTML table.
#[derive(Debug, Clone, PartialEq)]
pub struct PortGeometry {
    /// `Ports.encodePortNameToId`: `p` and the MD5 of the member name.
    pub id: String,
    /// Distance from the top of the node to the top of the row.
    pub position: f64,
    /// Height of the row.
    pub height: f64,
    /// Which of several members with the same name wins (Java's 100 for a word match, 50 for
    /// a substring).
    pub score: i32,
}

impl PortGeometry {
    pub fn new(id: String, position: f64, height: f64, score: i32) -> PortGeometry {
        PortGeometry {
            id,
            position,
            height,
            score,
        }
    }
}

/// Where an edge touches a node (`EntityPort`).
#[derive(Debug, Clone, PartialEq)]
pub struct EntityPort {
    entity_uid: String,
    port_id: Option<String>,
}

impl EntityPort {
    pub fn create(entity_uid: &str, port_id: Option<String>) -> EntityPort {
        EntityPort {
            entity_uid: entity_uid.to_string(),
            port_id,
        }
    }

    pub fn for_port(entity_uid: &str) -> EntityPort {
        EntityPort::create(entity_uid, Some("P".to_string()))
    }

    pub fn full_string(&self) -> String {
        match &self.port_id {
            Some(p) => format!("{}:{}", self.entity_uid, p),
            None => self.entity_uid.clone(),
        }
    }

    fn is_shielded(&self) -> bool {
        self.entity_uid.ends_with(":h")
    }

    pub fn prefix(&self) -> String {
        if self.is_shielded() {
            self.entity_uid[..self.entity_uid.len() - 2].to_string()
        } else {
            self.entity_uid.clone()
        }
    }

    pub fn starts_with(&self, center_id: &str) -> bool {
        self.entity_uid.starts_with(center_id)
    }

    pub fn equals_id(&self, other: &EntityPort) -> bool {
        self.entity_uid == other.entity_uid
    }
}

/// Pixels to inches as written in DOT (`pixelToInches`).
pub fn pixel_to_inches(pixel: f64) -> String {
    format!("{:.6}", pixel / 72.0)
}

pub fn min_xy(points: &[Point]) -> Point {
    let mut minx = points[0].0;
    let mut miny = points[0].1;
    for p in &points[1..] {
        minx = minx.min(p.0);
        miny = miny.min(p.1);
    }
    (minx, miny)
}

pub fn max_xy(points: &[Point]) -> Point {
    let mut maxx = points[0].0;
    let mut maxy = points[0].1;
    for p in &points[1..] {
        maxx = maxx.max(p.0);
        maxy = maxy.max(p.1);
    }
    (maxx, maxy)
}

/// Value of `name="..."` after position `starting` (`SvekUtils.getValue`).
pub fn get_value(svg: &str, starting: usize, name: &str) -> Option<f64> {
    let key = format!("{name}=\"");
    let p1 = svg[starting..].find(&key)? + starting + key.len();
    let p2 = svg[p1..].find('"')? + p1;
    svg[p1..p2].parse().ok()
}

pub const D_EQUALS: &str = "d=\"";
pub const POINTS_EQUALS: &str = "points=\"";

/// A piece of the layout SVG with the y flip applied to the points read from it (`SvgResult`).
#[derive(Debug, Clone)]
pub struct SvgResult<'a> {
    svg: &'a str,
    full_height: f64,
}

impl<'a> SvgResult<'a> {
    pub fn new(svg: &'a str, full_height: f64) -> SvgResult<'a> {
        SvgResult { svg, full_height }
    }

    pub fn svg(&self) -> &'a str {
        self.svg
    }

    pub fn index_of(&self, s: &str, pos: usize) -> Option<usize> {
        self.svg.get(pos..)?.find(s).map(|i| i + pos)
    }

    pub fn substring(&self, pos: usize) -> SvgResult<'a> {
        SvgResult {
            svg: &self.svg[pos..],
            full_height: self.full_height,
        }
    }

    pub fn substring_range(&self, start: usize, end: usize) -> SvgResult<'a> {
        SvgResult {
            svg: &self.svg[start..end],
            full_height: self.full_height,
        }
    }

    fn first_point(&self, tmp: &str) -> Option<Point> {
        let mut it = tmp.split(',').filter(|s| !s.is_empty());
        let x: f64 = it.next()?.trim().parse().ok()?;
        let y: f64 = it.next()?.trim().parse().ok()?;
        Some((x, y * 1.0 + self.full_height))
    }

    /// Points separated by any char of `separator` (`getPoints`); empty on a parse failure.
    pub fn points(&self, separator: &str) -> Vec<Point> {
        let mut out = Vec::new();
        for tok in self
            .svg
            .split(|c| separator.contains(c))
            .filter(|s| !s.is_empty())
        {
            match self.first_point(tok) {
                Some(p) => out.push(p),
                None => return Vec::new(),
            }
        }
        out
    }

    pub fn next_point(&self) -> Option<Point> {
        self.first_point(self.svg)
    }

    /// Points of the attribute that starts with `searched` (`extractList`).
    pub fn extract_list(&self, searched: &str) -> Vec<Point> {
        let Some(p2) = self.index_of(searched, 0) else {
            return Vec::new();
        };
        let Some(p3) = self.index_of("\"", p2 + searched.len()) else {
            return Vec::new();
        };
        self.substring_range(p2 + searched.len(), p3).points(" MC")
    }

    /// Position of the first stroke or fill using `color` (`getIndexFromColor`).
    pub fn index_from_color(&self, color: i32) -> Option<usize> {
        let hex = hex_rgb(color).to_lowercase();
        for key in [
            format!("stroke=\"{hex}\""),
            format!(";stroke:{hex};"),
            format!("fill=\"{hex}\""),
        ] {
            if let Some(i) = self.svg.find(&key) {
                return Some(i);
            }
        }
        None
    }

    pub fn is_path_consistent(&self) -> bool {
        self.svg.starts_with('M')
    }

    /// The `M x,y C ...` path data as a [`DotPath`] (`toDotPath`).
    pub fn to_dot_path(&self) -> Option<DotPath> {
        if !self.is_path_consistent() {
            return None;
        }
        let pos_c = self.index_of("C", 0)?;
        let start = self.substring_range(1, pos_c).next_point()?;
        let pts = self.substring(pos_c + 1).points(" ");
        let mut beziers = Vec::new();
        let (mut x, mut y) = start;
        for tri in pts.chunks(3) {
            if tri.len() < 3 {
                break;
            }
            beziers.push(Cubic::new(
                x, y, tri[0].0, tri[0].1, tri[1].0, tri[1].1, tri[2].0, tri[2].1,
            ));
            x = tri[2].0;
            y = tri[2].1;
        }
        Some(DotPath::from_beziers(beziers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_and_inches() {
        assert_eq!(hex_rgb(0x2b), "#00002b");
        assert_eq!(pixel_to_inches(36.0), "0.500000");
        let mut c = ColorSequence::default();
        assert_eq!(c.get_value(), 2);
    }

    #[test]
    fn ports() {
        let p = EntityPort::create("sh0002:h", None);
        assert_eq!(p.prefix(), "sh0002");
        assert_eq!(EntityPort::for_port("sh0003").full_string(), "sh0003:P");
        assert!(p.starts_with("sh"));
    }

    #[test]
    fn svg_points() {
        let s = "<polygon points=\"10,-20 30,-40 \" />";
        let r = SvgResult::new(s, 100.0);
        let pts = r.extract_list(POINTS_EQUALS);
        assert_eq!(pts, vec![(10.0, 80.0), (30.0, 60.0)]);
        assert_eq!(min_xy(&pts), (10.0, 60.0));
        assert_eq!(max_xy(&pts), (30.0, 80.0));
        let path = SvgResult::new("M0,-0 C0,-10 0,-20 0,-30", 50.0);
        let dp = path.to_dot_path().unwrap();
        assert_eq!(dp.start_point(), (0.0, 50.0));
        assert_eq!(dp.end_point(), (0.0, 20.0));
        assert_eq!(get_value("<ellipse cx=\"5\" cy=\"6\"", 0, "cy"), Some(6.0));
    }

    #[test]
    fn color_lookup() {
        let svg = "<path stroke=\"#00002b\" d=\"M0,0\"/>";
        let r = SvgResult::new(svg, 0.0);
        assert_eq!(r.index_from_color(0x2b), Some(6));
        assert_eq!(r.index_from_color(0x2c), None);
    }
}
