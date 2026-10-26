//! Ideas and the branches holding them (`Idea`, `IdeaShape`, `Branch`).

use crate::command::CommandExecutionResult;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::stereo::Stereotype;
use crate::style::{queries, MergeStrategy, Style, StyleBuilder, StyleQuery};
use std::sync::LazyLock;

/// How a node is framed (`IdeaShape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdeaShape {
    Box,
    None,
    Pseudo,
}

impl IdeaShape {
    /// `_` after the stars removes the box.
    pub fn from_desc(s: Option<&str>) -> IdeaShape {
        if s == Some("_") {
            IdeaShape::None
        } else {
            IdeaShape::Box
        }
    }
}

type Query = LazyLock<StyleQuery>;

/// The style queries of the nodes of one diagram type.
pub struct NodeQueries {
    pub node: &'static Query,
    pub boxless: &'static Query,
    pub leaf: &'static Query,
    pub leaf_boxless: &'static Query,
    pub root: &'static Query,
    pub root_boxless: &'static Query,
}

pub static MINDMAP_QUERIES: NodeQueries = NodeQueries {
    node: &queries::MINDMAPDIAG_NODE,
    boxless: &queries::MINDMAPDIAG_NODE_BOXLESS,
    leaf: &queries::MINDMAPDIAG_NODE_LEAF,
    leaf_boxless: &queries::MINDMAPDIAG_NODE_LEAF_BOXLESS,
    root: &queries::MINDMAPDIAG_NODE_ROOT,
    root_boxless: &queries::MINDMAPDIAG_NODE_ROOT_BOXLESS,
};

pub static WBS_QUERIES: NodeQueries = NodeQueries {
    node: &queries::WBSDIAG_NODE,
    boxless: &queries::WBSDIAG_NODE_BOXLESS,
    leaf: &queries::WBSDIAG_NODE_LEAF,
    leaf_boxless: &queries::WBSDIAG_NODE_LEAF_BOXLESS,
    root: &queries::WBSDIAG_NODE_ROOT,
    root_boxless: &queries::WBSDIAG_NODE_ROOT_BOXLESS,
};

/// What the style of a node depends on.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub level: i32,
    pub shape: IdeaShape,
    pub is_leaf: bool,
    pub stereotype: Option<Stereotype>,
}

impl NodeInfo {
    /// `getDefaultStyleDefinitionNode(level)`.
    fn query(&self, queries: &NodeQueries, level: i32) -> StyleQuery {
        let boxless = self.shape == IdeaShape::None;
        let base: &StyleQuery = if level == 0 {
            if boxless {
                queries.root_boxless
            } else {
                queries.root
            }
        } else if boxless && self.is_leaf {
            queries.leaf_boxless
        } else if boxless {
            queries.boxless
        } else if self.is_leaf {
            queries.leaf
        } else {
            queries.node
        };
        base.with_stereotype_obj(self.stereotype.as_ref())
            .add_level(level)
    }
}

/// Style of a node, merged with the `*` styles of its ancestors (`Idea.getStyle`).
/// `ancestors` goes from the parent up to the root.
pub fn node_style(
    builder: &StyleBuilder,
    queries: &NodeQueries,
    me: &NodeInfo,
    ancestors: &[NodeInfo],
) -> Style {
    let mut result = builder
        .merged_style_special(&me.query(queries, me.level), 0)
        .expect("style of node");
    let mut rank = 0;
    for up in ancestors {
        let query = up.query(queries, me.level).add_star();
        rank -= 1;
        if let Some(parent) = builder.merged_style_special(&query, rank) {
            result = result.merge_with(&parent, MergeStrategy::OverwriteExistingValue);
        }
    }
    result
}

/// A node of the mind map.
#[derive(Debug, Clone)]
pub struct Idea {
    pub label: Display,
    pub level: i32,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub shape: IdeaShape,
    pub back_color: Option<HColor>,
    pub stereotype: Option<Stereotype>,
}

/// One side of the root: the ideas hang from the root at index 0 (`Branch`).
#[derive(Debug, Clone, Default)]
pub struct Branch {
    pub ideas: Vec<Idea>,
    last: Option<usize>,
}

impl Branch {
    pub fn init_root(
        &mut self,
        back_color: Option<HColor>,
        label: Display,
        shape: IdeaShape,
        stereotype: Option<Stereotype>,
    ) {
        self.ideas = vec![Idea {
            label,
            level: 0,
            parent: None,
            children: Vec::new(),
            shape,
            back_color,
            stereotype,
        }];
        self.last = Some(0);
    }

    pub fn has_root(&self) -> bool {
        !self.ideas.is_empty()
    }

    pub fn has_children(&self) -> bool {
        self.ideas.first().is_some_and(|r| !r.children.is_empty())
    }

    fn parent_of_last(&self, nb: i32) -> usize {
        let mut result = self.last.expect("last");
        for _ in 0..nb {
            result = self.ideas[result].parent.expect("parent");
        }
        result
    }

    fn create_idea(
        &mut self,
        parent: usize,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        stereotype: Option<Stereotype>,
    ) {
        let index = self.ideas.len();
        self.ideas.push(Idea {
            label,
            level,
            parent: Some(parent),
            children: Vec::new(),
            shape,
            back_color,
            stereotype,
        });
        self.ideas[parent].children.push(index);
        self.last = Some(index);
    }

    pub fn add(
        &mut self,
        back_color: Option<HColor>,
        level: i32,
        label: Display,
        shape: IdeaShape,
        stereotype: Option<Stereotype>,
    ) -> CommandExecutionResult {
        let Some(last) = self.last else {
            return CommandExecutionResult::error("Check your indentation ?");
        };
        let last_level = self.ideas[last].level;
        if level == last_level + 1 {
            self.create_idea(last, back_color, level, label, shape, stereotype);
            return CommandExecutionResult::ok();
        }
        if level <= last_level {
            let parent = self.parent_of_last(last_level - level + 1);
            self.create_idea(parent, back_color, level, label, shape, stereotype);
            return CommandExecutionResult::ok();
        }
        CommandExecutionResult::error("error42L")
    }

    /// Style information of an idea and of its ancestors, nearest first.
    pub fn infos(&self, index: usize) -> (NodeInfo, Vec<NodeInfo>) {
        let info = |i: usize| {
            let idea = &self.ideas[i];
            NodeInfo {
                level: idea.level,
                shape: idea.shape,
                is_leaf: idea.children.is_empty(),
                stereotype: idea.stereotype.clone(),
            }
        };
        let mut ancestors = Vec::new();
        let mut up = self.ideas[index].parent;
        while let Some(p) = up {
            ancestors.push(info(p));
            up = self.ideas[p].parent;
        }
        (info(index), ancestors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Display {
        Display::create(&[s])
    }

    #[test]
    fn shapes() {
        assert_eq!(IdeaShape::from_desc(Some("_")), IdeaShape::None);
        assert_eq!(IdeaShape::from_desc(None), IdeaShape::Box);
    }

    #[test]
    fn branch_tree() {
        let mut b = Branch::default();
        assert!(!b.has_root());
        assert!(!b.add(None, 1, d("x"), IdeaShape::Box, None).is_ok());
        b.init_root(None, d("r"), IdeaShape::Box, None);
        assert!(b.has_root() && !b.has_children());
        assert!(b.add(None, 1, d("a"), IdeaShape::Box, None).is_ok());
        assert!(b.add(None, 2, d("a1"), IdeaShape::Box, None).is_ok());
        assert!(b.add(None, 1, d("b"), IdeaShape::Box, None).is_ok());
        assert!(!b.add(None, 3, d("c"), IdeaShape::Box, None).is_ok());
        assert!(b.has_children());
        assert_eq!(b.ideas[0].children, vec![1, 3]);
        assert_eq!(b.ideas[1].children, vec![2]);
        let (info, ancestors) = b.infos(2);
        assert!(info.is_leaf);
        assert_eq!(ancestors.len(), 2);
    }
}
