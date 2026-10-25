//! A node given to Graphviz (`SvekNode`).

use super::image::IEntityImage;
use super::types::{hex_rgb, pixel_to_inches, ColorSequence, ShapeType};
use crate::abel::{Entity, EntityPosition, Together};
use crate::klimt::dotpath::{Point, RectangleArea};
use crate::klimt::font::{Dimension, StringBounder};
use crate::style::Margins;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

pub struct NodeData {
    pub shape_type: ShapeType,
    pub dim_image: Option<Dimension>,
    pub uid: String,
    pub color: i32,
    pub min_x: f64,
    pub min_y: f64,
    pub shield: Option<Margins>,
    pub entity_position: EntityPosition,
    pub image: Rc<dyn IEntityImage>,
    pub sb: Rc<dyn StringBounder>,
    pub cluster: Option<usize>,
    pub entity: Entity,
    pub polygon: Option<Vec<Point>>,
    /// Whether a link with a `[qualifier]` ends here (`hasKal`).
    pub has_kal: bool,
}

#[derive(Clone)]
pub struct SvekNode(Rc<RefCell<NodeData>>);

impl PartialEq for SvekNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl SvekNode {
    pub fn new(
        entity: Entity,
        image: Rc<dyn IEntityImage>,
        colors: &mut ColorSequence,
        sb: Rc<dyn StringBounder>,
        has_kal: bool,
    ) -> SvekNode {
        let color = colors.get_value();
        SvekNode(Rc::new(RefCell::new(NodeData {
            shape_type: image.shape_type(),
            dim_image: None,
            uid: format!("sh{color:04}"),
            color,
            min_x: 0.0,
            min_y: 0.0,
            shield: None,
            entity_position: entity.entity_position(),
            image,
            sb,
            cluster: None,
            entity,
            polygon: None,
            has_kal,
        })))
    }

    pub fn get(&self) -> Ref<'_, NodeData> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, NodeData> {
        self.0.borrow_mut()
    }

    pub fn uid(&self) -> String {
        self.get().uid.clone()
    }

    pub fn entity(&self) -> Entity {
        self.get().entity.clone()
    }

    pub fn shape_type(&self) -> ShapeType {
        self.get().shape_type
    }

    pub fn image(&self) -> Rc<dyn IEntityImage> {
        self.get().image.clone()
    }

    pub fn together(&self) -> Option<Rc<Together>> {
        self.get().entity.together()
    }

    fn dim(&self) -> Dimension {
        if let Some(d) = self.get().dim_image {
            return d;
        }
        let d = {
            let n = self.get();
            n.image.calculate_dimension(&*n.sb)
        };
        self.get_mut().dim_image = Some(d);
        d
    }

    pub fn width(&self) -> f64 {
        self.dim().width
    }

    pub fn height(&self) -> f64 {
        self.dim().height
    }

    pub fn min_x(&self) -> f64 {
        self.get().min_x
    }

    pub fn min_y(&self) -> f64 {
        self.get().min_y
    }

    pub fn rectangle_area(&self) -> RectangleArea {
        RectangleArea::new(
            self.min_x(),
            self.min_y(),
            self.min_x() + self.width(),
            self.min_y() + self.height(),
        )
    }

    pub fn move_delta(&self, dx: f64, dy: f64) {
        let mut n = self.get_mut();
        n.min_x += dx;
        n.min_y += dy;
    }

    pub fn set_polygon(&self, min_x: f64, min_y: f64, points: &[Point]) {
        self.get_mut().polygon = Some(points.iter().map(|p| (p.0 - min_x, p.1 - min_y)).collect());
    }

    pub fn is_hidden(&self) -> bool {
        self.get().image.is_hidden()
    }

    fn shield(&self) -> Margins {
        if let Some(s) = self.get().shield {
            return s;
        }
        let s = {
            let n = self.get();
            n.image.shield(&*n.sb)
        };
        self.get_mut().shield = Some(s);
        s
    }

    pub fn is_shielded(&self) -> bool {
        let kal = self.get().has_kal;
        kal || !self.shield().is_zero()
    }

    pub fn point_2d(&self, x: f64, y: f64) -> Point {
        (self.min_x() + x, self.min_y() + y)
    }

    pub fn overscan_x(&self) -> f64 {
        let n = self.get();
        n.image.overscan_x(&*n.sb)
    }

    /// The node as written in the DOT file (`appendShape`).
    pub fn append_shape(&self, sb: &mut String) {
        let ty = self.shape_type();
        match ty {
            ShapeType::RectangleHtmlForPorts | ShapeType::RectanglePort => {
                // Ports are not supported yet: a plain rectangle stands for them.
            }
            ShapeType::RectangleWithCircleInside => {
                self.append_html(sb);
                sb.push('\n');
                return;
            }
            ShapeType::Rectangle if self.is_shielded() => {
                self.append_html(sb);
                sb.push('\n');
                return;
            }
            _ => {}
        }
        let uid = self.uid();
        sb.push_str(&uid);
        sb.push_str(" [");
        sb.push_str(match ty {
            ShapeType::Rectangle
            | ShapeType::RectangleWithCircleInside
            | ShapeType::Folder
            | ShapeType::RectangleHtmlForPorts
            | ShapeType::RectanglePort
            | ShapeType::Port => "shape=rect",
            ShapeType::Octagon => "shape=octagon",
            ShapeType::Hexagon => "shape=hexagon",
            ShapeType::Diamond => "shape=diamond",
            ShapeType::Circle => "shape=circle",
            ShapeType::Oval => "shape=ellipse",
            ShapeType::RoundRectangle => "shape=rect,style=rounded",
        });
        sb.push(',');
        sb.push_str("label=\"\"");
        sb.push(',');
        let color = self.get().color;
        let (width, height) = (self.width(), self.height());
        sb.push_str(&format!("width={}", pixel_to_inches(width)));
        sb.push(',');
        sb.push_str(&format!("height={}", pixel_to_inches(height)));
        sb.push(',');
        sb.push_str(&format!("color=\"{}\"", hex_rgb(color)));
        sb.push_str("];\n");
    }

    fn append_html(&self, sb: &mut String) {
        sb.push_str(&self.uid());
        sb.push_str(" [shape=plaintext,label=<");
        let shield = self.shield();
        let td = |sb: &mut String, w: f64, h: f64| {
            sb.push_str(&format!(
                "<TD FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\"></TD>",
                java_double(w),
                java_double(h)
            ));
        };
        sb.push_str("<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"0\">");
        sb.push_str("<TR><TD></TD>");
        td(sb, 1.0, shield.top);
        sb.push_str("<TD></TD></TR>");
        sb.push_str("<TR>");
        td(sb, shield.left, 1.0);
        let color = self.get().color;
        let (width, height) = (self.width(), self.height());
        sb.push_str(&format!(
            "<TD BGCOLOR=\"{}\" FIXEDSIZE=\"TRUE\" WIDTH=\"{}\" HEIGHT=\"{}\" PORT=\"h\"></TD>",
            hex_rgb(color),
            java_double(width),
            java_double(height)
        ));
        td(sb, shield.right, 1.0);
        sb.push_str("</TR>");
        sb.push_str("<TR><TD></TD>");
        td(sb, 1.0, shield.bottom);
        sb.push_str("<TD></TD></TR>");
        sb.push_str("</TABLE>");
        sb.push_str(">];\n");
    }
}

/// A double as Java prints it (`Double.toString`) for the values met here.
pub fn java_double(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e7 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}
