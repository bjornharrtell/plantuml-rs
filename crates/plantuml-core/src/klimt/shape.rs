//! Text blocks and the horizontal line shape, ported from `klimt/shape`.

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::style::Margins;
use std::rc::Rc;

/// Something with a size that can draw itself (`TextBlock`).
pub trait TextBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension;
    fn draw(&self, ug: &UGraphic);
    /// Background announced to a containing vertical block.
    fn backcolor(&self) -> Option<HColor> {
        None
    }
    /// Box `(x, y, width, height)` of a member inside this block (`getInnerPosition`).
    fn inner_position(&self, _member: &str, _sb: &dyn StringBounder) -> Option<InnerRect> {
        None
    }
}

/// `(x, y, width, height)`.
pub type InnerRect = (f64, f64, f64, f64);

/// Horizontal extent available at a given `y` (`Stencil`).
pub trait Stencil {
    fn starting_x(&self, sb: &dyn StringBounder, y: f64) -> f64;
    fn ending_x(&self, sb: &dyn StringBounder, y: f64) -> f64;
}

/// Stencil of a plain rectangle starting at x = 0.
pub struct RectangleStencil(pub f64);

impl Stencil for RectangleStencil {
    fn starting_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        0.0
    }
    fn ending_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        self.0
    }
}

struct Skip<'a> {
    inner: &'a dyn Stencil,
    start: f64,
    end: f64,
}

impl Stencil for Skip<'_> {
    fn starting_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        self.inner.starting_x(sb, y) + self.start
    }
    fn ending_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        self.inner.ending_x(sb, y) - self.end
    }
}

/// `firstHalf` (second = false) or `secondHalf` of a stencil around a centered title.
struct Half<'a> {
    inner: &'a dyn Stencil,
    title_width: f64,
    second: bool,
}

impl Stencil for Half<'_> {
    fn starting_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        let start = self.inner.starting_x(sb, y);
        if !self.second {
            return start;
        }
        let end = self.inner.ending_x(sb, y);
        end - (end - start - self.title_width) / 2.0
    }
    fn ending_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        let end = self.inner.ending_x(sb, y);
        if self.second {
            return end;
        }
        let start = self.inner.starting_x(sb, y);
        start + (end - start - self.title_width) / 2.0
    }
}

/// A line spanning the whole stencil, with optional centered title (`UHorizontalLine`).
#[derive(Clone)]
pub struct UHorizontalLine {
    pub skip_at_start: f64,
    pub skip_at_end: f64,
    pub title: Option<Rc<dyn TextBlock>>,
    pub style: char,
    pub default_thickness: f64,
}

impl UHorizontalLine {
    pub fn infinite(default_thickness: f64, style: char, title: Option<Rc<dyn TextBlock>>) -> Self {
        Self {
            skip_at_start: 0.0,
            skip_at_end: 0.0,
            title,
            style,
            default_thickness,
        }
    }

    pub fn is_double(&self) -> bool {
        self.style == '='
    }

    pub fn stroke(&self) -> UStroke {
        match self.style {
            '=' | '-' => UStroke::simple(),
            '.' => UStroke::new(1.0, 2.0, 1.0),
            _ => UStroke::with_thickness(self.default_thickness),
        }
    }

    /// Draws the line at height `y` of `ug` (`drawLineInternal`).
    pub fn draw_line_internal(
        &self,
        ug: &UGraphic,
        stencil: &dyn Stencil,
        y: f64,
        default_stroke: UStroke,
    ) {
        let stencil = Skip {
            inner: stencil,
            start: self.skip_at_start,
            end: self.skip_at_end,
        };
        let stroke = if self.style == '\0' {
            default_stroke
        } else {
            self.stroke()
        };
        let sb = ug.string_bounder();
        let ug_stroke = ug.apply_stroke(stroke);
        let Some(title) = &self.title else {
            self.draw_hline(&ug_stroke, &stencil, y);
            return;
        };
        let dim = title.calculate_dimension(sb);
        let first = Half {
            inner: &stencil,
            title_width: dim.width,
            second: false,
        };
        self.draw_hline(&ug_stroke, &first, y);
        let sx = stencil.starting_x(sb, y);
        let ex = stencil.ending_x(sb, y);
        drop(ug_stroke);
        self.draw_title(ug, sx, ex, y);
        let second = Half {
            inner: &stencil,
            title_width: dim.width,
            second: true,
        };
        let ug_stroke = ug.apply_stroke(stroke);
        self.draw_hline(&ug_stroke, &second, y);
    }

    fn draw_hline(&self, ug: &UGraphic, stencil: &dyn Stencil, y: f64) {
        self.simple_hline(ug, stencil, y);
        if self.is_double() {
            self.simple_hline(ug, stencil, y + 2.0);
        }
    }

    fn simple_hline(&self, ug: &UGraphic, stencil: &dyn Stencil, y: f64) {
        let sx = stencil.starting_x(ug.string_bounder(), y);
        let ex = stencil.ending_x(ug.string_bounder(), y);
        ug.apply_translate(UTranslate::new(sx, y))
            .draw(ULine::hline(ex - sx));
    }

    fn draw_title(&self, ug: &UGraphic, start_x: f64, end_x: f64, y: f64) {
        self.draw_title_internal(ug, start_x, end_x, y, false);
    }

    /// Draws the centered title, first blanking its area when `clear_area` (`drawTitleInternal`).
    pub fn draw_title_internal(
        &self,
        ug: &UGraphic,
        start_x: f64,
        end_x: f64,
        y: f64,
        clear_area: bool,
    ) {
        let Some(title) = &self.title else {
            return;
        };
        let dim = title.calculate_dimension(ug.string_bounder());
        let x1 = start_x + ((end_x - start_x) - dim.width) / 2.0;
        let y1 = y - dim.height / 2.0 - 0.5;
        let ug = ug.apply_translate(UTranslate::new(x1, y1));
        if clear_area {
            ug.apply_stroke(self.stroke())
                .draw(URectangle::build(dim.width, dim.height));
        }
        title.draw(&ug);
    }
}

impl std::fmt::Debug for UHorizontalLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UHorizontalLine({:?})", self.style)
    }
}

impl PartialEq for UHorizontalLine {
    fn eq(&self, o: &Self) -> bool {
        self.style == o.style
            && self.skip_at_start == o.skip_at_start
            && self.skip_at_end == o.skip_at_end
            && match (&self.title, &o.title) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}

/// Block with a fixed size that draws nothing.
pub struct EmptyBlock {
    pub width: f64,
    pub height: f64,
}

impl TextBlock for EmptyBlock {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(self.width, self.height)
    }
    fn draw(&self, _: &UGraphic) {}
}

/// Adds space around a block (`TextBlockMarged`).
pub struct MargedBlock {
    pub inner: Rc<dyn TextBlock>,
    pub margins: Margins,
}

impl TextBlock for MargedBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.inner.calculate_dimension(sb);
        d.delta(
            self.margins.left + self.margins.right,
            self.margins.top + self.margins.bottom,
        )
    }
    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        if dim.width > 0.0 {
            self.inner
                .draw(&ug.apply_translate(UTranslate::new(self.margins.left, self.margins.top)));
        }
    }
    fn inner_position(&self, member: &str, sb: &dyn StringBounder) -> Option<InnerRect> {
        let (x, y, w, h) = self.inner.inner_position(member, sb)?;
        Some((x + self.margins.left, y + self.margins.top, w, h))
    }
}

/// `TextBlockUtils.withMargin`.
pub fn with_margin(block: Rc<dyn TextBlock>, margins: Margins) -> Rc<dyn TextBlock> {
    Rc::new(MargedBlock {
        inner: block,
        margins,
    })
}

/// Stacks blocks top to bottom (`TextBlockVertical`).
pub struct VerticalBlock {
    pub blocks: Vec<Rc<dyn TextBlock>>,
    pub alignment: HorizontalAlignment,
}

impl TextBlock for VerticalBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut it = self.blocks.iter();
        let Some(first) = it.next() else {
            return Dimension::default();
        };
        it.fold(first.calculate_dimension(sb), |d, b| {
            d.merge_tb(b.calculate_dimension(sb))
        })
    }

    fn draw(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        let mut y = 0.0;
        for block in &self.blocks {
            let dim = block.calculate_dimension(ug.string_bounder());
            if let Some(back) = block.backcolor() {
                if !back.is_transparent() {
                    let u = ug.apply_translate(UTranslate::dy(y));
                    let u = u.apply_color(back.clone());
                    let u = u.apply_back_color(back);
                    u.draw(URectangle::build(total.width, dim.height));
                }
            }
            let dx = match self.alignment {
                HorizontalAlignment::Left => 0.0,
                HorizontalAlignment::Center => (total.width - dim.width) / 2.0,
                HorizontalAlignment::Right => total.width - dim.width,
            };
            block.draw(&ug.apply_translate(UTranslate::new(dx, y)));
            y += dim.height;
        }
    }

    fn inner_position(&self, member: &str, sb: &dyn StringBounder) -> Option<InnerRect> {
        let mut y = 0.0;
        for block in &self.blocks {
            if let Some((rx, ry, w, h)) = block.inner_position(member, sb) {
                return Some((rx, ry + y, w, h));
            }
            y += block.calculate_dimension(sb).height;
        }
        None
    }
}

/// Puts blocks side by side (`TextBlockHorizontal`).
pub struct HorizontalBlock {
    pub blocks: Vec<Rc<dyn TextBlock>>,
    pub alignment: VerticalAlignment,
}

impl TextBlock for HorizontalBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut it = self.blocks.iter();
        let Some(first) = it.next() else {
            return Dimension::default();
        };
        it.fold(first.calculate_dimension(sb), |d, b| {
            d.merge_lr(b.calculate_dimension(sb))
        })
    }

    fn draw(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        let mut x = 0.0;
        for block in &self.blocks {
            let dim = block.calculate_dimension(ug.string_bounder());
            let dy = match self.alignment {
                VerticalAlignment::Center => (total.height - dim.height) / 2.0,
                VerticalAlignment::Bottom => total.height - dim.height,
                VerticalAlignment::Top => 0.0,
            };
            block.draw(&ug.apply_translate(UTranslate::new(x, dy)));
            x += dim.width;
        }
    }
}

/// Enforces a minimum width (`TextBlockMinWidth`).
pub struct MinWidthBlock {
    pub inner: Rc<dyn TextBlock>,
    pub min_width: f64,
    pub alignment: HorizontalAlignment,
}

impl TextBlock for MinWidthBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.inner
            .calculate_dimension(sb)
            .at_least(self.min_width, 0.0)
    }
    fn draw(&self, ug: &UGraphic) {
        let inner = self.inner.calculate_dimension(ug.string_bounder());
        let full = self.calculate_dimension(ug.string_bounder());
        let diff = full.width - inner.width;
        let dx = match self.alignment {
            HorizontalAlignment::Left => 0.0,
            HorizontalAlignment::Center => diff / 2.0,
            HorizontalAlignment::Right => diff,
        };
        self.inner.draw(&ug.apply_translate(UTranslate::dx(dx)));
    }
}

/// Text with a rounded border and padding (`TextBlockBordered`).
pub struct TextBlockBordered {
    pub text_block: Rc<dyn TextBlock>,
    pub stroke: Option<UStroke>,
    pub border_color: Option<HColor>,
    pub background_color: Option<HColor>,
    pub corner_size: f64,
    pub margins: Margins,
}

impl TextBlockBordered {
    fn text_height(&self, sb: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(sb).height + self.margins.top + self.margins.bottom
    }

    fn text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(sb).width + self.margins.left + self.margins.right
    }

    fn no_border(&self) -> bool {
        self.stroke.is_some_and(|s| s.thickness == 0.0)
    }
}

impl TextBlock for TextBlockBordered {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(self.text_width(sb) + 1.0, self.text_height(sb) + 1.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let rect =
            URectangle::build(self.text_width(sb), self.text_height(sb)).rounded(self.corner_size);
        let back = match &self.background_color {
            Some(b) if !b.is_transparent() && b != ug.default_background() => b.clone(),
            _ => HColor::TRANSPARENT,
        };
        let color = if self.no_border() {
            back.clone()
        } else {
            self.border_color.clone().unwrap_or(HColor::TRANSPARENT)
        };
        if !back.is_transparent() || !color.is_transparent() {
            let mut u = ug.apply_back_color(back).apply_color(color.clone());
            if let Some(stroke) = self.stroke {
                u = u.apply_stroke(stroke);
            }
            u.draw(rect);
        }
        self.text_block.draw(
            &ug.apply_color(color)
                .apply_translate(UTranslate::new(self.margins.left, self.margins.top)),
        );
    }
}

/// Vertical dashed-or-plain line interrupted at the `skip` heights (`VerticalLine`).
pub fn draw_vertical_line(ug: &UGraphic, y1: f64, y2: f64, skip: &[f64]) {
    use crate::klimt::drawing::ugraphic::UPath;
    let (y1, y2) = (y1.min(y2), y1.max(y2));
    let ug = ug.apply_back_color(HColor::TRANSPARENT);
    let mut drawn = false;
    let mut path = UPath::new();
    path.move_to(0.0, y1);
    for &step in skip {
        if step < y1 {
            continue;
        }
        drawn = true;
        if step == y2 {
            path.line_to(0.0, y2);
        } else {
            path.line_to(0.0, y2.min(step - 3.0));
            if y2 > step {
                path.arc_to(4.0, 4.0, 0.0, false, true, 0.0, step + 9.0);
                continue;
            }
        }
        ug.draw(path);
        path = UPath::new();
        let current = step + 9.0;
        path.move_to(0.0, current);
        if current >= y2 {
            break;
        }
    }
    if !drawn {
        path.line_to(0.0, y2);
        ug.draw(path);
    }
}
