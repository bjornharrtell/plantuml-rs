//! Symbols that reshape the horizontal lines of their text: database, queue and node.

use super::common::*;
use super::USymbol;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    HLineDrawer, UGraphic, ULine, UPath, UPolygon, UShape, UStroke, UTranslate,
};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{Stencil, TextBlock, UHorizontalLine};
use crate::style::SName;
use std::rc::Rc;

// ---- database -----------------------------------------------------------------------------

fn database_closing_path(width: f64) -> UPath {
    let mut closing = UPath::new();
    closing.move_to(0.0, 10.0);
    closing.cubic_to(0.0, 20.0, width / 2.0, 20.0, width / 2.0, 20.0);
    closing.cubic_to(width / 2.0, 20.0, width, 20.0, width, 10.0);
    closing
}

fn draw_database(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let mut shape = UPath::new();
    shape.delta_shadow = fashion.delta_shadow();
    shape.move_to(0.0, 10.0);
    shape.cubic_to(0.0, 0.0, width / 2.0, 0.0, width / 2.0, 0.0);
    shape.cubic_to(width / 2.0, 0.0, width, 0.0, width, 10.0);
    shape.line_to(width, height - 10.0);
    shape.cubic_to(width, height, width / 2.0, height, width / 2.0, height);
    shape.cubic_to(width / 2.0, height, 0.0, height, 0.0, height - 10.0);
    shape.line_to(0.0, 10.0);
    ug.draw(shape);
    ug.apply_back_color(HColor::TRANSPARENT)
        .draw(database_closing_path(width));
    ug.apply_translate(UTranslate::new(width, height))
        .draw(UShape::Empty(10.0, 10.0));
}

/// Lines of a database follow the curved bottom of the symbol (`MyUGraphicDatabase`).
struct DatabaseHLine {
    ending_x: f64,
}

impl HLineDrawer for DatabaseHLine {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, translate: UTranslate) {
        let closing = database_closing_path(self.ending_x);
        let ug = ug.apply_translate(translate);
        let styled = ug
            .apply_stroke(line.stroke())
            .apply_back_color(HColor::TRANSPARENT);
        styled
            .apply_translate(UTranslate::dy(-15.0))
            .draw(closing.clone());
        if line.is_double() {
            styled
                .apply_translate(UTranslate::dy(-15.0 + 2.0))
                .draw(closing);
        }
        line.draw_title_internal(&ug, 0.0, self.ending_x, 0.0, true);
    }
}

fn wrap_database(ug: &UGraphic, width: f64, _height: f64) -> UGraphic {
    ug.apply_hline_drawer(Rc::new(DatabaseHLine { ending_x: width }))
}

fn database_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_database(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(
            &ug,
            &a.title,
            &a.stereotype,
            a.width,
            2.0 + 20.0,
            2.0 + 20.0,
        );
    })
}

/// `USymbolDatabase`.
pub fn database() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Database],
        spec: SmallSpec {
            margin: Margin::new(10.0, 10.0, 24.0, 5.0),
            stencil: false,
            alignment: Some(HorizontalAlignment::Center),
            shape: draw_database,
            wrap: Some(wrap_database),
            text_shift: (0.0, 0.0),
        },
        big: database_big,
        supp_height: 15,
        supp_width: 0,
    })
}

// ---- queue --------------------------------------------------------------------------------

const QUEUE_DX: f64 = 5.0;

fn draw_queue(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let dx = QUEUE_DX;
    let mut shape = UPath::new();
    shape.delta_shadow = fashion.delta_shadow();
    shape.move_to(dx, 0.0);
    shape.line_to(width - dx, 0.0);
    shape.cubic_to(width, 0.0, width, height / 2.0, width, height / 2.0);
    shape.cubic_to(width, height / 2.0, width, height, width - dx, height);
    shape.line_to(dx, height);
    shape.cubic_to(0.0, height, 0.0, height / 2.0, 0.0, height / 2.0);
    shape.cubic_to(0.0, height / 2.0, 0.0, 0.0, dx, 0.0);
    ug.draw(shape);
    let mut closing = UPath::new();
    closing.move_to(width - dx, 0.0);
    closing.cubic_to(
        width - dx * 2.0,
        0.0,
        width - dx * 2.0,
        height / 2.0,
        width - dx * 2.0,
        height / 2.0,
    );
    closing.cubic_to(
        width - dx * 2.0,
        height,
        width - dx,
        height,
        width - dx,
        height,
    );
    ug.apply_back_color(HColor::TRANSPARENT).draw(closing);
}

/// Horizontal lines follow the rounded right end of the queue (`MyUGraphicQueue`).
struct QueueStencil {
    x1: f64,
    x2: f64,
    full_height: f64,
}

impl Stencil for QueueStencil {
    fn starting_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        0.0
    }

    fn ending_x(&self, _: &dyn StringBounder, y: f64) -> f64 {
        let half = self.full_height / 2.0;
        let factor = if y <= half {
            1.0 - y / half
        } else {
            (y - half) / half
        };
        (self.x2 - self.x1) * factor + self.x1
    }
}

fn wrap_queue(ug: &UGraphic, width: f64, height: f64) -> UGraphic {
    let stencil = QueueStencil {
        x1: width - 2.0 * QUEUE_DX,
        x2: width - QUEUE_DX,
        full_height: height,
    };
    ug.apply_stencil(Rc::new(stencil), UStroke::simple())
}

fn queue_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_queue(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 2.0, 2.0);
    })
}

/// `USymbolQueue`.
pub fn queue() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Queue],
        spec: SmallSpec {
            margin: Margin::new(5.0, 15.0, 5.0, 5.0),
            stencil: false,
            alignment: Some(HorizontalAlignment::Center),
            shape: draw_queue,
            wrap: Some(wrap_queue),
            text_shift: (0.0, 0.0),
        },
        big: queue_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- node ---------------------------------------------------------------------------------

const NODE_MARGIN: Margin = Margin::new(15.0, 25.0, 20.0, 10.0);

fn draw_node(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let mut shape = UPolygon::default();
    shape.add_point(0.0, 10.0);
    shape.add_point(10.0, 0.0);
    shape.add_point(width, 0.0);
    shape.add_point(width, height - 10.0);
    shape.add_point(width - 10.0, height);
    shape.add_point(0.0, height);
    shape.add_point(0.0, 10.0);
    shape.delta_shadow = fashion.delta_shadow();
    ug.draw(shape);
    ug.apply_translate(UTranslate::new(width - 10.0, 10.0))
        .draw(ULine::new(10.0, -10.0));
    ug.apply_translate(UTranslate::dy(10.0))
        .draw(ULine::hline(width - 10.0));
    ug.apply_translate(UTranslate::new(width - 10.0, 10.0))
        .draw(ULine::vline(height - 10.0));
    ug.apply_translate(UTranslate::new(0.0, height))
        .draw(UShape::Empty(10.0, 10.0));
}

/// Lines of a node end with the diagonal of the 3D side (`MyUGraphicNode`).
struct NodeHLine {
    ending_x: f64,
}

impl NodeHLine {
    fn draw_internal(&self, ug: &UGraphic, line: &UHorizontalLine) {
        let ug = ug
            .apply_stroke(line.stroke())
            .apply_back_color(HColor::TRANSPARENT);
        ug.draw(ULine::hline(self.ending_x - 10.0));
        ug.apply_translate(UTranslate::dx(self.ending_x - 10.0))
            .draw(ULine::new(10.0, -10.0));
    }
}

impl HLineDrawer for NodeHLine {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, translate: UTranslate) {
        let ug = ug.apply_translate(translate);
        self.draw_internal(&ug, line);
        if line.is_double() {
            self.draw_internal(&ug.apply_translate(UTranslate::dy(2.0)), line);
        }
        line.draw_title_internal(&ug, 0.0, self.ending_x - 10.0, 0.0, true);
    }
}

fn wrap_node(ug: &UGraphic, width: f64, _height: f64) -> UGraphic {
    ug.apply_hline_drawer(Rc::new(NodeHLine { ending_x: width }))
}

fn node_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_node(&ug, a.width, a.height, &a.fashion);
        let ug = ug.apply_translate(UTranslate::new(-4.0, 11.0));
        let dim_stereo = a.stereotype.calculate_dimension(ug.string_bounder());
        let pos_x = if a.stereo_alignment == HorizontalAlignment::Right {
            a.width - dim_stereo.width - NODE_MARGIN.x1
        } else {
            (a.width - dim_stereo.width) / 2.0
        };
        a.stereotype
            .draw(&ug.apply_translate(UTranslate::new(pos_x, 2.0)));
        let dim_title = a.title.calculate_dimension(ug.string_bounder());
        let pos_title = (a.width - dim_title.width) / 2.0;
        a.title
            .draw(&ug.apply_translate(UTranslate::new(pos_title, 2.0 + dim_stereo.height)));
    })
}

/// `USymbolNode`.
pub fn node() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Node],
        spec: SmallSpec {
            margin: NODE_MARGIN,
            stencil: false,
            alignment: None,
            shape: draw_node,
            wrap: Some(wrap_node),
            text_shift: (0.0, 0.0),
        },
        big: node_big,
        supp_height: 5,
        supp_width: 60,
    })
}
