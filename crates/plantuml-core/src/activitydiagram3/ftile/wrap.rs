//! Tiles composed of other tiles, and the arrows between them.

use super::snake::Snake;
use super::{same_ftile, Connection, Ftile, FtileGeometry, FtileGraphic, WeldingPoint};
use crate::activitydiagram3::rainbow::{LinkRendering, Rainbow};
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::drawing::ugraphic::UTranslate;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn tile_key(f: &dyn Ftile) -> usize {
    (f as *const dyn Ftile).cast::<()>() as usize
}

/// Two tiles one above the other (`FtileAssemblySimple`).
pub struct FtileAssemblySimple {
    tile1: Rc<dyn Ftile>,
    tile2: Rc<dyn Ftile>,
    geometry: RefCell<Option<FtileGeometry>>,
    translations: RefCell<HashMap<usize, Option<UTranslate>>>,
}

impl FtileAssemblySimple {
    pub fn new(tile1: Rc<dyn Ftile>, tile2: Rc<dyn Ftile>) -> Rc<Self> {
        Rc::new(Self {
            tile1,
            tile2,
            geometry: RefCell::new(None),
            translations: RefCell::new(HashMap::new()),
        })
    }

    fn translated1(&self, sb: &dyn StringBounder) -> UTranslate {
        let left = self.calculate_dimension(sb).left();
        UTranslate::dx(left - self.tile1.calculate_dimension(sb).left())
    }

    fn translated2(&self, sb: &dyn StringBounder) -> UTranslate {
        let dim1 = self.tile1.calculate_dimension(sb);
        let left = self.calculate_dimension(sb).left();
        UTranslate::new(
            left - self.tile2.calculate_dimension(sb).left(),
            dim1.height,
        )
    }

    fn translate_slow(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if same_ftile(child, self.tile1.as_ref()) {
            return Some(self.translated1(sb));
        }
        if same_ftile(child, self.tile2.as_ref()) {
            return Some(self.translated2(sb));
        }
        if let Some(t) = self.tile1.translate_for(child, sb) {
            return Some(t.compose(self.translated1(sb)));
        }
        self.tile2
            .translate_for(child, sb)
            .map(|t| t.compose(self.translated2(sb)))
    }
}

impl Ftile for FtileAssemblySimple {
    fn skin(&self) -> Rc<SkinParam> {
        self.tile1.skin()
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.tile1.in_link_rendering()
    }

    fn out_link_rendering(&self) -> LinkRendering {
        LinkRendering::none()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.tile1.swimlane_in()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.tile2.swimlane_out()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut r = self.tile1.swimlanes();
        for s in self.tile2.swimlanes() {
            if !r.iter().any(|x| Rc::ptr_eq(x, &s)) {
                r.push(s);
            }
        }
        r
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        if let Some(g) = *self.geometry.borrow() {
            return g;
        }
        let g = self
            .tile1
            .calculate_dimension(sb)
            .append_bottom(&self.tile2.calculate_dimension(sb));
        *self.geometry.borrow_mut() = Some(g);
        g
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let key = tile_key(child);
        if let Some(t) = self.translations.borrow().get(&key) {
            return *t;
        }
        let t = self.translate_slow(child, sb);
        self.translations.borrow_mut().insert(key, t);
        t
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let t1 = self.translate_for(self.tile1.as_ref(), sb).expect("tile1");
        let t2 = self.translate_for(self.tile2.as_ref(), sb).expect("tile2");
        g.apply_translate(t1).draw_ftile(self.tile1.as_ref());
        g.apply_translate(t2).draw_ftile(self.tile2.as_ref());
    }

    fn welding_points(&self) -> Vec<WeldingPoint> {
        let mut r = self.tile1.welding_points();
        r.extend(self.tile2.welding_points());
        r
    }

    fn my_children(&self) -> Vec<Rc<dyn Ftile>> {
        vec![self.tile1.clone(), self.tile2.clone()]
    }

    fn inner_connections(&self) -> Vec<Rc<dyn Connection>> {
        Vec::new()
    }

    fn arrow_horizontal_alignment(&self) -> crate::klimt::geom::HorizontalAlignment {
        self.tile1.arrow_horizontal_alignment()
    }
}

/// Changes what a tile announces without changing its look
/// (`FtileDecorateIn`, `FtileDecorateOut`, `FtileDecorateWelding`).
pub struct FtileDecorate {
    inner: Rc<dyn Ftile>,
    in_rendering: Option<LinkRendering>,
    out_rendering: Option<LinkRendering>,
    welding: Option<Vec<WeldingPoint>>,
}

impl FtileDecorate {
    pub fn decorate_in(inner: Rc<dyn Ftile>, link: LinkRendering) -> Rc<Self> {
        Rc::new(Self {
            inner,
            in_rendering: Some(link),
            out_rendering: None,
            welding: None,
        })
    }

    pub fn decorate_out(inner: Rc<dyn Ftile>, link: LinkRendering) -> Rc<Self> {
        Rc::new(Self {
            inner,
            in_rendering: None,
            out_rendering: Some(link),
            welding: None,
        })
    }

    pub fn welding(inner: Rc<dyn Ftile>, points: Vec<WeldingPoint>) -> Rc<Self> {
        Rc::new(Self {
            inner,
            in_rendering: None,
            out_rendering: None,
            welding: Some(points),
        })
    }
}

impl Ftile for FtileDecorate {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.inner.clone())
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_rendering
            .clone()
            .unwrap_or_else(|| self.inner.in_link_rendering())
    }

    fn out_link_rendering(&self) -> LinkRendering {
        self.out_rendering
            .clone()
            .unwrap_or_else(|| self.inner.out_link_rendering())
    }

    fn welding_points(&self) -> Vec<WeldingPoint> {
        self.welding
            .clone()
            .unwrap_or_else(|| self.inner.welding_points())
    }
}

/// Space above and below a tile (`FtileMargedVertically`).
pub struct FtileMargedVertically {
    inner: Rc<dyn Ftile>,
    margin1: f64,
    margin2: f64,
}

impl FtileMargedVertically {
    pub fn new(inner: Rc<dyn Ftile>, margin1: f64, margin2: f64) -> Rc<Self> {
        Rc::new(Self {
            inner,
            margin1,
            margin2,
        })
    }
}

impl Ftile for FtileMargedVertically {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        self.inner
            .calculate_dimension(sb)
            .inc_vertically(self.margin1, self.margin2)
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if same_ftile(child, self.inner.as_ref()) {
            return Some(UTranslate::dy(self.margin1));
        }
        self.inner
            .translate_for(child, sb)
            .map(|t| t.compose(UTranslate::dy(self.margin1)))
    }

    fn draw(&self, g: &FtileGraphic) {
        let g = if self.margin1 > 0.0 {
            g.apply_translate(UTranslate::dy(self.margin1))
        } else {
            g.clone()
        };
        g.draw_ftile(self.inner.as_ref());
    }
}

/// A tile widened on the right so that a label fits (`FtileMargedRight`).
pub struct FtileMargedRight {
    inner: Rc<dyn Ftile>,
    max_x: f64,
}

impl FtileMargedRight {
    pub fn new(inner: Rc<dyn Ftile>, max_x: f64) -> Rc<Self> {
        Rc::new(Self { inner, max_x })
    }
}

impl Ftile for FtileMargedRight {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let orig = self.inner.calculate_dimension(sb);
        assert!(orig.width <= self.max_x, "tile wider than its margin");
        FtileGeometry::with_out(
            self.max_x,
            orig.height,
            orig.left(),
            orig.in_y(),
            orig.out_y(),
        )
    }
}

/// A tile that also draws arrows between its children (`FtileWithConnection`).
pub struct FtileWithConnection {
    inner: Rc<dyn Ftile>,
    connections: Vec<Rc<dyn Connection>>,
}

impl FtileWithConnection {
    pub fn wrap(inner: Rc<dyn Ftile>, connections: Vec<Rc<dyn Connection>>) -> Rc<dyn Ftile> {
        if connections.is_empty() {
            return inner;
        }
        Rc::new(Self { inner, connections })
    }
}

impl Ftile for FtileWithConnection {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.inner.clone())
    }

    fn draw(&self, g: &FtileGraphic) {
        g.draw_ftile(self.inner.as_ref());
        for c in &self.connections {
            g.draw_connection(c.as_ref());
        }
    }

    fn inner_connections(&self) -> Vec<Rc<dyn Connection>> {
        self.connections.clone()
    }
}

/// The plain arrow from the exit of a tile to the entry of the next (`ConnectionVerticalDown`).
pub struct ConnectionVerticalDown {
    ftile1: Rc<dyn Ftile>,
    ftile2: Rc<dyn Ftile>,
    p1: (f64, f64),
    p2: (f64, f64),
    color: Rainbow,
    text_block: Option<Rc<dyn TextBlock>>,
}

impl ConnectionVerticalDown {
    pub fn new(
        ftile1: Rc<dyn Ftile>,
        ftile2: Rc<dyn Ftile>,
        p1: (f64, f64),
        p2: (f64, f64),
        color: Rainbow,
        text_block: Option<Rc<dyn TextBlock>>,
    ) -> Self {
        assert!(color.size() > 0, "empty rainbow");
        Self {
            ftile1,
            ftile2,
            p1,
            p2,
            color,
            text_block,
        }
    }

    fn simple_snake(&self) -> Snake {
        let skin = self.ftile1.skin();
        let arrows = super::snake::Arrows::from_skin(&skin);
        let mut snake = Snake::create(
            &skin,
            self.color.clone(),
            Some(arrows.as_to(crate::klimt::geom::Direction::Down)),
        )
        .with_label(
            self.text_block.clone(),
            self.ftile1.arrow_horizontal_alignment(),
        );
        snake.add_point(self.p1.0, self.p1.1);
        snake.add_point(self.p2.0, self.p2.1);
        snake
    }

    pub fn max_x(&self, sb: &dyn StringBounder) -> f64 {
        self.simple_snake().max_x(sb)
    }
}

impl Connection for ConnectionVerticalDown {
    fn ftile1(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.ftile1.clone())
    }

    fn ftile2(&self) -> Option<Rc<dyn Ftile>> {
        Some(self.ftile2.clone())
    }

    fn draw(&self, g: &FtileGraphic) {
        g.draw_snake(self.simple_snake());
    }

    fn draw_translate(&self, g: &FtileGraphic, t1: UTranslate, t2: UTranslate) {
        let skin = self.ftile1.skin();
        let arrows = super::snake::Arrows::from_skin(&skin);
        let mut snake = Snake::create(
            &skin,
            self.color.clone(),
            Some(arrows.as_to(crate::klimt::geom::Direction::Down)),
        )
        .with_label(
            self.text_block.clone(),
            self.ftile1.arrow_horizontal_alignment(),
        );
        let mp1 = (self.p1.0 + t1.dx, self.p1.1 + t1.dy);
        let mp2 = (self.p2.0 + t2.dx, self.p2.1 + t2.dy);
        let middle = (mp1.1 + mp2.1) / 2.0;
        snake.add_point(mp1.0, mp1.1);
        snake.add_point(mp1.0, middle);
        snake.add_point(mp2.0, middle);
        snake.add_point(mp2.0, mp2.1);
        g.draw_snake(snake);
    }
}
