//! A package drawn as a Graphviz cluster (`Cluster`).

use super::cluster_header::{default_style_query, ClusterHeader};
use super::edge::SvekEdge;
use super::node::SvekNode;
use crate::abel::{Entity, EntityPosition};
use crate::klimt::dotpath::{Point, RectangleArea};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

pub const CENTER_ID: &str = "za";

pub struct ClusterData {
    pub parent: Option<Cluster>,
    pub group: Entity,
    pub nodes: Vec<SvekNode>,
    pub children: Vec<Cluster>,
    pub color: i32,
    pub color_title: i32,
    pub color_note_top: i32,
    pub color_note_bottom: i32,
    pub rectangle_area: Option<RectangleArea>,
    pub xy_title: Option<Point>,
    pub xy_note_top: Option<Point>,
    pub xy_note_bottom: Option<Point>,
    /// Width and height of the title and attributes (`ClusterHeader`).
    pub title_and_attribute: (f64, f64),
    pub header: Option<Rc<ClusterHeader>>,
    pub hidden: bool,
}

#[derive(Clone)]
pub struct Cluster(Rc<RefCell<ClusterData>>);

impl PartialEq for Cluster {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Cluster {
    pub fn new_root(group: Entity, colors: [i32; 4]) -> Cluster {
        Cluster(Rc::new(RefCell::new(ClusterData {
            parent: None,
            group,
            nodes: Vec::new(),
            children: Vec::new(),
            color: colors[0],
            color_title: colors[1],
            color_note_top: colors[2],
            color_note_bottom: colors[3],
            rectangle_area: None,
            xy_title: None,
            xy_note_top: None,
            xy_note_bottom: None,
            title_and_attribute: (0.0, 0.0),
            header: None,
            hidden: false,
        })))
    }

    pub fn create_child(&self, group: Entity, colors: [i32; 4], header: ClusterHeader) -> Cluster {
        let child = Cluster::new_root(group, colors);
        {
            let mut c = child.get_mut();
            c.parent = Some(self.clone());
            c.title_and_attribute = (f64::from(header.width), f64::from(header.height));
            c.header = Some(Rc::new(header));
        }
        self.get_mut().children.push(child.clone());
        child
    }

    pub fn get(&self) -> Ref<'_, ClusterData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, ClusterData> {
        self.0.borrow_mut()
    }

    pub fn group(&self) -> Entity {
        self.get().group.clone()
    }

    pub fn groups(&self) -> Vec<Entity> {
        vec![self.group()]
    }

    pub fn add_node(&self, node: SvekNode) {
        self.get_mut().nodes.push(node);
    }

    pub fn nodes(&self) -> Vec<SvekNode> {
        self.get().nodes.clone()
    }

    pub fn children(&self) -> Vec<Cluster> {
        self.get().children.clone()
    }

    pub fn parent(&self) -> Option<Cluster> {
        self.get().parent.clone()
    }

    pub fn rectangle_area(&self) -> Option<RectangleArea> {
        self.get().rectangle_area
    }

    pub fn set_position(&self, min: Point, max: Point) {
        self.get_mut().rectangle_area = Some(RectangleArea::build(min, max));
    }

    pub fn set_title_position(&self, p: Point) {
        self.get_mut().xy_title = Some(p);
    }

    pub fn title_and_attribute_width(&self) -> f64 {
        self.get().title_and_attribute.0
    }

    pub fn title_and_attribute_height(&self) -> f64 {
        self.get().title_and_attribute.1
    }

    pub fn cluster_id(&self) -> String {
        format!("cluster{}", self.get().color)
    }

    pub fn special_point_id(group: &Entity) -> String {
        format!("{CENTER_ID}{}", group.uid())
    }

    pub fn move_delta(&self, dx: f64, dy: f64) {
        let mut c = self.get_mut();
        let d = &mut *c;
        for p in [&mut d.xy_note_top, &mut d.xy_note_bottom, &mut d.xy_title]
            .into_iter()
            .flatten()
        {
            p.0 += dx;
            p.1 += dy;
        }
        if let Some(r) = c.rectangle_area {
            c.rectangle_area = Some(r.move_by(dx, dy));
        }
    }

    fn is_normal_position(n: &SvekNode) -> bool {
        n.get().entity_position == EntityPosition::Normal
    }

    /// Nodes that start an inverted edge come first (`getNodesOrderedTop`).
    fn nodes_ordered_top(&self, lines: &[SvekEdge]) -> Vec<SvekNode> {
        let nodes = self.nodes();
        let mut firsts: Vec<SvekNode> = Vec::new();
        for l in lines {
            if l.is_inverted() {
                let prefix = l.start_uid_prefix();
                if let Some(sh) = nodes.iter().find(|n| n.uid() == prefix) {
                    if Self::is_normal_position(sh) {
                        firsts.insert(0, sh.clone());
                    }
                }
            }
        }
        firsts
    }

    fn nodes_ordered_without_top(&self, lines: &[SvekEdge]) -> Vec<SvekNode> {
        let mut all: Vec<SvekNode> = self
            .nodes()
            .into_iter()
            .filter(Self::is_normal_position)
            .collect();
        for l in lines {
            if l.is_inverted() {
                let prefix = l.start_uid_prefix();
                if let Some(pos) = all.iter().position(|n| n.uid() == prefix) {
                    all.remove(pos);
                }
            }
        }
        all
    }

    /// First part of the DOT body: the nodes that must come first (`printCluster1`).
    pub fn print_cluster1(&self, sb: &mut String, lines: &[SvekEdge]) -> bool {
        let tmp = self.nodes_ordered_top(lines);
        if tmp.is_empty() {
            return false;
        }
        for n in tmp {
            n.append_shape(sb);
        }
        true
    }

    /// Second part: the other nodes and the sub clusters (`printCluster2`).
    pub fn print_cluster2(&self, sb: &mut String, lines: &[SvekEdge], use_rank_same: bool) {
        for node in self.nodes_ordered_without_top(lines) {
            node.append_shape(sb);
        }
        if use_rank_same {
            for same in self.rank_same(lines) {
                sb.push_str(&same);
                sb.push('\n');
            }
        }
        for child in self.children() {
            child.print_internal(sb, lines);
        }
    }

    fn rank_same(&self, lines: &[SvekEdge]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let nodes = self.nodes();
        let in_cluster = |uid: &str| nodes.iter().any(|n| n.uid() == uid);
        for l in lines {
            if l.has_entry_point() {
                continue;
            }
            if in_cluster(&l.start_uid_prefix()) && in_cluster(&l.end_uid_prefix()) {
                if let Some(same) = l.rank_same() {
                    if !out.contains(&same) {
                        out.push(same);
                    }
                }
            }
        }
        out
    }

    fn print_internal(&self, _sb: &mut String, _lines: &[SvekEdge]) {
        // Clusters of packages are added with `ClusterDotString`.
    }
}

impl Cluster {
    /// Draws the package (`Cluster.drawU`).
    pub fn draw(
        &self,
        ug: &crate::klimt::drawing::ugraphic::UGraphic,
        skin: &crate::skin::SkinParam,
        hidden: bool,
    ) {
        use crate::klimt::color::colors::ColorType;
        use crate::klimt::color::HColor;
        use crate::klimt::drawing::ugraphic::{UShape, UTranslate};
        use crate::klimt::fashion::Fashion;
        use crate::klimt::geom::HorizontalAlignment;
        use crate::style::PName;
        let group = self.group();
        if hidden {
            return;
        }
        let (Some(header), Some(area)) = (self.get().header.clone(), self.rectangle_area()) else {
            return;
        };
        let full_name = group.name();
        if !full_name.starts_with("##") {
            ug.draw(UShape::Comment(format!("cluster {full_name}")));
        }
        let diagram_style = skin.diagram_style_name();
        let symbol = group
            .usymbol()
            .unwrap_or_else(|| crate::decoration::usymbol::usymbols::package());
        let query = default_style_query(diagram_style, Some(&symbol), group.group_type())
            .with_stereotype_obj(group.stereotype().as_ref());
        let style = skin
            .style_builder()
            .merged_style(&query)
            .expect("group style");
        let shadowing = style.shadowing();
        let colors = group.colors();
        let border = colors
            .color(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let mut rounded = style.value(PName::RoundCorner).as_double();
        if skin.strict_uml_style() {
            rounded = 0.0;
        }
        let diagonal = style.value(PName::DiagonalCorner).as_double();
        let mut attrs: Vec<(&str, String)> = vec![("class", "cluster".to_string())];
        attrs.push((
            "data-qualified-name",
            crate::skin::rose::line::fix_group_value(&group.qualified_name()),
        ));
        if let Some(l) = group.location() {
            attrs.push(("data-source-line", l.position.to_string()));
        }
        attrs.push(("id", group.uid()));
        ug.start_group(&attrs);
        let stroke = colors
            .specific_line_stroke()
            .unwrap_or_else(|| style.stroke());
        let back = colors
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        let back = if back.is_transparent() {
            HColor::TRANSPARENT
        } else {
            back
        };
        if skin.is_state_diagram() && group.usymbol().is_none() {
            draw_state_cluster(
                ug, &group, &header, area, skin, border, stroke, rounded, shadowing,
            );
            ug.close_group();
            return;
        }
        let package_style = skin.package_style();
        let symbol = group
            .usymbol()
            .or_else(|| package_style.to_usymbol())
            .unwrap_or(symbol);
        let fashion = Fashion::new(Some(back), Some(border))
            .with_delta_shadow(shadowing)
            .with_stroke(stroke)
            .with_corner(rounded, diagonal);
        let big = symbol.as_big(
            header.title.clone(),
            header.title_alignment,
            header.stereo.clone(),
            area.width(),
            area.height(),
            fashion,
            HorizontalAlignment::Center,
        );
        big.draw(&ug.apply_translate(UTranslate::new(area.min_x, area.min_y)));
        ug.close_group();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_state_cluster(
    ug: &crate::klimt::drawing::ugraphic::UGraphic,
    group: &Entity,
    header: &ClusterHeader,
    area: crate::klimt::dotpath::RectangleArea,
    skin: &crate::skin::SkinParam,
    border: crate::klimt::color::HColor,
    stroke: crate::klimt::drawing::ugraphic::UStroke,
    rounded: f64,
    shadowing: f64,
) {
    use crate::klimt::color::colors::ColorType;
    use crate::klimt::drawing::ugraphic::UTranslate;
    use crate::style::{queries, PName, SName};

    let description_style = skin
        .style_builder()
        .merged_style(
            &queries::STATEDIAG_STATE
                .clone()
                .add(SName::Description)
                .with_stereotype_obj(group.stereotype().as_ref()),
        )
        .expect("state description style");
    let name_style = skin
        .style_builder()
        .merged_style(
            &queries::STATEDIAG_STATE
                .clone()
                .add(SName::Name)
                .with_stereotype_obj(group.stereotype().as_ref()),
        )
        .expect("state name style");
    let attribute = header.attribute.clone();
    let sb = ug.string_bounder();
    let title = header.title.calculate_dimension(sb);
    let attribute_dim = attribute.calculate_dimension(sb);
    let name_height = title.height + super::image::MARGIN;
    let description_height = if attribute_dim.height > 0.0 {
        attribute_dim.height + super::image::MARGIN
    } else {
        0.0
    };
    let name_back = group
        .colors()
        .color(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| name_style.value(PName::BackGroundColor).as_color());
    let description_back = group
        .colors()
        .color(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| description_style.value(PName::BackGroundColor).as_color());
    let body_style = skin
        .style_builder()
        .merged_style(
            &queries::STATEDIAG_STATE
                .clone()
                .add(SName::Body)
                .with_stereotype_obj(group.stereotype().as_ref()),
        )
        .expect("state body style");
    let body_back = group
        .colors()
        .color(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| body_style.value(PName::BackGroundColor).as_color());
    let container = crate::svek::rounded_container::RoundedContainer::new(
        border.clone(),
        area.width(),
        area.height(),
        name_height,
        description_height,
        name_back,
        description_back,
        body_back,
        stroke,
        rounded,
        shadowing,
    );
    container.draw(&ug.apply_translate(UTranslate::new(area.min_x, area.min_y)));
    let title_x = (area.width() - title.width) / 2.0;
    header
        .title
        .draw(&ug.apply_translate(UTranslate::new(area.min_x + title_x, area.min_y)));
    if attribute_dim.height > 0.0 {
        attribute.draw(&ug.apply_translate(UTranslate::new(
            area.min_x + super::image::MARGIN,
            area.min_y + name_height + super::image::MARGIN / 2.0,
        )));
    }
}
