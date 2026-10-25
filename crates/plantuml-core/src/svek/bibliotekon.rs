//! Registry of the nodes, edges and clusters of one layout (`Bibliotekon`).

use super::cluster::Cluster;
use super::edge::SvekEdge;
use super::node::SvekNode;
use super::types::{ColorSequence, EntityPort};
use crate::abel::{Entity, Link};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

pub struct BibData {
    pub all_cluster: Vec<Cluster>,
    pub nodes: Vec<(Entity, SvekNode)>,
    pub lines0: Vec<SvekEdge>,
    pub lines1: Vec<SvekEdge>,
    pub all_lines: Vec<SvekEdge>,
    pub color_sequence: ColorSequence,
}

#[derive(Clone)]
pub struct Bibliotekon(Rc<RefCell<BibData>>);

impl Default for Bibliotekon {
    fn default() -> Self {
        Bibliotekon::new()
    }
}

impl Bibliotekon {
    pub fn new() -> Bibliotekon {
        Bibliotekon(Rc::new(RefCell::new(BibData {
            all_cluster: Vec::new(),
            nodes: Vec::new(),
            lines0: Vec::new(),
            lines1: Vec::new(),
            all_lines: Vec::new(),
            color_sequence: ColorSequence::default(),
        })))
    }

    pub fn get(&self) -> Ref<'_, BibData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, BibData> {
        self.0.borrow_mut()
    }

    pub fn next_color(&self) -> i32 {
        self.get_mut().color_sequence.get_value()
    }

    pub fn add_node(&self, entity: Entity, node: SvekNode) {
        let mut d = self.get_mut();
        if let Some(e) = d.nodes.iter_mut().find(|(e, _)| *e == entity) {
            e.1 = node;
        } else {
            d.nodes.push((entity, node));
        }
    }

    pub fn node(&self, ent: &Entity) -> Option<SvekNode> {
        self.get()
            .nodes
            .iter()
            .find(|(e, _)| e == ent)
            .map(|(_, n)| n.clone())
    }

    pub fn all_nodes(&self) -> Vec<SvekNode> {
        self.get().nodes.iter().map(|(_, n)| n.clone()).collect()
    }

    pub fn leaf_of(&self, node: &SvekNode) -> Option<Entity> {
        self.get()
            .nodes
            .iter()
            .find(|(_, n)| n == node)
            .map(|(e, _)| e.clone())
    }

    pub fn cluster_of(&self, ent: &Entity) -> Option<Cluster> {
        self.get()
            .all_cluster
            .iter()
            .find(|c| c.groups().contains(ent))
            .cloned()
    }

    pub fn add_cluster(&self, c: Cluster) {
        self.get_mut().all_cluster.push(c);
    }

    pub fn all_cluster(&self) -> Vec<Cluster> {
        self.get().all_cluster.clone()
    }

    pub fn add_line(&self, line: SvekEdge) {
        let mut d = self.get_mut();
        d.all_lines.push(line.clone());
        if line.length() == 1 {
            if line.has_note_label_text() {
                for i in 0..d.lines0.len() {
                    let other = d.lines0[i].clone();
                    if !other.has_note_label_text() && line.same_connections(&other) {
                        d.lines0.insert(i, line);
                        return;
                    }
                }
            }
            d.lines0.push(line);
        } else {
            d.lines1.push(line);
        }
    }

    pub fn all_lines(&self) -> Vec<SvekEdge> {
        self.get().all_lines.clone()
    }

    pub fn lines0(&self) -> Vec<SvekEdge> {
        self.get().lines0.clone()
    }

    pub fn lines1(&self) -> Vec<SvekEdge> {
        self.get().lines1.clone()
    }

    pub fn line_connected_to(&self, leaf: &Entity) -> Vec<SvekEdge> {
        self.all_lines()
            .into_iter()
            .filter(|l| l.is_link_from_or_to(leaf))
            .collect()
    }

    pub fn line_of(&self, link: &Link) -> Option<SvekEdge> {
        self.all_lines().into_iter().find(|l| l.is_link(link))
    }

    /// Node id used in edges, with `:h` when the node is shielded (`getNodeUid`).
    pub fn node_uid(&self, ent: &Entity) -> String {
        if let Some(n) = self.node(ent) {
            let mut uid = n.uid();
            if n.is_shielded() {
                uid.push_str(":h");
            }
            return uid;
        }
        if ent.is_group() {
            return Cluster::special_point_id(ent);
        }
        panic!("unknown entity {ent:?}");
    }

    pub fn entity_port(&self, ent: &Entity, port: Option<&str>) -> EntityPort {
        let uid = self.node_uid(ent);
        if ent.entity_position().use_port_p() {
            EntityPort::for_port(&uid)
        } else {
            EntityPort::create(&uid, port.map(encode_port_name))
        }
    }

    pub fn max_x(&self) -> Vec<(String, f64)> {
        self.get()
            .nodes
            .iter()
            .map(|(e, n)| (e.name(), n.min_x() + n.width()))
            .collect()
    }
}

/// `Ports.encodePortNameToId`: "p" and the MD5 of the name.
pub fn encode_port_name(name: &str) -> String {
    format!("p{}", crate::string_utils::md5_hex(name))
}
