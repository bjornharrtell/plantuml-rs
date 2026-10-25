//! Tiles without participants: divider, delay, extra space and new page.

use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::real::{self, Real};
use crate::sequencediagram::model::Event;
use crate::skin::component::{Area, Component, ComponentType};
use crate::skin::rose::Rose;
use std::rc::Rc;

pub struct DividerTile {
    args: Rc<TileArguments>,
    event: usize,
    y_gauge: YGauge,
}

impl DividerTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<DividerTile> {
        let mut t = DividerTile {
            args,
            event,
            y_gauge: current_y.clone(),
        };
        t.y_gauge = YGauge::create(current_y.max(), t.preferred_height());
        Rc::new(t)
    }

    fn component(&self) -> Rc<dyn Component> {
        let Event::Divider {
            text,
            style_builder,
            ..
        } = &self.args.model.events[self.event]
        else {
            unreachable!()
        };
        let style = style_builder
            .merged_style(&ComponentType::Divider.style_query())
            .expect("divider style");
        Rose::create_component(
            &style,
            ComponentType::Divider,
            &self.args.skin_param,
            Some(text),
        )
    }
}

impl Tile for DividerTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.args.xorigin.clone()
    }

    fn max_x(&self) -> Real {
        let w = self.component().preferred_dimension(self.args.sb()).width;
        self.args.xorigin.add_fixed(w)
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let comp = self.component();
        let dim = comp.preferred_dimension(ug.string_bounder());
        let area = Area::create(
            self.args.border2() - self.args.border1() - self.args.xorigin.value(),
            dim.height,
        );
        let ug = ug.apply_translate(UTranslate::dx(self.args.border1()));
        comp.draw(&ug, &area, ug.is_background_layer());
    }
}

pub struct DelayTile {
    args: Rc<TileArguments>,
    event: usize,
    y_gauge: YGauge,
}

impl DelayTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<DelayTile> {
        let mut t = DelayTile {
            args,
            event,
            y_gauge: current_y.clone(),
        };
        t.y_gauge = YGauge::create(current_y.max(), t.preferred_height());
        Rc::new(t)
    }

    fn component(&self) -> Rc<dyn Component> {
        let Event::Delay {
            text,
            style_builder,
        } = &self.args.model.events[self.event]
        else {
            unreachable!()
        };
        let style = style_builder
            .merged_style(&ComponentType::DelayText.style_query())
            .expect("delay style");
        Rose::create_component(
            &style,
            ComponentType::DelayText,
            &self.args.skin_param,
            Some(text),
        )
    }

    fn middle(&self) -> Real {
        let sb = self.args.sb();
        let spaces = self.args.living_spaces.values();
        let first = spaces.first().expect("participant").pos_c(sb);
        let last = spaces.last().expect("participant").pos_c(sb);
        real::middle(&first, &last)
    }

    fn width(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).width
    }
}

impl Tile for DelayTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.middle().add_fixed(-self.width() / 2.0)
    }

    fn max_x(&self) -> Real {
        self.middle().add_fixed(self.width() / 2.0)
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn draw(&self, ug: &UGraphic) {
        let comp = self.component();
        let dim = comp.preferred_dimension(ug.string_bounder());
        let area = Area::create(self.width(), dim.height);
        let ypos = self.y_gauge.min().value();
        let ug = ug.apply_translate(UTranslate::dy(ypos));
        self.args.living_spaces.delay_on(ypos, dim.height);
        let ug = ug.apply_translate(UTranslate::dx(self.min_x().value()));
        comp.draw(&ug, &area, ug.is_background_layer());
    }
}

pub struct HSpaceTile {
    args: Rc<TileArguments>,
    event: usize,
    pixel: f64,
    y_gauge: YGauge,
}

impl HSpaceTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<HSpaceTile> {
        let Event::HSpace(p) = &args.model.events[event] else {
            unreachable!()
        };
        let pixel = f64::from(*p);
        Rc::new(HSpaceTile {
            y_gauge: YGauge::create_propagating(current_y, pixel),
            args,
            event,
            pixel,
        })
    }
}

impl Tile for HSpaceTile {
    fn preferred_height(&self) -> f64 {
        self.pixel
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.args.xorigin.clone()
    }

    fn max_x(&self) -> Real {
        self.args.xorigin.add_fixed(10.0)
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn draw(&self, _: &UGraphic) {}
}

const MARGIN_Y: f64 = 10.0;

pub struct NewpageTile {
    args: Rc<TileArguments>,
    event: usize,
    y_gauge: YGauge,
}

impl NewpageTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<NewpageTile> {
        let mut t = NewpageTile {
            args,
            event,
            y_gauge: current_y.clone(),
        };
        t.y_gauge = YGauge::create(current_y.max(), t.preferred_height());
        Rc::new(t)
    }

    fn component(&self) -> Rc<dyn Component> {
        let Event::Newpage { style_builder } = &self.args.model.events[self.event] else {
            unreachable!()
        };
        let style = style_builder
            .merged_style(&ComponentType::Newpage.style_query())
            .expect("newpage style");
        Rose::create_component(&style, ComponentType::Newpage, &self.args.skin_param, None)
    }
}

impl Tile for NewpageTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_height(self.args.sb()) + 2.0 * MARGIN_Y
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.args.xorigin.clone()
    }

    fn max_x(&self) -> Real {
        self.args.xorigin.clone()
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn draw(&self, ug: &UGraphic) {
        if ug.is_background_layer() {
            return;
        }
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let comp = self.component();
        let area = Area::create(
            self.args.border2() - self.args.border1() - self.args.xorigin.value(),
            comp.preferred_height(self.args.sb()),
        );
        let ug = ug.apply_translate(UTranslate::new(self.args.border1(), MARGIN_Y));
        comp.draw(&ug, &area, false);
    }
}
