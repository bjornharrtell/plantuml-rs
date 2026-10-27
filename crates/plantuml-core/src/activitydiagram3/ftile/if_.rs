//! An `if` with its branches (`FtileIf`).

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
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use std::rc::Rc;

const MERGE: f64 = 24.0;

/// Geometry of a conditional tile (`if` and `switch` differ slightly).
#[derive(Clone, Copy)]
pub struct ConditionalLayout {
    pub branch_gap: f64,
    pub branch_space: f64,
    pub merge_gap: f64,
    pub labels_above: bool,
}

impl ConditionalLayout {
    pub fn if_() -> Self {
        Self {
            branch_gap: 40.0,
            branch_space: 10.0,
            merge_gap: 6.0,
            labels_above: true,
        }
    }

    pub fn switch() -> Self {
        Self {
            branch_gap: 50.0,
            branch_space: 40.556,
            merge_gap: 10.0,
            labels_above: false,
        }
    }
}

pub struct Branch {
    pub tile: FtileRef,
    pub label: Option<Rc<dyn TextBlock>>,
}

/// A condition with branches that join in a merge diamond (`FtileIf`).
pub struct FtileIf {
    condition: Diamond,
    branches: Vec<Branch>,
    layout: ConditionalLayout,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileIf {
    pub fn new(
        condition: Display,
        branches: Vec<Branch>,
        layout: ConditionalLayout,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> Rc<Self> {
        Rc::new(Self {
            condition: Diamond::new(condition, &skin, back),
            branches,
            layout,
            skin,
            in_link_rendering,
            out_link_rendering,
        })
    }

    fn branch_width(&self, i: usize, sb: &dyn StringBounder) -> f64 {
        self.branches[i].tile.calculate_dimension(sb).width
    }

    fn width(&self, sb: &dyn StringBounder) -> f64 {
        let n = self.branches.len();
        let widths: f64 = (0..n).map(|i| self.branch_width(i, sb)).sum();
        let diamond = self.condition.width(sb);
        (widths + self.layout.branch_gap * (n.saturating_sub(1)) as f64).max(diamond)
    }

    fn branch_offset(&self, i: usize, sb: &dyn StringBounder) -> (f64, f64) {
        let mut x = 0.0;
        for k in 0..i {
            x += self.branch_width(k, sb) + self.layout.branch_gap;
        }
        (x, self.condition.height + self.layout.branch_space)
    }

    fn branch_y(&self, sb: &dyn StringBounder) -> f64 {
        self.branch_offset(0, sb).1
    }

    fn branch_height(&self, sb: &dyn StringBounder) -> f64 {
        self.branches
            .iter()
            .map(|b| b.tile.calculate_dimension(sb).height)
            .fold(0.0, f64::max)
    }

    /// Where the condition and the merge diamond sit: the middle of the branch centres, which is
    /// where the Graphviz output puts them when the branches have different widths (`dimTotal.getLeft`).
    fn center(&self, sb: &dyn StringBounder) -> f64 {
        let first = self.branch_offset(0, sb).0 + self.branch_width(0, sb) / 2.0;
        let last = self.branch_offset(self.branches.len() - 1, sb).0
            + self.branch_width(self.branches.len() - 1, sb) / 2.0;
        (first + last) / 2.0
    }

    fn diamond_x(&self, sb: &dyn StringBounder) -> f64 {
        self.center(sb) - self.condition.width(sb) / 2.0
    }

    fn merge_offset(&self, sb: &dyn StringBounder) -> (f64, f64) {
        (
            self.center(sb) - MERGE / 2.0,
            self.branch_y(sb) + self.branch_height(sb) + self.layout.merge_gap,
        )
    }

    fn height(&self, sb: &dyn StringBounder) -> f64 {
        self.merge_offset(sb).1 + MERGE
    }
}

impl Ftile for FtileIf {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let width = self.width(sb);
        let height = self.height(sb);
        FtileGeometry::with_out(width, height, self.center(sb), 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        for i in 0..self.branches.len() {
            let (x, y) = self.branch_offset(i, sb);
            let tile = &self.branches[i].tile;
            if super::same_ftile(child, tile.as_ref()) {
                return Some(UTranslate::new(x, y));
            }
            if let Some(t) = tile.translate_for(child, sb) {
                return Some(t.compose(UTranslate::new(x, y)));
            }
        }
        None
    }

    fn my_children(&self) -> Vec<FtileRef> {
        self.branches.iter().map(|b| b.tile.clone()).collect()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.branches.first().and_then(|b| b.tile.swimlane_in())
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.branches.last().and_then(|b| b.tile.swimlane_out())
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut result = Vec::new();
        for branch in &self.branches {
            for lane in branch.tile.swimlanes() {
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
        let dw = self.condition.width(sb);
        let dx = self.diamond_x(sb);
        let middle = self.condition.height / 2.0;
        let (mx, my) = self.merge_offset(sb);
        let merge_y = my + MERGE / 2.0;
        let color = crate::activitydiagram3::ftile::factory::in_link_rendering_color(
            &self.skin,
            self.branches[0].tile.as_ref(),
        );
        let arrows = Arrows::from_skin(&self.skin);
        let n = self.branches.len();

        // Shapes first, connections last (as Java draws them): a branch label sits at the vertex of
        // the condition but must stay readable, and the merge diamond must cover neither.
        self.condition.draw(g, dx, 0.0, dw);

        for i in 0..n {
            let Some(label) = &self.branches[i].label else {
                continue;
            };
            let (bx, by) = self.branch_offset(i, sb);
            let center = bx + self.branch_width(i, sb) / 2.0;
            let dim = label.calculate_dimension(sb);
            let y = if self.layout.labels_above {
                (middle - dim.height - 2.0).max(0.0)
            } else {
                (self.condition.height + by - dim.height) / 2.0
            };
            // Java right-aligns the first label on the left vertex and left-aligns the last one on
            // the right vertex; a middle branch is centred over its connection.
            let x = if i == 0 {
                dx - dim.width
            } else if i == n - 1 {
                dx + dw
            } else {
                center - dim.width / 2.0
            };
            label.draw(&g.ug().apply_translate(UTranslate::new(x, y)));
        }

        for i in 0..n {
            let (bx, by) = self.branch_offset(i, sb);
            g.apply_translate(UTranslate::new(bx, by))
                .draw_ftile(self.branches[i].tile.as_ref());
        }

        let mut merge = UPolygon::new(vec![
            (MERGE / 2.0, 0.0),
            (MERGE, MERGE / 2.0),
            (MERGE / 2.0, MERGE),
            (0.0, MERGE / 2.0),
        ]);
        let style = self
            .skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ACTIVITY_DIAMOND)
            .expect("diamond style");
        merge.delta_shadow = 0.0;
        g.ug()
            .apply_translate(UTranslate::new(mx, my))
            .apply_color(style.value(PName::LineColor).as_color())
            .apply_back_color(style.value(PName::BackGroundColor).as_color())
            .apply_stroke(style.stroke())
            .draw(merge);

        for i in 0..n {
            let (bx, by) = self.branch_offset(i, sb);
            let bw = self.branch_width(i, sb);
            let bh = self.branches[i].tile.calculate_dimension(sb).height;
            let center = bx + bw / 2.0;
            let mut down = Snake::create(
                &self.skin,
                color.clone(),
                Some(arrows.as_to(Direction::Down)),
            );
            if i == 0 {
                down.add_point(dx, middle);
                down.add_point(center, middle);
            } else if i == n - 1 {
                down.add_point(dx + dw, middle);
                down.add_point(center, middle);
            } else {
                down.add_point(dx + dw / 2.0, self.condition.height);
                down.add_point(center, self.condition.height);
            }
            down.add_point(center, by);
            g.draw_snake(down);

            let incoming = self.branches[i].tile.in_link_rendering();
            if let Some(label) = super::factory::arrow_text_block(&self.skin, incoming.display()) {
                let dim = label.calculate_dimension(sb);
                label.draw(&g.ug().apply_translate(UTranslate::new(
                    center - dim.width / 2.0,
                    (self.condition.height + by - dim.height) / 2.0,
                )));
            }

            let outgoing = self.branches[i].tile.out_link_rendering();
            if let Some(label) = super::factory::arrow_text_block(&self.skin, outgoing.display()) {
                let dim = label.calculate_dimension(sb);
                label.draw(&g.ug().apply_translate(UTranslate::new(
                    center - dim.width / 2.0,
                    merge_y - dim.height - 2.0,
                )));
            }

            let to = if i == 0 {
                Direction::Right
            } else if i == n - 1 {
                Direction::Left
            } else {
                Direction::Down
            };
            let mut up = Snake::create(&self.skin, color.clone(), Some(arrows.as_to(to)));
            up.add_point(center, by + bh);
            if i == 0 {
                up.add_point(center, merge_y);
                up.add_point(mx, merge_y);
            } else if i == n - 1 {
                up.add_point(center, merge_y);
                up.add_point(mx + MERGE, merge_y);
            } else {
                up.add_point(center, my);
            }
            g.draw_snake(up);
        }
    }
}
