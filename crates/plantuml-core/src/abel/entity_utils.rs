//! Relations between groups and links (`EntityUtils`, `Entity.isAutarkic`).

use super::{Entity, EntityPosition, GroupType, Link};

fn is_parent(group_to_be_tested: Option<Entity>, parent_group: &Entity) -> bool {
    let Some(mut current) = group_to_be_tested else {
        return false;
    };
    if !current.is_group() {
        return false;
    }
    while !current.is_root() {
        if &current == parent_group {
            return true;
        }
        match current.parent_container() {
            Some(p) if p.is_group() => current = p,
            _ => return false,
        }
    }
    false
}

/// Both ends of the link are inside the group (`isPureInnerLink12`).
pub fn is_pure_inner_link12(group: &Entity, link: &Link) -> bool {
    is_parent(link.entity1().parent_container(), group)
        && is_parent(link.entity2().parent_container(), group)
}

/// Both ends are inside the group, or both outside (`isPureInnerLink3`).
pub fn is_pure_inner_link3(group: &Entity, link: &Link) -> bool {
    is_parent(link.entity2().parent_container(), group)
        == is_parent(link.entity1().parent_container(), group)
}

/// A group that can be laid out on its own (`Entity.isAutarkic`).
pub fn is_autarkic(group: &Entity, links: &[Link]) -> bool {
    match group.group_type() {
        Some(GroupType::Package) => return false,
        Some(
            GroupType::InnerActivity | GroupType::ConcurrentActivity | GroupType::ConcurrentState,
        ) => return true,
        _ => {}
    }
    if links.iter().any(|l| !is_pure_inner_link3(group, l)) {
        return false;
    }
    group
        .leafs()
        .iter()
        .all(|l| l.entity_position() == EntityPosition::Normal)
}
