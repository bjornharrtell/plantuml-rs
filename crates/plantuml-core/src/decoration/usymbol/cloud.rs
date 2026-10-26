//! The cloud symbol (`USymbolCloud`), whose outline is drawn from a seeded pseudo-random walk.

use super::common::*;
use super::USymbol;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath};
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::style::SName;
use std::rc::Rc;

type Point = (f64, f64);

/// `java.util.Random`, needed to draw the same clouds as Java.
pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    const MULTIPLIER: i64 = 0x5DEECE66D;
    const MASK: i64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(0xB) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    pub fn next_double(&mut self) -> f64 {
        let hi = (self.next(26) as i64) << 27;
        let lo = self.next(27) as i64;
        (hi + lo) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Orthonormal frame going from `p1` to `p2` (`CoordinateChange`).
struct CoordinateChange {
    p1: Point,
    u: Point,
    v: Point,
    len: f64,
}

impl CoordinateChange {
    fn create(p1: Point, p2: Point) -> Self {
        let len = (p2.0 - p1.0).hypot(p2.1 - p1.1);
        let u = if len == 0.0 {
            (0.0, 0.0)
        } else {
            ((p2.0 - p1.0) / len, (p2.1 - p1.1) / len)
        };
        Self {
            p1,
            u,
            v: (-u.1, u.0),
            len,
        }
    }

    fn true_coordinate(&self, a: f64, b: f64) -> Point {
        (
            self.p1.0 + a * self.u.0 + b * self.v.0,
            self.p1.1 + a * self.u.1 + b * self.v.1,
        )
    }
}

fn rnd(rng: &mut JavaRandom, a: f64, b: f64) -> f64 {
    rng.next_double() * (b - a) + a
}

fn rnd_point(rng: &mut JavaRandom, pt: Point, v: f64) -> Point {
    let x = pt.0 + v * rng.next_double();
    let y = pt.1 + v * rng.next_double();
    (x, y)
}

fn mv_x(pt: Point, dx: f64) -> Point {
    (pt.0 + dx, pt.1)
}

fn mv_y(pt: Point, dy: f64) -> Point {
    (pt.0, pt.1 + dy)
}

fn bubble_line(
    rng: &mut JavaRandom,
    points: &mut Vec<Point>,
    p1: Point,
    p2: Point,
    mut bubble_size: f64,
) {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let mut nb = (length / bubble_size) as i64;
    if nb == 0 {
        bubble_size = length / 2.0;
        nb = (length / bubble_size) as i64;
    }
    for i in 0..nb {
        let pt = change.true_coordinate(i as f64 * length / nb as f64, 0.0);
        points.push(rnd_point(rng, pt, bubble_size * 0.2));
    }
}

fn special_line(
    bubble_size: f64,
    rng: &mut JavaRandom,
    points: &mut Vec<Point>,
    p1: Point,
    p2: Point,
) {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let offset = -rnd(rng, 1.0, 1.0 + 12.0_f64.min(bubble_size * 0.8));
    let middle = change.true_coordinate(length / 2.0, offset);
    bubble_line(rng, points, p1, middle, bubble_size);
    bubble_line(rng, points, middle, p2, bubble_size);
}

fn add_curve(rng: &mut JavaRandom, path: &mut UPath, p1: Point, p2: Point) {
    let change = CoordinateChange::create(p1, p2);
    let length = change.len;
    let coef = rnd(rng, 0.25, 0.35);
    let k1 = rnd(rng, 0.4, 0.55);
    let middle = change.true_coordinate(length * coef, -length * k1);
    let k2 = rnd(rng, 0.4, 0.55);
    let middle2 = change.true_coordinate(length * (1.0 - coef), -length * k2);
    path.cubic_to(middle.0, middle.1, middle2.0, middle2.1, p2.0, p2.1);
}

/// Outline of a cloud of the given size (`getSpecificFrontierForCloudNew`).
pub fn cloud_path(width: f64, height: f64) -> UPath {
    let mut rng = JavaRandom::new(width as i64 + 7919 * height as i64);
    let mut points: Vec<Point> = Vec::new();
    let mut bubble_size = 11.0;
    if width.max(height) / bubble_size > 16.0 {
        bubble_size = width.max(height) / 16.0;
    }
    let margin1 = 8.0;
    let a = (margin1, margin1);
    let b = (width - margin1, margin1);
    let c = (width - margin1, height - margin1);
    let d = (margin1, height - margin1);
    if width > 100.0 && height > 100.0 {
        let margin2 = 7.0;
        special_line(
            bubble_size,
            &mut rng,
            &mut points,
            mv_x(a, margin2),
            mv_x(b, -margin2),
        );
        points.push(mv_y(b, margin2));
        special_line(
            bubble_size,
            &mut rng,
            &mut points,
            mv_y(b, margin2),
            mv_y(c, -margin2),
        );
        points.push(mv_x(c, -margin2));
        special_line(
            bubble_size,
            &mut rng,
            &mut points,
            mv_x(c, -margin2),
            mv_x(d, margin2),
        );
        points.push(mv_y(d, -margin2));
        special_line(
            bubble_size,
            &mut rng,
            &mut points,
            mv_y(d, -margin2),
            mv_y(a, margin2),
        );
        points.push(mv_x(a, margin2));
    } else {
        special_line(bubble_size, &mut rng, &mut points, a, b);
        special_line(bubble_size, &mut rng, &mut points, b, c);
        special_line(bubble_size, &mut rng, &mut points, c, d);
        special_line(bubble_size, &mut rng, &mut points, d, a);
    }
    let mut result = UPath::new();
    let Some(&first) = points.first() else {
        return result;
    };
    points.push(first);
    result.move_to(first.0, first.1);
    for i in 0..points.len() - 1 {
        add_curve(&mut rng, &mut result, points[i], points[i + 1]);
    }
    result
}

fn draw_cloud(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let mut shape = cloud_path(width, height);
    shape.delta_shadow = fashion.delta_shadow();
    ug.draw(shape);
}

fn cloud_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_cloud(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 13.0, 13.0);
    })
}

/// `USymbolCloud`.
pub fn cloud() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Cloud],
        spec: SmallSpec {
            margin: Margin::new(15.0, 15.0, 15.0, 15.0),
            stencil: true,
            alignment: Some(HorizontalAlignment::Center),
            shape: draw_cloud,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: cloud_big,
        supp_height: 0,
        supp_width: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_random_matches_the_jdk() {
        // new Random(42): nextDouble() = 0.7275636800328681, then 0.6832234717598454.
        let mut r = JavaRandom::new(42);
        assert!((r.next_double() - 0.7275636800328681).abs() < 1e-15);
        assert!((r.next_double() - 0.6832234717598454).abs() < 1e-15);
    }
}
