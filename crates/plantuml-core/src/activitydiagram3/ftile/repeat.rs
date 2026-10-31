//! A `repeat` loop (`FtileRepeat`, with the `break` handling of `FtileFactoryDelegatorRepeat`).

use super::factory::{arrow_text_block, in_link_rendering_color, FtileRef};
use super::if_links::{Marged, Merge};
use super::if_long::DiamondInside2;
use super::leaf::FtileEmpty;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::wrap::FtileAssemblySimple;
use super::{Ftile, FtileGeometry, FtileGraphic, WeldingPoint};
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

const HEXAGON_HALF_SIZE: f64 = 12.0;

/// What a repeat loop is made of, before it is laid out.
pub struct RepeatParts {
    pub body: FtileRef,
    /// The `repeat :label;` box; a small diamond when there is none.
    pub entry: Option<FtileRef>,
    pub test: Display,
    pub yes: Option<Display>,
    pub out: Option<Display>,
    pub backward: Option<FtileRef>,
    /// The arrow labels around the `backward` box, or the one on the way back when there is none.
    pub back1: Display,
    pub back2: Display,
    pub entry_color: Option<HColor>,
    pub test_color: Option<HColor>,
}

/// A loop whose condition is tested at the end (`FtileRepeat`).
pub struct FtileRepeat {
    repeat: FtileRef,
    diamond1: FtileRef,
    diamond2: FtileRef,
    backward: Option<FtileRef>,
    back1: Option<Rc<dyn TextBlock>>,
    back2: Option<Rc<dyn TextBlock>>,
    tbin: Option<Rc<dyn TextBlock>>,
    tbout: Option<Rc<dyn TextBlock>>,
    skin: Rc<SkinParam>,
    in_link_rendering: LinkRendering,
    out_link_rendering: LinkRendering,
}

impl FtileRepeat {
    pub fn create(
        parts: RepeatParts,
        skin: Rc<SkinParam>,
        sb: &dyn StringBounder,
        in_link_rendering: LinkRendering,
        out_link_rendering: LinkRendering,
    ) -> FtileRef {
        let RepeatParts {
            body,
            entry,
            test,
            yes,
            out,
            backward,
            back1,
            back2,
            entry_color,
            test_color,
        } = parts;
        let default_back = || Diamond::new(Display::null(), &skin, None).back();
        let color1 = entry_color.unwrap_or_else(default_back);
        let color2 = test_color.unwrap_or_else(|| color1.clone());
        let block = |d: &Option<Display>| d.as_ref().and_then(|d| arrow_text_block(&skin, d));
        let diamond1: FtileRef = match entry {
            Some(tile) => tile,
            None => Rc::new(Merge::new(None, None, None, color1, skin.clone())),
        };
        let no_out = !body.calculate_dimension(sb).has_point_out();
        let diamond2: FtileRef = if no_out && test.is_null() {
            FtileEmpty::new(&skin, 0.0, 0.0, None)
        } else {
            Rc::new(DiamondInside2 {
                diamond: Diamond::new(test, &skin, Some(color2)),
                north: None,
                south: block(&out),
                west: None,
                east: block(&yes),
                skin: skin.clone(),
                north_widens: false,
            })
        };
        let tbin = arrow_text_block(&skin, body.in_link_rendering().display());
        let tbout = arrow_text_block(&skin, body.out_link_rendering().display());
        let result: FtileRef = Rc::new(Self {
            repeat: body,
            diamond1,
            diamond2,
            backward,
            back1: arrow_text_block(&skin, &back1),
            back2: arrow_text_block(&skin, &back2),
            tbin,
            tbout,
            skin,
            in_link_rendering,
            out_link_rendering,
        });
        result
    }

    fn left(&self, sb: &dyn StringBounder) -> f64 {
        let (d1, d2) = (
            self.diamond1.calculate_dimension(sb),
            self.diamond2.calculate_dimension(sb),
        );
        let repeat_left = self.repeat.calculate_dimension(sb).left();
        repeat_left.max(d1.width / 2.0).max(d2.width / 2.0)
    }

    fn right(&self, sb: &dyn StringBounder) -> f64 {
        let (d1, d2) = (
            self.diamond1.calculate_dimension(sb),
            self.diamond2.calculate_dimension(sb),
        );
        let r = self.repeat.calculate_dimension(sb);
        let right = r.width - r.left();
        right.max(d1.width / 2.0).max(d2.width / 2.0)
    }

    fn internal(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let r = self.repeat.calculate_dimension(sb);
        let mut width = (self.left(sb) + self.right(sb)).max(2.0 * HEXAGON_HALF_SIZE);
        if let Some(b) = &self.backward {
            width += b.calculate_dimension(sb).width;
        }
        (
            width + 2.0 * HEXAGON_HALF_SIZE,
            d1.height + r.height + d2.height + 8.0 * HEXAGON_HALF_SIZE,
        )
    }

    fn translate_repeat(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let r = self.repeat.calculate_dimension(sb);
        let (_, height) = self.internal(sb);
        let space = height - d1.height - d2.height - r.height;
        (self.left(sb) - r.left(), d1.height + space / 2.0)
    }

    fn translate_diamond1(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let d1 = self.diamond1.calculate_dimension(sb);
        (self.left(sb) - d1.width / 2.0, 0.0)
    }

    fn translate_diamond2(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let d2 = self.diamond2.calculate_dimension(sb);
        let (_, height) = self.internal(sb);
        (self.left(sb) - d2.width / 2.0, height - d2.height)
    }

    fn translate_backward(&self, backward: &FtileRef, sb: &dyn StringBounder) -> (f64, f64) {
        let (width, height) = self.internal(sb);
        let b = backward.calculate_dimension(sb);
        (width - b.width, (height - b.height) / 2.0)
    }
}

impl Ftile for FtileRepeat {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (width, height) = self.internal(sb);
        FtileGeometry::with_out(width, height, self.left(sb), 0.0, height)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let (rx, ry) = self.translate_repeat(sb);
        let (dx, dy) = self.translate_diamond1(sb);
        let t = UTranslate::new(rx, ry);
        if super::same_ftile(child, self.repeat.as_ref()) {
            return Some(t);
        }
        if let Some(c) = self.repeat.translate_for(child, sb) {
            return Some(c.compose(t));
        }
        let t = UTranslate::new(dx, dy);
        if super::same_ftile(child, self.diamond1.as_ref()) {
            return Some(t);
        }
        self.diamond1.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn my_children(&self) -> Vec<FtileRef> {
        vec![
            self.repeat.clone(),
            self.diamond1.clone(),
            self.diamond2.clone(),
        ]
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.repeat.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.diamond2.swimlane_out()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes = self.repeat.swimlanes();
        let others = self
            .diamond1
            .swimlanes()
            .into_iter()
            .chain(self.diamond2.swimlanes())
            .chain(self.backward.iter().flat_map(|b| b.swimlanes()));
        for lane in others {
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
        let (rx, ry) = self.translate_repeat(sb);
        let (d1x, d1y) = self.translate_diamond1(sb);
        let (d2x, d2y) = self.translate_diamond2(sb);
        g.apply_translate(UTranslate::new(rx, ry))
            .draw_ftile(self.repeat.as_ref());
        g.apply_translate(UTranslate::new(d1x, d1y))
            .draw_ftile(self.diamond1.as_ref());
        g.apply_translate(UTranslate::new(d2x, d2y))
            .draw_ftile(self.diamond2.as_ref());
        if let Some(b) = &self.backward {
            let (bx, by) = self.translate_backward(b, sb);
            g.apply_translate(UTranslate::new(bx, by))
                .draw_ftile(b.as_ref());
        }

        let arrows = Arrows::from_skin(&self.skin);
        let color = in_link_rendering_color(&self.skin, self.repeat.as_ref());
        let d1 = self.diamond1.calculate_dimension(sb);
        let d2 = self.diamond2.calculate_dimension(sb);
        let rg = self.repeat.calculate_dimension(sb);
        let (total_w, _) = self.internal(sb);
        let new_snake =
            |dir: Direction| Snake::create(&self.skin, color.clone(), Some(arrows.as_to(dir)));

        // ConnectionIn
        let (p1x, p1y) = (d1x + d1.left(), d1y + d1.out_y);
        let (p2x, p2y) = (rx + rg.left(), ry + rg.in_y);
        let mut into = new_snake(Direction::Down)
            .with_label(self.tbin.clone(), self.arrow_horizontal_alignment());
        into.add_point(p1x, p1y);
        if p1x != p2x {
            let my = (p1y + p2y) / 2.0;
            into.add_point(p1x, my);
            into.add_point(p2x, my);
        }
        into.add_point(p2x, p2y);
        g.draw_snake(into);

        // ConnectionOut
        if rg.has_point_out() {
            let mut out = new_snake(Direction::Down)
                .with_label(self.tbout.clone(), self.arrow_horizontal_alignment());
            out.add_point(rx + rg.left(), ry + rg.out_y);
            out.add_point(d2x + d2.left(), d2y + d2.in_y);
            g.draw_snake(out);
        }

        match &self.backward {
            None => {
                // ConnectionBackSimple2
                let (x1, y1) = (d2x + d2.width, d2y + d2.height / 2.0);
                let (x2, y2) = (d1x + d1.width, d1y + d1.height / 2.0);
                let xmax = total_w - HEXAGON_HALF_SIZE;
                let mut back = new_snake(Direction::Left)
                    .emphasize_direction(Direction::Up)
                    .with_label(self.back1.clone(), self.arrow_horizontal_alignment());
                back.add_point(x1, y1);
                back.add_point(xmax, y1);
                back.add_point(xmax, y2);
                back.add_point(x2, y2);
                g.draw_snake(back);
            }
            Some(b) => {
                let bg = b.calculate_dimension(sb);
                let (bx, by) = self.translate_backward(b, sb);
                // ConnectionBackBackward1: the condition to the bottom of the `backward` box
                let (x2, y2) = (bx + bg.left(), by + bg.out_y);
                let center = d2x + d2.width / 2.0;
                let x1 = if x2 < center { d2x } else { d2x + d2.width };
                let y1 = d2y + d2.height / 2.0;
                let mut up = new_snake(Direction::Up)
                    .with_label(self.back1.clone(), self.arrow_horizontal_alignment());
                up.add_point(x1, y1);
                up.add_point(x2, y1);
                up.add_point(x2, y2);
                g.draw_snake(up);
                // ConnectionBackBackward2: from the top of the box to the start
                let (ix, iy) = (bx + bg.left(), by + bg.in_y);
                let (ex, ey) = (d1x + d1.width, d1y + d1.height / 2.0);
                let mut left = new_snake(Direction::Left)
                    .with_label(self.back2.clone(), self.arrow_horizontal_alignment());
                left.add_point(ix, iy);
                left.add_point(ix, ey);
                left.add_point(ex, ey);
                g.draw_snake(left);
            }
        }
    }
}

/// The loop followed by the diamond its `break`s lead to (`FtileFactoryDelegatorRepeat`).
pub struct RepeatBreaks {
    assembly: Rc<FtileAssemblySimple>,
    inner: FtileRef,
    diamond_break: FtileRef,
    points: Vec<WeldingPoint>,
    skin: Rc<SkinParam>,
}

impl RepeatBreaks {
    pub fn wrap(
        repeat: FtileRef,
        points: Vec<WeldingPoint>,
        color: Option<HColor>,
        skin: Rc<SkinParam>,
    ) -> FtileRef {
        if points.is_empty() {
            return repeat;
        }
        let back = color.unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back());
        let diamond_break: FtileRef = Rc::new(Merge::new(None, None, None, back, skin.clone()));
        let assembly =
            FtileAssemblySimple::new(Marged::horizontal(repeat, 10.0, 0.0), diamond_break.clone());
        Rc::new(Self {
            inner: assembly.clone(),
            assembly,
            diamond_break,
            points,
            skin,
        })
    }
}

impl Ftile for RepeatBreaks {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.inner.clone())
    }

    fn draw(&self, g: &FtileGraphic) {
        g.draw_ftile(self.inner.as_ref());
        let sb = g.string_bounder();
        let arrows = Arrows::from_skin(&self.skin);
        let color = in_link_rendering_color(&self.skin, self.inner.as_ref());
        let diamond = self.diamond_break.calculate_dimension(sb);
        let to = self
            .assembly
            .translate_for(self.diamond_break.as_ref(), sb)
            .unwrap_or_default();
        for (i, point) in self.points.iter().enumerate() {
            let Some(from) = self.assembly.translate_for(point.tile.as_ref(), sb) else {
                continue;
            };
            let mid = to.dy + diamond.height / 2.0;
            if i == 0 {
                let mut s = Snake::create(
                    &self.skin,
                    color.clone(),
                    Some(arrows.as_to(Direction::Right)),
                );
                s.add_point(from.dx, from.dy);
                s.add_point(0.0, from.dy);
                s.add_point(0.0, mid);
                s.add_point(to.dx, mid);
                g.draw_snake(s);
            } else {
                let mut s = Snake::create(
                    &self.skin,
                    color.clone(),
                    Some(arrows.as_to(Direction::Left)),
                );
                s.add_point(from.dx, from.dy);
                s.add_point(0.0, from.dy);
                g.draw_snake(s);
            }
        }
    }
}
