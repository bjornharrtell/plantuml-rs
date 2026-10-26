//! A description element (`EntityImageDescription`): an arbitrary symbol around a label.

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::Entity;
use crate::cucadiagram::CucaDiagram;
use crate::decoration::usymbol::USymbol;
use crate::klimt::color::colors::ColorType;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, UShape, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Guillemet;
use crate::style::{queries, Margins, PName, SName, Style};
use std::any::Any;
use std::rc::Rc;

fn style_of(skin: &SkinParam, entity: &Entity, q: &crate::style::StyleQuery) -> Style {
    let q = q.with_stereotype_obj(entity.stereotype().as_ref());
    skin.style_builder()
        .merged_style(&q)
        .unwrap_or_else(|| panic!("no style for description"))
}

fn empty_block() -> Rc<dyn TextBlock> {
    Rc::new(crate::klimt::shape::EmptyBlock {
        width: 0.0,
        height: 0.0,
    })
}

/// The symbol chosen for an entity, falling back to the component style.
pub fn entity_symbol(entity: &Entity, skin: &SkinParam) -> Rc<dyn USymbol> {
    entity
        .usymbol()
        .unwrap_or_else(|| skin.component_style().to_usymbol())
}

fn symbol_shape(symbol: &Rc<dyn USymbol>, hide_text: bool) -> ShapeType {
    let names = symbol.snames();
    if names
        .iter()
        .any(|n| matches!(n, SName::Folder | SName::Package | SName::Frame | SName::Node))
    {
        ShapeType::Folder
    } else if names.iter().any(|n| matches!(n, SName::Hexagon)) {
        ShapeType::Hexagon
    } else if names
        .iter()
        .any(|n| matches!(n, SName::Usecase | SName::Business))
    {
        ShapeType::Oval
    } else {
        let _ = hide_text;
        ShapeType::Rectangle
    }
}

pub struct EntityImageDescription {
    entity: Entity,
    desc: Rc<dyn TextBlock>,
    stereo: Rc<dyn TextBlock>,
    as_small: Rc<dyn TextBlock>,
    hide_text: bool,
    shape_type: ShapeType,
    hidden: bool,
}

impl EntityImageDescription {
    pub fn new(entity: Entity, cuca: &CucaDiagram, skin: &SkinParam) -> EntityImageDescription {
        let symbol = entity_symbol(&entity, skin);
        let hide_text = symbol.snames().contains(&SName::Interface);
        let shape_type = symbol_shape(&symbol, hide_text);

        let mut element = queries::ELEMENT.clone().add(SName::ComponentDiagram);
        element = element.add_all(&symbol.snames());
        let style = style_of(skin, &entity, &element);
        let style_title = style_of(skin, &entity, &element.clone().add(SName::Title));
        let style = style.override_colors(&entity.colors());
        let style_title = style_title.override_colors(&entity.colors());
        let skin_simple = skin.skin_simple();

        let desc = entity
            .display()
            .create_block(
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
        let name = Display::with_newlines(Some(&entity.name()))
            .create_block(
                &style_title.font_configuration(Some(&entity.colors())),
                HorizontalAlignment::Center,
                &skin_simple,
                &style_title.wrap_width(),
                CreoleMode::Full,
                None,
                None,
                0.0,
                0.0,
            );

        let stereo = match (&entity.stereotype(), cuca.visible_stereotype_labels(&entity)) {
            (Some(st), Some(labels))
                if st.label(Guillemet::DoubleComparator).is_some() && !labels.is_empty() =>
            {
                let q = element
                    .clone()
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

        let back = entity
            .colors()
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style_title.value(PName::BackGroundColor).as_color());
        let fore = style_title.value(PName::LineColor).as_color();
        let fashion = Fashion::new(Some(back), Some(fore))
            .with_stroke(entity.colors().mute_stroke(style_title.stroke()))
            .with_delta_shadow(style_title.shadowing())
            .with_corner(
                style_title.value(PName::RoundCorner).as_double(),
                style_title.value(PName::DiagonalCorner).as_double(),
            );

        let as_small = if hide_text {
            symbol.as_small(
                Some(empty_block()),
                empty_block(),
                empty_block(),
                fashion,
                HorizontalAlignment::Center,
            )
        } else {
            symbol.as_small(
                Some(name.clone()),
                desc.clone(),
                stereo.clone(),
                fashion,
                HorizontalAlignment::Center,
            )
        };

        EntityImageDescription {
            hidden: cuca.is_entity_hidden(&entity),
            entity,
            desc,
            stereo,
            as_small,
            hide_text,
            shape_type,
        }
    }
}

impl TextBlock for EntityImageDescription {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.as_small.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let name = self.entity.name();
        ug.draw(UShape::Comment(format!("entity {name}")));
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
        self.as_small.draw(ug);
        if self.hide_text {
            let sb = ug.string_bounder();
            let small = self.as_small.calculate_dimension(sb);
            let desc = self.desc.calculate_dimension(sb);
            let stereo = self.stereo.calculate_dimension(sb);
            let space = 8.0;
            let posx1 = (small.width - desc.width) / 2.0;
            self.desc.draw(
                &ug.apply_translate(UTranslate::new(posx1, space + small.height)),
            );
            let posx2 = (small.width - stereo.width) / 2.0;
            self.stereo
                .draw(&ug.apply_translate(UTranslate::new(posx2, -space - stereo.height)));
        }
        ug.close_group();
    }
}

impl IEntityImage for EntityImageDescription {
    fn shape_type(&self) -> ShapeType {
        self.shape_type
    }

    fn shield(&self, sb: &dyn StringBounder) -> Margins {
        if !self.hide_text {
            return Margins::default();
        }
        let small = self.as_small.calculate_dimension(sb);
        let desc = self.desc.calculate_dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        let x = desc.width.max(stereo.width);
        let mut supp_x = x - small.width;
        if supp_x < 1.0 {
            supp_x = 1.0;
        }
        let y = 1.0f64.max(desc.height).max(stereo.height);
        Margins::new(y, supp_x / 2.0, y, supp_x / 2.0)
    }

    fn overscan_x(&self, sb: &dyn StringBounder) -> f64 {
        if !self.hide_text {
            return 0.0;
        }
        let small = self.as_small.calculate_dimension(sb);
        let desc = self.desc.calculate_dimension(sb);
        let stereo = self.stereo.calculate_dimension(sb);
        let posx1 = (small.width - desc.width) / 2.0;
        let posx2 = (small.width - stereo.width) / 2.0;
        (-posx1).max(-posx2).max(0.0)
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
