//! A node of the model: a leaf (class, state, ...) or a group (package, ...) (`Entity`).

use super::bodier::Bodier;
use super::{CucaNote, EntityPosition, GroupType, LeafType, Stereotag, Tip, Together};
use crate::klimt::color::colors::Colors;
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::geom::Position;
use crate::plasma::Quark;
use crate::skin::visibility::VisibilityModifier;
use crate::stereo::{Guillemet, Stereotype};
use crate::style::Margins;
use crate::text::LineLocation;
use crate::url::Url;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeSet;
use std::rc::Rc;

/// A shape given by a stereotype such as `<<Node>>` (`USymbol`).
#[derive(Clone)]
pub struct SymbolRef(pub Rc<dyn crate::decoration::usymbol::USymbol>);

impl std::fmt::Debug for SymbolRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("USymbol")
    }
}

/// Image given to a group that is drawn as a single block (`overrideImage`).
#[derive(Clone)]
pub struct SvekImageRef(pub Rc<dyn crate::svek::image::IEntityImage>);

impl std::fmt::Debug for SvekImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SvekImage")
    }
}

#[derive(Debug)]
pub struct EntityData {
    pub svek_image: Option<SvekImageRef>,
    pub symbol: Option<SymbolRef>,
    pub quark: Quark<Entity>,
    pub url: Option<Url>,
    pub bodier: Bodier,
    pub uid: String,
    pub display: Display,
    pub legend: Option<DisplayPositioned>,
    pub leaf_type: Option<LeafType>,
    pub stereotype: Option<Stereotype>,
    pub stereostyles: Vec<String>,
    pub generic: Option<String>,
    pub group_type: Option<GroupType>,
    pub margins: Margins,
    pub port_short_names: BTreeSet<String>,
    pub xposition: i32,
    pub raw_layout: i32,
    pub concurrent_separator: char,
    pub location: Option<LineLocation>,
    pub tags: Vec<Stereotag>,
    pub notes_top: Vec<CucaNote>,
    pub notes_bottom: Vec<CucaNote>,
    pub together: Option<Rc<Together>>,
    pub packed: bool,
    pub is_static: bool,
    pub visibility: Option<VisibilityModifier>,
    pub colors: Colors,
    pub tips: Vec<(String, Tip)>,
}

/// Shared handle on an entity; equality is identity.
#[derive(Clone)]
pub struct Entity(Rc<RefCell<EntityData>>);

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.0.borrow();
        write!(
            f,
            "{} {}({:?})[{:?}] {}",
            d.quark.qualified_name(),
            (0..d.display.size())
                .map(|i| d.display.text_at(i))
                .collect::<Vec<_>>()
                .join("\\n"),
            d.leaf_type,
            d.group_type,
            d.uid
        )
    }
}

pub enum EntityKind {
    Leaf(LeafType),
    Group(GroupType),
}

impl Entity {
    pub fn new(
        location: Option<LineLocation>,
        quark: Quark<Entity>,
        unique_value: i32,
        bodier: Bodier,
        kind: EntityKind,
        raw_layout: i32,
    ) -> Entity {
        let uid = if quark.is_root() {
            "entroot".to_string()
        } else {
            crate::string_utils::get_uid("ent", unique_value)
        };
        let (leaf_type, group_type) = match kind {
            EntityKind::Leaf(t) => (Some(t), None),
            EntityKind::Group(g) => (None, Some(g)),
        };
        let e = Entity(Rc::new(RefCell::new(EntityData {
            svek_image: None,
            symbol: None,
            quark: quark.clone(),
            url: None,
            bodier,
            uid,
            display: Display::empty(),
            legend: None,
            leaf_type,
            stereotype: None,
            stereostyles: Vec::new(),
            generic: None,
            group_type,
            margins: Margins::default(),
            port_short_names: BTreeSet::new(),
            xposition: 0,
            raw_layout,
            concurrent_separator: '\0',
            location,
            tags: Vec::new(),
            notes_top: Vec::new(),
            notes_bottom: Vec::new(),
            together: None,
            packed: false,
            is_static: false,
            visibility: None,
            colors: Colors::empty(),
            tips: Vec::new(),
        })));
        quark.set_data(e.clone());
        e
    }

    pub fn get(&self) -> Ref<'_, EntityData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, EntityData> {
        self.0.borrow_mut()
    }

    pub fn usymbol(&self) -> Option<Rc<dyn crate::decoration::usymbol::USymbol>> {
        self.get().symbol.as_ref().map(|s| s.0.clone())
    }

    pub fn set_usymbol(&self, symbol: Option<Rc<dyn crate::decoration::usymbol::USymbol>>) {
        self.get_mut().symbol = symbol.map(SymbolRef);
    }

    pub fn quark(&self) -> Quark<Entity> {
        self.get().quark.clone()
    }

    pub fn uid(&self) -> String {
        self.get().uid.clone()
    }

    pub fn name(&self) -> String {
        self.get().quark.name().to_string()
    }

    pub fn qualified_name(&self) -> String {
        self.get().quark.qualified_name().to_string()
    }

    pub fn location(&self) -> Option<LineLocation> {
        self.get().location
    }

    pub fn leaf_type(&self) -> Option<LeafType> {
        self.get().leaf_type
    }

    pub fn is_group(&self) -> bool {
        let d = self.get();
        assert!(d.group_type.is_none() || d.leaf_type.is_none());
        d.group_type.is_some()
    }

    pub fn is_root(&self) -> bool {
        self.get().quark.is_root()
    }

    pub fn group_type(&self) -> Option<GroupType> {
        self.get().group_type
    }

    pub fn display(&self) -> Display {
        self.get().display.clone()
    }

    pub fn set_display(&self, display: Display) {
        self.get_mut().display = display;
    }

    pub fn stereotype(&self) -> Option<Stereotype> {
        self.get().stereotype.clone()
    }

    pub fn set_stereotype(&self, s: Option<Stereotype>) {
        self.get_mut().stereotype = s;
    }

    pub fn set_stereostyle(&self, stereo: &str) {
        self.get_mut().stereostyles = crate::stereo::stereostyles(stereo);
    }

    pub fn set_generic(&self, generic: Option<String>) {
        self.get_mut().generic = generic;
    }

    pub fn generic(&self) -> Option<String> {
        self.get().generic.clone()
    }

    pub fn add_url(&self, url: Url) {
        self.get_mut().url = Some(url);
    }

    pub fn url(&self) -> Option<Url> {
        self.get().url.clone()
    }

    pub fn has_url(&self) -> bool {
        let d = self.get();
        (!d.display.is_null() && d.display.has_url()) || d.bodier.has_url() || d.url.is_some()
    }

    pub fn colors(&self) -> Colors {
        self.get().colors.clone()
    }

    pub fn set_colors(&self, colors: Colors) {
        self.get_mut().colors = colors;
    }

    pub fn set_visibility_modifier(&self, v: Option<VisibilityModifier>) {
        self.get_mut().visibility = v;
    }

    pub fn set_static(&self, s: bool) {
        self.get_mut().is_static = s;
    }

    pub fn is_static(&self) -> bool {
        self.get().is_static
    }

    pub fn add_stereotag(&self, tag: Stereotag) {
        let mut d = self.get_mut();
        if !d.tags.contains(&tag) {
            d.tags.push(tag);
        }
    }

    pub fn stereotags(&self) -> Vec<Stereotag> {
        self.get().tags.clone()
    }

    pub fn add_note(&self, note: Display, position: Position, colors: Colors) {
        let n = CucaNote::build(note, position, colors);
        match position {
            Position::Top => self.get_mut().notes_top.push(n),
            Position::Bottom => self.get_mut().notes_bottom.push(n),
            _ => {}
        }
    }

    pub fn notes(&self, position: Position) -> Vec<CucaNote> {
        match position {
            Position::Top => self.get().notes_top.clone(),
            Position::Bottom => self.get().notes_bottom.clone(),
            _ => panic!("bad note position"),
        }
    }

    pub fn put_tip(
        &self,
        member: &str,
        display: Display,
        colors: Colors,
        stereotype: Option<Stereotype>,
    ) {
        let mut d = self.get_mut();
        let tip = Tip {
            display,
            colors,
            stereotype,
        };
        if let Some(e) = d.tips.iter_mut().find(|(m, _)| m == member) {
            e.1 = tip;
        } else {
            d.tips.push((member.to_string(), tip));
        }
    }

    pub fn tips(&self) -> Vec<(String, Tip)> {
        self.get().tips.clone()
    }

    pub fn set_legend(&self, legend: DisplayPositioned) {
        self.get_mut().legend = Some(legend);
    }

    pub fn legend(&self) -> Option<DisplayPositioned> {
        self.get().legend.clone()
    }

    pub fn margins(&self) -> Margins {
        self.get().margins
    }

    /// `ensureMargins`: grows the margins to at least `m`.
    pub fn ensure_margins(&self, m: Margins) {
        let mut d = self.get_mut();
        d.margins = Margins::new(
            d.margins.top.max(m.top),
            d.margins.right.max(m.right),
            d.margins.bottom.max(m.bottom),
            d.margins.left.max(m.left),
        );
    }

    pub fn add_port_short_name(&self, name: &str) {
        self.get_mut().port_short_names.insert(name.to_string());
    }

    pub fn port_short_names(&self) -> Vec<String> {
        self.get().port_short_names.iter().cloned().collect()
    }

    pub fn raw_layout(&self) -> i32 {
        self.get().raw_layout
    }

    pub fn xposition(&self) -> i32 {
        self.get().xposition
    }

    pub fn set_xposition(&self, p: i32) {
        self.get_mut().xposition = p;
    }

    pub fn concurrent_separator(&self) -> char {
        self.get().concurrent_separator
    }

    pub fn set_concurrent_separator(&self, c: char) {
        self.get_mut().concurrent_separator = c;
    }

    pub fn set_together(&self, t: Option<Rc<Together>>) {
        self.get_mut().together = t;
    }

    pub fn together(&self) -> Option<Rc<Together>> {
        self.get().together.clone()
    }

    pub fn is_packed(&self) -> bool {
        self.get().packed
    }

    pub fn set_packed(&self) {
        self.get_mut().packed = true;
    }

    /// `muteToType(LeafType, USymbol)`: false when the change is not allowed.
    pub fn mute_to_type(&self, new_type: LeafType) -> bool {
        use LeafType::*;
        let mut d = self.get_mut();
        let current = d.leaf_type;
        if current != Some(StillUnknown) {
            if Some(new_type) == current {
                return true;
            }
            let classy = |t: LeafType| {
                matches!(
                    t,
                    Annotation | AbstractClass | Class | Enum | Interface | Record | Dataclass
                )
            };
            match current {
                Some(c) if classy(c) => {}
                _ => return false,
            }
            if !classy(new_type) && new_type != Object {
                return false;
            }
        }
        if current == Some(Class) && new_type == Object {
            d.bodier.mute_class_to_object();
        }
        d.leaf_type = Some(new_type);
        true
    }

    /// `muteToType(LeafType)`: unconditional.
    pub fn force_leaf_type(&self, new_type: LeafType) {
        let mut d = self.get_mut();
        if d.leaf_type == Some(LeafType::Class) && new_type == LeafType::Object {
            d.bodier.mute_class_to_object();
        }
        d.group_type = None;
        d.leaf_type = Some(new_type);
    }

    pub fn svek_image(&self) -> Option<Rc<dyn crate::svek::image::IEntityImage>> {
        self.get().svek_image.as_ref().map(|i| i.0.clone())
    }

    pub fn set_svek_image(&self, image: Option<Rc<dyn crate::svek::image::IEntityImage>>) {
        self.get_mut().svek_image = image.map(SvekImageRef);
    }

    /// `overrideImage`: the group becomes a leaf drawn as `image`.
    pub fn override_image(
        &self,
        image: Rc<dyn crate::svek::image::IEntityImage>,
        leaf_type: LeafType,
    ) {
        let mut d = self.get_mut();
        d.svek_image = Some(SvekImageRef(image));
        d.url = None;
        d.group_type = None;
        d.leaf_type = Some(leaf_type);
    }

    /// Whether a group above this entity was replaced by an image.
    pub fn is_inside_overridden(&self) -> bool {
        let mut current = self.parent_container();
        while let Some(p) = current {
            if p.get().svek_image.is_some() {
                return true;
            }
            current = p.parent_container();
        }
        false
    }

    pub fn mute_to_group_type(&self, new_type: GroupType) {
        let mut d = self.get_mut();
        d.group_type = Some(new_type);
        d.leaf_type = None;
    }

    pub fn entity_position(&self) -> EntityPosition {
        let d = self.get();
        match d.leaf_type {
            Some(LeafType::Portin) => return EntityPosition::Portin,
            Some(LeafType::Portout) => return EntityPosition::Portout,
            Some(LeafType::State) => {}
            _ => return EntityPosition::Normal,
        }
        if d.quark.is_root() {
            return EntityPosition::Normal;
        }
        match &d.stereotype {
            None => EntityPosition::Normal,
            Some(s) => EntityPosition::from_stereotype(
                &s.label(Guillemet::DoubleComparator).unwrap_or_default(),
            ),
        }
    }

    pub fn parent_container(&self) -> Option<Entity> {
        let d = self.get();
        d.quark.parent().and_then(|p| p.data())
    }

    pub fn leafs(&self) -> Vec<Entity> {
        self.get()
            .quark
            .children()
            .iter()
            .filter_map(Quark::data)
            .filter(|e| !e.is_group())
            .collect()
    }

    pub fn groups(&self) -> Vec<Entity> {
        self.get()
            .quark
            .children()
            .iter()
            .filter_map(Quark::data)
            .filter(Entity::is_group)
            .collect()
    }

    pub fn count_children(&self) -> usize {
        self.get().quark.count_children()
    }

    pub fn add_field_or_method(&self, s: &str) -> bool {
        self.get_mut().bodier.add_field_or_method(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plasma::Plasma;

    fn leaf(p: &Plasma<Entity>, name: &str, n: i32) -> Entity {
        Entity::new(
            None,
            p.root().child(name),
            n,
            Bodier::create_leaf(LeafType::Class, Rc::default()),
            EntityKind::Leaf(LeafType::Class),
            0,
        )
    }

    #[test]
    fn types_and_tree() {
        let p: Plasma<Entity> = Plasma::new();
        let root = Entity::new(
            None,
            p.root(),
            0,
            Bodier::create_group(),
            EntityKind::Group(GroupType::Root),
            0,
        );
        let a = leaf(&p, "A", 1);
        assert_eq!(a.uid(), "ent0001");
        assert_eq!(a.parent_container(), Some(root.clone()));
        assert_eq!(root.leafs(), vec![a.clone()]);
        assert!(!a.is_group());
        assert!(a.mute_to_type(LeafType::Interface));
        assert!(!a.mute_to_type(LeafType::Usecase));
        a.ensure_margins(Margins::same(4.0));
        a.ensure_margins(Margins::new(0.0, 8.0, 0.0, 0.0));
        assert_eq!(a.margins(), Margins::new(4.0, 8.0, 4.0, 4.0));
    }
}
