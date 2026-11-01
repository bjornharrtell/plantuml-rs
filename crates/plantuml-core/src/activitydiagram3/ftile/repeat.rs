//! A `repeat` loop (`FtileRepeat`, with the `break` handling of `FtileFactoryDelegatorRepeat`).

use super::factory::{arrow_text_block, in_link_rendering_color, FtileRef};
use super::if_links::{Marged, Merge};
use super::if_long::DiamondInside2;
use super::leaf::FtileEmpty;
use super::snake::{Arrows, Snake};
use super::while_::Diamond;
use super::wrap::FtileAssemblySimple;
use super::{FnConnection, Ftile, FtileGeometry, FtileGraphic, WeldingPoint};
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
    pub swimlane: Option<Rc<Swimlane>>,
    /// The lane of the condition at the end.
    pub swimlane_out: Option<Rc<Swimlane>>,
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
    swimlane: Option<Rc<Swimlane>>,
    swimlane_out: Option<Rc<Swimlane>>,
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
            swimlane,
            swimlane_out,
        } = parts;
        let default_back = || Diamond::new(Display::null(), &skin, None).back();
        let color1 = entry_color.unwrap_or_else(default_back);
        let color2 = test_color.unwrap_or_else(|| color1.clone());
        let block = |d: &Option<Display>| d.as_ref().and_then(|d| arrow_text_block(&skin, d));
        let diamond1: FtileRef = match entry {
            Some(tile) => tile,
            None => Rc::new(Merge::new(
                None,
                None,
                None,
                color1,
                skin.clone(),
                swimlane.clone(),
            )),
        };
        let no_out = !body.calculate_dimension(sb).has_point_out();
        let diamond2: FtileRef = if no_out && test.is_null() {
            FtileEmpty::new(&skin, 0.0, 0.0, None)
        } else {
            Rc::new(DiamondInside2 {
                swimlane: swimlane_out.clone(),
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
            swimlane,
            swimlane_out,
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
        let half = HEXAGON_HALF_SIZE;
        let align = self.arrow_horizontal_alignment();
        let skin = self.skin.clone();
        let new_snake = {
            let (skin, color, arrows) = (skin.clone(), color.clone(), arrows);
            move |dir: Direction| Snake::create(&skin, color.clone(), Some(arrows.as_to(dir)))
        };
        let diamond1: Option<FtileRef> = Some(self.diamond1.clone());
        let diamond2: Option<FtileRef> = Some(self.diamond2.clone());

        // ConnectionIn
        let (p1x, p1y) = (d1x + d1.left(), d1y + d1.out_y);
        let (p2x, p2y) = (rx + rg.left(), ry + rg.in_y);
        let draw_in = {
            let (new_snake, tbin) = (new_snake.clone(), self.tbin.clone());
            move |g: &FtileGraphic, p1: (f64, f64), p2: (f64, f64)| {
                let mut into = new_snake(Direction::Down).with_label(tbin.clone(), align);
                into.add_point(p1.0, p1.1);
                if p1.0 != p2.0 {
                    let my = (p1.1 + p2.1) / 2.0;
                    into.add_point(p1.0, my);
                    into.add_point(p2.0, my);
                }
                into.add_point(p2.0, p2.1);
                g.draw_snake(into);
            }
        };
        let conn = FnConnection::new(diamond1.clone(), Some(self.repeat.clone()), {
            let draw_in = draw_in.clone();
            move |g| draw_in(g, (p1x, p1y), (p2x, p2y))
        })
        .translatable(move |g, t1, t2| {
            draw_in(g, (p1x + t1.dx, p1y + t1.dy), (p2x + t2.dx, p2y + t2.dy))
        });
        g.draw_connection(&conn);

        match &self.backward {
            None => {
                let same_lane = match (&self.swimlane, &self.swimlane_out) {
                    (None, _) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                };
                let (a1, a2) = ((d2x, d2y), (d1x, d1y));
                let label = self.back1.clone();
                let ns = new_snake.clone();
                let smaller = same_lane
                    && self
                        .swimlane
                        .as_ref()
                        .is_some_and(|s| s.is_smaller_than_all_others(&self.repeat.swimlanes()));
                if !same_lane {
                    // ConnectionBackComplex1
                    let draw = move |g: &FtileGraphic, p1: (f64, f64), p2: (f64, f64)| {
                        let y1 = p1.1 + d2.height / 2.0;
                        let mut x2 = p2.0 + d1.width;
                        let y2 = p2.1 + d1.height / 2.0;
                        let x1a = p1.0 + d2.width;
                        let x1b = p1.0 + d2.width / 2.0 + rg.width / 2.0 + half;
                        let mut snake;
                        if x2 < x1a {
                            snake = ns(Direction::Left).emphasize_direction(Direction::Up);
                            snake.add_point(x1a, y1);
                            if x1a < x1b {
                                snake.add_point(x1b, y1);
                                snake.add_point(x1b, y2);
                            } else {
                                snake.add_point(x1a + 10.0, y1);
                                snake.add_point(x1a + 10.0, y2);
                            }
                        } else {
                            x2 = p2.0;
                            snake = ns(Direction::Right).emphasize_direction(Direction::Up);
                            snake.add_point(x1a, y1);
                            let middle = x1a / 4.0 + x2 * 3.0 / 4.0;
                            snake.add_point(middle, y1);
                            snake.add_point(middle, y2);
                        }
                        snake.add_point(x2, y2);
                        g.draw_snake(snake);
                    };
                    let conn = FnConnection::new(diamond2.clone(), diamond1.clone(), {
                        let draw = draw.clone();
                        move |g| draw(g, a1, a2)
                    })
                    .translatable(move |g, t1, t2| {
                        draw(
                            g,
                            (a1.0 + t1.dx, a1.1 + t1.dy),
                            (a2.0 + t2.dx, a2.1 + t2.dy),
                        )
                    });
                    g.draw_connection(&conn);
                } else if smaller {
                    // ConnectionBackSimple1
                    let label2 = label.clone();
                    let ns2 = ns.clone();
                    let conn = FnConnection::new(diamond2.clone(), diamond1.clone(), move |g| {
                        let mut snake = ns(Direction::Right)
                            .emphasize_direction(Direction::Up)
                            .with_label(label.clone(), align);
                        let (x1, y1) = (a1.0, a1.1 + d2.height / 2.0);
                        let (x2, y2) = (a2.0, a2.1 + d1.height / 2.0);
                        snake.add_point(x1, y1);
                        snake.add_point(-half, y1);
                        snake.add_point(-half, y2);
                        snake.add_point(x2, y2);
                        g.draw_snake(snake);
                    })
                    .translatable(move |g, t1, t2| {
                        let p1 = (a1.0 + t1.dx, a1.1 + t1.dy);
                        let p2 = (a2.0 + t2.dx, a2.1 + t2.dy);
                        let mut snake = ns2(Direction::Left)
                            .emphasize_direction(Direction::Up)
                            .with_label(label2.clone(), align);
                        let (x1, y1) = (p1.0, p1.1 + d2.height / 2.0);
                        let (x2, y2) = (p2.0, p2.1 + d1.height / 2.0);
                        snake.add_point(x1, y1);
                        let xmax = p1.0 + d2.width / 2.0 + rg.width / 2.0 + half;
                        snake.add_point(xmax, y1);
                        snake.add_point(xmax, y2);
                        snake.add_point(x2, y2);
                        g.draw_snake(snake);
                    });
                    g.draw_connection(&conn);
                } else {
                    // ConnectionBackSimple2
                    let label2 = label.clone();
                    let ns2 = ns.clone();
                    let conn = FnConnection::new(diamond2.clone(), diamond1.clone(), move |g| {
                        let (x1, y1) = (a1.0 + d2.width, a1.1 + d2.height / 2.0);
                        let (x2, y2) = (a2.0 + d1.width, a2.1 + d1.height / 2.0);
                        let xmax = total_w - half;
                        let mut back = ns(Direction::Left)
                            .emphasize_direction(Direction::Up)
                            .with_label(label.clone(), align);
                        back.add_point(x1, y1);
                        back.add_point(xmax, y1);
                        back.add_point(xmax, y2);
                        back.add_point(x2, y2);
                        g.draw_snake(back);
                    })
                    .translatable(move |g, t1, t2| {
                        let p1 = (a1.0 + t1.dx, a1.1 + t1.dy);
                        let p2 = (a2.0 + t2.dx, a2.1 + t2.dy);
                        let (x1, y1) = (p1.0 + d2.width, p1.1 + d2.height / 2.0);
                        let (x2a, x2b) = (p2.0, p2.0 + d1.width);
                        let on_a = x1 < (x2a + x2b) / 2.0;
                        let x2 = if on_a { x2a } else { x2b };
                        let y2 = p2.1 + d1.height / 2.0;
                        let dir = if on_a {
                            Direction::Right
                        } else {
                            Direction::Left
                        };
                        let mut snake = ns2(dir)
                            .emphasize_direction(Direction::Up)
                            .with_label(label2.clone(), align);
                        snake.add_point(x1, y1);
                        let xmiddle = (x1 + x2) / 2.0;
                        snake.add_point(xmiddle, y1);
                        snake.add_point(xmiddle, y2);
                        snake.add_point(x2, y2);
                        g.draw_snake(snake);
                    });
                    g.draw_connection(&conn);
                }
            }
            Some(b) => {
                let bg = b.calculate_dimension(sb);
                let (bx, by) = self.translate_backward(b, sb);
                // ConnectionBackBackward1: the condition to the bottom of the `backward` box
                let (x2, y2) = (bx + bg.left(), by + bg.out_y);
                let (label1, label2) = (self.back1.clone(), self.back2.clone());
                let (ns, ns2) = (new_snake.clone(), new_snake.clone());
                let draw_up = move |g: &FtileGraphic, p1: (f64, f64), p2: (f64, f64)| {
                    let center = p1.0 + d2.width / 2.0;
                    let x1 = if p2.0 < center { p1.0 } else { p1.0 + d2.width };
                    let y1 = p1.1 + d2.height / 2.0;
                    let mut up = ns(Direction::Up).with_label(label1.clone(), align);
                    up.add_point(x1, y1);
                    up.add_point(p2.0, y1);
                    up.add_point(p2.0, p2.1);
                    g.draw_snake(up);
                };
                let conn = FnConnection::new(diamond2.clone(), Some(b.clone()), {
                    let draw_up = draw_up.clone();
                    move |g| draw_up(g, (d2x, d2y), (x2, y2))
                })
                .translatable(move |g, t1, t2| {
                    draw_up(g, (d2x + t1.dx, d2y + t1.dy), (x2 + t2.dx, y2 + t2.dy))
                });
                g.draw_connection(&conn);
                // ConnectionBackBackward2: from the top of the box to the start
                let (ix, iy) = (bx + bg.left(), by + bg.in_y);
                let (ex, ey) = (d1x, d1y);
                let ns3 = new_snake.clone();
                let label3 = self.back2.clone();
                let conn = FnConnection::new(Some(b.clone()), diamond1.clone(), move |g| {
                    let mut left = ns2(Direction::Left).with_label(label2.clone(), align);
                    left.add_point(ix, iy);
                    left.add_point(ix, ey + d1.height / 2.0);
                    left.add_point(ex + d1.width, ey + d1.height / 2.0);
                    g.draw_snake(left);
                })
                .translatable(move |g, t1, t2| {
                    let (x1, y1) = (ix + t1.dx, iy + t1.dy);
                    let mut x2 = ex + t2.dx;
                    let y2 = ey + t2.dy + d1.height / 2.0;
                    if x2 < x1 {
                        x2 += d1.width;
                    }
                    let dir = if x2 < x1 {
                        Direction::Left
                    } else {
                        Direction::Right
                    };
                    let mut snake = ns3(dir).with_label(label3.clone(), align);
                    snake.add_point(x1, y1);
                    snake.add_point(x1, y2);
                    snake.add_point(x2, y2);
                    g.draw_snake(snake);
                });
                g.draw_connection(&conn);
            }
        }

        // ConnectionOut
        if rg.has_point_out() {
            let (o1x, o1y) = (rx + rg.left(), ry + rg.out_y);
            let (o2x, o2y) = (d2x + d2.left(), d2y + d2.in_y);
            let tbout = self.tbout.clone();
            let ns = new_snake.clone();
            let (skin, color) = (skin.clone(), color.clone());
            let conn = FnConnection::new(Some(self.repeat.clone()), diamond2.clone(), move |g| {
                let mut out = ns(Direction::Down).with_label(tbout.clone(), align);
                out.add_point(o1x, o1y);
                out.add_point(o2x, o2y);
                g.draw_snake(out);
            })
            .translatable({
                let (ns, tbout) = (new_snake.clone(), self.tbout.clone());
                move |g, t1, t2| {
                    let mp1 = (o1x + t1.dx, o1y + t1.dy);
                    let mp2 = (o2x + t2.dx, o2y + t2.dy);
                    let middle = (mp1.1 + mp2.1) / 2.0;
                    let mut s = Snake::create(&skin, color.clone(), None);
                    s.add_point(mp1.0, mp1.1);
                    s.add_point(mp1.0, middle);
                    s.add_point(mp2.0, middle);
                    g.draw_snake(s);
                    let mut small = ns(Direction::Down).with_label(tbout.clone(), align);
                    small.add_point(mp2.0, middle);
                    small.add_point(mp2.0, mp2.1);
                    g.draw_snake(small);
                }
            });
            g.draw_connection(&conn);
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
        swimlane: Option<Rc<Swimlane>>,
    ) -> FtileRef {
        if points.is_empty() {
            return repeat;
        }
        let back = color.unwrap_or_else(|| Diamond::new(Display::null(), &skin, None).back());
        let diamond_break: FtileRef =
            Rc::new(Merge::new(None, None, None, back, skin.clone(), swimlane));
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
