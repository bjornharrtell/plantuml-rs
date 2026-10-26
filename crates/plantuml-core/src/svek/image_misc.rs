//! Small shapes of the model (`EntityImageAssociationPoint`, `EntityImageAssociation`).

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::Entity;
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UEllipse, UGraphic, UPolygon};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName, SName};
use std::any::Any;

const POINT_SIZE: f64 = 4.0;
const DIAMOND_SIZE: f64 = 12.0;

/// The dot where a link meets an association class.
pub struct EntityImageAssociationPoint {
    color: HColor,
    hidden: bool,
}

impl EntityImageAssociationPoint {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let q = queries::CLASSDIAG_ARROW.with_stereotype_obj(entity.stereotype().as_ref());
        let style = skin.style_builder().merged_style(&q).expect("arrow style");
        Self {
            color: style.value(PName::LineColor).as_color(),
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageAssociationPoint {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(POINT_SIZE, POINT_SIZE)
    }

    fn draw(&self, ug: &UGraphic) {
        ug.apply_color(self.color.clone())
            .apply_back_color(self.color.clone())
            .draw(UEllipse::build(POINT_SIZE, POINT_SIZE));
    }
}

impl IEntityImage for EntityImageAssociationPoint {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The diamond of a diamond association.
pub struct EntityImageAssociation {
    style: crate::style::Style,
    hidden: bool,
}

impl EntityImageAssociation {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let q = queries::DIAMOND.add(SName::ClassDiagram);
        Self {
            style: skin
                .style_builder()
                .merged_style(&q)
                .expect("diamond style"),
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageAssociation {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(DIAMOND_SIZE * 2.0, DIAMOND_SIZE * 2.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let s = DIAMOND_SIZE;
        let mut diams = UPolygon::default();
        diams.add_point(s, 0.0);
        diams.add_point(s * 2.0, s);
        diams.add_point(s, s * 2.0);
        diams.add_point(0.0, s);
        diams.add_point(s, 0.0);
        diams.delta_shadow = self.style.shadowing();
        ug.apply_color(self.style.value(PName::LineColor).as_color())
            .apply_back_color(self.style.value(PName::BackGroundColor).as_color())
            .apply_stroke(self.style.stroke())
            .draw(diams);
    }
}

impl IEntityImage for EntityImageAssociation {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Diamond
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
