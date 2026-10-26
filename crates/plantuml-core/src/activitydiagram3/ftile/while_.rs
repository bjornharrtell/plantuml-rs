//! A `while` loop (`FtileWhile`, `FtileDiamond`).

use super::factory::FtileRef;
use super::snake::{Arrows, Snake};
use super::{Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UPolygon, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, HorizontalAlignment};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const DIAMOND_HEIGHT: f64 = 24.0;
const DIAMOND_SLOPE: f64 = 12.0;
const SPACE_AROUND: f64 = 20.0;
const BOTTOM_GAP: f64 = 10.0;
const LOOP_MARGIN: f64 = 12.0;

/// The condition diamond (`FtileDiamond`).
pub(crate) struct Diamond {
    label: Rc<dyn TextBlock>,
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

    pub(crate) fn width(&self, sb: &dyn StringBounder) -> f64 {
        let body = self.body.calculate_dimension(sb);
        self.diamond_width(sb).max(body.width) + 2.0 * LOOP_MARGIN
    }

    fn body_offset(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let body = self.body.calculate_dimension(sb);
        let x = (self.width(sb) - body.width) / 2.0;
        let y = self.diamond.height + SPACE_AROUND;
        (x, y)
    }

    fn height(&self, sb: &dyn StringBounder) -> f64 {
        let body = self.body.calculate_dimension(sb);
        self.body_offset(sb).1 + body.height + 2.0 * BOTTOM_GAP
    }
}

impl Ftile for FtileWhile {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let width = self.width(sb);
        let height = self.height(sb);
        FtileGeometry::with_out(width, height, width / 2.0, 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let (x, y) = self.body_offset(sb);
        if super::same_ftile(child, self.body.as_ref()) {
            return Some(UTranslate::new(x, y));
        }
        self.body
            .translate_for(child, sb)
            .map(|t| t.compose(UTranslate::new(x, y)))
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
        let width = self.width(sb);
        let dw = self.diamond_width(sb);
        let dx = (width - dw) / 2.0;
        let (bx, by) = self.body_offset(sb);
        let body = self.body.calculate_dimension(sb);
        let height = self.height(sb);
        let middle = self.diamond.height / 2.0;

        let color = crate::activitydiagram3::ftile::factory::in_link_rendering_color(
            &self.skin,
            self.body.as_ref(),
        );
        let arrows = Arrows::from_skin(&self.skin);

        // condition to body
        let mut down = Snake::create(&self.skin, color.clone(), Some(arrows.as_to(Direction::Down)));
        down.add_point(bx + body.width / 2.0, self.diamond.height);
        down.add_point(bx + body.width / 2.0, by);
        g.draw_snake(down);

        // body back to the diamond (the loop)
        let mut back = Snake::create(&self.skin, color.clone(), Some(arrows.as_to(Direction::Left)));
        back.add_point(bx + body.width / 2.0, by + body.height);
        back.add_point(bx + body.width / 2.0, by + body.height + BOTTOM_GAP);
        back.add_point(width, by + body.height + BOTTOM_GAP);
        back.add_point(width, middle);
        back.add_point(dx + dw, middle);
        g.draw_snake(back);

        // exit of the loop
        let mut exit = Snake::create(&self.skin, color, None);
        exit.add_point(dx, middle);
        exit.add_point(0.0, middle);
        exit.add_point(0.0, height);
        exit.add_point(width / 2.0, height);
        g.draw_snake(exit);

        self.diamond.draw(g, dx, 0.0, dw);
        g.apply_translate(UTranslate::new(bx, by))
            .draw_ftile(self.body.as_ref());
    }
}
