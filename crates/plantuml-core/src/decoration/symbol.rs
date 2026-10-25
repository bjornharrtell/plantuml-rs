//! Database and queue symbols (`USymbolDatabase`, `USymbolQueue`), small form only.

use crate::klimt::drawing::ugraphic::{UGraphic, UPath, UShape, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{Stencil, TextBlock, VerticalBlock};
use std::rc::Rc;

/// Space around the label of a symbol (`USymbol.Margin`).
struct Margin {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Margin {
    fn add_dimension(&self, dim: Dimension) -> Dimension {
        Dimension::new(
            dim.width + self.x1 + self.x2,
            dim.height + self.y1 + self.y2,
        )
    }
}

fn merge_tb(stereotype: Rc<dyn TextBlock>, label: Rc<dyn TextBlock>) -> Rc<dyn TextBlock> {
    Rc::new(VerticalBlock {
        blocks: vec![stereotype, label],
        alignment: HorizontalAlignment::Center,
    })
}

fn draw_database(ug: &UGraphic, width: f64, height: f64, shadowing: f64) {
    let mut shape = UPath::new();
    shape.delta_shadow = shadowing;
    shape.move_to(0.0, 10.0);
    shape.cubic_to(0.0, 0.0, width / 2.0, 0.0, width / 2.0, 0.0);
    shape.cubic_to(width / 2.0, 0.0, width, 0.0, width, 10.0);
    shape.line_to(width, height - 10.0);
    shape.cubic_to(width, height, width / 2.0, height, width / 2.0, height);
    shape.cubic_to(width / 2.0, height, 0.0, height, 0.0, height - 10.0);
    shape.line_to(0.0, 10.0);
    ug.draw(shape);
    let mut closing = UPath::new();
    closing.move_to(0.0, 10.0);
    closing.cubic_to(0.0, 20.0, width / 2.0, 20.0, width / 2.0, 20.0);
    closing.cubic_to(width / 2.0, 20.0, width, 20.0, width, 10.0);
    ug.apply_back_color(crate::klimt::color::HColor::TRANSPARENT)
        .draw(closing);
    ug.apply_translate(UTranslate::new(width, height))
        .draw(UShape::Empty(10.0, 10.0));
}

struct DatabaseBlock {
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
}

impl DatabaseBlock {
    fn margin() -> Margin {
        Margin {
            x1: 10.0,
            x2: 10.0,
            y1: 24.0,
            y2: 5.0,
        }
    }
}

impl TextBlock for DatabaseBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let label = self.label.calculate_dimension(sb);
        let stereo = self.stereotype.calculate_dimension(sb);
        Self::margin().add_dimension(stereo.merge_tb(label))
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        let ug = self.fashion.apply(ug);
        draw_database(&ug, dim.width, dim.height, self.fashion.delta_shadow());
        let margin = Self::margin();
        let tb = merge_tb(self.stereotype.clone(), self.label.clone());
        tb.draw(&ug.apply_translate(UTranslate::new(margin.x1, margin.y1)));
    }
}

/// Small database symbol around `label` (`USymbolDatabase.asSmall`).
pub fn database_small(
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
) -> Rc<dyn TextBlock> {
    Rc::new(DatabaseBlock {
        label,
        stereotype,
        fashion,
    })
}

const QUEUE_DX: f64 = 5.0;

fn draw_queue(ug: &UGraphic, width: f64, height: f64, shadowing: f64) {
    let dx = QUEUE_DX;
    let mut shape = UPath::new();
    shape.delta_shadow = shadowing;
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
    ug.apply_back_color(crate::klimt::color::HColor::TRANSPARENT)
        .draw(closing);
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

struct QueueBlock {
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
}

impl QueueBlock {
    fn margin() -> Margin {
        Margin {
            x1: 5.0,
            x2: 15.0,
            y1: 5.0,
            y2: 5.0,
        }
    }
}

impl TextBlock for QueueBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let label = self.label.calculate_dimension(sb);
        let stereo = self.stereotype.calculate_dimension(sb);
        Self::margin().add_dimension(stereo.merge_tb(label))
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.calculate_dimension(ug.string_bounder());
        let ug = self.fashion.apply(ug);
        draw_queue(&ug, dim.width, dim.height, self.fashion.delta_shadow());
        let margin = Self::margin();
        let tb = merge_tb(self.stereotype.clone(), self.label.clone());
        let stencil = QueueStencil {
            x1: dim.width - 2.0 * QUEUE_DX,
            x2: dim.width - QUEUE_DX,
            full_height: dim.height,
        };
        let ug2 = ug.apply_stencil(Rc::new(stencil), UStroke::simple());
        tb.draw(&ug2.apply_translate(UTranslate::new(margin.x1, margin.y1)));
    }
}

/// Small queue symbol around `label` (`USymbolQueue.asSmall`).
pub fn queue_small(
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
) -> Rc<dyn TextBlock> {
    Rc::new(QueueBlock {
        label,
        stereotype,
        fashion,
    })
}
