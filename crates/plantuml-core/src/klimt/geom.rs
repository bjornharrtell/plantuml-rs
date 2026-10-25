//! Geometry helpers, ported from `klimt/geom`.

/// Horizontal text/box alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "LEFT" => Some(Self::Left),
            "CENTER" => Some(Self::Center),
            "RIGHT" => Some(Self::Right),
            _ => None,
        }
    }

    pub fn from_string_or(s: Option<&str>, default: Self) -> Self {
        s.and_then(Self::from_string).unwrap_or(default)
    }

    /// Graphviz `labeljust` letter.
    pub fn graphviz_value(self) -> &'static str {
        match self {
            Self::Left => "l",
            Self::Center => "c",
            Self::Right => "r",
        }
    }

    /// Offset of an object of `width` inside `full_width`.
    pub fn position(self, width: f64, full_width: f64) -> f64 {
        match self {
            Self::Right => full_width - width,
            Self::Center => (full_width - width) / 2.0,
            Self::Left => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    Top,
    Center,
    #[default]
    Bottom,
}

impl VerticalAlignment {
    pub fn from_string(s: &str) -> Self {
        if s.eq_ignore_ascii_case("top") {
            Self::Top
        } else {
            Self::Bottom
        }
    }
}

/// Bounding box accumulator (`MinMax`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinMax {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl MinMax {
    pub fn empty(init_to_zero: bool) -> Self {
        if init_to_zero {
            Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            }
        } else {
            Self {
                min_x: f64::MAX,
                min_y: f64::MAX,
                max_x: -f64::MAX,
                max_y: -f64::MAX,
            }
        }
    }

    pub fn from_max(max_x: f64, max_y: f64) -> Self {
        Self::empty(true).add_point(max_x, max_y)
    }

    pub fn add_point(self, x: f64, y: f64) -> Self {
        Self {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
        }
    }

    pub fn add_min_max(self, o: MinMax) -> Self {
        Self {
            min_x: self.min_x.min(o.min_x),
            min_y: self.min_y.min(o.min_y),
            max_x: self.max_x.max(o.max_x),
            max_y: self.max_y.max(o.max_y),
        }
    }

    /// True while no point was added to a box created with `empty(false)`.
    pub fn is_infinity(&self) -> bool {
        self.min_x == f64::MAX || self.max_x == -f64::MAX
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self {
            min_x: self.min_x + dx,
            min_y: self.min_y + dy,
            max_x: self.max_x + dx,
            max_y: self.max_y + dy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_max() {
        let m = MinMax::empty(false).add_point(1.0, 2.0).add_point(4.0, 6.0);
        assert_eq!((m.width(), m.height()), (3.0, 4.0));
        let z = MinMax::from_max(2.0, 2.0).add_min_max(m);
        assert_eq!((z.min_x, z.max_y), (0.0, 6.0));
        assert_eq!(m.translate(1.0, 1.0).min_x, 2.0);
    }

    #[test]
    fn alignment() {
        assert_eq!(
            HorizontalAlignment::from_string("Right"),
            Some(HorizontalAlignment::Right)
        );
        assert_eq!(HorizontalAlignment::from_string("x"), None);
        assert_eq!(
            HorizontalAlignment::from_string_or(None, HorizontalAlignment::Center),
            HorizontalAlignment::Center
        );
        assert_eq!(HorizontalAlignment::Center.graphviz_value(), "c");
        assert_eq!(HorizontalAlignment::Right.position(2.0, 10.0), 8.0);
        assert_eq!(HorizontalAlignment::Center.position(2.0, 10.0), 4.0);
        assert_eq!(HorizontalAlignment::Left.position(2.0, 10.0), 0.0);
        assert_eq!(
            VerticalAlignment::from_string("TOP"),
            VerticalAlignment::Top
        );
        assert_eq!(
            VerticalAlignment::from_string("x"),
            VerticalAlignment::Bottom
        );
    }
}

/// Cardinal direction (`Direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Right,
    Left,
    Down,
    Up,
}

impl Direction {
    pub fn inv(self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Left => Direction::Right,
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }

    pub fn from_char(c: char) -> Direction {
        match c {
            '<' => Direction::Left,
            '>' => Direction::Right,
            '^' => Direction::Up,
            _ => Direction::Down,
        }
    }

    pub fn clockwise(self) -> Direction {
        match self {
            Direction::Right => Direction::Down,
            Direction::Left => Direction::Up,
            Direction::Down => Direction::Left,
            Direction::Up => Direction::Right,
        }
    }
}

/// Position of a note around its entity (`Position`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    Right,
    Left,
    Bottom,
    Top,
}

impl Position {
    pub fn from_string(s: &str) -> Option<Position> {
        match s.to_uppercase().as_str() {
            "RIGHT" => Some(Position::Right),
            "LEFT" => Some(Position::Left),
            "BOTTOM" => Some(Position::Bottom),
            "TOP" => Some(Position::Top),
            _ => None,
        }
    }

    pub fn with_rankdir(self, rankdir: crate::skin::Rankdir) -> Position {
        if rankdir == crate::skin::Rankdir::TopToBottom {
            return self;
        }
        match self {
            Position::Right => Position::Bottom,
            Position::Left => Position::Top,
            Position::Bottom => Position::Right,
            Position::Top => Position::Left,
        }
    }
}

/// Side of a rectangle (`Side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}
