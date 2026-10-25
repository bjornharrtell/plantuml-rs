//! Activation events as tiles (`LifeEventTile`).

use super::communication::LIVE_DELTA_SIZE;
use super::living::{EventsHistoryMode, LivingSpace};
use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::real::Real;
use crate::sequencediagram::model::{Event, LifeEvent, LifeEventType};
use crate::skin::component::{Area, ComponentType};
use crate::skin::rose::Rose;
use std::rc::Rc;

pub struct LifeEventTile {
    args: Rc<TileArguments>,
    event: usize,
    life_event: LifeEvent,
    living_space: Rc<LivingSpace>,
    y_gauge: YGauge,
}

impl LifeEventTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<LifeEventTile> {
        let Event::LifeEvent(le) = &args.model.events[event] else {
            panic!("not a life event");
        };
        let life_event = le.clone();
        let living_space = args.living_spaces.get(life_event.participant);
        let mut tile = LifeEventTile {
            args,
            event,
            life_event,
            living_space,
            y_gauge: current_y.clone(),
        };
        tile.y_gauge = YGauge::create_propagating(current_y, tile.preferred_height());
        Rc::new(tile)
    }

    fn is_destroy_without_message(&self) -> bool {
        self.life_event.message.is_none() && self.life_event.kind == LifeEventType::Destroy
    }

    fn cross(&self) -> Rc<dyn crate::skin::component::Component> {
        Rose::create_component(
            &self.life_event.style(),
            ComponentType::Destroy,
            &self.args.skin_param,
            None,
        )
    }
}

impl Tile for LifeEventTile {
    fn preferred_height(&self) -> f64 {
        if self.is_destroy_without_message() {
            return self.cross().preferred_dimension(self.args.sb()).height;
        }
        0.0
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn on_gauge_resolved(&self) {
        let y = self.y_gauge.min().value();
        self.living_space.add_step_for_livebox(self.event, y);
        if self.life_event.kind == LifeEventType::Destroy {
            self.living_space.go_destroy(y);
        }
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        let level = self
            .living_space
            .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate);
        let adj = if level > 0 { LIVE_DELTA_SIZE } else { 0.0 };
        self.living_space.pos_c(self.args.sb()).add_fixed(-adj)
    }

    fn max_x(&self) -> Real {
        let level = self
            .living_space
            .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate);
        let adj = if level > 0 {
            level as f64 * LIVE_DELTA_SIZE
        } else {
            0.0
        };
        self.living_space.pos_c(self.args.sb()).add_fixed(adj)
    }

    fn stable_min_x(&self) -> Vec<Real> {
        vec![self.drawn_min_x()]
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        if self.is_destroy_without_message() {
            let cross = self.cross();
            let dim = cross.preferred_dimension(ug.string_bounder());
            let x = self.living_space.pos_c(ug.string_bounder()).value();
            cross.draw(
                &ug.apply_translate(UTranslate::dx(x - dim.width / 2.0)),
                &Area::default(),
                ug.is_background_layer(),
            );
        }
    }
}
