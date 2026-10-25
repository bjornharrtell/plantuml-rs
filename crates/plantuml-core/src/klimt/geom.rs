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

#[cfg(test)]
mod tests {
    use super::*;

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
