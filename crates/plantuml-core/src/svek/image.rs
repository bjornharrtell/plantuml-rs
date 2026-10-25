//! What a node of the graph draws (`IEntityImage`).

use super::types::ShapeType;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::style::Margins;
use std::any::Any;

pub const CORNER: f64 = 25.0;
pub const MARGIN: f64 = 5.0;
pub const MARGIN_LINE: f64 = 5.0;

pub trait IEntityImage: TextBlock {
    fn shape_type(&self) -> ShapeType;

    fn shield(&self, _sb: &dyn StringBounder) -> Margins {
        Margins::default()
    }

    fn overscan_x(&self, _sb: &dyn StringBounder) -> f64 {
        0.0
    }

    fn is_hidden(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
}

/// Nothing to draw: used for an empty diagram (`EntityImageSimpleEmpty`).
pub struct EntityImageSimpleEmpty;

impl TextBlock for EntityImageSimpleEmpty {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> crate::klimt::font::Dimension {
        crate::klimt::font::Dimension::new(10.0, 10.0)
    }

    fn draw(&self, _ug: &crate::klimt::drawing::ugraphic::UGraphic) {}
}

impl IEntityImage for EntityImageSimpleEmpty {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
