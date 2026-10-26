//! One box of key/value rows (`TextBlockJson`).

use crate::json::JsonValue;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::style::{queries, Margins, PName, Style};
use crate::yaml::Highlighted;
use std::rc::Rc;

const MIN_WIDTH: f64 = 30.0;
const MIN_HEIGHT: f64 = 15.0;

struct Line {
    b1: Rc<dyn TextBlock>,
    b2: Option<Rc<dyn TextBlock>>,
    highlighted: Option<Highlighted>,
}

impl Line {
    fn height_of_row(&self, sb: &dyn StringBounder) -> f64 {
        let h = self.b1.calculate_dimension(sb).height;
        match &self.b2 {
            None => h,
            Some(b2) => h.max(b2.calculate_dimension(sb).height),
        }
    }
}

pub struct TextBlockJson {
    lines: Vec<Line>,
    skin: Rc<SkinParam>,
    root: JsonValue,
    diagram_name: crate::style::SName,
}

fn short_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Null => "<U+2400>".to_string(),
        JsonValue::Number(n) => n.clone(),
        JsonValue::True => "<U+2611> true".to_string(),
        JsonValue::False => "<U+2610> false".to_string(),
        _ => "   ".to_string(),
    }
}

impl TextBlockJson {
    pub fn new(
        skin: &Rc<SkinParam>,
        diagram_name: crate::style::SName,
        root: &JsonValue,
        all_highlighted: &[Highlighted],
    ) -> Self {
        let mut this = Self {
            lines: Vec::new(),
            skin: skin.clone(),
            root: root.clone(),
            diagram_name,
        };
        match root {
            JsonValue::Object(members) => {
                for (key, value) in members {
                    let highlighted = is_highlighted(key, all_highlighted);
                    let b1 = this.text_block(&this.style_to_use(true, highlighted.as_ref()), key);
                    let b2 = this.text_block(
                        &this.style_to_use(false, highlighted.as_ref()),
                        &short_string(value),
                    );
                    this.lines.push(Line {
                        b1,
                        b2: Some(b2),
                        highlighted,
                    });
                }
            }
            JsonValue::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    let highlighted = is_highlighted(&i.to_string(), all_highlighted);
                    let b = this.text_block(
                        &this.style_to_use(false, highlighted.as_ref()),
                        &short_string(value),
                    );
                    this.lines.push(Line {
                        b1: b,
                        b2: None,
                        highlighted,
                    });
                }
            }
            _ => {}
        }
        this
    }

    fn merged(&self, query: &crate::style::StyleQuery) -> Style {
        self.skin
            .style_builder()
            .merged_style(query)
            .expect("json diagram style")
    }

    fn style_to_use(&self, header: bool, highlighted: Option<&Highlighted>) -> Style {
        let stereotype = highlighted.and_then(Highlighted::stereotype);
        let query = match (header, highlighted) {
            (true, Some(_)) => queries::NODE_HEADER_HIGHLIGHT
                .add(self.diagram_name)
                .with_stereotype_obj(stereotype),
            (false, Some(_)) => queries::NODE_HIGHLIGHT
                .add(self.diagram_name)
                .with_stereotype_obj(stereotype),
            (true, None) => queries::NODE_HEADER.add(self.diagram_name),
            (false, None) => queries::NODE.add(self.diagram_name),
        };
        self.merged(&query)
    }

    fn text_block(&self, style: &Style, text: &str) -> Rc<dyn TextBlock> {
        let result = Display::with_newlines(Some(text)).create_block(
            &style.font_configuration(None),
            style.horizontal_alignment(),
            &self.skin.skin_simple(),
            &style.wrap_width(),
            CreoleMode::NoCreole,
            None,
            None,
            0.0,
            0.0,
        );
        with_margin(result, Margins::new(2.0, 5.0, 2.0, 5.0))
    }

    /// The values that get a box of their own, per row (`children`).
    pub fn children(&self) -> Vec<Option<&JsonValue>> {
        let pick = |v: &'_ JsonValue| match v {
            JsonValue::Object(_) | JsonValue::Array(_) => Some(()),
            _ => None,
        };
        match &self.root {
            JsonValue::Object(members) => members.iter().map(|(_, v)| pick(v).map(|_| v)).collect(),
            JsonValue::Array(values) => values.iter().map(|v| pick(v).map(|_| v)).collect(),
            _ => Vec::new(),
        }
    }

    pub fn keys(&self) -> Vec<String> {
        match &self.root {
            JsonValue::Object(members) => members.iter().map(|(k, _)| k.clone()).collect(),
            JsonValue::Array(values) => (0..values.len()).map(|i| i.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn is_array(&self) -> bool {
        self.root.is_array()
    }

    pub fn width_col_a(&self, sb: &dyn StringBounder) -> f64 {
        self.lines
            .iter()
            .map(|l| l.b1.calculate_dimension(sb).width)
            .fold(0.0, f64::max)
    }

    pub fn width_col_b(&self, sb: &dyn StringBounder) -> f64 {
        self.lines
            .iter()
            .filter_map(|l| l.b2.as_ref())
            .map(|b| b.calculate_dimension(sb).width)
            .fold(0.0, f64::max)
    }

    pub fn all_heights(&self, sb: &dyn StringBounder) -> Vec<f64> {
        self.lines.iter().map(|l| l.height_of_row(sb)).collect()
    }

    fn total_height(&self, sb: &dyn StringBounder) -> f64 {
        let height: f64 = self.lines.iter().map(|l| l.height_of_row(sb)).sum();
        if height == 0.0 {
            MIN_HEIGHT
        } else {
            height
        }
    }
}

fn is_highlighted(key: &str, all: &[Highlighted]) -> Option<Highlighted> {
    all.iter().find(|h| h.is_key_highlight(key)).cloned()
}

impl TextBlock for TextBlockJson {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(
            self.width_col_a(sb) + self.width_col_b(sb),
            self.total_height(sb),
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let full_dim = self.calculate_dimension(sb);
        let mut true_width = full_dim.width;
        let width_col_a = self.width_col_a(sb);
        let width_col_b = self.width_col_b(sb);
        let style_node = self.merged(&queries::NODE.add(self.diagram_name));
        let ug_node = ug
            .apply_stroke(style_node.stroke())
            .apply_color(style_node.value(PName::LineColor).as_color());
        let mut y: f64 = self.lines.iter().map(|l| l.height_of_row(sb)).sum();
        if y == 0.0 {
            y = MIN_HEIGHT;
        }
        if true_width == 0.0 {
            true_width = MIN_WIDTH;
        }
        let round = style_node.value(PName::RoundCorner).as_double();
        let full_rect = URectangle::build(true_width, y).rounded(round);
        let back = style_node.value(PName::BackGroundColor).as_color();
        ug_node
            .apply_back_color(back.clone())
            .apply_color(back)
            .draw(full_rect.clone());

        let style_separator = self.merged(&queries::NODE_SEPARATOR.add(self.diagram_name));
        let ug_separator = ug
            .apply_stroke(style_separator.stroke())
            .apply_color(style_separator.value(PName::LineColor).as_color());
        let mut y = 0.0;
        for line in &self.lines {
            let ugline = ug_separator.apply_translate(UTranslate::dy(y));
            let height = line.height_of_row(sb);
            if let Some(h) = &line.highlighted {
                let back = URectangle::build(true_width - 2.0, height).rounded(4.0);
                let style = self.merged(
                    &queries::NODE_HIGHLIGHT
                        .add(self.diagram_name)
                        .with_stereotype_obj(h.stereotype()),
                );
                let cell_back = style.value(PName::BackGroundColor).as_color();
                ugline
                    .apply_color(cell_back.clone())
                    .apply_back_color(cell_back)
                    .apply_translate(UTranslate::new(1.5, 0.0))
                    .draw(back);
            }
            if y > 0.0 {
                ugline.draw(ULine::hline(true_width));
            }
            let alignment = style_node.horizontal_alignment();
            draw_aligned(&ugline, &line.b1, alignment, width_col_a);
            if let Some(b2) = &line.b2 {
                let ug_col_b = ugline.apply_translate(UTranslate::dx(width_col_a));
                draw_aligned(&ug_col_b, b2, alignment, width_col_b);
                ug_col_b.draw(ULine::vline(height));
            }
            y += height;
        }
        ug_node.draw(full_rect);
    }
}

/// `HorizontalAlignment.draw(ug, tb, 0, 0, width)`.
fn draw_aligned(
    ug: &UGraphic,
    block: &Rc<dyn TextBlock>,
    alignment: crate::klimt::geom::HorizontalAlignment,
    width: f64,
) {
    let w = block.calculate_dimension(ug.string_bounder()).width;
    block.draw(&ug.apply_translate(UTranslate::dx(alignment.position(w, width))));
}
