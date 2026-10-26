//! The box of an object (`EntityImageObject`): name and fields without a class spot.

use super::image::IEntityImage;
use super::image_class::{ClassStencil, MembersArea};
use super::types::ShapeType;
use crate::abel::{Entity, EntityPortion};
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::body::LineBefore;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UShape, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, EmptyBlock, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Guillemet;
use crate::style::{queries, Margins, PName, SName, Style};
use std::any::Any;
use std::rc::Rc;

fn empty_block() -> Rc<dyn TextBlock> {
    Rc::new(EmptyBlock {
        width: 0.0,
        height: 0.0,
    })
}

fn style_of(skin: &SkinParam, entity: &Entity, q: &crate::style::StyleQuery) -> Style {
    let q = q.with_stereotype_obj(entity.stereotype().as_ref());
    skin.style_builder()
        .merged_style(&q)
        .unwrap_or_else(|| panic!("no style for object"))
}

struct ObjectHeader {
    name: Rc<dyn TextBlock>,
    stereo: Rc<dyn TextBlock>,
}

impl ObjectHeader {
    fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let name = self.name.calculate_dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        Dimension::new(
            name.width.max(stereo.width),
            name.height + stereo.height,
        )
    }

    fn draw(&self, ug: &UGraphic, width: f64, height: f64) {
        let sb = ug.string_bounder();
        let dim = self.dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        let name = self.name.calculate_dimension(sb);
        let y = (height - dim.height) / 2.0;
        let x_stereo = (width - stereo.width) / 2.0;
        self.stereo
            .draw(&ug.apply_translate(UTranslate::new(x_stereo, y)));
        let x_name = (width - name.width) / 2.0;
        self.name
            .draw(&ug.apply_translate(UTranslate::new(x_name, y + stereo.height)));
    }
}

pub struct EntityImageObject {
    entity: Entity,
    header: ObjectHeader,
    body: Option<Rc<dyn TextBlock>>,
    round_corner: f64,
    style: Style,
    style_header: Style,
    hidden: bool,
}

impl EntityImageObject {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageObject {
        let style = style_of(skin, &entity, &queries::OBJECTDIAG_OBJECT);
        let style_header = style_of(skin, &entity, &queries::OBJECTDIAG_OBJECT_HEADER);
        let round_corner = style.value(PName::RoundCorner).as_double();
        let show_fields = cuca.show_portion(EntityPortion::Field, &entity);

        let mut display = entity.display();
        if skin.strict_uml_style() {
            display = display.underlined_name();
        }
        let skin_simple = skin.skin_simple();
        let name = display.create_block(
            &style_header.font_configuration(Some(&entity.colors())),
            HorizontalAlignment::Center,
            &skin_simple,
            &style_header.wrap_width(),
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        );
        let name = with_margin(name, style.padding());

        let stereo = match (&entity.stereotype(), cuca.visible_stereotype_labels(&entity)) {
            (Some(st), Some(labels))
                if st.label(Guillemet::DoubleComparator).is_some() && !labels.is_empty() =>
            {
                let q = queries::OBJECTDIAG_OBJECT
                    .add(SName::Stereotype)
                    .for_stereotype_itself(Some(st));
                let s = skin.style_builder().merged_style(&q).expect("stereo style");
                let block = Display::create(&labels).create_block(
                    &s.font_configuration(None),
                    HorizontalAlignment::Center,
                    &skin_simple,
                    &crate::klimt::creole::LineBreakStrategy::NONE,
                    CreoleMode::Full,
                    None,
                    None,
                    0.0,
                    0.0,
                );
                with_margin(block, Margins::new(0.0, 1.0, 0.0, 1.0))
            }
            _ => empty_block(),
        };

        let fields = entity.get().bodier.fields_to_display();
        let body: Option<Rc<dyn TextBlock>> = if !show_fields {
            None
        } else if fields.is_empty() {
            Some(Rc::new(LineBefore {
                block: Rc::new(EmptyBlock {
                    width: 10.0,
                    height: 16.0,
                }),
                separator: '\0',
                title: None,
                default_thickness: style.value(PName::LineThickness).as_double(),
            }))
        } else {
            let area: Rc<dyn TextBlock> = Rc::new(MembersArea {
                members: fields,
                config: style.font_configuration(Some(&entity.colors())),
                wrap: style.wrap_width(),
                skin: skin.clone(),
                icon_size: skin.class_attribute_icon_size(),
                radius: skin.circled_character_radius(),
            });
            Some(Rc::new(LineBefore {
                block: with_margin(area, Margins::new(4.0, 6.0, 4.0, 6.0)),
                separator: '\0',
                title: None,
                default_thickness: style.value(PName::LineThickness).as_double(),
            }))
        };

        EntityImageObject {
            hidden: cuca.is_entity_hidden(&entity),
            entity,
            header: ObjectHeader { name, stereo },
            body,
            round_corner,
            style,
            style_header,
        }
    }
}

impl TextBlock for EntityImageObject {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let title = self.header.dimension(sb);
        let body = self
            .body
            .as_ref()
            .map_or(Dimension::new(0.0, 0.0), |b| b.calculate_dimension(sb));
        let mut width = body.width.max(title.width + 10.0);
        let min_class_width = self.style.value(PName::MinimumWidth).as_double();
        if width < min_class_width {
            width = min_class_width;
        }
        Dimension::new(width, title.height + body.height)
    }

    fn draw(&self, ug: &UGraphic) {
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
        self.draw_internal(ug);
        ug.close_group();
    }
}

impl EntityImageObject {
    fn draw_internal(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim_total = self.calculate_dimension(sb);
        let dim_header = self.header.dimension(sb);
        let (width_total, height_total) = (dim_total.width, dim_total.height);
        let mut rect = URectangle::build(width_total, height_total).rounded(self.round_corner);
        let colors = self.entity.colors();
        let mut border = colors.color(ColorType::Line).cloned();
        let mut header_back = colors.color(ColorType::Header).cloned();
        let mut back = colors.color(ColorType::Back).cloned();
        let shadow = self.style.shadowing();
        let border = border
            .take()
            .unwrap_or_else(|| self.style.value(PName::LineColor).as_color());
        let header_back = header_back.take().unwrap_or_else(|| match &back {
            None => self.style_header.value(PName::BackGroundColor).as_color(),
            Some(b) => b.clone(),
        });
        let back = back
            .take()
            .unwrap_or_else(|| self.style.value(PName::BackGroundColor).as_color());
        rect.delta_shadow = shadow;
        let ug = ug.apply_color(border).apply_back_color(back.clone());
        let stroke = colors.mute_stroke(self.style.stroke());
        if back != header_back {
            let mut rect2 =
                URectangle::build(width_total, dim_header.height).rounded(self.round_corner);
            rect2.delta_shadow = 0.0;
            ug.clone()
                .apply_back_color(header_back)
                .apply_stroke(stroke)
                .draw(rect2);
            rect.delta_shadow = 0.0;
            ug.clone()
                .apply_stroke(stroke)
                .apply_back_color(HColor::TRANSPARENT)
                .draw(rect);
        } else {
            ug.apply_stroke(stroke).draw(rect);
        }
        self.header
            .draw(&ug, width_total, dim_header.height);
        if let Some(body) = &self.body {
            let ug2 = ug.apply_stencil(Rc::new(ClassStencil(width_total)), stroke);
            body.draw(&ug2.apply_translate(UTranslate::dy(dim_header.height)));
        }
    }
}

impl IEntityImage for EntityImageObject {
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

#[allow(dead_code)]
fn _unused(_: &UShape) {}
