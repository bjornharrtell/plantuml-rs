//! Predicates selecting entities (`EntityGender`, `EntityGenderUtils`).

use super::{Entity, LeafType};
use crate::stereo::Guillemet;
use std::rc::Rc;

#[derive(Clone)]
pub struct EntityGender {
    contains: Rc<dyn Fn(&Entity) -> bool>,
    gender: Option<String>,
}

impl EntityGender {
    pub fn contains(&self, e: &Entity) -> bool {
        (self.contains)(e)
    }

    pub fn gender(&self) -> Option<&str> {
        self.gender.as_deref()
    }

    pub fn by_entity_type(t: LeafType) -> EntityGender {
        EntityGender {
            contains: Rc::new(move |e| e.leaf_type() == Some(t)),
            gender: Some(format!("{t:?}")),
        }
    }

    pub fn by_entity_alone(entity: &Entity) -> EntityGender {
        let uid = entity.uid();
        EntityGender {
            contains: Rc::new({
                let uid = uid.clone();
                move |e| e.uid() == uid
            }),
            gender: Some(uid),
        }
    }

    pub fn by_stereotype(stereotype: &str) -> EntityGender {
        let s = stereotype.to_string();
        EntityGender {
            contains: Rc::new({
                let s = s.clone();
                move |e| {
                    e.stereotype()
                        .is_some_and(|st| st.labels(Guillemet::DoubleComparator).contains(&s))
                }
            }),
            gender: Some(s),
        }
    }

    pub fn by_package(group: &Entity) -> EntityGender {
        assert!(!group.is_root());
        let group = group.clone();
        EntityGender {
            contains: Rc::new(move |e| match e.parent_container() {
                Some(p) => !p.is_root() && p == group,
                None => false,
            }),
            gender: None,
        }
    }

    pub fn and(g1: EntityGender, g2: EntityGender) -> EntityGender {
        EntityGender {
            contains: Rc::new(move |e| g1.contains(e) && g2.contains(e)),
            gender: None,
        }
    }

    pub fn all() -> EntityGender {
        EntityGender {
            contains: Rc::new(|_| true),
            gender: None,
        }
    }

    pub fn empty_methods() -> EntityGender {
        EntityGender {
            contains: Rc::new(|e| e.get().bodier.methods_to_display().is_empty()),
            gender: None,
        }
    }

    pub fn empty_fields() -> EntityGender {
        EntityGender {
            contains: Rc::new(|e| e.get().bodier.fields_to_display().is_empty()),
            gender: None,
        }
    }

    pub fn by_class_name(name: &str) -> EntityGender {
        let n = name.to_string();
        EntityGender {
            contains: Rc::new({
                let n = n.clone();
                move |e| e.name() == n
            }),
            gender: Some(n),
        }
    }
}
