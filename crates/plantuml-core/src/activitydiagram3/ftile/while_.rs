//! A `while` loop (`FtileWhile`, `FtileDiamond`).

use super::factory::FtileRef;
use super::snake::{Arrows, MergeStrategy, Snake};
use super::{Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UPolygon, UShape, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, HorizontalAlignment};
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

/// A loop whose condition is tested before the body (`FtileWhile`).
pub struct FtileWhile {
    body: FtileRef,
    diamond: Diamond,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileWhile {
    pub fn new(
        body: FtileRef,
        test: Display,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> Rc<Self> {
        let diamond = Diamond::new(test, &skin, back);
        Rc::new(Self {
            body,
            diamond,
            skin,
            in_link_rendering,
            out_link_rendering,
        })
    }

    fn diamond_width(&self, sb: &dyn StringBounder) -> f64 {
        self.diamond.width(sb)
    }

    fn diamond_geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = (self.diamond_width(sb), self.diamond.height);
        FtileGeometry::with_out(w, h, w / 2.0, 0.0, h)
    }

    /// `calculateDimensionFtile`: the diamond over the body, a channel on the left for the exit and
    /// one on the right for the loop-back.
    fn geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let diamond = self.diamond_geometry(sb);
        let body = self.body.calculate_dimension(sb);
        let stacked = diamond.append_bottom(&body);
        let height = stacked.height + 4.0 * HEXAGON_HALF_SIZE;
        FtileGeometry::with_out(
            stacked.width + 2.0 * HEXAGON_HALF_SIZE + HEXAGON_HALF_SIZE,
            height,
            stacked.left() + 2.0 * HEXAGON_HALF_SIZE,
            diamond.in_y,
            height,
        )
    }

    fn body_translate(&self, sb: &dyn StringBounder) -> UTranslate {
        let diamond = self.diamond_geometry(sb);
        let total = self.geometry(sb);
        let body = self.body.calculate_dimension(sb);
        let y = diamond.height + (total.height - diamond.height - body.height) / 2.0;
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
        let t = self.body_translate(sb);
        if super::same_ftile(child, self.body.as_ref()) {
            return Some(t);
        }
        self.body.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn my_children(&self) -> Vec<FtileRef> {
        vec![self.body.clone()]
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.body.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.body.swimlane_out()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        self.body.swimlanes()
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
            diamond_t.dy + diamond_geo.height,
        );
        let p2 = (body_t.dx + body_geo.left(), body_t.dy + body_geo.in_y);
        down.add_point(p1.0, p1.1);
        down.add_point(p2.0, p2.1);
        g.draw_snake(down);

        // ConnectionBackSimple: body back to the diamond
        if body_geo.has_point_out() {
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
            .emphasize_direction(Direction::Up);
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
        let mut base = Snake::create(&self.skin, color, None);
        base.add_point(HEXAGON_HALF_SIZE, total.height);
        base.add_point(total.left(), total.height);
        g.draw_snake(base);

        self.diamond
            .draw(g, diamond_t.dx, diamond_t.dy, diamond_geo.width);
        g.apply_translate(body_t).draw_ftile(self.body.as_ref());
    }
}
