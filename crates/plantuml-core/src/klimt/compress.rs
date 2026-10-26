//! Squeezing empty bands out of a drawing (`klimt/compress`: `SlotSet`, `SlotFinder`,
//! `CompressionTransform`, `UGraphicCompressOnXorY`, `CompressionXorYBuilder`).
//!
//! The drawing is run once into a [`SlotRecorder`] to find which bands of an axis are covered by
//! shapes; every gap between them is then shortened by 10 (5 on each side), and a second run goes
//! through a [`CompressLayer`] that moves the shapes accordingly.

use crate::klimt::drawing::ugraphic::{find_slots, UGraphic, ULine, UShape, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    OnX,
    OnY,
}

/// A band `[start, end]` of one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub start: f64,
    pub end: f64,
}

impl Slot {
    pub fn new(start: f64, end: f64) -> Option<Slot> {
        (start < end).then_some(Slot { start, end })
    }

    pub fn size(&self) -> f64 {
        self.end - self.start
    }

    fn contains(&self, v: f64) -> bool {
        v >= self.start && v <= self.end
    }

    fn intersects(&self, o: &Slot) -> bool {
        self.contains(o.start)
            || self.contains(o.end)
            || o.contains(self.start)
            || o.contains(self.end)
    }

    fn merge(&self, o: &Slot) -> Slot {
        Slot {
            start: self.start.min(o.start),
            end: self.end.max(o.end),
        }
    }
}

/// Disjoint bands (`SlotSet`).
#[derive(Debug, Clone, Default)]
pub struct SlotSet {
    all: Vec<Slot>,
}

impl SlotSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn slots(&self) -> &[Slot] {
        &self.all
    }

    /// Adds a band, merging it with those it touches; empty bands are ignored.
    pub fn add_slot(&mut self, start: f64, end: f64) {
        let Some(mut new_slot) = Slot::new(start, end) else {
            return;
        };
        let mut rest = Vec::with_capacity(self.all.len() + 1);
        for s in self.all.drain(..) {
            if s.intersects(&new_slot) {
                new_slot = new_slot.merge(&s);
            } else {
                rest.push(s);
            }
        }
        rest.push(new_slot);
        self.all = rest;
    }

    /// The gaps between the bands.
    pub fn reverse(&self) -> SlotSet {
        let mut sorted = self.all.clone();
        sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
        let mut result = SlotSet::new();
        let mut last: Option<Slot> = None;
        for s in sorted {
            if let Some(l) = last {
                result.add_slot(l.end, s.start);
            }
            last = Some(s);
        }
        result
    }

    /// Bands shrunk by `margin` on both sides; those not larger than twice the margin vanish.
    pub fn smaller(&self, margin: f64) -> SlotSet {
        let mut result = SlotSet::new();
        for s in &self.all {
            if s.size() <= 2.0 * margin {
                continue;
            }
            result.add_slot(s.start + margin, s.end - margin);
        }
        result
    }
}

/// Maps a coordinate to the compressed one (`CompressionTransform`).
#[derive(Debug, Clone)]
pub struct CompressionTransform {
    all: Vec<Slot>,
}

impl CompressionTransform {
    pub fn new(set: &SlotSet) -> Self {
        Self {
            all: set.slots().to_vec(),
        }
    }

    pub fn transform(&self, v: f64) -> f64 {
        v - self.delta(v)
    }

    fn delta(&self, v: f64) -> f64 {
        let mut result = 0.0;
        for s in &self.all {
            if s.start > v {
                continue;
            }
            if v > s.end {
                result += s.size();
            } else {
                result += v - s.start;
            }
        }
        result
    }
}

/// Records which bands of an axis are covered by shapes (`SlotFinder`).
pub struct SlotRecorder {
    mode: CompressionMode,
    origin: UTranslate,
    slots: RefCell<SlotSet>,
}

impl SlotRecorder {
    pub fn new(mode: CompressionMode, origin: UTranslate) -> Self {
        Self {
            mode,
            origin,
            slots: RefCell::new(SlotSet::new()),
        }
    }

    pub fn slots(&self) -> SlotSet {
        self.slots.borrow().clone()
    }

    fn add(&self, x1: f64, x2: f64, y1: f64, y2: f64) {
        let (ox, oy) = (self.origin.dx, self.origin.dy);
        match self.mode {
            CompressionMode::OnX => self.slots.borrow_mut().add_slot(x1 - ox, x2 - ox),
            CompressionMode::OnY => self.slots.borrow_mut().add_slot(y1 - oy, y2 - oy),
        }
    }

    pub fn record(&self, sb: &dyn StringBounder, x: f64, y: f64, shape: &UShape) {
        match shape {
            UShape::Rectangle(r) => self.add(x, x + r.width, y, y + r.height),
            UShape::Path(p) => {
                if let Some((x1, y1, x2, y2)) = p.min_max() {
                    self.add(x + x1, x + x2, y + y1, y + y2);
                }
            }
            UShape::Polygon(p) => {
                if let Some((x1, y1, x2, y2)) = p.min_max() {
                    self.add(x + x1, x + x2, y + y1, y + y2);
                }
            }
            UShape::Ellipse(e) => self.add(x, x + e.width, y, y + e.height),
            UShape::Text(t) => {
                let dim = sb.calculate_dimension(&t.font.font(), &t.text);
                let top = y - (dim.height - 1.5);
                self.add(x, x + dim.width, top, top + dim.height);
            }
            UShape::Empty(w, h) => self.add(x, x + w, y, y + h),
            _ => {}
        }
    }
}

/// Moves the shapes drawn through it along one axis (`UGraphicCompressOnXorY`).
pub struct CompressLayer {
    mode: CompressionMode,
    origin: UTranslate,
    transform: Rc<CompressionTransform>,
    parent: Option<Rc<CompressLayer>>,
}

impl CompressLayer {
    pub fn new(
        mode: CompressionMode,
        origin: UTranslate,
        transform: Rc<CompressionTransform>,
        parent: Option<Rc<CompressLayer>>,
    ) -> Self {
        Self {
            mode,
            origin,
            transform,
            parent,
        }
    }

    /// New position and shape for what is drawn at `(x, y)`.
    pub fn apply(&self, x: f64, y: f64, shape: UShape) -> (f64, f64, UShape) {
        let (ox, oy) = (self.origin.dx, self.origin.dy);
        let ct = |v: f64| self.transform.transform(v);
        let (mut x, mut y, mut shape) = (x, y, shape);
        let on_x = self.mode == CompressionMode::OnX;
        match &mut shape {
            UShape::Rectangle(r) => {
                if on_x {
                    r.width = ct(x - ox + r.width) - ct(x - ox);
                } else {
                    r.height = ct(y - oy + r.height) - ct(y - oy);
                }
            }
            UShape::Line(l) => {
                let (mut x1, mut y1, mut x2, mut y2) = if on_x {
                    (ox + ct(x - ox), y, ox + ct(x - ox + l.dx), y + l.dy)
                } else {
                    (x, oy + ct(y - oy), x + l.dx, oy + ct(y - oy + l.dy))
                };
                if y1 > y2 {
                    std::mem::swap(&mut x1, &mut x2);
                    std::mem::swap(&mut y1, &mut y2);
                }
                *l = ULine::new(x2 - x1, y2 - y1);
                x = x1;
                y = y1;
                return self.parent_apply(x, y, shape);
            }
            _ => {}
        }
        if on_x {
            x = ox + ct(x - ox);
        } else {
            y = oy + ct(y - oy);
        }
        self.parent_apply(x, y, shape)
    }

    fn parent_apply(&self, x: f64, y: f64, shape: UShape) -> (f64, f64, UShape) {
        match &self.parent {
            Some(p) => p.apply(x, y, shape),
            None => (x, y, shape),
        }
    }
}

/// Compresses the empty bands of `inner` along one axis (`CompressionXorYBuilder`).
pub struct CompressionXorYBuilder {
    mode: CompressionMode,
    inner: Rc<dyn TextBlock>,
    transform: RefCell<Option<Rc<CompressionTransform>>>,
}

impl CompressionXorYBuilder {
    pub fn build(mode: CompressionMode, inner: Rc<dyn TextBlock>) -> Rc<dyn TextBlock> {
        Rc::new(Self {
            mode,
            inner,
            transform: RefCell::new(None),
        })
    }

    fn transform(&self, sb: &dyn StringBounder) -> Rc<CompressionTransform> {
        if let Some(t) = self.transform.borrow().as_ref() {
            return t.clone();
        }
        let slots = find_slots(sb.to_rc(), self.mode, |ug| self.inner.draw(ug));
        let t = Rc::new(CompressionTransform::new(&slots.reverse().smaller(5.0)));
        *self.transform.borrow_mut() = Some(t.clone());
        t
    }
}

impl TextBlock for CompressionXorYBuilder {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let t = self.transform(sb);
        let dim = self.inner.calculate_dimension(sb);
        match self.mode {
            CompressionMode::OnX => Dimension::new(t.transform(dim.width), dim.height),
            CompressionMode::OnY => Dimension::new(dim.width, t.transform(dim.height)),
        }
    }

    fn draw(&self, ug: &UGraphic) {
        let t = self.transform(ug.string_bounder());
        self.inner.draw(&ug.apply_compression(self.mode, t));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_merge_and_invert() {
        let mut s = SlotSet::new();
        s.add_slot(0.0, 10.0);
        s.add_slot(40.0, 50.0);
        s.add_slot(5.0, 12.0);
        s.add_slot(3.0, 3.0);
        assert_eq!(s.slots().len(), 2);
        let gaps = s.reverse();
        assert_eq!(
            gaps.slots(),
            &[Slot {
                start: 12.0,
                end: 40.0
            }]
        );
        let small = gaps.smaller(5.0);
        assert_eq!(
            small.slots(),
            &[Slot {
                start: 17.0,
                end: 35.0
            }]
        );
        assert!(SlotSet::new().reverse().slots().is_empty());
        let tiny = SlotSet {
            all: vec![Slot {
                start: 0.0,
                end: 10.0,
            }],
        };
        assert!(tiny.smaller(5.0).slots().is_empty());
    }

    #[test]
    fn transform_shortens_gaps() {
        let mut s = SlotSet::new();
        s.add_slot(17.0, 35.0);
        let t = CompressionTransform::new(&s);
        assert_eq!(t.transform(10.0), 10.0);
        assert_eq!(t.transform(20.0), 17.0);
        assert_eq!(t.transform(50.0), 32.0);
    }

    #[test]
    fn layer_moves_rectangles_and_lines() {
        let mut s = SlotSet::new();
        s.add_slot(10.0, 20.0);
        let t = Rc::new(CompressionTransform::new(&s));
        let x = CompressLayer::new(CompressionMode::OnX, UTranslate::default(), t.clone(), None);
        let (nx, ny, shape) = x.apply(
            30.0,
            4.0,
            UShape::Rectangle(crate::klimt::drawing::ugraphic::URectangle::build(
                10.0, 5.0,
            )),
        );
        assert_eq!((nx, ny), (20.0, 4.0));
        assert!(matches!(shape, UShape::Rectangle(r) if r.width == 10.0));
        let y = CompressLayer::new(CompressionMode::OnY, UTranslate::default(), t, None);
        let (_, ny, shape) = y.apply(0.0, 30.0, UShape::Line(ULine::new(0.0, -15.0)));
        assert_eq!(ny, 10.0);
        assert!(matches!(shape, UShape::Line(l) if l.dy == 10.0));
        let (_, _, shape) = y.apply(
            0.0,
            5.0,
            UShape::Rectangle(crate::klimt::drawing::ugraphic::URectangle::build(
                1.0, 40.0,
            )),
        );
        assert!(matches!(shape, UShape::Rectangle(r) if r.height == 30.0));
    }
}
