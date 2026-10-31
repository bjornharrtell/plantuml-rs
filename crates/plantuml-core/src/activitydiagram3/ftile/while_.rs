//! A `while` loop (`FtileWhile`, `FtileDiamond`).

use super::factory::{arrow_text_block, FtileRef};
use super::snake::{Arrows, MergeStrategy, Snake};
use super::{Ftile, FtileGeometry, FtileGraphic};
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
        ])
    }

    pub(crate) fn draw(&self, g: &FtileGraphic, x: f64, y: f64, width: f64) {
        let ug = g
            .ug()
            .apply_translate(UTranslate::new(x, y))
            .apply_color(self.border.clone())
            .apply_back_color(self.back.clone())
            .apply_stroke(self.stroke);
        ug.draw(self.polygon(width));
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
        self.body.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.body.swimlane_out()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes = self.body.swimlanes();
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

        // ConnectionIn: diamond to body
        let mut down = Snake::create(
            &self.skin,
            color.clone(),
            Some(arrows.as_to(Direction::Down)),
        );
        let p1 = (
            diamond_t.dx + diamond_geo.left(),
            diamond_t.dy + diamond_geo.out_y,
        );
        let p2 = (body_t.dx + body_geo.left(), body_t.dy + body_geo.in_y);
        down.add_point(p1.0, p1.1);
        down.add_point(p2.0, p2.1);
        g.draw_snake(down);

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
                let mut up =
                    Snake::create(&self.skin, color.clone(), Some(arrows.as_to(Direction::Up)))
                        .with_vertical_label(self.back_label.clone(), VerticalAlignment::Bottom);
                up.add_point(x1, y1);
                up.add_point(x1, y1bis);
                up.add_point(bx, y1bis);
                up.add_point(bx, by);
                g.draw_snake(up);
                g.ug()
                    .apply_translate(UTranslate::new(x1, y1bis))
                    .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
            }
            let (ix, iy) = (back_t.dx + back_geo.left(), back_t.dy + back_geo.in_y);
            let mut left = Snake::create(
                &self.skin,
                color.clone(),
                Some(arrows.as_to(Direction::Left)),
            )
            .with_label(self.back2_label.clone(), self.arrow_horizontal_alignment());
            left.add_point(ix, iy);
            left.add_point(ix, y2);
            left.add_point(x2, y2);
            g.draw_snake(left);
        }

        // ConnectionBackSimple: body back to the diamond
        if self.backward.is_none() && body_geo.has_point_out() {
            let (x1, y1) = (body_t.dx + body_geo.left(), body_t.dy + body_geo.out_y);
            let x2 = diamond_t.dx + diamond_geo.width;
            let y2 = diamond_t.dy + diamond_geo.in_y + (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
            let bottom = body_t.dy + body_geo.height;
            let y1bis = y1.max(bottom) + HEXAGON_HALF_SIZE;
            let mut back = Snake::create(
                &self.skin,
                color.clone(),
                Some(arrows.as_to(Direction::Left)),
            )
            .emphasize_direction(Direction::Up)
            .with_vertical_label(self.back_label.clone(), VerticalAlignment::Bottom);
            back.add_point(x1, y1);
            back.add_point(x1, y1bis);
            back.add_point(total.width, y1bis);
            back.add_point(total.width, y2);
            back.add_point(x2, y2);
            g.draw_snake(back);
            g.ug()
                .apply_translate(UTranslate::new(x1, y1bis))
                .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
        }

        // ConnectionOut: the exit channel on the left
        let half = (diamond_geo.out_y - diamond_geo.in_y) / 2.0;
        let y1 = diamond_t.dy + diamond_geo.in_y + half;
        let mut exit = Snake::create(&self.skin, color.clone(), None)
            .with_merge(MergeStrategy::Limited)
            .emphasize_direction(Direction::Down);
        exit.add_point(diamond_t.dx, y1);
        exit.add_point(HEXAGON_HALF_SIZE, y1);
        exit.add_point(HEXAGON_HALF_SIZE, total.height);
        g.draw_snake(exit);
        let mut base = Snake::create(&self.skin, color.clone(), None);
        base.add_point(HEXAGON_HALF_SIZE, total.height);
        base.add_point(total.left(), total.height);
        g.draw_snake(base);

        // a `break` leaves through the exit channel (`FtileFactoryDelegatorWhile`)
        for point in self.body.welding_points() {
            let Some(t) = self.translate_for(point.tile.as_ref(), sb) else {
                continue;
            };
            let mut leave = Snake::create(
                &self.skin,
                color.clone(),
                Some(arrows.as_to(Direction::Left)),
            );
            leave.add_point(t.dx, t.dy);
            leave.add_point(HEXAGON_HALF_SIZE, t.dy);
            g.draw_snake(leave);
        }

        let dw = self.diamond_width(sb);
        self.diamond.draw(g, diamond_t.dx, diamond_t.dy, dw);
        let dh = self.diamond.height;
        let at = |x: f64, y: f64| {
            g.ug()
                .apply_translate(UTranslate::new(diamond_t.dx + x, diamond_t.dy + y))
        };
        if let Some(yes) = &self.yes {
            yes.draw(&at(4.0 + dw / 2.0, dh));
        }
        if let Some(out) = &self.out {
            let d = out.calculate_dimension(sb);
            out.draw(&at(-d.width, -d.height + dh / 2.0));
        }
        g.apply_translate(body_t).draw_ftile(self.body.as_ref());
        if let Some(backward) = &self.backward {
            g.apply_translate(self.backward_translate(backward, sb))
                .draw_ftile(backward.as_ref());
        }
    }
}
