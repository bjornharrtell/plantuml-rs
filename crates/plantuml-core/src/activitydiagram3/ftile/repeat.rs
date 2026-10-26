//! A `repeat` loop (`FtileRepeat`): body first, condition at the bottom.

use super::factory::FtileRef;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::{Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UPolygon, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::Direction;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const START_DIAMOND: f64 = 24.0;
const SPACE_AROUND: f64 = 20.0;
const LOOP_MARGIN: f64 = 12.0;

enum Start {
    Diamond,
    Tile(FtileRef),
}

/// A loop whose condition is tested at the end (`FtileRepeat`).
pub struct FtileRepeat {
    body: FtileRef,
    start: Start,
    condition: Diamond,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileRepeat {
    pub fn new(
        body: FtileRef,
        start: Option<FtileRef>,
        test: Display,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> Rc<Self> {
        let condition = Diamond::new(test, &skin, back);
        Rc::new(Self {
            body,
            start: match start {
                Some(t) => Start::Tile(t),
                None => Start::Diamond,
            },
            condition,
            skin,
            in_link_rendering,
            out_link_rendering,
        })
    }

    fn start_dim(&self, sb: &dyn StringBounder) -> (f64, f64) {
        match &self.start {
            Start::Diamond => (START_DIAMOND, START_DIAMOND),
            Start::Tile(t) => {
                let d = t.calculate_dimension(sb);
                (d.width, d.height)
            }
        }
    }

    fn condition_width(&self, sb: &dyn StringBounder) -> f64 {
        self.condition.width(sb)
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        let body = self.body.calculate_dimension(sb);
        let start = self.start_dim(sb);
        self.condition_width(sb).max(body.width).max(start.0) + 2.0 * LOOP_MARGIN
    }

    fn start_offset(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let start = self.start_dim(sb);
        ((self.width(sb) - start.0) / 2.0, 0.0)
    }

    fn body_offset(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let body = self.body.calculate_dimension(sb);
        let start = self.start_dim(sb);
        ((self.width(sb) - body.width) / 2.0, start.1 + SPACE_AROUND)
    }

    fn condition_offset(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let body = self.body.calculate_dimension(sb);
        let (_, by) = self.body_offset(sb);
        let cw = self.condition_width(sb);
        ((self.width(sb) - cw) / 2.0, by + body.height + SPACE_AROUND)
    }

    fn height(&self, sb: &dyn StringBounder) -> f64 {
        self.condition_offset(sb).1 + self.condition.height
    }
}

impl Ftile for FtileRepeat {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let width = self.width(sb);
        let height = self.height(sb);
        FtileGeometry::with_out(width, height, width / 2.0, 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if let Start::Tile(t) = &self.start {
            let (x, y) = self.start_offset(sb);
            if super::same_ftile(child, t.as_ref()) {
                return Some(UTranslate::new(x, y));
            }
            if let Some(inner) = t.translate_for(child, sb) {
                return Some(inner.compose(UTranslate::new(x, y)));
            }
        }
        let (x, y) = self.body_offset(sb);
        if super::same_ftile(child, self.body.as_ref()) {
            return Some(UTranslate::new(x, y));
        }
        self.body
            .translate_for(child, sb)
            .map(|t| t.compose(UTranslate::new(x, y)))
    }

    fn my_children(&self) -> Vec<FtileRef> {
        let mut children = vec![self.body.clone()];
        if let Start::Tile(t) = &self.start {
            children.push(t.clone());
        }
        children
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        match &self.start {
            Start::Diamond => self.body.swimlane_in(),
            Start::Tile(t) => t.swimlane_in(),
        }
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
        let width = self.width(sb);
        let body = self.body.calculate_dimension(sb);
        let (sx, sy) = self.start_offset(sb);
        let (sdx, sdy) = self.start_dim(sb);
        let (bx, by) = self.body_offset(sb);
        let (cx, cy) = self.condition_offset(sb);
        let cw = self.condition_width(sb);
        let color = crate::activitydiagram3::ftile::factory::in_link_rendering_color(
            &self.skin,
            self.body.as_ref(),
        );
        let arrows = Arrows::from_skin(&self.skin);
        let start_middle = sdy / 2.0;
        let condition_middle = cy + self.condition.height / 2.0;

        // start to body
        let mut down = Snake::create(
            &self.skin,
            color.clone(),
            Some(arrows.as_to(Direction::Down)),
        );
        down.add_point(sx + sdx / 2.0, sdy);
        down.add_point(bx + body.width / 2.0, by);
        g.draw_snake(down);

        // body to condition
        let mut down = Snake::create(
            &self.skin,
            color.clone(),
            Some(arrows.as_to(Direction::Down)),
        );
        down.add_point(bx + body.width / 2.0, by + body.height);
        down.add_point(cx + cw / 2.0, cy);
        g.draw_snake(down);

        // condition back to the start
        let mut back = Snake::create(
            &self.skin,
            color.clone(),
            Some(arrows.as_to(Direction::Left)),
        );
        back.add_point(cx + cw, condition_middle);
        back.add_point(width, condition_middle);
        back.add_point(width, start_middle);
        back.add_point(sx + sdx, start_middle);
        g.draw_snake(back);

        if let Start::Diamond = self.start {
            let mut polygon = UPolygon::new(vec![
                (START_DIAMOND / 2.0, 0.0),
                (START_DIAMOND, START_DIAMOND / 2.0),
                (START_DIAMOND / 2.0, START_DIAMOND),
                (0.0, START_DIAMOND / 2.0),
            ]);
            let style = self
                .skin
                .style_builder()
                .merged_style(&queries::ACTIVITYDIAG_ACTIVITY_DIAMOND)
                .expect("diamond style");
            let back = style.value(PName::BackGroundColor).as_color();
            let border = style.value(PName::LineColor).as_color();
            polygon.delta_shadow = 0.0;
            g.ug()
                .apply_translate(UTranslate::new(sx, sy))
                .apply_color(border)
                .apply_back_color(back)
                .draw(polygon);
        } else if let Start::Tile(t) = &self.start {
            g.apply_translate(UTranslate::new(sx, sy))
                .draw_ftile(t.as_ref());
        }

        self.condition.draw(g, cx, cy, cw);
        g.apply_translate(UTranslate::new(bx, by))
            .draw_ftile(self.body.as_ref());
    }
}
