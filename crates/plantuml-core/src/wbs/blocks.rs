//! Drawing of the tree (`ITF`, `ITFLeaf`, `ITFComposed`, `Fork`, `WBSTextBlock`).

use super::element::WElement;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{with_margin, EmptyBlock, TextBlock};
use crate::mindmap::{BoxBlock, IdeaShape};
use crate::skin::SkinParam;
use crate::style::{Margins, PName};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Point = (f64, f64);

/// A node of the tree that can be connected to its parent (`ITF`).
pub trait Itf: TextBlock {
    /// Where the line to the parent arrives when the node is below it.
    fn t1(&self, sb: &dyn StringBounder) -> Point;
    /// Where the line to the parent arrives when the node is on its right.
    fn f1(&self, sb: &dyn StringBounder) -> Point;
    /// Where the line to the parent arrives when the node is on its left.
    fn f2(&self, sb: &dyn StringBounder) -> Point;
    fn main_box_width(&self, sb: &dyn StringBounder) -> f64 {
        self.calculate_dimension(sb).width
    }
    fn set_forced_min_width(&self, _width: f64) {}
}

/// Text or box of an element (`WBSTextBlock.buildMain`).
fn build_main(element: &WElement, skin: &SkinParam) -> (Rc<dyn TextBlock>, Option<Rc<BoxBlock>>) {
    match element.shape {
        IdeaShape::Pseudo => (
            Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            }),
            None,
        ),
        IdeaShape::Box => {
            let b = Rc::new(BoxBlock::new(
                &element.style,
                skin,
                &element.label,
                element.back_color.as_ref(),
            ));
            (b.clone(), Some(b))
        }
        IdeaShape::None => {
            let style = &element.style;
            let text = element.label.create_block(
                &style.font_configuration(None),
                style.horizontal_alignment(),
                &skin.skin_simple(),
                &style.wrap_width(),
                CreoleMode::Full,
                None,
                None,
                0.0,
                0.0,
            );
            (with_margin(text, Margins::new(1.0, 3.0, 1.0, 0.0)), None)
        }
    }
}

fn is_auto_width(element: &WElement) -> bool {
    element
        .style
        .value(PName::Width)
        .as_string()
        .eq_ignore_ascii_case("auto")
}

/// Draws a connection line of the style of the element (`WBSTextBlock.drawLine`).
fn draw_line(ug: &UGraphic, element: &WElement, x1: f64, y1: f64, x2: f64, y2: f64) {
    let style = &element.arrow_style;
    let ug = ug
        .apply_color(style.value(PName::LineColor).as_color())
        .apply_stroke(style.stroke());
    let p1 = (x1.min(x2), y1);
    let p2 = (x1.max(x2), y2);
    ug.apply_translate(UTranslate::new(p1.0, p1.1))
        .draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
}

/// A node without children.
pub struct ItfLeaf {
    element: Rc<WElement>,
    block: Rc<dyn TextBlock>,
    fbox: Option<Rc<BoxBlock>>,
    forced_min_width: Cell<f64>,
}

impl ItfLeaf {
    pub fn new(element: &Rc<WElement>, skin: &SkinParam) -> Self {
        let (block, fbox) = build_main(element, skin);
        Self {
            element: element.clone(),
            block,
            fbox,
            forced_min_width: Cell::new(0.0),
        }
    }
}

impl TextBlock for ItfLeaf {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim = self.block.calculate_dimension(sb);
        if self.fbox.is_none() && self.forced_min_width.get() > dim.width {
            return Dimension::new(self.forced_min_width.get(), dim.height);
        }
        dim
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        self.element
            .set_geometry(ug.translate(), self.calculate_dimension(sb));
        let forced = self.forced_min_width.get();
        if self.fbox.is_none() && forced > 0.0 {
            let width = self.block.calculate_dimension(sb).width;
            if forced > width {
                self.block
                    .draw(&ug.apply_translate(UTranslate::dx((forced - width) / 2.0)));
                return;
            }
        }
        self.block.draw(ug);
    }
}

impl Itf for ItfLeaf {
    fn t1(&self, sb: &dyn StringBounder) -> Point {
        (self.calculate_dimension(sb).width / 2.0, 0.0)
    }

    fn f1(&self, sb: &dyn StringBounder) -> Point {
        (0.0, self.calculate_dimension(sb).height / 2.0)
    }

    fn f2(&self, sb: &dyn StringBounder) -> Point {
        let d = self.calculate_dimension(sb);
        (d.width, d.height / 2.0)
    }

    fn set_forced_min_width(&self, width: f64) {
        self.forced_min_width.set(width);
        if let Some(b) = &self.fbox {
            b.set_minimum_width(width);
        }
    }
}

const DELTA1X: f64 = 10.0;

/// A node with children on its left and right (`ITFComposed`).
pub struct ItfComposed {
    element: Rc<WElement>,
    left: Vec<Rc<dyn Itf>>,
    right: Vec<Rc<dyn Itf>>,
    main: Rc<dyn TextBlock>,
    main_box: Option<Rc<BoxBlock>>,
    margin_bottom: f64,
    auto_width_applied: Cell<bool>,
    cache: RefCell<Option<Dimension>>,
}

/// Builds the block of an element and of its descendants (`ITFComposed.build2`).
pub fn build_itf(skin: &SkinParam, element: &Rc<WElement>) -> Rc<dyn Itf> {
    if element.is_leaf {
        return Rc::new(ItfLeaf::new(element, skin));
    }
    let left = element.left.iter().map(|c| build_itf(skin, c)).collect();
    let right = element.right.iter().map(|c| build_itf(skin, c)).collect();
    let (main, main_box) = build_main(element, skin);
    let margin_bottom = if element.shape == IdeaShape::Pseudo {
        0.0
    } else {
        element.style.margin().bottom
    };
    Rc::new(ItfComposed {
        element: element.clone(),
        left,
        right,
        main,
        main_box,
        margin_bottom,
        auto_width_applied: Cell::new(false),
        cache: RefCell::new(None),
    })
}

fn coll_width(sb: &dyn StringBounder, all: &[Rc<dyn Itf>]) -> f64 {
    all.iter()
        .map(|c| c.calculate_dimension(sb).width)
        .fold(0.0, f64::max)
}

fn coll_height(sb: &dyn StringBounder, all: &[Rc<dyn Itf>], deltay: f64) -> f64 {
    all.iter()
        .map(|c| deltay + c.calculate_dimension(sb).height)
        .sum()
}

impl ItfComposed {
    fn apply_auto_width(&self, sb: &dyn StringBounder) {
        if self.auto_width_applied.replace(true) || !is_auto_width(&self.element) {
            return;
        }
        let all: Vec<&Rc<dyn Itf>> = self.left.iter().chain(self.right.iter()).collect();
        let max = all.iter().map(|c| c.main_box_width(sb)).fold(0.0, f64::max);
        for c in all {
            c.set_forced_min_width(max);
        }
        *self.cache.borrow_mut() = None;
    }

    fn w1(&self, sb: &dyn StringBounder) -> f64 {
        let main_width = self.main.calculate_dimension(sb).width;
        (main_width / 2.0).max(DELTA1X + coll_width(sb, &self.left))
    }
}

impl TextBlock for ItfComposed {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.apply_auto_width(sb);
        if let Some(d) = *self.cache.borrow() {
            return d;
        }
        let main = self.main.calculate_dimension(sb);
        let height = main.height
            + coll_height(sb, &self.left, self.margin_bottom).max(coll_height(
                sb,
                &self.right,
                self.margin_bottom,
            ));
        let width = (main.width / 2.0).max(DELTA1X + coll_width(sb, &self.left))
            + (main.width / 2.0).max(DELTA1X + coll_width(sb, &self.right));
        let dim = Dimension::new(width, height);
        *self.cache.borrow_mut() = Some(dim);
        dim
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        self.apply_auto_width(sb);
        let main = self.main.calculate_dimension(sb);
        self.element.set_geometry(ug.translate(), main);
        let w1 = self.w1(sb);
        self.main
            .draw(&ug.apply_translate(UTranslate::dx(w1 - main.width / 2.0)));
        let x = w1;
        let mut y = main.height;
        let mut last_y1 = y;
        for child in &self.left {
            y += self.margin_bottom;
            let child_dim = child.calculate_dimension(sb);
            let f2 = child.f2(sb);
            last_y1 = y + f2.1;
            draw_line(
                ug,
                &self.element,
                x - child_dim.width - DELTA1X + f2.0,
                last_y1,
                x,
                last_y1,
            );
            child.draw(&ug.apply_translate(UTranslate::new(x - child_dim.width - DELTA1X, y)));
            y += child_dim.height;
        }
        y = main.height;
        let mut last_y2 = y;
        for child in &self.right {
            y += self.margin_bottom;
            let child_dim = child.calculate_dimension(sb);
            let f1 = child.f1(sb);
            last_y2 = y + f1.1;
            draw_line(ug, &self.element, x, last_y2, x + DELTA1X + f1.0, last_y2);
            child.draw(&ug.apply_translate(UTranslate::new(x + DELTA1X, y)));
            y += child_dim.height;
        }
        draw_line(ug, &self.element, x, main.height, x, last_y1.max(last_y2));
    }
}

impl Itf for ItfComposed {
    fn t1(&self, sb: &dyn StringBounder) -> Point {
        (self.w1(sb), 0.0)
    }

    fn f1(&self, sb: &dyn StringBounder) -> Point {
        let main = self.main.calculate_dimension(sb);
        (self.w1(sb) - main.width / 2.0, main.height / 2.0)
    }

    fn f2(&self, sb: &dyn StringBounder) -> Point {
        let main = self.main.calculate_dimension(sb);
        (self.w1(sb) + main.width / 2.0, main.height / 2.0)
    }

    fn main_box_width(&self, sb: &dyn StringBounder) -> f64 {
        self.main.calculate_dimension(sb).width
    }

    fn set_forced_min_width(&self, width: f64) {
        if let Some(b) = &self.main_box {
            b.set_minimum_width(width);
        }
    }
}

/// The root and its children laid out in a row (`Fork`).
pub struct Fork {
    element: Rc<WElement>,
    main: Rc<dyn TextBlock>,
    right: Vec<Rc<dyn Itf>>,
    auto_width_applied: Cell<bool>,
    cache: RefCell<Option<Dimension>>,
}

const FORK_DELTA1X: f64 = 20.0;
const FORK_DELTAY: f64 = 40.0;

impl Fork {
    pub fn new(skin: &SkinParam, element: &Rc<WElement>) -> Self {
        let (main, _) = build_main(element, skin);
        let right = element.right.iter().map(|c| build_itf(skin, c)).collect();
        Self {
            element: element.clone(),
            main,
            right,
            auto_width_applied: Cell::new(false),
            cache: RefCell::new(None),
        }
    }

    fn apply_auto_width(&self, sb: &dyn StringBounder) {
        if self.auto_width_applied.replace(true) || !is_auto_width(&self.element) {
            return;
        }
        let max = self
            .right
            .iter()
            .map(|c| c.main_box_width(sb))
            .fold(0.0, f64::max);
        for c in &self.right {
            c.set_forced_min_width(max);
        }
        *self.cache.borrow_mut() = None;
    }
}

impl TextBlock for Fork {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.apply_auto_width(sb);
        if let Some(d) = *self.cache.borrow() {
            return d;
        }
        let mut width = 0.0;
        let mut height: f64 = 0.0;
        for child in &self.right {
            let d = child.calculate_dimension(sb);
            height = height.max(d.height);
            width += d.width;
        }
        if self.right.len() > 1 {
            width += (self.right.len() - 1) as f64 * FORK_DELTA1X;
        }
        let main = self.main.calculate_dimension(sb);
        height += main.height + FORK_DELTAY;
        let dim = Dimension::new(width.max(main.width), height);
        *self.cache.borrow_mut() = Some(dim);
        dim
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        self.apply_auto_width(sb);
        let main = self.main.calculate_dimension(sb);
        let y0 = main.height;
        let y1 = y0 + FORK_DELTAY / 2.0;
        let y2 = y0 + FORK_DELTAY;
        let main_width = main.width;
        let line = |x1, ya, x2, yb| draw_line(ug, &self.element, x1, ya, x2, yb);
        if self.right.is_empty() {
            self.main.draw(ug);
            line(main_width / 2.0, y0, main_width / 2.0, y1);
            return;
        }
        let mut x = 0.0;
        let first_x = self.right[0].t1(sb).0;
        let mut last_x = first_x;
        for child in &self.right {
            last_x = x + child.t1(sb).0;
            line(last_x, y1, last_x, y2);
            child.draw(&ug.apply_translate(UTranslate::new(x, y2)));
            x += child.calculate_dimension(sb).width + FORK_DELTA1X;
        }
        let pos_main;
        if last_x > first_x {
            line(first_x, y1, last_x, y1);
            pos_main = first_x + (last_x - first_x - main_width) / 2.0;
        } else {
            let full = self.calculate_dimension(sb);
            pos_main = (full.width - main_width) / 2.0;
            line(first_x, y1, pos_main + main_width / 2.0, y1);
        }
        self.main
            .draw(&ug.apply_translate(UTranslate::dx(pos_main)));
        line(
            pos_main + main_width / 2.0,
            y0,
            pos_main + main_width / 2.0,
            y1,
        );
    }
}
