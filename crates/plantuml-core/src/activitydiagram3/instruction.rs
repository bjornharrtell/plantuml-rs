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
use crate::decoration::usymbol::USymbol;
use crate::klimt::color::colors::Colors;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
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

    /// The list is a single `stop` (`isOnlySingleStopOrSpot`).
    pub fn is_only_single_stop(&self) -> bool {
        self.all.len() == 1
            && self.all[0]
                .borrow()
                .as_any()
                .downcast_ref::<InstructionNode>()
                .is_some_and(|n| n.kind() == NodeKind::Stop)
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

/// `break`: leaves the enclosing loop (`InstructionBreak`).
pub struct InstructionBreak {
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
}

impl InstructionBreak {
    pub fn new(swimlane: Option<Rc<Swimlane>>, in_link_rendering: LinkRendering) -> Self {
        Self {
            swimlane,
            in_link_rendering,
        }
    }
}

impl Instruction for InstructionBreak {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        super::ftile::leaf::FtileBreak::new(&factory.skin(), self.swimlane.clone())
    }

    fn kill(&mut self) -> bool {
        false
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn contains_break(&self) -> bool {
        true
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
        self.branches.last_mut().is_some_and(|b| b.kill())
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

/// A `switch` with its branches (`InstructionSwitch`).
pub struct InstructionSwitch {
    branches: Vec<(InstructionList, Option<Display>)>,
    test: Display,
    back_color: Option<HColor>,
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl InstructionSwitch {
    pub fn new(
        test: Display,
        back_color: Option<HColor>,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            branches: Vec::new(),
            test,
            back_color,
            swimlane,
            in_link_rendering,
            out_link_rendering: LinkRendering::none(),
        }
    }

    pub fn case(&mut self, label: Option<Display>) {
        self.branches
            .push((InstructionList::new(self.swimlane.clone()), label));
    }

    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        self.out_link_rendering = link;
    }
}

impl Instruction for InstructionSwitch {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let skin = factory.skin();
        let branches = self
            .branches
            .iter()
            .map(|(list, label)| super::ftile::if_::Branch {
                tile: list.create_ftile(factory),
                label: label
                    .as_ref()
                    .and_then(|d| super::ftile::factory::arrow_text_block(&skin, d)),
            })
            .collect();
        super::ftile::if_::FtileIf::new(
            self.test.clone(),
            branches,
            super::ftile::if_::ConditionalLayout::switch(),
            self.back_color.clone(),
            skin,
            self.in_link_rendering.clone(),
            self.out_link_rendering.clone(),
        )
    }

    fn add(&mut self, other: InsRef) -> CommandExecutionResult {
        match self.branches.last_mut() {
            Some((list, _)) => list.add(other),
            None => CommandExecutionResult::error("No 'case' in this switch"),
        }
    }

    fn kill(&mut self) -> bool {
        self.branches
            .last_mut()
            .is_some_and(|(list, _)| list.kill())
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn contains_break(&self) -> bool {
        self.branches.iter().any(|(l, _)| l.contains_break())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        for (list, _) in &self.branches {
            for lane in list.swimlanes() {
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
        self.branches.last().and_then(|(l, _)| l.swimlane_out())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Keeps the `break`s of the branches visible to the enclosing loop (`FtileDecorateWelding`).
fn weld(tile: FtileRef, points: Vec<super::ftile::WeldingPoint>) -> FtileRef {
    if points.is_empty() {
        tile
    } else {
        FtileDecorate::welding(tile, points)
    }
}

/// One branch of an `if` (`Branch`): the condition it tests, the labels around it and its body.
pub struct IfBranch {
    list: InstructionList,
    label_test: Display,
    label_positive: Option<Display>,
    inlabel: Option<Display>,
    color: Option<HColor>,
}

impl IfBranch {
    fn new(
        swimlane: Option<Rc<Swimlane>>,
        label_test: Display,
        label_positive: Option<Display>,
        inlabel: Option<Display>,
        color: Option<HColor>,
    ) -> Self {
        Self {
            list: InstructionList::new(swimlane),
            label_test,
            label_positive,
            inlabel,
            color,
        }
    }
}

/// An `if` with its branches (`InstructionIf`): one or more tested branches, then an optional `else`.
pub struct InstructionIf {
    thens: Vec<IfBranch>,
    else_branch: Option<IfBranch>,
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl InstructionIf {
    pub fn new(
        test: Display,
        yes: Option<Display>,
        back_color: Option<HColor>,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            thens: vec![IfBranch::new(swimlane.clone(), test, yes, None, back_color)],
            else_branch: None,
            swimlane,
            in_link_rendering,
            out_link_rendering: LinkRendering::none(),
        }
    }

    fn current(&mut self) -> &mut IfBranch {
        match self.else_branch.as_mut() {
            Some(b) => b,
            None => self.thens.last_mut().expect("one branch"),
        }
    }

    fn all_branches(&self) -> impl Iterator<Item = &IfBranch> {
        self.thens.iter().chain(self.else_branch.iter())
    }

    /// `else (label)`; `false` when there already is one (`switchToElse2`).
    pub fn else_branch(&mut self, label: Option<Display>) -> bool {
        if self.else_branch.is_some() {
            return false;
        }
        self.else_branch = Some(IfBranch::new(
            self.swimlane.clone(),
            Display::null(),
            label,
            None,
            None,
        ));
        true
    }

    /// `elseif (test) then (label)` (`elseIf`).
    pub fn else_if(
        &mut self,
        inlabel: Option<Display>,
        test: Display,
        label: Option<Display>,
        color: Option<HColor>,
    ) -> bool {
        if self.else_branch.is_some() {
            return false;
        }
        self.thens.push(IfBranch::new(
            self.swimlane.clone(),
            test,
            label,
            inlabel,
            color,
        ));
        true
    }

    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        self.out_link_rendering = link;
    }

    /// Remembers the arrow leaving the current branch (`addElse`).
    pub fn branch_out(&mut self, link: LinkRendering) {
        self.current().list.set_out_rendering(link);
    }
}

impl Instruction for InstructionIf {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let skin = factory.skin();
        let tile_of = |b: &IfBranch| super::ftile::if_::Branch {
            tile: b.list.create_ftile(factory),
            label: b
                .label_positive
                .as_ref()
                .and_then(|d| super::ftile::factory::arrow_text_block(&skin, d)),
        };
        if self.thens.len() > 1 {
            let thens: Vec<super::ftile::if_long::LongBranch> = self
                .thens
                .iter()
                .map(|b| super::ftile::if_long::LongBranch {
                    tile: b.list.create_ftile(factory),
                    label_test: b.label_test.clone(),
                    label_positive: b.label_positive.clone(),
                    inlabel: b.inlabel.clone(),
                    color: b.color.clone(),
                })
                .collect();
            let else_branch = match &self.else_branch {
                Some(b) => super::ftile::if_long::LongBranch {
                    tile: b.list.create_ftile(factory),
                    label_test: b.label_test.clone(),
                    label_positive: b.label_positive.clone(),
                    inlabel: None,
                    color: None,
                },
                None => super::ftile::if_long::LongBranch {
                    tile: InstructionList::new(self.swimlane.clone()).create_ftile(factory),
                    label_test: Display::null(),
                    label_positive: None,
                    inlabel: None,
                    color: None,
                },
            };
            let welding: Vec<_> = thens
                .iter()
                .chain(std::iter::once(&else_branch))
                .flat_map(|b| b.tile.welding_points())
                .collect();
            let result = super::ftile::if_long::FtileIfLongHorizontal::new(
                thens,
                else_branch,
                self.thens[0].color.clone(),
                skin,
                factory.string_bounder(),
                self.in_link_rendering.clone(),
                self.out_link_rendering.clone(),
            );
            return weld(result, welding);
        }
        let empty_or_stop = |b: &IfBranch| b.list.is_empty() || b.list.is_only_single_stop();
        let else_ref = self.else_branch.as_ref();
        let else_plain = else_ref.is_none_or(&empty_or_stop);
        let then_plain = empty_or_stop(&self.thens[0]);
        // Java's `FtileIfDown` handles a branch that is empty or only a `stop` next to a real one
        // (not ported yet); everything else is `FtileIfWithLinks`.
        let down = (else_plain && !then_plain) || (then_plain && !else_plain);
        if !down {
            let missing_else =
                IfBranch::new(self.swimlane.clone(), Display::null(), None, None, None);
            let else_info = else_ref.unwrap_or(&missing_else);
            let make = |b: &IfBranch| {
                let tile = b.list.create_ftile(factory);
                super::ftile::if_links::CondBranch {
                    tile,
                    label_test: b.label_test.clone(),
                    label_positive: b.label_positive.clone(),
                    color: b.color.clone(),
                    empty: b.list.is_empty(),
                }
            };
            let (first, second) = (make(&self.thens[0]), make(else_info));
            let welding: Vec<_> = first
                .tile
                .welding_points()
                .into_iter()
                .chain(second.tile.welding_points())
                .collect();
            let sb = factory.string_bounder();
            let result = super::ftile::if_links::FtileIfLinks::create(
                first,
                second,
                self.thens[0].color.clone(),
                skin,
                sb.as_ref(),
                self.in_link_rendering.clone(),
                self.out_link_rendering.clone(),
            );
            return weld(result, welding);
        }
        let branches: Vec<super::ftile::if_::Branch> = self.all_branches().map(tile_of).collect();
        let welding: Vec<_> = branches
            .iter()
            .flat_map(|b| b.tile.welding_points())
            .collect();
        let result = super::ftile::if_::FtileIf::new(
            self.thens[0].label_test.clone(),
            branches,
            super::ftile::if_::ConditionalLayout::if_(),
            self.thens[0].color.clone(),
            skin,
            self.in_link_rendering.clone(),
            self.out_link_rendering.clone(),
        );
        weld(result, welding)
    }

    fn add(&mut self, other: InsRef) -> CommandExecutionResult {
        self.current().list.add(other)
    }

    fn kill(&mut self) -> bool {
        self.current().list.kill()
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn contains_break(&self) -> bool {
        self.all_branches().any(|b| b.list.contains_break())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        for b in self.all_branches() {
            for lane in b.list.swimlanes() {
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
        self.all_branches()
            .last()
            .and_then(|b| b.list.swimlane_out())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A `repeat` loop (`InstructionRepeat`).
pub struct InstructionRepeat {
    list: InstructionList,
    test: Display,
    start_label: Option<Display>,
    back_color: Option<HColor>,
    colors: Colors,
    swimlane: Option<Rc<Swimlane>>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl InstructionRepeat {
    pub fn new(
        start_label: Option<Display>,
        colors: Colors,
        back_color: Option<HColor>,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            list: InstructionList::new(swimlane.clone()),
            test: Display::null(),
            start_label,
            back_color,
            colors,
            swimlane,
            in_link_rendering,
            out_link_rendering: LinkRendering::none(),
        }
    }

    pub fn set_test(&mut self, test: Display) {
        self.test = test;
    }

    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        self.out_link_rendering = link;
    }
}

impl Instruction for InstructionRepeat {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let body = self.list.create_ftile(factory);
        let start = self.start_label.as_ref().map(|label| {
            factory.activity(
                label,
                self.swimlane.clone(),
                BoxStyle::Plain,
                &self.colors,
                None,
                None,
            )
        });
        super::ftile::repeat::FtileRepeat::new(
            body,
            start,
            self.test.clone(),
            self.back_color.clone(),
            factory.skin(),
            self.in_link_rendering.clone(),
            self.out_link_rendering.clone(),
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
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.list.swimlane_out()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A `while` loop (`InstructionWhile`).
/// The `backward :text;` box of a loop.
struct Backward {
    label: Display,
    box_style: BoxStyle,
    stereotype: Option<Stereotype>,
    incoming1: LinkRendering,
    incoming2: LinkRendering,
    swimlane: Option<Rc<Swimlane>>,
}

pub struct InstructionWhile {
    backward: Option<Backward>,
    list: InstructionList,
    test: Display,
    yes: Option<Display>,
    out_display: Option<Display>,
    back_color: Option<HColor>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl InstructionWhile {
    pub fn new(
        test: Display,
        yes: Option<Display>,
        back_color: Option<HColor>,
        swimlane: Option<Rc<Swimlane>>,
        in_link_rendering: LinkRendering,
    ) -> Self {
        Self {
            backward: None,
            list: InstructionList::new(swimlane),
            test,
            yes,
            out_display: None,
            back_color,
            in_link_rendering,
            out_link_rendering: LinkRendering::none(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_backward(
        &mut self,
        label: Display,
        box_style: BoxStyle,
        incoming1: LinkRendering,
        incoming2: LinkRendering,
        stereotype: Option<Stereotype>,
        swimlane: Option<Rc<Swimlane>>,
    ) {
        self.backward = Some(Backward {
            label,
            box_style,
            stereotype,
            incoming1,
            incoming2,
            swimlane,
        });
    }

    pub fn set_out_display(&mut self, out: Option<Display>) {
        self.out_display = out;
    }

    pub fn set_out_rendering(&mut self, link: LinkRendering) {
        self.out_link_rendering = link;
    }
}

impl Instruction for InstructionWhile {
    fn create_ftile(&self, factory: &dyn FtileFactory) -> FtileRef {
        let body = self.list.create_ftile(factory);
        let (back1, back2, backward) = match &self.backward {
            Some(b) => (
                b.incoming1.display().clone(),
                b.incoming2.display().clone(),
                Some(factory.activity(
                    &b.label,
                    b.swimlane.clone(),
                    b.box_style,
                    &Colors::empty(),
                    b.stereotype.clone(),
                    None,
                )),
            ),
            None => (
                self.out_link_rendering.display().clone(),
                Display::null(),
                None,
            ),
        };
        super::ftile::while_::FtileWhile::new(
            body,
            self.test.clone(),
            super::ftile::while_::WhileLabels {
                yes: self.yes.clone(),
                out: self.out_display.clone(),
                back1,
                back2,
                backward,
            },
            self.back_color.clone(),
            factory.skin(),
            self.in_link_rendering.clone(),
            self.out_link_rendering.clone(),
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
