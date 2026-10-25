//! Text body split by `--`, `==`, `..`, `__` separators (`BodyEnhanced2`, `BodyEnhancedAbstract`,
//! `TextBlockLineBefore`).

use super::display::{Display, DisplayLine};
use super::{LineBreakStrategy, SkinSimple};
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, MinWidthBlock, TextBlock, UHorizontalLine, VerticalBlock};
use crate::style::Margins;
use std::cell::RefCell;
use std::rc::Rc;

/// Whether a line separates two parts of a body (`isBlockSeparator`).
pub fn is_block_separator(s: &str) -> bool {
    (s.starts_with("--") && s.ends_with("--"))
        || (s.starts_with("==") && s.ends_with("=="))
        || (s.starts_with("..") && s.ends_with("..") && s != "...")
        || (s.starts_with("__") && s.ends_with("__"))
}

/// A block preceded by a separator line, possibly titled (`TextBlockLineBefore`).
pub struct LineBefore {
    pub block: Rc<dyn TextBlock>,
    pub separator: char,
    pub title: Option<Rc<dyn TextBlock>>,
    pub default_thickness: f64,
}

impl TextBlock for LineBefore {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim = self.block.calculate_dimension(sb);
        match &self.title {
            Some(t) => {
                let d = t.calculate_dimension(sb);
                dim.at_least(d.width + 8.0, d.height)
            }
            None => dim,
        }
    }

    fn draw(&self, ug: &UGraphic) {
        let line = |title: Option<Rc<dyn TextBlock>>| UHorizontalLine {
            skip_at_start: 1.0,
            skip_at_end: 1.0,
            title,
            style: self.separator,
            default_thickness: self.default_thickness,
        };
        if self.title.is_none() {
            ug.draw(line(None));
        }
        self.block.draw(ug);
        if self.title.is_some() {
            ug.draw(line(self.title.clone()));
        }
    }
}

/// Body of a note or a class compartment made of several parts (`BodyEnhanced2`).
pub struct BodyEnhanced {
    raw: Display,
    skin: SkinSimple,
    align: HorizontalAlignment,
    title_config: FontConfiguration,
    line_break: LineBreakStrategy,
    min_width: f64,
    default_thickness: f64,
    area: RefCell<Option<Rc<dyn TextBlock>>>,
}

impl BodyEnhanced {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        raw: Display,
        skin: SkinSimple,
        align: HorizontalAlignment,
        title_config: FontConfiguration,
        line_break: LineBreakStrategy,
        min_width: f64,
        default_thickness: f64,
    ) -> Self {
        Self {
            raw,
            skin,
            align,
            title_config,
            line_break,
            min_width,
            default_thickness,
            area: RefCell::new(None),
        }
    }

    fn title(&self, s: &str) -> Option<Rc<dyn TextBlock>> {
        if s.chars().count() <= 4 {
            return None;
        }
        let chars: Vec<char> = s.chars().collect();
        let inner: String = chars[2..chars.len() - 2].iter().collect();
        Some(
            Display::with_newlines(Some(inner.trim())).create_simple_block(
                &self.title_config,
                HorizontalAlignment::Left,
                &self.skin,
            ),
        )
    }

    fn text_block(&self, display: &Display) -> Rc<dyn TextBlock> {
        display.create_block(
            &self.title_config,
            self.align,
            &self.skin,
            &self.line_break,
            display.creole_mode(),
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn decorate(
        &self,
        block: Rc<dyn TextBlock>,
        separator: char,
        title: Option<Rc<dyn TextBlock>>,
        sb: &dyn StringBounder,
    ) -> Rc<dyn TextBlock> {
        if separator == '\0' {
            return block;
        }
        let Some(title) = title else {
            return Rc::new(LineBefore {
                block: with_margin(block, Margins::new(4.0, 0.0, 4.0, 0.0)),
                separator,
                title: None,
                default_thickness: self.default_thickness,
            });
        };
        let dim = title.calculate_dimension(sb);
        let raw: Rc<dyn TextBlock> = Rc::new(LineBefore {
            block: with_margin(block, Margins::new(dim.height / 2.0, 6.0, 4.0, 0.0)),
            separator,
            title: Some(title),
            default_thickness: self.default_thickness,
        });
        with_margin(raw, Margins::new(dim.height / 2.0, 0.0, 0.0, 0.0))
    }

    fn area(&self, sb: &dyn StringBounder) -> Rc<dyn TextBlock> {
        if let Some(a) = self.area.borrow().as_ref() {
            return a.clone();
        }
        let mut blocks: Vec<Rc<dyn TextBlock>> = Vec::new();
        let mut separator = '\0';
        let mut title: Option<Rc<dyn TextBlock>> = None;
        let mut start = 0;
        for (i, line) in self.raw.lines().iter().enumerate() {
            let DisplayLine::Text(s) = line else { continue };
            if is_block_separator(s) {
                let part = self.raw.sub_list(start, i);
                blocks.push(self.decorate(self.text_block(&part), separator, title.take(), sb));
                separator = s.chars().next().unwrap_or('\0');
                title = self.title(s);
                start = i + 1;
            }
        }
        let part = self.raw.sub_list(start, self.raw.size());
        blocks.push(self.decorate(self.text_block(&part), separator, title, sb));
        let mut area: Rc<dyn TextBlock> = if blocks.len() == 1 {
            blocks.remove(0)
        } else {
            Rc::new(VerticalBlock {
                blocks,
                alignment: self.align,
            })
        };
        if self.min_width > 0.0 {
            area = Rc::new(MinWidthBlock {
                inner: area,
                min_width: self.min_width,
                alignment: self.align,
            });
        }
        *self.area.borrow_mut() = Some(area.clone());
        area
    }
}

impl TextBlock for BodyEnhanced {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.area(sb).calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        self.area(ug.string_bounder()).draw(ug);
    }
}
