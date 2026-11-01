//! A `while` loop (`FtileWhile`, `FtileDiamond`).

use super::factory::{arrow_text_block, FtileRef};
use super::snake::{Arrows, MergeStrategy, Snake};
use super::leaf::FtileEmpty;
use super::{FnConnection, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UPolygon, UShape, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const DIAMOND_HEIGHT: f64 = 24.0;
const DIAMOND_SLOPE: f64 = 12.0;
const HEXAGON_HALF_SIZE: f64 = 12.0;

/// The condition diamond (`FtileDiamond`).
pub(crate) struct Diamond {
    pub(crate) label: Rc<dyn TextBlock>,
    back: HColor,
    border: HColor,
    stroke: UStroke,
    pub(crate) height: f64,
}

impl Diamond {
    pub(crate) fn new(label: Display, skin: &SkinParam, back: Option<HColor>) -> Diamond {
        let style = skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ACTIVITY_DIAMOND)
            .expect("diamond style");
        let block = label.create_block(
            &style.font_configuration(None),
            HorizontalAlignment::Center,
            &skin.skin_simple(),
            &crate::klimt::creole::LineBreakStrategy::NONE,
            CreoleMode::SimpleLine,
            None,
            None,
            0.0,
            0.0,
        );
        let back = back.unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
        let border = style.value(PName::LineColor).as_color();
        Diamond {
            height: DIAMOND_HEIGHT,
            label: block,
            back,
            border,
            stroke: style.stroke(),
        }
    }

    pub(crate) fn back(&self) -> HColor {
        self.back.clone()
    }

    pub(crate) fn width(&self, sb: &dyn StringBounder) -> f64 {
        (self.label.calculate_dimension(sb).width + 24.0).max(48.0)
    }

    fn polygon(&self, width: f64) -> UPolygon {
        UPolygon::new(vec![
            (DIAMOND_SLOPE, 0.0),
            (width - DIAMOND_SLOPE, 0.0),
            (width, self.height / 2.0),
            (width - DIAMOND_SLOPE, self.height),
            (DIAMOND_SLOPE, self.height),
            (0.0, self.height / 2.0),
            (DIAMOND_SLOPE, 0.0),
        ])
    }

    fn styled(&self, g: &FtileGraphic, x: f64, y: f64) -> crate::klimt::drawing::ugraphic::UGraphic {
        g.ug()
            .apply_translate(UTranslate::new(x, y))
            .apply_color(self.border.clone())
            .apply_back_color(self.back.clone())
            .apply_stroke(self.stroke)
    }

    pub(crate) fn draw_polygon(&self, g: &FtileGraphic, x: f64, y: f64, width: f64) {
        self.styled(g, x, y).draw(self.polygon(width));
    }

    /// The condition, once the labels around the polygon are drawn (`FtileDiamondInside.drawU`).
    pub(crate) fn draw_label(&self, g: &FtileGraphic, x: f64, y: f64, width: f64) {
        let ug = self.styled(g, x, y);
        let dim = self.label.calculate_dimension(g.string_bounder());
        let lx = (width - dim.width) / 2.0;
        let ly = (self.height - dim.height) / 2.0;
        self.label
            .draw(&ug.apply_translate(UTranslate::new(lx, ly)));
    }
}

/// The labels of a loop: `is (yes)` under the condition and `endwhile (no)` beside it.
pub struct WhileLabels {
    pub yes: Option<Display>,
    pub out: Option<Display>,
    /// The label of the arrow back to the condition (`incoming1`) and, with a `backward`, of the one
    /// from the `backward` box (`incoming2`).
    pub back1: Display,
    pub back2: Display,
    /// The `backward :text;` box drawn on the right of the loop.
    pub backward: Option<FtileRef>,
}

/// A loop whose condition is tested before the body (`FtileWhile`).
pub struct FtileWhile {
    body: FtileRef,
    diamond: Diamond,
    yes: Option<Rc<dyn TextBlock>>,
    out: Option<Rc<dyn TextBlock>>,
    back_label: Option<Rc<dyn TextBlock>>,
    back2_label: Option<Rc<dyn TextBlock>>,
    backward: Option<FtileRef>,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
    swimlane: Option<Rc<Swimlane>>,
}

impl FtileWhile {
    pub fn new(
        body: FtileRef,
        test: Display,
        labels: WhileLabels,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
        swimlane: Option<Rc<Swimlane>>,
    ) -> Rc<Self> {
        let diamond = Diamond::new(test, &skin, back);
        let block = |d: &Option<Display>| d.as_ref().and_then(|d| arrow_text_block(&skin, d));
        let yes = block(&labels.yes);
        let out = block(&labels.out);
        let back_label = arrow_text_block(&skin, &labels.back1);
        let back2_label = arrow_text_block(&skin, &labels.back2);
        Rc::new(Self {
            body,
            diamond,
            yes,
            out,
            back_label,
            back2_label,
            backward: labels.backward,
            skin,
            in_link_rendering,
            out_link_rendering,
            swimlane,
        })
    }

    fn diamond_width(&self, sb: &dyn StringBounder) -> f64 {
        self.diamond.width(sb)
    }

    fn label_height(block: &Option<Rc<dyn TextBlock>>, sb: &dyn StringBounder) -> f64 {
        block
            .as_ref()
            .map_or(0.0, |b| b.calculate_dimension(sb).height)
    }

    /// The condition with the `is (yes)` label under it (`FtileDiamondInside.incHeight`).
    fn diamond_geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = (self.diamond_width(sb), self.diamond.height);
        FtileGeometry::with_out(w, h + Self::label_height(&self.yes, sb), w / 2.0, 0.0, h)
    }

    /// `calculateDimensionFtile`: the diamond over the body, a channel on the left for the exit and
    /// one on the right for the loop-back.
    fn geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let diamond = self.diamond_geometry(sb);
        let body = self.body.calculate_dimension(sb);
        let stacked = diamond.append_bottom(&body);
        let height =
            stacked.height + 4.0 * HEXAGON_HALF_SIZE + Self::label_height(&self.back_label, sb);
        FtileGeometry::with_out(
            stacked.width + 2.0 * HEXAGON_HALF_SIZE + HEXAGON_HALF_SIZE + self.backward_width(sb),
            height,
            stacked.left() + 2.0 * HEXAGON_HALF_SIZE,
            diamond.in_y,
            height,
        )
    }

    fn backward_width(&self, sb: &dyn StringBounder) -> f64 {
        self.backward
            .as_ref()
            .map_or(0.0, |b| b.calculate_dimension(sb).width)
    }

    fn backward_translate(&self, backward: &FtileRef, sb: &dyn StringBounder) -> UTranslate {
        let total = self.geometry(sb);
        let dim = backward.calculate_dimension(sb);
        UTranslate::new(total.width - dim.width, (total.height - dim.height) / 2.0)
    }

    fn body_translate(&self, sb: &dyn StringBounder) -> UTranslate {
        let diamond = self.diamond_geometry(sb);
        let total = self.geometry(sb);
        let body = self.body.calculate_dimension(sb);
        let supp = Self::label_height(&self.back_label, sb);
        let y = diamond.height + (total.height - diamond.height - body.height - supp) / 2.0;
        UTranslate::new(total.left() - body.left(), y)
    }

    fn diamond_translate(&self, sb: &dyn StringBounder) -> UTranslate {
        UTranslate::new(
            self.geometry(sb).left() - self.diamond_geometry(sb).left(),
            0.0,
        )
    }
}

impl Ftile for FtileWhile {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        self.geometry(sb)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if let Some(b) = &self.backward {
            let t = self.backward_translate(b, sb);
            if super::same_ftile(child, b.as_ref()) {
                return Some(t);
            }
            if let Some(c) = b.translate_for(child, sb) {
                return Some(c.compose(t));
            }
        }
        let t = self.body_translate(sb);
        if super::same_ftile(child, self.body.as_ref()) {
            return Some(t);
        }
        self.body.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn my_children(&self) -> Vec<FtileRef> {
        let mut children = vec![self.body.clone()];
        children.extend(self.backward.clone());
        children
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes = self.body.swimlanes();
        if let Some(lane) = &self.swimlane {
            if !lanes.iter().any(|l| Rc::ptr_eq(l, lane)) {
                lanes.push(lane.clone());
            }
        }
        for lane in self.backward.iter().flat_map(|b| b.swimlanes()) {
            if !lanes.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                lanes.push(lane);
            }
        }
        lanes
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn out_link_rendering(&self) -> LinkRendering {
        self.out_link_rendering.clone()
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let total = self.geometry(sb);
        let diamond_geo = self.diamond_geometry(sb);
        let body_geo = self.body.calculate_dimension(sb);
        let body_t = self.body_translate(sb);
        let diamond_t = self.diamond_translate(sb);
        let color = crate::activitydiagram3::ftile::factory::in_link_rendering_color(
            &self.skin,
            self.body.as_ref(),
        );
        let arrows = Arrows::from_skin(&self.skin);

        let skin = self.skin.clone();
        let align = self.arrow_horizontal_alignment();
        // the condition diamond stands in for its tile in the arrows
        let diamond_tile: FtileRef = FtileEmpty::new(&self.skin, 0.0, 0.0, self.swimlane.clone());
        let body_ref = self.body.clone();
        let ns = {
            let (skin, color, arrows) = (skin.clone(), color.clone(), arrows.clone());
            move |dir: Direction| Snake::create(&skin, color.clone(), Some(arrows.as_to(dir)))
        };

        // ConnectionIn: diamond to body
        let p1 = (
            diamond_t.dx + diamond_geo.left(),
            diamond_t.dy + diamond_geo.out_y,
        );
        let p2 = (body_t.dx + body_geo.left(), body_t.dy + body_geo.in_y);
        let conn = FnConnection::new(Some(diamond_tile.clone()), Some(body_ref.clone()), {
            let ns = ns.clone();
            move |g| {
                let mut down = ns(Direction::Down);
                down.add_point(p1.0, p1.1);
                down.add_point(p2.0, p2.1);
                g.draw_snake(down);
            }
        })
        .translatable({
            let ns = ns.clone();
            move |g, t1, t2| {
                let mp1 = (p1.0 + t1.dx, p1.1 + t1.dy);
                let mp2 = (p2.0 + t2.dx, p2.1 + t2.dy);
                let middle = (mp1.1 + mp2.1) / 2.0;
                let mut s = ns(Direction::Down).with_merge(MergeStrategy::Limited);
                s.add_point(mp1.0, mp1.1);
                s.add_point(mp1.0, middle);
                s.add_point(mp2.0, middle);
                s.add_point(mp2.0, mp2.1);
                g.draw_snake(s);
            }
        });
        g.draw_connection(&conn);

        // ConnectionBackBackward1 and 2: through the `backward` box on the right
        if let Some(backward) = &self.backward {
            let back_geo = backward.calculate_dimension(sb);
            let back_t = self.backward_translate(backward, sb);
            let x2 = diamond_t.dx + diamond_geo.width;
            let y2 = diamond_t.dy + diamond_geo.in_y + (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
            if body_geo.has_point_out() {
                let (x1, y1) = (body_t.dx + body_geo.left(), body_t.dy + body_geo.out_y);
                let bottom = body_t.dy + body_geo.height;
                let y1bis = y1.max(bottom) + HEXAGON_HALF_SIZE;
                let (bx, by) = (back_t.dx + back_geo.left(), back_t.dy + back_geo.out_y);
                let (ns, label) = (ns.clone(), self.back_label.clone());
                let conn = FnConnection::new(Some(body_ref.clone()), Some(backward.clone()), move |g| {
                    let mut up = ns(Direction::Up)
                        .with_vertical_label(label.clone(), VerticalAlignment::Bottom);
                    up.add_point(x1, y1);
                    up.add_point(x1, y1bis);
                    up.add_point(bx, y1bis);
                    up.add_point(bx, by);
                    g.draw_snake(up);
                    g.ug()
                        .apply_translate(UTranslate::new(x1, y1bis))
                        .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
                });
                g.draw_connection(&conn);
            }
            let (ix, iy) = (back_t.dx + back_geo.left(), back_t.dy + back_geo.in_y);
            let (ns, label) = (ns.clone(), self.back2_label.clone());
            let conn = FnConnection::new(
                Some(backward.clone()),
                Some(diamond_tile.clone()),
                move |g| {
                    let mut left = ns(Direction::Left).with_label(label.clone(), align);
                    left.add_point(ix, iy);
                    left.add_point(ix, y2);
                    left.add_point(x2, y2);
                    g.draw_snake(left);
                },
            );
            g.draw_connection(&conn);
        }

        // ConnectionBackSimple: body back to the diamond
        if self.backward.is_none() && body_geo.has_point_out() {
            let (x1, y1) = (body_t.dx + body_geo.left(), body_t.dy + body_geo.out_y);
            let x2 = diamond_t.dx + diamond_geo.width;
            let y2 = diamond_t.dy + diamond_geo.in_y + (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
            let bottom = body_t.dy + body_geo.height;
            let y1bis = y1.max(bottom) + HEXAGON_HALF_SIZE;
            let (ns1, ns2, label) = (ns.clone(), ns.clone(), self.back_label.clone());
            let total_width = total.width;
            let arrow_up = arrows.as_to(Direction::Up);
            let back_color = color.colors()[0].arrow_color.clone();
            let conn = FnConnection::new(Some(body_ref.clone()), Some(diamond_tile.clone()), move |g| {
                let mut back = ns1(Direction::Left)
                    .emphasize_direction(Direction::Up)
                    .with_vertical_label(label.clone(), VerticalAlignment::Bottom);
                back.add_point(x1, y1);
                back.add_point(x1, y1bis);
                back.add_point(total_width, y1bis);
                back.add_point(total_width, y2);
                back.add_point(x2, y2);
                g.draw_snake(back);
                g.ug()
                    .apply_translate(UTranslate::new(x1, y1bis))
                    .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
            })
            .translatable(move |g, t1, t2| {
                let q1 = (x1 + t1.dx, y1 + t1.dy);
                let ex2 = diamond_t.dx + t2.dx + diamond_geo.width;
                let ey2 = diamond_t.dy
                    + t2.dy
                    + diamond_geo.in_y
                    + (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
                let mut s = ns2(Direction::Left).with_merge(MergeStrategy::Limited);
                s.add_point(q1.0, q1.1);
                s.add_point(q1.0, q1.1 + HEXAGON_HALF_SIZE);
                let xx = t1.dx.max(t2.dx) + total_width;
                s.add_point(xx, q1.1 + HEXAGON_HALF_SIZE);
                s.add_point(xx, ey2);
                s.add_point(ex2, ey2);
                g.draw_snake(s);
                g.ug()
                    .apply_translate(UTranslate::new(q1.0, q1.1 + HEXAGON_HALF_SIZE))
                    .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
                // the arrow head, on the way up
                let up = g
                    .ug()
                    .apply_color(back_color.clone())
                    .apply_back_color(back_color.clone());
                up.apply_translate(UTranslate::new(xx, (q1.1 + ey2) / 2.0))
                    .draw(arrow_up.clone());
            });
            g.draw_connection(&conn);
        }

        // ConnectionOut: the exit channel on the left
        let half = (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
        let y1 = diamond_t.dy + diamond_geo.in_y + half;
        let (total_height, total_left) = (total.height, total.left());
        let conn = FnConnection::new(Some(diamond_tile.clone()), None, {
            let (skin, color) = (skin.clone(), color.clone());
            move |g| {
                let mut exit = Snake::create(&skin, color.clone(), None)
                    .with_merge(MergeStrategy::Limited)
                    .emphasize_direction(Direction::Down);
                exit.add_point(diamond_t.dx, y1);
                exit.add_point(HEXAGON_HALF_SIZE, y1);
                exit.add_point(HEXAGON_HALF_SIZE, total_height);
                g.draw_snake(exit);
                let mut base = Snake::create(&skin, color.clone(), None);
                base.add_point(HEXAGON_HALF_SIZE, total_height);
                base.add_point(total_left, total_height);
                g.draw_snake(base);
            }
        });
        g.draw_connection(&conn);

        // a `break` leaves through the exit channel (`FtileFactoryDelegatorWhile`)
        for point in self.body.welding_points() {
            let Some(t) = self.translate_for(point.tile.as_ref(), sb) else {
                continue;
            };
            let ns = ns.clone();
            let conn = FnConnection::new(Some(point.tile.clone()), None, move |g| {
                let mut leave = ns(Direction::Left);
                leave.add_point(t.dx, t.dy);
                leave.add_point(HEXAGON_HALF_SIZE, t.dy);
                g.draw_snake(leave);
            });
            g.draw_connection(&conn);
        }

        g.apply_translate(body_t).draw_ftile(self.body.as_ref());
        let dw = self.diamond_width(sb);
        let dh = self.diamond.height;
        let at = |x: f64, y: f64| {
            g.ug()
                .apply_translate(UTranslate::new(diamond_t.dx + x, diamond_t.dy + y))
        };
        if g.accepts(&self.swimlane) {
            // `FtileDiamondInside.drawU`: polygon, north, label, west
            self.diamond
                .draw_polygon(g, diamond_t.dx, diamond_t.dy, dw);
            if let Some(yes) = &self.yes {
                yes.draw(&at(4.0 + dw / 2.0, dh));
            }
            self.diamond.draw_label(g, diamond_t.dx, diamond_t.dy, dw);
            if let Some(out) = &self.out {
                let d = out.calculate_dimension(sb);
                out.draw(&at(-d.width, -d.height + dh / 2.0));
            }
        }
        if let Some(backward) = &self.backward {
            g.apply_translate(self.backward_translate(backward, sb))
                .draw_ftile(backward.as_ref());
        }
    }
}
