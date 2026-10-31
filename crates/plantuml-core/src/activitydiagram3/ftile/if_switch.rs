//! A `switch`: the test in a diamond, the cases side by side and a merge diamond
//! (`FtileSwitchNude`, `FtileSwitchWithDiamonds`, `FtileSwitchWithManyLinks`, `FtileSwitchWithOneLink`).

use super::factory::{arrow_text_block, FtileRef};
use super::if_links::Merge;
use super::if_long::DiamondInside2;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::{same_ftile, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::{LinkRendering, Rainbow};
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::UTranslate;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, VerticalAlignment};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::queries;
use std::rc::Rc;

const SUPP15: f64 = 15.0;
const X_SEPARATION: f64 = 20.0;
const MARGIN: f64 = 10.0;
const HEXAGON_HALF_SIZE: f64 = 12.0;

/// A `case` (`Branch`).
pub struct SwitchBranch {
    pub tile: FtileRef,
    pub label: Option<Display>,
}

/// Room for the label of the arrow into a case and of the one out of it
/// (`FtileDecorateInLabel`, `FtileDecorateOutLabel`).
struct Labelled {
    inner: FtileRef,
    in_label: (f64, f64),
    out_label: (f64, f64),
}

impl Ftile for Labelled {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let g = self.inner.calculate_dimension(sb);
        let (xl, yl) = self.in_label;
        let (mut width, mut height) = (g.width, g.height + yl);
        let (left, in_y) = (g.left(), g.in_y + yl);
        let out_y = g.out_y + yl;
        let right = width - left;
        if xl > right {
            width += xl - right;
        }
        let (xo, yo) = self.out_label;
        height += yo;
        let right = width - left;
        if xo > right {
            width += xo - right;
        }
        if g.has_point_out() {
            FtileGeometry::with_out(width, height, left, in_y, out_y)
        } else {
            FtileGeometry::new(width, height, left, in_y)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let t = UTranslate::dy(self.in_label.1);
        if same_ftile(child, self.inner.as_ref()) {
            return Some(t);
        }
        self.inner.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn draw(&self, g: &FtileGraphic) {
        g.apply_translate(UTranslate::dy(self.in_label.1))
            .draw_ftile(self.inner.as_ref());
    }
}

pub struct FtileSwitch {
    tiles: Vec<FtileRef>,
    positive: Vec<Option<Rc<dyn TextBlock>>>,
    special: Vec<Option<Rc<dyn TextBlock>>>,
    diamond1: Rc<DiamondInside2>,
    diamond2: Rc<Merge>,
    big_diamond: bool,
    w13: f64,
    w9: f64,
    skin: Rc<SkinParam>,
    arrow_color: Rainbow,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

fn size(block: &Option<Rc<dyn TextBlock>>, sb: &dyn StringBounder) -> (f64, f64) {
    block.as_ref().map_or((0.0, 0.0), |b| {
        let d = b.calculate_dimension(sb);
        (d.width, d.height)
    })
}

impl FtileSwitch {
    pub fn create(
        test: Display,
        branches: Vec<SwitchBranch>,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        sb: &dyn StringBounder,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> FtileRef {
        let color = back;
        let diamond1 = Rc::new(DiamondInside2 {
            diamond: Diamond::new(test, &skin, color.clone()),
            north: None,
            south: None,
            west: None,
            east: None,
            skin: skin.clone(),
            north_widens: false,
        });
        let diamond2_back = color
            .clone()
            .unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back());
        let diamond2 = Rc::new(Merge::new(None, None, None, diamond2_back, skin.clone()));
        let mut tiles: Vec<FtileRef> = Vec::new();
        let mut positive = Vec::new();
        let mut special = Vec::new();
        for b in branches {
            let pos = b.label.as_ref().and_then(|d| arrow_text_block(&skin, d));
            let spe = arrow_text_block(&skin, b.tile.out_link_rendering().display());
            tiles.push(Rc::new(Labelled {
                inner: b.tile,
                in_label: size(&pos, sb),
                out_label: size(&spe, sb),
            }));
            positive.push(pos);
            special.push(spe);
        }
        let style = skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ARROW)
            .expect("arrow style");
        let arrow_color = Rainbow::from_style(&style);
        let first = tiles[0].calculate_dimension(sb);
        let last = tiles[tiles.len() - 1].calculate_dimension(sb);
        let w13 =
            diamond1.calculate_dimension(sb).width - (first.width - first.left()) - last.left();
        let n = tiles.len();
        let w9: f64 = tiles
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= 1 && i + 1 < n)
            .map(|(_, t)| t.calculate_dimension(sb).width)
            .sum();
        Rc::new(Self {
            tiles,
            positive,
            special,
            diamond1,
            diamond2,
            big_diamond: w13 > w9,
            w13,
            w9,
            skin,
            arrow_color,
            in_link_rendering,
            out_link_rendering,
        })
    }

    fn many(&self) -> bool {
        self.tiles.len() > 1
    }

    fn y_delta_1a(&self, sb: &dyn StringBounder) -> f64 {
        if !self.many() {
            return 20.0;
        }
        let mut max = 10.0f64;
        for p in &self.positive {
            max = max.max(size(p, sb).1);
        }
        if self.big_diamond {
            max += self.diamond1.calculate_dimension(sb).height / 2.0;
        }
        max + 10.0
    }

    fn y_delta_1b(&self) -> f64 {
        10.0
    }

    fn nude_height(&self, sb: &dyn StringBounder) -> f64 {
        self.tiles
            .iter()
            .map(|t| t.calculate_dimension(sb).height)
            .fold(0.0, f64::max)
            + 100.0
    }

    fn nude(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let width: f64 = self
            .tiles
            .iter()
            .map(|t| t.calculate_dimension(sb).width)
            .sum::<f64>()
            + X_SEPARATION * (self.tiles.len() - 1) as f64;
        FtileGeometry::new(width, self.nude_height(sb), width / 2.0, 0.0)
    }

    fn internal(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let delta = self.y_delta_1a(sb) + self.y_delta_1b();
        if self.big_diamond {
            let height = d1.height + self.nude_height(sb) + d2.height + delta;
            let t0 = self.tiles[0].calculate_dimension(sb);
            let last = self.tiles[self.tiles.len() - 1].calculate_dimension(sb);
            let width = t0.width + SUPP15 + self.w13 + SUPP15 + last.width;
            FtileGeometry::with_out(width, height, t0.left() + SUPP15 + d1.left(), 0.0, height)
        } else {
            let all = d1.append_bottom(&self.nude(sb)).append_bottom(&d2);
            if all.has_point_out() {
                FtileGeometry::with_out(
                    all.width,
                    all.height + delta,
                    all.left(),
                    all.in_y,
                    all.out_y + delta,
                )
            } else {
                FtileGeometry::new(all.width, all.height + delta, all.left(), all.in_y)
            }
        }
    }

    fn translate_main(&self, sb: &dyn StringBounder) -> UTranslate {
        UTranslate::dy(self.diamond1.calculate_dimension(sb).height + self.y_delta_1a(sb))
    }

    fn translate_of(&self, index: usize, sb: &dyn StringBounder) -> UTranslate {
        let main = self.translate_main(sb);
        let n = self.tiles.len();
        if self.big_diamond {
            if index == n - 1 {
                let t0 = self.tiles[0].calculate_dimension(sb).width;
                return UTranslate::dx(t0 + self.w13 + SUPP15 + SUPP15).compose(main);
            }
            let suppx = (self.w13 - self.w9) / (n - 1) as f64;
            let dx: f64 = self.tiles[..index]
                .iter()
                .map(|t| t.calculate_dimension(sb).width + suppx)
                .sum();
            return UTranslate::dx(dx).compose(main);
        }
        let dx: f64 = self.tiles[..index]
            .iter()
            .map(|t| t.calculate_dimension(sb).width + X_SEPARATION)
            .sum();
        UTranslate::dx(dx).compose(main)
    }

    fn translate_diamond1(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.internal(sb);
        UTranslate::dx(total.left() - self.diamond1.calculate_dimension(sb).left())
    }

    fn translate_diamond2(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.internal(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        UTranslate::new(total.left() - d2.width / 2.0, total.height - d2.height)
    }

    fn has_out(&self, i: usize, sb: &dyn StringBounder) -> bool {
        self.tiles[i].calculate_dimension(sb).has_point_out()
    }
}

impl Ftile for FtileSwitch {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let total = self.internal(sb);
        if (0..self.tiles.len()).any(|i| self.has_out(i, sb)) {
            total
        } else {
            total.without_point_out()
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        for (i, tile) in self.tiles.iter().enumerate() {
            let t = self.translate_of(i, sb);
            if same_ftile(child, tile.as_ref()) {
                return Some(t);
            }
            if let Some(c) = tile.translate_for(child, sb) {
                return Some(c.compose(t));
            }
        }
        if same_ftile(child, self.diamond1.as_ref()) {
            return Some(self.translate_diamond1(sb));
        }
        if same_ftile(child, self.diamond2.as_ref()) {
            return Some(self.translate_diamond2(sb));
        }
        None
    }

    fn my_children(&self) -> Vec<FtileRef> {
        let mut children = self.tiles.clone();
        children.push(self.diamond1.clone());
        children.push(self.diamond2.clone());
        children
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.tiles[0].swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes: Vec<Rc<Swimlane>> = Vec::new();
        for t in &self.tiles {
            for lane in t.swimlanes() {
                if !lanes.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                    lanes.push(lane);
                }
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
        let n = self.tiles.len();
        let (d1t, d2t) = (self.translate_diamond1(sb), self.translate_diamond2(sb));
        g.apply_translate(d1t).draw_ftile(self.diamond1.as_ref());
        for (i, tile) in self.tiles.iter().enumerate() {
            g.apply_translate(self.translate_of(i, sb))
                .draw_ftile(tile.as_ref());
        }
        let has_out = self.calculate_dimension(sb).has_point_out();
        if has_out {
            g.apply_translate(d2t).draw_ftile(self.diamond2.as_ref());
        }

        let arrows = Arrows::from_skin(&self.skin);
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let color = self.arrow_color.clone();
        let snake =
            |dir: Direction| Snake::create(&self.skin, color.clone(), Some(arrows.as_to(dir)));
        let mid1 = (d1.in_y + d1.out_y) / 2.0;
        let mid2 = (d2.in_y + d2.out_y) / 2.0;
        let in_point = |i: usize| {
            let t = self.translate_of(i, sb);
            let g = self.tiles[i].calculate_dimension(sb);
            (t.dx + g.left(), t.dy + g.in_y)
        };
        let out_point = |i: usize| {
            let t = self.translate_of(i, sb);
            let g = self.tiles[i].calculate_dimension(sb);
            (t.dx + g.left(), t.dy + g.out_y)
        };

        if !self.many() {
            // FtileSwitchWithOneLink
            let (x2, y2) = in_point(0);
            let y1 = d1t.dy + d1.out_y;
            let mut top = snake(Direction::Down)
                .with_label(self.positive[0].clone(), self.arrow_horizontal_alignment());
            top.add_point(x2, y1);
            top.add_point(x2, y2);
            g.draw_snake(top);
            if self.has_out(0, sb) {
                let (x1, y1) = out_point(0);
                let mut bottom = snake(Direction::Down);
                bottom.add_point(x1, y1);
                bottom.add_point(x1, d2t.dy + d2.in_y);
                g.draw_snake(bottom);
            }
            return;
        }

        // ingoing arrows
        let p1b = (d1t.dx + d1.width, d1t.dy + mid1);
        let p1d = (d1t.dx, d1t.dy + mid1);
        let p1c = (d1t.dx + d1.left(), d1t.dy + d1.out_y);
        for i in [0, n - 1] {
            let is_last = i == n - 1 && n > 1;
            let (x1, y1) = if is_last { p1b } else { p1d };
            let (x2, y2) = in_point(i);
            let mut s = snake(Direction::Down)
                .with_label(self.positive[i].clone(), self.arrow_horizontal_alignment());
            s.add_point(x1, y1);
            if is_last && x1 > x2 {
                s.add_point(x1 + HEXAGON_HALF_SIZE, y1);
                s.add_point(x1 + HEXAGON_HALF_SIZE, y1 + d1.height);
                s.add_point(x2, y1 + d1.height);
            } else {
                s.add_point(x2, y1);
            }
            s.add_point(x2, y2);
            g.draw_snake(s);
        }
        for i in 1..n - 1 {
            let (x2, y2) = in_point(i);
            let mut s = snake(Direction::Down)
                .with_vertical_label(self.positive[i].clone(), VerticalAlignment::Center);
            if x2 < p1d.0 - MARGIN || x2 > p1b.0 + MARGIN {
                s.add_point(x2, p1d.1);
                s.add_point(x2, y2);
            } else {
                let (x1, y1) = p1c;
                let ym = (y1 * 2.0 + y2) / 3.0;
                s.add_point(x1, y1);
                s.add_point(x1, ym);
                s.add_point(x2, ym);
                s.add_point(x2, y2);
            }
            g.draw_snake(s);
        }

        // outgoing arrows
        let Some(last_out) = (0..n).rev().find(|i| self.has_out(*i, sb)) else {
            return;
        };
        let first_out = (0..n - 1).find(|i| self.has_out(*i, sb)).unwrap_or(n);
        let a = (d2t.dx + d2.left(), d2t.dy + d2.in_y);
        let b = (d2t.dx + d2.width, d2t.dy + mid2);
        let d = (d2t.dx, d2t.dy + mid2);
        let vertical_then_horizontal = |i: usize| {
            let (x1, y1) = out_point(i);
            let (p2, dir) = if x1 < d.0 {
                (d, Direction::Right)
            } else if x1 > b.0 {
                (b, Direction::Left)
            } else {
                (a, Direction::Down)
            };
            let mut s =
                snake(dir).with_vertical_label(self.special[i].clone(), VerticalAlignment::Center);
            s.add_point(x1, y1);
            if dir == Direction::Left && p2.0 > x1 - 10.0 {
                s.add_point(x1, p2.1 - 8.0);
                s.add_point(x1 + 12.0, p2.1 - 8.0);
                s.add_point(x1 + 12.0, p2.1);
            } else {
                s.add_point(x1, p2.1);
            }
            s.add_point(p2.0, p2.1);
            g.draw_snake(s);
        };
        if first_out < n {
            vertical_then_horizontal(first_out);
        }
        if last_out > 0 {
            vertical_then_horizontal(last_out);
        }
        for i in first_out + 1..last_out {
            if !self.has_out(i, sb) {
                continue;
            }
            let (x1, y1) = out_point(i);
            let ym = (y1 + a.1) / 2.0;
            let mut s = snake(Direction::Down)
                .with_vertical_label(self.special[i].clone(), VerticalAlignment::Center);
            if x1 < p1d.0 - MARGIN || x1 > p1b.0 + MARGIN {
                s.add_point(x1, y1);
                s.add_point(x1, d.1);
            } else {
                s.add_point(x1, y1);
                s.add_point(x1, ym);
                s.add_point(a.0, ym);
                s.add_point(a.0, a.1);
            }
            g.draw_snake(s);
        }
    }
}
