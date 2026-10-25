//! Packet diagrams (`packetdiag` package).

mod commands;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{CreoleMode, LineBreakStrategy};
use crate::klimt::drawing::ugraphic::{UGraphic, URectangle, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{draw_vertical_line, EmptyBlock, TextBlock, VerticalBlock};
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::cell::RefCell;
use std::rc::Rc;

pub const DEFAULT_COL_WIDTH: i32 = 16;
const V_LINE_FULL: f64 = 32.0;
const V_LINE_SHORT: f64 = V_LINE_FULL / 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleDirection {
    Ltr,
    Rtl,
}

/// One declared range of bits (`PacketItem`).
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PacketItem {
    width: i32,
    height: i32,
    bit_start: i32,
    bit_end: i32,
    desc: String,
    text_rotation: i32,
}

impl PacketItem {
    fn of_range(start: i32, end: i32, height: i32, desc: &str) -> PacketItem {
        PacketItem {
            width: end - start + 1,
            height,
            bit_start: start,
            bit_end: end,
            desc: desc.to_string(),
            text_rotation: 0,
        }
    }

    fn to_block(&self) -> PacketBlock {
        PacketBlock::new(self.width, self.height, &self.desc)
    }

    fn to_block_open(&self, new_width: i32, open_left: bool, open_right: bool) -> PacketBlock {
        let mut b = PacketBlock::new(new_width.min(self.width), self.height, &self.desc);
        b.left_open = open_left;
        b.right_open = open_right;
        b
    }
}

/// A cell of the grid (`PacketBlock`).
#[derive(Debug, Clone)]
pub struct PacketBlock {
    width: i32,
    height: i32,
    label: String,
    left_open: bool,
    right_open: bool,
}

impl PacketBlock {
    fn new(width: i32, height: i32, label: &str) -> Self {
        Self {
            width,
            height,
            label: label.to_string(),
            left_open: false,
            right_open: false,
        }
    }

    fn style(skin: &SkinParam) -> Style {
        skin.style_builder()
            .merged_style(&queries::PACKETDIAG_RECTANGLE)
            .expect("packetdiag rectangle style")
    }

    fn label_block(skin: &Rc<SkinParam>, label: &str) -> Rc<dyn TextBlock> {
        let style = Self::style(skin);
        Display::with_newlines(Some(label)).create_block(
            &style.font_configuration(None),
            HorizontalAlignment::Center,
            &skin.skin_simple(),
            &LineBreakStrategy::NONE,
            CreoleMode::SimpleLine,
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn label_block_abbr(
        &self,
        skin: &Rc<SkinParam>,
        sb: &dyn StringBounder,
        bit_width: f64,
    ) -> Rc<dyn TextBlock> {
        let font = Self::style(skin).font();
        let pad = "...";
        let pad_dim = sb.calculate_dimension(&font, pad);
        let mut label_dim = sb.calculate_dimension(&font, &self.label);
        let req_width = f64::from(self.width) * bit_width;
        if label_dim.width < req_width {
            return Self::label_block(skin, &self.label);
        }
        let chars: Vec<char> = self.label.chars().collect();
        let mut abbr = self.label.clone();
        for i in (1..=chars.len()).rev() {
            if label_dim.width + pad_dim.width < req_width {
                break;
            }
            abbr = format!("{}{pad}", chars[..i].iter().collect::<String>());
            label_dim = sb.calculate_dimension(&font, &abbr);
        }
        Self::label_block(skin, &abbr)
    }

    fn shape_block(
        &self,
        skin: &Rc<SkinParam>,
        sb: &dyn StringBounder,
        bit_width: f64,
        bit_height: f64,
    ) -> Rc<dyn TextBlock> {
        let v_margin = 10.0;
        let req_width = f64::from(self.width) * bit_width;
        let fashion = Self::style(skin).symbol_context(None);
        let label = self.label_block_abbr(skin, sb, bit_width);
        let label_height = label.calculate_dimension(sb).height;
        let total_spare =
            (f64::from(self.height) * bit_height - label_height - v_margin - v_margin).max(0.0);
        let top_spare = total_spare / 2.0;
        let bottom_spare = total_spare - top_spare;
        let top: Rc<dyn TextBlock> = Rc::new(EmptyBlock {
            width: req_width,
            height: top_spare,
        });
        let bottom: Rc<dyn TextBlock> = Rc::new(EmptyBlock {
            width: req_width,
            height: bottom_spare,
        });
        Rc::new(ShapeBlock {
            req_width,
            top_spare,
            bottom_spare,
            label_height,
            v_margin,
            fashion,
            label,
            top,
            bottom,
        })
    }
}

struct ShapeBlock {
    req_width: f64,
    top_spare: f64,
    bottom_spare: f64,
    label_height: f64,
    v_margin: f64,
    fashion: Fashion,
    label: Rc<dyn TextBlock>,
    top: Rc<dyn TextBlock>,
    bottom: Rc<dyn TextBlock>,
}

impl TextBlock for ShapeBlock {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(
            self.req_width,
            self.top_spare + self.label_height + self.bottom_spare + self.v_margin + self.v_margin,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        if dim.width == 0.0 || dim.height == 0.0 {
            return;
        }
        let ug = ug.apply_stencil(
            Rc::new(crate::klimt::shape::RectangleStencil(dim.width)),
            UStroke::simple(),
        );
        let ug = self.fashion.apply(&ug);
        let mut rect =
            URectangle::build(dim.width, dim.height).rounded(self.fashion.round_corner());
        rect.delta_shadow = self.fashion.delta_shadow();
        ug.draw(rect);
        let tb: Rc<dyn TextBlock> = Rc::new(VerticalBlock {
            blocks: vec![self.top.clone(), self.label.clone()],
            alignment: HorizontalAlignment::Center,
        });
        let full = VerticalBlock {
            blocks: vec![tb, self.bottom.clone()],
            alignment: HorizontalAlignment::Center,
        };
        full.draw(&ug.apply_translate(UTranslate::dy(self.v_margin)));
    }
}

/// A tick of the bit ruler (`PacketIndicator`).
struct PacketIndicator {
    full: bool,
    numbered: bool,
    bit_number: String,
    style: Style,
    skin: Rc<SkinParam>,
}

impl PacketIndicator {
    fn number_block(&self) -> Rc<dyn TextBlock> {
        Display::create(&[self.bit_number.as_str()]).create_block(
            &self.style.font_configuration(None),
            HorizontalAlignment::Left,
            &self.skin.skin_simple(),
            &LineBreakStrategy::NONE,
            CreoleMode::NoCreole,
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let number = self.number_block().calculate_dimension(sb);
        number.merge_tb(Dimension::new(
            0.0,
            if self.full { V_LINE_FULL } else { V_LINE_SHORT },
        ))
    }

    fn draw(&self, ug: &UGraphic) {
        let number = self.number_block();
        let number_dim = number.calculate_dimension(ug.string_bounder());
        let line_offset_y = if self.full { 0.0 } else { V_LINE_SHORT };
        let line_length = if self.full { V_LINE_FULL } else { V_LINE_SHORT };
        if self.numbered {
            let y = if self.full { 0.0 } else { V_LINE_SHORT };
            number.draw(&ug.apply_translate(UTranslate::new(-number_dim.width / 2.0, y)));
        }
        let line_color = self
            .style
            .symbol_context(None)
            .fore_color()
            .cloned()
            .unwrap_or(crate::klimt::color::HColor::TRANSPARENT);
        let stroke = self.style.stroke();
        let ug = ug
            .apply_translate(UTranslate::dy(number_dim.height))
            .apply_color(line_color)
            .apply_stroke(stroke);
        draw_vertical_line(&ug, line_offset_y, line_offset_y + line_length, &[]);
    }
}

/// The diagram being built by commands (`PacketDiagram`).
#[derive(Debug, Clone)]
pub struct PacketDiagram {
    pub base: DiagramBase,
    col_width: i32,
    bit_width: f64,
    bit_height: f64,
    scale_interval: i32,
    use_default_scale_interval: bool,
    same_height: bool,
    scale_direction: ScaleDirection,
    packet_items: Vec<PacketItem>,
    grid: Rc<RefCell<Vec<Vec<PacketBlock>>>>,
    indicators_cols: Rc<RefCell<i32>>,
}

impl PacketDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        let col_width = DEFAULT_COL_WIDTH;
        Self {
            base: DiagramBase::new(source, DiagramType::Packet),
            col_width,
            bit_width: 24.0,
            bit_height: 32.0,
            scale_interval: col_width / 2,
            use_default_scale_interval: true,
            same_height: false,
            scale_direction: ScaleDirection::Ltr,
            packet_items: Vec::new(),
            grid: Rc::new(RefCell::new(Vec::new())),
            indicators_cols: Rc::new(RefCell::new(0)),
        }
    }

    pub fn set_col_width(&mut self, w: i32) {
        if w > 0 {
            self.col_width = w;
        }
    }

    pub fn update_scale_interval(&mut self, value: i32) {
        if value > 0 {
            self.scale_interval = value.min(self.col_width);
            self.use_default_scale_interval = false;
        }
    }

    pub fn update_node_height(&mut self, h: i32) {
        self.bit_height = f64::from(h.max(0));
    }

    pub fn set_scale_direction(&mut self, dir: &str) {
        self.scale_direction = if dir.eq_ignore_ascii_case("rtl") {
            ScaleDirection::Rtl
        } else {
            ScaleDirection::Ltr
        };
    }

    pub fn set_same_height(&mut self, b: bool) {
        self.same_height = b;
    }

    pub fn add_packet_item_range(
        &mut self,
        start: i32,
        end: i32,
        height: i32,
        desc: &str,
        rotation: i32,
    ) {
        let mut item = PacketItem::of_range(start, end, height, desc);
        item.text_rotation = rotation;
        self.packet_items.push(item);
    }

    pub fn last_packet_end(&self) -> Option<i32> {
        self.packet_items.last().map(|p| p.bit_end)
    }

    fn style(&self) -> Style {
        self.base
            .skin_param
            .style_builder()
            .merged_style(&queries::PACKETDIAG)
            .expect("packetdiag style")
    }

    fn full_indicator_interval(&self) -> i32 {
        if self.col_width >= 4 {
            self.col_width / 4
        } else {
            self.col_width
        }
    }

    /// Lays the items out in rows (`adjustLayout`).
    fn adjust_layout(&self) -> Vec<Vec<PacketBlock>> {
        let mut grid: Vec<Vec<PacketBlock>> = Vec::new();
        let mut curr_row: Vec<PacketBlock> = Vec::new();
        let mut remain_row_width = self.col_width;
        for packet in &self.packet_items {
            let overflow = packet.width - remain_row_width;
            let carry: Option<PacketItem> = if overflow > 0 {
                let margin = packet.width - remain_row_width;
                let bit_end = packet.bit_end - margin;
                curr_row.push(packet.to_block_open(remain_row_width, false, true));
                Some(PacketItem {
                    width: margin,
                    height: packet.height,
                    bit_start: bit_end,
                    bit_end: packet.bit_end,
                    desc: packet.desc.clone(),
                    text_rotation: 0,
                })
            } else {
                curr_row.push(packet.to_block());
                None
            };
            if let Some(p) = carry {
                remain_row_width = self.col_width;
                grid.push(std::mem::take(&mut curr_row));
                let row_cnt = p.width / remain_row_width;
                let remain = p.width % remain_row_width;
                for _ in 0..row_cnt {
                    grid.push(vec![p.to_block_open(remain_row_width, true, true)]);
                }
                if remain > 0 {
                    curr_row.push(p.to_block_open(remain_row_width, true, false));
                    remain_row_width -= remain;
                } else {
                    grid.last_mut().expect("row")[0].right_open = false;
                }
            } else {
                remain_row_width -= packet.width;
                if remain_row_width == 0 {
                    remain_row_width = self.col_width;
                    grid.push(std::mem::take(&mut curr_row));
                }
            }
        }
        if !curr_row.is_empty() {
            grid.push(curr_row);
        }
        for blocks in &mut grid {
            if self.same_height {
                let max_h = blocks.iter().map(|b| b.height).max().unwrap_or(1);
                for b in blocks.iter_mut() {
                    b.height = max_h;
                }
            }
            if self.scale_direction == ScaleDirection::Rtl {
                blocks.reverse();
            }
        }
        grid
    }

    /// Finalizes the layout (`build`); done once before drawing.
    fn build(&mut self) {
        if self.use_default_scale_interval {
            self.update_scale_interval(self.col_width / 2);
        }
        let grid = self.adjust_layout();
        if let Some(first) = grid.first() {
            let max_width: i32 = first.iter().map(|b| b.width).sum();
            self.col_width = max_width.min(self.col_width);
        }
        *self.grid.borrow_mut() = grid;
        *self.indicators_cols.borrow_mut() = self.col_width;
        let style = self.style();
        self.bit_width = style.value(PName::FontSize).as_double() * 3.0;
    }

    fn indicators(&self, skin: &Rc<SkinParam>) -> Vec<PacketIndicator> {
        let full_interval = self.full_indicator_interval();
        let cols = self.col_width;
        let order: Vec<i32> = if self.scale_direction == ScaleDirection::Rtl {
            (0..=cols).map(|i| cols - i).collect()
        } else {
            (0..=cols).collect()
        };
        order
            .into_iter()
            .map(|i| PacketIndicator {
                full: i % full_interval == 0,
                numbered: i % self.scale_interval == 0,
                bit_number: i.to_string(),
                style: self.style(),
                skin: skin.clone(),
            })
            .collect()
    }
}

struct PacketBlockTree {
    indicators: Vec<PacketIndicator>,
    grid: Vec<Vec<PacketBlock>>,
    skin: Rc<SkinParam>,
    bit_width: f64,
    bit_height: f64,
}

impl TextBlock for PacketBlockTree {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut indicator_total_width = 0.0;
        let mut indicator_max_height: f64 = 0.0;
        for ind in &self.indicators {
            let d = ind.dimension(sb);
            indicator_total_width += self.bit_width;
            indicator_max_height = indicator_max_height.max(d.height);
        }
        let mut grid_max_width: f64 = 0.0;
        let mut grid_total_height = 0.0;
        for blocks in &self.grid {
            let mut row_width = 0.0;
            let mut row_height = 0.0;
            for b in blocks {
                let dim = b
                    .shape_block(&self.skin, sb, self.bit_width, self.bit_height)
                    .calculate_dimension(sb);
                row_width += dim.width;
                row_height = dim.height;
            }
            grid_max_width = grid_max_width.max(row_width);
            grid_total_height += row_height;
        }
        Dimension::new(
            indicator_total_width.max(grid_max_width),
            indicator_max_height + grid_total_height,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let mut ug = ug.clone();
        let mut max_width = 0.0;
        let mut max_height: f64 = 0.0;
        for ind in &self.indicators {
            ind.draw(&ug);
            ug = ug.apply_translate(UTranslate::dx(self.bit_width));
            let d = ind.dimension(ug.string_bounder());
            max_width += self.bit_width;
            max_height = max_height.max(d.height);
        }
        ug = ug.apply_translate(UTranslate::new(-max_width, max_height));
        for blocks in &self.grid {
            let mut total_width = 0.0;
            let mut draw_height = 0.0;
            for b in blocks {
                let tb = b.shape_block(
                    &self.skin,
                    ug.string_bounder(),
                    self.bit_width,
                    self.bit_height,
                );
                let dim = tb.calculate_dimension(ug.string_bounder());
                total_width += dim.width;
                draw_height = dim.height;
                tb.draw(&ug);
                ug = ug.apply_translate(UTranslate::dx(dim.width));
            }
            ug = ug.apply_translate(UTranslate::new(-total_width, draw_height));
        }
    }
}

impl Diagram for PacketDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        Rc::new(PacketBlockTree {
            indicators: self.indicators(skin),
            grid: self.grid.borrow().clone(),
            skin: skin.clone(),
            bit_width: self.bit_width,
            bit_height: self.bit_height,
        })
    }
}

impl HasBase for PacketDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for PacketDiagram {
    fn make_diagram_ready(&mut self) {
        self.build();
    }
}

pub struct PacketDiagramFactory;

impl DiagramFactory for PacketDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Packet
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            PacketDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
