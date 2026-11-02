//! Parallel branches of a fork (`FtileForkInner`, `ParallelBuilderFork`, `FtileBlackBlock`).

use super::factory::{arrow_text_block, in_link_rendering_color, FtileRef};
use super::if_links::Merge;
use super::leaf::FtileEmpty;
use super::snake::{Arrows, Snake};
use super::{FnConnection, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::Rainbow;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::colors::Colors;
use crate::klimt::drawing::ugraphic::{ULine, URectangle, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::Direction;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const BAR_HEIGHT: f64 = 6.0;
/// Height of the thin line of a `split` (`FtileThinSplit`).
const THIN_HEIGHT: f64 = 1.5;
const MERGE_DIAMOND: f64 = 24.0;

/// What closes and opens the parallel branches: black bars (`fork`), thin lines (`split`) or a bar above and a
/// diamond below (`end merge`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelKind {
    Fork,
    Split,
    Merge,
}
const BRANCH_MARGIN: f64 = 14.0;
const SPACE_AROUND_BAR: f64 = 20.0;

/// The parallel branches between two synchronization bars (`ParallelBuilderFork`).
pub struct FtileParallel {
    kind: ParallelKind,
    branches: Vec<FtileRef>,
    colors: Colors,
    skin: Rc<SkinParam>,
    in_link_rendering: crate::activitydiagram3::rainbow::LinkRendering,
    /// The lanes of the bars above and under the branches.
    swimlane_in: Option<Rc<Swimlane>>,
    swimlane_out: Option<Rc<Swimlane>>,
}

impl FtileParallel {
    pub fn new(
        kind: ParallelKind,
        branches: Vec<FtileRef>,
        colors: Colors,
        skin: Rc<SkinParam>,
        in_link_rendering: crate::activitydiagram3::rainbow::LinkRendering,
        swimlane_in: Option<Rc<Swimlane>>,
        swimlane_out: Option<Rc<Swimlane>>,
    ) -> Rc<Self> {
        assert!(!branches.is_empty());
        Rc::new(Self {
            kind,
            branches,
            colors,
            skin,
            in_link_rendering,
            swimlane_in,
            swimlane_out,
        })
    }

    fn top_height(&self) -> f64 {
        if self.kind == ParallelKind::Split {
            THIN_HEIGHT
        } else {
            BAR_HEIGHT
        }
    }

    fn bottom_height(&self) -> f64 {
        match self.kind {
            ParallelKind::Fork => BAR_HEIGHT,
            ParallelKind::Split => THIN_HEIGHT,
            ParallelKind::Merge => MERGE_DIAMOND,
        }
    }

    fn has_out(&self, sb: &dyn StringBounder) -> bool {
        self.branches
            .iter()
            .any(|b| b.calculate_dimension(sb).has_point_out())
    }

    /// Extra width for an arrow label wider than the branch (`getSuppForIncomingArrow`).
    fn branch_extra(&self, i: usize, sb: &dyn StringBounder) -> f64 {
        let dim = self.branches[i].calculate_dimension(sb);
        let mut extra = 0.0_f64;
        for display in [
            self.branches[i].in_link_rendering().display(),
            self.branches[i].out_link_rendering().display(),
        ] {
            if let Some(block) = arrow_text_block(&self.skin, display) {
                let width = block.calculate_dimension(sb).width;
                extra = extra.max(dim.left() + width - dim.width);
            }
        }
        extra.max(0.0)
    }

    /// Width of a branch once its horizontal margin is added.
    fn branch_width(&self, i: usize, sb: &dyn StringBounder) -> f64 {
        self.branches[i].calculate_dimension(sb).width
            + 2.0 * BRANCH_MARGIN
            + self.branch_extra(i, sb)
    }

    fn x_offset(&self, i: usize, sb: &dyn StringBounder) -> f64 {
        (0..i).map(|k| self.branch_width(k, sb)).sum::<f64>()
    }

    fn max_height(&self, sb: &dyn StringBounder) -> f64 {
        self.branches
            .iter()
            .map(|b| b.calculate_dimension(sb).height)
            .fold(0.0, f64::max)
    }

    fn supp_top(&self, sb: &dyn StringBounder) -> f64 {
        (0..self.branches.len())
            .map(|i| {
                let rendering = self.branches[i].in_link_rendering();
                let display = rendering.display();
                arrow_text_block(&self.skin, display)
                    .map(|b| b.calculate_dimension(sb).height)
                    .unwrap_or(0.0)
            })
            .fold(0.0, f64::max)
    }

    fn supp_bottom(&self, sb: &dyn StringBounder) -> f64 {
        (0..self.branches.len())
            .map(|i| {
                let rendering = self.branches[i].out_link_rendering();
                let display = rendering.display();
                arrow_text_block(&self.skin, display)
                    .map(|b| b.calculate_dimension(sb).height)
                    .unwrap_or(0.0)
            })
            .fold(0.0, f64::max)
    }

    fn branch_height(&self, sb: &dyn StringBounder) -> f64 {
        self.max_height(sb) + 2.0 * SPACE_AROUND_BAR + self.supp_top(sb) + self.supp_bottom(sb)
    }

    fn bar_width(&self, sb: &dyn StringBounder) -> f64 {
        (0..self.branches.len())
            .map(|i| self.branch_width(i, sb))
            .sum()
    }

    fn bar_color(&self) -> crate::klimt::color::HColor {
        let style = self
            .skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ACTIVITYBAR)
            .expect("activity bar style");
        self.colors.color_for(&style, PName::BackGroundColor)
    }

    fn branch_content_offset(&self, i: usize, sb: &dyn StringBounder) -> (f64, f64) {
        let dim = self.branches[i].calculate_dimension(sb);
        let dy = self.supp_top(sb) + SPACE_AROUND_BAR + (self.max_height(sb) - dim.height) / 2.0;
        (self.x_offset(i, sb) + BRANCH_MARGIN, dy)
    }
}

impl Ftile for FtileParallel {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let width = self.bar_width(sb);
        let height = self.top_height() + self.branch_height(sb) + self.bottom_height();
        if self.kind == ParallelKind::Split && !self.has_out(sb) {
            // a `split` whose branches all end has no way out (`FtileKilled`)
            return FtileGeometry::new(width, height, width / 2.0, 0.0);
        }
        FtileGeometry::with_out(width, height, width / 2.0, 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        for (i, branch) in self.branches.iter().enumerate() {
            if super::same_ftile(child, branch.as_ref()) {
                let (dx, dy) = self.branch_content_offset(i, sb);
                return Some(UTranslate::new(dx, self.top_height() + dy));
            }
            if let Some(t) = branch.translate_for(child, sb) {
                let (dx, dy) = self.branch_content_offset(i, sb);
                return Some(t.compose(UTranslate::new(dx, self.top_height() + dy)));
            }
        }
        None
    }

    fn my_children(&self) -> Vec<FtileRef> {
        self.branches.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_in.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane_out.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        let ends = self
            .swimlane_in
            .iter()
            .chain(self.swimlane_out.iter())
            .cloned();
        for lane in self.branches.iter().flat_map(|b| b.swimlanes()).chain(ends) {
            if !result.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                result.push(lane);
            }
        }
        result
    }

    fn in_link_rendering(&self) -> crate::activitydiagram3::rainbow::LinkRendering {
        self.in_link_rendering.clone()
    }

    fn out_link_rendering(&self) -> crate::activitydiagram3::rainbow::LinkRendering {
        self.branches
            .last()
            .map_or_else(crate::activitydiagram3::rainbow::LinkRendering::none, |b| {
                b.out_link_rendering()
            })
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let width = self.bar_width(sb);
        let branch_height = self.branch_height(sb);
        let top_h = self.top_height();
        // the bars stand in for their tiles in the arrows (`FtileBlackBlock`)
        let bar_in: FtileRef = FtileEmpty::new(&self.skin, 0.0, 0.0, self.swimlane_in.clone());
        let bar_out: FtileRef = FtileEmpty::new(&self.skin, 0.0, 0.0, self.swimlane_out.clone());
        let arrows = Arrows::from_skin(&self.skin);
        let down = arrows.as_to(Direction::Down);
        let bar_color = self.bar_color();
        let draw_bar = |y: f64, lane: &Option<Rc<Swimlane>>| {
            if !g.accepts(lane) {
                return;
            }
            let mut rect = URectangle::build(width, BAR_HEIGHT)
                .rounded(5.0)
                .ignore_for_compression_on_x();
            rect.delta_shadow = 0.0;
            g.ug()
                .apply_translate(UTranslate::new(0.0, y))
                .apply_color(bar_color.clone())
                .apply_back_color(bar_color.clone())
                .draw(rect);
        };
        let center = width / 2.0;
        // the thin lines of a `split` span the branch columns, up to the centre (`FtileThinSplit.setGeom`)
        let columns: Vec<(f64, bool, Rainbow)> = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let (dx, _) = self.branch_content_offset(i, sb);
                let dim = b.calculate_dimension(sb);
                (
                    dx + dim.left(),
                    dim.has_point_out(),
                    in_link_rendering_color(&self.skin, b.as_ref()),
                )
            })
            .collect();
        let draw_thin = |y: f64, lane: &Option<Rc<Swimlane>>, only_out: bool| {
            if !g.accepts(lane) {
                return;
            }
            let xs: Vec<f64> = columns
                .iter()
                .filter(|c| !only_out || c.1)
                .map(|c| c.0)
                .collect();
            let (Some(first), Some(last)) = (xs.first().copied(), xs.last().copied()) else {
                return;
            };
            let (first, last) = (first.min(center), last.max(center));
            let color = columns[0].2.colors().first().map(|c| c.arrow_color.clone());
            let Some(color) = color else { return };
            g.ug()
                .apply_translate(UTranslate::new(first, y))
                .apply_color(color)
                .apply_stroke(UStroke::with_thickness(1.5))
                .draw(ULine::new(last - first, 0.0));
        };
        if self.kind == ParallelKind::Split {
            draw_thin(0.0, &self.swimlane_in, false);
        } else {
            draw_bar(0.0, &self.swimlane_in);
        }
        let diamond_y = top_h + branch_height;
        let diamond = Merge::new(
            None,
            None,
            None,
            {
                let style = self
                    .skin
                    .style_builder()
                    .merged_style(&queries::ACTIVITYDIAG_ACTIVITY)
                    .expect("activity style");
                style.value(PName::BackGroundColor).as_color()
            },
            self.skin.clone(),
            self.swimlane_out.clone(),
        );
        let mut outs: Vec<FnConnection> = Vec::new();
        for (i, branch) in self.branches.iter().enumerate() {
            let (dx, dy) = self.branch_content_offset(i, sb);
            let dim = branch.calculate_dimension(sb);
            let color = in_link_rendering_color(&self.skin, branch.as_ref());
            let x = dx + dim.left();
            let (y_in, y_out_a, y_out_b) =
                (top_h + dy, top_h + dy + dim.height, top_h + branch_height);
            // ConnectionIn
            let top_label = arrow_text_block(&self.skin, branch.in_link_rendering().display());
            let align = self.arrow_horizontal_alignment();
            let (skin, c, d, l) = (
                self.skin.clone(),
                color.clone(),
                down.clone(),
                top_label.clone(),
            );
            let top = FnConnection::new(Some(bar_in.clone()), Some(branch.clone()), move |g| {
                let mut s =
                    Snake::create(&skin, c.clone(), Some(d.clone())).with_label(l.clone(), align);
                s.add_point(x, top_h);
                s.add_point(x, y_in);
                g.draw_snake(s);
            })
            .translatable({
                let (skin, c, d) = (self.skin.clone(), color.clone(), down.clone());
                move |g, t1, t2| {
                    let (mp1, mp2) = ((x + t1.dx, top_h + t1.dy), (x + t2.dx, y_in + t2.dy));
                    let middle = mp1.1 + 4.0;
                    let mut s = Snake::create(&skin, c.clone(), Some(d.clone()))
                        .ignore_for_compression()
                        .with_label(top_label.clone(), align);
                    s.add_point(mp1.0, mp1.1);
                    s.add_point(mp1.0, middle);
                    s.add_point(mp2.0, middle);
                    s.add_point(mp2.0, mp2.1);
                    g.draw_snake(s);
                }
            });
            g.draw_connection(&top);
            // ConnectionOut
            if dim.has_point_out() {
                let bottom_label =
                    arrow_text_block(&self.skin, branch.out_link_rendering().display());
                let (skin, c, d, l) = (
                    self.skin.clone(),
                    color.clone(),
                    down.clone(),
                    bottom_label.clone(),
                );
                if self.kind == ParallelKind::Merge {
                    // `ConnectionHorizontalThenVertical`: into the left, top or right vertex of the diamond
                    let (skin, c) = (self.skin.clone(), color.clone());
                    let arrows = Arrows::from_skin(&self.skin);
                    let (cx, half) = (center, MERGE_DIAMOND / 2.0);
                    let (x1, y1) = (x, y_out_a);
                    let (target, dir) = if x1 < cx - half {
                        ((cx - half, diamond_y + half), Direction::Right)
                    } else if x1 > cx + half {
                        ((cx + half, diamond_y + half), Direction::Left)
                    } else {
                        ((cx, diamond_y), Direction::Down)
                    };
                    let mut s = Snake::create(&skin, c, Some(arrows.as_to(dir)));
                    s.add_point(x1, y1);
                    s.add_point(x1, target.1);
                    s.add_point(target.0, target.1);
                    g.draw_snake(s);
                    g.apply_translate(UTranslate::new(dx, top_h + dy))
                        .draw_ftile(branch.as_ref());
                    continue;
                }
                let bottom =
                    FnConnection::new(Some(branch.clone()), Some(bar_out.clone()), move |g| {
                        let mut s = Snake::create(&skin, c.clone(), Some(d.clone()))
                            .with_label(l.clone(), align);
                        s.add_point(x, y_out_a);
                        s.add_point(x, y_out_b);
                        g.draw_snake(s);
                    })
                    .translatable({
                        let (skin, c, d) = (self.skin.clone(), color.clone(), down.clone());
                        move |g, t1, t2| {
                            let (mp1, mp2) =
                                ((x + t1.dx, y_out_a + t1.dy), (x + t2.dx, y_out_b + t2.dy));
                            let middle = mp2.1 - 14.0;
                            let mut s = Snake::create(&skin, c.clone(), Some(d.clone()))
                                .ignore_for_compression()
                                .with_label(bottom_label.clone(), align);
                            s.add_point(mp1.0, mp1.1);
                            s.add_point(mp1.0, middle);
                            s.add_point(mp2.0, middle);
                            s.add_point(mp2.0, mp2.1);
                            g.draw_snake(s);
                        }
                    });
                outs.push(bottom);
            }
            g.apply_translate(UTranslate::new(dx, top_h + dy))
                .draw_ftile(branch.as_ref());
        }
        match self.kind {
            ParallelKind::Fork => draw_bar(top_h + branch_height, &self.swimlane_out),
            ParallelKind::Split => draw_thin(top_h + branch_height, &self.swimlane_out, true),
            ParallelKind::Merge => {
                g.apply_translate(UTranslate::new(center - MERGE_DIAMOND / 2.0, diamond_y))
                    .draw_ftile(&diamond);
            }
        }
        for bottom in &outs {
            g.draw_connection(bottom);
        }
    }
}
