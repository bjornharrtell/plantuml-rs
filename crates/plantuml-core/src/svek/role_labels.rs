//! Placement of role labels next to the quantifier of a link end (`RoleLabels`).

use crate::klimt::dotpath::{distance, Point, RectangleArea};
use crate::klimt::font::Dimension;

const GAP: f64 = 2.0;
const MIN_DISTANCE_FOR_DIRECTION: f64 = 15.0;
const SEARCH_STEP: f64 = 2.0;
const SEARCH_RANGE: f64 = 30.0;

#[allow(clippy::too_many_arguments)]
pub fn position(
    q_dim: Dimension,
    r_dim: Dimension,
    quantifier_pos: Point,
    this_endpoint: Point,
    other_endpoint: Point,
    path_samples: Option<&[Point]>,
    obstacles: &[RectangleArea],
) -> Point {
    let segment;
    let samples: &[Point] = match path_samples {
        Some(s) => s,
        None => {
            segment = sample_segment(this_endpoint, other_endpoint);
            &segment
        }
    };
    let towards = point_away_from(this_endpoint, other_endpoint, samples);
    let dir_x = towards.0 - this_endpoint.0;
    let dir_y = towards.1 - this_endpoint.1;
    if dir_x.abs() + dir_y.abs() < 0.001 {
        return (quantifier_pos.0, quantifier_pos.1 + q_dim.height);
    }
    let preferred = mirrored_position(
        q_dim,
        r_dim,
        quantifier_pos,
        this_endpoint,
        dir_x,
        dir_y,
        samples,
    );
    let mut forbidden: Vec<RectangleArea> = obstacles.to_vec();
    forbidden.push(RectangleArea::new(
        quantifier_pos.0,
        quantifier_pos.1,
        quantifier_pos.0 + q_dim.width,
        quantifier_pos.1 + q_dim.height,
    ));
    if is_free(preferred, r_dim, &forbidden, None) {
        return preferred;
    }
    search_free_position(
        q_dim,
        r_dim,
        quantifier_pos,
        this_endpoint,
        dir_x,
        dir_y,
        samples,
        &forbidden,
    )
    .unwrap_or(preferred)
}

fn mirrored_position(
    q_dim: Dimension,
    r_dim: Dimension,
    quantifier_pos: Point,
    this_endpoint: Point,
    dir_x: f64,
    dir_y: f64,
    samples: &[Point],
) -> Point {
    if dir_y.abs() >= dir_x.abs() {
        let role_y = quantifier_pos.1;
        let q_center_x = quantifier_pos.0 + q_dim.width / 2.0;
        let line_x = this_endpoint.0;
        let mut role_x;
        if q_center_x < line_x {
            role_x = line_x + GAP;
            for pt in samples {
                if pt.1 >= role_y - GAP && pt.1 <= role_y + r_dim.height + GAP {
                    role_x = role_x.max(pt.0 + GAP);
                }
            }
        } else {
            role_x = line_x - r_dim.width - GAP;
            for pt in samples {
                if pt.1 >= role_y - GAP && pt.1 <= role_y + r_dim.height + GAP {
                    role_x = role_x.min(pt.0 - r_dim.width - GAP);
                }
            }
        }
        return (role_x, role_y);
    }
    let role_x = quantifier_pos.0;
    let q_center_y = quantifier_pos.1 + q_dim.height / 2.0;
    let line_y = this_endpoint.1;
    let mut role_y;
    if q_center_y < line_y {
        role_y = line_y + GAP;
        for pt in samples {
            if pt.0 >= role_x - GAP && pt.0 <= role_x + r_dim.width + GAP {
                role_y = role_y.max(pt.1 + GAP);
            }
        }
    } else {
        role_y = line_y - r_dim.height - GAP;
        for pt in samples {
            if pt.0 >= role_x - GAP && pt.0 <= role_x + r_dim.width + GAP {
                role_y = role_y.min(pt.1 - r_dim.height - GAP);
            }
        }
    }
    (role_x, role_y)
}

#[allow(clippy::too_many_arguments)]
fn search_free_position(
    q_dim: Dimension,
    r_dim: Dimension,
    quantifier_pos: Point,
    endpoint: Point,
    dir_x: f64,
    dir_y: f64,
    samples: &[Point],
    forbidden: &[RectangleArea],
) -> Option<Point> {
    let length = dir_x.hypot(dir_y);
    let tx = dir_x / length;
    let ty = dir_y / length;
    let qx = quantifier_pos.0 + q_dim.width / 2.0;
    let qy = quantifier_pos.1 + q_dim.height / 2.0;
    let along = (qx - endpoint.0) * tx + (qy - endpoint.1) * ty;
    let ideal_x = 2.0 * (endpoint.0 + tx * along) - qx;
    let ideal_y = 2.0 * (endpoint.1 + ty * along) - qy;
    let mut best: Option<Point> = None;
    let mut best_distance = f64::MAX;
    let mut x = endpoint.0 - r_dim.width - SEARCH_RANGE;
    while x <= endpoint.0 + SEARCH_RANGE {
        let mut y = endpoint.1 - r_dim.height - SEARCH_RANGE;
        while y <= endpoint.1 + SEARCH_RANGE {
            let delta_x = x + r_dim.width / 2.0 - ideal_x;
            let delta_y = y + r_dim.height / 2.0 - ideal_y;
            let dist =
                (delta_x * tx + delta_y * ty).abs() + 2.0 * (-delta_x * ty + delta_y * tx).abs();
            if dist < best_distance && is_free((x, y), r_dim, forbidden, Some(samples)) {
                best = Some((x, y));
                best_distance = dist;
            }
            y += SEARCH_STEP;
        }
        x += SEARCH_STEP;
    }
    best
}

fn is_free(
    pos: Point,
    dim: Dimension,
    forbidden: &[RectangleArea],
    samples: Option<&[Point]>,
) -> bool {
    let (x1, y1) = pos;
    let x2 = x1 + dim.width;
    let y2 = y1 + dim.height;
    for a in forbidden {
        if x1 < a.max_x && x2 > a.min_x && y1 < a.max_y && y2 > a.min_y {
            return false;
        }
    }
    if let Some(samples) = samples {
        for pt in samples {
            if pt.0 >= x1 - GAP && pt.0 <= x2 + GAP && pt.1 >= y1 - GAP && pt.1 <= y2 + GAP {
                return false;
            }
        }
    }
    true
}

fn point_away_from(endpoint: Point, other: Point, samples: &[Point]) -> Point {
    let mut result = other;
    let mut best = f64::MAX;
    for pt in samples {
        let d = distance(*pt, endpoint);
        if d >= MIN_DISTANCE_FOR_DIRECTION && d < best {
            best = d;
            result = *pt;
        }
    }
    result
}

fn sample_segment(p1: Point, p2: Point) -> Vec<Point> {
    let steps = ((distance(p1, p2) / 2.0).ceil() as i64).max(1);
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            (p1.0 + (p2.0 - p1.0) * t, p1.1 + (p2.1 - p1.1) * t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_to_other_side() {
        let q = Dimension::new(10.0, 10.0);
        let r = Dimension::new(20.0, 10.0);
        let p = position(q, r, (-12.0, 5.0), (0.0, 0.0), (0.0, 100.0), None, &[]);
        assert!(p.0 >= 0.0);
        assert_eq!(p.1, 5.0);
    }

    #[test]
    fn degenerate_direction() {
        let q = Dimension::new(10.0, 10.0);
        let r = Dimension::new(20.0, 10.0);
        let p = position(q, r, (1.0, 2.0), (0.0, 0.0), (0.0, 0.0), None, &[]);
        assert_eq!(p, (1.0, 12.0));
    }

    #[test]
    fn search_around_obstacle() {
        let q = Dimension::new(10.0, 10.0);
        let r = Dimension::new(20.0, 10.0);
        let obstacle = RectangleArea::new(0.0, 0.0, 40.0, 40.0);
        let p = position(
            q,
            r,
            (-12.0, 5.0),
            (0.0, 0.0),
            (0.0, 100.0),
            None,
            &[obstacle],
        );
        assert!(p.0 < 0.0 || p.0 >= 40.0 || p.1 >= 40.0 || p.1 + 10.0 <= 0.0);
    }
}
