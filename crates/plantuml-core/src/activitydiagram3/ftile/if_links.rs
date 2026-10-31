//! The general two-branch `if`: a condition, two branches side by side and a merge diamond
//! (`ConditionalBuilder.createWithLinks`, `FtileIfNude`, `FtileIfWithDiamonds`, `FtileIfWithLinks`).

use super::factory::{
    arrow_text_block, in_link_rendering_color, out_link_rendering_color, FtileRef,
};
use super::if_long::{DiamondInside2, MinWidthCentered};
use super::leaf::FtileEmpty;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::{same_ftile, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::instruction::PositionedNote;
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UPolygon, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::Direction;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use crate::svek::image_note::Opale;
use std::cell::Cell;
use std::rc::Rc;

const SUPP_WIDTH: f64 = 20.0;
const HEXAGON_HALF_SIZE: f64 = 12.0;

/// A branch of a conditional (`Branch`).
pub struct CondBranch {
    pub tile: FtileRef,
    pub label_test: Display,
    pub label_positive: Option<Display>,
    pub color: Option<HColor>,
    /// The branch has no instruction at all (`Branch.isEmpty`).
    pub empty: bool,
}

/// Adds room around a tile (`FtileMarged`).
pub(crate) struct Marged {
    pub(crate) inner: FtileRef,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) top: f64,
    pub(crate) bottom: f64,
}

impl Marged {
    pub(crate) fn horizontal(inner: FtileRef, left: f64, right: f64) -> FtileRef {
        Rc::new(Self {
            inner,
            left,
            right,
            top: 0.0,
            bottom: 0.0,
        })
    }

    pub(crate) fn vertical(inner: FtileRef, top: f64, bottom: f64) -> FtileRef {
        Rc::new(Self {
            inner,
            left: 0.0,
            right: 0.0,
            top,
            bottom,
        })
    }
}

impl Ftile for Marged {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let g = self.inner.calculate_dimension(sb);
        let (w, h, left) = (
            g.width + self.left + self.right,
            g.height + self.top + self.bottom,
            g.left() + self.left,
        );
        if g.has_point_out() {
            FtileGeometry::with_out(w, h, left, g.in_y + self.top, g.out_y + self.top)
        } else {
            FtileGeometry::new(w, h, left, g.in_y + self.top)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let t = UTranslate::new(self.left, self.top);
        if same_ftile(child, self.inner.as_ref()) {
            return Some(t);
        }
        self.inner.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn draw(&self, g: &FtileGraphic) {
        g.apply_translate(UTranslate::new(self.left, self.top))
            .draw_ftile(self.inner.as_ref());
    }
}

/// The empty diamond closing a conditional (`FtileDiamond`).
pub(crate) struct Merge {
    pub(crate) north: Option<Rc<dyn TextBlock>>,
    west: Option<Rc<dyn TextBlock>>,
    east: Option<Rc<dyn TextBlock>>,
    back: HColor,
    skin: Rc<SkinParam>,
}

impl Merge {
    pub(crate) fn new(
        north: Option<Rc<dyn TextBlock>>,
        west: Option<Rc<dyn TextBlock>>,
        east: Option<Rc<dyn TextBlock>>,
        back: HColor,
        skin: Rc<SkinParam>,
    ) -> Self {
        Self {
            north,
            west,
            east,
            back,
            skin,
        }
    }

    pub(crate) fn label_height(&self, sb: &dyn StringBounder) -> f64 {
        let h = |b: &Option<Rc<dyn TextBlock>>| {
            b.as_ref().map_or(0.0, |b| b.calculate_dimension(sb).height)
        };
        h(&self.east).max(h(&self.west))
    }
}

impl Ftile for Merge {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let side = 2.0 * HEXAGON_HALF_SIZE;
        let north = self
            .north
            .as_ref()
            .map_or(0.0, |n| n.calculate_dimension(sb).height);
        FtileGeometry::with_out(side, side + north, side / 2.0, north, side + north)
    }

    fn draw(&self, g: &FtileGraphic) {
        let style = self
            .skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ACTIVITY_DIAMOND)
            .expect("diamond style");
        let side = 2.0 * HEXAGON_HALF_SIZE;
        let mut polygon = UPolygon::new(vec![
            (side / 2.0, 0.0),
            (side, side / 2.0),
            (side / 2.0, side),
            (0.0, side / 2.0),
        ]);
        polygon.delta_shadow = 0.0;
        let sb = g.string_bounder();
        let north_height = self
            .north
            .as_ref()
            .map_or(0.0, |n| n.calculate_dimension(sb).height);
        let g = &g.apply_translate(UTranslate::dy(north_height));
        g.ug()
            .apply_color(style.value(PName::LineColor).as_color())
            .apply_back_color(self.back.clone())
            .apply_stroke(style.stroke())
            .draw(polygon);
        if let Some(north) = &self.north {
            north.draw(
                &g.ug()
                    .apply_translate(UTranslate::new(HEXAGON_HALF_SIZE * 1.5, -north_height)),
            );
        }
        if let Some(west) = &self.west {
            let d = west.calculate_dimension(sb);
            west.draw(
                &g.ug()
                    .apply_translate(UTranslate::new(-d.width, -d.height + HEXAGON_HALF_SIZE)),
            );
        }
        if let Some(east) = &self.east {
            let d = east.calculate_dimension(sb);
            east.draw(&g.ug().apply_translate(UTranslate::new(
                2.0 * HEXAGON_HALF_SIZE,
                -d.height + HEXAGON_HALF_SIZE,
            )));
        }
    }
}

/// The conditional itself (`FtileIfWithLinks`).
pub struct FtileIfLinks {
    opale_left: Option<Opale>,
    opale_right: Option<Opale>,
    x_delta_note: Cell<f64>,
    y_delta_note: Cell<f64>,
    supp_width_note: Cell<f64>,
    diamond1: FtileRef,
    diamond2: FtileRef,
    merge_label_height: f64,
    tile1: FtileRef,
    tile2: FtileRef,
    empty1: bool,
    empty2: bool,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileIfLinks {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        branch1: CondBranch,
        branch2: CondBranch,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        sb: &dyn StringBounder,
        notes: &[PositionedNote],
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> FtileRef {
        let block = |d: &Option<Display>| d.as_ref().and_then(|d| arrow_text_block(&skin, d));
        let (label1, label2) = (
            block(&branch1.label_positive),
            block(&branch2.label_positive),
        );
        let color = branch1.color.clone().or_else(|| back.clone());
        let min1: FtileRef = Rc::new(MinWidthCentered {
            inner: branch1.tile.clone(),
            min_width: 30.0,
        });
        let min2: FtileRef = Rc::new(MinWidthCentered {
            inner: branch2.tile.clone(),
            min_width: 30.0,
        });
        let two_branches = min1.calculate_dimension(sb).has_point_out()
            && min2.calculate_dimension(sb).has_point_out();
        let diamond1: FtileRef = Rc::new(DiamondInside2 {
            diamond: Diamond::new(branch1.label_test.clone(), &skin, color.clone()),
            north: None,
            south: None,
            west: label1.clone(),
            east: label2.clone(),
            skin: skin.clone(),
            north_widens: false,
        });
        let (diamond2, merge_label_height): (FtileRef, f64) = if two_branches {
            let out =
                |b: &CondBranch| arrow_text_block(&skin, b.tile.out_link_rendering().display());
            let merge = Merge {
                north: None,
                west: out(&branch1),
                east: out(&branch2),
                back: branch2
                    .color
                    .clone()
                    .or(color)
                    .unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back()),
                skin: skin.clone(),
            };
            let h = merge.label_height(sb);
            (Rc::new(merge), h)
        } else {
            (
                FtileEmpty::new(&skin, 0.0, HEXAGON_HALF_SIZE / 2.0, None),
                0.0,
            )
        };
        let tile1 = Marged::horizontal(min1, 10.0, 10.0);
        let tile2 = Marged::horizontal(min2, 10.0, 10.0);
        let mut opale_left = None;
        let mut opale_right = None;
        for note in notes {
            if note.left && opale_left.is_none() {
                opale_left = Some(super::note::opale_of(note, &skin));
            } else if !note.left && opale_right.is_none() {
                opale_right = Some(super::note::opale_of(note, &skin));
            }
        }
        let core = Rc::new(Self {
            opale_left,
            opale_right,
            x_delta_note: Cell::new(0.0),
            y_delta_note: Cell::new(0.0),
            supp_width_note: Cell::new(0.0),
            diamond1,
            diamond2,
            merge_label_height,
            tile1,
            tile2,
            empty1: branch1.empty,
            empty2: branch2.empty,
            skin,
            in_link_rendering,
            out_link_rendering,
        });
        // room for the notes beside the condition (`FtileIfWithDiamonds` constructor)
        if let Some(left) = &core.opale_left {
            let d = left.dimension(sb);
            let pos1 = core.translate_diamond1(sb).dx;
            if d.width > pos1 {
                core.x_delta_note.set(d.width - pos1);
            }
            core.y_delta_note.set(core.y_delta_note.get().max(d.height));
        }
        if let Some(right) = &core.opale_right {
            let d = right.dimension(sb);
            let d1 = core.diamond1.calculate_dimension(sb);
            let pos1 = core.translate_diamond1(sb).dx + d1.width + d.width;
            let pos2 = core.internal(sb).width;
            if pos1 > pos2 {
                core.supp_width_note.set(pos1 - pos2);
            }
            core.y_delta_note.set(core.y_delta_note.get().max(d.height));
        }
        // margins for the labels beside the condition and for tall labels (`createWithLinks`)
        let width = |b: &Option<Rc<dyn TextBlock>>| {
            b.as_ref().map_or((0.0, 0.0), |b| {
                let d = b.calculate_dimension(sb);
                (d.width, d.height)
            })
        };
        let (l1, l2) = (width(&label1), width(&label2));
        let d1 = core.diamond1.calculate_dimension(sb);
        let dx = core.translate_diamond1(sb).dx;
        let diff1 = (l1.0 - dx).max(0.0);
        let needed = dx + d1.width + l2.0;
        let diff2 = (needed - core.calculate_dimension(sb).width).max(0.0);
        let supp = (l1.1.max(l2.1) - d1.height).max(0.0);
        let result: FtileRef = core;
        Marged::vertical(Marged::horizontal(result, diff1, diff2), supp, 0.0)
    }

    fn two_branches(&self, sb: &dyn StringBounder) -> bool {
        self.tile1.calculate_dimension(sb).has_point_out()
            && self.tile2.calculate_dimension(sb).has_point_out()
    }

    fn y_delta_1a(&self) -> f64 {
        if self.swimlanes().len() > 1 {
            20.0
        } else {
            10.0
        }
    }

    fn y_delta_1b(&self, sb: &dyn StringBounder) -> f64 {
        if self.swimlanes().len() > 1 {
            10.0
        } else if self.two_branches(sb) {
            6.0
        } else {
            0.0
        }
    }

    /// `calculateDimensionInternalSlow` of the diamonds tile: no exit point removed yet.
    fn internal(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let (t1, t2) = (
            self.tile1.calculate_dimension(sb),
            self.tile2.calculate_dimension(sb),
        );
        let (xn, yn, sw) = (
            self.x_delta_note.get(),
            self.y_delta_note.get(),
            self.supp_width_note.get(),
        );
        let inner_margin = ((t1.width - t1.left()) + t2.left()).max(d1.width + SUPP_WIDTH);
        let width = xn + t1.left() + inner_margin + (t2.width - t2.left()) + sw;
        let height = yn + t1.height.max(t2.height);
        let nude = FtileGeometry::with_out(
            width,
            height,
            xn + t1.left() + inner_margin / 2.0,
            yn,
            height,
        );
        let all = d1.append_bottom(&nude).append_bottom(&d2);
        let delta = self.y_delta_1a() + self.y_delta_1b(sb) + self.merge_label_height;
        if all.has_point_out() {
            FtileGeometry::with_out(
                all.width,
                all.height + delta,
                all.left(),
                all.in_y + yn,
                all.out_y + delta,
            )
        } else {
            FtileGeometry::new(all.width, all.height + delta, all.left(), all.in_y + yn)
        }
    }

    fn translate_diamond1(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.internal(sb);
        UTranslate::new(
            total.left() - self.diamond1.calculate_dimension(sb).left(),
            self.y_delta_note.get(),
        )
    }

    fn translate_diamond2(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.internal(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        UTranslate::new(total.left() - d2.width / 2.0, total.height - d2.height)
    }

    fn branch_y(&self, sb: &dyn StringBounder) -> f64 {
        self.diamond1.calculate_dimension(sb).height + self.y_delta_1a()
    }

    fn translate_branch1(&self, sb: &dyn StringBounder) -> UTranslate {
        UTranslate::new(
            self.x_delta_note.get(),
            self.y_delta_note.get() + self.branch_y(sb),
        )
    }

    fn translate_branch2(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.internal(sb);
        let t2 = self.tile2.calculate_dimension(sb);
        UTranslate::new(
            total.width - t2.width - self.supp_width_note.get(),
            self.y_delta_note.get() + self.branch_y(sb),
        )
    }
}

impl Ftile for FtileIfLinks {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let total = self.internal(sb);
        let out = self.tile1.calculate_dimension(sb).has_point_out()
            || self.tile2.calculate_dimension(sb).has_point_out();
        if out {
            total
        } else {
            total.without_point_out()
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let parts: [(&FtileRef, UTranslate); 4] = [
            (&self.diamond1, self.translate_diamond1(sb)),
            (&self.tile1, self.translate_branch1(sb)),
            (&self.tile2, self.translate_branch2(sb)),
            (&self.diamond2, self.translate_diamond2(sb)),
        ];
        for (part, t) in parts {
            if same_ftile(child, part.as_ref()) {
                return Some(t);
            }
            if let Some(c) = part.translate_for(child, sb) {
                return Some(c.compose(t));
            }
        }
        None
    }

    fn my_children(&self) -> Vec<FtileRef> {
        vec![
            self.diamond1.clone(),
            self.diamond2.clone(),
            self.tile1.clone(),
            self.tile2.clone(),
        ]
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.tile1.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes: Vec<Rc<Swimlane>> = Vec::new();
        for part in [&self.tile1, &self.tile2] {
            for lane in part.swimlanes() {
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
        let (d1t, d2t) = (self.translate_diamond1(sb), self.translate_diamond2(sb));
        let (b1t, b2t) = (self.translate_branch1(sb), self.translate_branch2(sb));
        let d1_geo = self.diamond1.calculate_dimension(sb);
        if let Some(left) = &self.opale_left {
            let x = d1t.dx - left.dimension(sb).width;
            left.draw(&g.ug().apply_translate(UTranslate::dx(x)), None);
        }
        if let Some(right) = &self.opale_right {
            right.draw(
                &g.ug()
                    .apply_translate(UTranslate::dx(d1t.dx + d1_geo.width)),
                None,
            );
        }
        g.apply_translate(d1t).draw_ftile(self.diamond1.as_ref());
        g.apply_translate(b1t).draw_ftile(self.tile1.as_ref());
        g.apply_translate(b2t).draw_ftile(self.tile2.as_ref());
        g.apply_translate(d2t).draw_ftile(self.diamond2.as_ref());

        let arrows = Arrows::from_skin(&self.skin);
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let total = self.internal(sb);
        let mid1 = (d1.in_y + d1.out_y) / 2.0;
        let mid2 = (d2.in_y + d2.out_y) / 2.0;
        let branches = [
            (&self.tile1, b1t, self.empty1, 0.0, 0.0),
            (&self.tile2, b2t, self.empty2, d1.width, d2.width),
        ];

        // ConnectionHorizontalThenVertical: from the condition's side vertices to the branches
        for (tile, t, empty, x1, _) in &branches {
            let color = in_link_rendering_color(&self.skin, tile.as_ref());
            let tg = tile.calculate_dimension(sb);
            let end = if *empty {
                None
            } else {
                Some(arrows.as_to(Direction::Down))
            };
            let mut s = Snake::create(&self.skin, color, end);
            let (px, py) = (t.dx + tg.left(), t.dy + tg.in_y);
            s.add_point(d1t.dx + x1, d1t.dy + mid1);
            s.add_point(px, d1t.dy + mid1);
            s.add_point(px, py);
            g.draw_snake(s);
        }

        // the exits: to the merge diamond, or straight down when only one branch leaves
        let outs: Vec<bool> = branches
            .iter()
            .map(|(tile, ..)| tile.calculate_dimension(sb).has_point_out())
            .collect();
        for (tile, t, empty, _, x2) in &branches {
            let tg = tile.calculate_dimension(sb);
            if !tg.has_point_out() {
                continue;
            }
            let color = out_link_rendering_color(&self.skin, tile.as_ref());
            let (x1, y1) = (t.dx + tg.left(), t.dy + tg.out_y);
            if outs[0] && outs[1] {
                let (x2, y2) = (d2t.dx + x2, d2t.dy + mid2);
                let arrow = if x2 > x1 {
                    Direction::Right
                } else {
                    Direction::Left
                };
                let mut s = Snake::create(&self.skin, color, Some(arrows.as_to(arrow)));
                if *empty {
                    s = s.emphasize_direction(Direction::Down);
                }
                s.add_point(x1, y1);
                s.add_point(x1, y2);
                s.add_point(x2, y2);
                g.draw_snake(s);
            } else {
                let mut s = Snake::create(&self.skin, color, None);
                if *empty {
                    s = s.emphasize_direction(Direction::Down);
                }
                s.add_point(x1, y1);
                s.add_point(x1, total.height);
                s.add_point(total.left(), total.height);
                s.add_point(total.left(), total.height);
                g.draw_snake(s);
            }
        }
    }
}
