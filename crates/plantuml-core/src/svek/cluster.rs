//! A package drawn as a Graphviz cluster (`Cluster`).

use super::edge::SvekEdge;
use super::node::SvekNode;
use crate::abel::{Entity, EntityPosition};
use crate::klimt::dotpath::{Point, RectangleArea};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

pub const CENTER_ID: &str = "za";

pub struct ClusterData {
    pub parent: Option<Cluster>,
    pub group: Entity,
    pub nodes: Vec<SvekNode>,
    pub children: Vec<Cluster>,
    pub color: i32,
    pub color_title: i32,
    pub color_note_top: i32,
    pub color_note_bottom: i32,
    pub rectangle_area: Option<RectangleArea>,
    pub xy_title: Option<Point>,
    pub xy_note_top: Option<Point>,
    pub xy_note_bottom: Option<Point>,
    /// Width and height of the title and attributes (`ClusterHeader`).
    pub title_and_attribute: (f64, f64),
}

#[derive(Clone)]
pub struct Cluster(Rc<RefCell<ClusterData>>);

impl PartialEq for Cluster {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Cluster {
    pub fn new_root(group: Entity, colors: [i32; 4]) -> Cluster {
        Cluster(Rc::new(RefCell::new(ClusterData {
            parent: None,
            group,
            nodes: Vec::new(),
            children: Vec::new(),
            color: colors[0],
            color_title: colors[1],
            color_note_top: colors[2],
            color_note_bottom: colors[3],
            rectangle_area: None,
            xy_title: None,
            xy_note_top: None,
            xy_note_bottom: None,
            title_and_attribute: (0.0, 0.0),
        })))
    }

    pub fn create_child(&self, group: Entity, colors: [i32; 4], title: (f64, f64)) -> Cluster {
        let child = Cluster::new_root(group, colors);
        {
            let mut c = child.get_mut();
            c.parent = Some(self.clone());
            c.title_and_attribute = title;
        }
        self.get_mut().children.push(child.clone());
        child
    }

    pub fn get(&self) -> Ref<'_, ClusterData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, ClusterData> {
        self.0.borrow_mut()
    }

    pub fn group(&self) -> Entity {
        self.get().group.clone()
    }

    pub fn groups(&self) -> Vec<Entity> {
        vec![self.group()]
    }

    pub fn add_node(&self, node: SvekNode) {
        self.get_mut().nodes.push(node);
    }

    pub fn nodes(&self) -> Vec<SvekNode> {
        self.get().nodes.clone()
    }

    pub fn children(&self) -> Vec<Cluster> {
        self.get().children.clone()
    }

    pub fn parent(&self) -> Option<Cluster> {
        self.get().parent.clone()
    }

    pub fn rectangle_area(&self) -> Option<RectangleArea> {
        self.get().rectangle_area
    }

    pub fn set_position(&self, min: Point, max: Point) {
        self.get_mut().rectangle_area = Some(RectangleArea::build(min, max));
    }

    pub fn set_title_position(&self, p: Point) {
        self.get_mut().xy_title = Some(p);
    }

    pub fn title_and_attribute_width(&self) -> f64 {
        self.get().title_and_attribute.0
    }

    pub fn title_and_attribute_height(&self) -> f64 {
        self.get().title_and_attribute.1
    }

    pub fn cluster_id(&self) -> String {
        format!("cluster{}", self.get().color)
    }

    pub fn special_point_id(group: &Entity) -> String {
        format!("{CENTER_ID}{}", group.uid())
    }

    pub fn move_delta(&self, dx: f64, dy: f64) {
        let mut c = self.get_mut();
        for p in [&mut c.xy_note_top, &mut c.xy_note_bottom, &mut c.xy_title]
            .into_iter()
            .flatten()
        {
            p.0 += dx;
            p.1 += dy;
        }
        if let Some(r) = c.rectangle_area {
            c.rectangle_area = Some(r.move_by(dx, dy));
        }
    }

    fn is_normal_position(n: &SvekNode) -> bool {
        n.get().entity_position == EntityPosition::Normal
    }

    /// Nodes that start an inverted edge come first (`getNodesOrderedTop`).
    fn nodes_ordered_top(&self, lines: &[SvekEdge]) -> Vec<SvekNode> {
        let nodes = self.nodes();
        let mut firsts: Vec<SvekNode> = Vec::new();
        for l in lines {
            if l.is_inverted() {
                let prefix = l.start_uid_prefix();
                if let Some(sh) = nodes.iter().find(|n| n.uid() == prefix) {
                    if Self::is_normal_position(sh) {
                        firsts.insert(0, sh.clone());
                    }
                }
            }
        }
        firsts
    }

    fn nodes_ordered_without_top(&self, lines: &[SvekEdge]) -> Vec<SvekNode> {
        let mut all: Vec<SvekNode> = self
            .nodes()
            .into_iter()
            .filter(Self::is_normal_position)
            .collect();
        for l in lines {
            if l.is_inverted() {
                let prefix = l.start_uid_prefix();
                if let Some(pos) = all.iter().position(|n| n.uid() == prefix) {
                    all.remove(pos);
                }
            }
        }
        all
    }

    /// First part of the DOT body: the nodes that must come first (`printCluster1`).
    pub fn print_cluster1(&self, sb: &mut String, lines: &[SvekEdge]) -> bool {
        let tmp = self.nodes_ordered_top(lines);
        if tmp.is_empty() {
            return false;
        }
        for n in tmp {
            n.append_shape(sb);
        }
        true
    }

    /// Second part: the other nodes and the sub clusters (`printCluster2`).
    pub fn print_cluster2(&self, sb: &mut String, lines: &[SvekEdge], use_rank_same: bool) {
        for node in self.nodes_ordered_without_top(lines) {
            node.append_shape(sb);
        }
        if use_rank_same {
            for same in self.rank_same(lines) {
                sb.push_str(&same);
                sb.push('\n');
            }
        }
        for child in self.children() {
            child.print_internal(sb, lines);
        }
    }

    fn rank_same(&self, lines: &[SvekEdge]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let nodes = self.nodes();
        let in_cluster = |uid: &str| nodes.iter().any(|n| n.uid() == uid);
        for l in lines {
            if l.has_entry_point() {
                continue;
            }
            if in_cluster(&l.start_uid_prefix()) && in_cluster(&l.end_uid_prefix()) {
                if let Some(same) = l.rank_same() {
                    if !out.contains(&same) {
                        out.push(same);
                    }
                }
            }
        }
        out
    }

    fn print_internal(&self, _sb: &mut String, _lines: &[SvekEdge]) {
        // Clusters of packages are added with `ClusterDotString`.
    }
}
