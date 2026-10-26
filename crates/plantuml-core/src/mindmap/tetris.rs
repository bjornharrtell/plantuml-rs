//! Packing of the children of a finger (`StripeFrontier`, `SymetricalTee`, `Tetris`).

/// A horizontal stretch of the frontier at a given height (`Stripe`).
#[derive(Debug, Clone, Copy)]
struct Stripe {
    x1: f64,
    x2: f64,
    value: f64,
}

/// Skyline of the placed elements (`StripeFrontier`).
#[derive(Debug, Clone)]
pub struct StripeFrontier {
    /// Sorted by start, contiguous from `-MAX` to `MAX`.
    stripes: Vec<Stripe>,
}

impl Default for StripeFrontier {
    fn default() -> Self {
        Self {
            stripes: vec![Stripe {
                x1: -f64::MAX,
                x2: f64::MAX,
                value: -f64::MAX,
            }],
        }
    }
}

impl StripeFrontier {
    pub fn is_empty(&self) -> bool {
        self.stripes.len() == 1
    }

    /// Highest value of the stripes touching `x1..x2` (`getContact`).
    pub fn get_contact(&self, x1: f64, x2: f64) -> f64 {
        self.collisions(x1, x2)
            .into_iter()
            .map(|i| self.stripes[i].value)
            .fold(-f64::MAX, f64::max)
    }

    /// Indexes of the stripes overlapping `x1..x2` (`collisionning`).
    fn collisions(&self, x1: f64, x2: f64) -> Vec<usize> {
        let mut result = Vec::new();
        for (i, s) in self.stripes.iter().enumerate() {
            if x1 >= s.x2 {
                continue;
            }
            result.push(i);
            if x2 <= s.x2 {
                break;
            }
        }
        result
    }

    pub fn add_segment(&mut self, x1: f64, x2: f64, value: f64) {
        if x2 <= x1 {
            return;
        }
        let collisions = self.collisions(x1, x2);
        if collisions.len() > 1 {
            let starts: Vec<f64> = collisions[1..]
                .iter()
                .map(|&i| self.stripes[i].x1)
                .collect();
            let mut x = x1;
            for start in starts {
                self.add_segment(x, start, value);
                x = start;
            }
            self.add_segment(x, x2, value);
        } else if let Some(&i) = collisions.first() {
            self.add_single(x1, x2, value, i);
        }
    }

    fn add_single(&mut self, x1: f64, x2: f64, value: f64, touch: usize) {
        let t = self.stripes[touch];
        if value <= t.value {
            return;
        }
        let mut replacement = Vec::with_capacity(3);
        if t.x1 != x1 {
            replacement.push(Stripe {
                x1: t.x1,
                x2: x1,
                value: t.value,
            });
        }
        replacement.push(Stripe { x1, x2, value });
        if x2 != t.x2 {
            replacement.push(Stripe {
                x1: x2,
                x2: t.x2,
                value: t.value,
            });
        }
        self.stripes.splice(touch..=touch, replacement);
    }
}

/// A T shape: a thick short part followed by a thinner long part (`SymetricalTee`).
#[derive(Debug, Clone, Copy)]
pub struct SymetricalTee {
    pub thickness1: f64,
    pub elongation1: f64,
    pub thickness2: f64,
    pub elongation2: f64,
}

/// A tee at a vertical position (`SymetricalTeePositioned`).
#[derive(Debug, Clone, Copy)]
pub struct SymetricalTeePositioned {
    tee: SymetricalTee,
    y: f64,
}

impl SymetricalTeePositioned {
    pub fn new(tee: SymetricalTee) -> Self {
        Self { tee, y: 0.0 }
    }

    fn segment_a1_y(&self) -> f64 {
        self.y - self.tee.thickness1 / 2.0
    }

    fn segment_a2_y(&self) -> f64 {
        self.y - self.tee.thickness2 / 2.0
    }

    fn move_so_that_segment_a1_is_on(&mut self, new_y: f64) {
        self.y += new_y - self.segment_a1_y();
    }

    fn move_so_that_segment_a2_is_on(&mut self, new_y: f64) {
        self.y += new_y - self.segment_a2_y();
    }

    fn translate(&mut self, delta: f64) {
        self.y += delta;
    }

    fn segment_b1_y(&self) -> f64 {
        self.y + self.tee.thickness1 / 2.0
    }

    fn segment_b2_y(&self) -> f64 {
        self.y + self.tee.thickness2 / 2.0
    }

    fn max_x(&self) -> f64 {
        self.tee.elongation1 + self.tee.elongation2
    }

    fn half_thickness(&self) -> f64 {
        (self.tee.thickness1 / 2.0).max(self.tee.thickness2 / 2.0)
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    fn max(self, other: Self) -> Self {
        if other.y > self.y {
            other
        } else {
            self
        }
    }
}

/// Stacks tees top to bottom without overlap (`Tetris`).
#[derive(Debug, Clone)]
pub struct Tetris {
    frontier: StripeFrontier,
    elements: Vec<SymetricalTeePositioned>,
    min_y: f64,
    max_y: f64,
}

impl Default for Tetris {
    fn default() -> Self {
        Self {
            frontier: StripeFrontier::default(),
            elements: Vec::new(),
            min_y: f64::MAX,
            max_y: -f64::MAX,
        }
    }
}

impl Tetris {
    /// Centers the stack around 0 (`balance`).
    pub fn balance(&mut self) {
        if self.elements.is_empty() {
            return;
        }
        for e in &self.elements {
            self.min_y = self.min_y.min(e.y - e.half_thickness());
            self.max_y = self.max_y.max(e.y + e.half_thickness());
        }
        let mean = (self.min_y + self.max_y) / 2.0;
        for e in &mut self.elements {
            e.translate(-mean);
        }
    }

    pub fn height(&self) -> f64 {
        if self.elements.is_empty() {
            return 0.0;
        }
        self.max_y - self.min_y
    }

    pub fn width(&self) -> f64 {
        self.elements
            .iter()
            .map(SymetricalTeePositioned::max_x)
            .fold(0.0, f64::max)
    }

    pub fn add(&mut self, tee: SymetricalTee) {
        if self.frontier.is_empty() {
            self.add_internal(SymetricalTeePositioned::new(tee));
            return;
        }
        let c1 = self.frontier.get_contact(0.0, tee.elongation1);
        let c2 = self
            .frontier
            .get_contact(tee.elongation1, tee.elongation1 + tee.elongation2);
        let mut p1 = SymetricalTeePositioned::new(tee);
        p1.move_so_that_segment_a1_is_on(c1);
        let mut p2 = SymetricalTeePositioned::new(tee);
        p2.move_so_that_segment_a2_is_on(c2);
        self.add_internal(p1.max(p2));
    }

    fn add_internal(&mut self, result: SymetricalTeePositioned) {
        self.elements.push(result);
        self.frontier
            .add_segment(0.0, result.tee.elongation1, result.segment_b1_y());
        let (x1, x2) = (
            result.tee.elongation1,
            result.tee.elongation1 + result.tee.elongation2,
        );
        if x1 != x2 {
            self.frontier.add_segment(x1, x2, result.segment_b2_y());
        }
    }

    pub fn elements(&self) -> &[SymetricalTeePositioned] {
        &self.elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tee(t1: f64, e1: f64, t2: f64, e2: f64) -> SymetricalTee {
        SymetricalTee {
            thickness1: t1,
            elongation1: e1,
            thickness2: t2,
            elongation2: e2,
        }
    }

    #[test]
    fn frontier_keeps_the_highest_value() {
        let mut f = StripeFrontier::default();
        assert!(f.is_empty());
        f.add_segment(0.0, 10.0, 5.0);
        f.add_segment(5.0, 20.0, 8.0);
        f.add_segment(2.0, 3.0, 1.0);
        assert!(!f.is_empty());
        assert_eq!(f.get_contact(0.0, 4.0), 5.0);
        assert_eq!(f.get_contact(0.0, 6.0), 8.0);
        assert_eq!(f.get_contact(15.0, 30.0), 8.0);
        assert_eq!(f.get_contact(30.0, 40.0), -f64::MAX);
        f.add_segment(3.0, 3.0, 99.0);
        assert_eq!(f.get_contact(3.0, 4.0), 5.0);
    }

    #[test]
    fn tetris_stacks_and_balances() {
        let mut t = Tetris::default();
        t.balance();
        assert_eq!(t.height(), 0.0);
        t.add(tee(10.0, 20.0, 0.0, 0.0));
        t.add(tee(10.0, 30.0, 0.0, 0.0));
        t.add(tee(10.0, 5.0, 4.0, 40.0));
        t.balance();
        assert_eq!(t.elements().len(), 3);
        assert_eq!(t.height(), 30.0);
        assert_eq!(t.width(), 45.0);
        let ys: Vec<f64> = t
            .elements()
            .iter()
            .map(SymetricalTeePositioned::y)
            .collect();
        assert_eq!(ys, vec![-10.0, 0.0, 10.0]);
    }

    #[test]
    fn tee_goes_below_a_wide_neighbour() {
        let mut t = Tetris::default();
        t.add(tee(10.0, 10.0, 10.0, 50.0));
        t.add(tee(10.0, 10.0, 10.0, 50.0));
        t.balance();
        let ys: Vec<f64> = t
            .elements()
            .iter()
            .map(SymetricalTeePositioned::y)
            .collect();
        assert_eq!(ys, vec![-5.0, 5.0]);
    }
}
