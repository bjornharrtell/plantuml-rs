//! State shared by diagrams made of entities and links (`CucaDiagram`).

use crate::abel::bodier::{Bodier, HiddenVisibilities};
use crate::abel::entity::EntityKind;
use crate::abel::gender::EntityGender;
use crate::abel::hide::HideOrShow;
use crate::abel::{Entity, EntityPortion, GroupType, LeafType, Link, LinkConstraint, Together};
use crate::command::CommandExecutionResult;
use crate::diagram::DiagramBase;
use crate::klimt::creole::display::Display;
use crate::plasma::{Plasma, Quark};
use crate::skin::visibility::VisibilityModifier;
use crate::stereo::{Guillemet, Stereotype};
use crate::text::LineLocation;
use std::rc::Rc;

/// A diagram that owns a [`CucaDiagram`], so commands can be shared between diagram types.
pub trait CucaHolder: crate::command::common::HasBase {
    fn cuca(&self) -> &CucaDiagram;
    fn cuca_mut(&mut self) -> &mut CucaDiagram;
}

/// What the stack of open groups holds (`Bag`).
#[derive(Debug, Clone)]
pub enum Bag {
    Entity(Entity),
    Together(Rc<Together>),
}

struct EntityHideOrShow {
    gender: EntityGender,
    portion: EntityPortion,
    show: bool,
}

pub struct CucaDiagram {
    pub base: DiagramBase,
    hide_or_shows: Vec<EntityHideOrShow>,
    hide_visibility: HiddenVisibilities,
    hides2: Vec<HideOrShow>,
    removed: Vec<HideOrShow>,
    cpt1: i32,
    cpt2: i32,
    stacks: Vec<Bag>,
    visibility_modifier_present: bool,
    links: Vec<Link>,
    namespace: Plasma<Entity>,
    root: Quark<Entity>,
    raw_layout: i32,
    last_entity: Option<Entity>,
    namespace_separator: Option<String>,
    pub warning_or_error: Option<String>,
}

impl CucaDiagram {
    pub fn new(base: DiagramBase) -> CucaDiagram {
        let namespace: Plasma<Entity> = Plasma::new();
        let root = namespace.root();
        let root_entity = Entity::new(
            None,
            root.clone(),
            0,
            Bodier::create_group(),
            EntityKind::Group(GroupType::Root),
            0,
        );
        CucaDiagram {
            base,
            hide_or_shows: Vec::new(),
            hide_visibility: Rc::default(),
            hides2: Vec::new(),
            removed: Vec::new(),
            cpt1: 0,
            cpt2: 0,
            stacks: vec![Bag::Entity(root_entity)],
            visibility_modifier_present: false,
            links: Vec::new(),
            namespace,
            root,
            raw_layout: 0,
            last_entity: None,
            namespace_separator: None,
            warning_or_error: None,
        }
    }

    pub fn namespace_separator(&self) -> Option<&str> {
        self.namespace_separator.as_deref()
    }

    pub fn set_namespace_separator(&mut self, sep: Option<&str>) {
        self.namespace_separator = sep.map(str::to_string);
        self.namespace.set_separator(sep);
    }

    pub fn starting_pass(&mut self) {
        self.last_entity = None;
        self.cpt2 = 0;
        self.stacks.truncate(1);
    }

    pub fn root_group(&self) -> Entity {
        self.root.data().expect("root entity")
    }

    pub fn current_group(&self) -> Entity {
        for bag in self.stacks.iter().rev() {
            if let Bag::Entity(e) = bag {
                return e.clone();
            }
        }
        panic!("no current group");
    }

    pub fn current_together(&self) -> Option<Rc<Together>> {
        match self.stacks.last() {
            Some(Bag::Together(t)) => Some(t.clone()),
            _ => None,
        }
    }

    pub fn clean_id(&self, id: Option<&str>) -> Option<String> {
        id.map(crate::string_utils::eventually_remove_starting_and_ending_double_quote)
    }

    pub fn set_last_entity(&mut self, e: Option<Entity>) {
        self.last_entity = e;
    }

    pub fn last_entity(&self) -> Option<Entity> {
        self.last_entity.clone()
    }

    pub fn unique_sequence_value(&mut self) -> i32 {
        self.cpt1 += 1;
        self.cpt1
    }

    pub fn unique_sequence(&mut self, prefix: &str) -> String {
        self.cpt1 += 1;
        format!("{prefix}{}", self.cpt1)
    }

    pub fn unique_sequence2(&mut self, prefix: &str) -> String {
        self.cpt2 += 1;
        format!("{prefix}{}", self.cpt2)
    }

    pub fn hidden_visibilities(&self) -> HiddenVisibilities {
        self.hide_visibility.clone()
    }

    pub fn raw_layout(&self) -> i32 {
        self.raw_layout
    }

    pub fn inc_raw_layout(&mut self) {
        self.raw_layout += 1;
    }

    pub fn is_visibility_modifier_present(&self) -> bool {
        self.visibility_modifier_present
    }

    pub fn set_visibility_modifier_present(&mut self, v: bool) {
        self.visibility_modifier_present = v;
    }

    // quarks

    pub fn quarks(&self) -> Vec<Quark<Entity>> {
        self.namespace.quarks()
    }

    pub fn first_with_name(&self, name: &str) -> Option<Quark<Entity>> {
        self.namespace.first_with_name(name)
    }

    pub fn count_by_name(&self, name: &str) -> usize {
        self.namespace.count_by_name(name)
    }

    /// `quarkInContextSafe`: the quark for `full` as seen from the current group.
    pub fn quark_in_context(
        &self,
        reuse_existing_child: bool,
        full: &str,
    ) -> Result<Quark<Entity>, (String, i32)> {
        let Some(sep) = self.namespace_separator() else {
            if let Some(q) = self.first_with_name(full) {
                return Ok(q);
            }
            return Ok(self.current_group().quark().child(full));
        };
        if full.ends_with(sep) || full.contains(&format!("{sep}{sep}")) {
            return Err((format!("Bad name since {sep} is a separator"), 3));
        }
        let current = self.current_group().quark();
        if let Some(rest) = full.strip_prefix(sep) {
            return Ok(self.root.child(rest));
        }
        let Some(x) = full.find(sep) else {
            if reuse_existing_child && self.count_by_name(full) == 1 {
                let by_name = self.first_with_name(full).expect("counted");
                if by_name != current {
                    return Ok(by_name);
                }
            }
            return Ok(current.child(full));
        };
        let first = self.root.child_if_exists(&full[..x]);
        if let Some(f) = &first {
            if let Some(d) = f.data() {
                if !d.is_group() {
                    return Err((format!("Not a package: {}", &full[..x]), 0));
                }
            }
            return Ok(self.root.child(full));
        }
        Ok(current.child(full))
    }

    pub fn remove_port_id(&self, id: &str) -> String {
        if self.namespace_separator() == Some("::") {
            return id.to_string();
        }
        match id.rfind("::") {
            None => id.to_string(),
            Some(x) => id[..x].to_string(),
        }
    }

    pub fn port_id(&self, id: &str) -> Option<String> {
        if self.namespace_separator() == Some("::") {
            return None;
        }
        id.rfind("::").map(|x| id[x + 2..].to_string())
    }

    pub fn port_for(&self, ent_string: &str, ident: &Quark<Entity>) -> Option<String> {
        let x = ent_string.rfind("::")?;
        if ent_string.starts_with(ident.name()) {
            Some(ent_string[x + 2..].to_string())
        } else {
            None
        }
    }

    // creation

    pub fn create_leaf(
        &mut self,
        location: Option<LineLocation>,
        quark: Quark<Entity>,
        leaf_type: LeafType,
    ) -> Entity {
        let bodier = Bodier::create_leaf(leaf_type, self.hide_visibility.clone());
        let uid = self.unique_sequence_value();
        Entity::new(
            location,
            quark,
            uid,
            bodier,
            EntityKind::Leaf(leaf_type),
            self.raw_layout,
        )
    }

    pub fn create_group(
        &mut self,
        location: Option<LineLocation>,
        quark: Quark<Entity>,
        group_type: GroupType,
    ) -> Entity {
        if let Some(d) = quark.data() {
            return d;
        }
        let uid = self.unique_sequence_value();
        Entity::new(
            location,
            quark,
            uid,
            Bodier::create_group(),
            EntityKind::Group(group_type),
            self.raw_layout,
        )
    }

    /// `reallyCreateLeaf`.
    pub fn really_create_leaf(
        &mut self,
        location: Option<LineLocation>,
        ident: Quark<Entity>,
        display: Display,
        leaf_type: LeafType,
    ) -> Entity {
        assert!(ident.data().is_none());
        assert!(!display.is_null());
        let result = self.create_leaf(location, ident, leaf_type);
        self.last_entity = Some(result.clone());
        result.set_together(self.current_together());
        result.set_display(display);
        if leaf_type.is_like_class() {
            self.eventually_build_phantom_groups(location);
        }
        result
    }

    pub fn eventually_build_phantom_groups(&mut self, location: Option<LineLocation>) {
        for quark in self.quarks() {
            if quark.data().is_some() {
                continue;
            }
            if quark.count_children() > 0 {
                let display = Display::with_newlines(Some(quark.name()));
                let g = self.create_group(location, quark, GroupType::Package);
                g.set_display(display);
            }
        }
    }

    pub fn goto_together(&mut self) -> CommandExecutionResult {
        let parent = self.current_together();
        self.stacks
            .push(Bag::Together(Rc::new(Together { parent })));
        CommandExecutionResult::ok()
    }

    pub fn goto_group(
        &mut self,
        location: Option<LineLocation>,
        quark: Quark<Entity>,
        display: Display,
        group_type: GroupType,
        usymbol: Option<Rc<dyn crate::decoration::usymbol::USymbol>>,
    ) -> CommandExecutionResult {
        if quark.data().is_none() {
            let g = self.create_group(location, quark.clone(), group_type);
            g.set_together(self.current_together());
            g.set_display(display);
        }
        let ent = quark.data().expect("created");
        ent.mute_to_group_type(group_type);
        if usymbol.is_some() {
            ent.set_usymbol(usymbol);
        }
        self.stacks.push(Bag::Entity(ent));
        CommandExecutionResult::ok()
    }

    pub fn end_group(&mut self) -> bool {
        if self.stacks.len() <= 1 {
            return false;
        }
        self.stacks.pop();
        true
    }

    pub fn group_by_code(&self, code: &str) -> Option<Entity> {
        self.first_with_name(code).and_then(|q| q.data())
    }

    pub fn is_group_code(&self, code: &str) -> bool {
        self.first_with_name(code)
            .is_some_and(|q| Self::is_group_quark(&q))
    }

    pub fn is_group_quark(q: &Quark<Entity>) -> bool {
        q.data().is_some_and(|e| e.is_group())
    }

    pub fn children_groups(&self, entity: &Entity) -> Vec<Entity> {
        entity.groups()
    }

    pub fn leafs(&self) -> Vec<Entity> {
        self.quarks()
            .into_iter()
            .filter(|q| !q.is_root())
            .filter_map(|q| q.data())
            .filter(|e| !e.is_group())
            .collect()
    }

    pub fn groups(&self) -> Vec<Entity> {
        self.quarks()
            .into_iter()
            .filter(|q| !q.is_root())
            .filter_map(|q| q.data())
            .filter(Entity::is_group)
            .collect()
    }

    pub fn groups_and_root(&self) -> Vec<Entity> {
        self.quarks()
            .into_iter()
            .filter_map(|q| q.data())
            .filter(Entity::is_group)
            .collect()
    }

    pub fn has_url(&self) -> bool {
        self.quarks()
            .iter()
            .any(|q| q.data().is_some_and(|e| e.has_url()))
    }

    // links

    pub fn links(&self) -> &[Link] {
        &self.links
    }

    pub fn add_link(&mut self, link: Link) {
        if link.is_single() && self.links.iter().any(|l| link.same_connections(l)) {
            return;
        }
        self.links.push(link);
    }

    pub fn remove_link(&mut self, link: &Link) {
        let pos = self
            .links
            .iter()
            .position(|l| l == link)
            .expect("link present");
        self.links.remove(pos);
    }

    pub fn last_link(&self) -> Option<Link> {
        self.links
            .iter()
            .rev()
            .find(|l| {
                l.entity1().leaf_type() != Some(LeafType::Note)
                    && l.entity2().leaf_type() != Some(LeafType::Note)
            })
            .cloned()
    }

    pub fn two_last_links(&self) -> Option<Vec<Link>> {
        let mut result = Vec::new();
        for l in self.links.iter().rev() {
            if l.entity1().leaf_type() != Some(LeafType::Note)
                && l.entity2().leaf_type() != Some(LeafType::Note)
            {
                result.push(l.clone());
                if result.len() == 2 {
                    return Some(result);
                }
            }
        }
        None
    }

    pub fn is_standalone(&self, ent: &Entity) -> bool {
        !self.links.iter().any(|l| l.contains(ent))
    }

    pub fn is_standalone_for_argo(&self, ent: &Entity) -> bool {
        !self
            .links
            .iter()
            .any(|l| !(self.is_link_hidden(l) || l.is_invis()) && l.contains(ent))
    }

    pub fn constraint_on_links(
        &mut self,
        link1: &Link,
        link2: &Link,
        display: Display,
    ) -> CommandExecutionResult {
        let c = LinkConstraint { display };
        link1.set_link_constraint(Some(c.clone()));
        link2.set_link_constraint(Some(c));
        CommandExecutionResult::ok()
    }

    // hide / show / remove

    pub fn hide_or_show(&mut self, gender: EntityGender, portion: EntityPortion, show: bool) {
        for p in portion.as_set() {
            self.hide_or_shows.push(EntityHideOrShow {
                gender: gender.clone(),
                portion: p,
                show,
            });
        }
    }

    pub fn hide_or_show2(&mut self, what: &str, show: bool) {
        let what = self.fix_what(what);
        self.hides2.push(HideOrShow::new(&what, show));
    }

    pub fn remove_or_restore(&mut self, what: &str, show: bool) {
        let what = self.fix_what(what);
        self.removed.push(HideOrShow::new(&what, show));
    }

    fn fix_what(&self, what: &str) -> String {
        if let Some(sep) = self.namespace_separator() {
            let cq = self.current_group().quark();
            if !cq.qualified_name().is_empty() {
                return format!("{}{}{}", cq.qualified_name(), sep, what);
            }
        }
        what.to_string()
    }

    pub fn hide_or_show_visibility_modifier(
        &mut self,
        visibilities: &[VisibilityModifier],
        show: bool,
    ) {
        let mut set = self.hide_visibility.borrow_mut();
        for v in visibilities {
            if show {
                set.remove(v);
            } else {
                set.insert(*v);
            }
        }
    }

    pub fn show_portion(&self, portion: EntityPortion, entity: &Entity) -> bool {
        if self.base.skin_param.strict_uml_style() && portion == EntityPortion::CircledCharacter {
            return false;
        }
        let mut result = true;
        for cmd in &self.hide_or_shows {
            if cmd.portion == portion && cmd.gender.contains(entity) {
                result = cmd.show;
            }
        }
        result
    }

    pub fn visible_stereotype_labels(&self, entity: &Entity) -> Option<Vec<String>> {
        let st = entity.stereotype()?;
        Some(
            st.labels(Guillemet::DoubleComparator)
                .into_iter()
                .filter(|l| self.is_stereotype_label_shown(l))
                .collect(),
        )
    }

    fn is_stereotype_label_shown(&self, label: &str) -> bool {
        let mut result = true;
        for cmd in &self.hide_or_shows {
            if cmd.portion != EntityPortion::Stereotype {
                continue;
            }
            match cmd.gender.gender() {
                None => result = cmd.show,
                Some(g) if g == label => result = cmd.show,
                _ => {}
            }
        }
        result
    }

    pub fn is_stereotype_removed(&self, stereotype: &Stereotype) -> bool {
        let mut result = false;
        for h in &self.removed {
            result = h.apply_stereotype(result, Some(stereotype));
        }
        result
    }

    pub fn is_alone_and_unlinked(&self, entity: &Entity) -> bool {
        if entity.is_group() {
            return entity
                .quark()
                .children()
                .iter()
                .filter_map(Quark::data)
                .all(|c| self.is_alone_and_unlinked(&c));
        }
        for link in &self.links {
            if link.contains(entity) {
                let other = link.other(entity);
                let removed = self.is_removed_ignore_unlinked(&other);
                if !removed && !link.type_().is_invisible() {
                    return false;
                }
            }
        }
        true
    }

    fn note_with_single_link_attached_to(&self, note: &Entity) -> Option<Entity> {
        if note.leaf_type() != Some(LeafType::Note) {
            return None;
        }
        let mut other: Option<Entity> = None;
        for link in &self.links {
            if link.type_().is_invisible() || !link.contains(note) {
                continue;
            }
            if other.is_some() {
                return None;
            }
            let o = link.other(note);
            if o.leaf_type() == Some(LeafType::Note) {
                return None;
            }
            other = Some(o);
        }
        other
    }

    fn is_orphaned_transition_label(&self, leaf: &Entity) -> bool {
        if leaf.leaf_type() != Some(LeafType::StateTransitionLabel) {
            return false;
        }
        for link in &self.links {
            if !link.contains(leaf) {
                continue;
            }
            let other = link.other(leaf);
            if other != *leaf && (self.is_entity_removed(&other) || self.is_entity_hidden(&other)) {
                return true;
            }
        }
        false
    }

    /// `Entity.isHidden`.
    pub fn is_entity_hidden(&self, e: &Entity) -> bool {
        if e.is_root() {
            return false;
        }
        let parent = e.parent_container();
        if parent.as_ref() == Some(e) {
            return false;
        }
        if let Some(p) = &parent {
            if self.is_entity_hidden(p) {
                return true;
            }
        }
        self.is_hidden(e)
    }

    /// `Entity.isRemoved`.
    pub fn is_entity_removed(&self, e: &Entity) -> bool {
        if e.is_root() {
            return false;
        }
        let parent = e.parent_container();
        if parent.as_ref() == Some(e) {
            return false;
        }
        if let Some(p) = &parent {
            if self.is_entity_removed(p) {
                return true;
            }
        }
        self.is_removed(e)
    }

    pub fn is_link_hidden(&self, l: &Link) -> bool {
        l.is_hidden_flag()
            || self.is_entity_hidden(&l.entity1())
            || self.is_entity_hidden(&l.entity2())
    }

    pub fn is_link_removed(&self, l: &Link) -> bool {
        if let Some(st) = l.stereotype() {
            if self.is_stereotype_removed(&st) {
                return true;
            }
        }
        self.is_entity_removed(&l.entity1()) || self.is_entity_removed(&l.entity2())
    }

    pub fn is_hidden(&self, leaf: &Entity) -> bool {
        if leaf.is_root() {
            return false;
        }
        if let Some(other) = self.note_with_single_link_attached_to(leaf) {
            if other != *leaf {
                return self.is_hidden(&other);
            }
        }
        let alone = |e: &Entity| self.is_alone_and_unlinked(e);
        let mut hidden = false;
        for h in &self.hides2 {
            hidden = h.apply(hidden, leaf, &alone);
        }
        if !hidden && self.is_orphaned_transition_label(leaf) {
            hidden = true;
        }
        hidden
    }

    pub fn is_removed(&self, leaf: &Entity) -> bool {
        if leaf.is_root() {
            return false;
        }
        if let Some(other) = self.note_with_single_link_attached_to(leaf) {
            return self.is_removed(&other);
        }
        let alone = |e: &Entity| self.is_alone_and_unlinked(e);
        let mut result = false;
        for h in &self.removed {
            result = h.apply(result, leaf, &alone);
        }
        if !result && self.is_orphaned_transition_label(leaf) {
            result = true;
        }
        result
    }

    pub fn is_removed_ignore_unlinked(&self, leaf: &Entity) -> bool {
        let alone = |e: &Entity| self.is_alone_and_unlinked(e);
        let mut result = false;
        for h in &self.removed {
            if !h.is_about_unlinked() {
                result = h.apply(result, leaf, &alone);
            }
        }
        result
    }

    /// `Entity.isEmpty`: no child left after removals.
    pub fn is_empty(&self, entity: &Entity) -> bool {
        entity
            .quark()
            .children()
            .iter()
            .filter_map(Quark::data)
            .all(|c| self.is_entity_removed(&c))
    }

    /// Distance and angle of labels (`getLabeldistance`, `getLabelangle`).
    pub fn label_distance(&self) -> String {
        for key in ["labeldistance", "defaultlabeldistance"] {
            if let Some(s) = self.base.pragma.get(key) {
                if is_number(s) {
                    return s.clone();
                }
            }
        }
        "1.7".to_string()
    }

    pub fn label_angle(&self) -> String {
        for key in ["labelangle", "defaultlabelangle"] {
            if let Some(s) = self.base.pragma.get(key) {
                if is_number(s) {
                    return s.clone();
                }
            }
        }
        "25".to_string()
    }

    pub fn reset_pragma_label(&mut self) {
        self.base.pragma.remove("labeldistance");
        self.base.pragma.remove("labelangle");
    }
}

fn is_number(s: &str) -> bool {
    static P: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[+-]?(\.?\d+|\d+\.\d*)$").expect("valid"));
    P.is_match(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::UmlSource;
    use crate::diagram::DiagramType;

    fn diagram() -> CucaDiagram {
        let mut d = CucaDiagram::new(DiagramBase::new(
            UmlSource::from_text("@startuml\n@enduml"),
            DiagramType::Class,
        ));
        d.set_namespace_separator(Some("."));
        d
    }

    #[test]
    fn leaves_and_packages() {
        let mut d = diagram();
        let q = d.quark_in_context(false, "a.B").unwrap();
        let e = d.really_create_leaf(None, q, Display::create(&["B"]), LeafType::Class);
        assert_eq!(e.qualified_name(), "a.B");
        assert_eq!(d.leafs().len(), 1);
        assert_eq!(d.groups().len(), 1);
        assert_eq!(e.parent_container().unwrap().name(), "a");
        assert!(d.quark_in_context(false, "x.").is_err());
    }

    #[test]
    fn hide_and_remove() {
        let mut d = diagram();
        let q = d.quark_in_context(false, "A").unwrap();
        let a = d.really_create_leaf(None, q, Display::create(&["A"]), LeafType::Class);
        assert!(!d.is_hidden(&a));
        d.hide_or_show2("A", false);
        assert!(d.is_hidden(&a));
        d.remove_or_restore("A", false);
        assert!(d.is_removed(&a));
    }

    #[test]
    fn labels() {
        let d = diagram();
        assert_eq!(d.label_distance(), "1.7");
        assert_eq!(d.label_angle(), "25");
    }
}
