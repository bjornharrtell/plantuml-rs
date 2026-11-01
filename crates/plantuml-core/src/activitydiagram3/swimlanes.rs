//! The flow as a whole: swimlanes, the instruction being filled and the drawing of the result
//! (`Swimlanes`, `Swimlane`, `Recentred`).
//!
//! With several lanes the flow is drawn once per lane, through a filter keeping the shapes of that
//! lane (`UGraphicInterceptorOneSwimlane`), then once more for the arrows that cross lanes
//! (`Swimlanes.drawWhenSwimlanes`).

use super::ftile::factory::{default_factory, FtileRef};
use super::ftile::FtileGraphic;
use super::instruction::{InsRef, InstructionList};
use super::rainbow::LinkRendering;
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::compress::{CompressionMode, CompressionXorYBuilder};
use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{find_limits, ULine, URectangle, UShape, UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, MinMax};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A lane of the diagram (`Swimlane`).
pub struct Swimlane {
    name: String,
    order: usize,
    display: RefCell<Display>,
    colors: RefCell<Colors>,
    actual_width: Cell<f64>,
    translate: Cell<UTranslate>,
    min_max: Cell<MinMax>,
}

impl Swimlane {
    pub fn new(name: &str, order: usize) -> Self {
        Self {
            name: name.to_string(),
            order,
            display: RefCell::new(Display::with_newlines(Some(name))),
            colors: RefCell::new(Colors::empty()),
            actual_width: Cell::new(0.0),
            translate: Cell::new(UTranslate::default()),
            min_max: Cell::new(MinMax::empty(true)),
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

    /// Whether no lane of `others` comes before this one (`isSmallerThanAllOthers`).
    pub fn is_smaller_than_all_others(&self, others: &[Rc<Swimlane>]) -> bool {
        if others.len() == 1 && std::ptr::eq(others[0].as_ref(), self) {
            return false;
        }
        others.iter().all(|o| o.order >= self.order)
    }

    /// Where the lane is drawn, once the widths are known.
    pub fn translate(&self) -> UTranslate {
        self.translate.get()
    }

    pub fn set_translate(&self, t: UTranslate) {
        self.translate.set(t);
    }

    /// Bounds of what the lane draws, before it is moved.
    pub fn min_max(&self) -> MinMax {
        self.min_max.get()
    }

    pub fn set_min_max(&self, m: MinMax) {
        self.min_max.set(m);
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
            lanes: self.swimlanes.clone(),
            special: RefCell::new(Vec::new()),
            dividers: RefCell::new(Vec::new()),
            size_computed: Cell::new(false),
            min_max: RefCell::new(None),
            ftile: RefCell::new(None),
        });
        let block = CompressionXorYBuilder::build(CompressionMode::OnX, block);
        let block = CompressionXorYBuilder::build(CompressionMode::OnY, block);
        Rc::new(Recentred::new(block))
    }
}

/// The vertical rule between two lanes (`LaneDivider`).
struct LaneDivider {
    x1: f64,
    x2: f64,
    height: f64,
}

impl LaneDivider {
    fn width(&self) -> f64 {
        self.x1 + self.x2
    }

    fn draw(&self, ug: &UGraphic, style: &Style) {
        ug.draw(UShape::Empty(self.x1 + self.x2, 1.0));
        ug.apply_translate(UTranslate::dx(self.x1))
            .apply_stroke(style.stroke())
            .apply_color(style.value(PName::LineColor).as_color())
            .draw(ULine::vline(self.height));
    }
}

/// Draws the whole flow (`Swimlanes.drawU`).
pub struct SwimlanesBlock {
    skin: Rc<SkinParam>,
    root: InsRef,
    lanes: Vec<Rc<Swimlane>>,
    /// The lanes and a last empty one that closes the right side (`swimlanesSpecial`).
    special: RefCell<Vec<Rc<Swimlane>>>,
    dividers: RefCell<Vec<LaneDivider>>,
    size_computed: Cell<bool>,
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

    fn full(&self, sb: &dyn StringBounder) -> FtileRef {
        if let Some(f) = self.ftile.borrow().clone() {
            return f;
        }
        let factory = default_factory(self.skin.clone(), sb.to_rc());
        let full = self.root.borrow().create_ftile(factory.as_ref());
        *self.ftile.borrow_mut() = Some(full.clone());
        full
    }

    fn style(&self) -> Style {
        self.skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_SWIMLANE)
            .expect("swimlane style")
    }

    fn title(&self, lane: &Swimlane) -> Rc<dyn TextBlock> {
        let style = self.style();
        let wrap = match self.skin.get_value("swimlanewraptitlewidth") {
            Some(v) => LineBreakStrategy::new(v),
            None => style.wrap_width(),
        };
        let wrap = if wrap.is_auto() {
            LineBreakStrategy::new(&format!("{}", lane.actual_width() as i64))
        } else {
            wrap
        };
        let display = lane.display();
        display.create_block(
            &style.font_configuration(None),
            HorizontalAlignment::Left,
            &self.skin.skin_simple(),
            &wrap,
            display.creole_mode(),
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn titles_height(&self, sb: &dyn StringBounder) -> f64 {
        self.lanes
            .iter()
            .map(|l| self.title(l).calculate_dimension(sb).height)
            .fold(0.0, f64::max)
    }

    fn title_height_translate(&self, sb: &dyn StringBounder) -> UTranslate {
        let h = self.titles_height(sb);
        UTranslate::dy(if h > 0.0 { h + 5.0 } else { 0.0 })
    }

    fn width_without_title(lane: &Swimlane) -> f64 {
        lane.min_max().width()
    }

    /// Padding on a side of the lane `i - 1` so that its title fits (`getHalfMissingSpace`).
    fn half_missing_space(&self, sb: &dyn StringBounder, i: usize, min: f64) -> f64 {
        let special = self.special.borrow();
        if i == 0 || i > special.len() {
            return 5.0;
        }
        let lane = &special[i - 1];
        let actual = min.max(Self::width_without_title(lane));
        let title_width = self.title(lane).calculate_dimension(sb).width;
        if title_width <= actual {
            return 5.0;
        }
        5.0f64.max(5.0 + (title_width - actual) / 2.0)
    }

    /// Measures every lane, then places them (`ensureSizeComputed`).
    fn ensure_size_computed(&self, ug: &UGraphic) {
        if self.size_computed.replace(true) || self.lanes.len() <= 1 {
            return;
        }
        let sb = ug.string_bounder();
        let full = self.full(sb);
        for lane in &self.lanes {
            let m = find_limits(sb.to_rc(), false, |ug| {
                let g = FtileGraphic::new(ug.clone()).for_swimlane(lane.clone());
                g.draw_ftile(full.as_ref());
                g.flush();
            });
            lane.set_min_max(m);
        }
        let last = Rc::new(Swimlane::new("", usize::MAX));
        last.set_min_max(MinMax::empty(true));
        let mut special = self.lanes.clone();
        special.push(last);
        *self.special.borrow_mut() = special.clone();

        let mut min = f64::from(self.skin_swimlane_width());
        if min == SWIMLANE_WIDTH_SAME {
            for lane in &self.lanes {
                min = min.max(Self::width_without_title(lane));
            }
        }
        for lane in &special {
            lane.set_actual_width(min.max(Self::width_without_title(lane)));
        }

        let title_dy = self.title_height_translate(sb).dy;
        let height = full.calculate_dimension(sb).height + title_dy;
        let mut dividers = Vec::new();
        let mut xpos = 0.0;
        for (i, lane) in special.iter().enumerate() {
            let divider = LaneDivider {
                x1: self.half_missing_space(sb, i, min),
                x2: self.half_missing_space(sb, i + 1, min),
                height,
            };
            let xx = xpos + divider.width() - lane.min_max().min_x
                + (lane.actual_width() - Self::width_without_title(lane)) / 2.0;
            lane.set_translate(UTranslate::dx(xx));
            xpos += lane.actual_width() + divider.width();
            dividers.push(divider);
        }
        *self.dividers.borrow_mut() = dividers;
    }

    fn skin_swimlane_width(&self) -> i32 {
        match self.skin.get_value("swimlanewidth") {
            Some(v) if v.eq_ignore_ascii_case("same") => SWIMLANE_WIDTH_SAME as i32,
            Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                v.parse().unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn draw_when_swimlanes(&self, ug: &UGraphic, full: &FtileRef) {
        let sb = ug.string_bounder();
        let title_dy = self.title_height_translate(sb);
        let style = self.style();
        let special = self.special.borrow().clone();
        let dividers = self.dividers.borrow();

        self.draw_titles_background(ug, &style, &special);

        let full_height = full.calculate_dimension(sb).height;
        let base = FtileGraphic::new(ug.clone());
        for (i, lane) in special.iter().enumerate() {
            let divider1 = &dividers[i];
            let xpos = lane.translate().dx + lane.min_max().min_x;
            if let Some(back) = lane.colors().color(ColorType::Back) {
                if !back.is_transparent() {
                    let divider2 = &dividers[i + 1];
                    let background = ug
                        .apply_back_color(back.clone())
                        .apply_color(back.clone())
                        .apply_translate(UTranslate::dx(xpos - divider1.x2));
                    let width = lane.actual_width() + divider1.x2 + divider2.x1;
                    let height = full_height + title_dy.dy;
                    background.draw(
                        URectangle::build(width, height)
                            .ignore_for_compression_on_x()
                            .ignore_for_compression_on_y(),
                    );
                }
            }
            base.for_swimlane(lane.clone())
                .apply_translate(lane.translate())
                .apply_translate(title_dy)
                .draw_ftile(full.as_ref());
            divider1.draw(&ug.apply_translate(UTranslate::dx(xpos - divider1.width())), &style);
        }

        let cross = base.for_cross().apply_translate(title_dy);
        cross.draw_ftile(full.as_ref());
        base.flush();

        self.draw_titles(ug, &special);
    }

    fn draw_titles_background(&self, ug: &UGraphic, style: &Style, special: &[Rc<Swimlane>]) {
        let color = style.value(PName::BackGroundColor).as_color();
        let height = self.titles_height(ug.string_bounder());
        let width = special.last().map_or(0.0, |l| l.translate().dx) - 2.0 * 5.0 - 1.0;
        let back = URectangle::build(width, height)
            .ignore_for_compression_on_x()
            .ignore_for_compression_on_y();
        ug.apply_translate(UTranslate::dx(5.0))
            .apply_back_color(color.clone())
            .apply_color(color)
            .draw(back);
    }

    fn draw_titles(&self, ug: &UGraphic, special: &[Rc<Swimlane>]) {
        for lane in special.iter().take(self.lanes.len()) {
            let title = self.title(lane);
            let x2 = lane.translate().dx + lane.min_max().min_x;
            ug.apply_translate(UTranslate::dx(x2))
                .draw_centered_text(title.as_ref(), Self::width_without_title(lane));
        }
    }
}

const SWIMLANE_WIDTH_SAME: f64 = -1.0;

impl TextBlock for SwimlanesBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let m = self.min_max(sb);
        Dimension::new(m.width(), m.height())
    }

    fn draw(&self, ug: &UGraphic) {
        log::debug!("drawing the flow in {} lane(s)", self.lanes.len().max(1));
        self.ensure_size_computed(ug);
        let full = self.full(ug.string_bounder());
        if self.lanes.len() > 1 {
            self.draw_when_swimlanes(ug, &full);
            return;
        }
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
