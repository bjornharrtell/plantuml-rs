//! Salt (wireframe) diagrams (`salt` package).

mod element;
mod parser;

use crate::command::common::HasBase;
use crate::command::{
    create_system, Command, CommandDiagram, CommandExecutionResult, ErrorUml, SingleLineCommand,
    UmlSource,
};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::regex::IRegex;
use crate::skin::SkinParam;
use crate::style::Margins;
use element::Elmt;
use std::cell::RefCell;
use std::rc::Rc;

/// The diagram being built by commands (`PSystemSalt`).
pub struct SaltDiagram {
    pub base: DiagramBase,
    data: Vec<String>,
    element: RefCell<Option<Elmt>>,
}

impl SaltDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Salt),
            data: Vec::new(),
            element: RefCell::new(None),
        }
    }

    /// Lines that are not sprite definitions (`manageSprite`); sprites are not supported.
    fn data_without_sprites(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut it = self.data.iter();
        while let Some(s) = it.next() {
            if s == "hide stereotype" || s.starts_with("skinparam ") || s.starts_with("scale ") {
                continue;
            }
            if s.starts_with("sprite $") {
                log::warn!("sprites are not supported in salt diagrams");
                for l in it.by_ref() {
                    if l == "}" {
                        break;
                    }
                }
                continue;
            }
            result.push(s.clone());
        }
        result
    }
}

struct SaltBlock {
    element: Elmt,
    size: Dimension,
}

impl TextBlock for SaltBlock {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        self.size
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_color(HColor::BLACK);
        self.element.draw(&ug, 0, self.size);
        self.element.draw(&ug, 1, self.size);
    }
}

impl Diagram for SaltDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn default_margins(&self) -> Margins {
        Margins::same(5.0)
    }

    fn text_block(&self, sb: &Rc<dyn StringBounder>, _skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        let Some(element) = self.element.borrow().clone() else {
            return Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            });
        };
        let preferred = element.preferred_dimension(sb.as_ref(), 0.0, 0.0);
        let size = if element.may_draw_beyond_preferred_dimension() {
            let m = find_limits(sb.clone(), true, |ug| {
                element.draw(ug, 0, preferred);
                element.draw(ug, 1, preferred);
            });
            preferred.at_least(m.max_x, m.max_y)
        } else {
            preferred
        };
        Rc::new(SaltBlock { element, size })
    }
}

impl HasBase for SaltDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for SaltDiagram {
    /// Builds the widgets, reporting a malformed source (`createElement`).
    fn check_final_error(&self) -> Option<String> {
        let lines = self.data_without_sprites();
        match parser::Parser::new(&lines).create_root() {
            Ok(e) => {
                *self.element.borrow_mut() = Some(e);
                None
            }
            Err(e) => Some(e),
        }
    }
}

/// Any line is salt content (`CommandAnything`).
fn command_anything() -> Box<dyn Command<SaltDiagram>> {
    Box::new(
        SingleLineCommand::new(
            IRegex::concat(vec![
                IRegex::start(),
                IRegex::named("ALL", "(.*)"),
                IRegex::end(),
            ]),
            |d: &mut SaltDiagram, _, arg, _| {
                let s = arg.get("ALL", 0).unwrap_or("");
                if d.data.is_empty() && s.trim() == "salt" {
                    return CommandExecutionResult::error("This is not needed anymore.");
                }
                d.data.push(s.to_string());
                CommandExecutionResult::ok()
            },
        )
        .no_trim(),
    )
}

pub struct SaltDiagramFactory;

impl DiagramFactory for SaltDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Salt
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let mut cmds: Vec<Box<dyn Command<SaltDiagram>>> =
            crate::command::common::common_commands();
        cmds.push(command_anything());
        let diagram = create_system(&cmds, SaltDiagram::from_source(source.clone()), source)?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests;
