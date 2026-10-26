//! Parallel branches of a fork (`FtileForkInner`, `ParallelBuilderFork`, `FtileBlackBlock`).

use super::factory::{arrow_text_block, in_link_rendering_color, FtileRef};
use super::wrap::ConnectionVerticalDown;
use super::{Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::colors::Colors;
use crate::klimt::drawing::ugraphic::{URectangle, UTranslate};
use crate::klimt::font::StringBounder;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const BAR_HEIGHT: f64 = 6.0;
const BRANCH_MARGIN: f64 = 14.0;
const SPACE_AROUND_BAR: f64 = 20.0;

/// The parallel branches between two synchronization bars (`ParallelBuilderFork`).
pub struct FtileParallel {
    branches: Vec<FtileRef>,
    colors: Colors,
    skin: Rc<SkinParam>,
    in_link_rendering: crate::activitydiagram3::rainbow::LinkRendering,
}

impl FtileParallel {
    pub fn new(
        branches: Vec<FtileRef>,
        colors: Colors,
        skin: Rc<SkinParam>,
        in_link_rendering: crate::activitydiagram3::rainbow::LinkRendering,
    ) -> Rc<Self> {
        assert!(!branches.is_empty());
        Rc::new(Self {
            branches,
            colors,
            skin,
            in_link_rendering,
        })
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
        let height = BAR_HEIGHT + self.branch_height(sb) + BAR_HEIGHT;
        FtileGeometry::with_out(width, height, width / 2.0, 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        for (i, branch) in self.branches.iter().enumerate() {
            if super::same_ftile(child, branch.as_ref()) {
                let (dx, dy) = self.branch_content_offset(i, sb);
                return Some(UTranslate::new(dx, BAR_HEIGHT + dy));
            }
            if let Some(t) = branch.translate_for(child, sb) {
                let (dx, dy) = self.branch_content_offset(i, sb);
                return Some(t.compose(UTranslate::new(dx, BAR_HEIGHT + dy)));
            }
        }
        None
    }

    fn my_children(&self) -> Vec<FtileRef> {
        self.branches.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.branches.first().and_then(|b| b.swimlane_in())
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.branches.last().and_then(|b| b.swimlane_out())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        for branch in &self.branches {
            for lane in branch.swimlanes() {
                if !result.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                    result.push(lane);
                }
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
        for (i, branch) in self.branches.iter().enumerate() {
            let (dx, dy) = self.branch_content_offset(i, sb);
            let dim = branch.calculate_dimension(sb);
            let color = in_link_rendering_color(&self.skin, branch.as_ref());
            let x = dx + dim.left();
            let top_label = arrow_text_block(&self.skin, branch.in_link_rendering().display());
            let top = ConnectionVerticalDown::new(
                branch.clone(),
                branch.clone(),
                (x, 0.0),
                (x, BAR_HEIGHT + dy),
                color.clone(),
                top_label,
            );
            g.draw_connection(&top);
            let bottom_label = arrow_text_block(&self.skin, branch.out_link_rendering().display());
            let bottom = ConnectionVerticalDown::new(
                branch.clone(),
                branch.clone(),
                (x, BAR_HEIGHT + dy + dim.height),
                (x, BAR_HEIGHT + branch_height),
                color,
                bottom_label,
            );
            g.draw_connection(&bottom);
            g.apply_translate(UTranslate::new(dx, BAR_HEIGHT + dy))
                .draw_ftile(branch.as_ref());
        }
        let color = self.bar_color();
        for y in [0.0, BAR_HEIGHT + branch_height] {
            let mut rect = URectangle::build(width, BAR_HEIGHT).rounded(2.5);
            rect.delta_shadow = 0.0;
            g.ug()
                .apply_translate(UTranslate::new(0.0, y))
                .apply_color(color.clone())
                .apply_back_color(color.clone())
                .draw(rect);
        }
    }
}
