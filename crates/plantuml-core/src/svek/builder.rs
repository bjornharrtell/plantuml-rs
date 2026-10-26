//! Lays a diagram out with Graphviz and draws it (`GraphvizImageBuilder`, `DotStringFactory`, `SvekResult`).

use super::bibliotekon::Bibliotekon;
use super::cluster::Cluster;
use super::edge::SvekEdge;
use super::image::{EntityImageSimpleEmpty, IEntityImage};
use super::image_class::EntityImageClass;
use super::node::SvekNode;
use super::types::{
    get_value, max_xy, min_xy, pixel_to_inches, ShapeType, SvgResult, POINTS_EQUALS,
};
use crate::abel::{Entity, Link};
use crate::cucadiagram::CucaDiagram;
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic, UShape, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::{Rankdir, SkinParam};
use std::cell::RefCell;
use std::rc::Rc;

/// An image for a leaf of the diagram (`GeneralImageBuilder.createEntityImageBlock`).
pub(crate) fn create_entity_image(
    entity: &Entity,
    cuca: &CucaDiagram,
    skin: &SkinParam,
) -> Rc<dyn IEntityImage> {
    match entity.leaf_type() {
        Some(_) if skin.is_state_diagram() => {
            if let Some(image) = super::image_state::state_image(entity, cuca, skin) {
                return image;
            }
            Rc::new(EntityImageSimpleEmpty)
        }
        Some(crate::abel::LeafType::Object) => {
            Rc::new(super::image_object::EntityImageObject::new(
                entity.clone(),
                cuca,
                skin,
            ))
        }
        Some(t) if t.is_like_class() => Rc::new(EntityImageClass::new(entity.clone(), cuca, skin)),
        Some(crate::abel::LeafType::Portin | crate::abel::LeafType::Portout) => {
            Rc::new(super::image_port::EntityImagePort::new(
                entity.clone(),
                cuca,
                skin,
            ))
        }
        Some(
            crate::abel::LeafType::Description
            | crate::abel::LeafType::Usecase
            | crate::abel::LeafType::UsecaseBusiness
            | crate::abel::LeafType::Circle,
        ) => Rc::new(super::image_description::EntityImageDescription::new(
            entity.clone(),
            cuca,
            skin,
        )),
        Some(crate::abel::LeafType::PointForAssociation) => Rc::new(
            super::image_misc::EntityImageAssociationPoint::new(entity, cuca, skin),
        ),
        Some(crate::abel::LeafType::Association) => Rc::new(
            super::image_misc::EntityImageAssociation::new(entity, cuca, skin),
        ),
        Some(crate::abel::LeafType::Note) => {
            Rc::new(super::image_note::EntityImageNote::new(entity, cuca, skin))
        }
        other => {
            log::warn!("no image for {other:?}");
            Rc::new(EntityImageSimpleEmpty)
        }
    }
}

struct DotLayout {
    bib: Bibliotekon,
    root: Cluster,
    skin: SkinParam,
}

impl DotLayout {
    fn dot_string(&self, sb: &dyn StringBounder, dot_strings: &[String]) -> String {
        let lines = self.bib.all_lines();
        let max_dzeta = |f: &dyn Fn(&SvekEdge) -> f64| lines.iter().map(f).fold(0.0, f64::max);
        let mut nodesep = max_dzeta(&|l| l.horizontal_dzeta(sb)) / 10.0;
        if nodesep < 35.0 {
            nodesep = 35.0;
        }
        let ns = self.skin.as_int("nodesep", 0);
        if ns != 0 {
            nodesep = f64::from(ns);
        }
        let mut ranksep = max_dzeta(&|l| l.vertical_dzeta(sb)) / 10.0;
        if ranksep < 60.0 {
            ranksep = 60.0;
        }
        let rs = self.skin.as_int("ranksep", 0);
        if rs != 0 {
            ranksep = f64::from(rs);
        }
        let mut out = String::from("digraph unix {\n");
        for s in dot_strings {
            if s.starts_with("ranksep") {
                out.push_str(&format!("ranksep={};", pixel_to_inches(ranksep)));
            } else if s.starts_with("nodesep") {
                out.push_str(&format!("nodesep={};", pixel_to_inches(nodesep)));
            } else {
                out.push_str(s);
            }
            out.push('\n');
        }
        out.push_str("remincross=true;\nsearchsize=500;\n");
        let ortho = match self.skin.get_value("linetype") {
            Some(v) if v.eq_ignore_ascii_case("polyline") => {
                out.push_str("splines=polyline;\n");
                false
            }
            Some(v) if v.eq_ignore_ascii_case("ortho") => {
                out.push_str("splines=ortho;forcelabels=true;\n");
                true
            }
            _ => false,
        };
        if self.skin.rankdir == Rankdir::LeftToRight {
            out.push_str("rankdir=LR;\n");
        }
        self.root.print_cluster1(&mut out, &lines);
        for line in self.bib.lines0() {
            line.append_line(&mut out, ortho);
        }
        self.root
            .print_cluster2(&mut out, &lines, self.skin.use_rank_same());
        for line in self.bib.lines1() {
            line.append_line(&mut out, ortho);
        }
        out.push_str("\n}");
        out
    }

    fn solve(&self, svg: &str) -> Result<(), String> {
        static HEADER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r#"(?m)<svg\s+width="(\d+)pt"\s+height="(\d+)pt""#).expect("valid")
        });
        let c = HEADER.captures(svg).ok_or("no svg header")?;
        let full_height: f64 = c[2].parse().map_err(|_| "bad height")?;
        let result = SvgResult::new(svg, full_height);
        for node in self.bib.all_nodes() {
            let title = format!("<title>{}</title>", node.uid());
            let mut idx = svg.find(&title).ok_or("node not found")?;
            match node.shape_type() {
                ShapeType::Rectangle
                | ShapeType::RectangleHtmlForPorts
                | ShapeType::RectangleWithCircleInside
                | ShapeType::Folder
                | ShapeType::Diamond
                | ShapeType::RectanglePort => {
                    let points = result.substring(idx).extract_list(POINTS_EQUALS);
                    if points.is_empty() {
                        return Err("no points for node".into());
                    }
                    let min = min_xy(&points);
                    node.move_delta(min.0, min.1);
                }
                ShapeType::RoundRectangle => {
                    let idx2 = svg[idx + 1..].find("d=\"").map(|i| i + idx + 1);
                    let pidx = svg[idx + 1..].find("points=\"").map(|i| i + idx + 1);
                    let points = match (idx2, pidx) {
                        (Some(i2), p) if p.is_none_or(|p| i2 < p) => {
                            result.substring(i2).extract_list(super::types::D_EQUALS)
                        }
                        (_, Some(mut p)) => {
                            let mut pts = result.substring(p).extract_list(POINTS_EQUALS);
                            for _ in 0..3 {
                                if let Some(n) = svg[p + 1..].find("points=\"") {
                                    p += 1 + n;
                                    pts.extend(result.substring(p).extract_list(POINTS_EQUALS));
                                }
                            }
                            pts
                        }
                        _ => Vec::new(),
                    };
                    if points.is_empty() {
                        return Err("no points for node".into());
                    }
                    let min = min_xy(&points);
                    node.move_delta(min.0, min.1);
                }
                ShapeType::Octagon | ShapeType::Hexagon => {
                    idx = svg[idx + 1..]
                        .find("points=\"")
                        .map(|i| i + idx + 1)
                        .ok_or("no points")?;
                    let points = result.substring(idx).extract_list(POINTS_EQUALS);
                    let min = min_xy(&points);
                    node.move_delta(min.0, min.1);
                    node.set_polygon(min.0, min.1, &points);
                }
                ShapeType::Circle | ShapeType::Oval => {
                    let cx = get_value(svg, idx, "cx").ok_or("cx")?;
                    let cy = get_value(svg, idx, "cy").ok_or("cy")? + full_height;
                    let rx = get_value(svg, idx, "rx").ok_or("rx")?;
                    let ry = get_value(svg, idx, "ry").ok_or("ry")?;
                    node.move_delta(cx - rx, cy - ry);
                }
                ShapeType::Port => return Err("unsupported shape".into()),
            }
        }
        for cluster in self.bib.all_cluster() {
            if cluster.group().is_packed() {
                continue;
            }
            let color = cluster.get().color;
            let idx = result.index_from_color(color).ok_or("cluster color")?;
            let points = result.substring(idx).extract_list(POINTS_EQUALS);
            if points.is_empty() {
                return Err("no cluster points".into());
            }
            cluster.set_position(min_xy(&points), max_xy(&points));
        }
        for line in self.bib.all_lines() {
            line.solve_line(&result);
        }
        let nodes = self.bib.all_nodes();
        for line in self.bib.all_lines() {
            line.manage_collision(&nodes);
        }
        Ok(())
    }

    fn move_delta(&self, dx: f64, dy: f64) {
        for n in self.bib.all_nodes() {
            n.move_delta(dx, dy);
        }
        for l in self.bib.all_lines() {
            l.move_delta(dx, dy);
        }
        for c in self.bib.all_cluster() {
            c.move_delta(dx, dy);
        }
    }
}

/// Everything laid out, ready to draw (`SvekResult`).
struct SvekResult {
    layout: DotLayout,
    min_max: RefCell<Option<crate::klimt::geom::MinMax>>,
}

impl TextBlock for SvekResult {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        if self.min_max.borrow().is_none() {
            let mm = find_limits(sb.to_rc(), false, |ug| self.draw_all(ug));
            self.layout.move_delta(6.0 - mm.min_x, 6.0 - mm.min_y);
            *self.min_max.borrow_mut() = Some(mm);
        }
        let mm = self.min_max.borrow().expect("computed");
        Dimension::new(mm.width() + 15.0, mm.height() + 15.0)
    }

    fn draw(&self, ug: &UGraphic) {
        self.draw_all(ug);
    }
}

impl SvekResult {
    fn draw_all(&self, ug: &UGraphic) {
        for node in self.layout.bib.all_nodes() {
            let ug2 = if node.is_hidden() {
                ug.apply_hidden(true)
            } else {
                ug.clone()
            };
            node.image()
                .draw(&ug2.apply_translate(UTranslate::new(node.min_x(), node.min_y())));
        }
        let mut ids = Vec::new();
        for line in self.layout.bib.all_lines() {
            let ug2 = if line.is_hidden() {
                ug.apply_hidden(true)
            } else {
                ug.clone()
            };
            line.draw(&ug2, &mut ids);
        }
    }
}

/// Lays the diagram out and returns what draws it.
pub fn build_image(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
) -> Rc<dyn TextBlock> {
    let leafs = cuca.leafs();
    let links: Vec<Link> = ordered_links(cuca);
    if leafs.is_empty() {
        return Rc::new(EmptyImage);
    }
    let bib = Bibliotekon::new();
    let root_entity = cuca.root_group();
    let colors = [
        bib.next_color(),
        bib.next_color(),
        bib.next_color(),
        bib.next_color(),
    ];
    let root = Cluster::new_root(root_entity.clone(), colors);
    if false && leafs.len() == 1 && cuca.groups().is_empty() && links.is_empty() {
        let single = leafs[0].clone();
        if single.parent_container().is_some_and(|p| p.is_root()) {
            let image = create_entity_image(&single, cuca, skin);
            return Rc::new(Degenerated { image });
        }
    }
    for ent in &leafs {
        if cuca.is_entity_removed(ent) {
            continue;
        }
        if ent.parent_container().is_some_and(|p| p.is_root()) {
            let image = create_entity_image(ent, cuca, skin);
            let node = SvekNode::new(
                ent.clone(),
                image,
                &mut bib.get_mut().color_sequence,
                sb.clone(),
                links.iter().any(|l| {
                    (l.entity1() == *ent && l.has_kal1()) || (l.entity2() == *ent && l.has_kal2())
                }),
            );
            bib.add_node(ent.clone(), node.clone());
            root.add_node(node);
        }
    }
    for link in &links {
        if !cuca.is_link_removed(link) && (link.role1().is_some() || link.role2().is_some()) {
            SvekEdge::reserve_room_for_roles(link, skin, &**sb);
        }
    }
    for link in &links {
        if cuca.is_link_removed(link) {
            continue;
        }
        let (e1, e2) = (link.entity1(), link.entity2());
        // A leaf inside a package has no node yet (clusters are not laid out), so the link
        // cannot be drawn; skipping it keeps the diagram renderable instead of panicking.
        if !entity_has_node(&bib, &e1) || !entity_has_node(&bib, &e2) {
            log::warn!(
                "link skipped: {} or {} has no node (grouped entities are not laid out yet)",
                e1.qualified_name(),
                e2.qualified_name()
            );
            continue;
        }
        let edge = SvekEdge::new(link.clone(), skin, sb.clone(), &bib);
        bib.add_line(edge);
    }
    let layout = DotLayout {
        bib,
        root,
        skin: skin.clone(),
    };
    let dot = layout.dot_string(&**sb, &dot_strings(cuca));
    log::debug!("dot input:\n{dot}");
    let svg = match graphviz_pure::render_dot(&dot, "dot", "svg") {
        Ok(s) => s,
        Err(e) => {
            log::error!("graphviz failed: {e}");
            return Rc::new(EmptyImage);
        }
    };
    if let Err(e) = layout.solve(&svg) {
        log::error!("cannot read the layout: {e}");
        return Rc::new(EmptyImage);
    }
    Rc::new(SvekResult {
        layout,
        min_max: RefCell::new(None),
    })
}

fn dot_strings(cuca: &CucaDiagram) -> Vec<String> {
    let mut out = vec!["nodesep=.35;".to_string(), "ranksep=0.8;".to_string()];
    if let Some(a) = cuca.base.pragma.get("aspect") {
        out.push(format!("aspect={};", a.replace(',', ".")));
    }
    if let Some(r) = cuca.base.pragma.get("ratio") {
        out.push(format!("ratio={r};"));
    }
    out
}

/// Whether `ent` takes part in the layout: a node, or a group drawn as a cluster.
fn entity_has_node(bib: &Bibliotekon, ent: &Entity) -> bool {
    bib.node(ent).is_some() || ent.is_group()
}

/// Links grouped by the entities they connect (`getOrderedLinks`).
pub(crate) fn ordered_links(cuca: &CucaDiagram) -> Vec<Link> {
    let mut result: Vec<Link> = Vec::new();
    for link in cuca.links() {
        let mut inserted = false;
        for i in 0..result.len() {
            if result[i].same_connections(link) {
                let mut j = i;
                while j < result.len() && result[j].same_connections(link) {
                    j += 1;
                }
                result.insert(j, link.clone());
                inserted = true;
                break;
            }
        }
        if !inserted {
            result.push(link.clone());
        }
    }
    result
}

struct EmptyImage;

impl TextBlock for EmptyImage {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(10.0, 10.0)
    }

    fn draw(&self, _ug: &UGraphic) {}
}

/// A diagram with a single entity and no link (`EntityImageDegenerated`).
struct Degenerated {
    image: Rc<dyn IEntityImage>,
}

impl TextBlock for Degenerated {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.image.calculate_dimension(sb).delta(14.0, 14.0)
    }

    fn draw(&self, ug: &UGraphic) {
        self.image
            .draw(&ug.apply_translate(UTranslate::new(7.0, 7.0)));
        let dim = self.calculate_dimension(ug.string_bounder());
        ug.apply_translate(UTranslate::new(dim.width - 7.0, dim.height - 7.0))
            .draw(UShape::Empty(7.0, 7.0));
    }
}
