//! Tiles of the activity diagram (`activitydiagram3/ftile`).
//!
//! An [`Ftile`] is a laid out piece of the flow with one entry point and at most one exit point.
//! Tiles are built by an [`factory::FtileFactory`] chain from the instructions and composed
//! by wrapper tiles ([`wrap`]); arrows between tiles are [`Connection`]s drawn as [`snake::Snake`]s.

pub mod box_style;
pub mod factory;
pub mod group;
pub mod leaf;
pub mod snake;
pub mod wrap;

use super::rainbow::LinkRendering;
use super::swimlanes::Swimlane;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::skin::SkinParam;
use snake::Snake;
use std::cell::RefCell;
use std::rc::Rc;

/// Marks a geometry without exit point (`Double.MIN_NORMAL`).
const NO_OUT: f64 = f64::MIN_POSITIVE;

/// Size of a tile and where the flow enters and leaves it (`FtileGeometry`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FtileGeometry {
    pub width: f64,
    pub height: f64,
    left: f64,
    in_y: f64,
    out_y: f64,
}

impl FtileGeometry {
    /// A geometry without exit point.
    pub fn new(width: f64, height: f64, left: f64, in_y: f64) -> Self {
        Self::with_out(width, height, left, in_y, NO_OUT)
    }

    pub fn with_out(width: f64, height: f64, left: f64, in_y: f64, out_y: f64) -> Self {
        Self {
            width,
            height,
            left,
            in_y,
            out_y,
        }
    }

    pub fn left(&self) -> f64 {
        self.left
    }

    pub fn right(&self) -> f64 {
        self.width - self.left
    }

    pub fn in_y(&self) -> f64 {
        self.in_y
    }

    pub fn out_y(&self) -> f64 {
        self.out_y
    }

    pub fn has_point_out(&self) -> bool {
        self.out_y != NO_OUT
    }

    pub fn point_in(&self) -> (f64, f64) {
        (self.left, self.in_y)
    }

    pub fn point_out(&self) -> (f64, f64) {
        assert!(self.has_point_out(), "no exit point");
        (self.left, self.out_y)
    }

    pub fn translate(&self, t: UTranslate) -> Self {
        if self.has_point_out() {
            Self::with_out(
                self.width,
                self.height,
                self.left + t.dx,
                self.in_y + t.dy,
                self.out_y + t.dy,
            )
        } else {
            Self::new(self.width, self.height, self.left + t.dx, self.in_y + t.dy)
        }
    }

    pub fn inc_vertically(&self, missing1: f64, missing2: f64) -> Self {
        Self::with_out(
            self.width,
            self.height + missing1 + missing2,
            self.left,
            self.in_y + missing1,
            if self.has_point_out() {
                self.out_y + missing1
            } else {
                self.out_y
            },
        )
    }

    pub fn add_top(&self, north: f64) -> Self {
        let out = if self.has_point_out() {
            self.out_y + north
        } else {
            NO_OUT
        };
        Self::with_out(
            self.width,
            self.height + north,
            self.left,
            self.in_y + north,
            out,
        )
    }

    pub fn add_bottom(&self, south: f64) -> Self {
        Self::with_out(
            self.width,
            self.height + south,
            self.left,
            self.in_y,
            self.out_y,
        )
    }

    pub fn add_margin_x(&self, margin: f64) -> Self {
        Self::with_out(
            self.width + 2.0 * margin,
            self.height,
            self.left + margin,
            self.in_y,
            self.out_y,
        )
    }

    pub fn fixed_height(&self, h: f64) -> Self {
        Self::with_out(self.width, h, self.left, self.in_y, self.out_y)
    }

    pub fn without_point_out(&self) -> Self {
        Self::new(self.width, self.height, self.left, self.in_y)
    }

    /// Stacks `other` under this geometry (`FtileGeometryMerger`).
    pub fn append_bottom(&self, other: &FtileGeometry) -> Self {
        let left = self.left.max(other.left);
        let dx1 = left - self.left;
        let dx2 = left - other.left;
        let width = (self.width + dx1).max(other.width + dx2);
        let height = self.height + other.height;
        if other.has_point_out() {
            Self::with_out(width, height, left, self.in_y, other.out_y + self.height)
        } else {
            Self::new(width, height, left, self.in_y)
        }
    }
}

/// Identity of a tile, for child lookups.
fn addr(f: &dyn Ftile) -> *const () {
    (f as *const dyn Ftile).cast()
}

/// Whether both references are the same tile.
pub fn same_ftile(a: &dyn Ftile, b: &dyn Ftile) -> bool {
    addr(a) == addr(b)
}

/// A laid out part of the flow (`Ftile`).
///
/// Wrapper tiles return the tile they decorate from [`Ftile::decorated`] and inherit the
/// delegating defaults (`FtileDecorate`); leaves override the geometry, swimlanes and `draw`.
pub trait Ftile {
    fn skin(&self) -> Rc<SkinParam>;

    /// The tile a decorator wraps.
    fn decorated(&self) -> Option<Rc<dyn Ftile>> {
        None
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        self.decorated()
            .expect("a tile without geometry")
            .calculate_dimension(sb)
    }

    fn draw(&self, g: &FtileGraphic) {
        if let Some(inner) = self.decorated() {
            g.draw_ftile(inner.as_ref());
        }
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.decorated()
            .map_or_else(LinkRendering::none, |d| d.in_link_rendering())
    }

    fn out_link_rendering(&self) -> LinkRendering {
        self.decorated()
            .map_or_else(LinkRendering::none, |d| d.out_link_rendering())
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.decorated().and_then(|d| d.swimlane_in())
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.decorated().and_then(|d| d.swimlane_out())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        self.decorated().map_or_else(Vec::new, |d| d.swimlanes())
    }

    /// Where `child` is drawn inside this tile; `None` when it is not a descendant.
    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let inner = self.decorated()?;
        if same_ftile(child, inner.as_ref()) {
            return Some(UTranslate::default());
        }
        inner.translate_for(child, sb)
    }

    fn my_children(&self) -> Vec<Rc<dyn Ftile>> {
        self.decorated().into_iter().collect()
    }

    fn inner_connections(&self) -> Vec<Rc<dyn Connection>> {
        self.decorated()
            .map_or_else(Vec::new, |d| d.inner_connections())
    }

    fn welding_points(&self) -> Vec<WeldingPoint> {
        self.decorated()
            .map_or_else(Vec::new, |d| d.welding_points())
    }

    fn arrow_horizontal_alignment(&self) -> HorizontalAlignment {
        match self.decorated() {
            Some(d) => d.arrow_horizontal_alignment(),
            None => HorizontalAlignment::from_string_or(
                self.skin().get_value("arrowmessagealignment"),
                HorizontalAlignment::Left,
            ),
        }
    }
}

/// Point where a `break` leaves a loop (`WeldingPoint`); filled in with loops.
#[derive(Debug, Clone, PartialEq)]
pub struct WeldingPoint;

/// An arrow between two tiles (`Connection`).
pub trait Connection {
    fn ftile1(&self) -> Option<Rc<dyn Ftile>>;
    fn ftile2(&self) -> Option<Rc<dyn Ftile>>;
    fn draw(&self, g: &FtileGraphic);

    /// Connections that can be redrawn between two moved positions (`ConnectionTranslatable`).
    fn draw_translate(&self, _g: &FtileGraphic, _t1: UTranslate, _t2: UTranslate) {}
}

struct PendingSnake {
    snake: Snake,
    ug: UGraphic,
    dx: f64,
    dy: f64,
}

impl PendingSnake {
    fn moved(&self) -> Snake {
        self.snake.moved(self.dx, self.dy)
    }

    fn touches_other(&self, other: &PendingSnake) -> bool {
        if other.snake.cannot_be_touched() {
            return false;
        }
        let last = self.snake.last();
        let first = other.snake.first();
        snake::points_same(
            (last.0 + self.dx, last.1 + self.dy),
            (first.0 + other.dx, first.1 + other.dy),
        )
    }

    fn merge(&self, new_item: &PendingSnake) -> Option<PendingSnake> {
        let merged = self
            .moved()
            .merge(&new_item.moved(), self.ug.string_bounder())?;
        Some(PendingSnake {
            snake: merged.moved(-self.dx, -self.dy),
            ug: self.ug.clone(),
            dx: self.dx,
            dy: self.dy,
        })
    }
}

/// Drawing context of tiles: a [`UGraphic`] that also collects arrows and merges them before
/// they are drawn (`UGraphicForSnake` and `UGraphicDispatchFtile`).
#[derive(Clone)]
pub struct FtileGraphic {
    ug: UGraphic,
    snakes: Rc<RefCell<Vec<PendingSnake>>>,
    dx: f64,
    dy: f64,
}

impl FtileGraphic {
    pub fn new(ug: UGraphic) -> Self {
        Self {
            ug,
            snakes: Rc::new(RefCell::new(Vec::new())),
            dx: 0.0,
            dy: 0.0,
        }
    }

    /// The plain drawing context, at the current position.
    pub fn ug(&self) -> &UGraphic {
        &self.ug
    }

    pub fn string_bounder(&self) -> &dyn StringBounder {
        self.ug.string_bounder()
    }

    pub fn apply_translate(&self, t: UTranslate) -> Self {
        Self {
            ug: self.ug.apply_translate(t),
            snakes: self.snakes.clone(),
            dx: self.dx + t.dx,
            dy: self.dy + t.dy,
        }
    }

    pub fn translation(&self) -> UTranslate {
        UTranslate::new(self.dx, self.dy)
    }

    pub fn draw_ftile(&self, ftile: &dyn Ftile) {
        ftile.draw(self);
    }

    pub fn draw_connection(&self, connection: &dyn Connection) {
        connection.draw(self);
    }

    /// Queues an arrow, merged with a pending one when they touch (`addPendingSnake`).
    pub fn draw_snake(&self, snake: Snake) {
        let item = PendingSnake {
            snake,
            ug: self.ug.clone(),
            dx: self.dx,
            dy: self.dy,
        };
        let mut snakes = self.snakes.borrow_mut();
        for pos in 0..snakes.len() {
            if let Some(merged) = snakes[pos].merge(&item) {
                snakes[pos] = merged;
                return;
            }
        }
        snakes.push(item);
    }

    /// Draws the queued arrows (`flushUg`).
    pub fn flush(&self) {
        let snakes = std::mem::take(&mut *self.snakes.borrow_mut());
        for s in &snakes {
            let touched = snakes.iter().any(|o| s.touches_other(o));
            let snake = if touched {
                s.snake.clone().without_end_decoration()
            } else {
                s.snake.clone()
            };
            snake.draw_internal(&s.ug);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_arithmetic() {
        let g = FtileGeometry::with_out(40.0, 30.0, 20.0, 0.0, 30.0);
        assert!(g.has_point_out());
        assert_eq!(g.point_in(), (20.0, 0.0));
        assert_eq!(g.point_out(), (20.0, 30.0));
        assert_eq!(g.right(), 20.0);
        let t = g.translate(UTranslate::new(5.0, 7.0));
        assert_eq!(t.point_out(), (25.0, 37.0));
        let v = g.inc_vertically(2.0, 3.0);
        assert_eq!((v.height, v.in_y(), v.out_y()), (35.0, 2.0, 32.0));
        assert_eq!(g.add_bottom(4.0).height, 34.0);
        assert_eq!(g.add_top(4.0).out_y(), 34.0);
        assert_eq!(g.add_margin_x(3.0).left(), 23.0);
        assert_eq!(g.fixed_height(1.0).height, 1.0);
        let stop = FtileGeometry::new(10.0, 10.0, 5.0, 0.0);
        assert!(!stop.has_point_out());
        assert!(!stop.add_top(1.0).has_point_out());
        assert!(!g.without_point_out().has_point_out());
        let s = stop.translate(UTranslate::dx(1.0));
        assert_eq!(s.left(), 6.0);
        let both = g.append_bottom(&stop);
        assert_eq!((both.width, both.height, both.left()), (40.0, 40.0, 20.0));
        assert!(!both.has_point_out());
        let both = stop.append_bottom(&g);
        assert_eq!(both.left(), 20.0);
        assert_eq!(both.out_y(), 40.0);
    }
}
