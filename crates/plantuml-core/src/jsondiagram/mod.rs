//! JSON, YAML and HCL data diagrams (`jsondiagram` package): the data as boxes of key/value
//! rows, linked from the row that holds each nested value.

mod layout;
mod style_extractor;
mod text_block;

use crate::command::common::{common_commands, HasBase};
use crate::command::{CommandControl, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::json::JsonValue;
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::style::Margins;
use crate::text::BlocLines;
use crate::yaml::Highlighted;
use layout::JsonLayout;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
pub use style_extractor::StyleExtractor;

/// A parsed data diagram (`JsonDiagram`).
pub struct JsonDiagram {
    base: DiagramBase,
    root: Option<JsonValue>,
    highlighted: Vec<Highlighted>,
    #[allow(dead_code)]
    handwritten: bool,
}

impl HasBase for JsonDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl JsonDiagram {
    fn new(
        source: UmlSource,
        diagram_type: DiagramType,
        json: Option<JsonValue>,
        highlighted: Vec<Highlighted>,
        extractor: &StyleExtractor,
    ) -> Self {
        let root = json.map(|j| match j {
            JsonValue::Array(a) if a.is_empty() => JsonValue::Array(vec![JsonValue::from_str("")]),
            JsonValue::Object(o) if o.is_empty() => JsonValue::Array(vec![JsonValue::from_str("")]),
            JsonValue::Array(_) | JsonValue::Object(_) => j,
            scalar => JsonValue::Array(vec![scalar]),
        });
        let mut this = Self {
            base: DiagramBase::new(source, diagram_type),
            root,
            highlighted,
            handwritten: extractor.is_handwritten(),
        };
        if let Some(scale) = extractor.scale() {
            let lines = BlocLines::single_string(scale);
            for cmd in common_commands::<JsonDiagram>() {
                if cmd.is_valid(&lines) == CommandControl::Ok {
                    let _ = cmd.execute(&mut this, &lines, crate::command::ParserPass::One);
                }
            }
        }
        this
    }
}

impl Diagram for JsonDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        Rc::new(JsonBlock {
            root: self.root.clone(),
            highlighted: self.highlighted.clone(),
            skin: skin.clone(),
            diagram_type: self.base.diagram_type,
            sb: sb.clone(),
            layout: OnceCell::new(),
            dimension: RefCell::new(None),
        })
    }
}

/// What is drawn: the boxes, or a message when the data cannot be read (`JsonDiagram.drawU`).
struct JsonBlock {
    root: Option<JsonValue>,
    highlighted: Vec<Highlighted>,
    skin: Rc<SkinParam>,
    diagram_type: DiagramType,
    sb: Rc<dyn StringBounder>,
    layout: OnceCell<Option<JsonLayout>>,
    dimension: RefCell<Option<Dimension>>,
}

impl JsonBlock {
    fn draw_content(&self, ug: &UGraphic) {
        let Some(root) = &self.root else {
            let text = format!(
                "Your data does not sound like {} data",
                self.diagram_type.svg_name()
            );
            let fc = FontConfiguration::black_blue_true(UFont::monospace(14));
            let block = Display::with_newlines(Some(&text)).create_block(
                &fc,
                HorizontalAlignment::Left,
                &self.skin.skin_simple(),
                &crate::klimt::creole::LineBreakStrategy::NONE,
                CreoleMode::Full,
                None,
                None,
                0.0,
                0.0,
            );
            with_margin(block, Margins::new(2.0, 5.0, 2.0, 5.0)).draw(ug);
            return;
        };
        let layout = self.layout.get_or_init(|| {
            layout::layout(
                root,
                &self.highlighted,
                &self.skin,
                self.diagram_type.style_name(),
                self.sb.as_ref(),
            )
        });
        if let Some(layout) = layout {
            layout.draw(ug);
        }
    }
}

impl TextBlock for JsonBlock {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        let cached = *self.dimension.borrow();
        if let Some(d) = cached {
            return d;
        }
        let limits = find_limits(self.sb.clone(), true, |ug| self.draw_content(ug));
        let d = Dimension::new(limits.width(), limits.height());
        *self.dimension.borrow_mut() = Some(d);
        d
    }

    fn draw(&self, ug: &UGraphic) {
        self.draw_content(ug);
    }
}

/// Reads the data lines of the source: the lines between `@start` and `@end`.
fn data_lines(extractor: &StyleExtractor) -> &[String] {
    let all = extractor.lines();
    if all.len() >= 2 {
        &all[1..all.len() - 1]
    } else {
        &[]
    }
}

fn source_strings(source: &UmlSource) -> Vec<String> {
    source
        .lines
        .iter()
        .map(|l| l.string().to_string())
        .collect()
}

/// Shared tail of the three factories: title and styles are applied unless `styled` is false.
fn finish(
    source: &UmlSource,
    diagram_type: DiagramType,
    data: Option<JsonValue>,
    highlighted: Vec<Highlighted>,
    extractor: &StyleExtractor,
    styled: bool,
) -> Box<dyn Diagram> {
    let mut diagram = JsonDiagram::new(source.clone(), diagram_type, data, highlighted, extractor);
    if styled {
        if let Err(e) = extractor.apply_styles(&mut diagram.base.skin_param) {
            log::error!("{e}");
        }
        if let Some(title) = extractor.title() {
            diagram.base.set_title(DisplayPositioned::single(
                Display::with_newlines(Some(title)),
                Some(HorizontalAlignment::Center),
                Some(VerticalAlignment::Center),
            ));
        }
    }
    Box::new(diagram)
}

pub struct JsonDiagramFactory;

impl DiagramFactory for JsonDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Json
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let extractor = StyleExtractor::new(&source_strings(source));
        let mut highlighted = Vec::new();
        let mut text = String::new();
        for line in data_lines(&extractor) {
            if line.starts_with('#') {
                if Highlighted::matches_definition(line) {
                    highlighted.extend(Highlighted::build(line));
                }
            } else {
                text.push_str(line);
                text.push('\n');
            }
        }
        let json = JsonValue::parse(&text).ok();
        Ok(finish(
            source,
            DiagramType::Json,
            json,
            highlighted,
            &extractor,
            true,
        ))
    }
}

pub struct YamlDiagramFactory;

impl DiagramFactory for YamlDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Yaml
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let extractor = StyleExtractor::new(&source_strings(source));
        let mut highlighted = Vec::new();
        let mut lines = Vec::new();
        for line in data_lines(&extractor) {
            if Highlighted::matches_definition(line) {
                highlighted.extend(Highlighted::build(line));
            } else {
                lines.push(line.clone());
            }
        }
        let yaml = crate::yaml::parser::parse_yaml(&lines).unwrap_or_else(|e| {
            log::error!("yaml: {e}");
            None
        });
        Ok(finish(
            source,
            DiagramType::Yaml,
            yaml,
            highlighted,
            &extractor,
            true,
        ))
    }
}

pub struct HclDiagramFactory;

impl DiagramFactory for HclDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Hcl
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let extractor = StyleExtractor::new(&source_strings(source));
        let hcl = crate::hcl::parse_hcl(data_lines(&extractor)).unwrap_or_else(|e| {
            log::error!("hcl: {e}");
            None
        });
        Ok(finish(
            source,
            DiagramType::Hcl,
            hcl,
            Vec::new(),
            &extractor,
            false,
        ))
    }
}

#[cfg(test)]
mod tests;
