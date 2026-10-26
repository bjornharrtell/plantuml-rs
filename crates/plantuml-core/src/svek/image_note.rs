//! A note of a class diagram, with the pointer to its entity (`EntityImageNote`, `Opale`).

use super::image::IEntityImage;
use super::types::ShapeType;
use crate::abel::Entity;
use crate::cucadiagram::CucaDiagram;
use crate::klimt::color::colors::ColorType;
use crate::klimt::color::HColor;
use crate::klimt::creole::body::BodyEnhanced;
use crate::klimt::creole::display::Display;
use crate::klimt::dotpath::{distance, Point};
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{Direction, HorizontalAlignment};
use crate::klimt::shape::{EmptyBlock, RectangleStencil, TextBlock};
use crate::skin::rose::note::{corner, polygon_normal};
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

const MARGIN_X1: f64 = 6.0;
const MARGIN_X2: f64 = 15.0;
const MARGIN_Y: f64 = 5.0;
const CORNER_SIZE: f64 = 10.0;
/// Half width of the base of the pointer (`Opale.delta`).
const DELTA: f64 = 4.0;

fn limitation(v: f64, min: f64, max: f64) -> f64 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

/// Note outline with a pointer towards `tip` (`Opale.getPolygonLeft`, `Right`, `Up`, `Down`).
///
/// `base` is where the link leaves the note, `tip` where it reaches the other entity.
pub fn polygon_with_pointer(
    strategy: Direction,
    base: Point,
    tip: Point,
    width: f64,
    height: f64,
    round: f64,
) -> UPath {
    let r = round / 2.0;
    let mut p = UPath::new();
    let arc = |p: &mut UPath, x: f64, y: f64| p.arc_to(r, r, 0.0, false, false, x, y);
    p.move_to(0.0, r);
    match strategy {
        Direction::Left => {
            let y1 = limitation(base.1 - DELTA, 0.0, height - 2.0 * DELTA);
            p.line_to(0.0, y1);
            p.line_to(tip.0, tip.1);
            p.line_to(0.0, y1 + 2.0 * DELTA);
            p.line_to(0.0, height - r);
            arc(&mut p, r, height);
            p.line_to(width - r, height);
            arc(&mut p, width, height - r);
            p.line_to(width, CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, 0.0);
            p.line_to(r, 0.0);
        }
        Direction::Right => {
            p.line_to(0.0, height - r);
            arc(&mut p, r, height);
            p.line_to(width - r, height);
            arc(&mut p, width, height - r);
            let y1 = limitation(base.1 - DELTA, CORNER_SIZE, height - 2.0 * DELTA);
            p.line_to(width, y1 + 2.0 * DELTA);
            p.line_to(tip.0, tip.1);
            p.line_to(width, y1);
            p.line_to(width, CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, 0.0);
            p.line_to(r, 0.0);
        }
        Direction::Up => {
            p.line_to(0.0, height - r);
            arc(&mut p, r, height);
            p.line_to(width - r, height);
            arc(&mut p, width, height - r);
            p.line_to(width, CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, 0.0);
            let x1 = limitation(base.0 - DELTA, 0.0, width - CORNER_SIZE);
            p.line_to(x1 + 2.0 * DELTA, 0.0);
            p.line_to(tip.0, tip.1);
            p.line_to(x1, 0.0);
            p.line_to(r, 0.0);
        }
        Direction::Down => {
            p.line_to(0.0, height - r);
            arc(&mut p, r, height);
            let x1 = limitation(base.0 - DELTA, 0.0, width);
            p.line_to(x1, height);
            p.line_to(tip.0, tip.1);
            p.line_to(x1 + 2.0 * DELTA, height);
            p.line_to(width - r, height);
            arc(&mut p, width, height - r);
            p.line_to(width, CORNER_SIZE);
            p.line_to(width - CORNER_SIZE, 0.0);
            p.line_to(r, 0.0);
        }
    }
    arc(&mut p, 0.0, r);
    p.close_path();
    p
}

/// Side of the box nearest to `pt` (`getOpaleStrategy`).
pub fn opale_strategy(width: f64, height: f64, pt: Point) -> Direction {
    let d_right = (width - pt.0).abs();
    let d_bottom = (height - pt.1).abs();
    let d_left = pt.0.abs();
    let d_top = pt.1.abs();
    if d_left <= d_right && d_left <= d_bottom && d_left <= d_top {
        Direction::Left
    } else if d_right <= d_bottom && d_right <= d_left && d_right <= d_top {
        Direction::Right
    } else if d_top <= d_right && d_top <= d_bottom && d_top <= d_left {
        Direction::Up
    } else {
        Direction::Down
    }
}

/// Where the link meets the note and the other entity, relative to the note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpalePath {
    pub start: Point,
    pub end: Point,
}

/// The note box with its optional pointer (`Opale`).
pub struct Opale {
    pub shadowing: f64,
    pub border: HColor,
    pub back: HColor,
    pub text: Rc<dyn TextBlock>,
    pub stroke: UStroke,
    pub round_corner: f64,
}

impl Opale {
    pub fn width(&self, sb: &dyn StringBounder) -> f64 {
        self.text.calculate_dimension(sb).width + MARGIN_X1 + MARGIN_X2
    }

    pub fn height(&self, sb: &dyn StringBounder) -> f64 {
        self.text.calculate_dimension(sb).height + 2.0 * MARGIN_Y
    }

    pub fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(self.width(sb), self.height(sb))
    }

    /// Draws the note; `pointer` is the side, the base and the tip of the pointer.
    pub fn draw(&self, ug: &UGraphic, pointer: Option<(Direction, Point, Point)>) {
        let sb = ug.string_bounder();
        let (width, height) = (self.width(sb), self.height(sb));
        let mut polygon = match pointer {
            None => polygon_normal(width, height, self.round_corner),
            Some((strategy, base, tip)) => {
                polygon_with_pointer(strategy, base, tip, width, height, self.round_corner)
            }
        };
        polygon.delta_shadow = self.shadowing;
        let ug = ug
            .apply_back_color(self.back.clone())
            .apply_color(self.border.clone())
            .apply_stroke(self.stroke);
        ug.draw(polygon);
        ug.draw(corner(width, self.round_corner));
        let stencil = ug.apply_stencil(Rc::new(RectangleStencil(width)), UStroke::simple());
        self.text
            .draw(&stencil.apply_translate(UTranslate::new(MARGIN_X1, MARGIN_Y)));
    }
}

/// Text of a note, split on separator lines (`BodyFactory.create3`).
pub fn note_body(
    display: Display,
    skin: &SkinParam,
    style: &Style,
    align: HorizontalAlignment,
) -> Rc<dyn TextBlock> {
    if display.size() == 1 && display.text_at(0).is_empty() {
        return Rc::new(EmptyBlock {
            width: 0.0,
            height: 0.0,
        });
    }
    Rc::new(BodyEnhanced::new(
        display,
        skin.skin_simple(),
        align,
        style.font_configuration(None),
        style.wrap_width(),
        style.value(PName::MinimumWidth).as_double(),
        style.value(PName::LineThickness).as_double(),
    ))
}

pub struct EntityImageNote {
    entity: Entity,
    opale: Opale,
    hidden: bool,
    link: Cell<Option<OpalePath>>,
}

impl EntityImageNote {
    pub fn new(entity: &Entity, cuca: &CucaDiagram, skin: &SkinParam) -> Self {
        let q = queries::NOTE
            .add(super::edge::skin_style_name(skin))
            .with_stereotype_obj(entity.stereotype().as_ref());
        let style = skin.style_builder().merged_style(&q).expect("note style");
        let colors = entity.colors();
        let back = colors
            .color(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        Self {
            entity: entity.clone(),
            opale: Opale {
                shadowing: style.shadowing(),
                border: style.value(PName::LineColor).as_color(),
                back,
                text: note_body(entity.display(), skin, &style, style.horizontal_alignment()),
                stroke: colors.mute_stroke(style.stroke()),
                round_corner: f64::from(skin.as_int("roundcorner", 0)),
            },
            hidden: cuca.is_entity_hidden(entity),
            link: Cell::new(None),
        }
    }

    pub fn entity(&self) -> &Entity {
        &self.entity
    }

    /// Tells where the link of the note goes (`setOpaleLink`, `setOpaleLine`).
    pub fn set_opale_path(&self, path: Option<OpalePath>) {
        self.link.set(path);
    }

    fn draw_internal(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let (width, height) = (self.opale.width(sb), self.opale.height(sb));
        let pointer = self.link.get().map(|OpalePath { start, end }| {
            let center = (width / 2.0, height / 2.0);
            // The end of the link nearest to the note is where the pointer leaves it.
            let (near, far) = if distance(start, center) < distance(end, center) {
                (start, end)
            } else {
                (end, start)
            };
            (opale_strategy(width, height, near), near, far)
        });
        self.opale.draw(ug, pointer);
    }
}

impl TextBlock for EntityImageNote {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.opale.dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        let mut attrs: Vec<(&str, String)> = vec![("class", "entity".to_string())];
        attrs.push(("data-qualified-name", self.entity.qualified_name()));
        if let Some(l) = self.entity.location() {
            attrs.push(("data-source-line", l.position.to_string()));
        }
        attrs.push(("id", self.entity.uid()));
        ug.start_group(&attrs);
        self.draw_internal(ug);
        ug.close_group();
    }
}

impl IEntityImage for EntityImageNote {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_picks_nearest_side() {
        assert_eq!(opale_strategy(100.0, 50.0, (0.0, 20.0)), Direction::Left);
        assert_eq!(opale_strategy(100.0, 50.0, (100.0, 20.0)), Direction::Right);
        assert_eq!(opale_strategy(100.0, 50.0, (40.0, 0.0)), Direction::Up);
        assert_eq!(opale_strategy(100.0, 50.0, (40.0, 50.0)), Direction::Down);
    }

    #[test]
    fn pointer_polygon_reaches_the_tip() {
        for (dir, base) in [
            (Direction::Left, (0.0, 20.0)),
            (Direction::Right, (100.0, 20.0)),
            (Direction::Up, (40.0, 0.0)),
            (Direction::Down, (40.0, 50.0)),
        ] {
            for round in [0.0, 10.0] {
                let p = polygon_with_pointer(dir, base, (-30.0, 90.0), 100.0, 50.0, round);
                assert!(p
                    .segments
                    .iter()
                    .any(|s| matches!(s, crate::klimt::drawing::svg::PathSeg::LineTo(x, y) if *x == -30.0 && *y == 90.0)));
            }
        }
    }
}
