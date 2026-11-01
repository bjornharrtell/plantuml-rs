//! A `switch`: the test in a diamond, the cases side by side and a merge diamond
//! (`FtileSwitchNude`, `FtileSwitchWithDiamonds`, `FtileSwitchWithManyLinks`, `FtileSwitchWithOneLink`).

use super::factory::{arrow_text_block, FtileRef};
use super::if_links::Merge;
use super::if_long::DiamondInside2;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::{same_ftile, FnConnection, Ftile, FtileGeometry, FtileGraphic};
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
    swimlane: Option<Rc<Swimlane>>,
}

fn size(block: &Option<Rc<dyn TextBlock>>, sb: &dyn StringBounder) -> (f64, f64) {
    block.as_ref().map_or((0.0, 0.0), |b| {
        let d = b.calculate_dimension(sb);
        (d.width, d.height)
    })
}

impl FtileSwitch {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        test: Display,
        branches: Vec<SwitchBranch>,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        sb: &dyn StringBounder,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
        swimlane: Option<Rc<Swimlane>>,
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
            swimlane: swimlane.clone(),
        });
        let diamond2_back = color
            .clone()
            .unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back());
        let diamond2 = Rc::new(
            Merge::new(
                None,
                None,
                None,
                diamond2_back,
                skin.clone(),
                swimlane.clone(),
            )
            .hexagon(),
        );
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
            swimlane,
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
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes: Vec<Rc<Swimlane>> = self.swimlane.iter().cloned().collect();
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
        let skin = self.skin.clone();
        let color = self.arrow_color.clone();
        let snake = {
            let (skin, color, arrows) = (skin.clone(), color.clone(), arrows);
            move |dir: Direction| Snake::create(&skin, color.clone(), Some(arrows.as_to(dir)))
        };
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
        let diamond1: FtileRef = self.diamond1.clone();
        let diamond2: FtileRef = self.diamond2.clone();

        if !self.many() {
            // FtileSwitchWithOneLink
            let (x2, y2) = in_point(0);
            let y1 = d1t.dy + d1.out_y;
            let (ns, label, align) = (
                snake.clone(),
                self.positive[0].clone(),
                self.arrow_horizontal_alignment(),
            );
            let conn = FnConnection::new(
                Some(diamond1.clone()),
                Some(self.tiles[0].clone()),
                move |g| {
                    let mut top = ns(Direction::Down).with_label(label.clone(), align);
                    top.add_point(x2, y1);
                    top.add_point(x2, y2);
                    g.draw_snake(top);
                },
            );
            g.draw_connection(&conn);
            if self.has_out(0, sb) {
                let (x1, y1) = out_point(0);
                let y2 = d2t.dy + d2.in_y;
                let ns = snake.clone();
                let conn = FnConnection::new(
                    Some(self.tiles[0].clone()),
                    Some(diamond2.clone()),
                    move |g| {
                        let mut bottom = ns(Direction::Down);
                        bottom.add_point(x1, y1);
                        bottom.add_point(x1, y2);
                        g.draw_snake(bottom);
                    },
                );
                g.draw_connection(&conn);
            }
            return;
        }

        // `differentSwimlane(this, tile)`
        let different = |from: Option<Rc<Swimlane>>, to: Option<Rc<Swimlane>>| match (from, to) {
            (Some(a), Some(b)) => !Rc::ptr_eq(&a, &b),
            _ => false,
        };
        let other_lane = |i: usize| different(self.swimlane.clone(), self.tiles[i].swimlane_in());

        // ingoing arrows
        let p1b = (d1t.dx + d1.width, d1t.dy + mid1);
        let p1d = (d1t.dx, d1t.dy + mid1);
        let p1c = (d1t.dx + d1.left(), d1t.dy + d1.out_y);
        for i in [0, n - 1] {
            let is_last = i == n - 1 && n > 1;
            let (x1, y1) = if is_last { p1b } else { p1d };
            let (x2, y2) = in_point(i);
            let (ns, label, align) = (
                snake.clone(),
                self.positive[i].clone(),
                self.arrow_horizontal_alignment(),
            );
            let dh = d1.height;
            let conn = FnConnection::new(
                Some(diamond1.clone()),
                Some(self.tiles[i].clone()),
                move |g| {
                    let mut s = ns(Direction::Down).with_label(label.clone(), align);
                    s.add_point(x1, y1);
                    if is_last && x1 > x2 {
                        s.add_point(x1 + HEXAGON_HALF_SIZE, y1);
                        s.add_point(x1 + HEXAGON_HALF_SIZE, y1 + dh);
                        s.add_point(x2, y1 + dh);
                    } else {
                        s.add_point(x2, y1);
                    }
                    s.add_point(x2, y2);
                    g.draw_snake(s);
                },
            );
            g.draw_connection(&conn);
        }
        for i in 0..n {
            let (x2, y2) = in_point(i);
            if i > 0 && i + 1 < n {
                let (ns, label) = (snake.clone(), self.positive[i].clone());
                let conn = FnConnection::new(
                    Some(diamond1.clone()),
                    Some(self.tiles[i].clone()),
                    move |g| {
                        let mut s = ns(Direction::Down)
                            .with_vertical_label(label.clone(), VerticalAlignment::Center);
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
                    },
                );
                g.draw_connection(&conn);
            }
            if other_lane(i) {
                // ConnectionHorizontalThenVerticalCrossSwimlane
                let p1 = (d1t.dx + d1.left(), d1t.dy + d1.out_y);
                let (ns, label, align) = (
                    snake.clone(),
                    self.positive[i].clone(),
                    self.arrow_horizontal_alignment(),
                );
                let (half_w, half_h) = (d1.width / 2.0, d1.height / 2.0);
                let ns_plain = ns.clone();
                let conn = FnConnection::new(
                    Some(diamond1.clone()),
                    Some(self.tiles[i].clone()),
                    move |g| {
                        let mut s = ns_plain(Direction::Down);
                        s.add_point(p1.0, p1.1);
                        s.add_point(x2, y2);
                        g.draw_snake(s);
                    },
                )
                .translatable(move |g, t1, t2| {
                    let mp1 = (p1.0 + t1.dx, p1.1 + t1.dy);
                    let mp2 = (x2 + t2.dx, y2 + t2.dy);
                    let mut s = ns(Direction::Down).with_label(label.clone(), align);
                    let start_x = if mp1.0 > mp2.0 {
                        mp1.0 - half_w
                    } else {
                        mp1.0 + half_w
                    };
                    s.add_point(start_x, p1.1 - half_h);
                    s.add_point(mp2.0, p1.1 - half_h);
                    s.add_point(mp2.0, mp2.1);
                    g.draw_snake(s);
                });
                g.draw_connection(&conn);
            }
        }

        // outgoing arrows
        let ok = |i: usize| self.has_out(i, sb) && !other_lane(i);
        let Some(last_out) = (0..n).rev().find(|i| ok(*i)) else {
            return;
        };
        let first_out = (0..n - 1).find(|i| ok(*i)).unwrap_or(n);
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
            let (ns, label) = (snake.clone(), self.special[i].clone());
            let conn = FnConnection::new(
                Some(self.tiles[i].clone()),
                Some(diamond2.clone()),
                move |g| {
                    let mut s =
                        ns(dir).with_vertical_label(label.clone(), VerticalAlignment::Center);
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
                },
            );
            g.draw_connection(&conn);
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
            let (ns, label) = (snake.clone(), self.special[i].clone());
            let conn = FnConnection::new(
                Some(self.tiles[i].clone()),
                Some(diamond2.clone()),
                move |g| {
                    let mut s = ns(Direction::Down)
                        .with_vertical_label(label.clone(), VerticalAlignment::Center);
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
                },
            );
            g.draw_connection(&conn);
        }

        // ConnectionVerticalThenHorizontalCrossSwimlane
        for tile in &self.tiles {
            let Some(origin) = find_last_with_point_out(tile, sb) else {
                continue;
            };
            if !different(origin.swimlane_out(), diamond2.swimlane_in()) {
                continue;
            }
            let Some(t) = self.translate_for(origin.as_ref(), sb) else {
                continue;
            };
            let og = origin.calculate_dimension(sb);
            let p1 = (t.dx + og.left(), t.dy + og.out_y);
            let p2 = (d2t.dx + d2.left(), d2t.dy + d2.in_y);
            let (skin, color, arrows) = (skin.clone(), color.clone(), arrows);
            let (half_w, half_h) = (d2.width / 2.0, d2.height / 2.0);
            let conn = FnConnection::new(Some(origin.clone()), Some(diamond2.clone()), |_| {})
                .translatable(move |g, t1, t2| {
                    let mp1 = (p1.0 + t1.dx, p1.1 + t1.dy);
                    let mp2 = (p2.0 + t2.dx, p2.1 + t2.dy);
                    let left = mp1.0 > mp2.0;
                    let arrow = arrows.as_to(if left {
                        Direction::Left
                    } else {
                        Direction::Right
                    });
                    let mut s = Snake::create(&skin, color.clone(), Some(arrow));
                    s.add_point(mp1.0, mp1.1);
                    s.add_point(mp1.0, mp2.1 + half_h);
                    s.add_point(
                        if left { mp2.0 + half_w } else { mp2.0 - half_w },
                        mp2.1 + half_h,
                    );
                    g.draw_snake(s);
                });
            g.draw_connection(&conn);
        }
    }
}

/// The first tile at or under `tile` that has an exit point (`findLastWithPointOut`).
fn find_last_with_point_out(tile: &FtileRef, sb: &dyn StringBounder) -> Option<FtileRef> {
    if tile.calculate_dimension(sb).has_point_out() {
        return Some(tile.clone());
    }
    tile.my_children()
        .iter()
        .find_map(|child| find_last_with_point_out(child, sb))
}
