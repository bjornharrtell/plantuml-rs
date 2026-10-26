//! The flow as a whole: swimlanes, the instruction being filled and the drawing of the result
//! (`Swimlanes`, `Swimlane`, `Recentred`).
//!
//! Only the default (unnamed) swimlane is drawn for now; `Swimlanes` already keeps the list of
//! named lanes and `SwimlanesBlock::draw` is the place where the lane dividers, titles and
//! per-lane interception of `drawWhenSwimlanes` come in.

use super::ftile::factory::{default_factory, FtileRef};
use super::ftile::FtileGraphic;
use super::instruction::{InsRef, InstructionList};
use super::rainbow::LinkRendering;
use crate::klimt::color::colors::Colors;
use crate::klimt::compress::{CompressionMode, CompressionXorYBuilder};
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::MinMax;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A lane of the diagram (`Swimlane`).
pub struct Swimlane {
    name: String,
    order: usize,
    display: RefCell<Display>,
    colors: RefCell<Colors>,
    actual_width: Cell<f64>,
}

impl Swimlane {
    pub fn new(name: &str, order: usize) -> Self {
        Self {
            name: name.to_string(),
            order,
            display: RefCell::new(Display::with_newlines(Some(name))),
            colors: RefCell::new(Colors::empty()),
            actual_width: Cell::new(0.0),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn order(&self) -> usize {
        self.order
    }

    pub fn display(&self) -> Display {
        self.display.borrow().clone()
    }

    pub fn set_display(&self, label: Display) {
        *self.display.borrow_mut() = label;
    }

    pub fn colors(&self) -> Colors {
        self.colors.borrow().clone()
    }

    pub fn set_colors(&self, colors: Colors) {
        *self.colors.borrow_mut() = colors;
    }

    pub fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    pub fn set_actual_width(&self, w: f64) {
        self.actual_width.set(w);
    }
}

/// Parsing state of the flow: lanes, the root instruction, where new instructions go and the
/// arrow decoration waiting for the next one.
pub struct Swimlanes {
    swimlanes: Vec<Rc<Swimlane>>,
    current_swimlane: Option<Rc<Swimlane>>,
    root: InsRef,
    current: InsRef,
    next_link_renderer: LinkRendering,
}

impl Default for Swimlanes {
    fn default() -> Self {
        Self::new()
    }
}

impl Swimlanes {
    pub fn new() -> Self {
        let root: InsRef = Rc::new(RefCell::new(InstructionList::empty()));
        Self {
            swimlanes: Vec::new(),
            current_swimlane: None,
            current: root.clone(),
            root,
            next_link_renderer: LinkRendering::none(),
        }
    }

    pub fn swimlane(&mut self, name: &str, colors: Option<Colors>, label: Option<Display>) {
        let lane = self.get_or_create(name);
        if let Some(c) = colors {
            lane.set_colors(lane.colors().add_colors(&c));
        }
        if let Some(l) = label {
            lane.set_display(l);
        }
        self.current_swimlane = Some(lane);
    }

    fn get_or_create(&mut self, name: &str) -> Rc<Swimlane> {
        if let Some(s) = self.swimlanes.iter().find(|s| s.name() == name) {
            return s.clone();
        }
        let s = Rc::new(Swimlane::new(name, self.swimlanes.len()));
        self.swimlanes.push(s.clone());
        s
    }

    pub fn swimlane_count(&self) -> usize {
        self.swimlanes.len()
    }

    pub fn current(&self) -> InsRef {
        self.current.clone()
    }

    pub fn set_current(&mut self, current: InsRef) {
        self.current = current;
    }

    pub fn current_swimlane(&self) -> Option<Rc<Swimlane>> {
        self.current_swimlane.clone()
    }

    pub fn next_link_renderer(&self) -> LinkRendering {
        self.next_link_renderer.clone()
    }

    pub fn set_next_link_renderer(&mut self, link: LinkRendering) {
        self.next_link_renderer = link;
    }

    /// The drawable flow (`getTextBlock` of the diagram).
    pub fn text_block(&self, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        let block: Rc<dyn TextBlock> = Rc::new(SwimlanesBlock {
            skin: skin.clone(),
            root: self.root.clone(),
            swimlane_count: self.swimlanes.len(),
            min_max: RefCell::new(None),
            ftile: RefCell::new(None),
        });
        let block = CompressionXorYBuilder::build(CompressionMode::OnX, block);
        let block = CompressionXorYBuilder::build(CompressionMode::OnY, block);
        Rc::new(Recentred::new(block))
    }
}

/// Draws the whole flow (`Swimlanes.drawU`).
pub struct SwimlanesBlock {
    skin: Rc<SkinParam>,
    root: InsRef,
    swimlane_count: usize,
    min_max: RefCell<Option<MinMax>>,
    /// The tiles, built on the first drawing (Java rebuilds them for each pass).
    ftile: RefCell<Option<FtileRef>>,
}

impl SwimlanesBlock {
    /// Bounds of what is drawn, found by drawing once (`getMinMax`).
    pub fn min_max(&self, sb: &dyn StringBounder) -> MinMax {
        if let Some(m) = *self.min_max.borrow() {
            return m;
        }
        let m = find_limits(sb.to_rc(), false, |ug| self.draw(ug));
        *self.min_max.borrow_mut() = Some(m);
        m
    }
}

impl TextBlock for SwimlanesBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let m = self.min_max(sb);
        Dimension::new(m.width(), m.height())
    }

    fn draw(&self, ug: &UGraphic) {
        // With several lanes `drawWhenSwimlanes` draws each lane through its own interceptor.
        log::debug!("drawing the flow in {} lane(s)", self.swimlane_count.max(1));
        let cached = self.ftile.borrow().clone();
        let full = cached.unwrap_or_else(|| {
            let factory = default_factory(self.skin.clone(), ug.string_bounder().to_rc());
            let full = self.root.borrow().create_ftile(factory.as_ref());
            *self.ftile.borrow_mut() = Some(full.clone());
            full
        });
        let g = FtileGraphic::new(ug.clone());
        g.draw_ftile(full.as_ref());
        g.flush();
    }
}

/// Moves the flow so that it starts at (5, 5) and adds a margin (`Recentred`).
pub struct Recentred {
    inner: Rc<dyn TextBlock>,
    min_max: RefCell<Option<MinMax>>,
}

impl Recentred {
    pub fn new(inner: Rc<dyn TextBlock>) -> Self {
        Self {
            inner,
            min_max: RefCell::new(None),
        }
    }

    fn min_max(&self, sb: &dyn StringBounder) -> MinMax {
        if let Some(m) = *self.min_max.borrow() {
            return m;
        }
        let m = find_limits(sb.to_rc(), false, |ug| self.inner.draw(ug));
        let m = MinMax {
            max_x: m.max_x + 15.0,
            max_y: m.max_y + 15.0,
            ..m
        };
        *self.min_max.borrow_mut() = Some(m);
        m
    }
}

impl TextBlock for Recentred {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let m = self.min_max(sb);
        Dimension::new(m.width(), m.height())
    }

    fn draw(&self, ug: &UGraphic) {
        let m = self.min_max(ug.string_bounder());
        self.inner
            .draw(&ug.apply_translate(UTranslate::new(-m.min_x + 5.0, -m.min_y + 5.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanes_are_created_once() {
        let mut s = Swimlanes::new();
        assert_eq!(s.swimlane_count(), 0);
        assert!(s.current_swimlane().is_none());
        s.swimlane("a", None, None);
        s.swimlane("b", Some(Colors::empty()), Some(Display::create(&["B"])));
        s.swimlane("a", None, None);
        assert_eq!(s.swimlane_count(), 2);
        let cur = s.current_swimlane().unwrap();
        assert_eq!(cur.name(), "a");
        assert_eq!(cur.order(), 0);
        cur.set_actual_width(3.0);
        assert_eq!(cur.actual_width(), 3.0);
        assert_eq!(cur.display().text_at(0), "a");
        cur.set_colors(Colors::empty());
        assert!(cur.colors().is_empty());
        s.set_next_link_renderer(LinkRendering::none());
        assert!(s.next_link_renderer().is_none());
        let root = s.current();
        s.set_current(root);
    }
}
