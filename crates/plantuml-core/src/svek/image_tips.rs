//! Notes on members of a class, pointing at the member (`EntityImageTips`).

use super::bibliotekon::Bibliotekon;
use super::image::IEntityImage;
use super::image_note::{note_body, Opale};
use super::types::ShapeType;
use crate::abel::{Entity, Tip};
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{Direction, HorizontalAlignment};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::any::Any;

const Y_SPACING: f64 = 10.0;

pub struct EntityImageTips {
    entity: Entity,
    /// The entity the tips are about.
    other: Option<Entity>,
    bib: Bibliotekon,
    skin: SkinParam,
    hidden: bool,
}

impl EntityImageTips {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam, bib: &Bibliotekon) -> Self {
        let other = cuca
            .links()
            .iter()
            .find(|l| l.contains(entity))
            .map(|l| l.other(entity));
        Self {
            entity: entity.clone(),
            other,
            bib: bib.clone(),
            skin: skin.clone(),
            hidden: cuca.is_entity_hidden(entity),
        }
    }

    fn opale(&self, tip: &Tip) -> Opale {
        let q = queries::NOTE
            .add(super::edge::skin_style_name(&self.skin))
            .with_stereotype_obj(tip.stereotype.as_ref());
        let style = self
            .skin
            .style_builder()
            .merged_style(&q)
            .expect("note style");
        let back = tip
            .colors
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        Opale {
            shadowing: style.shadowing(),
            border: style.value(PName::LineColor).as_color(),
            back,
            text: note_body(
                tip.display.clone(),
                &self.skin,
                &style,
                HorizontalAlignment::Left,
            ),
            stroke: style.stroke(),
            round_corner: 0.0,
        }
    }

    /// Whether the tips stand on the right of the entity they are about.
    fn is_right(&self) -> bool {
        self.entity.name().ends_with("RIGHT")
    }

    fn draw_tips(&self, ug: &UGraphic) -> Option<()> {
        let sb = ug.string_bounder();
        let other = self.other.clone()?;
        let node_me = self.bib.node(&self.entity)?;
        let node_other = self.bib.node(&other)?;
        // The pointer leaves the tip on the side facing the entity.
        let mut direction = if self.is_right() {
            Direction::Left
        } else {
            Direction::Right
        };
        let mut ug = ug.clone();
        let mut height = 0.0;
        for (member, tip) in self.entity.tips() {
            let best = other.get().bodier.best_match(&member)?.to_string();
            let (mx, my, mw, mh) = node_other.image().inner_position(&best, sb)?;
            let opale = self.opale(&tip);
            let dim = opale.dimension(sb);
            let pp1 = (0.0, dim.height / 2.0);
            let mut x = node_other.min_x() - node_me.min_x();
            if direction == Direction::Right && x < 0.0 {
                direction = direction.inv();
            }
            x += if direction == Direction::Left {
                mx + mw
            } else {
                mx
            };
            let y = node_other.min_y() - node_me.min_y() - height + my + mh / 2.0;
            opale.draw(&ug, Some((direction, pp1, (x, y))));
            ug = ug.apply_translate(UTranslate::dy(dim.height + Y_SPACING));
            height += dim.height + Y_SPACING;
        }
        Some(())
    }
}

impl TextBlock for EntityImageTips {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let (mut width, mut height) = (0.0f64, 0.0);
        for (_, tip) in self.entity.tips() {
            let dim = self.opale(&tip).dimension(sb);
            height += dim.height + Y_SPACING;
            width = width.max(dim.width);
        }
        Dimension::new(width, height)
    }

    fn draw(&self, ug: &UGraphic) {
        if self.draw_tips(ug).is_none() {
            log::warn!("cannot place the tips of {}", self.entity.name());
        }
    }
}

impl IEntityImage for EntityImageTips {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
