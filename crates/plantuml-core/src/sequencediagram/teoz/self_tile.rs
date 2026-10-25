//! Message from a participant to itself (`CommunicationTileSelf`).

use super::communication::{message_group_attributes, LIVE_DELTA_SIZE};
use super::living::{EventsHistoryMode, LivingSpace};
use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::real::Real;
use crate::sequencediagram::model::{Event, Message};
use crate::skin::component::{Area, ArrowComponent};
use crate::skin::rose::Rose;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CommunicationTileSelf {
    args: Rc<TileArguments>,
    event: usize,
    message: Message,
    living_space1: Rc<LivingSpace>,
    y_gauge: YGauge,
    cached: RefCell<Option<Rc<dyn ArrowComponent>>>,
}

impl CommunicationTileSelf {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<Self> {
        let Event::Message(message) = &args.model.events[event] else {
            panic!("not a message");
        };
        let message = message.clone();
        let ls1 = args.living_spaces.get(message.p1);
        let mut tile = CommunicationTileSelf {
            args,
            event,
            message,
            living_space1: ls1,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
            cached: RefCell::new(None),
        };
        let contact = tile.contact_point_relative();
        let height = tile.preferred_height();
        tile.y_gauge = if tile.message.core.parallel {
            YGauge::create_parallel(current_y, contact, height)
        } else {
            YGauge::create_with_contact(current_y, contact, height)
        };
        Rc::new(tile)
    }

    fn component(&self) -> Rc<dyn ArrowComponent> {
        if let Some(c) = self.cached.borrow().as_ref() {
            return c.clone();
        }
        let config = self.message.core.arrow_configuration.self_arrow();
        let comp = Rose::create_component_arrow(
            &self.message.core.used_style(),
            &config,
            &self.args.skin_param,
            &self.message.core.label_numbered(),
        );
        *self.cached.borrow_mut() = Some(comp.clone());
        comp
    }

    fn is_reverse_define(&self) -> bool {
        self.message.core.arrow_configuration.is_reverse_define()
    }

    fn comp_width(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).width
    }

    fn drawn_width(&self) -> f64 {
        self.component().drawn_width(self.args.sb())
    }

    fn ensure_own_box_clears_loop(&self) {
        let sb = self.args.sb();
        if self.is_reverse_define() {
            let overflow = self.living_space1.pos_b().value() - self.min_x().value();
            if overflow > 0.0 {
                self.living_space1.ensure_margin_before(overflow);
            }
        } else {
            let overflow = self.max_x().value() - self.living_space1.pos_d(sb).value();
            if overflow > 0.0 {
                self.living_space1.ensure_margin_after(overflow);
            }
        }
    }
}

impl Tile for CommunicationTileSelf {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn on_gauge_resolved(&self) {
        let sb = self.args.sb();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let p1 = comp.start_point(sb, dim);
        let p2 = comp.end_point(sb, dim);
        let y = self.y_gauge.min().value();
        let core = &self.message.core;
        if core.is_activate() {
            self.living_space1
                .add_step_for_livebox(self.event, y + p2.1);
        } else if core.is_deactivate() {
            self.living_space1
                .add_step_for_livebox(self.event, y + p1.1);
        } else if core.is_destroy() {
            self.living_space1
                .add_step_for_livebox(self.event, y + p2.1);
        }
    }

    fn add_constraints(&self) {
        let sb = self.args.sb();
        let spaces = &self.args.living_spaces;
        if self.is_reverse_define() {
            if let Some(previous) = spaces.previous(&self.living_space1) {
                self.living_space1
                    .pos_c(sb)
                    .ensure_bigger_than(&previous.pos_c2(sb).add_fixed(self.comp_width()));
            }
        } else if let Some(next) = spaces.next(&self.living_space1) {
            next.pos_c(sb).ensure_bigger_than(&self.max_x());
        }
        self.ensure_own_box_clears_loop();
    }

    fn min_x(&self) -> Real {
        let sb = self.args.sb();
        if self.is_reverse_define() {
            let adjustment = if self
                .living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate)
                > 0
            {
                LIVE_DELTA_SIZE
            } else {
                0.0
            };
            return self
                .living_space1
                .pos_c(sb)
                .add_fixed(-self.comp_width() - adjustment);
        }
        self.living_space1.pos_c(sb)
    }

    fn max_x(&self) -> Real {
        let sb = self.args.sb();
        if self.is_reverse_define() {
            return self.living_space1.pos_c2(sb);
        }
        self.living_space1.pos_c2(sb).add_fixed(self.comp_width())
    }

    fn drawn_min_x(&self) -> Real {
        let sb = self.args.sb();
        if self.is_reverse_define() {
            let adjustment = if self
                .living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate)
                > 0
            {
                LIVE_DELTA_SIZE
            } else {
                0.0
            };
            return self
                .living_space1
                .pos_c(sb)
                .add_fixed(-self.drawn_width() - adjustment);
        }
        self.min_x()
    }

    fn drawn_max_x(&self) -> Real {
        if self.is_reverse_define() {
            return self.max_x();
        }
        self.living_space1
            .pos_c2(self.args.sb())
            .add_fixed(self.drawn_width())
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
        self.component().y_point(self.args.sb())
    }

    fn match_anchor(&self, anchor: &str) -> bool {
        self.message.core.anchor.as_deref() == Some(anchor)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let sb = ug.string_bounder();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let mut x1 = self.min_x().value();
        let level_ignore =
            self.living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureActivate) as i32;
        let level_consider = self
            .living_space1
            .level_at(self.event, EventsHistoryMode::ConsiderFutureDeactivate)
            as i32;
        if !self.is_reverse_define() {
            x1 += LIVE_DELTA_SIZE * f64::from(level_ignore);
            if level_ignore < level_consider {
                x1 += LIVE_DELTA_SIZE * f64::from(level_consider - level_ignore);
            }
        }
        let mut area = Area::create(dim.width, dim.height);
        area.delta_x1 = f64::from(level_ignore - level_consider) * LIVE_DELTA_SIZE;
        area.level = level_ignore;
        area.live_delta_size = LIVE_DELTA_SIZE;
        let ug = ug.apply_translate(UTranslate::dx(x1));
        ug.start_group(&message_group_attributes(&self.message, &self.args));
        comp.draw(&ug, &area, ug.is_background_layer());
        ug.close_group();
    }
}
