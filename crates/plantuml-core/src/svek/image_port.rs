//! A port of a component (`EntityImagePort`): a small square with a label outside.

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::Entity;
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// Radius of a port square (`EntityPosition.RADIUS`).
const RADIUS: f64 = 6.0;

pub struct EntityImagePort {
    entity: Entity,
    desc: Rc<dyn TextBlock>,
    border: crate::klimt::color::HColor,
    back: crate::klimt::color::HColor,
    /// Whether the label is drawn above the square (`upPosition`).
    up: Cell<bool>,
    hidden: bool,
}

impl EntityImagePort {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImagePort {
        let style = skin
            .style_builder()
            .merged_style(&queries::PORT.with_stereotype_obj(entity.stereotype().as_ref()))
            .unwrap_or_else(|| panic!("no style for port"));
        let skin_simple = skin.skin_simple();
        let desc = entity.display().create_block(
            &style.font_configuration(Some(&entity.colors())),
            HorizontalAlignment::Center,
            &skin_simple,
            &style.wrap_width(),
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        );
        let colors = entity.colors();
        let border = colors
            .color(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let back = colors
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        EntityImagePort {
            hidden: cuca.is_entity_hidden(&entity),
            entity,
            desc,
            border,
            back,
            up: Cell::new(false),
        }
    }

    /// Tells whether the label must be drawn above the square (`upPosition`).
    pub fn set_up(&self, up: bool) {
        self.up.set(up);
    }
}

impl TextBlock for EntityImagePort {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(RADIUS * 2.0, RADIUS * 2.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = self.desc.calculate_dimension(sb);
        let x = -(dim.width - RADIUS * 2.0) / 2.0;
        let y = if self.up.get() {
            -RADIUS * 2.0 - dim.height
        } else {
            RADIUS * 2.0
        };
        let mut attrs: Vec<(&str, String)> = vec![("class", "entity".to_string())];
        attrs.push((
            "data-qualified-name",
            crate::skin::rose::line::fix_group_value(&self.entity.qualified_name()),
        ));
        if let Some(l) = self.entity.location() {
            attrs.push(("data-source-line", l.position.to_string()));
        }
        attrs.push(("id", self.entity.uid()));
        ug.start_group(&attrs);
        self.desc.draw(&ug.apply_translate(UTranslate::new(x, y)));
        ug.apply_color(self.border.clone())
            .apply_back_color(self.back.clone())
            .apply_stroke(UStroke::with_thickness(1.5))
            .draw(URectangle::build(RADIUS * 2.0, RADIUS * 2.0));
        ug.close_group();
    }
}

impl IEntityImage for EntityImagePort {
    fn shape_type(&self) -> ShapeType {
        ShapeType::RectanglePort
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
