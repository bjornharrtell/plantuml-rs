//! An `if` with `elseif` branches: the conditions side by side (`FtileIfLongHorizontal`).

use super::factory::{arrow_text_block, in_link_rendering_color, FtileRef};
use super::snake::{Arrows, MergeStrategy, Snake};
use super::while_::Diamond;
use super::wrap::{FtileAssemblySimple, FtileMargedVertically};
use super::{same_ftile, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::UTranslate;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::Direction;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::rc::Rc;

const X_SEPARATION: f64 = 20.0;
const MIN_TILE_WIDTH: f64 = 30.0;
const COUPLE_Y: f64 = 25.0;
const HEXAGON_HALF_SIZE: f64 = 12.0;

/// A branch of the long `if` (`Branch`).
pub struct LongBranch {
    pub tile: FtileRef,
    pub label_test: Display,
    pub label_positive: Option<Display>,
    pub inlabel: Option<Display>,
    pub color: Option<HColor>,
}

/// Centres a tile in at least `min_width` (`FtileMinWidthCentered`).
pub(crate) struct MinWidthCentered {
    pub(crate) inner: FtileRef,
    pub(crate) min_width: f64,
}

impl MinWidthCentered {
    fn dx(&self, sb: &dyn StringBounder) -> f64 {
        let w = self.inner.calculate_dimension(sb).width;
        (self.min_width.max(w) - w) / 2.0
    }
}

impl Ftile for MinWidthCentered {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let geo = self.inner.calculate_dimension(sb);
        let dx = self.dx(sb);
        let width = geo.width + 2.0 * dx;
        if geo.has_point_out() {
            FtileGeometry::with_out(width, geo.height, geo.left() + dx, geo.in_y, geo.out_y)
        } else {
            FtileGeometry::new(width, geo.height, geo.left() + dx, geo.in_y)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let t = UTranslate::dx(self.dx(sb));
        if same_ftile(child, self.inner.as_ref()) {
            return Some(t);
        }
        self.inner.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn draw(&self, g: &FtileGraphic) {
        let dx = self.dx(g.string_bounder());
        g.apply_translate(UTranslate::dx(dx))
            .draw_ftile(self.inner.as_ref());
    }
}

/// A condition with labels around it (`FtileDiamondInside2`).
pub(crate) struct DiamondInside2 {
    pub(crate) diamond: Diamond,
    pub(crate) north: Option<Rc<dyn TextBlock>>,
    pub(crate) west: Option<Rc<dyn TextBlock>>,
    pub(crate) east: Option<Rc<dyn TextBlock>>,
    pub(crate) skin: Rc<SkinParam>,
    /// `FtileDiamondInside2` widens for a wide `north` label, `FtileDiamondInside` does not.
    pub(crate) north_widens: bool,
}

impl DiamondInside2 {
    fn alone(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let label = self.diamond.label.calculate_dimension(sb);
        if label.width == 0.0 || label.height == 0.0 {
            (2.0 * HEXAGON_HALF_SIZE, 2.0 * HEXAGON_HALF_SIZE)
        } else {
            (
                label.width.max(2.0 * HEXAGON_HALF_SIZE) + 2.0 * HEXAGON_HALF_SIZE,
                label.height.max(2.0 * HEXAGON_HALF_SIZE),
            )
        }
    }

    fn size(block: &Option<Rc<dyn TextBlock>>, sb: &dyn StringBounder) -> (f64, f64) {
        block.as_ref().map_or((0.0, 0.0), |b| {
            let d = b.calculate_dimension(sb);
            (d.width, d.height)
        })
    }
}

impl Ftile for DiamondInside2 {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = self.alone(sb);
        let north = Self::size(&self.north, sb);
        let left = w / 2.0;
        let width = if self.north_widens && north.0 > left {
            left + north.0
        } else {
            w
        };
        FtileGeometry::with_out(width, h + north.1, left, 0.0, h)
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let (w, h) = self.alone(sb);
        self.diamond.draw(g, 0.0, 0.0, w);
        if let Some(north) = &self.north {
            north.draw(&g.ug().apply_translate(UTranslate::new(4.0 + w / 2.0, h)));
        }
        if let Some(west) = &self.west {
            let d = west.calculate_dimension(sb);
            west.draw(
                &g.ug()
                    .apply_translate(UTranslate::new(-d.width, -d.height + h / 2.0)),
            );
        }
        if let Some(east) = &self.east {
            let d = east.calculate_dimension(sb);
            east.draw(
                &g.ug()
                    .apply_translate(UTranslate::new(w, -d.height + h / 2.0)),
            );
        }
    }
}

struct Couple {
    assembly: Rc<FtileAssemblySimple>,
    diamond: FtileRef,
    tile: FtileRef,
    margin: f64,
}

impl Couple {
    fn geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let g = self.assembly.calculate_dimension(sb);
        let (w, left) = (g.width + self.margin, g.left() + self.margin);
        if g.has_point_out() {
            FtileGeometry::with_out(w, g.height, left, g.in_y, g.out_y)
        } else {
            FtileGeometry::new(w, g.height, left, g.in_y)
        }
    }

    /// Where `child` (the diamond or the tile) sits inside the couple.
    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> UTranslate {
        self.assembly
            .translate_for(child, sb)
            .expect("couple child")
            .compose(UTranslate::dx(self.margin))
    }
}

/// An `if` whose `elseif` conditions sit side by side (`FtileIfLongHorizontal`).
pub struct FtileIfLongHorizontal {
    couples: Vec<Couple>,
    tile2: FtileRef,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileIfLongHorizontal {
    pub fn new(
        thens: Vec<LongBranch>,
        else_branch: LongBranch,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        sb: Rc<dyn StringBounder>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> Rc<Self> {
        let sb = sb.as_ref();
        let last = thens.len() - 1;
        let mut diamonds: Vec<Rc<DiamondInside2>> = Vec::new();
        let mut inlabel_sizes = Vec::new();
        let mut tiles: Vec<FtileRef> = Vec::new();
        for (i, b) in thens.iter().enumerate() {
            tiles.push(Rc::new(MinWidthCentered {
                inner: b.tile.clone(),
                min_width: MIN_TILE_WIDTH,
            }));
            let color = b.color.clone().or_else(|| back.clone());
            let west = b.inlabel.as_ref().and_then(|d| arrow_text_block(&skin, d));
            inlabel_sizes.push(
                west.as_ref()
                    .map_or(0.0, |w| w.calculate_dimension(sb).width),
            );
            let east = if i == last {
                else_branch
                    .label_positive
                    .as_ref()
                    .and_then(|d| arrow_text_block(&skin, d))
            } else {
                None
            };
            diamonds.push(Rc::new(DiamondInside2 {
                diamond: Diamond::new(b.label_test.clone(), &skin, color),
                north: b
                    .label_positive
                    .as_ref()
                    .and_then(|d| arrow_text_block(&skin, d)),
                west,
                east,
                skin: skin.clone(),
                north_widens: true,
            }));
        }
        let tile2: FtileRef = Rc::new(MinWidthCentered {
            inner: else_branch.tile,
            min_width: MIN_TILE_WIDTH,
        });
        // alignDiamonds: every condition reaches the same out_y, with 20 below
        let max_out_y = diamonds
            .iter()
            .map(|d| d.calculate_dimension(sb).out_y)
            .fold(0.0, f64::max);
        let couples = diamonds
            .iter()
            .zip(&tiles)
            .zip(&inlabel_sizes)
            .map(|((d, tile), margin)| {
                let missing = max_out_y - d.calculate_dimension(sb).out_y;
                let diamond: FtileRef = FtileMargedVertically::new(d.clone(), missing / 2.0, 20.0);
                Couple {
                    assembly: FtileAssemblySimple::new(diamond.clone(), tile.clone()),
                    diamond,
                    tile: tile.clone(),
                    margin: *margin,
                }
            })
            .collect();
        Rc::new(Self {
            couples,
            tile2,
            skin,
            in_link_rendering,
            out_link_rendering,
        })
    }

    fn max_out_y(&self, sb: &dyn StringBounder) -> f64 {
        self.couples
            .iter()
            .map(|c| c.diamond.calculate_dimension(sb).out_y)
            .fold(0.0, f64::max)
    }

    fn diamonds_height(&self, sb: &dyn StringBounder) -> f64 {
        self.couples
            .iter()
            .map(|c| c.diamond.calculate_dimension(sb).height)
            .fold(0.0, f64::max)
    }

    /// `calculateDimensionInternal`: no exit point.
    fn internal(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let (mut w, mut h) = (0.0, 0.0f64);
        for c in &self.couples {
            let g = c.geometry(sb);
            w += g.width;
            h = h.max(g.height);
        }
        let g2 = self.tile2.calculate_dimension(sb);
        w += g2.width;
        h = h.max(g2.height + self.diamonds_height(sb) / 2.0);
        (
            w + X_SEPARATION * self.couples.len() as f64,
            h + self.max_out_y(sb).max(100.0),
        )
    }

    fn couple_translate(&self, i: usize, sb: &dyn StringBounder) -> UTranslate {
        let mut x = 0.0;
        for c in &self.couples[..i] {
            x += c.geometry(sb).width + X_SEPARATION;
        }
        UTranslate::new(x, COUPLE_Y)
    }

    fn tile2_translate(&self, sb: &dyn StringBounder) -> UTranslate {
        let (w, h) = self.internal(sb);
        let g2 = self.tile2.calculate_dimension(sb);
        UTranslate::new(w - g2.width, (h - g2.height) / 2.0)
    }

    fn diamond_translate(&self, i: usize, sb: &dyn StringBounder) -> UTranslate {
        let c = &self.couples[i];
        c.translate_for(c.diamond.as_ref(), sb)
            .compose(self.couple_translate(i, sb))
    }

    fn tile_translate(&self, i: usize, sb: &dyn StringBounder) -> UTranslate {
        let c = &self.couples[i];
        c.translate_for(c.tile.as_ref(), sb)
            .compose(self.couple_translate(i, sb))
    }

    fn all_tiles(&self) -> impl Iterator<Item = &FtileRef> {
        self.couples
            .iter()
            .map(|c| &c.tile)
            .chain(std::iter::once(&self.tile2))
    }
}

impl Ftile for FtileIfLongHorizontal {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = self.internal(sb);
        if self
            .all_tiles()
            .any(|t| t.calculate_dimension(sb).has_point_out())
        {
            FtileGeometry::with_out(w, h, w / 2.0, 0.0, h)
        } else {
            FtileGeometry::new(w, h, w / 2.0, 0.0)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if same_ftile(child, self.tile2.as_ref()) {
            return Some(self.tile2_translate(sb));
        }
        for (i, c) in self.couples.iter().enumerate() {
            for part in [&c.diamond, &c.tile] {
                if same_ftile(child, part.as_ref()) {
                    return Some(if same_ftile(child, c.tile.as_ref()) {
                        self.tile_translate(i, sb)
                    } else {
                        self.diamond_translate(i, sb)
                    });
                }
                if let Some(t) = part.translate_for(child, sb) {
                    let base = if same_ftile(part.as_ref(), c.tile.as_ref()) {
                        self.tile_translate(i, sb)
                    } else {
                        self.diamond_translate(i, sb)
                    };
                    return Some(t.compose(base));
                }
            }
        }
        self.tile2
            .translate_for(child, sb)
            .map(|t| t.compose(self.tile2_translate(sb)))
    }

    fn my_children(&self) -> Vec<FtileRef> {
        self.all_tiles().cloned().collect()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.couples[0].tile.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result: Vec<Rc<Swimlane>> = Vec::new();
        for t in self.all_tiles() {
            for lane in t.swimlanes() {
                if !result.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                    result.push(lane);
                }
            }
        }
        result
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_link_rendering.clone()
    }

    fn out_link_rendering(&self) -> LinkRendering {
        self.out_link_rendering.clone()
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        for (i, c) in self.couples.iter().enumerate() {
            let t = self.couple_translate(i, sb);
            let gc = g.apply_translate(t);
            // the couple's own margin, then the diamond over its tile
            let gm = gc.apply_translate(UTranslate::dx(c.margin));
            gm.draw_ftile(c.assembly.as_ref());
        }
        g.apply_translate(self.tile2_translate(sb))
            .draw_ftile(self.tile2.as_ref());

        let (total_w, total_h) = self.internal(sb);
        let total_left = total_w / 2.0;
        let arrows = Arrows::from_skin(&self.skin);
        let color = in_link_rendering_color(&self.skin, self.couples[0].tile.as_ref());
        let snake =
            |dir: Direction| Snake::create(&self.skin, color.clone(), Some(arrows.as_to(dir)));
        let n = self.couples.len();
        let mut nb_out = 0;

        for i in 0..n {
            let c = &self.couples[i];
            let dg = c.diamond.calculate_dimension(sb);
            let dt = self.diamond_translate(i, sb);
            let tg = c.tile.calculate_dimension(sb);
            let tt = self.tile_translate(i, sb);
            if tg.has_point_out() {
                nb_out += 1;
            }
            // ConnectionVerticalIn
            let mut s = snake(Direction::Down);
            s.add_point(dt.dx + dg.left(), dt.dy + dg.out_y);
            s.add_point(tt.dx + tg.left(), tt.dy + tg.in_y);
            g.draw_snake(s);
            // ConnectionVerticalOut
            if tg.has_point_out() {
                let x = tt.dx + tg.left();
                let mut s = snake(Direction::Down);
                s.add_point(x, tt.dy + tg.out_y);
                s.add_point(x, total_h);
                g.draw_snake(s);
            }
        }
        // ConnectionHorizontal between the conditions
        for i in 0..n.saturating_sub(1) {
            let g1 = self.couples[i].diamond.calculate_dimension(sb);
            let g2 = self.couples[i + 1].diamond.calculate_dimension(sb);
            let t1 = self.diamond_translate(i, sb);
            let t2 = self.diamond_translate(i + 1, sb);
            let mut s = snake(Direction::Right);
            s.add_point(t1.dx + g1.left() * 2.0, t1.dy + (g1.in_y + g1.out_y) / 2.0);
            s.add_point(t2.dx, t2.dy + (g2.in_y + g2.out_y) / 2.0);
            g.draw_snake(s);
        }
        // ConnectionIn
        {
            let dg = self.couples[0].diamond.calculate_dimension(sb);
            let dt = self.diamond_translate(0, sb);
            let (px, py) = (dt.dx + dg.left(), dt.dy + dg.in_y);
            let mut s = snake(Direction::Down);
            s.add_point(total_left, 0.0);
            s.add_point(px, 0.0);
            s.add_point(px, py);
            g.draw_snake(s);
        }
        // ConnectionLastElseIn
        let t2 = self.tile2_translate(sb);
        let g2 = self.tile2.calculate_dimension(sb);
        {
            let dg = self.couples[n - 1].diamond.calculate_dimension(sb);
            let dt = self.diamond_translate(n - 1, sb);
            let (x1, y1) = (dt.dx + dg.left() * 2.0, dt.dy + (dg.in_y + dg.out_y) / 2.0);
            let x2 = t2.dx + g2.left();
            let mut s = snake(Direction::Down);
            s.add_point(x1, y1);
            s.add_point(x2, y1);
            s.add_point(x2, t2.dy + g2.in_y);
            g.draw_snake(s);
        }
        // ConnectionLastElseOut
        if g2.has_point_out() {
            let x = t2.dx + g2.left();
            let mut s = snake(Direction::Down);
            s.add_point(x, t2.dy + g2.out_y);
            s.add_point(x, total_h);
            if nb_out == 0 {
                s.add_point(total_left, total_h);
            }
            g.draw_snake(s);
        }
        // ConnectionHline: joins the exits of the branches
        if nb_out > 0 && self.calculate_dimension(sb).has_point_out() {
            let (mut min_x, mut max_x) = (total_w / 2.0, total_w / 2.0);
            min_x = min_x.min(total_left);
            max_x = max_x.max(total_left);
            for (i, c) in self.couples.iter().enumerate() {
                let tg = c.tile.calculate_dimension(sb);
                if tg.has_point_out() {
                    let x = self.tile_translate(i, sb).dx + tg.left();
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
            if g2.has_point_out() {
                let x = t2.dx + g2.left();
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
            let mut s =
                Snake::create(&self.skin, color.clone(), None).with_merge(MergeStrategy::None);
            s.add_point(min_x, total_h);
            s.add_point(max_x, total_h);
            g.draw_snake(s);
        }
    }
}
