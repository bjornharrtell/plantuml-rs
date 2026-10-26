//! Mind map diagrams (`mindmap` package).

mod box_block;
mod commands;
mod finger;
mod idea;
mod tetris;

pub use box_block::BoxBlock;
pub use idea::{node_style, IdeaShape, NodeInfo, WBS_QUERIES};

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::{Rankdir, SkinParam};
use crate::stereo::Stereotype;
use finger::FingerImpl;
use idea::Branch;
use std::rc::Rc;

/// One tree of ideas, with the ideas going right and the ones going left (`MindMap`).
#[derive(Debug, Clone, Default)]
pub struct MindMap {
    regular: Branch,
    reverse: Branch,
    multiplier: i32,
}

impl MindMap {
    fn is_full(&self, level: i32) -> bool {
        level == 0 && self.regular.has_root()
    }

    fn add_idea_internal(
        &mut self,
        stereotype: Option<Stereotype>,
        back_color: Option<HColor>,
        mut level: i32,
        label: Display,
        shape: IdeaShape,
        direction: bool,
    ) -> CommandExecutionResult {
        if !self.reverse.has_root() && !self.regular.has_root() {
            level = 0;
        }
        if level == 0 {
            self.regular
                .init_root(back_color.clone(), label.clone(), shape, stereotype.clone());
            self.reverse.init_root(back_color, label, shape, stereotype);
            return CommandExecutionResult::ok();
        }
        if self.multiplier == 0 {
            self.multiplier = level;
        }
        if level % self.multiplier != 0 {
            return CommandExecutionResult::error("Bad indentation");
        }
        level /= self.multiplier;
        if !direction {
            return self
                .reverse
                .add(back_color, level, label, shape, stereotype);
        }
        self.regular
            .add(back_color, level, label, shape, stereotype)
    }
}

/// The laid out fingers of a [`MindMap`].
struct MindMapBlock {
    regular: Option<Rc<FingerImpl>>,
    reverse: Option<Rc<FingerImpl>>,
    has_root: bool,
    top_to_bottom: bool,
}

impl MindMapBlock {
    fn new(mindmap: &MindMap, skin: &Rc<SkinParam>) -> Self {
        let builder = skin.style_builder();
        let mut regular = None;
        let mut reverse = None;
        let has_root = mindmap.regular.has_root() || mindmap.reverse.has_root();
        if has_root {
            if mindmap.reverse.has_children() {
                reverse = Some(FingerImpl::build(
                    &mindmap.reverse,
                    0,
                    skin,
                    &builder,
                    false,
                ));
            }
            if (reverse.is_none() || mindmap.regular.has_children()) && mindmap.regular.has_root() {
                regular = Some(FingerImpl::build(&mindmap.regular, 0, skin, &builder, true));
            }
            if let (Some(r), Some(_)) = (&reverse, &regular) {
                r.do_not_draw_first_phalanx();
            }
        }
        Self {
            regular,
            reverse,
            has_root,
            top_to_bottom: skin.rankdir == Rankdir::TopToBottom,
        }
    }

    fn half_thickness(finger: &Option<Rc<FingerImpl>>, sb: &dyn StringBounder) -> f64 {
        finger.as_ref().map_or(0.0, |f| f.full_thickness(sb) / 2.0)
    }

    fn x12(finger: &Option<Rc<FingerImpl>>, sb: &dyn StringBounder) -> f64 {
        finger
            .as_ref()
            .map_or(0.0, |f| f.full_elongation(sb) + f.x12())
    }
}

impl TextBlock for MindMapBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let y1 = Self::half_thickness(&self.regular, sb);
        let y2 = Self::half_thickness(&self.reverse, sb);
        let y = y1.max(y2);
        let width = Self::x12(&self.reverse, sb) + Self::x12(&self.regular, sb);
        let height = y + y2.max(y1);
        if self.top_to_bottom {
            Dimension::new(height, width)
        } else {
            Dimension::new(width, height)
        }
    }

    fn draw(&self, ug: &UGraphic) {
        if !self.has_root {
            return;
        }
        let sb = ug.string_bounder();
        let y =
            Self::half_thickness(&self.regular, sb).max(Self::half_thickness(&self.reverse, sb));
        let x = Self::x12(&self.reverse, sb);
        let ug = if self.top_to_bottom {
            ug.apply_translate(UTranslate::new(y, x))
        } else {
            ug.apply_translate(UTranslate::new(x, y))
        };
        if let Some(f) = &self.regular {
            f.draw(&ug);
        }
        if let Some(f) = &self.reverse {
            f.draw(&ug);
        }
    }
}

/// All the mind maps of the source stacked vertically.
struct MindMaps {
    maps: Vec<MindMapBlock>,
}

impl TextBlock for MindMaps {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut width: f64 = 0.0;
        let mut height = 0.0;
        for map in &self.maps {
            let dim = map.calculate_dimension(sb);
            height += dim.height;
            width = width.max(dim.width);
        }
        Dimension::new(width + 10.0, height)
    }

    fn draw(&self, ug: &UGraphic) {
        let mut ug = ug.clone();
        for map in &self.maps {
            map.draw(&ug);
            let dim = map.calculate_dimension(ug.string_bounder());
            ug = ug.apply_translate(UTranslate::dy(dim.height));
        }
    }
}

/// The diagram being built by commands (`MindMapDiagram`).
#[derive(Debug, Clone)]
pub struct MindMapDiagram {
    pub base: DiagramBase,
    mindmaps: Vec<MindMap>,
    default_direction: bool,
    first: Option<String>,
}

impl MindMapDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        let mut base = DiagramBase::new(source, DiagramType::Mindmap);
        base.skin_param.rankdir = Rankdir::LeftToRight;
        Self {
            base,
            mindmaps: vec![MindMap::default()],
            default_direction: true,
            first: None,
        }
    }

    /// `left side`, `right side`, `top to bottom direction`... set where ideas go by default.
    pub fn set_default_direction(&mut self, right_or_down: bool) {
        self.default_direction = right_or_down;
    }

    fn last(&mut self) -> &mut MindMap {
        self.mindmaps.last_mut().expect("one mindmap")
    }

    pub fn add_idea(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        direction: bool,
    ) -> CommandExecutionResult {
        let stereotype = label.ending_stereotype();
        let label = if stereotype.is_some() {
            label.remove_ending_stereotype()
        } else {
            label
        };
        self.add_with_stereotype(stereotype, back_color, level, label, shape, direction)
    }

    pub fn add_idea_default(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        stereotype: Option<Stereotype>,
    ) -> CommandExecutionResult {
        let direction = self.default_direction;
        self.add_with_stereotype(stereotype, back_color, level, label, shape, direction)
    }

    pub fn add_idea_in_default_direction(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
    ) -> CommandExecutionResult {
        let direction = self.default_direction;
        self.add_idea(back_color, level, label, shape, direction)
    }

    fn add_with_stereotype(
        &mut self,
        stereotype: Option<Stereotype>,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        direction: bool,
    ) -> CommandExecutionResult {
        if self.last().is_full(level) {
            self.mindmaps.push(MindMap::default());
        }
        self.last()
            .add_idea_internal(stereotype, back_color, level, label, shape, direction)
    }

    /// Depth given by the stars (or other markers) of a line (`getSmartLevel`).
    pub fn get_smart_level(&mut self, marker: &str) -> Result<i32, String> {
        let first = self.first.get_or_insert_with(|| marker.to_string()).clone();
        let mut marker = marker.to_string();
        if marker.ends_with("**") {
            marker = marker.replace('\t', " ").trim().to_string();
        }
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
}

impl Diagram for MindMapDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        Rc::new(MindMaps {
            maps: self
                .mindmaps
                .iter()
                .map(|m| MindMapBlock::new(m, skin))
                .collect(),
        })
    }
}

impl HasBase for MindMapDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for MindMapDiagram {}

pub struct MindMapDiagramFactory;

impl DiagramFactory for MindMapDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Mindmap
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            MindMapDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests;
