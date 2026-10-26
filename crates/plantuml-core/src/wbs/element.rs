//! Elements of the work breakdown structure (`WElement`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::UTranslate;
use crate::klimt::font::Dimension;
use crate::mindmap::{node_style, IdeaShape, NodeInfo, WBS_QUERIES};
use crate::stereo::Stereotype;
use crate::style::{queries, Style, StyleBuilder};
use std::cell::RefCell;
use std::rc::Rc;

/// An element while the tree is being declared.
#[derive(Debug, Clone)]
pub struct WNode {
    pub back_color: Option<HColor>,
    pub label: Display,
    pub level: i32,
    pub parent: Option<usize>,
    pub stereotype: Option<Stereotype>,
    pub left: Vec<usize>,
    pub right: Vec<usize>,
    pub shape: IdeaShape,
}

impl WNode {
    pub fn is_leaf(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}

/// A styled element, remembering where it was drawn.
pub struct WElement {
    pub back_color: Option<HColor>,
    pub label: Display,
    pub shape: IdeaShape,
    pub style: Style,
    /// Style of the lines drawn below the element.
    pub arrow_style: Style,
    pub is_leaf: bool,
    pub left: Vec<Rc<WElement>>,
    pub right: Vec<Rc<WElement>>,
    geometry: RefCell<Option<(UTranslate, Dimension)>>,
}

impl WElement {
    pub fn set_geometry(&self, position: UTranslate, dimension: Dimension) {
        *self.geometry.borrow_mut() = Some((position, dimension));
    }

    pub fn geometry(&self) -> Option<(UTranslate, Dimension)> {
        *self.geometry.borrow()
    }
}

/// Styles every node; children always come after their parent in `nodes`.
pub fn build_elements(nodes: &[WNode], builder: &StyleBuilder) -> Vec<Rc<WElement>> {
    let info = |i: usize| NodeInfo {
        level: nodes[i].level,
        shape: nodes[i].shape,
        is_leaf: nodes[i].is_leaf(),
        stereotype: nodes[i].stereotype.clone(),
    };
    let mut built: Vec<Option<Rc<WElement>>> = vec![None; nodes.len()];
    for i in (0..nodes.len()).rev() {
        let node = &nodes[i];
        let mut ancestors = Vec::new();
        let mut up = node.parent;
        while let Some(p) = up {
            ancestors.push(info(p));
            up = nodes[p].parent;
        }
        let style = node_style(builder, &WBS_QUERIES, &info(i), &ancestors);
        let arrow_query = queries::WBSDIAG_ARROW
            .add_level(node.level)
            .with_stereotype_obj(node.stereotype.as_ref());
        let arrow_style = builder
            .merged_style(&arrow_query)
            .expect("arrow style of wbs");
        let get = |ids: &[usize]| -> Vec<Rc<WElement>> {
            ids.iter()
                .map(|&c| built[c].clone().expect("child built"))
                .collect()
        };
        let element = Rc::new(WElement {
            back_color: node.back_color.clone(),
            label: node.label.clone(),
            shape: node.shape,
            style,
            arrow_style,
            is_leaf: node.is_leaf(),
            left: get(&node.left),
            right: get(&node.right),
            geometry: RefCell::new(None),
        });
        built[i] = Some(element);
    }
    built.into_iter().map(|e| e.expect("built")).collect()
}
