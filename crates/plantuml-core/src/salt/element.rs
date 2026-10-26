//! Widgets of salt diagrams (`salt/element`).

use crate::klimt::color::{HColor, Rgba};
use crate::klimt::creole::display::Display;
use crate::klimt::creole::SkinSimple;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPath, UPolygon, URectangle, UShape, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::string_utils::trin;
use std::cell::{Cell as StdCell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::LazyLock;

/// A widget (`Element`).
pub trait Element {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, y: f64) -> Dimension;
    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension);
    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        false
    }
    /// Text of a plain text element.
    fn text(&self) -> Option<&str> {
        None
    }
}

pub type Elmt = Rc<dyn Element>;

// ---- colors (`AbstractElement`) ----

fn gray(light: u8, dark: u8) -> HColor {
    HColor::simple(Rgba::rgb(light, light, light))
        .with_dark(HColor::simple(Rgba::rgb(dark, dark, dark)))
}

fn black() -> HColor {
    HColor::BLACK.with_dark(HColor::WHITE)
}

fn white() -> HColor {
    HColor::WHITE.with_dark(HColor::BLACK)
}

fn color88() -> HColor {
    gray(0x88, 0x88)
}

fn color_aa() -> HColor {
    gray(0xAA, 0x66)
}

fn color_bb() -> HColor {
    gray(0xBB, 0x55)
}

fn color_dd() -> HColor {
    gray(0xDD, 0x33)
}

fn color_ee() -> HColor {
    gray(0xEE, 0x22)
}

fn rect(w: f64, h: f64) -> UShape {
    UShape::Rectangle(URectangle::build(w, h))
}

/// Font used by all widgets (`UFontFactory.byDefault(12)`).
pub fn default_font(size: i32) -> UFont {
    UFont::sans_serif(size)
}

fn config(font: UFont) -> FontConfiguration {
    FontConfiguration::black_blue_true(font)
}

fn text_block(lines: &[String], config: &FontConfiguration) -> Rc<dyn TextBlock> {
    Display::create(lines).create_simple_block(
        config,
        HorizontalAlignment::Left,
        &SkinSimple::default(),
    )
}

// ---- AbstractElementText ----

static OPEN_ICON: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<&[-\w]+>").expect("valid"));
static HTML_TAG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\[\[[^\[\]]+\]\]|</?[a-zA-Z][^>]*>").expect("valid"));

struct TextCore {
    block: Rc<dyn TextBlock>,
    config: FontConfiguration,
    char_length: usize,
}

impl TextCore {
    fn new(text: &str, font: UFont, manage_length: bool) -> Self {
        let config = config(font);
        let (char_length, text) = if manage_length {
            let n = OPEN_ICON.replace_all(text, "00");
            let n = HTML_TAG.replace_all(&n, "");
            (n.encode_utf16().count(), trin(text).to_string())
        } else {
            (0, text.to_string())
        };
        let block = text_block(&[text], &config);
        Self {
            block,
            config,
            char_length,
        }
    }

    fn draw_text(&self, ug: &UGraphic, x: f64, y: f64) {
        self.block.draw(&ug.apply_translate(UTranslate::new(x, y)));
    }

    fn pure_text_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.block.calculate_dimension(sb)
    }

    fn text_dimension_at(&self, sb: &dyn StringBounder, _x: f64) -> Dimension {
        let result = self.block.calculate_dimension(sb);
        if self.char_length == 0 {
            return result;
        }
        Dimension::new(
            result.width.max(self.char_length as f64 * 8.0),
            result.height,
        )
    }
}

// ---- ElementEmpty, ElementText, ElementLine, ElementImage ----

pub struct ElementEmpty;

impl Element for ElementEmpty {
    fn preferred_dimension(&self, _: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        Dimension::new(1.0, 1.0)
    }
    fn draw(&self, _: &UGraphic, _: i32, _: Dimension) {}
}

pub struct ElementText {
    block: Rc<dyn TextBlock>,
    show: bool,
    text: String,
}

impl ElementText {
    pub fn new(lines: &[String], font: UFont) -> Self {
        let block = text_block(lines, &config(font));
        let text = lines.first().cloned().unwrap_or_default();
        Self {
            block,
            show: text != ".",
            text,
        }
    }
}

impl Element for ElementText {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        self.block.calculate_dimension(sb)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index == 0 && self.show {
            self.block.draw(ug);
        }
    }
    fn text(&self) -> Option<&str> {
        Some(&self.text)
    }
}

pub struct ElementLine {
    separator: char,
}

impl ElementLine {
    pub fn new(separator: char) -> Self {
        Self { separator }
    }
}

impl Element for ElementLine {
    fn preferred_dimension(&self, _: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        Dimension::new(10.0, 6.0)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        if z_index != 0 {
            return;
        }
        let ug = ug.apply_color(color_aa());
        let mut y2 = dim.height / 2.0;
        if self.separator == '=' {
            y2 -= 1.0;
        }
        let w = dim.width;
        let at = |y: f64| ug.apply_translate(UTranslate::new(0.0, y));
        match self.separator {
            '=' => {
                at(y2)
                    .apply_stroke(UStroke::simple())
                    .draw(ULine::hline(w));
                at(y2 + 2.0)
                    .apply_stroke(UStroke::simple())
                    .draw(ULine::hline(w));
            }
            '.' => at(y2)
                .apply_stroke(UStroke::new(1.0, 2.0, 1.0))
                .draw(ULine::hline(w)),
            '-' => at(y2)
                .apply_stroke(UStroke::simple())
                .draw(ULine::hline(w)),
            _ => at(y2)
                .apply_stroke(UStroke::with_thickness(1.5))
                .draw(ULine::hline(w)),
        }
    }
}

pub struct ElementImage {
    img: Vec<Vec<char>>,
}

impl ElementImage {
    pub fn new(img: &[String]) -> Self {
        Self {
            img: img.iter().map(|l| l.chars().collect()).collect(),
        }
    }
}

impl Element for ElementImage {
    fn preferred_dimension(&self, _: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        Dimension::new(
            self.img.first().map_or(0, Vec::len) as f64,
            self.img.len() as f64,
        )
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index != 0 {
            return;
        }
        let ug = ug.apply_color(black());
        for (j, line) in self.img.iter().enumerate() {
            for (i, c) in line.iter().enumerate() {
                if *c == 'X' {
                    ug.apply_translate(UTranslate::new(i as f64, j as f64))
                        .draw(UShape::Pixel);
                }
            }
        }
    }
}

/// Element retrieved from the dictionary (`WrappedElement`).
pub struct WrappedElement(pub Elmt);

impl Element for WrappedElement {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        self.0.preferred_dimension(sb, 0.0, 0.0)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        self.0.draw(ug, z_index, dim);
    }
    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        self.0.may_draw_beyond_preferred_dimension()
    }
}

// ---- ElementButton, ElementTextField, ElementDroplist ----

pub struct ElementButton {
    core: TextCore,
}

impl ElementButton {
    pub fn new(text: &str, font: UFont) -> Self {
        Self {
            core: TextCore::new(text, font, true),
        }
    }
}

impl Element for ElementButton {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, _: f64) -> Dimension {
        let stroke = 2.5;
        let (mx, my) = (2.0, 2.0);
        let dim = self.core.text_dimension_at(sb, x + stroke + mx);
        dim.delta(2.0 * mx, 2.0 * my).delta(2.0 * stroke, 2.0 * stroke)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index != 0 {
            return;
        }
        let (stroke, my) = (2.5, 2.0);
        let dim = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
        let ug = ug
            .apply_stroke(UStroke::with_thickness(stroke))
            .apply_back_color(color_ee())
            .apply_color(black());
        ug.apply_translate(UTranslate::new(stroke, stroke)).draw(
            URectangle::build(dim.width - 2.0 * stroke, dim.height - 2.0 * stroke).rounded(10.0),
        );
        let pure = self.core.pure_text_dimension(ug.string_bounder());
        self.core
            .draw_text(&ug, (dim.width - pure.width) / 2.0, stroke + my);
    }
}

pub struct ElementTextField {
    core: TextCore,
}

impl ElementTextField {
    pub fn new(text: &str, font: UFont) -> Self {
        Self {
            core: TextCore::new(text, font, true),
        }
    }
}

impl Element for ElementTextField {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, _: f64) -> Dimension {
        self.core.text_dimension_at(sb, x).delta(6.0, 2.0)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index != 0 {
            return;
        }
        self.core.draw_text(ug, 3.0, 0.0);
        let dim = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
        let text_dim = self.core.text_dimension_at(ug.string_bounder(), 0.0);
        ug.apply_translate(UTranslate::new(1.0, text_dim.height))
            .draw(ULine::hline(dim.width - 3.0));
        let y3 = text_dim.height - 3.0;
        ug.apply_translate(UTranslate::new(1.0, y3))
            .draw(ULine::vline(2.0));
        ug.apply_translate(UTranslate::new(3.0 + text_dim.width + 1.0, y3))
            .draw(ULine::vline(2.0));
    }
}

pub struct ElementDroplist {
    core: TextCore,
    open_drop: Option<Rc<dyn TextBlock>>,
}

const DROP_BOX: f64 = 12.0;

impl ElementDroplist {
    pub fn new(text: &str, font: UFont) -> Self {
        let head = text.split('^').next().unwrap_or("");
        let core = TextCore::new(head, font, true);
        let mut drop: Vec<String> = text
            .split('^')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !drop.is_empty() {
            drop.remove(0);
        }
        let open_drop = if drop.is_empty() {
            None
        } else {
            Some(text_block(&drop, &core.config))
        };
        Self { core, open_drop }
    }
}

impl Element for ElementDroplist {
    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        self.open_drop.is_some()
    }
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, _: f64) -> Dimension {
        self.core
            .text_dimension_at(sb, x + 2.0)
            .delta(4.0 + DROP_BOX, 4.0)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        let dim = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
        let ug = ug.apply_color(black());
        if z_index == 0 {
            ug.apply_back_color(color_ee())
                .draw(rect(dim.width - 1.0, dim.height - 1.0));
            self.core.draw_text(&ug, 2.0, 2.0);
            let xline = dim.width - DROP_BOX;
            ug.apply_translate(UTranslate::dx(xline))
                .draw(ULine::vline(dim.height - 1.0));
            let mut poly = UPolygon::default();
            poly.add_point(0.0, 0.0);
            poly.add_point(DROP_BOX - 6.0, 0.0);
            let dim_text = self.core.pure_text_dimension(ug.string_bounder());
            poly.add_point((DROP_BOX - 6.0) / 2.0, dim_text.height - 8.0);
            ug.apply_back_color(ug.color().clone())
                .apply_translate(UTranslate::new(xline + 3.0, 6.0))
                .draw(poly);
        }
        if let (Some(open), 1) = (&self.open_drop, z_index) {
            let d = open.calculate_dimension(ug.string_bounder());
            let dim_open = d.at_least(dim.width - 1.0, 0.0);
            let ug = ug.apply_translate(UTranslate::dy(dim.height - 1.0));
            ug.apply_back_color(color_ee())
                .draw(rect(dim_open.width - 1.0, dim_open.height - 1.0));
            open.draw(&ug);
        }
    }
}

// ---- ElementRadioCheckbox ----

pub struct ElementRadioCheckbox {
    block: Rc<dyn TextBlock>,
    radio: bool,
    checked: bool,
}

impl ElementRadioCheckbox {
    pub fn new(text: &str, font: UFont, radio: bool, checked: bool) -> Self {
        Self {
            block: text_block(&[text.to_string()], &config(font)),
            radio,
            checked,
        }
    }
}

impl Element for ElementRadioCheckbox {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        self.block.calculate_dimension(sb).delta(20.0, 0.0)
    }
    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index != 0 {
            return;
        }
        let ug = ug.apply_color(black());
        self.block.draw(&ug.apply_translate(UTranslate::dx(20.0)));
        let height = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0).height;
        let ug = ug.apply_stroke(UStroke::with_thickness(1.5));
        if self.radio {
            ug.apply_translate(UTranslate::new(2.0, (height - 10.0) / 2.0))
                .draw(UEllipse::build(10.0, 10.0));
            if self.checked {
                ug.apply_back_color(ug.color().clone())
                    .apply_translate(UTranslate::new(2.0 + (10.0 - 4.0) / 2.0, (height - 4.0) / 2.0))
                    .draw(UEllipse::build(4.0, 4.0));
            }
        } else {
            ug.apply_translate(UTranslate::new(2.0, (height - 10.0) / 2.0))
                .draw(rect(10.0, 10.0));
            if self.checked {
                let poly = UPolygon::new(vec![(0.0, 0.0), (3.0, 3.0), (10.0, -6.0), (3.0, 1.0)]);
                ug.apply_back_color(ug.color().clone())
                    .apply_translate(UTranslate::new(3.0, 6.0))
                    .draw(poly);
            }
        }
    }
}

// ---- menus ----

pub struct ElementMenuEntry {
    block: Rc<dyn TextBlock>,
    text: String,
    background: RefCell<Option<HColor>>,
    x: StdCell<f64>,
}

impl ElementMenuEntry {
    fn new(text: &str, font: UFont) -> Self {
        Self {
            block: text_block(&[text.to_string()], &config(font)),
            text: text.to_string(),
            background: RefCell::new(None),
            x: StdCell::new(0.0),
        }
    }

    fn preferred_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        if self.text == "-" {
            return Dimension::new(10.0, 5.0);
        }
        self.block.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_color(black());
        if let Some(bg) = self.background.borrow().clone() {
            let dim = self.preferred_dimension(ug.string_bounder());
            ug.apply_back_color(bg).draw(rect(dim.width, dim.height));
        }
        self.block.draw(&ug);
    }
}

struct ElementMenuPopup {
    entries: Vec<ElementMenuEntry>,
    font: UFont,
}

impl ElementMenuPopup {
    fn add_entry(&mut self, s: &str) {
        self.entries
            .push(ElementMenuEntry::new(s, self.font.clone()));
    }

    fn preferred_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let (mut w, mut h) = (0.0_f64, 0.0);
        for e in &self.entries {
            let d = e.preferred_dimension(sb);
            w = w.max(d.width);
            h += d.height;
        }
        Dimension::new(w, h)
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        if z_index != 1 {
            return;
        }
        let ug = ug.apply_color(black());
        ug.apply_back_color(color_dd()).draw(rect(dim.width, dim.height));
        let mut y1 = 0.0;
        for e in &self.entries {
            let h = e.preferred_dimension(ug.string_bounder()).height;
            if e.text == "-" {
                ug.apply_translate(UTranslate::dy(y1 + h / 2.0))
                    .draw(ULine::hline(dim.width));
            } else {
                e.draw(&ug.apply_translate(UTranslate::dy(y1)));
            }
            y1 += h;
        }
    }
}

pub struct ElementMenuBar {
    entries: Vec<Rc<ElementMenuEntry>>,
    popups: Vec<(Rc<ElementMenuEntry>, ElementMenuPopup)>,
    font: UFont,
}

impl ElementMenuBar {
    pub fn new(font: UFont) -> Self {
        Self {
            entries: Vec::new(),
            popups: Vec::new(),
            font,
        }
    }

    pub fn add_entry(&mut self, s: &str) {
        self.entries
            .push(Rc::new(ElementMenuEntry::new(s, self.font.clone())));
    }

    pub fn add_sub_entry(&mut self, s: &str, sub: &str) -> Result<(), String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.text == s)
            .cloned()
            .ok_or_else(|| format!("no menu entry {s}"))?;
        let idx = match self.popups.iter().position(|(e, _)| Rc::ptr_eq(e, &entry)) {
            Some(i) => i,
            None => {
                self.popups.push((
                    entry,
                    ElementMenuPopup {
                        entries: Vec::new(),
                        font: self.font.clone(),
                    },
                ));
                self.popups.len() - 1
            }
        };
        self.popups[idx].1.add_entry(sub);
        Ok(())
    }
}

impl Element for ElementMenuBar {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        let (mut w, mut h) = (0.0, 0.0_f64);
        for e in &self.entries {
            let d = e.preferred_dimension(sb);
            w += d.width + 10.0;
            h = h.max(d.height);
        }
        Dimension::new(w, h)
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        let preferred = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
        let ug = ug.apply_color(black());
        if z_index == 0 {
            let mut x1 = 0.0;
            ug.apply_back_color(color_dd()).draw(rect(dim.width, dim.height));
            for e in &self.entries {
                e.draw(&ug.apply_translate(UTranslate::dx(x1)));
                let w = e.preferred_dimension(ug.string_bounder()).width;
                e.x.set(x1);
                x1 += w + 10.0;
            }
        } else if z_index == 1 {
            for (e, _) in &self.popups {
                *e.background.borrow_mut() = Some(color_bb());
            }
            let y1 = preferred.height;
            for (e, p) in &self.popups {
                let xpopup = e.x.get();
                p.draw(
                    &ug.apply_translate(UTranslate::new(xpopup, y1)),
                    z_index,
                    p.preferred_dimension(ug.string_bounder()),
                );
            }
        }
    }
}

// ---- tab bar ----

pub struct ElementTabBar {
    tabs: Vec<ElementText>,
    font: UFont,
    pub vertical: bool,
}

impl ElementTabBar {
    pub fn new(font: UFont) -> Self {
        Self {
            tabs: Vec::new(),
            font,
            vertical: false,
        }
    }

    pub fn add_tab(&mut self, tab: &str) {
        self.tabs
            .push(ElementText::new(&[tab.to_string()], self.font.clone()));
    }
}

const TAB_M1: f64 = 2.0;
const TAB_M2: f64 = 3.0;
const TAB_M3: f64 = 10.0;

impl Element for ElementTabBar {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, y: f64) -> Dimension {
        let (mut w, mut h) = (0.0_f64, 0.0_f64);
        for t in &self.tabs {
            let d = t.preferred_dimension(sb, x, y);
            if self.vertical {
                h += d.height + TAB_M1 + TAB_M2 + TAB_M3;
                w = w.max(d.width);
            } else {
                w += d.width + TAB_M1 + TAB_M2 + TAB_M3;
                h = h.max(d.height);
            }
        }
        Dimension::new(w, h)
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        if z_index != 0 {
            return;
        }
        let ug = ug.apply_color(black());
        if self.vertical {
            let preferred = self.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
            let mut y1 = 0.0;
            for t in &self.tabs {
                t.draw(&ug.apply_translate(UTranslate::dy(y1 + TAB_M1)), z_index, dim);
                let h = t.preferred_dimension(ug.string_bounder(), 0.0, y1).height;
                ug.apply_translate(UTranslate::dy(y1))
                    .draw(ULine::hline(preferred.width));
                ug.apply_translate(UTranslate::dy(y1))
                    .draw(ULine::vline(h + TAB_M1 + TAB_M2));
                ug.apply_translate(UTranslate::dy(y1 + h + TAB_M1 + TAB_M2))
                    .draw(ULine::hline(preferred.width));
                ug.apply_translate(UTranslate::new(preferred.width, y1 + h + TAB_M1 + TAB_M2))
                    .draw(ULine::vline(TAB_M3));
                y1 += h + TAB_M1 + TAB_M2 + TAB_M3;
            }
        } else {
            let mut x1 = 0.0;
            for t in &self.tabs {
                t.draw(&ug.apply_translate(UTranslate::dx(x1 + TAB_M1)), z_index, dim);
                let d = t.preferred_dimension(ug.string_bounder(), x1, 0.0);
                let w = d.width;
                ug.apply_translate(UTranslate::dx(x1))
                    .draw(ULine::vline(d.height));
                ug.apply_translate(UTranslate::dx(x1))
                    .draw(ULine::hline(w + TAB_M1 + TAB_M2));
                ug.apply_translate(UTranslate::dx(x1 + w + TAB_M1 + TAB_M2))
                    .draw(ULine::vline(d.height));
                ug.apply_translate(UTranslate::new(x1 + w + TAB_M1 + TAB_M2, d.height))
                    .draw(ULine::hline(TAB_M3));
                x1 += w + TAB_M1 + TAB_M2 + TAB_M3;
            }
        }
    }
}

// ---- border ----

pub struct ElementBorder {
    pub north: Elmt,
    pub south: Elmt,
    pub east: Elmt,
    pub west: Elmt,
    pub center: Elmt,
}

impl ElementBorder {
    pub fn new() -> Self {
        let e = || -> Elmt { Rc::new(ElementEmpty) };
        Self {
            north: e(),
            south: e(),
            east: e(),
            west: e(),
            center: e(),
        }
    }
}

impl Default for ElementBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for ElementBorder {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, y: f64) -> Dimension {
        let n = self.north.preferred_dimension(sb, x, y);
        let s = self.south.preferred_dimension(sb, x, y);
        let e = self.east.preferred_dimension(sb, x, y);
        let w = self.west.preferred_dimension(sb, x, y);
        let c = self.center.preferred_dimension(sb, x, y);
        Dimension::new(
            n.width.max(s.width).max(w.width + c.width + e.width),
            n.height + w.height.max(c.height).max(e.height) + s.height,
        )
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        let sb = ug.string_bounder();
        let n = self.north.preferred_dimension(sb, 0.0, 0.0);
        let s = self.south.preferred_dimension(sb, 0.0, 0.0);
        let e = self.east.preferred_dimension(sb, 0.0, 0.0);
        let w = self.west.preferred_dimension(sb, 0.0, 0.0);
        let (ax, ay) = (w.width, n.height);
        let (bx, by) = (dim.width - e.width, n.height);
        let cy = dim.height - s.height;
        self.north.draw(ug, z_index, dim);
        self.south
            .draw(&ug.apply_translate(UTranslate::dy(cy)), z_index, dim);
        self.west
            .draw(&ug.apply_translate(UTranslate::dy(ay)), z_index, dim);
        self.east
            .draw(&ug.apply_translate(UTranslate::new(bx, by)), z_index, dim);
        self.center
            .draw(&ug.apply_translate(UTranslate::new(ax, ay)), z_index, dim);
    }

    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        [
            &self.north,
            &self.south,
            &self.east,
            &self.west,
            &self.center,
        ]
        .iter()
        .any(|e| e.may_draw_beyond_preferred_dimension())
    }
}

// ---- tables ----

/// Which lines a table draws (`TableStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStrategy {
    DrawNone,
    DrawOutside,
    DrawOutsideWithTitle,
    DrawHorizontal,
    DrawVertical,
    DrawAll,
}

impl TableStrategy {
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            ' ' => Self::DrawNone,
            '+' => Self::DrawOutside,
            '^' => Self::DrawOutsideWithTitle,
            '-' => Self::DrawHorizontal,
            '!' => Self::DrawVertical,
            '#' => Self::DrawAll,
            _ => return None,
        })
    }
}

/// Position of an element in the table (`Cell`).
#[derive(Debug, Clone)]
pub struct Cell {
    pub min_row: usize,
    pub max_row: usize,
    pub min_col: usize,
    pub max_col: usize,
}

impl Cell {
    fn new(row: usize, col: usize) -> Self {
        Self {
            min_row: row,
            max_row: row,
            min_col: col,
            max_col: col,
        }
    }
    fn nb_rows(&self) -> usize {
        self.max_row - self.min_row + 1
    }
    fn nb_cols(&self) -> usize {
        self.max_col - self.min_col + 1
    }
}

/// Puts elements in rows and columns (`Positionner2`).
#[derive(Default)]
pub struct Positionner {
    row: usize,
    col: usize,
    max_row: usize,
    max_col: usize,
    positions: Vec<(Elmt, Cell)>,
}

impl Positionner {
    pub fn add(&mut self, element: Elmt, newcol: bool) {
        self.positions.push((element, Cell::new(self.row, self.col)));
        self.update_max();
        if newcol {
            self.col += 1;
        } else {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn merge_left(&mut self, newcol: bool) {
        self.update_max();
        if newcol {
            self.col += 1;
        } else {
            self.row += 1;
            self.col = 0;
        }
        if let Some((_, last)) = self.positions.last_mut() {
            last.max_col += 1;
        }
    }

    fn update_max(&mut self) {
        self.max_row = self.max_row.max(self.row);
        self.max_col = self.max_col.max(self.col);
    }
}

/// Lines of a table (`Grid`).
struct Grid<'a> {
    rows_start: &'a [f64],
    cols_start: &'a [f64],
    strategy: TableStrategy,
    title: Rc<dyn TextBlock>,
    horizontals: SegmentSet,
    verticals: SegmentSet,
}

#[derive(Default)]
struct SegmentSet {
    order: Vec<(usize, usize)>,
    seen: HashSet<(usize, usize)>,
}

impl SegmentSet {
    fn add(&mut self, s: (usize, usize)) {
        if self.seen.insert(s) {
            self.order.push(s);
        }
    }
}

impl<'a> Grid<'a> {
    fn new(
        rows_start: &'a [f64],
        cols_start: &'a [f64],
        strategy: TableStrategy,
        title: Rc<dyn TextBlock>,
    ) -> Self {
        let mut g = Grid {
            rows_start,
            cols_start,
            strategy,
            title,
            horizontals: SegmentSet::default(),
            verticals: SegmentSet::default(),
        };
        if matches!(
            strategy,
            TableStrategy::DrawOutside
                | TableStrategy::DrawOutsideWithTitle
                | TableStrategy::DrawAll
        ) {
            let (nb_row, nb_col) = (rows_start.len(), cols_start.len());
            for c in 0..nb_col - 1 {
                g.horizontals.add((0, c));
                g.horizontals.add((nb_row - 1, c));
            }
            for r in 0..nb_row - 1 {
                g.verticals.add((r, 0));
                g.verticals.add((r, nb_col - 1));
            }
        }
        g
    }

    fn add_cell(&mut self, cell: &Cell) {
        use TableStrategy::*;
        if matches!(
            self.strategy,
            DrawNone | DrawOutside | DrawOutsideWithTitle
        ) {
            return;
        }
        if matches!(self.strategy, DrawHorizontal | DrawAll) {
            for c in cell.min_col..=cell.max_col {
                self.horizontals.add((cell.min_row, c));
                self.horizontals.add((cell.max_row + 1, c));
            }
        }
        if matches!(self.strategy, DrawVertical | DrawAll) {
            for r in cell.min_row..=cell.max_row {
                self.verticals.add((r, cell.min_col));
                self.verticals.add((r, cell.max_col + 1));
            }
        }
    }

    fn draw(&self, ug: &UGraphic, x: f64, y: f64, white: HColor) {
        for &(row, col) in &self.horizontals.order {
            let width = self.cols_start[col + 1] - self.cols_start[col];
            ug.apply_translate(UTranslate::new(
                x + self.cols_start[col],
                y + self.rows_start[row],
            ))
            .draw(ULine::hline(width));
        }
        for &(row, col) in &self.verticals.order {
            let height = self.rows_start[row + 1] - self.rows_start[row];
            ug.apply_translate(UTranslate::new(
                x + self.cols_start[col],
                y + self.rows_start[row],
            ))
            .draw(ULine::vline(height));
        }
        let dim = self.title.calculate_dimension(ug.string_bounder());
        if dim.width > 0.0 && dim.height > 0.0 {
            let ug2 = ug.apply_translate(UTranslate::new(x + 6.0, y));
            ug2.apply_back_color(white.clone())
                .apply_color(white)
                .draw(rect(dim.width, dim.height));
            self.title.draw(&ug2);
        }
    }
}

type Starts = (Vec<f64>, Vec<f64>);

/// Table of elements (`ElementPyramid`).
pub struct ElementPyramid {
    rows: usize,
    cols: usize,
    title: Rc<dyn TextBlock>,
    strategy: TableStrategy,
    positions: Vec<(Elmt, Cell)>,
    starts: RefCell<Option<Starts>>,
}

impl ElementPyramid {
    pub fn new(positionner: Positionner, strategy: TableStrategy, title: Option<&str>) -> Self {
        let title: Rc<dyn TextBlock> = match title {
            Some(t) => Display::with_newlines(Some(t)).create_simple_block(
                &config(UFont::sans_serif(10)),
                HorizontalAlignment::Left,
                &SkinSimple::default(),
            ),
            None => Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            }),
        };
        let mut rows = positionner.max_row + 1;
        let mut cols = positionner.max_col + 1;
        for (_, c) in &positionner.positions {
            rows = rows.max(c.max_row);
            cols = cols.max(c.max_col);
        }
        Self {
            rows,
            cols,
            title,
            strategy,
            positions: positionner.positions,
            starts: RefCell::new(None),
        }
    }

    fn init(&self, sb: &dyn StringBounder) -> Starts {
        if let Some(s) = self.starts.borrow().as_ref() {
            return s.clone();
        }
        let title_height = self.title.calculate_dimension(sb).height;
        let mut rows_start = vec![title_height / 2.0; self.rows + 1];
        let mut cols_start = vec![0.0; self.cols + 1];
        let mut all: Vec<&(Elmt, Cell)> = self.positions.iter().collect();
        all.sort_by(|a, b| {
            a.1.nb_cols()
                .cmp(&b.1.nb_cols())
                .then(a.1.min_col.cmp(&b.1.min_col))
        });
        for (elt, cell) in &all {
            let dim = elt.preferred_dimension(sb, 0.0, 0.0);
            ensure(&mut cols_start, cell.min_col, cell.max_col + 1, dim.width + 2.0);
        }
        all.sort_by(|a, b| {
            a.1.nb_rows()
                .cmp(&b.1.nb_rows())
                .then(a.1.min_row.cmp(&b.1.min_row))
        });
        for (elt, cell) in &all {
            let sup_y = if cell.min_row == 0 {
                title_height / 2.0
            } else {
                0.0
            };
            let dim = elt.preferred_dimension(sb, 0.0, 0.0);
            ensure(
                &mut rows_start,
                cell.min_row,
                cell.max_row + 1,
                dim.height + sup_y + 2.0,
            );
        }
        let result = (rows_start, cols_start);
        *self.starts.borrow_mut() = Some(result.clone());
        result
    }
}

fn ensure(starts: &mut [f64], first: usize, last: usize, size: f64) {
    let actual = starts[last] - starts[first];
    let missing = size - actual;
    if missing > 0.0 {
        for s in starts.iter_mut().skip(last) {
            *s += missing;
        }
    }
}

impl Element for ElementPyramid {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        let (rows_start, cols_start) = self.init(sb);
        Dimension::new(
            cols_start[cols_start.len() - 1],
            rows_start[rows_start.len() - 1] + self.title.calculate_dimension(sb).height,
        )
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        let ug = ug.apply_color(black());
        let (rows_start, cols_start) = self.init(ug.string_bounder());
        let title_height = self.title.calculate_dimension(ug.string_bounder()).height;
        let mut grid = Grid::new(&rows_start, &cols_start, self.strategy, self.title.clone());
        for (elt, cell) in &self.positions {
            let xcell = cols_start[cell.min_col];
            let sup_y = if cell.min_row == 0 {
                title_height / 2.0
            } else {
                0.0
            };
            let ycell = rows_start[cell.min_row] + sup_y;
            let width = cols_start[cell.max_col + 1] - cols_start[cell.min_col] - 1.0;
            let height = rows_start[cell.max_row + 1] - rows_start[cell.min_row] - 1.0;
            grid.add_cell(cell);
            elt.draw(
                &ug.apply_translate(UTranslate::new(xcell + 1.0, ycell + 1.0)),
                z_index,
                Dimension::new(width, height),
            );
        }
        if z_index == 0 {
            grid.draw(&ug, 0.0, 0.0, white());
        }
    }

    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        self.positions
            .iter()
            .any(|(e, _)| e.may_draw_beyond_preferred_dimension())
    }
}

/// Which scroll bars are shown (`ScrollStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollStrategy {
    Both,
    VerticalOnly,
    HorizontalOnly,
}

impl ScrollStrategy {
    pub fn from_desc(s: &str) -> Self {
        if s.ends_with('-') {
            Self::HorizontalOnly
        } else if s.ends_with('I') {
            Self::VerticalOnly
        } else {
            Self::Both
        }
    }
}

/// Table with scroll bars (`ElementPyramidScrolled`).
pub struct ElementPyramidScrolled {
    inner: ElementPyramid,
    strategy: ScrollStrategy,
}

impl ElementPyramidScrolled {
    pub fn new(positionner: Positionner, strategy: ScrollStrategy) -> Self {
        Self {
            inner: ElementPyramid::new(positionner, TableStrategy::DrawOutside, None),
            strategy,
        }
    }
}

const V1: f64 = 15.0;
const V2: f64 = 12.0;

fn triangle(pts: [(f64, f64); 3]) -> UPath {
    let mut p = UPath::new();
    p.move_to(pts[0].0, pts[0].1);
    p.line_to(pts[1].0, pts[1].1);
    p.line_to(pts[2].0, pts[2].1);
    p.line_to(pts[0].0, pts[0].1);
    p.close_path();
    p
}

fn draw_v(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(URectangle::build(width, height));
    ug.apply_translate(UTranslate::dy(V2)).draw(ULine::hline(width));
    ug.apply_translate(UTranslate::dy(height - V2))
        .draw(ULine::hline(width));
    let bg = black();
    ug.apply_translate(UTranslate::new(4.0, 4.0))
        .apply_back_color(bg.clone())
        .draw(triangle([(3.0, 0.0), (6.0, 5.0), (0.0, 5.0)]));
    ug.apply_translate(UTranslate::new(4.0, height - V2 + 4.0))
        .apply_back_color(bg)
        .draw(triangle([(3.0, 5.0), (6.0, 0.0), (0.0, 0.0)]));
}

fn draw_h(ug: &UGraphic, width: f64, height: f64) {
    ug.draw(URectangle::build(width, height));
    ug.apply_translate(UTranslate::dx(V2)).draw(ULine::vline(height));
    ug.apply_translate(UTranslate::dx(width - V2))
        .draw(ULine::vline(height));
    let bg = black();
    ug.apply_translate(UTranslate::new(4.0, 4.0))
        .apply_back_color(bg.clone())
        .draw(triangle([(0.0, 3.0), (5.0, 6.0), (5.0, 0.0)]));
    ug.apply_translate(UTranslate::new(width - V2 + 4.0, 4.0))
        .apply_back_color(bg)
        .draw(triangle([(5.0, 3.0), (0.0, 6.0), (0.0, 0.0)]));
}

impl Element for ElementPyramidScrolled {
    fn preferred_dimension(&self, sb: &dyn StringBounder, x: f64, y: f64) -> Dimension {
        let r = self.inner.preferred_dimension(sb, x, y);
        match self.strategy {
            ScrollStrategy::HorizontalOnly => r.delta(0.0, 30.0),
            ScrollStrategy::VerticalOnly => r.delta(30.0, 0.0),
            ScrollStrategy::Both => r.delta(30.0, 30.0),
        }
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, dim: Dimension) {
        self.inner.draw(ug, z_index, dim);
        let ug = ug.apply_color(black());
        let d = self.inner.preferred_dimension(ug.string_bounder(), 0.0, 0.0);
        if matches!(
            self.strategy,
            ScrollStrategy::Both | ScrollStrategy::VerticalOnly
        ) {
            draw_v(&ug.apply_translate(UTranslate::dx(d.width + 4.0)), V1, d.height);
        }
        if matches!(
            self.strategy,
            ScrollStrategy::Both | ScrollStrategy::HorizontalOnly
        ) {
            draw_h(&ug.apply_translate(UTranslate::dy(d.height + 4.0)), d.width, V1);
        }
    }

    fn may_draw_beyond_preferred_dimension(&self) -> bool {
        self.inner.may_draw_beyond_preferred_dimension()
    }
}

// ---- tree ----

/// Widths of the cells following the tree label (`ListWidth`).
#[derive(Default, Clone)]
struct ListWidth(Vec<f64>);

impl ListWidth {
    fn merge_max(&self, other: &ListWidth) -> ListWidth {
        let n = self.0.len().max(other.0.len());
        ListWidth(
            (0..n)
                .map(|i| {
                    let a = self.0.get(i).copied().unwrap_or(0.0);
                    let b = other.0.get(i).copied().unwrap_or(0.0);
                    a.max(b)
                })
                .collect(),
        )
    }

    fn total_width_with_margin(&self, margin: f64) -> f64 {
        let mut result = 0.0;
        for w in &self.0 {
            if result > 0.0 {
                result += margin;
            }
            result += w;
        }
        result
    }
}

struct ElementTreeEntry {
    first: Elmt,
    level: usize,
    others: Vec<Elmt>,
}

impl ElementTreeEntry {
    fn x_delta(&self) -> f64 {
        self.level as f64 * 10.0
    }

    fn first_cell_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.first
            .preferred_dimension(sb, 0.0, 0.0)
            .delta(self.x_delta(), 0.0)
    }

    fn other_cell_widths(&self, sb: &dyn StringBounder) -> ListWidth {
        ListWidth(
            self.others
                .iter()
                .map(|e| e.preferred_dimension(sb, 0.0, 0.0).width)
                .collect(),
        )
    }

    fn row_height(&self, sb: &dyn StringBounder) -> f64 {
        let mut h = self.first_cell_dimension(sb).height;
        for e in &self.others {
            h = h.max(e.preferred_dimension(sb, 0.0, 0.0).height);
        }
        h
    }
}

/// Tree, possibly with columns (`ElementTree`).
pub struct ElementTree {
    entries: Vec<ElementTreeEntry>,
    font: UFont,
    strategy: TableStrategy,
}

const TREE_MARGIN: f64 = 10.0;

impl ElementTree {
    pub fn new(font: UFont, strategy: TableStrategy) -> Self {
        Self {
            entries: Vec::new(),
            font,
            strategy,
        }
    }

    pub fn add_entry(&mut self, s: &str) {
        let level = s.chars().take_while(|c| *c == '+').count();
        let rest = trin(&s[level..]).to_string();
        let first = Rc::new(ElementText::new(&[rest], self.font.clone()));
        self.entries.push(ElementTreeEntry {
            first,
            level,
            others: Vec::new(),
        });
    }

    pub fn add_cell_to_entry(&mut self, e: Elmt) {
        if let Some(last) = self.entries.last_mut() {
            last.others.push(e);
        }
    }

    fn width_other(&self, sb: &dyn StringBounder) -> ListWidth {
        let mut merge = ListWidth::default();
        for e in &self.entries {
            merge = merge.merge_max(&e.other_cell_widths(sb));
        }
        merge
    }

    fn width1(&self, sb: &dyn StringBounder) -> f64 {
        self.entries
            .iter()
            .map(|e| e.first_cell_dimension(sb).width)
            .fold(0.0, f64::max)
    }
}

impl Element for ElementTree {
    fn preferred_dimension(&self, sb: &dyn StringBounder, _: f64, _: f64) -> Dimension {
        let w1 = self.width1(sb);
        let h: f64 = self.entries.iter().map(|e| e.row_height(sb)).sum();
        let mut w2 = self.width_other(sb).total_width_with_margin(TREE_MARGIN);
        if w2 > 0.0 {
            w2 += TREE_MARGIN;
        }
        Dimension::new(w1 + w2 + 2.0, h)
    }

    fn draw(&self, ug: &UGraphic, z_index: i32, _: Dimension) {
        if z_index != 0 {
            return;
        }
        let ug = ug.apply_color(black());
        let sb = ug.string_bounder();
        let w1 = self.width1(sb);
        let other_width = self.width_other(sb);
        let mut skeleton: Vec<(f64, f64)> = Vec::new();
        let mut yvar = 0.0;
        let mut rows = vec![yvar];
        let mut xvar = 0.0;
        let mut cols = vec![xvar];
        xvar += w1 + TREE_MARGIN / 2.0;
        cols.push(xvar);
        for w in &other_width.0 {
            xvar += w + TREE_MARGIN;
            cols.push(xvar);
        }
        for entry in &self.entries {
            entry
                .first
                .draw(
                    &ug.apply_translate(UTranslate::new(entry.x_delta(), yvar)),
                    0,
                    Dimension::new(0.0, 0.0),
                );
            let mut x = w1 + TREE_MARGIN;
            for (e, w) in entry.others.iter().zip(other_width.0.iter()) {
                e.draw(
                    &ug.apply_translate(UTranslate::new(x, yvar)),
                    0,
                    Dimension::new(0.0, 0.0),
                );
                x += w + TREE_MARGIN;
            }
            let h = entry.row_height(sb);
            skeleton.push((entry.x_delta() - 7.0, yvar + h / 2.0 - 1.0));
            yvar += h;
            rows.push(yvar);
        }
        let ug = ug.apply_color(color88());
        draw_skeleton(&ug, &skeleton);
        if self.strategy != TableStrategy::DrawNone {
            draw_grid2(&ug, &rows, &cols, self.strategy);
        }
    }
}

/// Lines linking the tree entries (`Skeleton`).
fn draw_skeleton(ug: &UGraphic, entries: &[(f64, f64)]) {
    for (i, &(x, y)) in entries.iter().enumerate() {
        if i + 1 < entries.len() && entries[i + 1].0 > x {
            ug.apply_translate(UTranslate::new(x, y)).draw(rect(2.0, 2.0));
        }
        let mut parent = None;
        for e in &entries[..i] {
            if e.0 < x {
                parent = Some(*e);
            }
        }
        if let Some((px, py)) = parent {
            let dy = y - py - 2.0;
            ug.apply_translate(UTranslate::new(px + 1.0, py + 3.0))
                .draw(ULine::vline(dy));
            let dx = x - px - 2.0;
            ug.apply_translate(UTranslate::new(px + 1.0, y + 1.0))
                .draw(ULine::hline(dx));
        }
    }
}

/// Lines of the tree table (`Grid2`).
fn draw_grid2(ug: &UGraphic, rows: &[f64], cols: &[f64], strategy: TableStrategy) {
    use TableStrategy::*;
    let (xmin, xmax) = (cols[0], cols[cols.len() - 1]);
    let (ymin, ymax) = (rows[0], rows[rows.len() - 1]);
    if matches!(strategy, DrawOutside | DrawOutsideWithTitle) {
        ug.apply_translate(UTranslate::new(xmin, ymin))
            .draw(ULine::hline(xmax - xmin));
        ug.apply_translate(UTranslate::new(xmin, ymax))
            .draw(ULine::hline(xmax - xmin));
        ug.apply_translate(UTranslate::new(xmin, ymin))
            .draw(ULine::vline(ymax - ymin));
        ug.apply_translate(UTranslate::new(xmax, ymin))
            .draw(ULine::vline(ymax - ymin));
    }
    if matches!(strategy, DrawHorizontal | DrawAll) {
        for y in rows {
            ug.apply_translate(UTranslate::new(xmin, *y))
                .draw(ULine::hline(xmax - xmin));
        }
    }
    if matches!(strategy, DrawVertical | DrawAll) {
        for x in cols {
            ug.apply_translate(UTranslate::new(*x, ymin))
                .draw(ULine::vline(ymax - ymin));
        }
    }
}
