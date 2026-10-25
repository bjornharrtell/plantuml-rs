//! A relation between two entities (`Link`, `LinkArg`).

use super::{CucaNote, Entity, LeafType, LinkArrow, NoteLinkStrategy};
use crate::decoration::link::{LinkDecor, LinkType};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::skin::visibility::VisibilityModifier;
use crate::stereo::Stereotype;
use crate::text::LineLocation;
use crate::url::Url;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

/// Label, quantifiers, roles and length of a link (`LinkArg`).
#[derive(Debug, Clone)]
pub struct LinkArg {
    pub label: Display,
    pub quantifier1: Option<String>,
    pub quantifier2: Option<String>,
    pub role1: Option<String>,
    pub role2: Option<String>,
    pub labeldistance: Option<String>,
    pub labelangle: Option<String>,
    pub kal1: Option<String>,
    pub kal2: Option<String>,
    pub visibility_modifier: Option<VisibilityModifier>,
    pub length: i32,
}

impl LinkArg {
    pub fn build(label: Display, length: i32) -> LinkArg {
        Self::build_with(label, length, true)
    }

    pub fn no_display(length: i32) -> LinkArg {
        Self::build_with(Display::null(), length, true)
    }

    pub fn build_with(label: Display, length: i32, manage_visibility_modifier: bool) -> LinkArg {
        let mut visibility_modifier = None;
        let new_label = if label.is_null() {
            Display::null()
        } else {
            if manage_visibility_modifier {
                let first = label.text_at(0);
                if VisibilityModifier::is_visibility_character(&first) {
                    visibility_modifier = VisibilityModifier::from_text(&first, false);
                }
            }
            label.manage_guillemet(manage_visibility_modifier)
        };
        LinkArg {
            label: new_label,
            quantifier1: None,
            quantifier2: None,
            role1: None,
            role2: None,
            labeldistance: None,
            labelangle: None,
            kal1: None,
            kal2: None,
            visibility_modifier,
            length,
        }
    }

    pub fn with_quantifier(&self, q1: Option<String>, q2: Option<String>) -> LinkArg {
        LinkArg {
            quantifier1: q1,
            quantifier2: q2,
            ..self.clone()
        }
    }

    pub fn with_role(&self, r1: Option<String>, r2: Option<String>) -> LinkArg {
        LinkArg {
            role1: r1,
            role2: r2,
            ..self.clone()
        }
    }

    pub fn with_kal(&self, k1: Option<String>, k2: Option<String>) -> LinkArg {
        LinkArg {
            kal1: k1,
            kal2: k2,
            ..self.clone()
        }
    }

    pub fn with_distance_angle(&self, d: Option<String>, a: Option<String>) -> LinkArg {
        LinkArg {
            labeldistance: d,
            labelangle: a,
            ..self.clone()
        }
    }

    pub fn inv(&self) -> LinkArg {
        LinkArg {
            quantifier1: self.quantifier2.clone(),
            quantifier2: self.quantifier1.clone(),
            kal1: self.kal2.clone(),
            kal2: self.kal1.clone(),
            role1: self.role2.clone(),
            role2: self.role1.clone(),
            ..self.clone()
        }
    }
}

/// Two links that must be ordered (`LinkConstraint`); the drawing part is not ported yet.
#[derive(Debug, Clone)]
pub struct LinkConstraint {
    pub display: Display,
}

#[derive(Debug)]
pub struct LinkData {
    pub cl1: Entity,
    pub cl2: Entity,
    pub port1: Option<String>,
    pub port2: Option<String>,
    pub link_arg: LinkArg,
    pub uid: String,
    pub note: Option<CucaNote>,
    pub invis: bool,
    pub weight: f64,
    pub constraint: bool,
    pub inverted: bool,
    pub link_arrow: LinkArrow,
    pub opale: bool,
    pub horizontal_solitary: bool,
    pub sametail: Option<String>,
    pub stereotype: Option<Stereotype>,
    pub location: Option<LineLocation>,
    pub url: Option<Url>,
    pub link_constraint: Option<LinkConstraint>,
    pub code_line: Option<LineLocation>,
    pub type_: LinkType,
    pub hidden: bool,
    pub single: bool,
    pub use_node_style: bool,
    pub colors: Colors,
    pub supplementary: Vec<Colors>,
}

/// Shared handle on a link; equality is identity.
#[derive(Clone)]
pub struct Link(Rc<RefCell<LinkData>>);

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.0.borrow();
        write!(
            f,
            "Link {{{}}} {:?}-->{:?}",
            d.link_arg.length, d.cl1, d.cl2
        )
    }
}

impl Link {
    /// `uid` is the already generated unique id (`lnk3`).
    pub fn new(
        location: Option<LineLocation>,
        uid: String,
        cl1: Entity,
        cl2: Entity,
        type_: LinkType,
        link_arg: LinkArg,
    ) -> Link {
        assert!(link_arg.length >= 1);
        Link(Rc::new(RefCell::new(LinkData {
            cl1,
            cl2,
            port1: None,
            port2: None,
            link_arg,
            uid,
            note: None,
            invis: false,
            weight: 1.0,
            constraint: true,
            inverted: false,
            link_arrow: LinkArrow::NoneOrSeveral,
            opale: false,
            horizontal_solitary: false,
            sametail: None,
            stereotype: None,
            location,
            url: None,
            link_constraint: None,
            code_line: None,
            type_,
            hidden: false,
            single: false,
            use_node_style: false,
            colors: Colors::empty(),
            supplementary: Vec::new(),
        })))
    }

    pub fn get(&self) -> Ref<'_, LinkData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, LinkData> {
        self.0.borrow_mut()
    }

    /// The same link seen from the other end (`getInv`); `uid` for the new link.
    pub fn inv(&self, uid: String) -> Link {
        let d = self.get();
        let result = Link::new(
            d.location,
            uid,
            d.cl2.clone(),
            d.cl1.clone(),
            d.type_.inversed(),
            d.link_arg.inv(),
        );
        {
            let mut r = result.get_mut();
            r.inverted = !d.inverted;
            r.port1 = d.port2.clone();
            r.port2 = d.port1.clone();
            r.url = d.url.clone();
            r.link_constraint = d.link_constraint.clone();
            r.stereotype = d.stereotype.clone();
            r.link_arrow = d.link_arrow;
        }
        result
    }

    pub fn entity1(&self) -> Entity {
        self.get().cl1.clone()
    }

    pub fn entity2(&self) -> Entity {
        self.get().cl2.clone()
    }

    pub fn uid(&self) -> String {
        self.get().uid.clone()
    }

    pub fn location(&self) -> Option<LineLocation> {
        self.get().location
    }

    pub fn id_comment_for_svg(&self) -> String {
        let d = self.get();
        let t = self.type_();
        let (n1, n2) = (d.cl1.name(), d.cl2.name());
        if t.looks_like_reverted_for_svg() {
            format!("{n1}-backto-{n2}")
        } else if t.looks_like_no_decor_at_all_svg() {
            format!("{n1}-{n2}")
        } else {
            format!("{n1}-to-{n2}")
        }
    }

    pub fn comment_for_svg(&self) -> String {
        let d = self.get();
        let (n1, n2) = (d.cl1.name(), d.cl2.name());
        if d.type_.looks_like_reverted_for_svg() {
            format!("reverse link {n1} to {n2}")
        } else {
            format!("link {n1} to {n2}")
        }
    }

    /// Type with the note/sametail adjustments (`getType`).
    pub fn type_(&self) -> LinkType {
        let d = self.get();
        if d.opale || d.sametail.is_some() {
            return LinkType::new(LinkDecor::None, LinkDecor::None);
        }
        d.type_
    }

    pub fn set_type(&self, t: LinkType) {
        self.get_mut().type_ = t;
    }

    fn is_really_group(ent: &Entity) -> bool {
        ent.is_group() && ent.groups().len() + ent.leafs().len() > 0
    }

    /// `getTypePatchCluster`: no decoration where a cluster is attached.
    pub fn type_patch_cluster(&self) -> LinkType {
        let mut result = self.type_();
        if Self::is_really_group(&self.entity1()) {
            result = result.without_decors2();
        }
        if Self::is_really_group(&self.entity2()) {
            result = result.without_decors1();
        }
        result
    }

    pub fn label(&self) -> Display {
        self.get().link_arg.label.clone()
    }

    pub fn length(&self) -> i32 {
        self.get().link_arg.length
    }

    pub fn set_length(&self, length: i32) {
        self.get_mut().link_arg.length = length;
    }

    pub fn quantifier1(&self) -> Option<String> {
        self.get().link_arg.quantifier1.clone()
    }

    pub fn quantifier2(&self) -> Option<String> {
        self.get().link_arg.quantifier2.clone()
    }

    pub fn role1(&self) -> Option<String> {
        self.get().link_arg.role1.clone()
    }

    pub fn role2(&self) -> Option<String> {
        self.get().link_arg.role2.clone()
    }

    pub fn is_invis(&self) -> bool {
        let d = self.get();
        d.type_.is_invisible() || d.invis
    }

    pub fn set_invis(&self, invis: bool) {
        self.get_mut().invis = invis;
    }

    pub fn is_between(&self, a: &Entity, b: &Entity) -> bool {
        let d = self.get();
        (*a == d.cl1 && *b == d.cl2) || (*a == d.cl2 && *b == d.cl1)
    }

    pub fn contains(&self, e: &Entity) -> bool {
        let d = self.get();
        d.cl1 == *e || d.cl2 == *e
    }

    pub fn contains_type(&self, t: LeafType) -> bool {
        let d = self.get();
        d.cl1.leaf_type() == Some(t) || d.cl2.leaf_type() == Some(t)
    }

    pub fn other(&self, e: &Entity) -> Entity {
        let d = self.get();
        if d.cl1 == *e {
            d.cl2.clone()
        } else if d.cl2 == *e {
            d.cl1.clone()
        } else {
            panic!("entity not in link")
        }
    }

    pub fn is_auto_link_of_a_group(&self) -> bool {
        let d = self.get();
        d.cl1.is_group() && d.cl2.is_group() && d.cl1 == d.cl2
    }

    pub fn is_autolink(&self) -> bool {
        let d = self.get();
        d.cl1 == d.cl2
    }

    pub fn is_constraint(&self) -> bool {
        self.get().constraint
    }

    pub fn set_constraint(&self, c: bool) {
        self.get_mut().constraint = c;
    }

    pub fn go_norank(&self) {
        self.set_constraint(false);
    }

    pub fn set_opale(&self, o: bool) {
        self.get_mut().opale = o;
    }

    pub fn is_opale(&self) -> bool {
        self.get().opale
    }

    pub fn link_arrow(&self) -> LinkArrow {
        let d = self.get();
        if d.inverted {
            d.link_arrow.reverse()
        } else {
            d.link_arrow
        }
    }

    pub fn set_link_arrow(&self, a: LinkArrow) {
        self.get_mut().link_arrow = a;
    }

    pub fn is_inverted(&self) -> bool {
        self.get().inverted
    }

    pub fn has_entry_point(&self) -> bool {
        let d = self.get();
        (!d.cl1.is_group() && d.cl1.entity_position() != super::EntityPosition::Normal)
            || (!d.cl2.is_group() && d.cl2.entity_position() != super::EntityPosition::Normal)
    }

    pub fn has_two_entry_points_same_container(&self) -> bool {
        let d = self.get();
        !d.cl1.is_group()
            && !d.cl2.is_group()
            && d.cl1.entity_position() != super::EntityPosition::Normal
            && d.cl2.entity_position() != super::EntityPosition::Normal
            && d.cl1.parent_container() == d.cl2.parent_container()
    }

    pub fn url(&self) -> Option<Url> {
        self.get().url.clone()
    }

    pub fn set_url(&self, u: Option<Url>) {
        self.get_mut().url = u;
    }

    pub fn has_url(&self) -> bool {
        let d = self.get();
        (!d.link_arg.label.is_null() && d.link_arg.label.has_url()) || d.url.is_some()
    }

    /// Hidden by `hide` or because an end is hidden (the entity checks are done by the diagram).
    pub fn is_hidden_flag(&self) -> bool {
        self.get().hidden
    }

    pub fn go_hidden(&self) {
        self.get_mut().hidden = true;
    }

    pub fn same_connections(&self, other: &Link) -> bool {
        let (a, b) = (self.get(), other.get());
        (a.cl1 == b.cl1 && a.cl2 == b.cl2) || (a.cl1 == b.cl2 && a.cl2 == b.cl1)
    }

    pub fn does_touch(&self, other: &Link) -> bool {
        let (a, b) = (self.get(), other.get());
        a.cl1 == b.cl1 || a.cl1 == b.cl2 || a.cl2 == b.cl1 || a.cl2 == b.cl2
    }

    pub fn sametail(&self) -> Option<String> {
        self.get().sametail.clone()
    }

    pub fn set_sametail(&self, s: Option<String>) {
        self.get_mut().sametail = s;
    }

    pub fn set_port_members(&self, port1: Option<String>, port2: Option<String>) {
        let mut d = self.get_mut();
        if let Some(p) = &port1 {
            d.cl1.add_port_short_name(p);
        }
        if let Some(p) = &port2 {
            d.cl2.add_port_short_name(p);
        }
        d.port1 = port1;
        d.port2 = port2;
    }

    pub fn note(&self) -> Option<CucaNote> {
        self.get().note.clone()
    }

    pub fn add_note(&self, note: CucaNote) {
        self.get_mut().note = Some(note);
    }

    pub fn add_note_from(&self, other: &Link, strategy: NoteLinkStrategy) {
        if let Some(n) = other.note() {
            self.get_mut().note = Some(n.with_strategy(strategy));
        }
    }

    pub fn stereotype(&self) -> Option<Stereotype> {
        self.get().stereotype.clone()
    }

    pub fn set_stereotype(&self, s: Option<Stereotype>) {
        self.get_mut().stereotype = s;
    }

    pub fn visibility_modifier(&self) -> Option<VisibilityModifier> {
        self.get().link_arg.visibility_modifier
    }

    pub fn link_arg(&self) -> LinkArg {
        self.get().link_arg.clone()
    }

    pub fn set_link_constraint(&self, c: Option<LinkConstraint>) {
        self.get_mut().link_constraint = c;
    }

    pub fn has_kal1(&self) -> bool {
        self.get()
            .link_arg
            .kal1
            .as_ref()
            .is_some_and(|k| !k.is_empty())
    }

    pub fn has_kal2(&self) -> bool {
        self.get()
            .link_arg
            .kal2
            .as_ref()
            .is_some_and(|k| !k.is_empty())
    }

    pub fn set_code_line(&self, l: Option<LineLocation>) {
        self.get_mut().code_line = l;
    }

    pub fn code_line(&self) -> Option<String> {
        self.get().code_line.map(|l| l.position.to_string())
    }

    // WithLinkType

    pub fn specific_color(&self) -> Option<HColor> {
        self.get().colors.color(ColorType::Line).cloned()
    }

    pub fn set_specific_color(&self, color: HColor, i: usize) {
        let mut d = self.get_mut();
        let c = d.colors.add(ColorType::Line, Some(color));
        if i == 0 {
            d.colors = c;
        } else {
            d.supplementary.push(c);
        }
    }

    pub fn supplementary_colors(&self) -> Vec<Colors> {
        self.get().supplementary.clone()
    }

    pub fn set_colors(&self, c: Colors) {
        self.get_mut().colors = c;
    }

    pub fn colors(&self) -> Colors {
        self.get().colors.clone()
    }

    pub fn go_dashed(&self) {
        let t = self.get().type_.go_dashed();
        self.get_mut().type_ = t;
    }

    pub fn go_dotted(&self) {
        let t = self.get().type_.go_dotted();
        self.get_mut().type_ = t;
    }

    pub fn go_bold(&self) {
        let t = self.get().type_.go_bold();
        self.get_mut().type_ = t;
    }

    pub fn go_thickness(&self, thickness: f64) {
        let t = self.get().type_.go_thickness(thickness);
        self.get_mut().type_ = t;
    }

    pub fn go_single(&self) {
        self.get_mut().single = true;
    }

    pub fn is_single(&self) -> bool {
        self.get().single
    }

    pub fn go_node_style(&self) {
        self.get_mut().use_node_style = true;
    }

    /// `applyStyle`: `;`-separated parts of `,`-separated keywords.
    pub fn apply_style(&self, arrow_style: Option<&str>) {
        let Some(style) = arrow_style else { return };
        for (i, part) in style.split(';').filter(|s| !s.is_empty()).enumerate() {
            for s in part.split(',').filter(|s| !s.is_empty()) {
                match s.to_lowercase().as_str() {
                    "dashed" => self.go_dashed(),
                    "bold" => self.go_bold(),
                    "dotted" => self.go_dotted(),
                    "hidden" => self.go_hidden(),
                    "single" => self.go_single(),
                    "plain" => {}
                    "node" => self.go_node_style(),
                    "norank" => self.go_norank(),
                    _ => {
                        if let Some(t) = s.strip_prefix("thickness=") {
                            if let Ok(v) = t.parse::<f64>() {
                                self.go_thickness(v);
                            }
                        } else {
                            self.set_specific_color(HColor::parse_or_white(s), i);
                        }
                    }
                }
            }
        }
    }
}
