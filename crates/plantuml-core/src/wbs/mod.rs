//! Work breakdown structure diagrams (`wbs` package).

mod blocks;
mod commands;
mod element;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::mindmap::IdeaShape;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, PName};
use crate::svek::extremity::{ExtremityFactory, ExtremityFactoryArrow};
use blocks::Fork;
use element::{build_elements, WElement, WNode};
use regex::Regex;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

static STEREOTYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(.*?)\s*(<<\s*(.*)\s*>>)\s*$").expect("valid"));

/// The diagram being built by commands (`WBSDiagram`).
#[derive(Debug, Clone)]
pub struct WbsDiagram {
    pub base: DiagramBase,
    nodes: Vec<WNode>,
    last: Option<usize>,
    first: Option<String>,
    codes: HashMap<String, usize>,
    links: Vec<(usize, usize, HColor)>,
}

impl WbsDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Wbs),
            nodes: Vec::new(),
            last: None,
            first: None,
            codes: HashMap::new(),
            links: Vec::new(),
        }
    }

    /// Adds an element whose label may end with a stereotype.
    pub fn add_idea_text(
        &mut self,
        code: Option<&str>,
        back_color: Option<HColor>,
        level: i32,
        label: &str,
        left: bool,
        shape: IdeaShape,
    ) -> CommandExecutionResult {
        let (label, stereotype) = match STEREOTYPE.captures(label) {
            Some(m) => (m[1].to_string(), Some(Stereotype::build(&m[2]))),
            None => (label.to_string(), None),
        };
        self.add_idea(
            code,
            back_color,
            level,
            Display::with_newlines(Some(&label)),
            stereotype,
            left,
            shape,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_idea(
        &mut self,
        code: Option<&str>,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        stereotype: Option<Stereotype>,
        left: bool,
        shape: IdeaShape,
    ) -> CommandExecutionResult {
        if level == 0 {
            if !self.nodes.is_empty() {
                return CommandExecutionResult::error(
                    "WBS diagram only accepts one root first declared.",
                );
            }
            self.nodes.push(WNode {
                back_color,
                label,
                level: 0,
                parent: None,
                stereotype,
                left: Vec::new(),
                right: Vec::new(),
                shape,
            });
            self.last = Some(0);
            return CommandExecutionResult::ok();
        }
        let Some(last) = self.last else {
            return CommandExecutionResult::error("Bad tree structure");
        };
        let last_level = self.nodes[last].level;
        let parent = if level == last_level + 1 {
            last
        } else if level <= last_level {
            let mut p = last;
            for _ in 0..(last_level - level + 1) {
                p = self.nodes[p].parent.expect("parent");
            }
            p
        } else {
            return CommandExecutionResult::error("Bad tree structure");
        };
        let index = self.nodes.len();
        self.nodes.push(WNode {
            back_color,
            label,
            level,
            parent: Some(parent),
            stereotype,
            left: Vec::new(),
            right: Vec::new(),
            shape,
        });
        // As in Java, a left child of the root is also put first on the right.
        if left && level == 1 {
            self.nodes[parent].right.insert(0, index);
        }
        if left {
            self.nodes[parent].left.push(index);
        } else {
            self.nodes[parent].right.push(index);
        }
        self.last = Some(index);
        if let Some(code) = code {
            self.codes.insert(code.to_string(), index);
        }
        CommandExecutionResult::ok()
    }

    /// Depth given by the stars (or other markers) of a line (`getSmartLevel`).
    pub fn get_smart_level(&mut self, marker: &str) -> Result<i32, String> {
        if self.nodes.is_empty() {
            self.first = Some(marker.to_string());
            return Ok(0);
        }
        let first = self.first.clone().unwrap_or_default();
        let marker = marker.replace('\t', " ");
        let len = marker.chars().count() as i32;
        let first_len = first.chars().count() as i32;
        if !marker.contains(' ') {
            return Ok(len - 1);
        }
        if marker.ends_with(&first) {
            return Ok(len - first_len);
        }
        if marker.trim().chars().count() == 1 {
            return Ok(len - 1);
        }
        if marker.starts_with(&first) {
            return Ok(len - first_len);
        }
        Err(format!("Bad indentation <{marker}>[{first}]"))
    }

    /// Connects two coded elements (`link`).
    pub fn link(
        &mut self,
        code1: &str,
        code2: &str,
        color: Option<HColor>,
        stereotype: Option<&Stereotype>,
    ) -> CommandExecutionResult {
        let Some(&e1) = self.codes.get(code1) else {
            return CommandExecutionResult::error(format!("No such node {code1}"));
        };
        let Some(&e2) = self.codes.get(code2) else {
            return CommandExecutionResult::error(format!("No such node {code2}"));
        };
        let color = color.unwrap_or_else(|| {
            let query = queries::WBSDIAG_ARROW.with_stereotype_obj(stereotype);
            self.base
                .skin_param
                .style_builder()
                .merged_style(&query)
                .map_or(HColor::TRANSPARENT, |s| {
                    s.value(PName::LineColor).as_color()
                })
        });
        self.links.push((e1, e2, color));
        CommandExecutionResult::ok()
    }
}

/// Arrow between two elements, drawn once their position is known (`WBSLink`).
fn draw_link(ug: &UGraphic, e1: &WElement, e2: &WElement, color: &HColor) {
    let (Some((p1, d1)), Some((p2, d2))) = (e1.geometry(), e2.geometry()) else {
        return;
    };
    let r1 = Rect::new(p1, d1);
    let r2 = Rect::new(p2, d2);
    let line = (r1.center_x(), r1.center_y(), r2.center_x(), r2.center_y());
    let (Some(c1), Some(c2)) = (r1.intersect(line), r2.intersect(line)) else {
        return;
    };
    let ug = ug.apply_color(color.clone());
    ug.apply_translate(UTranslate::new(c1.0, c1.1))
        .draw(ULine::new(c2.0 - c1.0, c2.1 - c1.1));
    let angle = (c2.1 - c1.1).atan2(c2.0 - c1.0);
    ExtremityFactoryArrow.create(c2, angle, None).draw(&ug);
}

struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

type Seg = (f64, f64, f64, f64);

fn intersect(a: Seg, b: Seg) -> Option<(f64, f64)> {
    let s1x = a.2 - a.0;
    let s1y = a.3 - a.1;
    let s2x = b.2 - b.0;
    let s2y = b.3 - b.1;
    let den = -s2x * s1y + s1x * s2y;
    let s = (-s1y * (a.0 - b.0) + s1x * (a.1 - b.1)) / den;
    let t = (s2x * (a.1 - b.1) - s2y * (a.0 - b.0)) / den;
    if (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t) {
        Some((a.0 + t * s1x, a.1 + t * s1y))
    } else {
        None
    }
}

impl Rect {
    fn new(p: UTranslate, d: Dimension) -> Self {
        Self {
            x: p.dx,
            y: p.dy,
            w: d.width,
            h: d.height,
        }
    }

    fn center_x(&self) -> f64 {
        self.x + self.w / 2.0
    }

    fn center_y(&self) -> f64 {
        self.y + self.h / 2.0
    }

    fn intersect(&self, line: Seg) -> Option<(f64, f64)> {
        let (x2, y2) = (self.x + self.w, self.y + self.h);
        [
            (self.x, self.y, x2, self.y),
            (x2, self.y, x2, y2),
            (x2, y2, self.x, y2),
            (self.x, y2, self.x, self.y),
        ]
        .into_iter()
        .find_map(|side| intersect(line, side))
    }
}

struct WbsBlock {
    fork: Fork,
    links: Vec<(Rc<WElement>, Rc<WElement>, HColor)>,
}

impl TextBlock for WbsBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.fork.calculate_dimension(sb).delta(20.0, 20.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::new(10.0, 10.0));
        let translate = ug.translate();
        self.fork.draw(&ug);
        let ug = ug.apply_translate(translate.reverse());
        for (e1, e2, color) in &self.links {
            draw_link(&ug, e1, e2, color);
        }
    }
}

impl Diagram for WbsDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        if self.nodes.is_empty() {
            return Rc::new(crate::klimt::shape::EmptyBlock {
                width: 0.0,
                height: 0.0,
            });
        }
        let builder = skin.style_builder();
        let elements = build_elements(&self.nodes, &builder);
        let fork = Fork::new(skin, &elements[0]);
        let links = self
            .links
            .iter()
            .map(|(a, b, c)| (elements[*a].clone(), elements[*b].clone(), c.clone()))
            .collect();
        Rc::new(WbsBlock { fork, links })
    }
}

impl HasBase for WbsDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for WbsDiagram {}

pub struct WbsDiagramFactory;

impl DiagramFactory for WbsDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Wbs
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            WbsDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests;
