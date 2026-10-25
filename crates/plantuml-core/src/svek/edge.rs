//! A link given to Graphviz and drawn from its answer (`SvekEdge`).

use super::bibliotekon::Bibliotekon;
use super::cluster::{Cluster, CENTER_ID};
use super::extremity::{extremity_factory_complete, middle_factory, Extremity};
use super::node::SvekNode;

use super::types::{hex_rgb, min_xy, EntityPort, SvgResult, D_EQUALS, POINTS_EQUALS};
use crate::abel::{Entity, Link, LinkArrow, NoteLinkStrategy};
use crate::decoration::link::LinkMiddleDecor;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::dotpath::{distance, DotPath, Point, RectangleArea};
use crate::klimt::drawing::ugraphic::{UGraphic, UStroke, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::{with_margin, EmptyBlock, HorizontalBlock, TextBlock, VerticalBlock};
use crate::skin::SkinParam;
use crate::style::{queries, Margins, PName};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

const CONSTRAINT_SPOT: Dimension = Dimension {
    width: 10.0,
    height: 10.0,
};

/// A block with a position (`Positionable`).
#[derive(Clone, Copy)]
struct Placed {
    pos: Point,
    dim: Dimension,
}

pub struct EdgeData {
    link: Link,
    skin: SkinParam,
    bib: Bibliotekon,
    sb: Rc<dyn StringBounder>,
    ltail: Option<Cluster>,
    lhead: Option<Cluster>,
    start_uid: EntityPort,
    end_uid: EntityPort,
    start_tail_text: Option<Rc<dyn TextBlock>>,
    end_head_text: Option<Rc<dyn TextBlock>>,
    start_tail_role: Option<Rc<dyn TextBlock>>,
    end_head_role: Option<Rc<dyn TextBlock>>,
    label_text: Option<Rc<dyn TextBlock>>,
    divide_label_width_by_two: bool,
    line_color: i32,
    note_label_color: i32,
    start_tail_color: i32,
    end_head_color: i32,
    dot_path: Option<DotPath>,
    start_tail_label_xy: Option<Placed>,
    end_head_label_xy: Option<Placed>,
    label_xy: Option<Placed>,
    extremity1: Option<Box<dyn Extremity>>,
    extremity2: Option<Box<dyn Extremity>>,
    dx: f64,
    dy: f64,
    opale: bool,
    background: HColor,
    default_thickness: Option<UStroke>,
    arrow_lollipop_color: HColor,
    label_shield: f64,
    use_rank_same: bool,
    pragma_horizontal_allowed: bool,
}

#[derive(Clone)]
pub struct SvekEdge(Rc<RefCell<EdgeData>>);

impl PartialEq for SvekEdge {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

fn cluster_of(bib: &Bibliotekon, ent: &Entity) -> Cluster {
    bib.cluster_of(ent).expect("cluster of group")
}

impl SvekEdge {
    /// Room for the roles around the ends of a link (`reserveRoomForRoles`).
    pub fn reserve_room_for_roles(link: &Link, skin: &SkinParam, sb: &dyn StringBounder) {
        let card_q = queries::ARROW_CARDINALITY
            .add(skin_style_name(skin))
            .with_stereotype_obj(link.stereotype().as_ref());
        let style = skin
            .style_builder()
            .merged_style(&card_q)
            .expect("cardinality style");
        let font = style.font_configuration(None);
        for (role, ent) in [
            (link.role1(), link.entity1()),
            (link.role2(), link.entity2()),
        ] {
            if let Some(r) = role {
                let width = role_block(&r, &font, skin)
                    .calculate_dimension(sb)
                    .width
                    .ceil()
                    + 2.0;
                ent.ensure_margins(Margins::new(0.0, width, 0.0, width));
            }
        }
    }

    pub fn new(
        link: Link,
        skin: &SkinParam,
        sb: Rc<dyn StringBounder>,
        bib: &Bibliotekon,
    ) -> SvekEdge {
        let (e1, e2) = (link.entity1(), link.entity2());
        let start_uid = bib.entity_port(&e1, link.get().port1.as_deref());
        let end_uid = bib.entity_port(&e2, link.get().port2.as_deref());
        let ltail = start_uid
            .starts_with(CENTER_ID)
            .then(|| cluster_of(bib, &e1));
        let lhead = end_uid.starts_with(CENTER_ID).then(|| cluster_of(bib, &e2));
        let colors = link.colors();
        let diagram_style = skin.style_builder();
        let arrow_q = queries::ARROW.add(skin_style_name(skin));
        let arrow_q = arrow_q.with_stereotype_obj(link.stereotype().as_ref());
        let arrow_style = diagram_style.merged_style(&arrow_q).expect("arrow style");
        let mut font = arrow_style.font_configuration(None);
        if let Some(c) = colors.color(ColorType::Text) {
            font = font.change_color(c.clone());
        }
        let card_q = queries::ARROW_CARDINALITY.add(skin_style_name(skin));
        let card_q = card_q.with_stereotype_obj(link.stereotype().as_ref());
        let card_style = diagram_style
            .merged_style(&card_q)
            .expect("cardinality style");
        let cardinality_font = card_style.font_configuration(None);
        let background = skin.background_color();
        let default_thickness = Some(arrow_style.stroke());
        let line_color = bib.next_color();
        let note_label_color = bib.next_color();
        let start_tail_color = bib.next_color();
        let end_head_color = bib.next_color();
        let skin_simple = skin.skin_simple();
        let mut divide = false;
        let label = link.label();
        let label_only: Option<Rc<dyn TextBlock>> = if label.is_null() {
            None
        } else {
            let alignment = skin.default_text_alignment(HorizontalAlignment::Center);
            let wrap = {
                let w = arrow_style.wrap_width();
                if w.max_width() > 0.0 {
                    w
                } else {
                    skin.max_message_size()
                }
            };
            let block = label.create_block(
                &font,
                alignment,
                &skin_simple,
                &wrap,
                CreoleMode::SimpleLine,
                None,
                None,
                0.0,
                0.0,
            );
            let block = add_visibility_modifier(block, &link, skin, &start_uid, &end_uid);
            Some(block)
        };
        let label_text: Option<Rc<dyn TextBlock>> = match link.note() {
            None => label_only,
            Some(note) => {
                let note_only = super::note_link::note_link_block(&note, skin);
                if matches!(
                    note.strategy,
                    NoteLinkStrategy::HalfNotPrinted | NoteLinkStrategy::HalfPrintedFull
                ) {
                    divide = true;
                }
                let lo = label_only.unwrap_or_else(|| {
                    Rc::new(EmptyBlock {
                        width: 0.0,
                        height: 0.0,
                    })
                });
                use crate::klimt::geom::Position;
                Some(match note.position {
                    Position::Left => Rc::new(HorizontalBlock {
                        blocks: vec![note_only, lo],
                        alignment: VerticalAlignment::Center,
                    }) as Rc<dyn TextBlock>,
                    Position::Right => Rc::new(HorizontalBlock {
                        blocks: vec![lo, note_only],
                        alignment: VerticalAlignment::Center,
                    }),
                    Position::Top => Rc::new(VerticalBlock {
                        blocks: vec![note_only, lo],
                        alignment: HorizontalAlignment::Center,
                    }),
                    Position::Bottom => Rc::new(VerticalBlock {
                        blocks: vec![lo, note_only],
                        alignment: HorizontalAlignment::Center,
                    }),
                })
            }
        };
        let quant = |q: Option<String>| {
            q.map(|q| {
                Display::with_newlines(Some(&q)).create_block(
                    &cardinality_font,
                    HorizontalAlignment::Center,
                    &skin_simple,
                    &crate::klimt::creole::LineBreakStrategy::NONE,
                    CreoleMode::Full,
                    None,
                    None,
                    0.0,
                    0.0,
                )
            })
        };
        let role = |r: Option<String>| r.map(|r| role_block(&r, &cardinality_font, skin));
        let label_shield = if link.type_().middle_decor() == LinkMiddleDecor::None {
            0.0
        } else {
            7.0
        };
        let data = EdgeData {
            skin: skin.clone(),
            bib: bib.clone(),
            sb,
            ltail,
            lhead,
            start_uid,
            end_uid,
            start_tail_text: quant(link.quantifier1()),
            end_head_text: quant(link.quantifier2()),
            start_tail_role: role(link.role1()),
            end_head_role: role(link.role2()),
            label_text,
            divide_label_width_by_two: divide,
            line_color,
            note_label_color,
            start_tail_color,
            end_head_color,
            dot_path: None,
            start_tail_label_xy: None,
            end_head_label_xy: None,
            label_xy: None,
            extremity1: None,
            extremity2: None,
            dx: 0.0,
            dy: 0.0,
            opale: false,
            background: background.clone(),
            default_thickness,
            arrow_lollipop_color: background,
            label_shield,
            use_rank_same: skin.use_rank_same(),
            pragma_horizontal_allowed: false,
            link,
        };
        SvekEdge(Rc::new(RefCell::new(data)))
    }

    pub fn get(&self) -> Ref<'_, EdgeData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, EdgeData> {
        self.0.borrow_mut()
    }

    pub fn link(&self) -> Link {
        self.get().link.clone()
    }

    pub fn length(&self) -> i32 {
        self.get().link.length()
    }

    pub fn is_inverted(&self) -> bool {
        self.get().link.is_inverted()
    }

    pub fn is_link(&self, l: &Link) -> bool {
        self.get().link == *l
    }

    pub fn is_link_from_or_to(&self, e: &Entity) -> bool {
        let l = self.get();
        l.link.entity1() == *e || l.link.entity2() == *e
    }

    pub fn same_connections(&self, other: &SvekEdge) -> bool {
        self.get().link.same_connections(&other.get().link)
    }

    pub fn has_entry_point(&self) -> bool {
        self.get().link.has_entry_point()
    }

    pub fn has_note_label_text(&self) -> bool {
        self.get().label_text.is_some()
    }

    pub fn start_uid_prefix(&self) -> String {
        self.get().start_uid.prefix()
    }

    pub fn end_uid_prefix(&self) -> String {
        self.get().end_uid.prefix()
    }

    pub fn is_hidden(&self) -> bool {
        let d = self.get();
        d.link.is_hidden_flag()
    }

    pub fn rank_same(&self) -> Option<String> {
        let d = self.get();
        if !d.pragma_horizontal_allowed && d.link.length() == 1 {
            return Some(format!(
                "{{rank=same; {}; {}}}",
                d.start_uid.prefix(),
                d.end_uid.prefix()
            ));
        }
        None
    }

    pub fn set_opale(&self, o: bool) {
        let mut d = self.get_mut();
        d.link.set_opale(o);
        d.opale = o;
    }

    pub fn move_delta(&self, dx: f64, dy: f64) {
        let mut d = self.get_mut();
        d.dx += dx;
        d.dy += dy;
    }

    pub fn dot_path(&self) -> Option<DotPath> {
        let d = self.get();
        d.dot_path.as_ref().map(|p| {
            let mut c = p.clone();
            c.move_delta(d.dx, d.dy);
            c
        })
    }

    pub fn start_contact_point(&self) -> Option<Point> {
        let d = self.get();
        d.dot_path
            .as_ref()
            .map(|p| (p.start_point().0 + d.dx, p.start_point().1 + d.dy))
    }

    pub fn end_contact_point(&self) -> Option<Point> {
        let d = self.get();
        d.dot_path
            .as_ref()
            .map(|p| (p.end_point().0 + d.dx, p.end_point().1 + d.dy))
    }

    fn decor_dzeta(&self) -> f64 {
        let t = self.get().link.type_();
        f64::from(t.decor1().margin() + t.decor2().margin())
    }

    pub fn horizontal_dzeta(&self, sb: &dyn StringBounder) -> f64 {
        let d = self.get();
        if d.start_uid.equals_id(&d.end_uid) {
            drop(d);
            return self.decor_dzeta();
        }
        if d.link.length() != 1 {
            return 0.0;
        }
        let mut sum = 0.0;
        for b in [&d.label_text, &d.start_tail_text, &d.end_head_text]
            .into_iter()
            .flatten()
        {
            sum += b.calculate_dimension(sb).width;
        }
        drop(d);
        sum + self.decor_dzeta()
    }

    pub fn vertical_dzeta(&self, sb: &dyn StringBounder) -> f64 {
        let d = self.get();
        if d.start_uid.equals_id(&d.end_uid) {
            drop(d);
            return self.decor_dzeta();
        }
        if d.link.length() == 1 {
            return 0.0;
        }
        let mut sum = 0.0;
        for b in [&d.label_text, &d.start_tail_text, &d.end_head_text]
            .into_iter()
            .flatten()
        {
            sum += b.calculate_dimension(sb).height;
        }
        drop(d);
        sum + self.decor_dzeta()
    }

    /// The edge as written in the DOT file (`appendLine`).
    pub fn append_line(&self, sb: &mut String, ortho: bool) {
        let d = self.get();
        sb.push_str(&d.start_uid.full_string());
        sb.push_str("->");
        sb.push_str(&d.end_uid.full_string());
        sb.push('[');
        let link_type = d.link.type_patch_cluster();
        let mut decoration = link_type.specific_decoration_svek().to_string();
        if !decoration.is_empty() && !decoration.ends_with(',') {
            decoration.push(',');
        }
        sb.push_str(&decoration);
        let length = d.link.length();
        if d.use_rank_same {
            if d.pragma_horizontal_allowed || d.link.is_invis() || length != 1 {
                sb.push_str(&format!("minlen={},", length - 1));
            }
        } else {
            sb.push_str(&format!("minlen={},", length - 1));
        }
        sb.push_str(&format!("color=\"{}\"", hex_rgb(d.line_color)));
        let has_label = d.label_text.is_some();
        let constraint_label = d.link.get().link_constraint.is_some();
        if has_label || constraint_label {
            sb.push(',');
            sb.push_str(if ortho { "xlabel=<" } else { "label=<" });
            let dim = match &d.label_text {
                Some(t) => t.calculate_dimension(&*d.sb),
                None => CONSTRAINT_SPOT,
            };
            let dim = dim.delta(2.0 * d.label_shield, 2.0 * d.label_shield);
            let dim = if d.divide_label_width_by_two {
                Dimension::new(dim.width / 2.0, dim.height)
            } else {
                dim
            };
            append_table(sb, dim, d.note_label_color);
            sb.push('>');
        }
        if let Some(t) = d.start_tail_text.as_ref().or(d.start_tail_role.as_ref()) {
            sb.push_str(",taillabel=<");
            append_table(sb, t.calculate_dimension(&*d.sb), d.start_tail_color);
            sb.push('>');
        }
        if let Some(t) = d.end_head_text.as_ref().or(d.end_head_role.as_ref()) {
            sb.push_str(",headlabel=<");
            append_table(sb, t.calculate_dimension(&*d.sb), d.end_head_color);
            sb.push('>');
        }
        if d.link.is_invis() {
            sb.push_str(",style=invis");
        }
        if !d.link.is_constraint() || d.link.has_two_entry_points_same_container() {
            sb.push_str(",constraint=false");
        }
        if let Some(s) = d.link.sametail() {
            sb.push_str(&format!(",sametail={s}"));
        }
        sb.push_str("];\n");
    }

    fn node1(&self) -> Option<SvekNode> {
        let d = self.get();
        d.bib.node(&d.link.entity1())
    }

    fn node2(&self) -> Option<SvekNode> {
        let d = self.get();
        d.bib.node(&d.link.entity2())
    }

    /// Reads the route of the edge and the label positions from the layout (`solveLine`).
    pub fn solve_line(&self, full: &SvgResult<'_>) {
        if self.get().link.is_invis() {
            return;
        }
        let line_color = self.get().line_color;
        let Some(idx) = full.index_from_color(line_color) else {
            return;
        };
        let Some(idx) = full.index_of(D_EQUALS, idx) else {
            return;
        };
        let Some(end) = full.index_of("\"", idx + 3) else {
            return;
        };
        let path = full.substring_range(idx + 3, end);
        if !path.is_path_consistent() {
            return;
        }
        let Some(mut dot_path) = path.to_dot_path() else {
            return;
        };
        let (start, endp) = (dot_path.start_point(), dot_path.end_point());
        let (n1, n2) = (self.node1(), self.node2());
        if let (Some(n1), Some(n2)) = (&n1, &n2) {
            let p1 = n1.rectangle_area().center();
            let p2 = n2.rectangle_area().center();
            let normal = distance(start, p1) + distance(endp, p2);
            let inversed = distance(start, p2) + distance(endp, p1);
            if inversed < normal {
                dot_path = dot_path.reverse();
            }
        }
        let (lhead, ltail) = {
            let d = self.get();
            (
                d.lhead.as_ref().and_then(Cluster::rectangle_area),
                d.ltail.as_ref().and_then(Cluster::rectangle_area),
            )
        };
        dot_path = dot_path.simulate_compound(lhead, ltail);
        let (bg, link_type) = {
            let d = self.get();
            (d.background.clone(), d.link.type_())
        };
        let ext = |dot_path: &mut DotPath,
                   center: Point,
                   decor: crate::decoration::link::LinkDecor,
                   angle: f64,
                   node: &Option<SvekNode>,
                   is_start: bool|
         -> Option<Box<dyn Extremity>> {
            let factory = extremity_factory_complete(decor, &bg)?;
            let side = node
                .as_ref()
                .and_then(|n| n.rectangle_area().closest_side(center));
            let extremity = factory.create(center, angle, side);
            let len = extremity.decoration_length();
            let (dx, dy) = (
                len * (angle - std::f64::consts::PI).cos(),
                len * (angle - std::f64::consts::PI).sin(),
            );
            if is_start {
                dot_path.move_start_point(dx, dy);
            } else {
                dot_path.move_end_point(dx, dy);
            }
            Some(extremity)
        };
        let (sp, sa) = (dot_path.start_point(), dot_path.start_angle());
        let e1 = ext(
            &mut dot_path,
            sp,
            link_type.decor2(),
            sa + std::f64::consts::PI,
            &n1,
            true,
        );
        let (ep, ea) = (dot_path.end_point(), dot_path.end_angle());
        let e2 = ext(&mut dot_path, ep, link_type.decor1(), ea, &n2, false);
        let xy = |color: i32| -> Option<Point> {
            let idx = full.index_from_color(color)?;
            let pts = full.substring(idx).extract_list(POINTS_EQUALS);
            if pts.is_empty() {
                None
            } else {
                Some(min_xy(&pts))
            }
        };
        let mut d = self.get_mut();
        d.extremity1 = e1;
        d.extremity2 = e2;
        let sb = d.sb.clone();
        if d.label_text.is_some() || d.link.get().link_constraint.is_some() {
            if let Some(pos) = xy(d.note_label_color) {
                let dim = match &d.label_text {
                    Some(t) => t.calculate_dimension(&*sb),
                    None => CONSTRAINT_SPOT,
                };
                d.label_xy = Some(Placed { pos, dim });
            }
        }
        if let Some(t) = d.start_tail_text.clone().or(d.start_tail_role.clone()) {
            if let Some(pos) = xy(d.start_tail_color) {
                d.start_tail_label_xy = Some(Placed {
                    pos,
                    dim: t.calculate_dimension(&*sb),
                });
            }
        }
        if let Some(t) = d.end_head_text.clone().or(d.end_head_role.clone()) {
            if let Some(pos) = xy(d.end_head_color) {
                d.end_head_label_xy = Some(Placed {
                    pos,
                    dim: t.calculate_dimension(&*sb),
                });
            }
        }
        if dot_path.beziers().len() > 1 {
            d.opale = false;
            d.link.set_opale(false);
        }
        d.dot_path = Some(dot_path);
    }

    /// Keeps the quantifier labels away from the nodes (`manageCollision`).
    pub fn manage_collision(&self, nodes: &[SvekNode]) {
        let mut d = self.get_mut();
        for sh in nodes {
            let area = sh.rectangle_area();
            let cl = RectangleArea::new(
                area.min_x - 8.0,
                area.min_y - 8.0,
                area.max_x + 8.0,
                area.max_y + 8.0,
            );
            if d.start_tail_text.is_some() || d.start_tail_role.is_some() {
                if let Some(p) = d.start_tail_label_xy {
                    d.start_tail_label_xy = Some(move_away_from(cl, p));
                }
            }
            if d.end_head_text.is_some() || d.end_head_role.is_some() {
                if let Some(p) = d.end_head_label_xy {
                    d.end_head_label_xy = Some(move_away_from(cl, p));
                }
            }
        }
    }

    pub fn draw(&self, ug: &UGraphic, ids: &mut Vec<String>) {
        let d = self.get();
        if d.opale || d.link.is_invis() {
            return;
        }
        let Some(dot_path) = d.dot_path.as_ref() else {
            return;
        };
        let link = &d.link;
        let mut attrs: Vec<(&str, String)> = vec![("class", "link".to_string())];
        attrs.push(("data-entity-1", link.entity1().uid()));
        attrs.push(("data-entity-2", link.entity2().uid()));
        if let Some(n) = link.type_().link_type_name() {
            attrs.push(("data-link-type", n.to_string()));
        }
        attrs.push(("id", link.uid()));
        ug.start_group(&attrs);
        let mut x = 0.0;
        let y = d.dy;
        if link.is_auto_link_of_a_group() {
            if let Some(cl) = d.bib.cluster_of(&link.entity1()) {
                if let Some(r) = cl.rectangle_area() {
                    x += r.width();
                    x -= dot_path.start_point().0 - r.min_x;
                }
            }
        }
        x += d.dx;
        let style = d
            .skin
            .style_builder()
            .merged_style(
                &queries::ARROW
                    .add(skin_style_name(&d.skin))
                    .with_stereotype_obj(link.stereotype().as_ref()),
            )
            .expect("arrow style");
        let suggested = style.stroke();
        let mut color = style.value(PName::LineColor).as_color();
        let head = style.value(PName::HeadColor);
        let mut arrow_head_color = if matches!(head, crate::style::Value::Null) {
            color.clone()
        } else {
            head.as_color()
        };
        let colors = link.colors();
        if let Some(c) = colors.color_or(ColorType::Arrow, ColorType::Line) {
            color = c.clone();
            arrow_head_color = color.clone();
        }
        let ug = ug
            .apply_back_color(HColor::TRANSPARENT)
            .apply_color(color.clone());
        let link_type = link.type_();
        let stroke = if !link_type.style().is_normal() {
            link_type.stroke3(d.default_thickness)
        } else {
            link_type.stroke3(Some(suggested))
        };
        let stroke = colors.specific_line_stroke().unwrap_or(stroke);
        let ug = ug.apply_stroke(stroke);
        let todraw = dot_path.clone();
        let tr = ug.apply_translate(UTranslate::new(x, y));
        self.draw_rainbow(&tr, &color, &arrow_head_color, &todraw, stroke, &d);
        let _ = ids;
        let ug2 = ug
            .apply_stroke(UStroke::with_thickness(1.0))
            .apply_color(color.clone());
        if let (Some(t), Some(p)) = (&d.label_text, &d.label_xy) {
            let hide = link
                .note()
                .is_some_and(|n| n.strategy == NoteLinkStrategy::HalfNotPrinted);
            if !hide {
                t.draw(&ug2.apply_translate(UTranslate::new(
                    x + p.pos.0 + d.label_shield,
                    y + p.pos.1 + d.label_shield,
                )));
            }
        }
        if let Some(p) = &d.start_tail_label_xy {
            if let Some(t) = &d.start_tail_text {
                t.draw(&ug2.apply_translate(UTranslate::new(x + p.pos.0, y + p.pos.1)));
                if let Some(r) = &d.start_tail_role {
                    self.draw_role(
                        &ug2,
                        r,
                        t,
                        p,
                        dot_path.start_point(),
                        dot_path.end_point(),
                        x,
                        y,
                        &d,
                    );
                }
            } else if let Some(r) = &d.start_tail_role {
                r.draw(&ug2.apply_translate(UTranslate::new(x + p.pos.0, y + p.pos.1)));
            }
        }
        if let Some(p) = &d.end_head_label_xy {
            if let Some(t) = &d.end_head_text {
                t.draw(&ug2.apply_translate(UTranslate::new(x + p.pos.0, y + p.pos.1)));
                if let Some(r) = &d.end_head_role {
                    self.draw_role(
                        &ug2,
                        r,
                        t,
                        p,
                        dot_path.end_point(),
                        dot_path.start_point(),
                        x,
                        y,
                        &d,
                    );
                }
            } else if let Some(r) = &d.end_head_role {
                r.draw(&ug2.apply_translate(UTranslate::new(x + p.pos.0, y + p.pos.1)));
            }
        }
        if link_type.middle_decor() != LinkMiddleDecor::None {
            let middle = dot_path.middle();
            let angle_deg = -middle.angle * 180.0 / std::f64::consts::PI;
            let mi = middle_factory(
                link_type.middle_decor(),
                &d.arrow_lollipop_color,
                &d.background,
            )
            .create(angle_deg - 45.0);
            mi.draw(&ug2.apply_translate(UTranslate::new(x + middle.point.0, y + middle.point.1)));
        }
        ug.close_group();
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_role(
        &self,
        ug: &UGraphic,
        role: &Rc<dyn TextBlock>,
        quantifier: &Rc<dyn TextBlock>,
        qpos: &Placed,
        this_end: Point,
        other_end: Point,
        x: f64,
        y: f64,
        d: &EdgeData,
    ) {
        let samples = d.dot_path.as_ref().map(DotPath::sample);
        let mut areas = Vec::new();
        for n in [self.node1(), self.node2()].into_iter().flatten() {
            areas.push(n.rectangle_area().move_by(-d.dx, -d.dy));
        }
        let pos = crate::svek::role_labels::position(
            quantifier.calculate_dimension(&*d.sb),
            role.calculate_dimension(&*d.sb),
            qpos.pos,
            this_end,
            other_end,
            samples.as_deref(),
            &areas,
        );
        role.draw(&ug.apply_translate(UTranslate::new(x + pos.0, y + pos.1)));
    }

    fn draw_rainbow(
        &self,
        ug: &UGraphic,
        color: &HColor,
        head_color: &HColor,
        todraw: &DotPath,
        stroke: UStroke,
        d: &EdgeData,
    ) {
        ug.draw(todraw);
        let link_type = d.link.type_();
        let thin = UStroke::with_thickness(stroke.thickness);
        for (ext, decor) in [
            (&d.extremity1, link_type.decor2()),
            (&d.extremity2, link_type.decor1()),
        ] {
            if head_color.is_transparent() {
                if let Some(e) = ext {
                    e.draw_line_if_transparent(&ug.apply_color(color.clone()).apply_stroke(thin));
                }
            } else if let Some(e) = ext {
                let mut uh = ug.apply_color(head_color.clone()).apply_stroke(thin);
                uh = if decor.is_fill() {
                    uh.apply_back_color(color.clone())
                } else {
                    uh.apply_back_color(HColor::TRANSPARENT)
                };
                e.draw(&uh);
            }
        }
    }
}

fn move_away_from(cl: RectangleArea, p: Placed) -> Placed {
    let r = RectangleArea::new(
        p.pos.0,
        p.pos.1,
        p.pos.0 + p.dim.width,
        p.pos.1 + p.dim.height,
    );
    let intersects =
        cl.min_x < r.max_x && cl.max_x > r.min_x && cl.min_y < r.max_y && cl.max_y > r.min_y;
    if !intersects {
        return p;
    }
    // Move along the shortest axis out of the rectangle (`PositionableUtils.moveAwayFrom`).
    let left = r.max_x - cl.min_x;
    let right = cl.max_x - r.min_x;
    let up = r.max_y - cl.min_y;
    let down = cl.max_y - r.min_y;
    let m = left.min(right).min(up).min(down);
    let (dx, dy) = if m == left {
        (-left, 0.0)
    } else if m == right {
        (right, 0.0)
    } else if m == up {
        (0.0, -up)
    } else {
        (0.0, down)
    };
    Placed {
        pos: (p.pos.0 + dx, p.pos.1 + dy),
        dim: p.dim,
    }
}

fn skin_style_name(_skin: &SkinParam) -> crate::style::SName {
    crate::style::SName::ClassDiagram
}

fn role_block(role: &str, font: &FontConfiguration, skin: &SkinParam) -> Rc<dyn TextBlock> {
    use crate::skin::visibility::VisibilityModifier;
    let mut text = role.to_string();
    let mut modifier = None;
    if skin.class_attribute_icon_size() > 0 {
        modifier = VisibilityModifier::from_text(role, true);
        if modifier.is_some() {
            text = role.chars().skip(1).collect::<String>().trim().to_string();
        }
    }
    let result = Display::with_newlines(Some(&text)).create_block(
        font,
        skin.default_text_alignment(HorizontalAlignment::Center),
        &skin.skin_simple(),
        &crate::klimt::creole::LineBreakStrategy::NONE,
        CreoleMode::Full,
        None,
        None,
        0.0,
        0.0,
    );
    match modifier {
        None => result,
        Some(m) => {
            let style = skin
                .style_builder()
                .merged_style(&m.style_query())
                .expect("icon style");
            let fore = style.value(PName::LineColor).as_color();
            let icon = with_margin(
                m.u_block(skin.class_attribute_icon_size(), fore, None, false),
                Margins::new(2.0, 1.0, 0.0, 0.0),
            );
            Rc::new(HorizontalBlock {
                blocks: vec![icon, result],
                alignment: VerticalAlignment::Center,
            })
        }
    }
}

fn add_visibility_modifier(
    block: Rc<dyn TextBlock>,
    link: &Link,
    skin: &SkinParam,
    start: &EntityPort,
    end: &EntityPort,
) -> Rc<dyn TextBlock> {
    let mut block = block;
    if let Some(m) = link.visibility_modifier() {
        let style = skin
            .style_builder()
            .merged_style(&m.style_query())
            .expect("icon style");
        let fore = style.value(PName::LineColor).as_color();
        let visibility = with_margin(
            m.u_block(skin.class_attribute_icon_size(), fore, None, false),
            Margins::new(2.0, 0.0, 0.0, 1.0),
        );
        block = Rc::new(HorizontalBlock {
            blocks: vec![visibility, block],
            alignment: VerticalAlignment::Center,
        });
    }
    let margin_label = if start.equals_id(end) { 6.0 } else { 1.0 };
    with_margin(
        block,
        Margins::new(margin_label, margin_label, margin_label, margin_label),
    )
}

pub fn append_table(sb: &mut String, dim: Dimension, color: i32) {
    sb.push_str(&format!(
        "<TABLE BGCOLOR=\"{}\" FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\"><TR><TD></TD></TR></TABLE>",
        hex_rgb(color),
        dim.width as i32,
        dim.height as i32
    ));
}

#[allow(dead_code)]
fn _unused(_: LinkArrow) {}
