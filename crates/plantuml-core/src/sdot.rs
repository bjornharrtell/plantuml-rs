//! Layout with the same drawing flow as PlantUML's Smetana path (`sdot`).
//!
//! Smetana is a Java port of the Graphviz `dot` layout. The reference images of the Vega tests are
//! made with it, so the nodes and links are laid out here by `graphviz-pure` (the layout core)
//! and drawn exactly as `CucaDiagramFileMakerSmetana` and `SmetanaEdge` do.

use crate::abel::{Entity, LeafType, Link};
use crate::cucadiagram::CucaDiagram;
use crate::json::JsonValue;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::dotpath::{Cubic, DotPath, Point, RectangleArea};
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, MinMax, VerticalAlignment};
use crate::klimt::shape::{with_margin, EmptyBlock, HorizontalBlock, TextBlock, VerticalBlock};
use crate::skin::{Rankdir, SkinParam};
use crate::style::{queries, Margins, PName};
use crate::svek::bibliotekon::Bibliotekon;
use crate::svek::builder::{create_entity_image, ordered_links};
use crate::svek::cluster::Cluster;
use crate::svek::cluster_header::ClusterHeader;
use crate::svek::edge::{role_block, skin_style_name};
use crate::svek::extremity::{extremity_factory_complete, middle_factory};
use crate::svek::image_note::{EntityImageNote, OpalePath};
use crate::svek::node::SvekNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Space the label of an edge takes in the layout, as an HTML table of that size (`createHackInit...`).
fn hack_label(dim: Dimension) -> String {
    format!(
        "<<TABLE FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\"><TR><TD></TD></TR></TABLE>>",
        dim.width as i32, dim.height as i32
    )
}

fn is_empty(b: &Rc<dyn TextBlock>, sb: &dyn StringBounder) -> bool {
    let d = b.calculate_dimension(sb);
    d.width == 0.0 && d.height == 0.0
}

struct EdgeTexts {
    label: Option<Rc<dyn TextBlock>>,
    tail_label: Option<Rc<dyn TextBlock>>,
    head_label: Option<Rc<dyn TextBlock>>,
    tail_role: Option<Rc<dyn TextBlock>>,
    head_role: Option<Rc<dyn TextBlock>>,
}

struct LaidEdge {
    link: Link,
    texts: EdgeTexts,
    hidden: bool,
    path: Option<DotPath>,
    label_pos: Option<Point>,
    head_label_pos: Option<Point>,
    tail_label_pos: Option<Point>,
}

struct Layout {
    bib: Bibliotekon,
    skin: SkinParam,
    nodes: Vec<(Entity, SvekNode, Point, Dimension)>,
    clusters: Vec<(Cluster, (Point, Point))>,
    edges: Vec<LaidEdge>,
    min_max: MinMax,
    canvas_margin: f64,
}

fn json_f64(v: &JsonValue) -> Option<f64> {
    match v {
        JsonValue::Number(s) => s.parse().ok(),
        JsonValue::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn parse_point(s: &str) -> Option<Point> {
    let (x, y) = s.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn parse_spline(s: &str) -> Option<DotPath> {
    let mut pts: Vec<Point> = Vec::new();
    for tok in s.split_whitespace() {
        let t = tok
            .strip_prefix("e,")
            .or_else(|| tok.strip_prefix("s,"))
            .unwrap_or(tok);
        pts.push(parse_point(t)?);
    }
    if pts.len() < 4 {
        return None;
    }
    let mut beziers = Vec::new();
    let mut i = 0;
    while i + 3 < pts.len() {
        let (a, b, c, d) = (pts[i], pts[i + 1], pts[i + 2], pts[i + 3]);
        beziers.push(Cubic::new(a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1));
        i += 3;
    }
    Some(DotPath::from_beziers(beziers))
}

fn mirrored_path(path: &DotPath, max: f64) -> DotPath {
    let m = |p: Point| (p.0, max - p.1);
    DotPath::from_beziers(
        path.beziers()
            .iter()
            .map(|b| {
                let (p1, c1, c2, p2) = (m(b.p1()), m(b.ctrl_p1()), m(b.ctrl_p2()), m(b.p2()));
                Cubic::new(p1.0, p1.1, c1.0, c1.1, c2.0, c2.1, p2.0, p2.1)
            })
            .collect(),
    )
}

fn arrow_style_font(
    skin: &SkinParam,
    link: Option<&Link>,
) -> crate::klimt::font::config::FontConfiguration {
    let mut q = queries::ARROW.add(skin_style_name(skin));
    if let Some(l) = link {
        q = q.with_stereotype_obj(l.stereotype().as_ref());
    }
    skin.style_builder()
        .merged_style(&q)
        .expect("arrow style")
        .font_configuration(None)
}

fn label_block(link: &Link, skin: &SkinParam, sb: &dyn StringBounder) -> Option<Rc<dyn TextBlock>> {
    let font = arrow_style_font(skin, Some(link));
    let mut label_only: Option<Rc<dyn TextBlock>> = None;
    let label = link.label();
    if !label.is_null() {
        let q = queries::ARROW
            .add(skin_style_name(skin))
            .with_stereotype_obj(link.stereotype().as_ref());
        let style = skin.style_builder().merged_style(&q).expect("arrow style");
        let wrap = {
            let w = style.wrap_width();
            if w.max_width() > 0.0 {
                w
            } else {
                skin.max_message_size()
            }
        };
        let block = label.create_block(
            &font,
            skin.default_text_alignment(HorizontalAlignment::Center),
            &skin.skin_simple(),
            &wrap,
            crate::klimt::creole::CreoleMode::SimpleLine,
            None,
            None,
            0.0,
            0.0,
        );
        let mut block = block;
        if let Some(m) = link.visibility_modifier() {
            let st = skin
                .style_builder()
                .merged_style(&m.style_query())
                .expect("icon style");
            let fore = st.value(PName::LineColor).as_color();
            let vis = with_margin(
                m.u_block(skin.class_attribute_icon_size(), fore, None, false),
                Margins::new(2.0, 0.0, 0.0, 1.0),
            );
            block = Rc::new(HorizontalBlock {
                blocks: vec![vis, block],
                alignment: VerticalAlignment::Center,
            });
        }
        label_only = Some(with_margin(block, Margins::same(1.0)));
    }
    let lo = |lo: Option<Rc<dyn TextBlock>>| -> Rc<dyn TextBlock> {
        lo.unwrap_or_else(|| {
            Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            })
        })
    };
    match link.note() {
        None => label_only.filter(|b| !is_empty(b, sb)),
        Some(note) => {
            let note_only = crate::svek::note_link::note_link_block(&note, skin);
            use crate::klimt::geom::Position;
            Some(match note.position {
                Position::Left => Rc::new(HorizontalBlock {
                    blocks: vec![note_only, lo(label_only)],
                    alignment: VerticalAlignment::Center,
                }) as Rc<dyn TextBlock>,
                Position::Right => Rc::new(HorizontalBlock {
                    blocks: vec![lo(label_only), note_only],
                    alignment: VerticalAlignment::Center,
                }),
                Position::Top => Rc::new(VerticalBlock {
                    blocks: vec![note_only, lo(label_only)],
                    alignment: HorizontalAlignment::Center,
                }),
                Position::Bottom => Rc::new(VerticalBlock {
                    blocks: vec![lo(label_only), note_only],
                    alignment: HorizontalAlignment::Center,
                }),
            })
        }
    }
}

fn marged(b: Rc<dyn TextBlock>, sb: &dyn StringBounder) -> Rc<dyn TextBlock> {
    if is_empty(&b, sb) {
        b
    } else {
        with_margin(b, Margins::same(1.0))
    }
}

fn edge_texts(link: &Link, skin: &SkinParam, sb: &dyn StringBounder) -> EdgeTexts {
    let font = arrow_style_font(skin, None);
    let quant = |q: Option<String>| {
        q.map(|q| {
            let b = crate::klimt::creole::display::Display::with_newlines(Some(&q)).create_block(
                &font,
                skin.default_text_alignment(HorizontalAlignment::Center),
                &skin.skin_simple(),
                &crate::klimt::creole::LineBreakStrategy::NONE,
                crate::klimt::creole::CreoleMode::Full,
                None,
                None,
                0.0,
                0.0,
            );
            marged(b, sb)
        })
    };
    let role = |r: Option<String>| r.map(|r| marged(role_block(&r, &font, skin), sb));
    let (q1, q2) = (quant(link.quantifier1()), quant(link.quantifier2()));
    let (r1, r2) = (role(link.role1()), role(link.role2()));
    EdgeTexts {
        label: label_block(link, skin, sb),
        tail_label: q1.clone().or_else(|| r1.clone()),
        head_label: q2.clone().or_else(|| r2.clone()),
        tail_role: if q1.is_some() { r1 } else { None },
        head_role: if q2.is_some() { r2 } else { None },
    }
}

/// Smetana rounds node sizes to whole points (`POINTS`).
fn java_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

fn java_f64(v: f64) -> String {
    crate::svek::node::java_double(v)
}

/// A package as laid out: its cluster, its leaves and its sub packages (`exportGroup`).
struct GroupNode {
    group: Entity,
    cluster: Cluster,
    leaves: Vec<Entity>,
    children: Vec<GroupNode>,
}

struct Collected {
    bib: Bibliotekon,
    root_leaves: Vec<Entity>,
    groups: Vec<GroupNode>,
    nodes: Vec<(Entity, SvekNode)>,
}

fn collect_groups(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    parent: &Entity,
    parent_cluster: &Cluster,
    nested: bool,
    c: &mut Collected,
) -> Vec<GroupNode> {
    let mut out = Vec::new();
    for g in parent.groups() {
        if cuca.is_entity_removed(&g) {
            continue;
        }
        if nested && g.group_type() == Some(crate::abel::GroupType::ConcurrentState) {
            continue;
        }
        if g.is_packed() {
            // A packed package is not drawn: its child keeps the merged title.
            out.extend(collect_groups(
                cuca,
                skin,
                sb,
                &g,
                parent_cluster,
                nested,
                c,
            ));
            continue;
        }
        let header = ClusterHeader::new(&g, cuca, skin, skin.diagram_style_name(), &**sb);
        let colors = [
            c.bib.next_color(),
            c.bib.next_color(),
            c.bib.next_color(),
            c.bib.next_color(),
        ];
        let cluster = parent_cluster.create_child(g.clone(), colors, header);
        cluster.get_mut().hidden = cuca.is_entity_hidden(&g);
        c.bib.add_cluster(cluster.clone());
        let mut leaves = Vec::new();
        for leaf in g.leafs() {
            if cuca.is_entity_removed(&leaf) {
                continue;
            }
            add_node(cuca, skin, sb, &leaf, &cluster, c);
            leaves.push(leaf);
        }
        let children = collect_groups(cuca, skin, sb, &g, &cluster, nested, c);
        out.push(GroupNode {
            group: g,
            cluster,
            leaves,
            children,
        });
    }
    out
}

fn add_node(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    ent: &Entity,
    cluster: &Cluster,
    c: &mut Collected,
) {
    let image: Rc<dyn crate::svek::image::IEntityImage> = if let Some(image) = ent.svek_image() {
        image
    } else if ent.leaf_type() == Some(LeafType::Tips) {
        Rc::new(crate::svek::image_tips::EntityImageTips::new(
            ent, cuca, skin, &c.bib,
        ))
    } else {
        create_entity_image(ent, cuca, skin)
    };
    let node = SvekNode::new(
        ent.clone(),
        image,
        &mut c.bib.get_mut().color_sequence,
        sb.clone(),
        false,
    );
    c.bib.add_node(ent.clone(), node.clone());
    cluster.add_node(node.clone());
    c.nodes.push((ent.clone(), node));
}

fn group_core_name(g: &Entity) -> String {
    format!("z{}", g.uid())
}

fn node_dot(n: &SvekNode, sb: &dyn StringBounder) -> String {
    let ports = n.image().ports(sb);
    // The table is written whenever a link names a member, even when no row matches (the name is
    // not a member), as Java does: the edge then keeps its port and Graphviz warns.
    if crate::svek::ports::has_ports(&n.entity()) {
        // A member-anchored link (`Foo::field --> ...`) needs the rows as ports; a plain box
        // would attach the edge to the border and lose the member.
        return format!(
            "{}\n",
            crate::svek::node::label_html_for_ports(&n.uid(), 0, n.width(), n.height(), &ports)
        );
    }
    format!(
        "{} [shape=box,label=\"\",width={},height={}];\n",
        n.uid(),
        java_f64(java_round(n.width()) / 72.0),
        java_f64(java_round(n.height()) / 72.0)
    )
}

/// Attaches an edge end to a member row (`EntityPort`): `uid:p<md5>` when the link names one.
fn dot_port(uid: String, port: Option<&str>, is_group: bool) -> String {
    match port {
        Some(p) if !p.is_empty() && !is_group => {
            format!("{uid}:{}", crate::svek::bibliotekon::encode_port_name(p))
        }
        _ => uid,
    }
}

fn group_dot(
    out: &mut String,
    g: &GroupNode,
    nodes: &[(Entity, SvekNode)],
    core_groups: &[Entity],
    sb: &dyn StringBounder,
) {
    out.push_str(&format!("subgraph {} {{\n", g.cluster.cluster_id()));
    let (w, h) = (
        g.cluster.title_and_attribute_width(),
        g.cluster.title_and_attribute_height(),
    );
    if w > 0.0 || h > 0.0 {
        out.push_str(&format!(
            "label={}\n",
            hack_label(Dimension::new(w, h - 5.0 + 8.0))
        ));
    }
    let mut margin = 16;
    if g.group
        .usymbol()
        .is_some_and(|u| u.supp_width_because_of_shape() > 0)
    {
        margin = margin.max(20);
    }
    out.push_str(&format!("margin={margin};\n"));
    for leaf in &g.leaves {
        if let Some((_, n)) = nodes.iter().find(|(e, _)| e == leaf) {
            out.push_str(&node_dot(n, sb));
        }
    }
    for child in &g.children {
        group_dot(out, child, nodes, core_groups, sb);
    }
    if core_groups.contains(&g.group) {
        out.push_str(&format!(
            "{} [shape=box,label=\"\",width=0.1,height=0.1];\n",
            group_core_name(&g.group)
        ));
    }
    out.push_str("}\n");
}

fn dot_endpoint(e: &Entity, nodes: &[(Entity, SvekNode)]) -> Option<String> {
    if e.is_group() {
        Some(group_core_name(e))
    } else {
        nodes.iter().find(|(x, _)| x == e).map(|(_, n)| n.uid())
    }
}

fn build_dot(
    skin: &SkinParam,
    collected: &Collected,
    edges: &[(Link, EdgeTexts)],
    sb: &dyn StringBounder,
    spacing: Spacing,
) -> String {
    let mut out = String::from("digraph g {\n");
    if skin.is_state_diagram() {
        out.push_str("margin=16;\n");
        out.push_str("remincross=true;\n");
        out.push_str("searchsize=500;\n");
    }
    let ns = skin.as_int("nodesep", 0);
    let ns = if ns != 0 {
        f64::from(ns)
    } else {
        spacing.nodesep
    };
    if ns != 0.0 {
        out.push_str(&format!(
            "nodesep={};\n",
            crate::svek::types::pixel_to_inches(ns)
        ));
    }
    let rs = skin.as_int("ranksep", 0);
    let rs = if rs != 0 {
        f64::from(rs)
    } else {
        spacing.ranksep
    };
    if rs != 0.0 {
        out.push_str(&format!(
            "ranksep={};\n",
            crate::svek::types::pixel_to_inches(rs)
        ));
    }
    if skin.rankdir == Rankdir::LeftToRight {
        out.push_str("rankdir=LR;\n");
    }
    let nodes = &collected.nodes;
    let mut core_groups: Vec<Entity> = Vec::new();
    for (l, _) in edges {
        for e in [l.entity1(), l.entity2()] {
            if e.is_group() && !core_groups.contains(&e) {
                core_groups.push(e);
            }
        }
    }
    for leaf in &collected.root_leaves {
        if let Some((_, n)) = nodes.iter().find(|(e, _)| e == leaf) {
            out.push_str(&node_dot(n, sb));
        }
    }
    for g in &collected.groups {
        group_dot(&mut out, g, nodes, &core_groups, sb);
    }
    for (link, texts) in edges {
        let (Some(a), Some(b)) = (
            dot_endpoint(&link.entity1(), nodes),
            dot_endpoint(&link.entity2(), nodes),
        ) else {
            continue;
        };
        let mut attrs = format!("arrowtail=none,arrowhead=none,minlen={}", link.length() - 1);
        for (name, block) in [
            ("label", &texts.label),
            ("taillabel", &texts.tail_label),
            ("headlabel", &texts.head_label),
        ] {
            if let Some(b) = block {
                if !is_empty(b, sb) {
                    attrs.push_str(&format!(
                        ",{name}={}",
                        hack_label(b.calculate_dimension(sb))
                    ));
                }
            }
        }
        let (port1, port2) = {
            let d = link.get();
            (d.port1.clone(), d.port2.clone())
        };
        let a = dot_port(a, port1.as_deref(), link.entity1().is_group());
        let b = dot_port(b, port2.as_deref(), link.entity2().is_group());
        out.push_str(&format!("{a} -> {b} [{attrs}];\n"));
    }
    out.push('}');
    out
}

/// Draws the label of a port on the outside of its cluster (`EntityImagePort.upPosition`).
fn orient_ports(
    placed: &[(Entity, SvekNode, Point, Dimension)],
    clusters: &[(Cluster, (Point, Point))],
) {
    for (ent, node, pos, _) in placed {
        if !matches!(ent.leaf_type(), Some(LeafType::Portin | LeafType::Portout)) {
            continue;
        }
        let Some(parent) = ent.parent_container() else {
            continue;
        };
        let Some((_, (ll, ur))) = clusters.iter().find(|(c, _)| c.group() == parent) else {
            continue;
        };
        if let Some(image) = node
            .image()
            .as_any()
            .downcast_ref::<crate::svek::image_port::EntityImagePort>()
        {
            image.set_up(pos.1 < (ll.1 + ur.1) / 2.0);
        }
    }
}

/// A note with exactly one visible link, to something that is not a note (`isOpalisable`).
fn is_opalisable(cuca: &CucaDiagram, ent: &Entity) -> bool {
    if ent.is_group() || ent.leaf_type() != Some(LeafType::Note) {
        return false;
    }
    let mut single: Option<&Link> = None;
    for link in cuca.links() {
        if link.is_invis() || !link.contains(ent) {
            continue;
        }
        if single.is_some() {
            return false;
        }
        single = Some(link);
    }
    single.is_some_and(|l| l.other(ent).leaf_type() != Some(LeafType::Note))
}

/// The link of such a note is drawn as the pointer of the note (`setOpaleLink`).
fn mark_opale_links(cuca: &CucaDiagram, links: &[Link], nodes: &[(Entity, SvekNode)]) {
    let has_node = |e: &Entity| nodes.iter().any(|(x, _)| x == e);
    for link in links {
        if cuca.is_link_removed(link) {
            continue;
        }
        let (e1, e2) = (link.entity1(), link.entity2());
        if (is_opalisable(cuca, &e1) && has_node(&e2))
            || (is_opalisable(cuca, &e2) && has_node(&e1))
        {
            link.set_opale(true);
        }
    }
}

fn bb_of(o: &JsonValue) -> Option<(Point, Point)> {
    let s = o.get("bb")?.as_str()?;
    let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    if v.len() == 4 {
        Some(((v[0], v[1]), (v[2], v[3])))
    } else {
        None
    }
}

/// Node and rank separation in pixels (`nodesep`/`ranksep` of the DOT input).
#[derive(Clone, Copy)]
pub struct Spacing {
    pub nodesep: f64,
    pub ranksep: f64,
}

impl Spacing {
    /// Graphviz defaults: what the Smetana flow leaves when the skin sets nothing.
    pub const GRAPHVIZ: Spacing = Spacing {
        nodesep: 0.0,
        ranksep: 0.0,
    };
    /// `DotStringFactory`: at least 35 px between nodes and 60 px between ranks (`nodesep=0.486111`,
    /// `ranksep=0.833333` at 72 dpi).
    pub const SVEK: Spacing = Spacing {
        nodesep: 35.0,
        ranksep: 60.0,
    };
}

fn layout(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    spacing: Spacing,
) -> Option<Layout> {
    layout_from(cuca, skin, sb, None, spacing)
}

/// Lays out the whole diagram, or the content of `root` when it is a nested
/// sub-layout (`CucaDiagramFileMakerSmetana` with a non-root group).
fn layout_from(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    root: Option<&Entity>,
    spacing: Spacing,
) -> Option<Layout> {
    let nested = root.is_some();
    let bib = Bibliotekon::new();
    let root_entity = root.cloned().unwrap_or_else(|| cuca.root_group());
    let colors = [
        bib.next_color(),
        bib.next_color(),
        bib.next_color(),
        bib.next_color(),
    ];
    let root_cluster = Cluster::new_root(root_entity.clone(), colors);
    let links: Vec<Link> = if nested {
        cuca.links()
            .iter()
            .filter(|l| crate::abel::is_pure_inner_link12(&root_entity, l))
            .cloned()
            .collect()
    } else {
        ordered_links(cuca)
    };
    let mut c = Collected {
        bib: bib.clone(),
        root_leaves: Vec::new(),
        groups: Vec::new(),
        nodes: Vec::new(),
    };
    c.groups = collect_groups(cuca, skin, sb, &root_entity, &root_cluster, nested, &mut c);
    for ent in root_entity.leafs() {
        if cuca.is_entity_removed(&ent) {
            continue;
        }
        if nested && ent.leaf_type() == Some(LeafType::StateConcurrent) {
            continue;
        }
        add_node(cuca, skin, sb, &ent, &root_cluster, &mut c);
        c.root_leaves.push(ent);
    }
    mark_opale_links(cuca, &links, &c.nodes);
    let edges: Vec<(Link, EdgeTexts)> = links
        .iter()
        .filter(|l| !cuca.is_link_removed(l))
        .map(|l| (l.clone(), edge_texts(l, skin, &**sb)))
        .collect();
    if c.nodes.is_empty() && c.groups.is_empty() {
        return None;
    }
    let dot = build_dot(skin, &c, &edges, &**sb, spacing);
    log::debug!("dot input:\n{dot}");
    let json = match graphviz_pure::render_dot(&dot, "dot", "json") {
        Ok(j) => j,
        Err(e) => {
            log::error!("graphviz failed: {e}");
            return None;
        }
    };
    let doc = JsonValue::parse(&json).ok()?;
    let objects = doc.get("objects")?;
    let mut placed: Vec<(Entity, SvekNode, Point, Dimension)> = Vec::new();
    let mut cluster_boxes: Vec<(Cluster, (Point, Point))> = Vec::new();
    let all_clusters = bib.all_cluster();
    for i in 0..objects.len() {
        let o = objects.get(i)?;
        let name = o.get("name")?.as_str()?;
        if let Some(cl) = all_clusters.iter().find(|cl| cl.cluster_id() == name) {
            if let Some(bb) = bb_of(o) {
                cluster_boxes.push((cl.clone(), bb));
            }
            continue;
        }
        let Some((ent, node)) = c.nodes.iter().find(|(_, n)| n.uid() == name) else {
            continue;
        };
        let pos = parse_point(o.get("pos")?.as_str()?)?;
        // The layout rounds the size of a node to whole points: use the drawn polygon.
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        // A `shape=box` node lists its box in `_draw_`; a `shape=plaintext` HTML table node only
        // lists the same polygon in `_ldraw_` (the label), so read both.
        for key in ["_draw_", "_ldraw_"] {
            let Some(draw) = o.get(key) else {
                continue;
            };
            for k in 0..draw.len() {
                if let Some(pts) = draw.get(k).and_then(|op| op.get("points")) {
                    for j in 0..pts.len() {
                        let p = pts.get(j)?;
                        let (x, y) = (json_f64(p.get(0usize)?)?, json_f64(p.get(1usize)?)?);
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                }
            }
        }
        placed.push((
            ent.clone(),
            node.clone(),
            pos,
            Dimension::new(max_x - min_x, max_y - min_y),
        ));
    }
    // Graphviz gives no bounding box to a cluster that holds nothing (`package foo { }`, or a C4
    // `System_Boundary(x, "y") { }`), so such a frame would not be drawn at all. Give it the size
    // of its own title and stack it below the laid out content.
    {
        const MARGIN: f64 = 16.0;
        let missing: Vec<Cluster> = all_clusters
            .iter()
            .filter(|cl| cl.parent().is_some())
            .filter(|cl| !cluster_boxes.iter().any(|(placed, _)| placed == *cl))
            .cloned()
            .collect();
        let mut bottom = -MARGIN;
        for cluster in missing {
            let width = cluster.title_and_attribute_width() + 2.0 * MARGIN;
            let height = cluster.title_and_attribute_height() + 2.0 * MARGIN;
            cluster_boxes.push((cluster, ((0.0, bottom - height), (width, bottom))));
            bottom -= height + MARGIN;
        }
    }
    orient_ports(&placed, &cluster_boxes);
    let mut mm = MinMax::empty(false);
    for (_, _, c, d) in &placed {
        // Smetana's box info: upper right has the smaller y.
        mm = mm
            .add_point(c.0 + d.width / 2.0, c.1 - d.height / 2.0)
            .add_point(c.0 - d.width / 2.0, c.1 + d.height / 2.0);
    }
    for (_, (ll, ur)) in &cluster_boxes {
        mm = mm.add_point(ur.0, ur.1).add_point(ll.0, ll.1);
    }
    let json_edges = doc.get("edges");
    let mut laid = Vec::new();
    let mut idx = 0usize;
    for (link, texts) in edges {
        if dot_endpoint(&link.entity1(), &c.nodes).is_none()
            || dot_endpoint(&link.entity2(), &c.nodes).is_none()
        {
            continue;
        }
        let je = json_edges.and_then(|e| e.get(idx));
        idx += 1;
        let prop = |k: &str| je.and_then(|e| e.get(k)).and_then(JsonValue::as_str);
        let path = prop("pos").and_then(parse_spline);
        laid.push(LaidEdge {
            hidden: cuca.is_link_hidden(&link),
            link,
            texts,
            path,
            label_pos: prop("lp").and_then(parse_point),
            head_label_pos: prop("head_lp").and_then(parse_point),
            tail_label_pos: prop("tail_lp").and_then(parse_point),
        });
    }
    Some(Layout {
        bib,
        skin: skin.clone(),
        nodes: placed,
        clusters: cluster_boxes,
        edges: laid,
        min_max: mm,
        canvas_margin: if nested { 0.0 } else { 6.0 },
    })
}

struct Drawing {
    layout: Layout,
    measured: RefCell<Option<MinMax>>,
}

impl Drawing {
    fn max(&self) -> f64 {
        self.layout.min_max.max_y + 6.0
    }

    fn baseline(&self) -> UTranslate {
        UTranslate::new(
            self.layout.canvas_margin,
            self.layout.canvas_margin - self.layout.min_max.min_y,
        )
    }

    fn baseline_dimension(&self) -> Dimension {
        Dimension::new(
            self.layout.min_max.width() + 2.0 * self.layout.canvas_margin + 4.0,
            self.layout.min_max.height() + self.layout.canvas_margin + 16.0,
        )
    }

    fn measured(&self, sb: &dyn StringBounder) -> MinMax {
        if let Some(m) = *self.measured.borrow() {
            return m;
        }
        let base = self.baseline();
        let m = find_limits(sb.to_rc(), false, |ug| {
            self.draw_content(&ug.apply_translate(base))
        });
        log::debug!("measured {m:?} base {:?}", self.baseline_dimension());
        *self.measured.borrow_mut() = Some(m);
        m
    }

    fn draw_content(&self, ug: &UGraphic) {
        let max = self.max();
        for (cluster, (ll, ur)) in &self.layout.clusters {
            cluster.set_position((ur.0, max - ur.1), (ll.0, max - ll.1));
            cluster.draw(ug, &self.layout.skin, cluster.get().hidden);
        }
        // Tips look at the position of the entity they are about.
        for (_, node, c, d) in &self.layout.nodes {
            let mut n = node.get_mut();
            n.min_x = c.0 - d.width / 2.0;
            n.min_y = max - (c.1 + d.height / 2.0);
        }
        for (_, node, c, d) in &self.layout.nodes {
            // `getCorner`: the lower left in layout coordinates, mirrored.
            let corner = (c.0 - d.width / 2.0, max - (c.1 + d.height / 2.0));
            let ug_node = if node.is_hidden() {
                ug.apply_hidden(true)
            } else {
                ug.clone()
            };
            let image = node.image();
            if let Some(note) = image.as_any().downcast_ref::<EntityImageNote>() {
                note.set_opale_path(self.opale_path(note.entity(), corner, max));
            }
            image.draw(&ug_node.apply_translate(UTranslate::new(corner.0, corner.1)));
        }
        for edge in &self.layout.edges {
            if edge.hidden || edge.link.is_invis() || edge.link.is_opale() {
                continue;
            }
            self.draw_edge(ug, edge, max);
        }
    }

    /// Ends of the link of a note, relative to the note (`SmetanaEdge.getStartPoint`).
    fn opale_path(&self, entity: &Entity, corner: Point, max: f64) -> Option<OpalePath> {
        let edge = self
            .layout
            .edges
            .iter()
            .find(|e| e.link.is_opale() && e.link.contains(entity))?;
        let path = mirrored_path(edge.path.as_ref()?, max);
        let rel = |p: Point| (p.0 - corner.0, p.1 - corner.1);
        Some(OpalePath {
            start: rel(path.start_point()),
            end: rel(path.end_point()),
        })
    }

    fn draw_edge(&self, ug: &UGraphic, edge: &LaidEdge, max: f64) {
        let link = &edge.link;
        let skin = &self.layout.skin;
        let mut attrs: Vec<(&str, String)> = vec![("class", "link".to_string())];
        attrs.push(("data-entity-1", link.entity1().uid()));
        attrs.push(("data-entity-2", link.entity2().uid()));
        if let Some(n) = link.type_().link_type_name() {
            attrs.push(("data-link-type", n.to_string()));
        }
        attrs.push(("id", link.uid()));
        ug.start_group(&attrs);
        if let Some(path) = &edge.path {
            let mut dot_path = mirrored_path(path, max);
            let area_of = |e: &Entity| {
                if e.is_group() {
                    self.layout
                        .bib
                        .cluster_of(e)
                        .and_then(|c| c.rectangle_area())
                } else {
                    None
                }
            };
            dot_path =
                dot_path.simulate_compound(area_of(&link.entity2()), area_of(&link.entity1()));
            let q = queries::ARROW
                .add(skin_style_name(skin))
                .with_stereotype_obj(link.stereotype().as_ref());
            let style = skin.style_builder().merged_style(&q).expect("arrow style");
            let mut color = style.value(PName::LineColor).as_color();
            let colors = link.colors();
            if let Some(c) = colors.color_or(ColorType::Arrow, ColorType::Line) {
                color = c.clone();
            }
            let ug = ug
                .apply_back_color(HColor::TRANSPARENT)
                .apply_color(color.clone());
            let link_type = link.type_();
            let suggested = style.stroke();
            let default_thickness = Some(style.stroke());
            let mut stroke = if !link_type.style().is_normal() {
                link_type.stroke3(default_thickness)
            } else {
                link_type.stroke3(Some(suggested))
            };
            if let Some(s) = colors.specific_line_stroke() {
                stroke = s;
            }
            let bg = skin.background_color();
            let mut ug_start = ug.apply_color(color.clone());
            if link_type.decor2().is_fill() {
                ug_start = ug_start.apply_back_color(color.clone());
            }
            let mut ug_end = ug.apply_color(color.clone());
            if link_type.decor1().is_fill() {
                ug_end = ug_end.apply_back_color(color.clone());
            }
            if let Some(f) = extremity_factory_complete(link_type.decor2(), &bg) {
                let start_angle = dot_path.start_angle() + std::f64::consts::PI;
                let e = f.create(dot_path.start_point(), start_angle, None);
                let len = e.decoration_length();
                dot_path.move_start_point(
                    len * (start_angle - std::f64::consts::PI).cos(),
                    len * (start_angle - std::f64::consts::PI).sin(),
                );
                e.draw(&ug_start);
            }
            if let Some(f) = extremity_factory_complete(link_type.decor1(), &bg) {
                let end_angle = dot_path.end_angle();
                let e = f.create(dot_path.end_point(), end_angle, None);
                let len = e.decoration_length();
                dot_path.move_end_point(
                    len * (end_angle - std::f64::consts::PI).cos(),
                    len * (end_angle - std::f64::consts::PI).sin(),
                );
                e.draw(&ug_end);
            }
            ug.apply_stroke(stroke)
                .apply_color(color.clone())
                .draw(&dot_path);
            if link_type.middle_decor() != crate::decoration::link::LinkMiddleDecor::None {
                let middle = dot_path.middle();
                let angle_deg = -middle.angle * 180.0 / std::f64::consts::PI;
                let mi =
                    middle_factory(link_type.middle_decor(), &color, &bg).create(angle_deg - 45.0);
                mi.draw(&ug.apply_translate(UTranslate::new(middle.point.0, middle.point.1)));
            }
            let _ = UStroke::simple();
            // Labels
            let at = |p: Option<Point>, b: &Option<Rc<dyn TextBlock>>| {
                if let (Some(p), Some(b)) = (p, b) {
                    let d = b.calculate_dimension(ug.string_bounder());
                    let tl = (p.0 - d.width / 2.0, max - (p.1 + d.height / 2.0));
                    b.draw(&ug.apply_translate(UTranslate::new(tl.0, tl.1)));
                    Some(tl)
                } else {
                    None
                }
            };
            at(edge.label_pos, &edge.texts.label);
            let head = at(edge.head_label_pos, &edge.texts.head_label);
            let tail = at(edge.tail_label_pos, &edge.texts.tail_label);
            let samples = mirrored_path(path, max).sample();
            let areas: Vec<RectangleArea> = [link.entity1(), link.entity2()]
                .iter()
                .filter_map(|e| self.layout.bib.node(e))
                .map(|n| n.rectangle_area())
                .collect();
            let mp = mirrored_path(path, max);
            let sb = ug.string_bounder();
            if let (Some(role), Some(q), Some(tl)) =
                (&edge.texts.tail_role, &edge.texts.tail_label, tail)
            {
                let pos = crate::svek::role_labels::position(
                    q.calculate_dimension(sb),
                    role.calculate_dimension(sb),
                    tl,
                    mp.start_point(),
                    mp.end_point(),
                    Some(&samples),
                    &areas,
                );
                role.draw(&ug.apply_translate(UTranslate::new(pos.0, pos.1)));
            }
            if let (Some(role), Some(q), Some(tl)) =
                (&edge.texts.head_role, &edge.texts.head_label, head)
            {
                let pos = crate::svek::role_labels::position(
                    q.calculate_dimension(sb),
                    role.calculate_dimension(sb),
                    tl,
                    mp.end_point(),
                    mp.start_point(),
                    Some(&samples),
                    &areas,
                );
                role.draw(&ug.apply_translate(UTranslate::new(pos.0, pos.1)));
            }
        }
        ug.close_group();
    }
}

impl TextBlock for Drawing {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let measured = self.measured(sb);
        let base = self.baseline_dimension();
        let extra_left = (-measured.min_x).max(0.0);
        let extra_top = (-measured.min_y).max(0.0);
        let extra_right = (measured.max_x - base.width).max(0.0);
        let extra_bottom = (measured.max_y - base.height).max(0.0);
        Dimension::new(
            base.width + extra_left + extra_right,
            base.height + extra_top + extra_bottom,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let measured = self.measured(ug.string_bounder());
        let extra_left = (-measured.min_x).max(0.0);
        let extra_top = (-measured.min_y).max(0.0);
        let mut shifted = ug.apply_translate(self.baseline());
        if extra_left != 0.0 || extra_top != 0.0 {
            shifted = shifted.apply_translate(UTranslate::new(extra_left, extra_top));
        }
        self.draw_content(&shifted);
    }
}

struct EmptyImage;

impl TextBlock for EmptyImage {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        Dimension::new(0.0, 0.0)
    }

    fn draw(&self, _ug: &UGraphic) {}
}

/// Lays a diagram out and returns what draws it.
pub fn build_image(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
) -> Rc<dyn TextBlock> {
    build_image_spaced(cuca, skin, sb, Spacing::GRAPHVIZ)
}

/// Lays a diagram out with explicit node and rank separation.
pub fn build_image_spaced(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    spacing: Spacing,
) -> Rc<dyn TextBlock> {
    match layout(cuca, skin, sb, spacing) {
        Some(layout) => Rc::new(Drawing {
            layout,
            measured: RefCell::new(None),
        }),
        None => Rc::new(EmptyImage),
    }
}

/// Lays a state diagram out and returns what draws it.
pub fn build_state_image(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
) -> Rc<dyn TextBlock> {
    simplify_states(cuca, skin, sb);
    build_image(cuca, skin, sb)
}

fn collect_groups_postorder(entity: &Entity, out: &mut Vec<Entity>) {
    for g in entity.groups() {
        collect_groups_postorder(&g, out);
        out.push(g);
    }
}

/// Turns autarkic composite states into pre-rendered leaves
/// (`CucaDiagramSimplifierStateSmetana`).
fn simplify_states(cuca: &CucaDiagram, skin: &SkinParam, sb: &Rc<dyn StringBounder>) {
    let root = cuca.root_group();
    let mut groups = Vec::new();
    collect_groups_postorder(&root, &mut groups);
    for group in groups {
        if !group.is_group() {
            continue;
        }
        let group_type = group.group_type();
        if group_type != Some(crate::abel::GroupType::State)
            && group_type != Some(crate::abel::GroupType::ConcurrentState)
        {
            continue;
        }
        if !crate::abel::is_autarkic(&group, cuca.links()) {
            continue;
        }
        let image = state_group_image(cuca, skin, sb, &group);
        let leaf_type = if group_type == Some(crate::abel::GroupType::ConcurrentState) {
            LeafType::StateConcurrent
        } else {
            LeafType::State
        };
        group.override_image(image, leaf_type);
    }
}

fn nested_state_image(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    root: &Entity,
) -> Rc<dyn crate::svek::image::IEntityImage> {
    let block: Rc<dyn TextBlock> = match layout_from(cuca, skin, sb, Some(root), Spacing::GRAPHVIZ)
    {
        Some(layout) => Rc::new(Drawing {
            layout,
            measured: RefCell::new(None),
        }),
        None => Rc::new(EmptyImage),
    };
    Rc::new(crate::svek::image_state::TextBlockToEntityImage::new(block))
}

/// `GroupMakerStateSmetana.getImage`.
fn state_group_image(
    cuca: &CucaDiagram,
    skin: &SkinParam,
    sb: &Rc<dyn StringBounder>,
    group: &Entity,
) -> Rc<dyn crate::svek::image::IEntityImage> {
    use crate::svek::image_state::{ConcurrentStates, InnerStateAutonom, PaddedEntityImage};
    const REGION_PADDING: f64 = 6.0;
    const SEPARATOR_OVERSHOOT: f64 = 8.0;
    if group.count_children() == 0 && group.groups().is_empty() {
        return Rc::new(crate::svek::image_state::EntityImageState::new(
            group, cuca, skin,
        ));
    }
    if group.group_type() == Some(crate::abel::GroupType::ConcurrentState) {
        let inner = nested_state_image(cuca, skin, sb, group);
        return Rc::new(PaddedEntityImage::uniform(inner, REGION_PADDING));
    }
    let nested = nested_state_image(cuca, skin, sb, group);
    let concurrent: Vec<Rc<dyn crate::svek::image::IEntityImage>> = group
        .leafs()
        .iter()
        .filter(|l| l.leaf_type() == Some(LeafType::StateConcurrent))
        .filter_map(|l| l.svek_image())
        .collect();
    if concurrent.is_empty() {
        return Rc::new(InnerStateAutonom::new(group, nested, cuca, skin));
    }
    let mut inners = vec![Rc::new(PaddedEntityImage::uniform(nested, REGION_PADDING))
        as Rc<dyn crate::svek::image::IEntityImage>];
    inners.extend(concurrent);
    let separator = group.concurrent_separator();
    let stacked = Rc::new(ConcurrentStates::new(inners, separator, skin));
    let right = if separator == '-' {
        SEPARATOR_OVERSHOOT
    } else {
        0.0
    };
    let bottom = if separator == '|' {
        SEPARATOR_OVERSHOOT
    } else {
        0.0
    };
    let padded = Rc::new(PaddedEntityImage::new(stacked, 0.0, 0.0, right, bottom));
    Rc::new(InnerStateAutonom::new(group, padded, cuca, skin))
}
