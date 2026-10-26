//! Square packing of standalone entities (`Magma`, `MagmaList`, `SquareMaker`).
//!
//! Groups of three or more entities without any link are arranged in a square by
//! adding invisible links, so that the layout engine does not put them all on one row.

use crate::abel::{Entity, Link, LinkArg};
use crate::cucadiagram::CucaDiagram;
use crate::decoration::link::{LinkDecor, LinkType};

/// Number of entities per row (`SquareMaker.computeBranch`).
fn compute_branch(size: usize) -> usize {
    let r = (size as f64).sqrt() as usize;
    if r * r == size {
        r
    } else {
        r + 1
    }
}

/// Index of the first entity of the last row (`SquareMaker.getBottomLeft`).
fn bottom_left(size: usize) -> usize {
    let s = compute_branch(size);
    ((size - 1) / s) * s
}

/// The links of a square: `(from, to, length)` over `0..size`.
fn square_links(size: usize) -> Vec<(usize, usize, i32)> {
    let branch = compute_branch(size);
    let mut result = Vec::new();
    let mut head_branch = 0;
    for i in 1..size {
        if i - head_branch == branch {
            result.push((head_branch, i, 2));
            head_branch = i;
        } else {
            result.push((i - 1, i, 1));
        }
    }
    result
}

/// A square of standalone entities (`Magma`).
struct Magma {
    standalones: Vec<Entity>,
}

impl Magma {
    /// The group the square belongs to, one level above the entity parent (`Magma.getContainer`).
    fn container(&self) -> Option<Entity> {
        self.standalones
            .first()?
            .parent_container()?
            .parent_container()
    }

    fn top_left(&self) -> Entity {
        self.standalones[0].clone()
    }

    fn top_right(&self) -> Entity {
        self.standalones[compute_branch(self.standalones.len()) - 1].clone()
    }

    fn bottom_left(&self) -> Entity {
        self.standalones[bottom_left(self.standalones.len())].clone()
    }
}

impl CucaDiagram {
    /// `applySingleStrategy`: pack standalone entities of each group in squares.
    pub fn apply_single_strategy(&mut self) {
        let groups = self.groups_and_root();
        let mut magmas: Vec<Magma> = Vec::new();
        let mut links: Vec<(Entity, Entity, i32)> = Vec::new();

        for g in &groups {
            let standalones: Vec<Entity> = g
                .leafs()
                .into_iter()
                .filter(|e| self.is_standalone(e))
                .collect();
            if standalones.len() < 3 {
                continue;
            }
            for (a, b, len) in square_links(standalones.len()) {
                links.push((standalones[a].clone(), standalones[b].clone(), len));
            }
            magmas.push(Magma { standalones });
        }

        for g in &groups {
            let group: Vec<&Magma> = magmas
                .iter()
                .filter(|m| m.container().as_ref() == Some(g))
                .collect();
            if group.len() < 3 {
                continue;
            }
            for (a, b, len) in square_links(group.len()) {
                let link = if len == 2 {
                    (group[a].bottom_left(), group[b].top_left())
                } else {
                    (group[a].top_right(), group[b].top_left())
                };
                links.push((link.0, link.1, len));
            }
        }

        for (from, to, len) in links {
            let uid = self.unique_sequence("lnk");
            let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).invisible();
            self.add_link(Link::new(None, uid, from, to, link_type, LinkArg::no_display(len)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_are_square() {
        assert_eq!(compute_branch(3), 2);
        assert_eq!(compute_branch(4), 2);
        assert_eq!(compute_branch(5), 3);
        assert_eq!(compute_branch(9), 3);
        assert_eq!(bottom_left(3), 2);
        assert_eq!(bottom_left(5), 3);
    }
}
