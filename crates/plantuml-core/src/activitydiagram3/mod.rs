//! Activity diagrams, new syntax (`activitydiagram3` package).
//!
//! Commands build a tree of [`instruction::Instruction`]s; at drawing time the tree is turned
//! into [`ftile::Ftile`]s by a chain of [`ftile::factory::FtileFactory`] and drawn by
//! [`swimlanes::SwimlanesBlock`].
//!
//! Extension points for the constructs still missing:
//! - a new `Instruction` implementation in `instruction.rs` (groups keep a `Weak` parent and are
//!   made current with `Swimlanes::set_current`);
//! - a new method on `FtileFactory` plus a delegator wrapped in `default_factory`;
//! - new tiles in `ftile/` (the composition helpers in `ftile/wrap.rs` are reusable);
//! - commands in `commands.rs`, in the order of `ActivityDiagramFactory3`.

pub mod commands;
pub mod ftile;
pub mod instruction;
pub mod rainbow;
pub mod stereogroup;
pub mod swimlanes;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::creole::display::Display;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::url::Url;
use ftile::box_style::BoxStyle;
use instruction::{InsRef, InstructionNode, InstructionSimple, NodeKind};
use rainbow::{LinkRendering, Rainbow};
use std::cell::RefCell;
use std::rc::Rc;
use stereogroup::Stereogroup;
use swimlanes::Swimlanes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwimlaneStrategy {
    Forbidden,
    Allowed,
}

pub struct ActivityDiagram3 {
    base: DiagramBase,
    swimlane_strategy: Option<SwimlaneStrategy>,
    swimlanes: Swimlanes,
    has_url: bool,
}

impl ActivityDiagram3 {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Activity),
            swimlane_strategy: None,
            swimlanes: Swimlanes::new(),
            has_url: false,
        }
    }

    fn manage_swimlane_strategy(&mut self) {
        if self.swimlane_strategy.is_none() {
            self.swimlane_strategy = Some(SwimlaneStrategy::Forbidden);
        }
    }

    /// `swimlane |name|`: only allowed before the first instruction.
    pub fn swimlane(
        &mut self,
        name: &str,
        colors: Option<crate::klimt::color::colors::Colors>,
        label: Option<Display>,
    ) -> CommandExecutionResult {
        if self.swimlane_strategy.is_none() {
            self.swimlane_strategy = Some(SwimlaneStrategy::Allowed);
        }
        if self.swimlane_strategy == Some(SwimlaneStrategy::Forbidden) {
            return CommandExecutionResult::error(
                "This swimlane must be defined at the start of the diagram.",
            );
        }
        self.swimlanes.swimlane(name, colors, label);
        CommandExecutionResult::ok()
    }

    fn current(&self) -> InsRef {
        self.swimlanes.current()
    }

    fn next_link_renderer(&self) -> LinkRendering {
        self.swimlanes.next_link_renderer()
    }

    fn reset_next_link(&mut self) {
        self.swimlanes.set_next_link_renderer(LinkRendering::none());
    }

    pub fn skin(&self) -> &SkinParam {
        &self.base.skin_param
    }

    pub fn add_activity(
        &mut self,
        activity: Display,
        box_style: BoxStyle,
        url: Option<Url>,
        stereogroup: &Stereogroup,
    ) -> CommandExecutionResult {
        let colors = stereogroup.inner_colors();
        self.manage_swimlane_strategy();
        let has_url = url.is_some() || activity.has_url();
        let ins = InstructionSimple::new(
            activity,
            self.next_link_renderer(),
            self.swimlanes.current_swimlane(),
            box_style,
            url,
            colors,
            stereogroup.build_stereotype(),
            self.base.skin_param.style_builder(),
        );
        let ins: InsRef = Rc::new(RefCell::new(ins));
        let added = self.current().borrow_mut().add(ins);
        if !added.is_ok() {
            return added;
        }
        self.reset_next_link();
        if has_url {
            self.has_url = true;
        }
        CommandExecutionResult::ok()
    }

    fn add_node(&mut self, kind: NodeKind, colors: crate::klimt::color::colors::Colors) {
        self.manage_swimlane_strategy();
        let ins = InstructionNode::new(
            kind,
            colors,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let ins: InsRef = Rc::new(RefCell::new(ins));
        self.current().borrow_mut().add(ins);
        if kind == NodeKind::Start {
            self.reset_next_link();
        }
    }

    pub fn start(&mut self, colors: crate::klimt::color::colors::Colors) {
        self.add_node(NodeKind::Start, colors);
    }

    pub fn stop(&mut self, colors: crate::klimt::color::colors::Colors) {
        self.add_node(NodeKind::Stop, colors);
    }

    pub fn end(&mut self, colors: crate::klimt::color::colors::Colors) {
        self.add_node(NodeKind::End, colors);
    }

    /// Color of the next arrow (`setColorNextArrow`).
    pub fn set_color_next_arrow(&mut self, color: Rainbow) {
        if color.size() == 0 {
            return;
        }
        self.swimlanes
            .set_next_link_renderer(LinkRendering::create(color));
    }

    /// Text of the next arrow (`setLabelNextArrow`).
    pub fn set_label_next_arrow(&mut self, label: Display) {
        let link = self.next_link_renderer().with_display(label);
        self.swimlanes.set_next_link_renderer(link);
    }

    pub fn has_url(&self) -> bool {
        self.has_url
    }
}

impl Diagram for ActivityDiagram3 {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        self.swimlanes.text_block(skin)
    }
}

impl HasBase for ActivityDiagram3 {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for ActivityDiagram3 {}

pub struct ActivityDiagramFactory3;

impl DiagramFactory for ActivityDiagramFactory3 {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Activity
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ActivityDiagram3::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests;
