//! Shapes of Chen EER diagrams (`EntityImageChenEntity`, `EntityImageChenRelationship`,
//! `EntityImageChenAttribute` and `EntityImageChenCircle`).

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::Entity;
use crate::cucadiagram::CucaDiagram;
use crate::decoration::usymbol::figures::TextBlockInEllipse;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, UPolygon, URectangle, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Guillemet;
use crate::style::{queries, PName, Style, StyleQuery};
use std::any::Any;
use std::rc::Rc;

const MARGIN: f64 = 5.0;
const MARGIN_LINE: f64 = 5.0;
/// Margin around the text of an attribute or a circle.
const ELLIPSE_MARGIN: f64 = 6.0;

fn style_of(skin: &SkinParam, entity: &Entity, q: &StyleQuery) -> Style {
    let q = q.with_stereotype_obj(entity.stereotype().as_ref());
    skin.style_builder()
        .merged_style(&q)
        .unwrap_or_else(|| panic!("no style for chen element"))
}

fn has_stereotype(entity: &Entity, stereotype: &str) -> bool {
    entity.stereotype().is_some_and(|st| {
        st.labels(Guillemet::DoubleComparator)
            .iter()
            .any(|l| l == stereotype)
    })
}

fn title_block(
    entity: &Entity,
    skin: &SkinParam,
    query: &StyleQuery,
    underline: bool,
) -> (Rc<dyn TextBlock>, Style) {
    let style = style_of(skin, entity, query);
    let mut fc: FontConfiguration = style.font_configuration(Some(&entity.colors()));
    if underline {
        fc = fc.underline();
    }
    let block = entity.display().create_block(
        &fc,
        HorizontalAlignment::Center,
        &skin.skin_simple(),
        &style.wrap_width(),
        CreoleMode::Full,
        None,
        None,
        0.0,
        0.0,
    );
    (block, style)
}

fn colors_of(entity: &Entity, style: &Style) -> (HColor, HColor) {
    let colors = entity.colors();
    let border = colors
        .color(ColorType::Line)
        .cloned()
        .unwrap_or_else(|| style.value(PName::LineColor).as_color());
    let back = colors
        .color(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
    (border, back)
}

fn attributes(entity: &Entity) -> (bool, bool, bool, String) {
    let name = entity.name().to_string();
    (
        has_stereotype(entity, "<<key>>"),
        has_stereotype(entity, "<<multi>>"),
        has_stereotype(entity, "<<derived>>"),
        name,
    )
}

fn start_group(ug: &UGraphic, entity: &Entity) {
    // Java starts a group with `UGroupType.ID` for these images, which carries no attribute in the
    // generated SVG; only the bare `<g>` shows up, so no attributes are written here either.
    let _ = entity;
    ug.start_group(&[]);
}

/// An entity (`EntityImageChenEntity`): a rectangle, doubled when `<<weak>>`.
pub struct EntityImageChenEntity {
    entity: Entity,
    title: Rc<dyn TextBlock>,
    style: Style,
    is_weak: bool,
    hidden: bool,
}

impl EntityImageChenEntity {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageChenEntity {
        let (title, style) = title_block(&entity, skin, &queries::CHENEER_ENTITY_TITLE, false);
        EntityImageChenEntity {
            hidden: cuca.is_entity_hidden(&entity),
            is_weak: has_stereotype(&entity, "<<weak>>"),
            entity,
            title,
            style,
        }
    }
}

impl TextBlock for EntityImageChenEntity {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim = self.title.calculate_dimension(sb);
        Dimension::new(
            dim.width + 2.0 * MARGIN + 2.0 * MARGIN_LINE,
            dim.height + 2.0 * MARGIN + 2.0 * MARGIN_LINE,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        start_group(ug, &self.entity);
        let sb = ug.string_bounder();
        let dim_total = self.calculate_dimension(sb);
        let dim_title = self.title.calculate_dimension(sb);
        let (border, back) = colors_of(&self.entity, &self.style);
        let stroke = self.entity.colors().mute_stroke(self.style.stroke());
        let ug = ug
            .apply_color(border)
            .apply_back_color(back)
            .apply_stroke(stroke);
        ug.draw(URectangle::build(dim_total.width, dim_total.height));
        if self.is_weak {
            ug.apply_translate(UTranslate::new(3.0, 3.0))
                .draw(URectangle::build(
                    dim_total.width - 6.0,
                    dim_total.height - 6.0,
                ));
        }
        let x = (dim_total.width - dim_title.width) / 2.0;
        let y = MARGIN + MARGIN_LINE;
        self.title.draw(&ug.apply_translate(UTranslate::new(x, y)));
        ug.close_group();
    }
}

impl IEntityImage for EntityImageChenEntity {
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

/// A relationship (`EntityImageChenRelationship`): a diamond, doubled when `<<identifying>>`.
pub struct EntityImageChenRelationship {
    entity: Entity,
    title: Rc<dyn TextBlock>,
    style: Style,
    is_identifying: bool,
    hidden: bool,
}

impl EntityImageChenRelationship {
    pub fn new(
        entity: Entity,
        cuca: &CucaDiagram,
        skin: &SkinParam,
    ) -> EntityImageChenRelationship {
        let (title, style) =
            title_block(&entity, skin, &queries::CHENEER_RELATIONSHIP_TITLE, false);
        EntityImageChenRelationship {
            hidden: cuca.is_entity_hidden(&entity),
            is_identifying: has_stereotype(&entity, "<<identifying>>"),
            entity,
            title,
            style,
        }
    }

    fn polygon(width: f64, height: f64) -> UPolygon {
        UPolygon::new(vec![
            (0.0, height / 2.0),
            (width / 2.0, 0.0),
            (width, height / 2.0),
            (width / 2.0, height),
        ])
    }
}

impl TextBlock for EntityImageChenRelationship {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim = self.title.calculate_dimension(sb);
        let diagonal = (dim.width + 2.0 * dim.height) / 5.0f64.sqrt() + 2.0 * MARGIN;
        Dimension::new(diagonal * 5.0f64.sqrt(), diagonal * 5.0f64.sqrt() / 2.0)
    }

    fn draw(&self, ug: &UGraphic) {
        start_group(ug, &self.entity);
        let sb = ug.string_bounder();
        let dim_total = self.calculate_dimension(sb);
        let dim_title = self.title.calculate_dimension(sb);
        let (border, back) = colors_of(&self.entity, &self.style);
        let stroke = self.entity.colors().mute_stroke(self.style.stroke());
        let ug = ug
            .apply_color(border)
            .apply_back_color(back)
            .apply_stroke(stroke);
        ug.draw(Self::polygon(dim_total.width, dim_total.height));
        if self.is_identifying {
            ug.apply_translate(UTranslate::new(10.0, 5.0))
                .draw(Self::polygon(
                    dim_total.width - 20.0,
                    dim_total.height - 10.0,
                ));
        }
        let x = (dim_total.width - dim_title.width) / 2.0;
        let y = (dim_total.height - dim_title.height) / 2.0;
        self.title.draw(&ug.apply_translate(UTranslate::new(x, y)));
        ug.close_group();
    }
}

impl IEntityImage for EntityImageChenRelationship {
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

fn ellipse_stroke(entity: &Entity, style: &Style, derived: bool) -> UStroke {
    let stroke = entity.colors().mute_stroke(style.stroke());
    if derived {
        UStroke::new(10.0, 10.0, stroke.thickness)
    } else {
        stroke
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_ellipse(
    ug: &UGraphic,
    entity: &Entity,
    style: &Style,
    title: &Rc<dyn TextBlock>,
    total: Dimension,
    title_dim: Dimension,
    derived: bool,
    multi: bool,
    title_y: f64,
) {
    let (border, back) = colors_of(entity, style);
    let ug = ug
        .apply_color(border)
        .apply_back_color(back)
        .apply_stroke(ellipse_stroke(entity, style, derived));
    ug.draw(UEllipse::build(total.width, total.height));
    if multi {
        ug.apply_translate(UTranslate::new(3.0, 3.0))
            .draw(UEllipse::build(total.width - 6.0, total.height - 6.0));
    }
    let x = (total.width - title_dim.width) / 2.0;
    title.draw(&ug.apply_translate(UTranslate::new(x, title_y)));
}

/// An attribute (`EntityImageChenAttribute`): an ellipse fitted around the text.
pub struct EntityImageChenAttribute {
    entity: Entity,
    title: Rc<dyn TextBlock>,
    style: Style,
    is_multi: bool,
    is_derived: bool,
    hidden: bool,
}

impl EntityImageChenAttribute {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageChenAttribute {
        let (is_key, is_multi, is_derived, _) = attributes(&entity);
        let (title, style) = title_block(&entity, skin, &queries::CHENEER_ATTRIBUTE_TITLE, is_key);
        EntityImageChenAttribute {
            hidden: cuca.is_entity_hidden(&entity),
            entity,
            title,
            style,
            is_multi,
            is_derived,
        }
    }

    fn ellipse(&self, sb: &dyn StringBounder) -> TextBlockInEllipse {
        TextBlockInEllipse::new(self.title.clone(), sb)
    }
}

impl TextBlock for EntityImageChenAttribute {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.ellipse(sb).dimension()
    }

    fn draw(&self, ug: &UGraphic) {
        start_group(ug, &self.entity);
        let sb = ug.string_bounder();
        let ellipse = self.ellipse(sb);
        let total = ellipse.dimension();
        let title_dim = self.title.calculate_dimension(sb);
        draw_ellipse(
            ug,
            &self.entity,
            &self.style,
            &self.title,
            total,
            title_dim,
            self.is_derived,
            self.is_multi,
            ELLIPSE_MARGIN,
        );
        ug.close_group();
    }
}

impl IEntityImage for EntityImageChenAttribute {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Oval
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A subclass discriminator circle (`EntityImageChenCircle`).
pub struct EntityImageChenCircle {
    entity: Entity,
    title: Rc<dyn TextBlock>,
    style: Style,
    is_multi: bool,
    is_derived: bool,
    hidden: bool,
}

impl EntityImageChenCircle {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageChenCircle {
        let (is_key, is_multi, is_derived, _) = attributes(&entity);
        let (title, style) = title_block(&entity, skin, &queries::CHENEER_CIRCLE_TITLE, is_key);
        EntityImageChenCircle {
            hidden: cuca.is_entity_hidden(&entity),
            entity,
            title,
            style,
            is_multi,
            is_derived,
        }
    }
}

impl TextBlock for EntityImageChenCircle {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(25.0, 25.0)
    }

    fn draw(&self, ug: &UGraphic) {
        start_group(ug, &self.entity);
        let sb = ug.string_bounder();
        let total = self.calculate_dimension(sb);
        let title_dim = self.title.calculate_dimension(sb);
        draw_ellipse(
            ug,
            &self.entity,
            &self.style,
            &self.title,
            total,
            title_dim,
            self.is_derived,
            self.is_multi,
            (total.height - title_dim.height) / 2.0,
        );
        ug.close_group();
    }
}

impl IEntityImage for EntityImageChenCircle {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Oval
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
