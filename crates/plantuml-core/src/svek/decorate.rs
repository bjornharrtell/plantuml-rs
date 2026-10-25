//! Text added above and below an image (`DecorateEntityImage`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::TextBlock;
use std::rc::Rc;

/// Attributes of an SVG group around a decoration (`UGroup`).
pub type Group = Vec<(&'static str, String)>;

pub struct DecorateEntityImage {
    original: Rc<dyn TextBlock>,
    group1: Option<Group>,
    horizontal1: HorizontalAlignment,
    text1: Option<Rc<dyn TextBlock>>,
    group2: Option<Group>,
    horizontal2: HorizontalAlignment,
    text2: Option<Rc<dyn TextBlock>>,
}

impl DecorateEntityImage {
    pub fn add_top(
        group: Group,
        original: Rc<dyn TextBlock>,
        text: Rc<dyn TextBlock>,
        horizontal: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        Rc::new(DecorateEntityImage {
            original,
            group1: Some(group),
            horizontal1: horizontal,
            text1: Some(text),
            group2: None,
            horizontal2: HorizontalAlignment::Center,
            text2: None,
        })
    }

    pub fn add_bottom(
        group: Group,
        original: Rc<dyn TextBlock>,
        text: Rc<dyn TextBlock>,
        horizontal: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        Rc::new(DecorateEntityImage {
            original,
            group1: None,
            horizontal1: HorizontalAlignment::Center,
            text1: None,
            group2: Some(group),
            horizontal2: horizontal,
            text2: Some(text),
        })
    }

    pub fn add(
        group: Group,
        original: Rc<dyn TextBlock>,
        text: Rc<dyn TextBlock>,
        horizontal: HorizontalAlignment,
        vertical: VerticalAlignment,
    ) -> Rc<dyn TextBlock> {
        if vertical == VerticalAlignment::Top {
            Self::add_top(group, original, text, horizontal)
        } else {
            Self::add_bottom(group, original, text, horizontal)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_top_and_bottom(
        original: Rc<dyn TextBlock>,
        group1: Group,
        text1: Option<Rc<dyn TextBlock>>,
        horizontal1: HorizontalAlignment,
        group2: Group,
        text2: Option<Rc<dyn TextBlock>>,
        horizontal2: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        Rc::new(DecorateEntityImage {
            original,
            group1: Some(group1),
            horizontal1,
            text1,
            group2: Some(group2),
            horizontal2,
            text2,
        })
    }

    fn text_dim(text: &Option<Rc<dyn TextBlock>>, sb: &dyn StringBounder) -> Dimension {
        text.as_ref()
            .map_or(Dimension::new(0.0, 0.0), |t| t.calculate_dimension(sb))
    }

    fn text_x(dim_text: Dimension, dim_total: Dimension, h: HorizontalAlignment) -> f64 {
        match h {
            HorizontalAlignment::Center => (dim_total.width - dim_text.width) / 2.0,
            HorizontalAlignment::Left => 0.0,
            HorizontalAlignment::Right => dim_total.width - dim_text.width,
        }
    }
}

impl TextBlock for DecorateEntityImage {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let original = self.original.calculate_dimension(sb);
        let dim1 = Self::text_dim(&self.text1, sb);
        let dim2 = Self::text_dim(&self.text2, sb);
        original.merge_tb(dim1.merge_tb(dim2))
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim_original = self.original.calculate_dimension(sb);
        let dim1 = Self::text_dim(&self.text1, sb);
        let dim2 = Self::text_dim(&self.text2, sb);
        let total = self.calculate_dimension(sb);
        let y_image = dim1.height;
        let y_text2 = y_image + dim_original.height;
        let x_image = (total.width - dim_original.width) / 2.0;
        if let Some(text1) = &self.text1 {
            let x = Self::text_x(dim1, total, self.horizontal1);
            if let Some(g) = &self.group1 {
                ug.start_group(g);
            }
            text1.draw(&ug.apply_translate(UTranslate::dx(x)));
            if self.group1.is_some() {
                ug.close_group();
            }
        }
        self.original
            .draw(&ug.apply_translate(UTranslate::new(x_image, y_image)));
        if let Some(text2) = &self.text2 {
            let x = Self::text_x(dim2, total, self.horizontal2);
            if let Some(g) = &self.group2 {
                ug.start_group(g);
            }
            text2.draw(&ug.apply_translate(UTranslate::new(x, y_text2)));
            if self.group2.is_some() {
                ug.close_group();
            }
        }
    }

    fn backcolor(&self) -> Option<HColor> {
        self.original.backcolor()
    }
}
