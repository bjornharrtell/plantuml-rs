//! Inheritance arrows grouped on one parent (`skinparam groupInheritance`): `Neighborhood` and the
//! `EntityImageProtected` that makes room for it.

use super::image::IEntityImage;
use super::types::{PortGeometry, ShapeType};
use crate::abel::{Entity, Link};
use crate::klimt::dotpath::Point;
use crate::klimt::drawing::ugraphic::{UGraphic, UPolygon, UShape, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{InnerRect, TextBlock};
use crate::style::Margins;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

/// Contact points of an edge with its two nodes, in drawing coordinates (`getStartContactPoint`).
pub type Contacts<'a> = &'a dyn Fn(&Link) -> Option<(Point, Point)>;

pub struct Neighborhood {
    leaf: Entity,
    sametail_links: Vec<Link>,
    all_but_sametails: Vec<Link>,
}

impl Neighborhood {
    pub fn new(leaf: Entity, sametail_links: Vec<Link>, all: Vec<Link>) -> Self {
        let all_but_sametails = all
            .into_iter()
            .filter(|l| !sametail_links.contains(l))
            .collect();
        Self {
            leaf,
            sametail_links,
            all_but_sametails,
        }
    }

    pub fn draw(&self, ug: &UGraphic, min: Point, dim: Dimension, contacts: Contacts) {
        let (min_x, min_y) = min;
        let rect = (min_x, min_y, min_x + dim.width, min_y + dim.height);
        let center = (min_x + dim.width / 2.0, min_y + dim.height / 2.0);
        let mut points: Vec<Point> = Vec::new();
        for link in &self.sametail_links {
            if let Some((start, _)) = contacts(link) {
                if !points.contains(&start) {
                    points.push(start);
                }
            }
        }
        for pt in points {
            let Some(inter) = intersection(rect, center, pt) else {
                continue;
            };
            let theta = (center.0 - pt.0).atan2(-(center.1 - pt.1));
            let middle = draw_extends(ug, inter, theta);
            draw_line(ug, middle, pt);
        }
        for link in &self.all_but_sametails {
            let Some((start, end)) = contacts(link) else {
                continue;
            };
            let contact = if link.entity1() == self.leaf {
                start
            } else {
                end
            };
            let Some(inter) = intersection(rect, center, contact) else {
                continue;
            };
            draw_line(ug, inter, contact);
        }
    }
}

fn draw_extends(ug: &UGraphic, contact: Point, theta: f64) -> Point {
    let mut poly = UPolygon::default();
    poly.add_point(0.0, 0.0);
    poly.add_point(7.0, 20.0);
    poly.add_point(-7.0, 20.0);
    poly.rotate(theta);
    let p1 = (contact.0 + poly.points[1].0, contact.1 + poly.points[1].1);
    let p2 = (contact.0 + poly.points[2].0, contact.1 + poly.points[2].1);
    ug.apply_translate(UTranslate::new(contact.0, contact.1))
        .draw(UShape::Polygon(poly));
    ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0)
}

fn draw_line(ug: &UGraphic, a: Point, b: Point) {
    ug.apply_translate(UTranslate::new(a.0, a.1))
        .draw(UShape::Line(crate::klimt::drawing::ugraphic::ULine {
            dx: b.0 - a.0,
            dy: b.1 - a.1,
            delta_shadow: 0.0,
        }));
}

/// Where the segment from `p1` to `p2` crosses the border of `rect` `(min_x, min_y, max_x, max_y)`.
fn intersection(rect: (f64, f64, f64, f64), p1: Point, p2: Point) -> Option<Point> {
    let (x0, y0, x1, y1) = rect;
    [
        ((x0, y0), (x1, y0)),
        ((x0, y1), (x1, y1)),
        ((x0, y0), (x0, y1)),
        ((x1, y0), (x1, y1)),
    ]
    .into_iter()
    .find_map(|(a, b)| segment_intersection(a, b, p1, p2))
}

fn segment_intersection(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
    const EPSILON: f64 = 0.001;
    let (x1, y1, x2, y2, x3, y3, x4, y4) = (p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, p4.0, p4.1);
    let d = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if d == 0.0 {
        return None;
    }
    let xi = ((x3 - x4) * (x1 * y2 - y1 * x2) - (x1 - x2) * (x3 * y4 - y3 * x4)) / d;
    let yi = ((y3 - y4) * (x1 * y2 - y1 * x2) - (y1 - y2) * (x3 * y4 - y3 * x4)) / d;
    let outside = |a: f64, b: f64, v: f64| v + EPSILON < a.min(b) || v - EPSILON > a.max(b);
    if outside(x1, x2, xi) || outside(x3, x4, xi) || outside(y1, y2, yi) || outside(y3, y4, yi) {
        return None;
    }
    Some((xi, yi))
}

/// An entity with room around it for the grouped arrows (`EntityImageProtected`).
pub struct EntityImageProtected {
    orig: Rc<dyn IEntityImage>,
    border: f64,
    neighborhood: Neighborhood,
}

impl EntityImageProtected {
    pub fn new(orig: Rc<dyn IEntityImage>, border: f64, neighborhood: Neighborhood) -> Self {
        Self {
            orig,
            border,
            neighborhood,
        }
    }

    /// Draws the grouped arrows around the entity placed at `corner` (`drawUntranslated`).
    pub fn draw_untranslated(&self, ug: &UGraphic, corner: Point, contacts: Contacts) {
        let dim = self.orig.calculate_dimension(ug.string_bounder());
        let min = (corner.0 + self.border, corner.1 + self.border);
        self.neighborhood.draw(ug, min, dim, contacts);
    }
}

impl TextBlock for EntityImageProtected {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.orig.calculate_dimension(sb);
        d.delta(2.0 * self.border, 2.0 * self.border)
    }

    fn draw(&self, ug: &UGraphic) {
        self.orig
            .draw(&ug.apply_translate(UTranslate::new(self.border, self.border)));
    }

    fn backcolor(&self) -> Option<crate::klimt::color::HColor> {
        self.orig.backcolor()
    }

    fn inner_position(&self, member: &str, sb: &dyn StringBounder) -> Option<InnerRect> {
        let (x, y, w, h) = self.orig.inner_position(member, sb)?;
        Some((x + self.border, y + self.border, w, h))
    }
}

impl IEntityImage for EntityImageProtected {
    fn shape_type(&self) -> ShapeType {
        self.orig.shape_type()
    }

    fn shield(&self, sb: &dyn StringBounder) -> Margins {
        self.orig.shield(sb)
    }

    fn overscan_x(&self, sb: &dyn StringBounder) -> f64 {
        self.orig.overscan_x(sb)
    }

    fn is_hidden(&self) -> bool {
        self.orig.is_hidden()
    }

    fn ports(&self, sb: &dyn StringBounder) -> Vec<PortGeometry> {
        self.orig.ports(sb)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `DotData.removeIrrelevantSametail`: arrows to one parent are grouped when at least `limit` of them
/// share it. Returns the neighborhood of each such parent, by entity uid.
pub fn remove_irrelevant_sametail(links: &[Link], limit: usize) -> HashMap<String, Neighborhood> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for link in links {
        if link.type_().decor2().is_extends_like() {
            link.set_sametail(Some(link.entity1().uid()));
        }
        let Some(tail) = link.sametail() else {
            continue;
        };
        match counts.iter_mut().find(|(k, _)| *k == tail) {
            Some((_, n)) => *n += 1,
            None => counts.push((tail, 1)),
        }
    }
    let mut result = HashMap::new();
    for (key, count) in counts {
        if count < limit {
            continue;
        }
        let some: Vec<Link> = links
            .iter()
            .filter(|l| l.sametail().as_deref() == Some(key.as_str()))
            .cloned()
            .collect();
        let leaf = some[0].entity1();
        let of_leaf: Vec<Link> = links
            .iter()
            .filter(|l| l.contains(&leaf))
            .cloned()
            .collect();
        result.insert(key, Neighborhood::new(leaf, some, of_leaf));
    }
    for link in links {
        if let Some(tail) = link.sametail() {
            if !result.contains_key(&tail) {
                link.set_sametail(None);
            }
        }
    }
    result
}
