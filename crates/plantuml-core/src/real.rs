//! Constraint solver for positions (`real` package): values that must keep minimum
//! distances from each other, relaxed by repeatedly pushing points apart.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

enum Node {
    Impl(f64),
    Delta {
        base: usize,
        diff: f64,
    },
    DeltaLive {
        base: usize,
        offset: Rc<dyn Fn() -> f64>,
    },
    Middle2(usize, usize),
    Max {
        all: Vec<usize>,
        cache: Option<f64>,
    },
    Min {
        all: Vec<usize>,
        cache: Option<f64>,
    },
    AbsMax,
    AbsMin,
}

struct Force {
    fixed: usize,
    moving: usize,
    minimum_distance: f64,
}

/// The nodes and forces of one axis (`RealLine`).
#[derive(Default)]
struct Line {
    nodes: RefCell<Vec<Node>>,
    forces: RefCell<Vec<Force>>,
    min: Cell<f64>,
    max: Cell<f64>,
}

/// A position on an axis, possibly derived from others (`Real`).
#[derive(Clone)]
pub struct Real {
    line: Rc<Line>,
    id: usize,
}

impl std::fmt::Debug for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Real#{}", self.id)
    }
}

/// Creates a fresh axis whose origin is fixed at 0 (`RealUtils.createOrigin`).
pub fn create_origin() -> Real {
    let line = Rc::new(Line::default());
    line.nodes.borrow_mut().push(Node::Impl(0.0));
    Real { line, id: 0 }
}

impl Line {
    fn push(&self, n: Node) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(n);
        nodes.len() - 1
    }

    fn value(&self, id: usize) -> f64 {
        enum Plan {
            Done(f64),
            Delta(usize, f64),
            Live(usize, Rc<dyn Fn() -> f64>),
            Mid(usize, usize),
            Max(Vec<usize>),
            Min(Vec<usize>),
        }
        let plan = match &self.nodes.borrow()[id] {
            Node::Impl(v) => Plan::Done(*v),
            Node::Delta { base, diff } => Plan::Delta(*base, *diff),
            Node::DeltaLive { base, offset } => Plan::Live(*base, offset.clone()),
            Node::Middle2(a, b) => Plan::Mid(*a, *b),
            Node::Max { all, cache } => match cache {
                Some(c) => Plan::Done(*c),
                None => Plan::Max(all.clone()),
            },
            Node::Min { all, cache } => match cache {
                Some(c) => Plan::Done(*c),
                None => Plan::Min(all.clone()),
            },
            Node::AbsMax => Plan::Done(self.max.get()),
            Node::AbsMin => Plan::Done(self.min.get()),
        };
        match plan {
            Plan::Done(v) => v,
            Plan::Delta(base, diff) => self.value(base) + diff,
            Plan::Live(base, offset) => self.value(base) + offset(),
            Plan::Mid(a, b) => (self.value(a) + self.value(b)) / 2.0,
            Plan::Max(all) => {
                let v = all
                    .iter()
                    .map(|i| self.value(*i))
                    .fold(f64::NEG_INFINITY, f64::max);
                if let Node::Max { cache, .. } = &mut self.nodes.borrow_mut()[id] {
                    *cache = Some(v);
                }
                v
            }
            Plan::Min(all) => {
                let v = all
                    .iter()
                    .map(|i| self.value(*i))
                    .fold(f64::INFINITY, f64::min);
                if let Node::Min { cache, .. } = &mut self.nodes.borrow_mut()[id] {
                    *cache = Some(v);
                }
                v
            }
        }
    }

    fn move_by(&self, id: usize, delta: f64) {
        let target = match &mut self.nodes.borrow_mut()[id] {
            Node::Impl(v) => {
                *v += delta;
                return;
            }
            Node::Delta { base, .. } | Node::DeltaLive { base, .. } => Some((*base, None)),
            Node::Middle2(a, b) => Some((*a, Some(*b))),
            _ => panic!("this position cannot be moved"),
        };
        if let Some((a, b)) = target {
            match b {
                None => self.move_by(a, delta),
                Some(b) => {
                    self.move_by(a, delta / 2.0);
                    self.move_by(b, delta / 2.0);
                }
            }
        }
    }

    fn apply(&self, f: usize) -> bool {
        let (fixed, moving, min) = {
            let forces = self.forces.borrow();
            (
                forces[f].fixed,
                forces[f].moving,
                forces[f].minimum_distance,
            )
        };
        let moving_value = self.value(moving);
        let fixed_value = self.value(fixed);
        let diff = (moving_value - fixed_value) - min;
        let ulp = |x: f64| {
            if x == 0.0 {
                f64::from_bits(1)
            } else {
                let b = x.abs().to_bits();
                f64::from_bits(b + 1) - x.abs()
            }
        };
        let epsilon = 0.000001f64.max(1000.0 * ulp(moving_value.abs().max(fixed_value.abs())));
        if diff >= -epsilon {
            return false;
        }
        self.move_by(moving, -diff);
        true
    }

    fn compile(&self) {
        let mut count = 0;
        loop {
            let mut done = true;
            let n = self.forces.borrow().len();
            for f in 0..n {
                if self.apply(f) {
                    done = false;
                }
            }
            if done {
                self.min.set(0.0);
                self.max.set(0.0);
                let n = self.nodes.borrow().len();
                for id in 0..n {
                    if matches!(self.nodes.borrow()[id], Node::AbsMax | Node::AbsMin) {
                        continue;
                    }
                    let v = self.value(id);
                    if v > self.max.get() {
                        self.max.set(v);
                    }
                    if v < self.min.get() {
                        self.min.set(v);
                    }
                }
                return;
            }
            count += 1;
            assert!(count <= 99999, "Infinite Loop?");
        }
    }
}

impl Real {
    fn derive(&self, n: Node) -> Real {
        Real {
            line: self.line.clone(),
            id: self.line.push(n),
        }
    }

    fn same_line(&self, other: &Real) {
        assert!(Rc::ptr_eq(&self.line, &other.line), "different axes");
    }

    /// Current value (`getCurrentValue`).
    pub fn value(&self) -> f64 {
        self.line.value(self.id)
    }

    /// A point exactly `delta` further (`addFixed`).
    pub fn add_fixed(&self, delta: f64) -> Real {
        self.derive(Node::Delta {
            base: self.id,
            diff: delta,
        })
    }

    /// A new point at least `delta` further (`addAtLeast`).
    pub fn add_at_least(&self, delta: f64) -> Real {
        enum Plan {
            Impl(f64),
            Delta(usize, f64),
            Live(usize, Rc<dyn Fn() -> f64>),
        }
        let plan = match &self.line.nodes.borrow()[self.id] {
            Node::Impl(v) => Plan::Impl(*v),
            Node::Delta { base, diff } => Plan::Delta(*base, *diff),
            Node::DeltaLive { base, offset } => Plan::Live(*base, offset.clone()),
            _ => panic!("add_at_least is not supported here"),
        };
        match plan {
            Plan::Impl(v) => {
                let result = self.derive(Node::Impl(v + delta));
                self.line.forces.borrow_mut().push(Force {
                    fixed: self.id,
                    moving: result.id,
                    minimum_distance: delta,
                });
                result
            }
            Plan::Delta(base, diff) => {
                let inner = Real {
                    line: self.line.clone(),
                    id: base,
                }
                .add_at_least(delta);
                self.derive(Node::Delta {
                    base: inner.id,
                    diff,
                })
            }
            Plan::Live(base, offset) => {
                let inner = Real {
                    line: self.line.clone(),
                    id: base,
                }
                .add_at_least(delta);
                self.derive(Node::DeltaLive {
                    base: inner.id,
                    offset,
                })
            }
        }
    }

    /// Forces this point to be at or after `other` (`ensureBiggerThan`).
    pub fn ensure_bigger_than(&self, other: &Real) {
        self.same_line(other);
        enum Plan {
            Force,
            Delta(usize, f64),
            Live(usize, Rc<dyn Fn() -> f64>),
            Min(Vec<usize>),
        }
        let plan = match &self.line.nodes.borrow()[self.id] {
            Node::Impl(_) | Node::Middle2(..) => Plan::Force,
            Node::Delta { base, diff } => Plan::Delta(*base, *diff),
            Node::DeltaLive { base, offset } => Plan::Live(*base, offset.clone()),
            Node::Min { all, .. } => Plan::Min(all.clone()),
            _ => panic!("ensure_bigger_than is not supported here"),
        };
        let at = |id| Real {
            line: self.line.clone(),
            id,
        };
        match plan {
            Plan::Force => self.line.forces.borrow_mut().push(Force {
                fixed: other.id,
                moving: self.id,
                minimum_distance: 0.0,
            }),
            Plan::Delta(base, diff) => at(base).ensure_bigger_than(&other.add_fixed(-diff)),
            Plan::Live(base, offset) => {
                let neg: Rc<dyn Fn() -> f64> = Rc::new(move || -offset());
                let shifted = other.with_live_offset(neg);
                at(base).ensure_bigger_than(&shifted);
            }
            Plan::Min(all) => {
                for id in all {
                    at(id).ensure_bigger_than(other);
                }
            }
        }
    }

    /// Adds an offset recomputed on every read (`RealUtils.withLiveOffset`).
    pub fn with_live_offset(&self, offset: Rc<dyn Fn() -> f64>) -> Real {
        self.derive(Node::DeltaLive {
            base: self.id,
            offset,
        })
    }

    /// Solves all constraints of the axis (`RealOrigin.compileNow`).
    pub fn compile_now(&self) {
        self.line.compile();
    }

    /// Largest value of the axis after compilation.
    pub fn max_absolute(&self) -> Real {
        self.derive(Node::AbsMax)
    }

    /// Smallest value of the axis after compilation.
    pub fn min_absolute(&self) -> Real {
        self.derive(Node::AbsMin)
    }
}

/// Middle of two movable points (`RealUtils.middle`).
pub fn middle(a: &Real, b: &Real) -> Real {
    a.same_line(b);
    a.derive(Node::Middle2(a.id, b.id))
}

/// Largest of several points (`RealUtils.max`).
pub fn max(reals: &[Real]) -> Real {
    let first = &reals[0];
    first.derive(Node::Max {
        all: reals.iter().map(|r| r.id).collect(),
        cache: None,
    })
}

/// Smallest of several points (`RealUtils.min`).
pub fn min(reals: &[Real]) -> Real {
    let first = &reals[0];
    first.derive(Node::Min {
        all: reals.iter().map(|r| r.id).collect(),
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_of_minimum_distances() {
        let o = create_origin();
        let a = o.add_at_least(10.0);
        let b = a.add_at_least(5.0);
        let c = o.add_at_least(30.0);
        b.ensure_bigger_than(&c);
        o.compile_now();
        assert_eq!(a.value(), 10.0);
        assert_eq!(b.value(), 30.0);
        assert_eq!(c.value(), 30.0);
    }

    #[test]
    fn forces_push_apart_not_together() {
        let o = create_origin();
        let a = o.add_at_least(0.0);
        let b = a.add_at_least(50.0);
        let c = b.add_at_least(0.0);
        o.compile_now();
        assert_eq!((a.value(), b.value(), c.value()), (0.0, 50.0, 50.0));
        assert_eq!(o.max_absolute().value(), 50.0);
        assert_eq!(o.min_absolute().value(), 0.0);
    }

    #[test]
    fn delta_and_live_offsets() {
        let o = create_origin();
        let a = o.add_at_least(10.0);
        let d = a.add_fixed(3.0);
        let e = d.add_at_least(7.0);
        let live = e.with_live_offset(Rc::new(|| 100.0));
        let bump = o.add_at_least(25.0);
        e.ensure_bigger_than(&bump);
        live.ensure_bigger_than(&o.add_fixed(0.0));
        o.compile_now();
        assert_eq!(a.value(), 10.0);
        assert_eq!(d.value(), 13.0);
        assert_eq!(e.value(), 25.0);
        assert_eq!(live.value(), 125.0);
        let again = live.add_at_least(1.0);
        o.compile_now();
        assert!(again.value() >= 126.0);
        let x = o.add_at_least(5.0);
        let y = x.add_fixed(2.0);
        let z = y.add_at_least(1.0);
        o.compile_now();
        assert!(z.value() >= y.value() + 1.0);
        let w = bump.add_fixed(1.0);
        w.ensure_bigger_than(&o.add_at_least(40.0));
        o.compile_now();
        assert!(bump.value() >= 39.0);
    }

    #[test]
    fn middle_max_min() {
        let o = create_origin();
        let a = o.add_at_least(10.0);
        let b = o.add_at_least(20.0);
        let m = middle(&a, &b);
        let mx = max(&[a.clone(), b.clone()]);
        let mn = min(&[a.clone(), b.clone()]);
        let c = o.add_at_least(0.0);
        c.ensure_bigger_than(&mx);
        o.compile_now();
        assert_eq!(m.value(), 15.0);
        assert_eq!(mx.value(), 20.0);
        assert_eq!(mn.value(), 10.0);
        assert_eq!(c.value(), 20.0);
        let lo = o.add_at_least(0.0);
        mn.ensure_bigger_than(&lo);
        o.compile_now();
        assert!(a.value() >= lo.value());
        m.ensure_bigger_than(&o.add_at_least(40.0));
        o.compile_now();
        assert!(m.value() >= 40.0 - 1e-6);
    }

    #[test]
    #[should_panic]
    fn max_cannot_be_moved() {
        let o = create_origin();
        let a = o.add_at_least(1.0);
        let m = max(&[a]);
        m.ensure_bigger_than(&o);
    }

    #[test]
    #[should_panic]
    fn mixing_axes_is_rejected() {
        let a = create_origin();
        let b = create_origin();
        a.add_at_least(1.0).ensure_bigger_than(&b);
    }
}
