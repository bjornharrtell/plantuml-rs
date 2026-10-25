//! Cubic Bezier paths computed by the graph layout (`DotPath`, `XCubicCurve2D`, `RectangleArea`).

use crate::klimt::geom::{MinMax, Side};

pub type Point = (f64, f64);

pub fn distance(a: Point, b: Point) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Squared distance from a point to a segment (`Line2D.ptSegDistSq`).
fn pt_seg_dist_sq(x1: f64, y1: f64, x2: f64, y2: f64, px: f64, py: f64) -> f64 {
    let x2 = x2 - x1;
    let y2 = y2 - y1;
    let mut px = px - x1;
    let mut py = py - y1;
    let mut dot = px * x2 + py * y2;
    let proj_sq;
    if dot <= 0.0 {
        proj_sq = 0.0;
    } else {
        px = x2 - px;
        py = y2 - py;
        dot = px * x2 + py * y2;
        if dot <= 0.0 {
            proj_sq = 0.0;
        } else {
            let p = dot * dot / (x2 * x2 + y2 * y2);
            proj_sq = p;
        }
    }
    let mut len_sq = px * px + py * py - proj_sq;
    if len_sq < 0.0 {
        len_sq = 0.0;
    }
    len_sq
}

/// A cubic Bezier curve (`XCubicCurve2D`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic {
    pub x1: f64,
    pub y1: f64,
    pub ctrlx1: f64,
    pub ctrly1: f64,
    pub ctrlx2: f64,
    pub ctrly2: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Cubic {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x1: f64,
        y1: f64,
        ctrlx1: f64,
        ctrly1: f64,
        ctrlx2: f64,
        ctrly2: f64,
        x2: f64,
        y2: f64,
    ) -> Cubic {
        Cubic {
            x1,
            y1,
            ctrlx1,
            ctrly1,
            ctrlx2,
            ctrly2,
            x2,
            y2,
        }
    }

    pub fn p1(&self) -> Point {
        (self.x1, self.y1)
    }

    pub fn p2(&self) -> Point {
        (self.x2, self.y2)
    }

    pub fn ctrl_p1(&self) -> Point {
        (self.ctrlx1, self.ctrly1)
    }

    pub fn ctrl_p2(&self) -> Point {
        (self.ctrlx2, self.ctrly2)
    }

    pub fn length(&self) -> f64 {
        ((self.x2 - self.x1).powi(2) + (self.y2 - self.y1).powi(2)).sqrt()
    }

    pub fn flatness_sq(&self) -> f64 {
        pt_seg_dist_sq(self.x1, self.y1, self.x2, self.y2, self.ctrlx1, self.ctrly1).max(
            pt_seg_dist_sq(self.x1, self.y1, self.x2, self.y2, self.ctrlx2, self.ctrly2),
        )
    }

    pub fn subdivide(&self) -> (Cubic, Cubic) {
        let (x1, y1, x2, y2) = (self.x1, self.y1, self.x2, self.y2);
        let mut centerx = (self.ctrlx1 + self.ctrlx2) / 2.0;
        let mut centery = (self.ctrly1 + self.ctrly2) / 2.0;
        let ctrlx1 = (x1 + self.ctrlx1) / 2.0;
        let ctrly1 = (y1 + self.ctrly1) / 2.0;
        let ctrlx2 = (x2 + self.ctrlx2) / 2.0;
        let ctrly2 = (y2 + self.ctrly2) / 2.0;
        let ctrlx12 = (ctrlx1 + centerx) / 2.0;
        let ctrly12 = (ctrly1 + centery) / 2.0;
        let ctrlx21 = (ctrlx2 + centerx) / 2.0;
        let ctrly21 = (ctrly2 + centery) / 2.0;
        centerx = (ctrlx12 + ctrlx21) / 2.0;
        centery = (ctrly12 + ctrly21) / 2.0;
        (
            Cubic::new(x1, y1, ctrlx1, ctrly1, ctrlx12, ctrly12, centerx, centery),
            Cubic::new(centerx, centery, ctrlx21, ctrly21, ctrlx2, ctrly2, x2, y2),
        )
    }

    pub fn reversed(&self) -> Cubic {
        Cubic::new(
            self.x2,
            self.y2,
            self.ctrlx2,
            self.ctrly2,
            self.ctrlx1,
            self.ctrly1,
            self.x1,
            self.y1,
        )
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Cubic {
        Cubic::new(
            self.x1 + dx,
            self.y1 + dy,
            self.ctrlx1 + dx,
            self.ctrly1 + dy,
            self.ctrlx2 + dx,
            self.ctrly2 + dy,
            self.x2 + dx,
            self.y2 + dy,
        )
    }

    pub fn starting_angle(&self) -> f64 {
        if self.p1() == self.ctrl_p1() {
            angle(self.p1(), self.p2())
        } else {
            angle(self.p1(), self.ctrl_p1())
        }
    }

    pub fn ending_angle(&self) -> f64 {
        if self.ctrl_p2() == self.p2() {
            angle(self.p1(), self.p2())
        } else {
            angle(self.ctrl_p2(), self.p2())
        }
    }
}

fn angle(p1: Point, p2: Point) -> f64 {
    (p2.1 - p1.1).atan2(p2.0 - p1.0)
}

/// Axis aligned area; the max sides are excluded (`RectangleArea`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangleArea {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl RectangleArea {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> RectangleArea {
        RectangleArea {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn build(p1: Point, p2: Point) -> RectangleArea {
        RectangleArea::new(
            p1.0.min(p2.0),
            p1.1.min(p2.1),
            p1.0.max(p2.0),
            p1.1.max(p2.1),
        )
    }

    pub fn move_by(&self, dx: f64, dy: f64) -> RectangleArea {
        RectangleArea::new(
            self.min_x + dx,
            self.min_y + dy,
            self.max_x + dx,
            self.max_y + dy,
        )
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn contains(&self, p: Point) -> bool {
        p.0 >= self.min_x && p.0 < self.max_x && p.1 >= self.min_y && p.1 < self.max_y
    }

    pub fn center(&self) -> Point {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }

    pub fn closest_side(&self, pt: Point) -> Option<Side> {
        let north = (self.min_y - pt.1).abs();
        let south = (self.max_y - pt.1).abs();
        let west = (self.min_x - pt.0).abs();
        let east = (self.max_x - pt.0).abs();
        let smaller = |v: f64, a: f64, b: f64, c: f64| v <= a && v <= b && v <= c;
        if smaller(north, west, east, south) {
            Some(Side::North)
        } else if smaller(south, north, west, east) {
            Some(Side::South)
        } else if smaller(east, north, west, south) {
            Some(Side::East)
        } else if smaller(west, north, east, south) {
            Some(Side::West)
        } else {
            None
        }
    }
}

/// A point and the direction of the line there (`PointAndAngle`).
#[derive(Debug, Clone, Copy)]
pub struct PointAndAngle {
    pub point: Point,
    pub angle: f64,
}

/// A spline made of cubic Beziers (`DotPath`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DotPath {
    beziers: Vec<Cubic>,
    pub comment: Option<String>,
    pub code_line: Option<String>,
}

impl DotPath {
    pub fn from_beziers(beziers: Vec<Cubic>) -> DotPath {
        DotPath {
            beziers,
            comment: None,
            code_line: None,
        }
    }

    pub fn beziers(&self) -> &[Cubic] {
        &self.beziers
    }

    pub fn start_point(&self) -> Point {
        self.beziers[0].p1()
    }

    pub fn end_point(&self) -> Point {
        self.beziers[self.beziers.len() - 1].p2()
    }

    pub fn set_comment_and_code_line(&mut self, comment: String, code_line: Option<String>) {
        self.comment = Some(comment);
        self.code_line = code_line;
    }

    pub fn move_delta(&mut self, dx: f64, dy: f64) {
        for c in &mut self.beziers {
            *c = c.translated(dx, dy);
        }
    }

    pub fn move_start_point(&mut self, mut dx: f64, mut dy: f64) {
        if self.beziers.len() > 1 && (dx * dx + dy * dy).sqrt() >= self.beziers[0].length() {
            dx -= self.beziers[1].x1 - self.beziers[0].x1;
            dy -= self.beziers[1].y1 - self.beziers[0].y1;
            self.beziers.remove(0);
        }
        let b = &mut self.beziers[0];
        b.x1 += dx;
        b.y1 += dy;
        b.ctrlx1 += dx;
        b.ctrly1 += dy;
    }

    pub fn move_end_point(&mut self, dx: f64, dy: f64) {
        let n = self.beziers.len();
        let b = &mut self.beziers[n - 1];
        b.x2 += dx;
        b.y2 += dy;
        b.ctrlx2 += dx;
        b.ctrly2 += dy;
    }

    pub fn start_angle(&self) -> f64 {
        let first = &self.beziers[0];
        let mut dx = first.ctrlx1 - first.x1;
        let mut dy = first.ctrly1 - first.y1;
        if dx == 0.0 && dy == 0.0 {
            dx = first.x2 - first.x1;
            dy = first.y2 - first.y1;
        }
        dy.atan2(dx)
    }

    pub fn end_angle(&self) -> f64 {
        let last = &self.beziers[self.beziers.len() - 1];
        let mut dx = last.x2 - last.ctrlx2;
        let mut dy = last.y2 - last.ctrly2;
        if dx == 0.0 && dy == 0.0 {
            dx = last.x2 - last.x1;
            dy = last.y2 - last.y1;
        }
        dy.atan2(dx)
    }

    pub fn reverse(&self) -> DotPath {
        DotPath::from_beziers(self.beziers.iter().rev().map(Cubic::reversed).collect())
    }

    pub fn min_max(&self) -> MinMax {
        let mut r = MinMax::empty(false);
        for c in &self.beziers {
            r = r
                .add_point(c.x1, c.y1)
                .add_point(c.x2, c.y2)
                .add_point(c.ctrlx1, c.ctrly1)
                .add_point(c.ctrlx2, c.ctrly2);
        }
        r
    }

    pub fn min_dist(&self, r: Point) -> f64 {
        let mut result = f64::MAX;
        for c in &self.beziers {
            for p in [c.p1(), c.p2(), c.ctrl_p1(), c.ctrl_p2()] {
                result = result.min(distance(r, p));
            }
        }
        result
    }

    pub fn is_line(&self) -> bool {
        self.beziers.iter().all(|c| c.flatness_sq() <= 0.001)
    }

    /// Control points of the subdivided curves (`sample`), in a deterministic order.
    pub fn sample(&self) -> Vec<Point> {
        fn go(bez: &Cubic, out: &mut Vec<Point>) {
            let (p1, p2) = (bez.ctrl_p1(), bez.ctrl_p2());
            if bez.flatness_sq() > 0.5 || distance(p1, p2) > 4.0 {
                let (l, r) = bez.subdivide();
                go(&l, out);
                go(&r, out);
            } else {
                for p in [p1, p2] {
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
        }
        let mut out = Vec::new();
        for b in &self.beziers {
            go(b, &mut out);
        }
        out
    }

    fn cost(&self, pt: Point) -> f64 {
        let (s, e) = (self.start_point(), self.end_point());
        let sq = |a: Point, b: Point| (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2);
        sq(pt, s) + sq(pt, e)
    }

    /// The point nearest to the ends and the direction there (`getMiddle`).
    pub fn middle(&self) -> PointAndAngle {
        let mut result: Option<Point> = None;
        let mut ang = 0.0;
        for bez in &self.beziers {
            let (left, right) = bez.subdivide();
            let cands = [
                (left.p1(), left.starting_angle()),
                (left.p2(), left.ending_angle()),
                (right.p1(), right.starting_angle()),
                (right.p2(), right.ending_angle()),
            ];
            for (i, (p, a)) in cands.into_iter().enumerate() {
                if i == 0 && result.is_none() || self.cost(p) < self.cost(result.expect("set")) {
                    result = Some(p);
                    ang = a;
                }
            }
        }
        PointAndAngle {
            point: result.expect("non empty path"),
            angle: ang,
        }
    }

    /// Cuts the path where it enters the head/tail clusters (`simulateCompound`).
    pub fn simulate_compound(
        &self,
        head: Option<RectangleArea>,
        tail: Option<RectangleArea>,
    ) -> DotPath {
        if head.is_none() && tail.is_none() {
            return self.clone();
        }
        let mut me = self.clone();
        if let Some(tail) = tail {
            if tail.contains(self.start_point()) {
                let mut result: Vec<Cubic> = Vec::new();
                let mut idx = 0;
                while idx + 1 < self.beziers.len() && tail.contains(self.beziers[idx].p2()) {
                    idx += 1;
                }
                if !tail.contains(self.beziers[idx].p2()) {
                    let mut current = self.beziers[idx];
                    for _ in 0..8 {
                        let (part1, part2) = current.subdivide();
                        if tail.contains(part1.p2()) {
                            current = part2;
                        } else {
                            result.insert(0, part2);
                            current = part1;
                        }
                    }
                    for b in &self.beziers[idx + 1..] {
                        result.push(*b);
                    }
                    me = DotPath::from_beziers(result);
                }
            }
        }
        if let Some(head) = head {
            if head.contains(self.end_point()) {
                let mut result: Vec<Cubic> = Vec::new();
                for cur in &me.beziers {
                    let mut current = *cur;
                    if !head.contains(current.p2()) {
                        result.push(current);
                    } else {
                        if head.contains(current.p1()) {
                            return me;
                        }
                        for _ in 0..8 {
                            let (part1, part2) = current.subdivide();
                            if head.contains(part1.p2()) {
                                current = part1;
                            } else {
                                result.push(part1);
                                current = part2;
                            }
                        }
                        return DotPath::from_beziers(result);
                    }
                }
            }
        }
        me
    }
}

impl DotPath {
    /// The path as a drawable shape (`toUPath`).
    pub fn to_upath(&self) -> crate::klimt::drawing::ugraphic::UPath {
        let mut path = crate::klimt::drawing::ugraphic::UPath::new();
        let mut start = true;
        for b in &self.beziers {
            if start {
                path.move_to(b.x1, b.y1);
                start = false;
            }
            path.cubic_to(b.ctrlx1, b.ctrly1, b.ctrlx2, b.ctrly2, b.x2, b.y2);
        }
        path
    }
}

impl From<&DotPath> for crate::klimt::drawing::ugraphic::UShape {
    fn from(p: &DotPath) -> Self {
        p.to_upath().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> DotPath {
        DotPath::from_beziers(vec![Cubic::new(0.0, 0.0, 0.0, 10.0, 0.0, 20.0, 0.0, 30.0)])
    }

    #[test]
    fn ends_and_angles() {
        let p = line();
        assert_eq!(p.start_point(), (0.0, 0.0));
        assert_eq!(p.end_point(), (0.0, 30.0));
        assert!((p.start_angle() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(p.is_line());
        let r = p.reverse();
        assert_eq!(r.start_point(), (0.0, 30.0));
    }

    #[test]
    fn middle_and_move() {
        let mut p = line();
        let m = p.middle();
        assert!((m.point.1 - 15.0).abs() < 1e-9 || m.point.1 < 30.0);
        p.move_end_point(0.0, -10.0);
        assert_eq!(p.end_point(), (0.0, 20.0));
        p.move_delta(5.0, 1.0);
        assert_eq!(p.start_point(), (5.0, 1.0));
        assert!(p.min_dist((5.0, 1.0)) < 1e-9);
        assert!(!p.sample().is_empty());
    }

    #[test]
    fn compound() {
        let p = line();
        let head = RectangleArea::new(-5.0, 20.0, 5.0, 40.0);
        let cut = p.simulate_compound(Some(head), None);
        assert!(cut.end_point().1 < 21.0);
        let tail = RectangleArea::new(-5.0, -5.0, 5.0, 10.0);
        let cut = p.simulate_compound(None, Some(tail));
        assert!(cut.start_point().1 >= 9.0);
    }

    #[test]
    fn area() {
        let a = RectangleArea::build((10.0, 10.0), (0.0, 0.0));
        assert!(a.contains((0.0, 0.0)));
        assert!(!a.contains((10.0, 5.0)));
        assert_eq!(a.closest_side((5.0, -3.0)), Some(Side::North));
        assert_eq!(a.center(), (5.0, 5.0));
    }
}
