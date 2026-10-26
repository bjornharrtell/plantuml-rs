//! What the commands build: the tree of instructions (`Instruction` and its implementations).
//!
//! Instructions are shared (`InsRef`) because the parser keeps a pointer to the group being
//! filled (`Swimlanes.currentInstruction`) while the root owns the whole tree. Compound
//! instructions added later (`if`, `while`, `repeat`, `fork`, groups...) keep a `Weak`
//! parent to give the current position back when they end.

use super::ftile::box_style::BoxStyle;
use super::ftile::factory::{FtileFactory, FtileRef};
use super::ftile::leaf::FtileEmpty;
use super::ftile::wrap::FtileDecorate;
use super::rainbow::LinkRendering;
use super::swimlanes::Swimlane;
use crate::command::CommandExecutionResult;
use crate::klimt::color::colors::Colors;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::decoration::usymbol::USymbol;
use crate::stereo::Stereotype;
use crate::style::{Style, StyleBuilder};
use crate::url::Url;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

pub type InsRef = Rc<RefCell<dyn Instruction>>;

pub trait Instruction {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef;

    /// Adds a child; only groups accept one.
    fn add(&mut self, _other: InsRef) -> CommandExecutionResult {
        panic!("not a group");
    }

    /// `kill`: ends the flow after this instruction.
    fn kill(&mut self) -> bool {
        false
    }

    fn in_link_rendering(&self) -> LinkRendering;

    fn contains_break(&self) -> bool {
        false
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>>;

    fn swimlane_in(&self) -> Option<Rc<Swimlane>>;

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in()
    }

    /// The enclosing group, if any.
    fn parent(&self) -> Option<InsRef> {
        None
    }

    fn as_any(&self) -> &dyn Any;

    /// Mutable access for the commands that fill a compound instruction.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        panic!("not mutable")
    }
}

fn single(s: &Option<Rc<Swimlane>>) -> Vec<Rc<Swimlane>> {
    s.iter().cloned().collect()
}

/// A sequence of instructions (`InstructionList`).
pub struct InstructionList {
    all: Vec<InsRef>,
    default_swimlane: Option<Rc<Swimlane>>,
    out_link_rendering: Option<LinkRendering>,
}

impl InstructionList {
    pub fn empty() -> Self {
        Self::new(None)
    }

    pub fn new(default_swimlane: Option<Rc<Swimlane>>) -> Self {
        Self {
            all: Vec::new(),
            default_swimlane,
            out_link_rendering: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    pub fn last(&self) -> Option<InsRef> {
        self.all.last().cloned()
    }

    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        self.out_link_rendering = Some(link);
    }
}

impl Instruction for InstructionList {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        if self.all.is_empty() {
            let mut result: FtileRef =
                FtileEmpty::new(&factory.skin(), 0.0, 0.0, self.default_swimlane.clone());
            if let Some(out) = &self.out_link_rendering {
                result = factory.decorate_in(result, out.clone());
            }
            return result;
        }
        let mut breaks = Vec::new();
        let mut result: Option<FtileRef> = None;
        for ins in &self.all {
            let ins = ins.borrow();
            let mut cur = ins.create_ftile(factory);
            breaks.extend(cur.welding_points());
            if !ins.in_link_rendering().is_none() {
                cur = factory.decorate_in(cur, ins.in_link_rendering());
            }
            result = Some(match result {
                None => cur,
                Some(prev) => factory.assembly(prev, cur),
            });
        }
        let mut result = result.expect("not empty");
        if let Some(out) = &self.out_link_rendering {
            result = factory.decorate_out(result, out.clone());
        }
        if !breaks.is_empty() {
            result = FtileDecorate::welding(result, breaks);
        }
        result
    }

    fn add(&mut self, other: InsRef) -> CommandExecutionResult {
        self.all.push(other);
        CommandExecutionResult::ok()
    }

    fn kill(&mut self) -> bool {
        match self.all.last() {
            None => false,
            Some(last) => last.borrow_mut().kill(),
        }
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.all
            .first()
            .map_or_else(LinkRendering::none, |i| i.borrow().in_link_rendering())
    }

    fn contains_break(&self) -> bool {
        self.all.iter().any(|i| i.borrow().contains_break())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut r: Vec<Rc<Swimlane>> = Vec::new();
        for i in &self.all {
            for s in i.borrow().swimlanes() {
                if !r.iter().any(|x| Rc::ptr_eq(x, &s)) {
                    r.push(s);
                }
            }
        }
        r
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.default_swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        let all = self.swimlanes();
        match all.len() {
            0 => None,
            1 => all.into_iter().next(),
            _ => self.all.last().and_then(|l| l.borrow().swimlane_out()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An action (`InstructionSimple`).
pub struct InstructionSimple {
    killed: bool,
    label: Display,
    colors: Colors,
    in_link_rendering: LinkRendering,
    box_style: BoxStyle,
    url: Option<Url>,
    stereotype: Option<Stereotype>,
    style_builder: StyleBuilder,
    swimlane: Option<Rc<Swimlane>>,
}

impl InstructionSimple {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: Display,
        in_link_rendering: LinkRendering,
        swimlane: Option<Rc<Swimlane>>,
        box_style: BoxStyle,
        url: Option<Url>,
        colors: Colors,
        stereotype: Option<Stereotype>,
        style_builder: StyleBuilder,
    ) -> Self {
        Self {
            killed: false,
            label,
            colors,
            in_link_rendering,
            box_style,
            url,
            stereotype,
            style_builder,
            swimlane,
        }
    }

    pub fn is_killed(&self) -> bool {
        self.killed
    }
}

impl Instruction for InstructionSimple {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let mut result = factory.activity(
            &self.label,
            self.swimlane.clone(),
            self.box_style,
            &self.colors,
            self.stereotype.clone(),
            Some(self.style_builder.clone()),
        );
        if let Some(url) = &self.url {
            result = factory.add_url(result, url);
        }
        result
    }

    fn kill(&mut self) -> bool {
        self.killed = true;
        true
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        single(&self.swimlane)
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Which node a [`InstructionNode`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Start,
    Stop,
    End,
}

/// `start`, `stop` and `end` (`InstructionStart`, `InstructionStop`, `InstructionEnd`).
pub struct InstructionNode {
    kind: NodeKind,
    colors: Colors,
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
}

impl InstructionNode {
    pub fn new(
        kind: NodeKind,
        colors: Colors,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            kind,
            colors,
            swimlane,
            in_link_rendering,
        }
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }
}

impl Instruction for InstructionNode {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        match self.kind {
            NodeKind::Start => factory.start(self.swimlane.clone(), &self.colors),
            NodeKind::Stop => factory.stop(self.swimlane.clone(), &self.colors),
            NodeKind::End => factory.end(self.swimlane.clone(), &self.colors),
        }
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        single(&self.swimlane)
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Parallel branches of a fork (`InstructionFork`).
pub struct InstructionFork {
    branches: Vec<InstructionList>,
    colors: Colors,
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
}

impl InstructionFork {
    pub fn new(
        colors: Colors,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            branches: vec![InstructionList::new(swimlane.clone())],
            colors,
            swimlane,
            in_link_rendering,
        }
    }

    /// Ends the current branch and starts a new one (`forkAgain`).
    pub fn fork_again(&mut self) {
        self.branches
            .push(InstructionList::new(self.swimlane.clone()));
    }

    /// Remembers the arrow leaving the current branch (`manageOutRendering`).
    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        if let Some(last) = self.branches.last_mut() {
            last.set_out_rendering(link);
        }
    }
}

impl Instruction for InstructionFork {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let branches = self
            .branches
            .iter()
            .map(|b| b.create_ftile(factory))
            .collect();
        super::ftile::fork::FtileParallel::new(
            branches,
            self.colors.clone(),
            factory.skin(),
            self.in_link_rendering.clone(),
        )
    }

    fn add(&mut self, other: InsRef) -> CommandExecutionResult {
        self.branches
            .last_mut()
            .expect("at least one branch")
            .add(other)
    }

    fn kill(&mut self) -> bool {
        self.branches
            .last_mut()
            .is_some_and(|b| b.kill())
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn contains_break(&self) -> bool {
        self.branches.iter().any(|b| b.contains_break())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        for branch in &self.branches {
            for lane in branch.swimlanes() {
                if !result.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                    result.push(lane);
                }
            }
        }
        result
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.branches.last().and_then(|b| b.swimlane_out())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A group or partition around a sequence of instructions (`InstructionGroup`).
pub struct InstructionGroup {
    list: InstructionList,
    title: Display,
    back_color: Option<HColor>,
    symbol: Rc<dyn USymbol>,
    style: Style,
    in_link_rendering: LinkRendering,
}

impl InstructionGroup {
    pub fn new(
        title: Display,
        back_color: Option<HColor>,
        symbol: Rc<dyn USymbol>,
        style: Style,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            list: InstructionList::new(swimlane),
            title,
            back_color,
            symbol,
            style,
            in_link_rendering,
        }
    }
}

impl Instruction for InstructionGroup {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let inner = self.list.create_ftile(factory);
        super::ftile::group::FtileGroup::new(
            inner,
            self.title.clone(),
            self.back_color.clone(),
            self.symbol.clone(),
            &self.style,
            factory.skin().as_ref(),
        )
    }

    fn add(&mut self, other: InsRef) -> CommandExecutionResult {
        self.list.add(other)
    }

    fn kill(&mut self) -> bool {
        self.list.kill()
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn contains_break(&self) -> bool {
        self.list.contains_break()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        self.list.swimlanes()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.list.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.list.swimlane_out()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: NodeKind) -> InsRef {
        Rc::new(RefCell::new(InstructionNode::new(
            kind,
            Colors::empty(),
            None,
            LinkRendering::none(),
        )))
    }

    #[test]
    fn list_collects_children() {
        let mut list = InstructionList::empty();
        assert!(list.is_empty());
        assert!(!list.kill());
        assert!(list.last().is_none());
        assert!(list.in_link_rendering().is_none());
        assert!(list.swimlane_out().is_none());
        list.add(node(NodeKind::Start));
        list.add(node(NodeKind::Stop));
        assert!(!list.is_empty());
        assert!(!list.contains_break());
        assert!(!list.kill());
        let last = list.last().unwrap();
        let last = last.borrow();
        let n = last.as_any().downcast_ref::<InstructionNode>().unwrap();
        assert_eq!(n.kind(), NodeKind::Stop);
        assert!(list.swimlanes().is_empty());
        assert!(list.parent().is_none());
    }

    #[test]
    fn simple_can_be_killed() {
        let skin = crate::skin::SkinParam::new();
        let mut s = InstructionSimple::new(
            Display::create(&["a"]),
            LinkRendering::none(),
            None,
            BoxStyle::Plain,
            None,
            Colors::empty(),
            None,
            skin.style_builder(),
        );
        assert!(!s.is_killed());
        assert!(s.kill());
        assert!(s.is_killed());
        assert!(s.swimlane_in().is_none());
    }
}
