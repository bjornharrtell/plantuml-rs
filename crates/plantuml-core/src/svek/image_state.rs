//! Nodes of state diagrams (`EntityImageState`, `EntityImageStateCommon` and the
//! small pseudo-state images).

use super::image::{IEntityImage, MARGIN, MARGIN_LINE};
use super::types::ShapeType;
use crate::abel::{Entity, EntityPosition, LeafType};
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPolygon, URectangle, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::skin::{Rankdir, SkinParam};
use crate::style::{queries, Margins, PName, SName, Style, StyleQuery};
use std::any::Any;
use std::rc::Rc;

fn empty() -> Rc<dyn TextBlock> {
    Rc::new(EmptyBlock {
        width: 0.0,
        height: 0.0,
    })
}

fn state_query(skin: &SkinParam, entity: &Entity, q: &StyleQuery) -> Style {
    skin.style_builder()
        .merged_style(&q.with_stereotype_obj(entity.stereotype().as_ref()))
        .expect("state style")
}

fn description_query() -> StyleQuery {
    queries::STATEDIAG_STATE.clone().add(SName::Description)
}

fn name_block(entity: &Entity, style: &Style, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let display = entity.display();
    if display.is_null() {
        return empty();
    }
    display.create_block(
        &style.font_configuration(Some(&entity.colors())),
        style.horizontal_alignment(),
        &skin.skin_simple(),
        &style.wrap_width(),
        CreoleMode::Full,
        None,
        None,
        0.0,
        0.0,
    )
}

/// `Entity.getStateDescription`: the description body of a state.
pub fn state_description_block(entity: &Entity, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let style = state_query(skin, entity, &description_query());
    body_block(entity, &style, skin)
}

fn body_block(entity: &Entity, style: &Style, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let body = entity.get();
    if body.bodier.raw_body().is_empty() {
        return empty();
    }
    Display::create(body.bodier.raw_body()).create_block(
        &style.font_configuration(Some(&entity.colors())),
        style.horizontal_alignment(),
        &skin.skin_simple(),
        &style.wrap_width(),
        CreoleMode::Full,
        None,
        None,
        0.0,
        0.0,
    )
}

fn group(ug: &UGraphic, entity: &Entity, class: &str) {
    let mut attrs: Vec<(&str, String)> = vec![("class", class.to_string())];
    attrs.push((
        "data-qualified-name",
        crate::skin::rose::line::fix_group_value(&entity.qualified_name()),
    ));
    if let Some(l) = entity.location() {
        attrs.push(("data-source-line", l.position.to_string()));
    }
    attrs.push(("id", entity.uid()));
    ug.start_group(&attrs);
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

/// A rounded state box with its name and its description body.
pub struct EntityImageState {
    entity: Entity,
    name: Rc<dyn TextBlock>,
    body: Rc<dyn TextBlock>,
    style: Style,
    style_name: Style,
    style_description: Style,
    hidden: bool,
}

impl EntityImageState {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let style = state_query(skin, entity, &queries::STATEDIAG_STATE);
        let style_name = state_query(skin, entity, &queries::STATEDIAG_STATE_NAME);
        let style_description = state_query(skin, entity, &description_query());
        Self {
            entity: entity.clone(),
            name: name_block(entity, &style_name, skin),
            body: body_block(entity, &style_description, skin),
            style,
            style_name,
            style_description,
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageState {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim = self
            .name
            .calculate_dimension(sb)
            .merge_tb(self.body.calculate_dimension(sb));
        dim.delta(
            MARGIN * 2.0 + 2.0 * MARGIN_LINE,
            MARGIN * 2.0 + 2.0 * MARGIN_LINE,
        )
        .at_least(50.0, 50.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        self.draw_internal(ug);
        ug.close_group();
    }
}

impl EntityImageState {
    fn draw_internal(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim_total = self.calculate_dimension(sb);
        let dim_name = self.name.calculate_dimension(sb);
        let y_line = MARGIN + dim_name.height + MARGIN_LINE;
        let (border, back) = colors_of(&self.entity, &self.style);
        let name_back = self
            .entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| self.style_name.value(PName::BackGroundColor).as_color());
        let description_back = self
            .entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| {
                self.style_description
                    .value(PName::BackGroundColor)
                    .as_color()
            });
        let corner = self.style.value(PName::RoundCorner).as_double();
        let stroke = self.entity.colors().mute_stroke(self.style.stroke());

        let ug = ug
            .apply_color(border.clone())
            .apply_back_color(back.clone());
        if name_back == description_back {
            let mut rect = URectangle::build(dim_total.width, dim_total.height).rounded(corner);
            rect.delta_shadow = self.style.shadowing();
            ug.apply_stroke(stroke).draw(rect);
        } else {
            let mut rect = URectangle::build(dim_total.width, dim_total.height).rounded(corner);
            rect.delta_shadow = 0.0;
            ug.apply_back_color(description_back)
                .apply_stroke(stroke)
                .draw(rect);
            let mut top = URectangle::build(dim_total.width, y_line).rounded(corner);
            top.delta_shadow = 0.0;
            ug.apply_back_color(name_back.clone())
                .apply_color(name_back.clone())
                .apply_stroke(stroke)
                .draw(top);
            let mut cover = URectangle::build(dim_total.width, corner / 2.0);
            cover.delta_shadow = 0.0;
            ug.apply_back_color(name_back.clone())
                .apply_color(name_back.clone())
                .apply_stroke(stroke)
                .apply_translate(UTranslate::dy(y_line - cover.height))
                .draw(cover);
            ug.apply_stroke(stroke)
                .apply_back_color(HColor::TRANSPARENT)
                .draw(URectangle::build(dim_total.width, dim_total.height).rounded(corner));
        }
        ug.apply_stroke(stroke)
            .apply_color(border)
            .apply_translate(UTranslate::dy(y_line))
            .draw(ULine::hline(dim_total.width));

        self.name.draw(&ug.apply_translate(UTranslate::new(
            (dim_total.width - dim_name.width) / 2.0,
            MARGIN,
        )));
        self.body
            .draw(&ug.apply_translate(UTranslate::new(MARGIN, y_line + MARGIN_LINE)));
    }
}

impl IEntityImage for EntityImageState {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn shield(&self, _sb: &dyn StringBounder) -> Margins {
        self.entity.margins()
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A state without a body when `hide empty description` is set
/// (`EntityImageStateEmptyDescription`).
pub struct EntityImageStateEmptyDescription {
    entity: Entity,
    name: Rc<dyn TextBlock>,
    style: Style,
    style_description: Style,
    hidden: bool,
}

impl EntityImageStateEmptyDescription {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        Self {
            entity: entity.clone(),
            name: name_block(
                entity,
                &state_query(skin, entity, &queries::STATEDIAG_STATE_NAME),
                skin,
            ),
            style: state_query(skin, entity, &queries::STATEDIAG_STATE),
            style_description: state_query(skin, entity, &description_query()),
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageStateEmptyDescription {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.name
            .calculate_dimension(sb)
            .delta(MARGIN * 2.0, MARGIN * 2.0)
            .at_least(50.0, 40.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let sb = ug.string_bounder();
        let dim_total = self.calculate_dimension(sb);
        let dim_name = self.name.calculate_dimension(sb);
        let (border, back) = colors_of(&self.entity, &self.style_description);
        let mut rect = URectangle::build(dim_total.width, dim_total.height)
            .rounded(self.style.value(PName::RoundCorner).as_double());
        rect.delta_shadow = self.style.shadowing();
        ug.apply_color(border)
            .apply_back_color(back)
            .apply_stroke(self.entity.colors().mute_stroke(self.style.stroke()))
            .draw(rect);
        self.name.draw(&ug.apply_translate(UTranslate::new(
            (dim_total.width - dim_name.width) / 2.0,
            (dim_total.height - dim_name.height) / 2.0,
        )));
        ug.close_group();
    }
}

impl IEntityImage for EntityImageStateEmptyDescription {
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

/// An entry or exit point on the border of a composite state
/// (`EntityImageStateBorder`).
pub struct EntityImageStateBorder {
    entity: Entity,
    description: Rc<dyn TextBlock>,
    position: EntityPosition,
    style: Style,
    hidden: bool,
    up: bool,
}

impl EntityImageStateBorder {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam, parent_up: bool) -> Self {
        let style = state_query(skin, entity, &queries::STATEDIAG_STATE);
        Self {
            entity: entity.clone(),
            description: name_block(entity, &style, skin),
            position: entity.entity_position(),
            style,
            hidden: cuca.is_entity_hidden(entity),
            up: parent_up,
        }
    }
}

impl TextBlock for EntityImageStateBorder {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.description.calculate_dimension(sb);
        let size = EntityPosition::RADIUS * 2.0;
        Dimension::new(
            d.width.max(size),
            d.height + size + if d.height > 0.0 { 2.0 } else { 0.0 },
        )
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let sb = ug.string_bounder();
        let d = self.description.calculate_dimension(sb);
        let size = EntityPosition::RADIUS * 2.0;
        let x = (self.calculate_dimension(sb).width - size) / 2.0;
        let y = if self.up { d.height } else { 0.0 };
        self.description.draw(&ug.apply_translate(UTranslate::new(
            x + (size - d.width) / 2.0,
            if self.up { 0.0 } else { size },
        )));
        let (border, back) = colors_of(&self.entity, &self.style);
        let ug = ug
            .apply_color(border)
            .apply_back_color(back)
            .apply_stroke(UStroke::with_thickness(1.5))
            .apply_translate(UTranslate::new(x, y));
        self.position.draw_symbol(&ug);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageStateBorder {
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

/// The initial or junction pseudo-state (`EntityImageCircleStart`).
pub struct EntityImageCircleStart {
    entity: Entity,
    style: Style,
    hidden: bool,
    junction: bool,
}

impl EntityImageCircleStart {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        Self {
            entity: entity.clone(),
            style: state_query(
                skin,
                entity,
                &queries::CIRCLE_START.add(SName::StateDiagram),
            ),
            hidden: cuca.is_entity_hidden(entity),
            junction: entity.leaf_type() == Some(LeafType::Junction),
        }
    }

    fn size(&self) -> f64 {
        if self.junction {
            12.0
        } else {
            20.0
        }
    }
}

impl TextBlock for EntityImageCircleStart {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        let size = self.size();
        Dimension::new(size, size)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "start_entity");
        let (border, back) = colors_of(&self.entity, &self.style);
        let mut circle = UEllipse::build(self.size(), self.size());
        circle.delta_shadow = self.style.shadowing();
        ug.apply_color(border)
            .apply_back_color(back)
            .apply_stroke(self.style.stroke())
            .draw(circle);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageCircleStart {
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

/// The final state (`EntityImageCircleEnd`).
pub struct EntityImageCircleEnd {
    entity: Entity,
    style: Style,
    hidden: bool,
}

impl EntityImageCircleEnd {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        Self {
            entity: entity.clone(),
            style: state_query(skin, entity, &queries::CIRCLE_END.add(SName::StateDiagram)),
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageCircleEnd {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(22.0, 22.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "end_entity");
        let (border, back) = colors_of(&self.entity, &self.style);
        let mut circle = UEllipse::build(22.0, 22.0);
        circle.delta_shadow = self.style.shadowing();
        ug.apply_color(border.clone())
            .apply_back_color(HColor::TRANSPARENT)
            .apply_stroke(self.style.stroke())
            .draw(circle);
        let mut small = UEllipse::build(12.0, 12.0);
        small.delta_shadow = 0.0;
        ug.apply_color(if border == back { border } else { back.clone() })
            .apply_back_color(back)
            .apply_translate(UTranslate::new(5.0, 5.0))
            .draw(small);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageCircleEnd {
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

/// A choice pseudo-state (`EntityImageBranch`).
pub struct EntityImageBranch {
    entity: Entity,
    style: Style,
    hidden: bool,
}

impl EntityImageBranch {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        Self {
            entity: entity.clone(),
            style: state_query(skin, entity, &queries::ACTIVITYDIAG_ACTIVITY_DIAMOND),
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageBranch {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(24.0, 24.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let (border, back) = colors_of(&self.entity, &self.style);
        let s = 12.0;
        let mut diams = UPolygon::default();
        diams.add_point(s, 0.0);
        diams.add_point(s * 2.0, s);
        diams.add_point(s, s * 2.0);
        diams.add_point(0.0, s);
        diams.add_point(s, 0.0);
        diams.delta_shadow = self.style.shadowing();
        ug.apply_color(border)
            .apply_back_color(back)
            .apply_stroke(self.style.stroke())
            .draw(diams);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageBranch {
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

/// A fork or join bar (`EntityImageSynchroBar`).
pub struct EntityImageSynchroBar {
    entity: Entity,
    style: Style,
    hidden: bool,
    left_to_right: bool,
}

impl EntityImageSynchroBar {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        Self {
            entity: entity.clone(),
            style: state_query(skin, entity, &queries::ACTIVITYDIAG_ACTIVITYBAR),
            hidden: cuca.is_entity_hidden(entity),
            left_to_right: skin.rankdir == Rankdir::LeftToRight,
        }
    }
}

impl TextBlock for EntityImageSynchroBar {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        if self.left_to_right {
            Dimension::new(8.0, 80.0)
        } else {
            Dimension::new(80.0, 8.0)
        }
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let dim = self.calculate_dimension(ug.string_bounder());
        let mut rect = URectangle::build(dim.width, dim.height);
        rect.delta_shadow = self.style.shadowing();
        ug.apply_color(HColor::TRANSPARENT)
            .apply_back_color(self.style.value(PName::BackGroundColor).as_color())
            .draw(rect);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageSynchroBar {
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

/// A history pseudo-state (`EntityImagePseudoState`, `EntityImageDeepHistory`).
pub struct EntityImagePseudoState {
    entity: Entity,
    style: Style,
    text: Rc<dyn TextBlock>,
    hidden: bool,
}

impl EntityImagePseudoState {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam, deep: bool) -> Self {
        let mut q = queries::DIAMOND
            .add(SName::StateDiagram)
            .with_stereotype_obj(entity.stereotype().as_ref());
        q = q.with_stereotype("history");
        if deep {
            q = q.with_stereotype("deepHistory");
        }
        let style = skin
            .style_builder()
            .merged_style(&q)
            .expect("history style");
        let fc: FontConfiguration = style.font_configuration(Some(&entity.colors()));
        let text = Display::create(&[if deep { "H*" } else { "H" }]).create_simple_block(
            &fc,
            HorizontalAlignment::Center,
            &skin.skin_simple(),
        );
        Self {
            entity: entity.clone(),
            style,
            text,
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImagePseudoState {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(22.0, 22.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let (border, back) = colors_of(&self.entity, &self.style);
        let mut circle = UEllipse::build(22.0, 22.0);
        circle.delta_shadow = self.style.shadowing();
        ug.apply_color(border)
            .apply_back_color(back)
            .apply_stroke(self.entity.colors().mute_stroke(self.style.stroke()))
            .draw(circle);
        let d = self.text.calculate_dimension(ug.string_bounder());
        self.text.draw(&ug.apply_translate(UTranslate::new(
            (22.0 - d.width) / 2.0,
            (22.0 - d.height) / 2.0,
        )));
        ug.close_group();
    }
}

impl IEntityImage for EntityImagePseudoState {
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

/// The label of a transition, drawn as a node
/// (`skinparam stateDiagramEdgeLabelStyle node`).
pub struct EntityImageTransitionLabel {
    entity: Entity,
    text: Rc<dyn TextBlock>,
    hidden: bool,
}

impl EntityImageTransitionLabel {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let q = queries::ARROW
            .add(SName::StateDiagram)
            .with_stereotype_obj(entity.stereotype().as_ref());
        let style = skin.style_builder().merged_style(&q).expect("arrow style");
        let text = name_block(entity, &style, skin);
        Self {
            entity: entity.clone(),
            text,
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageTransitionLabel {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.text
            .calculate_dimension(sb)
            .delta(8.0, 8.0)
            .at_least(20.0, 20.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let sb = ug.string_bounder();
        let dim = self.calculate_dimension(sb);
        let text = self.text.calculate_dimension(sb);
        ug.draw(URectangle::build(dim.width, dim.height).rounded(5.0));
        self.text.draw(&ug.apply_translate(UTranslate::new(
            (dim.width - text.width) / 2.0,
            (dim.height - text.height) / 2.0,
        )));
        ug.close_group();
    }
}

impl IEntityImage for EntityImageTransitionLabel {
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

/// The `<<sdlreceive>>` variant of a state (`EntityImageState2`).
pub struct EntityImageState2 {
    entity: Entity,
    text: Rc<dyn TextBlock>,
    style: Style,
    hidden: bool,
}

impl EntityImageState2 {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let style = state_query(skin, entity, &queries::STATE);
        Self {
            entity: entity.clone(),
            text: name_block(entity, &style, skin),
            style,
            hidden: cuca.is_entity_hidden(entity),
        }
    }
}

impl TextBlock for EntityImageState2 {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.text.calculate_dimension(sb).delta(20.0, 20.0)
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let dim = self.calculate_dimension(ug.string_bounder());
        let text = self.text.calculate_dimension(ug.string_bounder());
        ug.apply_color(self.style.value(PName::LineColor).as_color())
            .apply_back_color(self.style.value(PName::BackGroundColor).as_color())
            .apply_stroke(self.style.stroke())
            .draw(URectangle::build(dim.width, dim.height));
        self.text.draw(&ug.apply_translate(UTranslate::new(
            (dim.width - text.width) / 2.0,
            (dim.height - text.height) / 2.0,
        )));
        ug.close_group();
    }
}

impl IEntityImage for EntityImageState2 {
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

/// A note used as a transition label (`EntityImageTransitionLabel` fallback).
pub fn state_image(
    entity: &Entity,
    cuca: &CucaDiagram,
    skin: &SkinParam,
) -> Option<Rc<dyn IEntityImage>> {
    let image: Rc<dyn IEntityImage> = match entity.leaf_type()? {
        LeafType::State => {
            if entity.entity_position() != EntityPosition::Normal {
                Rc::new(EntityImageStateBorder::new(entity, cuca, skin, false))
            } else if cuca.base.hide_empty_description && entity.get().bodier.raw_body().is_empty()
            {
                Rc::new(EntityImageStateEmptyDescription::new(entity, cuca, skin))
            } else if entity.stereotype().is_some_and(|s| {
                s.label(crate::stereo::Guillemet::DoubleComparator)
                    .is_some_and(|l| l == "<<sdlreceive>>")
            }) {
                Rc::new(EntityImageState2::new(entity, cuca, skin))
            } else {
                Rc::new(EntityImageState::new(entity, cuca, skin))
            }
        }
        LeafType::CircleStart | LeafType::Junction => {
            Rc::new(EntityImageCircleStart::new(entity, cuca, skin))
        }
        LeafType::CircleEnd => Rc::new(EntityImageCircleEnd::new(entity, cuca, skin)),
        LeafType::Branch | LeafType::StateChoice => {
            Rc::new(EntityImageBranch::new(entity, cuca, skin))
        }
        LeafType::SynchroBar | LeafType::StateForkJoin => {
            Rc::new(EntityImageSynchroBar::new(entity, cuca, skin))
        }
        LeafType::PseudoState => Rc::new(EntityImagePseudoState::new(entity, cuca, skin, false)),
        LeafType::DeepHistory => Rc::new(EntityImagePseudoState::new(entity, cuca, skin, true)),
        LeafType::StateTransitionLabel => {
            Rc::new(EntityImageTransitionLabel::new(entity, cuca, skin))
        }
        _ => return None,
    };
    Some(image)
}

/// A text block used as an entity image (`TextBlockToEntityImage`).
pub struct TextBlockToEntityImage {
    block: Rc<dyn TextBlock>,
}

impl TextBlockToEntityImage {
    pub fn new(block: Rc<dyn TextBlock>) -> Self {
        Self { block }
    }
}

impl TextBlock for TextBlockToEntityImage {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.block.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        self.block.draw(ug);
    }
}

impl IEntityImage for TextBlockToEntityImage {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Adds fixed padding around an inner image (`PaddedEntityImage`).
pub struct PaddedEntityImage {
    image: Rc<dyn IEntityImage>,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl PaddedEntityImage {
    pub fn new(image: Rc<dyn IEntityImage>, left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            image,
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn uniform(image: Rc<dyn IEntityImage>, padding: f64) -> Self {
        Self::new(image, padding, padding, padding, padding)
    }
}

impl TextBlock for PaddedEntityImage {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.image
            .calculate_dimension(sb)
            .delta(self.left + self.right, self.top + self.bottom)
    }

    fn draw(&self, ug: &UGraphic) {
        self.image
            .draw(&ug.apply_translate(UTranslate::new(self.left, self.top)));
    }
}

impl IEntityImage for PaddedEntityImage {
    fn shape_type(&self) -> ShapeType {
        self.image.shape_type()
    }

    fn shield(&self, sb: &dyn StringBounder) -> Margins {
        self.image.shield(sb)
    }

    fn overscan_x(&self, sb: &dyn StringBounder) -> f64 {
        self.image.overscan_x(sb)
    }

    fn is_hidden(&self) -> bool {
        self.image.is_hidden()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Regions of a concurrent state, stacked with a dashed separator
/// (`ConcurrentStates`).
pub struct ConcurrentStates {
    inners: Vec<Rc<dyn IEntityImage>>,
    vertical: bool,
    style: Style,
}

impl ConcurrentStates {
    pub fn new(inners: Vec<Rc<dyn IEntityImage>>, separator: char, skin: &SkinParam) -> Self {
        Self {
            inners,
            vertical: separator == '|',
            style: skin
                .style_builder()
                .merged_style(&queries::STATEDIAG_STATE)
                .expect("concurrent style"),
        }
    }

    fn add(&self, orig: Dimension, other: Dimension) -> Dimension {
        if self.vertical {
            Dimension::new(orig.width + other.width, orig.height.max(other.height))
        } else {
            Dimension::new(orig.width.max(other.width), orig.height + other.height)
        }
    }
}

impl TextBlock for ConcurrentStates {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut result = Dimension::new(0.0, 0.0);
        for inner in &self.inners {
            result = self.add(result, inner.calculate_dimension(sb));
        }
        result
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let total = self.calculate_dimension(sb);
        let border = self.style.value(PName::LineColor).as_color();
        let mut ug = ug.clone();
        for (i, inner) in self.inners.iter().enumerate() {
            inner.draw(&ug);
            let dim = inner.calculate_dimension(sb);
            let delta = if self.vertical {
                UTranslate::dx(dim.width)
            } else {
                UTranslate::dy(dim.height)
            };
            ug = ug.apply_translate(delta);
            if i + 1 < self.inners.len() {
                let separator = UStroke::new(8.0, 10.0, 1.5);
                let ug_sep = ug.apply_color(border.clone()).apply_stroke(separator);
                if self.vertical {
                    ug_sep.draw(ULine::vline(total.height + 8.0));
                } else {
                    ug_sep.draw(ULine::hline(total.width + 8.0));
                }
            }
        }
    }
}

impl IEntityImage for ConcurrentStates {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A composite state drawn around its pre-rendered content (`InnerStateAutonom`).
pub struct InnerStateAutonom {
    entity: Entity,
    inner: Rc<dyn IEntityImage>,
    name: Rc<dyn TextBlock>,
    attribute: Rc<dyn TextBlock>,
    style: Style,
    style_name: Style,
    style_description: Style,
    body_back: HColor,
    hidden: bool,
}

impl InnerStateAutonom {
    pub fn new(
        entity: &Entity,
        inner: Rc<dyn IEntityImage>,
        cuca: &CucaDiagram,
        skin: &SkinParam,
    ) -> Self {
        let style = state_query(skin, entity, &queries::STATEDIAG_STATE);
        let style_name = state_query(skin, entity, &queries::STATEDIAG_STATE_NAME);
        let style_description = state_query(skin, entity, &description_query());
        let body_style = state_query(
            skin,
            entity,
            &queries::STATEDIAG_STATE.clone().add(SName::Body),
        );
        let body_back = entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| body_style.value(PName::BackGroundColor).as_color());
        Self {
            entity: entity.clone(),
            inner,
            name: name_block(entity, &style_name, skin),
            attribute: body_block(entity, &style_description, skin),
            style,
            style_name,
            style_description,
            body_back,
            hidden: cuca.is_entity_hidden(entity),
        }
    }

    fn margin_for_fields(&self, sb: &dyn StringBounder) -> f64 {
        if self.attribute.calculate_dimension(sb).height > 0.0 {
            MARGIN
        } else {
            0.0
        }
    }

    fn space_y_for_inner(&self, sb: &dyn StringBounder) -> f64 {
        let name = self.name.calculate_dimension(sb);
        let attr = self.attribute.calculate_dimension(sb);
        let margin_for_fields = self.margin_for_fields(sb);
        MARGIN + name.height + MARGIN_LINE + attr.height + margin_for_fields + MARGIN_LINE
    }
}

impl TextBlock for InnerStateAutonom {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let name = self.name.calculate_dimension(sb);
        let attr = self.attribute.calculate_dimension(sb);
        let inner = self.inner.calculate_dimension(sb);
        let margin_for_fields = self.margin_for_fields(sb);
        let dim = name.merge_tb(attr).merge_tb(inner);
        dim.delta(
            MARGIN * 2.0 + 2.0 * MARGIN_LINE + margin_for_fields,
            MARGIN * 2.0 + 2.0 * MARGIN_LINE + margin_for_fields,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        group(ug, &self.entity, "entity");
        let sb = ug.string_bounder();
        let total = self.calculate_dimension(sb);
        let name = self.name.calculate_dimension(sb);
        let attr = self.attribute.calculate_dimension(sb);
        let name_height = MARGIN + name.height + MARGIN_LINE;
        let (border, _back) = colors_of(&self.entity, &self.style);
        let name_back = self
            .entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| self.style_name.value(PName::BackGroundColor).as_color());
        let description_back = self
            .entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| {
                self.style_description
                    .value(PName::BackGroundColor)
                    .as_color()
            });
        let corner = self.style.value(PName::RoundCorner).as_double();
        let stroke = self.entity.colors().mute_stroke(self.style.stroke());
        let description_height = attr.height + self.margin_for_fields(sb);
        let container = crate::svek::rounded_container::RoundedContainer::new(
            border,
            total.width,
            total.height,
            name_height,
            description_height,
            name_back,
            description_back,
            self.body_back.clone(),
            stroke,
            corner,
            self.style.shadowing(),
        );
        container.draw(ug);

        self.name
            .draw(&ug.apply_translate(UTranslate::new((total.width - name.width) / 2.0, MARGIN)));
        let attr_y = MARGIN + name.height + MARGIN;
        if attr.height > 0.0 {
            self.attribute
                .draw(&ug.apply_translate(UTranslate::new(MARGIN, attr_y)));
        }
        self.inner
            .draw(&ug.apply_translate(UTranslate::new(MARGIN, self.space_y_for_inner(sb))));
        ug.close_group();
    }
}

impl IEntityImage for InnerStateAutonom {
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
