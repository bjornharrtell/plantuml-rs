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
use crate::decoration::usymbol::USymbol;
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::Style;
use crate::url::Url;
use ftile::box_style::BoxStyle;
use instruction::{
    InsRef, InstructionFork, InstructionGroup, InstructionIf, InstructionNode, InstructionRepeat,
    InstructionSimple, InstructionSwitch, InstructionWhile, NodeKind,
};
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
    group_stack: Vec<InsRef>,
    fork_stack: Vec<InsRef>,
}

impl ActivityDiagram3 {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Activity),
            swimlane_strategy: None,
            swimlanes: Swimlanes::new(),
            has_url: false,
            group_stack: Vec::new(),
            fork_stack: Vec::new(),
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

    /// Opens a group or partition (`startGroup`).
    pub fn start_group(
        &mut self,
        title: Display,
        back_color: Option<HColor>,
        symbol: Rc<dyn USymbol>,
        style: Style,
    ) {
        self.manage_swimlane_strategy();
        let group = InstructionGroup::new(
            title,
            back_color,
            symbol,
            style,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let group: InsRef = Rc::new(RefCell::new(group));
        self.current().borrow_mut().add(group.clone());
        self.group_stack.push(self.current());
        self.swimlanes.set_current(group);
    }

    /// Starts a `switch` (`startSwitch`).
    pub fn start_switch(&mut self, test: Display, back_color: Option<crate::klimt::color::HColor>) {
        self.manage_swimlane_strategy();
        let switch = InstructionSwitch::new(
            test,
            back_color,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let switch: InsRef = Rc::new(RefCell::new(switch));
        self.current().borrow_mut().add(switch.clone());
        self.group_stack.push(self.current());
        self.swimlanes.set_current(switch);
    }

    /// Adds a branch to the current `switch` (`switchCase`).
    pub fn switch_case(&mut self, label: Option<Display>) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionSwitch>() {
            return CommandExecutionResult::error("Cannot find switch");
        }
        let mut current = current.borrow_mut();
        let Some(switch) = current.as_any_mut().downcast_mut::<InstructionSwitch>() else {
            return CommandExecutionResult::error("Cannot find switch");
        };
        switch.case(label);
        CommandExecutionResult::ok()
    }

    /// Closes the `switch` (`endswitch`).
    pub fn end_switch(&mut self) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionSwitch>() {
            return CommandExecutionResult::error("Cannot find switch");
        }
        {
            let mut current = current.borrow_mut();
            let Some(switch) = current.as_any_mut().downcast_mut::<InstructionSwitch>() else {
                return CommandExecutionResult::error("Cannot find switch");
            };
            switch.set_out_rendering(self.next_link_renderer());
        }
        self.reset_next_link();
        match self.group_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find switch"),
        }
    }

    /// Starts an `if` (`startIf`).
    pub fn start_if(
        &mut self,
        test: Display,
        yes: Option<Display>,
        back_color: Option<crate::klimt::color::HColor>,
    ) {
        self.manage_swimlane_strategy();
        let if_ = InstructionIf::new(
            test,
            yes,
            back_color,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let if_: InsRef = Rc::new(RefCell::new(if_));
        self.current().borrow_mut().add(if_.clone());
        self.group_stack.push(self.current());
        self.swimlanes.set_current(if_);
    }

    /// Adds an `else` / `elseif` branch to the current `if`.
    pub fn else_branch(&mut self, label: Option<Display>) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionIf>() {
            return CommandExecutionResult::error("Cannot find if");
        }
        let mut current = current.borrow_mut();
        let Some(if_) = current.as_any_mut().downcast_mut::<InstructionIf>() else {
            return CommandExecutionResult::error("Cannot find if");
        };
        if_.else_branch(label);
        CommandExecutionResult::ok()
    }

    /// Closes the `if` (`endif`).
    pub fn end_if(&mut self) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionIf>() {
            return CommandExecutionResult::error("Cannot find if");
        }
        {
            let mut current = current.borrow_mut();
            let Some(if_) = current.as_any_mut().downcast_mut::<InstructionIf>() else {
                return CommandExecutionResult::error("Cannot find if");
            };
            if_.set_out_rendering(self.next_link_renderer());
        }
        self.reset_next_link();
        match self.group_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find if"),
        }
    }

    /// Starts a `repeat` loop (`doRepeat`).
    pub fn do_repeat(
        &mut self,
        start_label: Option<Display>,
        colors: crate::klimt::color::colors::Colors,
        back_color: Option<crate::klimt::color::HColor>,
    ) {
        self.manage_swimlane_strategy();
        let repeat = InstructionRepeat::new(
            start_label,
            colors,
            back_color,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let repeat: InsRef = Rc::new(RefCell::new(repeat));
        self.current().borrow_mut().add(repeat.clone());
        self.group_stack.push(self.current());
        self.swimlanes.set_current(repeat);
    }

    /// Closes a `repeat` loop (`repeat while (...)`).
    pub fn end_repeat(&mut self, test: Display) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionRepeat>() {
            return CommandExecutionResult::error("Cannot find repeat");
        }
        {
            let mut current = current.borrow_mut();
            let Some(repeat) = current.as_any_mut().downcast_mut::<InstructionRepeat>() else {
                return CommandExecutionResult::error("Cannot find repeat");
            };
            repeat.set_test(test);
            repeat.set_out_rendering(self.next_link_renderer());
        }
        self.reset_next_link();
        match self.group_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find repeat"),
        }
    }

    /// Starts a `while` loop (`doWhile`).
    pub fn do_while(&mut self, test: Display, back_color: Option<crate::klimt::color::HColor>) {
        self.manage_swimlane_strategy();
        let while_ = InstructionWhile::new(
            test,
            back_color,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let while_: InsRef = Rc::new(RefCell::new(while_));
        self.current().borrow_mut().add(while_.clone());
        self.group_stack.push(self.current());
        self.swimlanes.set_current(while_);
    }

    /// Closes a `while` loop (`endwhile`).
    pub fn end_while(&mut self) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionWhile>() {
            return CommandExecutionResult::error("Cannot find while");
        }
        {
            let mut current = current.borrow_mut();
            let Some(while_) = current.as_any_mut().downcast_mut::<InstructionWhile>() else {
                return CommandExecutionResult::error("Cannot find while");
            };
            while_.set_out_rendering(self.next_link_renderer());
        }
        self.reset_next_link();
        match self.group_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find while"),
        }
    }

    /// Starts parallel branches (`fork`).
    pub fn fork(&mut self, colors: crate::klimt::color::colors::Colors) {
        self.manage_swimlane_strategy();
        let fork = InstructionFork::new(
            colors,
            self.swimlanes.current_swimlane(),
            self.next_link_renderer(),
        );
        let fork: InsRef = Rc::new(RefCell::new(fork));
        self.current().borrow_mut().add(fork.clone());
        self.fork_stack.push(self.current());
        self.swimlanes.set_current(fork);
        self.reset_next_link();
    }

    /// Starts the next parallel branch (`forkAgain`).
    pub fn fork_again(&mut self) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionFork>() {
            return CommandExecutionResult::error("Cannot find fork");
        }
        let mut current = current.borrow_mut();
        let Some(fork) = current.as_any_mut().downcast_mut::<InstructionFork>() else {
            return CommandExecutionResult::error("Cannot find fork");
        };
        fork.set_out_rendering(self.next_link_renderer());
        fork.fork_again();
        self.reset_next_link();
        CommandExecutionResult::ok()
    }

    /// Closes the parallel branches (`endFork`).
    pub fn end_fork(&mut self) -> CommandExecutionResult {
        let current = self.current();
        if !current.borrow().as_any().is::<InstructionFork>() {
            return CommandExecutionResult::error("Cannot find fork");
        }
        let mut current = current.borrow_mut();
        let Some(fork) = current.as_any_mut().downcast_mut::<InstructionFork>() else {
            return CommandExecutionResult::error("Cannot find fork");
        };
        fork.set_out_rendering(self.next_link_renderer());
        drop(current);
        self.reset_next_link();
        match self.fork_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find fork"),
        }
    }

    /// Closes the current group or partition (`closeGroup`).
    pub fn close_group(&mut self) -> CommandExecutionResult {
        let is_group = self.current().borrow().as_any().is::<InstructionGroup>();
        if !is_group {
            return CommandExecutionResult::error("Cannot find group");
        }
        match self.group_stack.pop() {
            Some(parent) => {
                self.swimlanes.set_current(parent);
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Cannot find group"),
        }
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
