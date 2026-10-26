//! The box of a class (`EntityImageClass`, `EntityImageClassHeader`, `MethodsOrFieldsArea`).

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::{Entity, EntityPortion, LeafType, Member};
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::body::LineBefore;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, URectangle, UShape, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::{
    with_margin, HorizontalBlock, InnerRect, Stencil, TextBlock, VerticalBlock,
};
use crate::skin::visibility::VisibilityModifier;
use crate::skin::SkinParam;
use crate::stereo::Guillemet;
use crate::style::{queries, Margins, PName, SName, Style};
use std::any::Any;
use std::rc::Rc;

fn margin_xy(x1: f64, x2: f64, y1: f64, y2: f64) -> Margins {
    Margins::new(y1, x2, y2, x1)
}

fn empty_block() -> Rc<dyn TextBlock> {
    Rc::new(crate::klimt::shape::EmptyBlock {
        width: 0.0,
        height: 0.0,
    })
}

fn style_of(skin: &SkinParam, entity: &Entity, q: &crate::style::StyleQuery) -> Style {
    let q = q.with_stereotype_obj(entity.stereotype().as_ref());
    skin.style_builder()
        .merged_style(&q)
        .unwrap_or_else(|| panic!("no style for class"))
}

/// Round spot with a letter (`CircledCharacter`).
struct CircledCharacter {
    c: char,
    radius: f64,
    back: HColor,
    border: HColor,
    font_color: HColor,
}

impl TextBlock for CircledCharacter {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(2.0 * self.radius, 2.0 * self.radius)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug
            .apply_color(self.border.clone())
            .apply_back_color(self.back.clone());
        ug.draw(UEllipse::build(self.radius * 2.0, self.radius * 2.0));
        let ug = ug
            .apply_color(self.font_color.appropriate_for(&self.back))
            .apply_translate(UTranslate::new(self.radius, self.radius));
        ug.draw(UShape::CenteredChar(self.c));
    }
}

fn circled_char(t: LeafType) -> char {
    match t {
        LeafType::Annotation => '@',
        LeafType::AbstractClass => 'A',
        LeafType::Class => 'C',
        LeafType::Interface => 'I',
        LeafType::Enum | LeafType::Entity => 'E',
        LeafType::Protocol => 'P',
        LeafType::Struct | LeafType::Stereotype => 'S',
        LeafType::Exception => 'X',
        LeafType::Metaclass => 'M',
        LeafType::Dataclass => 'D',
        LeafType::Record => 'R',
        _ => '?',
    }
}

fn spot_query(t: LeafType) -> crate::style::StyleQuery {
    match t {
        LeafType::Annotation => queries::SPOT_ANNOTATION.clone(),
        LeafType::AbstractClass => queries::SPOT_ABSTRACT_CLASS.clone(),
        LeafType::Interface => queries::SPOT_INTERFACE.clone(),
        LeafType::Enum => queries::SPOT_ENUM.clone(),
        LeafType::Entity => queries::SPOT_ENTITY.clone(),
        LeafType::Protocol => queries::SPOT_PROTOCOL.clone(),
        LeafType::Struct => queries::SPOT_STRUCT.clone(),
        LeafType::Exception => queries::SPOT_EXCEPTION.clone(),
        LeafType::Metaclass => queries::SPOT_META_CLASS.clone(),
        LeafType::Stereotype => queries::SPOT_STEREOTYPE.clone(),
        LeafType::Dataclass => queries::SPOT_DATA_CLASS.clone(),
        LeafType::Record => queries::SPOT_RECORD.clone(),
        _ => queries::SPOT_CLASS.clone(),
    }
}

fn circled_character(entity: &Entity, skin: &SkinParam) -> Rc<dyn TextBlock> {
    let leaf_type = entity.leaf_type().unwrap_or(LeafType::Class);
    let style = skin
        .style_builder()
        .merged_style(&spot_query(leaf_type))
        .expect("spot style");
    let spot_border = style.value(PName::LineColor).as_color();
    let spot_back = style.value(PName::BackGroundColor).as_color();
    let font_color = style.value(PName::FontColor).as_color();
    let radius = skin.circled_character_radius();
    let stereotype = entity.stereotype();
    if let Some(st) = &stereotype {
        if st.character() != '\0' {
            return Rc::new(CircledCharacter {
                c: st.character(),
                radius,
                back: st.html_color().cloned().unwrap_or(spot_back),
                border: spot_border,
                font_color,
            });
        }
    }
    Rc::new(CircledCharacter {
        c: circled_char(leaf_type),
        radius,
        back: spot_back,
        border: spot_border,
        font_color,
    })
}

/// Name, spot, stereotype and generic of a class (`HeaderLayout`).
pub struct ClassHeader {
    circled: Rc<dyn TextBlock>,
    stereo: Rc<dyn TextBlock>,
    name: Rc<dyn TextBlock>,
    generic: Rc<dyn TextBlock>,
}

impl ClassHeader {
    fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> ClassHeader {
        let leaf_type = entity.leaf_type().unwrap_or(LeafType::Class);
        let italic = matches!(leaf_type, LeafType::AbstractClass | LeafType::Interface);
        let stereotype = entity.stereotype();
        let old_fashion = skin.is_true("genericdisplay") && skin.value_is("genericdisplay", "old");
        let generic = if old_fashion { None } else { entity.generic() };
        let style_header = style_of(skin, entity, &queries::CLASSDIAG_CLASS_HEADER);
        let mut fc = style_header.font_configuration(Some(&entity.colors()));
        if italic {
            fc = fc.italic();
        }
        let mut display = entity.display();
        if old_fashion {
            if let Some(g) = entity.generic() {
                display = display.add_generic(&g);
            }
        }
        let skin_simple = skin.skin_simple();
        let mut name = display.create_block(
            &fc,
            HorizontalAlignment::Center,
            &skin_simple,
            &style_header.wrap_width(),
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        );
        let modifier = entity.get().visibility;
        name = match modifier {
            None => with_margin(name, margin_xy(3.0, 3.0, 0.0, 0.0)),
            Some(m) => {
                let (fore, back) = icon_colors(skin, m);
                let ublock = with_margin(
                    m.u_block(skin.class_attribute_icon_size(), fore, back, false),
                    margin_xy(0.0, 0.0, 4.0, 0.0),
                );
                let merged: Rc<dyn TextBlock> = Rc::new(HorizontalBlock {
                    blocks: vec![ublock, name],
                    alignment: VerticalAlignment::Center,
                });
                with_margin(merged, margin_xy(3.0, 3.0, 0.0, 0.0))
            }
        };
        let labels = cuca.visible_stereotype_labels(entity);
        let stereo = match (&stereotype, labels) {
            (Some(st), Some(labels))
                if st.label(Guillemet::DoubleComparator).is_some() && !labels.is_empty() =>
            {
                let q = queries::CLASSDIAG_CLASS
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
                with_margin(block, margin_xy(1.0, 1.0, 0.0, 0.0))
            }
            _ => empty_block(),
        };
        let generic_block = match generic {
            None => empty_block(),
            Some(g) => {
                let q = match &stereotype {
                    Some(st) => queries::CLASSDIAG_CLASS
                        .add(SName::Stereotype)
                        .for_stereotype_itself(Some(st)),
                    None => queries::CLASSDIAG_CLASS.add(SName::Stereotype),
                };
                let s = skin.style_builder().merged_style(&q).expect("stereo style");
                let b = Display::with_newlines(Some(&g)).create_block(
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
                let b = with_margin(b, margin_xy(1.0, 1.0, 1.0, 1.0));
                let gq = queries::CLASSDIAG_CLASS_GENERIC.with_stereotype_obj(stereotype.as_ref());
                let gs = skin
                    .style_builder()
                    .merged_style(&gq)
                    .expect("generic style");
                let b: Rc<dyn TextBlock> = Rc::new(GenericBlock {
                    block: b,
                    back: gs.value(PName::BackGroundColor).as_color(),
                    border: gs.value(PName::LineColor).as_color(),
                });
                with_margin(b, margin_xy(1.0, 1.0, 1.0, 1.0))
            }
        };
        let circled = if cuca.show_portion(EntityPortion::CircledCharacter, entity) {
            with_margin(
                circled_character(entity, skin),
                margin_xy(4.0, 0.0, 5.0, 5.0),
            )
        } else {
            empty_block()
        };
        ClassHeader {
            circled,
            stereo,
            name,
            generic: generic_block,
        }
    }

    fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let name = self.name.calculate_dimension(sb);
        let generic = self.generic.calculate_dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        let circle = self.circled.calculate_dimension(sb);
        let width = circle.width + stereo.width.max(name.width) + generic.width;
        let height = circle
            .height
            .max(stereo.height + name.height + 10.0)
            .max(generic.height);
        Dimension::new(width, height)
    }

    fn draw(&self, ug: &UGraphic, width: f64, height: f64) {
        let sb = ug.string_bounder();
        let name = self.name.calculate_dimension(sb);
        let generic = self.generic.calculate_dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        let circle = self.circled.calculate_dimension(sb);
        let width_stereo_and_name = stereo.width.max(name.width);
        let supp = (width - circle.width - width_stereo_and_name - generic.width).max(0.0);
        let h2 = (circle.width / 4.0).min(supp * 0.1);
        let h1 = (supp - h2) / 2.0;
        let y_circle = (height - circle.height) / 2.0;
        self.circled
            .draw(&ug.apply_translate(UTranslate::new(h1, y_circle)));
        let diff = height - stereo.height - name.height;
        let x_stereo = circle.width + (width_stereo_and_name - stereo.width) / 2.0 + h1 + h2;
        self.stereo
            .draw(&ug.apply_translate(UTranslate::new(x_stereo, diff / 2.0)));
        let x_name = circle.width + (width_stereo_and_name - name.width) / 2.0 + h1 + h2;
        self.name
            .draw(&ug.apply_translate(UTranslate::new(x_name, diff / 2.0 + stereo.height)));
        if generic.width > 0.0 {
            let delta = 4.0;
            self.generic
                .draw(&ug.apply_translate(UTranslate::new(width - generic.width + delta, -delta)));
        }
    }
}

struct GenericBlock {
    block: Rc<dyn TextBlock>,
    back: HColor,
    border: HColor,
}

impl TextBlock for GenericBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.block.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug
            .apply_back_color(self.back.clone())
            .apply_color(self.border.clone());
        let dim = self.calculate_dimension(ug.string_bounder());
        ug.apply_stroke(UStroke::new(2.0, 2.0, 1.0))
            .draw(URectangle::build(dim.width, dim.height));
        self.block.draw(&ug);
    }
}

/// Foreground and background of a visibility icon in a class.
fn icon_colors(skin: &SkinParam, m: VisibilityModifier) -> (HColor, Option<HColor>) {
    let style = skin
        .style_builder()
        .merged_style(&m.style_query())
        .expect("icon style");
    let fore = style.value(PName::LineColor).as_color();
    let back = if m.is_field() {
        None
    } else {
        Some(style.value(PName::BackGroundColor).as_color())
    };
    (fore, back)
}

/// The fields or the methods of a class (`MethodsOrFieldsArea`).
struct MembersArea {
    members: Vec<Member>,
    config: FontConfiguration,
    wrap: crate::klimt::creole::LineBreakStrategy,
    skin: SkinParam,
    icon_size: i32,
    radius: f64,
}

impl MembersArea {
    fn has_small_icon(&self) -> bool {
        self.icon_size != 0
            && self
                .members
                .iter()
                .any(|m| m.visibility_modifier().is_some())
    }

    fn block(&self, m: &Member) -> Rc<dyn TextBlock> {
        let mut config = self.config.clone();
        let with_char = self.icon_size == 0;
        let mut s = m.display(with_char);
        if with_char && s.starts_with('#') {
            s.insert(0, '\u{200B}');
        }
        if m.is_abstract() {
            config = config.italic();
        }
        if m.is_static() {
            config = config.underline();
        }
        Display::with_newlines(Some(&s)).create_block(
            &config,
            HorizontalAlignment::Left,
            &self.skin.skin_simple(),
            &self.wrap,
            CreoleMode::SimpleLine,
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn only_members(&self, sb: &dyn StringBounder) -> Dimension {
        let small_icon = if self.has_small_icon() {
            self.radius + 3.0
        } else {
            0.0
        };
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for m in &self.members {
            let d = self.block(m).calculate_dimension(sb);
            x = x.max(d.width);
            y += d.height;
        }
        Dimension::new(x + small_icon, y)
    }

    fn icon_block(&self, m: &Member) -> Rc<dyn TextBlock> {
        match m.visibility_modifier() {
            None => Rc::new(crate::klimt::shape::EmptyBlock {
                width: 1.0,
                height: 1.0,
            }),
            Some(v) => {
                let (fore, back) = icon_colors(&self.skin, v);
                v.u_block(self.icon_size, fore, back, m.has_url())
            }
        }
    }
}

impl TextBlock for MembersArea {
    fn inner_position(&self, member: &str, sb: &dyn StringBounder) -> Option<InnerRect> {
        let small_icon = if self.has_small_icon() {
            (self.radius + 3.0).trunc()
        } else {
            0.0
        };
        let mut y = 0.0;
        for m in &self.members {
            let d = self.block(m).calculate_dimension(sb);
            let row = if small_icon > 0.0 {
                self.icon_block(m)
                    .calculate_dimension(sb)
                    .height
                    .max(d.height)
            } else {
                d.height
            };
            if m.raw() == member {
                let ty = y + (row - d.height) / 2.0;
                return Some(if small_icon > 0.0 {
                    let icon = self.radius + 3.0;
                    (small_icon - icon, ty, d.width + icon, d.height)
                } else {
                    (0.0, ty, d.width, d.height)
                });
            }
            y += row;
        }
        None
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.only_members(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = self.only_members(sb);
        if self.has_small_icon() {
            let col2 = (self.radius + 3.0).trunc();
            let mut y = 0.0;
            for m in &self.members {
                let icon = self.icon_block(m);
                let text = self.block(m);
                let h1 = icon.calculate_dimension(sb).height;
                let h2 = text.calculate_dimension(sb).height;
                let max = h1.max(h2);
                icon.draw(&ug.apply_translate(UTranslate::new(0.0, 2.0 + y + (max - h1) / 2.0)));
                text.draw(&ug.apply_translate(UTranslate::new(col2, y + (max - h2) / 2.0)));
                y += max;
            }
        } else {
            let used: f64 = self
                .members
                .iter()
                .map(|m| self.block(m).calculate_dimension(sb).height)
                .sum();
            let space = (dim.height - used) / (self.members.len() as f64 + 1.0);
            let mut y = space;
            for m in &self.members {
                let b = self.block(m);
                b.draw(&ug.apply_translate(UTranslate::new(0.0, y)));
                y += b.calculate_dimension(sb).height + space;
            }
        }
    }
}

struct ClassStencil(f64);

impl Stencil for ClassStencil {
    fn starting_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        self.0
    }
}

pub struct EntityImageClass {
    entity: Entity,
    header: ClassHeader,
    body: Option<Rc<dyn TextBlock>>,
    round_corner: f64,
    style: Style,
    style_header: Style,
    hidden: bool,
    same_class_width: f64,
}

impl EntityImageClass {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageClass {
        let style = style_of(skin, &entity, &queries::CLASSDIAG_CLASS);
        let style_header = style_of(skin, &entity, &queries::CLASSDIAG_CLASS_HEADER);
        let round_corner = style.value(PName::RoundCorner).as_double();
        let show_methods = cuca.show_portion(EntityPortion::Method, &entity);
        let show_fields = cuca.show_portion(EntityPortion::Field, &entity);
        let body = Self::body(&entity, skin, show_methods, show_fields, &style);
        let header = ClassHeader::new(&entity, cuca, skin);
        EntityImageClass {
            hidden: cuca.is_entity_hidden(&entity),
            same_class_width: skin.same_class_width_value(),
            entity,
            header,
            body,
            round_corner,
            style,
            style_header,
        }
    }

    fn body(
        entity: &Entity,
        skin: &SkinParam,
        show_methods: bool,
        show_fields: bool,
        style: &Style,
    ) -> Option<Rc<dyn TextBlock>> {
        let (fields, methods) = {
            let d = entity.get();
            (d.bodier.fields_to_display(), d.bodier.methods_to_display())
        };
        let config = style.font_configuration(Some(&entity.colors()));
        let make = |members: Vec<Member>| -> Rc<dyn TextBlock> {
            let area: Rc<dyn TextBlock> = Rc::new(MembersArea {
                members,
                config: config.clone(),
                wrap: style.wrap_width(),
                skin: skin.clone(),
                icon_size: skin.class_attribute_icon_size(),
                radius: skin.circled_character_radius(),
            });
            Rc::new(LineBefore {
                block: with_margin(area, margin_xy(6.0, 6.0, 4.0, 4.0)),
                separator: '\0',
                title: None,
                default_thickness: style.value(PName::LineThickness).as_double(),
            })
        };
        let empty = || -> Rc<dyn TextBlock> {
            Rc::new(crate::klimt::shape::EmptyBlock {
                width: 0.0,
                height: 0.0,
            })
        };
        match (show_fields, show_methods) {
            (true, false) => Some(make(fields)),
            (false, true) => Some(make(methods)),
            (false, false) => Some(empty()),
            (true, true) => Some(Rc::new(VerticalBlock {
                blocks: vec![make(fields), make(methods)],
                alignment: HorizontalAlignment::Left,
            })),
        }
    }
}

impl TextBlock for EntityImageClass {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim_header = self.header.dimension(sb);
        let dim_body = self
            .body
            .as_ref()
            .map_or(Dimension::new(0.0, 0.0), |b| b.calculate_dimension(sb));
        let mut width = dim_body.width.max(dim_header.width);
        let min_class_width = self.style.value(PName::MinimumWidth).as_double();
        if width < min_class_width {
            width = min_class_width;
        }
        if width < self.same_class_width {
            width = self.same_class_width;
        }
        Dimension::new(width, dim_body.height + dim_header.height)
    }

    fn inner_position(&self, member: &str, sb: &dyn StringBounder) -> Option<InnerRect> {
        let (x, y, w, h) = self.body.as_ref()?.inner_position(member, sb)?;
        Some((x, y + self.header.dimension(sb).height, w, h))
    }

    fn draw(&self, ug: &UGraphic) {
        let name = self.entity.name();
        ug.draw(UShape::Comment(format!("class {name}")));
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

impl EntityImageClass {
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
        let mut ug_header = ug.clone();
        if self.round_corner == 0.0 && back != header_back {
            ug.apply_stroke(stroke).draw(rect);
            let mut rect2 = URectangle::build(width_total, dim_header.height);
            rect2.delta_shadow = 0.0;
            ug_header = ug_header.apply_back_color(header_back);
            ug_header.apply_stroke(stroke).draw(rect2);
        } else if self.round_corner != 0.0 && back != header_back {
            ug.apply_stroke(stroke).draw(rect.clone());
            let mut rect2 =
                URectangle::build(width_total, dim_header.height).rounded(self.round_corner);
            let mut rect3 = URectangle::build(width_total, self.round_corner / 2.0);
            rect2.delta_shadow = 0.0;
            rect3.delta_shadow = 0.0;
            ug_header = ug_header
                .apply_back_color(header_back.clone())
                .apply_color(header_back);
            ug_header.apply_stroke(stroke).draw(rect2);
            ug_header
                .apply_stroke(stroke)
                .apply_translate(UTranslate::dy(dim_header.height - rect3.height))
                .draw(rect3);
            rect.delta_shadow = 0.0;
            ug.apply_stroke(stroke)
                .apply_back_color(HColor::TRANSPARENT)
                .draw(rect);
        } else {
            ug.apply_stroke(stroke).draw(rect);
        }
        self.header
            .draw(&ug_header, dim_total.width, dim_header.height);
        if let Some(body) = &self.body {
            let ug2 = ug.apply_stencil(Rc::new(ClassStencil(width_total)), stroke);
            body.draw(&ug2.apply_translate(UTranslate::dy(dim_header.height)));
        }
    }
}

impl IEntityImage for EntityImageClass {
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
fn _unused(_: &UFont) {}
