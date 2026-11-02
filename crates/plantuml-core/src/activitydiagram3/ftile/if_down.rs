//! An `if` whose second branch is empty or only a `stop`: the real branch goes straight down under
//! the condition (`FtileIfDown`).

use super::factory::{
    arrow_text_block, in_link_rendering_color, out_link_rendering_color, FtileRef,
};
use super::if_links::{CondBranch, Marged, Merge};
use super::if_long::{DiamondInside2, MinWidthCentered};
use super::leaf::FtileEmpty;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::{same_ftile, FnConnection, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::instruction::PositionedNote;
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UShape, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::Direction;
use crate::skin::SkinParam;
use crate::svek::image_note::Opale;
use std::rc::Rc;

const HEXAGON_HALF_SIZE: f64 = 12.0;

/// What the builder decides before the tile exists (`ConditionalBuilder.createDown`).
pub struct DownParts {
    /// The branch that is drawn under the condition, and the one next to it.
    pub first: CondBranch,
    pub second: CondBranch,
    pub first_is_stop: bool,
    pub second_is_stop: bool,
    /// Both original branches leave, so a merge diamond is drawn.
    pub two_branches: bool,
    /// The text in the condition: always the first branch's, whichever one is drawn below.
    pub label_test: Display,
    /// The note beside the condition; only a single note is laid out (`FtileIfDown`).
    pub notes: Vec<PositionedNote>,
}

pub struct FtileIfDown {
    opale: Option<Opale>,
    then_block: FtileRef,
    diamond1: Rc<DiamondInside2>,
    diamond2: FtileRef,
    optional_stop: Option<FtileRef>,
    skin: Rc<SkinParam>,
    else_color: crate::activitydiagram3::rainbow::Rainbow,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
    /// The else path leaves by the west vertex (`ConnectionElse1`).
    else1: bool,
}

impl FtileIfDown {
    pub fn create(
        parts: DownParts,
        back: Option<HColor>,
        skin: Rc<SkinParam>,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
        swimlane: Option<Rc<Swimlane>>,
        sb: &dyn StringBounder,
    ) -> FtileRef {
        let DownParts {
            first,
            second,
            first_is_stop,
            second_is_stop,
            two_branches,
            notes,
            label_test,
        } = parts;
        let block = |d: &Option<Display>| d.as_ref().and_then(|d| arrow_text_block(&skin, d));
        let (tb1, tb2) = (block(&first.label_positive), block(&second.label_positive));
        let color = first.color.clone().or_else(|| back.clone());
        let merge_back = second
            .color
            .clone()
            .or(color.clone())
            .unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back());
        let diamond2_for_labels: FtileRef = if two_branches {
            let out =
                |b: &CondBranch| arrow_text_block(&skin, b.tile.out_link_rendering().display());
            Rc::new(Merge::new(
                out(&first),
                None,
                out(&second),
                merge_back,
                skin.clone(),
                swimlane.clone(),
            ))
        } else {
            FtileEmpty::new(&skin, 0.0, HEXAGON_HALF_SIZE / 2.0, swimlane.clone())
        };
        let else_color = out_link_rendering_color(&skin, second.tile.as_ref());
        let min_width = |t: FtileRef| -> FtileRef {
            Marged::horizontal(
                Rc::new(MinWidthCentered {
                    inner: t,
                    min_width: 30.0,
                }),
                10.0,
                10.0,
            )
        };
        let (then_block, optional_stop): (FtileRef, Option<FtileRef>) = if second_is_stop {
            (min_width(first.tile), Some(second.tile))
        } else if first_is_stop {
            (min_width(second.tile), Some(first.tile))
        } else if first.empty {
            (min_width(second.tile), None)
        } else {
            (min_width(first.tile), None)
        };
        // `ConnectionElse1`: with the branch in lanes to the right, the else path leaves by the west
        let else1 = optional_stop.is_none()
            && then_block.calculate_dimension(sb).has_point_out()
            && swimlane
                .as_ref()
                .is_some_and(|s| s.is_smaller_than_all_others(&then_block.swimlanes()));
        let tb1_south = tb1;
        let diamond1 = Rc::new(DiamondInside2 {
            diamond: Diamond::new(label_test.clone(), &skin, color.clone()),
            north: None,
            south: tb1_south,
            west: if else1 { tb2.clone() } else { None },
            east: if else1 { None } else { tb2.clone() },
            skin: skin.clone(),
            north_widens: false,
            swimlane: swimlane.clone(),
        });
        let diamond2: FtileRef = if optional_stop.is_some() {
            FtileEmpty::new(&skin, 0.0, 0.0, swimlane.clone())
        } else {
            diamond2_for_labels
        };
        let opale = (notes.len() == 1).then(|| super::note::opale_of(&notes[0], &skin));
        Rc::new(Self {
            opale,
            then_block,
            diamond1,
            diamond2,
            optional_stop,
            skin,
            else_color,
            in_link_rendering,
            out_link_rendering,
            else1,
        })
    }

    fn opale_size(&self, sb: &dyn StringBounder) -> (f64, f64) {
        self.opale.as_ref().map_or((0.0, 0.0), |o| {
            let d = o.dimension(sb);
            (d.width, d.height)
        })
    }

    fn additional_width(&self, sb: &dyn StringBounder) -> f64 {
        let Some(stop) = &self.optional_stop else {
            return 0.0;
        };
        let stop_width = stop.calculate_dimension(sb).width;
        stop_width.max(self.diamond1.east_width(sb) + stop_width / 2.0)
    }

    fn translate_then(&self, sb: &dyn StringBounder) -> UTranslate {
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let total = self.calculate_dimension(sb);
        let then = self.then_block.calculate_dimension(sb);
        let oh = self.opale_size(sb).1;
        let y = oh + d1.height + (total.height - oh - d1.height - d2.height - then.height) / 2.0;
        UTranslate::new(total.left() - then.left(), y)
    }

    fn translate_diamond1(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.calculate_dimension(sb);
        UTranslate::new(
            total.left() - self.diamond1.calculate_dimension(sb).left(),
            self.opale_size(sb).1,
        )
    }

    fn translate_diamond2(&self, sb: &dyn StringBounder) -> UTranslate {
        let total = self.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        UTranslate::new(total.left() - d2.left(), total.height - d2.height)
    }

    fn translate_stop(&self, stop: &FtileRef, sb: &dyn StringBounder) -> UTranslate {
        let total = self.calculate_dimension(sb);
        let d1 = self.diamond1.calculate_dimension(sb);
        let s = stop.calculate_dimension(sb);
        let north = d1.in_y;
        UTranslate::new(
            total.left() - d1.left() + d1.width + self.additional_width(sb),
            north + (d1.height - north - s.height) / 2.0,
        )
    }
}

impl Ftile for FtileIfDown {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let d1 = self.diamond1.calculate_dimension(sb);
        let then = self.then_block.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let geo = d1.append_bottom(&then).append_bottom(&d2);
        let (opale_w, opale_h) = self.opale_size(sb);
        let height = geo.height
            + 3.0 * HEXAGON_HALF_SIZE
            + HEXAGON_HALF_SIZE.max(self.diamond1.south_height(sb))
            + opale_h;
        let supp = (opale_w - geo.left()).max(0.0);
        let left = if supp > 0.0 {
            opale_w + d1.left()
        } else {
            geo.left()
        };
        let mut width = supp + geo.width + HEXAGON_HALF_SIZE;
        if let Some(stop) = &self.optional_stop {
            width += stop.calculate_dimension(sb).width + self.additional_width(sb);
        }
        if !then.has_point_out() && self.optional_stop.is_some() {
            FtileGeometry::new(width, height, left, d1.in_y + opale_h)
        } else {
            FtileGeometry::with_out(width, height, left, d1.in_y + opale_h, height)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let mut parts: Vec<(FtileRef, UTranslate)> = vec![
            (self.then_block.clone(), self.translate_then(sb)),
            (self.diamond1.clone(), self.translate_diamond1(sb)),
            (self.diamond2.clone(), self.translate_diamond2(sb)),
        ];
        if let Some(stop) = &self.optional_stop {
            parts.push((stop.clone(), self.translate_stop(stop, sb)));
        }
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
        let mut children = vec![
            self.then_block.clone(),
            self.diamond1.clone(),
            self.diamond2.clone(),
        ];
        children.extend(self.optional_stop.clone());
        children
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.diamond1.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        match &self.optional_stop {
            None => self.swimlane_in(),
            Some(_) => self.then_block.swimlane_out(),
        }
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes = self.then_block.swimlanes();
        if let Some(lane) = self.swimlane_in() {
            if !lanes.iter().any(|l| Rc::ptr_eq(l, &lane)) {
                lanes.push(lane);
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
        let (then_t, d1t) = (self.translate_then(sb), self.translate_diamond1(sb));
        if let Some(opale) = &self.opale {
            let x = d1t.dx - opale.dimension(sb).width;
            opale.draw(&g.ug().apply_translate(UTranslate::dx(x)), None);
        }
        g.apply_translate(then_t)
            .draw_ftile(self.then_block.as_ref());
        g.apply_translate(d1t).draw_ftile(self.diamond1.as_ref());
        match &self.optional_stop {
            None => {
                g.apply_translate(self.translate_diamond2(sb))
                    .draw_ftile(self.diamond2.as_ref());
            }
            Some(stop) => {
                g.apply_translate(self.translate_stop(stop, sb))
                    .draw_ftile(stop.as_ref());
            }
        }

        let arrows = Arrows::from_skin(&self.skin);
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let then_geo = self.then_block.calculate_dimension(sb);
        let total = self.calculate_dimension(sb);
        let d2t = self.translate_diamond2(sb);
        let in_color = in_link_rendering_color(&self.skin, self.then_block.as_ref());
        let mid1 = (d1.in_y + d1.out_y) / 2.0;
        let diamond1: FtileRef = self.diamond1.clone();
        let skin = self.skin.clone();
        let else_color = self.else_color.clone();

        // ConnectionIn: condition to the branch
        {
            let p1 = (d1t.dx + d1.left(), d1t.dy + d1.out_y);
            let p2 = (then_t.dx + then_geo.left(), then_t.dy + then_geo.in_y);
            let end = arrows.as_to(Direction::Down);
            let (skin1, color1, end1) = (skin.clone(), in_color.clone(), end.clone());
            let conn = FnConnection::new(
                Some(diamond1.clone()),
                Some(self.then_block.clone()),
                move |g| {
                    let mut down = Snake::create(&skin1, color1.clone(), Some(end1.clone()));
                    down.add_point(p1.0, p1.1);
                    down.add_point(p2.0, p2.1);
                    g.draw_snake(down);
                },
            )
            .translatable({
                let skin = skin.clone();
                move |g, t1, t2| {
                    let mp1 = (p1.0 + t1.dx, p1.1 + t1.dy);
                    let mp2 = (p2.0 + t2.dx, p2.1 + t2.dy);
                    let middle = (mp1.1 + mp2.1) / 2.0;
                    let mut s = Snake::create(&skin, in_color.clone(), Some(end.clone()));
                    s.add_point(mp1.0, mp1.1);
                    s.add_point(mp1.0, middle);
                    s.add_point(mp2.0, middle);
                    s.add_point(mp2.0, mp2.1);
                    g.draw_snake(s);
                }
            });
            g.draw_connection(&conn);
        }

        // the else path, from the condition's side vertex
        let (first, last): (FtileRef, FtileRef) = (diamond1.clone(), self.diamond2.clone());
        let else_conn = |arrow: Direction, points: Vec<(f64, f64)>, empty_at: (f64, f64)| {
            let (skin, color) = (skin.clone(), else_color.clone());
            let end = arrows.as_to(arrow);
            FnConnection::new(Some(first.clone()), Some(last.clone()), move |g| {
                let mut s = Snake::create(&skin, color.clone(), Some(end.clone()))
                    .emphasize_direction(Direction::Down);
                for p in &points {
                    s.add_point(p.0, p.1);
                }
                g.ug()
                    .apply_translate(UTranslate::new(empty_at.0, empty_at.1))
                    .draw(UShape::Empty(5.0, HEXAGON_HALF_SIZE));
                g.draw_snake(s);
            })
        };
        match &self.optional_stop {
            Some(stop) => {
                // ConnectionHorizontal: east vertex to the `stop`
                let st = self.translate_stop(stop, sb);
                let sg = stop.calculate_dimension(sb);
                let (skin, color, end) = (
                    skin.clone(),
                    self.else_color.clone(),
                    arrows.as_to(Direction::Right),
                );
                let (a, b) = (
                    (d1t.dx + d1.width, d1t.dy + mid1),
                    (st.dx, st.dy + sg.height / 2.0),
                );
                let conn =
                    FnConnection::new(Some(diamond1.clone()), Some(stop.clone()), move |g| {
                        let mut s = Snake::create(&skin, color.clone(), Some(end.clone()));
                        s.add_point(a.0, a.1);
                        s.add_point(b.0, b.1);
                        g.draw_snake(s);
                    });
                g.draw_connection(&conn);
            }
            None if then_geo.has_point_out() && self.else1 => {
                // ConnectionElse1: west vertex, round the branch on the left, into the merge diamond
                let (x1, y1) = (d1t.dx, d1t.dy + mid1);
                let (x2, y2) = (d2t.dx, d2t.dy + (d2.in_y + d2.out_y) / 2.0);
                let xmin = (x1 - HEXAGON_HALF_SIZE).min(then_t.dx);
                let conn = else_conn(
                    Direction::Right,
                    vec![(x1, y1), (xmin, y1), (xmin, y2), (x2, y2)],
                    (x2, y2 - HEXAGON_HALF_SIZE),
                );
                g.draw_connection(&conn);
            }
            None if then_geo.has_point_out() => {
                // ConnectionElse2: east vertex, round the branch, into the merge diamond
                let (x1, y1) = (d1t.dx + d1.width, d1t.dy + mid1);
                let (x2, y2) = (d2t.dx + d2.width, d2t.dy + (d2.in_y + d2.out_y) / 2.0);
                let xmax = (x1 + HEXAGON_HALF_SIZE).max(then_t.dx + then_geo.width);
                let conn = else_conn(
                    Direction::Left,
                    vec![(x1, y1), (xmax, y1), (xmax, y2), (x2, y2)],
                    (x2, y2 - HEXAGON_HALF_SIZE),
                );
                g.draw_connection(&conn);
            }
            None => {
                // ConnectionElseNoDiamond: the branch never leaves, so the else path is the exit
                let (x1, y1) = (d1t.dx + d1.width, d1t.dy + mid1);
                let (x2, y2) = (total.left(), total.height);
                let xmax = (x1 + HEXAGON_HALF_SIZE).max(then_t.dx + then_geo.width);
                let conn = else_conn(
                    Direction::Left,
                    vec![(x1, y1), (xmax, y1), (xmax, y2), (x2, y2)],
                    (x2, y2 - HEXAGON_HALF_SIZE),
                );
                g.draw_connection(&conn);
            }
        }

        // ConnectionOut: the branch to the merge diamond
        if then_geo.has_point_out() {
            let out_color = out_link_rendering_color(&self.skin, self.then_block.as_ref());
            let p1 = (then_t.dx + then_geo.left(), then_t.dy + then_geo.out_y);
            let p2 = (d2t.dx + d2.left(), d2t.dy + d2.in_y);
            let end = arrows.as_to(Direction::Down);
            let (skin1, color1, end1) = (skin.clone(), out_color.clone(), end.clone());
            let conn = FnConnection::new(
                Some(self.then_block.clone()),
                Some(self.diamond2.clone()),
                move |g| {
                    let mut s = Snake::create(&skin1, color1.clone(), Some(end1.clone()));
                    s.add_point(p1.0, p1.1);
                    s.add_point(p2.0, p2.1);
                    g.draw_snake(s);
                },
            )
            .translatable(move |g, t1, t2| {
                let mp1 = (p1.0 + t1.dx, p1.1 + t1.dy);
                let mp2 = (p2.0 + t2.dx, p2.1 + t2.dy);
                let middle = (mp1.1 + mp2.1) / 2.0;
                let mut s = Snake::create(&skin, out_color.clone(), Some(end.clone()));
                s.add_point(mp1.0, mp1.1);
                s.add_point(mp1.0, middle);
                s.add_point(mp2.0, middle);
                s.add_point(mp2.0, mp2.1);
                g.draw_snake(s);
            });
            g.draw_connection(&conn);
        }
    }
}
