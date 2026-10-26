//! EBNF diagrams (`ebnf` package): syntax diagrams of `@startebnf` blocks.
//!
//! One rule is `name = expression ;`, where `expression` is a sequence of identifiers, terminal
//! strings (`"..."`, `'...'`), special sequences (`?...?`), concatenations (`,`), alternations
//! (`|`), optional parts (`[ ]`) and repetitions (`{ }` and `}-`). Each rule becomes a tile tree
//! drawn as a syntax diagram.

mod brace;
mod commands;
mod corner_curved;
mod ebnf_engine;
mod ebnf_expression;
mod etile;
mod etile_alternation;
mod etile_box;
mod etile_concatenation;
mod etile_empty;
mod etile_one_or_more;
mod etile_optional2;
mod etile_with_circles;
mod etile_zero_or_more;
mod note;
mod shunting_yard;
mod symbol;
mod title_box;
mod token;
mod zigzag;

#[cfg(test)]
mod tests;

use self::ebnf_engine::syntax_error;
use self::ebnf_expression::EbnfExpression;
use self::etile::get_style_query;
use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{TextBlock, VerticalBlock};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use crate::text::BlocLines;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The style of every tile and text of the diagram (`ETile.getStyleQuery`).
///
/// The bundled skin has no `ebnf` block, so the merged style of the `element` ancestor is used;
/// when even that is missing an empty style falls back to the default property values.
fn style(skin: &SkinParam) -> Style {
    skin.style_builder()
        .merged_style(&get_style_query())
        .unwrap_or_else(|| Style::new(get_style_query(), BTreeMap::new()))
}

/// One entry of the diagram (`TextBlockable`): a rule or a free comment.
enum Expression {
    Rule(EbnfExpression),
    Note(String),
}

impl Expression {
    fn text_block(&self, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        match self {
            Expression::Rule(rule) => rule.text_block(skin),
            Expression::Note(text) => {
                let fc = style(skin).font_configuration(None);
                note::create(skin, &fc, text, Margins::new(5.0, 0.0, 15.0, 0.0))
            }
        }
    }
}

/// An EBNF diagram: a list of rules stacked vertically (`PSystemEbnf`).
pub struct PSystemEbnf {
    base: DiagramBase,
    expressions: Vec<Expression>,
}

impl PSystemEbnf {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Ebnf),
            expressions: Vec::new(),
        }
    }

    /// Parses one rule (`addBlocLines`); an error aborts the parsing of the diagram.
    pub fn add_bloc_lines(
        &mut self,
        lines: &BlocLines,
        comment_above: Option<String>,
        comment_below: Option<String>,
    ) -> CommandExecutionResult {
        match EbnfExpression::create(lines, comment_above, comment_below) {
            Ok(expression) => {
                self.expressions.push(Expression::Rule(expression));
                CommandExecutionResult::ok()
            }
            Err(message) => CommandExecutionResult::error(message),
        }
    }

    /// Adds a free comment between two rules (`addNote`).
    pub fn add_note(&mut self, note: String) {
        self.expressions.push(Expression::Note(note));
    }
}

impl Diagram for PSystemEbnf {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        if self.expressions.is_empty() {
            let fc = style(skin).font_configuration(None);
            return syntax_error(skin, &fc);
        }
        let blocks: Vec<Rc<dyn TextBlock>> = self
            .expressions
            .iter()
            .map(|e| e.text_block(skin))
            .collect();
        if blocks.len() == 1 {
            return blocks.into_iter().next().expect("one block");
        }
        Rc::new(VerticalBlock {
            blocks,
            alignment: HorizontalAlignment::Left,
        })
    }
}

impl HasBase for PSystemEbnf {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for PSystemEbnf {}

/// Builds EBNF diagrams (`PSystemEbnfFactory`).
pub struct EbnfDiagramFactory;

impl DiagramFactory for EbnfDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Ebnf
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            PSystemEbnf::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
