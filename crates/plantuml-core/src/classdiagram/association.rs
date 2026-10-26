//! Association classes: `(A, B) .. C` (`AbstractClassOrObjectDiagram.Association`).

use super::ClassDiagram;
use crate::abel::{Entity, LeafType, Link, LinkArg, NoteLinkStrategy};
use crate::command::CommandExecutionResult;
use crate::decoration::link::{LinkDecor, LinkType};
use crate::klimt::creole::display::Display;
use crate::text::LineLocation;

pub struct Association {
    entity1: Entity,
    entity2: Entity,
    associed: Entity,
    point: Entity,
    existing_link: Option<Link>,
    entity1_to_point: Option<Link>,
    point_to_entity2: Option<Link>,
    point_to_associed: Option<Link>,
    other: Option<usize>,
}

impl ClassDiagram {
    fn new_link(
        &mut self,
        loc: Option<LineLocation>,
        e1: &Entity,
        e2: &Entity,
        t: LinkType,
        arg: LinkArg,
    ) -> Link {
        let uid = self.cuca.unique_sequence("lnk");
        Link::new(loc, uid, e1.clone(), e2.clone(), t, arg)
    }

    fn found_link(&self, e1: &Entity, e2: &Entity) -> Option<Link> {
        self.cuca
            .links()
            .iter()
            .rev()
            .find(|l| l.is_between(e1, e2))
            .cloned()
    }

    fn existing_associations(&self, e1: &Entity, e2: &Entity) -> Vec<usize> {
        self.associations
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                (a.entity1 == *e1 && a.entity2 == *e2) || (a.entity1 == *e2 && a.entity2 == *e1)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn new_point(&mut self, loc: Option<LineLocation>, e1: &Entity, e2: &Entity) -> Entity {
        let id = self.cuca.unique_sequence("apoint");
        let quark = if e1.quark().parent() == e2.quark().parent() {
            e1.quark().parent().expect("parent").child(&id)
        } else {
            match self.cuca.quark_in_context(true, &id) {
                Ok(q) => q,
                Err(_) => self.cuca.current_group().quark().child(&id),
            }
        };
        self.cuca.really_create_leaf(
            loc,
            quark,
            Display::with_newlines(Some("")),
            LeafType::PointForAssociation,
        )
    }

    /// `(a, b) .. associed` (`associationClass` with a mode).
    pub fn association_class(
        &mut self,
        loc: Option<LineLocation>,
        mode: i32,
        e1: &Entity,
        e2: &Entity,
        associed: &Entity,
        link_type: LinkType,
        label: Display,
    ) -> bool {
        let same = self.existing_associations(e1, e2);
        if same.len() > 1 {
            return false;
        }
        let point = self.new_point(loc, e1, e2);
        let mut a = Association {
            entity1: e1.clone(),
            entity2: e2.clone(),
            associed: associed.clone(),
            point,
            existing_link: None,
            entity1_to_point: None,
            point_to_entity2: None,
            point_to_associed: None,
            other: None,
        };
        if same.is_empty() {
            self.create_new(&mut a, loc, mode, link_type, label);
            self.associations.push(a);
        } else {
            let idx = same[0];
            // createSecondAssociation
            a.existing_link = self.associations[idx].existing_link.clone();
            a.other = Some(idx);
            if a.existing_link.as_ref().is_some_and(|l| l.length() == 1) {
                let o = &self.associations[idx];
                o.entity1_to_point.as_ref().expect("link").set_length(2);
                o.point_to_entity2.as_ref().expect("link").set_length(2);
                o.point_to_associed.as_ref().expect("link").set_length(1);
            }
            self.create_in_second(&mut a, loc, link_type, label);
            self.associations.push(a);
        }
        true
    }

    fn create_new(
        &mut self,
        a: &mut Association,
        loc: Option<LineLocation>,
        mode: i32,
        link_type: LinkType,
        label: Display,
    ) {
        let existing = match self.found_link(&a.entity1, &a.entity2) {
            None => self.new_link(
                loc,
                &a.entity1,
                &a.entity2,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::no_display(2),
            ),
            Some(l) => {
                self.cuca.remove_link(&l);
                l
            }
        };
        let e1real = existing.entity1();
        let e2real = existing.entity2();
        let (dist, angle) = (self.cuca.label_distance(), self.cuca.label_angle());
        let arg1 = LinkArg::build(existing.label(), existing.length())
            .with_quantifier(existing.quantifier1(), None)
            .with_distance_angle(Some(dist.clone()), Some(angle.clone()));
        let e1_to_point = self.new_link(loc, &e1real, &a.point, existing.type_().part2(), arg1);
        e1_to_point.set_link_arrow(existing.link_arrow());
        let arg2 = LinkArg::no_display(existing.length())
            .with_quantifier(None, existing.quantifier2())
            .with_distance_angle(Some(dist), Some(angle));
        let point_to_e2 = self.new_link(loc, &a.point, &e2real, existing.type_().part1(), arg2);
        let mut length = 1;
        if existing.length() == 1 && a.entity1 != a.entity2 {
            length = 2;
        }
        if existing.length() == 2 && a.entity1 == a.entity2 {
            length = 2;
        }
        if length == 1 {
            e1_to_point.add_note_from(&existing, NoteLinkStrategy::Normal);
        } else {
            e1_to_point.add_note_from(&existing, NoteLinkStrategy::HalfPrintedFull);
            point_to_e2.add_note_from(&existing, NoteLinkStrategy::HalfNotPrinted);
        }
        self.cuca.add_link(e1_to_point.clone());
        self.cuca.add_link(point_to_e2.clone());
        let to_associed = if mode == 1 {
            self.new_link(
                loc,
                &a.point,
                &a.associed,
                link_type,
                LinkArg::build(label, length),
            )
        } else {
            self.new_link(
                loc,
                &a.associed,
                &a.point,
                link_type,
                LinkArg::build(label, length),
            )
        };
        self.cuca.add_link(to_associed.clone());
        a.existing_link = Some(existing);
        a.entity1_to_point = Some(e1_to_point);
        a.point_to_entity2 = Some(point_to_e2);
        a.point_to_associed = Some(to_associed);
    }

    fn create_in_second(
        &mut self,
        a: &mut Association,
        loc: Option<LineLocation>,
        link_type: LinkType,
        label: Display,
    ) {
        let existing = match self.found_link(&a.entity1, &a.entity2) {
            None => self.new_link(
                loc,
                &a.entity1,
                &a.entity2,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::no_display(2),
            ),
            Some(l) => {
                self.cuca.remove_link(&l);
                l
            }
        };
        let (dist, angle) = (self.cuca.label_distance(), self.cuca.label_angle());
        let arg1 = LinkArg::build(existing.label(), 2)
            .with_quantifier(existing.quantifier1(), None)
            .with_distance_angle(Some(dist.clone()), Some(angle.clone()));
        let e1_to_point = self.new_link(loc, &a.entity1, &a.point, existing.type_().part2(), arg1);
        let arg2 = LinkArg::no_display(2)
            .with_quantifier(None, existing.quantifier2())
            .with_distance_angle(Some(dist), Some(angle));
        let point_to_e2 = self.new_link(loc, &a.point, &a.entity2, existing.type_().part1(), arg2);
        self.cuca.add_link(e1_to_point.clone());
        self.cuca.add_link(point_to_e2.clone());
        let other_idx = a.other.expect("second association");
        let other_to_associed = self.associations[other_idx]
            .point_to_associed
            .clone()
            .expect("link");
        if other_to_associed.entity1().leaf_type() == Some(LeafType::PointForAssociation) {
            self.cuca.remove_link(&other_to_associed);
            let uid = self.cuca.unique_sequence("lnk");
            let inv = other_to_associed.inv(uid);
            self.cuca.add_link(inv.clone());
            self.associations[other_idx].point_to_associed = Some(inv);
        }
        let to_associed = self.new_link(
            loc,
            &a.point,
            &a.associed,
            link_type,
            LinkArg::build(label, 1),
        );
        self.cuca.add_link(to_associed.clone());
        let other_point = self.associations[other_idx].point.clone();
        let lnode = self.new_link(
            loc,
            &other_point,
            &a.point,
            LinkType::new(LinkDecor::None, LinkDecor::None),
            LinkArg::no_display(1),
        );
        lnode.set_invis(true);
        self.cuca.add_link(lnode);
        a.existing_link = Some(existing);
        a.entity1_to_point = Some(e1_to_point);
        a.point_to_entity2 = Some(point_to_e2);
        a.point_to_associed = Some(to_associed);
    }

    /// `(a, b) .. (c, d)`: a link between two association points.
    pub fn association_between(
        &mut self,
        loc: Option<LineLocation>,
        e1a: &Entity,
        e1b: &Entity,
        e2a: &Entity,
        e2b: &Entity,
        link_type: LinkType,
        label: Display,
    ) -> CommandExecutionResult {
        let same1 = self.existing_associations(e1a, e1b);
        let same2 = self.existing_associations(e2a, e2b);
        if same1.is_empty() && same2.is_empty() {
            let point1 = self.new_standalone_point(loc);
            let point2 = self.new_standalone_point(loc);
            self.insert_point_between(loc, e1a, e1b, &point1);
            self.insert_point_between(loc, e2a, e2b, &point2);
            let l = self.new_link(loc, &point1, &point2, link_type, LinkArg::build(label, 1));
            self.cuca.add_link(l);
            return CommandExecutionResult::ok();
        }
        CommandExecutionResult::error("Cannot link two associations points")
    }

    fn new_standalone_point(&mut self, loc: Option<LineLocation>) -> Entity {
        let id = self.cuca.unique_sequence("apoint");
        let quark = self.cuca.current_group().quark().child(&id);
        self.cuca.really_create_leaf(
            loc,
            quark,
            Display::with_newlines(Some("")),
            LeafType::PointForAssociation,
        )
    }

    fn insert_point_between(
        &mut self,
        loc: Option<LineLocation>,
        e1a: &Entity,
        e1b: &Entity,
        point: &Entity,
    ) {
        let existing = match self.found_link(e1a, e1b) {
            None => self.new_link(
                loc,
                e1a,
                e1b,
                LinkType::new(LinkDecor::None, LinkDecor::None),
                LinkArg::no_display(2),
            ),
            Some(l) => {
                self.cuca.remove_link(&l);
                l
            }
        };
        let (e1real, e2real) = if existing.is_inverted() {
            (existing.entity2(), existing.entity1())
        } else {
            (existing.entity1(), existing.entity2())
        };
        let (dist, angle) = (self.cuca.label_distance(), self.cuca.label_angle());
        let arg1 = LinkArg::build(existing.label(), existing.length())
            .with_quantifier(existing.quantifier1(), None)
            .with_distance_angle(Some(dist.clone()), Some(angle.clone()));
        let l1 = self.new_link(loc, &e1real, point, existing.type_().part2(), arg1);
        l1.set_link_arrow(existing.link_arrow());
        let arg2 = LinkArg::no_display(existing.length())
            .with_quantifier(None, existing.quantifier2())
            .with_distance_angle(Some(dist), Some(angle));
        let l2 = self.new_link(loc, point, &e2real, existing.type_().part1(), arg2);
        self.cuca.add_link(l1);
        self.cuca.add_link(l2);
    }
}
