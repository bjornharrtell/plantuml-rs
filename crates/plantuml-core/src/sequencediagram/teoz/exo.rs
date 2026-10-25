//! Message from or to the border of the diagram (`CommunicationExoTile`).

use super::communication::{group_attributes, LIVE_DELTA_SIZE};
use super::living::{EventsHistoryMode, LivingSpace};
use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::real::Real;
use crate::sequencediagram::model::{Event, MessageExo, MessageExoType};
use crate::skin::arrow::ArrowDecoration;
use crate::skin::component::{Area, ArrowComponent};
use crate::skin::rose::arrow::DIAM_CIRCLE;
use crate::skin::rose::Rose;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CommunicationExoTile {
    args: Rc<TileArguments>,
    event: usize,
    message: MessageExo,
    living_space: Rc<LivingSpace>,
    y_gauge: YGauge,
    cached: RefCell<Option<Rc<dyn ArrowComponent>>>,
}

impl CommunicationExoTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<Self> {
        let Event::MessageExo(message) = &args.model.events[event] else {
            panic!("not an exo message");
        };
        let message = message.clone();
        let living_space = args.living_spaces.get(message.participant);
        let mut tile = CommunicationExoTile {
            args,
            event,
            message,
            living_space,
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
        let mut config = self.message.core.arrow_configuration.clone();
        if self.message.exo_type.direction() == -1 {
            config = config.reverse();
        }
        let comp = Rose::create_component_arrow(
            &self.message.core.used_style(),
            &config,
            &self.args.skin_param,
            &self.message.core.label_numbered(),
        );
        *self.cached.borrow_mut() = Some(comp.clone());
        comp
    }

    fn preferred_width(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).width
    }

    fn point1(&self) -> Real {
        let sb = self.args.sb();
        if self.message.exo_type.is_right_border() {
            return self.living_space.pos_c(sb);
        }
        self.living_space
            .pos_c(sb)
            .add_fixed(-self.preferred_width())
    }

    fn point2(&self) -> Real {
        let sb = self.args.sb();
        if self.message.exo_type.is_left_border() {
            return self.living_space.pos_c(sb);
        }
        self.point1().add_fixed(self.preferred_width())
    }

    fn is_from_right_border(&self) -> bool {
        self.message.exo_type.is_right_border() && !self.message.short_arrow
    }

    fn is_from_left_border(&self) -> bool {
        self.message.exo_type.is_left_border() && !self.message.short_arrow
    }

    fn point1_value(&self) -> f64 {
        if self.is_from_left_border() {
            return self.args.border1();
        }
        self.point1().value()
    }

    fn point2_value(&self) -> f64 {
        if self.is_from_right_border() {
            return self.args.border2();
        }
        self.point2().value()
    }
}

impl Tile for CommunicationExoTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn on_gauge_resolved(&self) {
        let sb = self.args.sb();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let arrow_y = comp.start_point(sb, dim).1;
        self.living_space
            .add_step_for_livebox(self.event, self.y_gauge.min().value() + arrow_y);
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {
        if self.message.exo_type.is_right_border() {
            return;
        }
        self.living_space
            .pos_c(self.args.sb())
            .ensure_bigger_than(&self.args.xorigin.add_fixed(self.preferred_width()));
    }

    fn min_x(&self) -> Real {
        self.point1()
    }

    fn max_x(&self) -> Real {
        self.point2()
    }

    fn middle_x(&self) -> f64 {
        if !self.is_from_left_border() {
            return (self.min_x().value() + self.max_x().value()) / 2.0;
        }
        let min = self.point1_value();
        let max = min + self.preferred_width();
        (min + max) / 2.0
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
        let comp = self.component();
        let dim = comp.preferred_dimension(ug.string_bounder());
        let mut x1 = self.point1_value();
        let mut x2 = self.point2_value();
        let text_delta_x = if self.is_from_left_border() {
            self.point1().value() - x1
        } else if self.is_from_right_border() {
            self.point2().value() - x2
        } else {
            0.0
        };
        let level = self
            .living_space
            .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
        if level > 0 {
            if self.message.exo_type.is_right_border() {
                x1 += LIVE_DELTA_SIZE * level as f64;
            } else {
                x2 += LIVE_DELTA_SIZE * (level as f64 - 2.0);
            }
        }
        let config = &self.message.core.arrow_configuration;
        let exo_type = self.message.exo_type;
        let shift = DIAM_CIRCLE / 2.0 + 2.0;
        if config.decoration1() == ArrowDecoration::Circle && exo_type == MessageExoType::FromLeft {
            x1 += shift;
        }
        if config.decoration2() == ArrowDecoration::Circle && exo_type == MessageExoType::ToLeft {
            x1 += shift;
        }
        if config.decoration2() == ArrowDecoration::Circle && exo_type == MessageExoType::ToRight {
            x2 -= shift;
        }
        if config.decoration1() == ArrowDecoration::Circle && exo_type == MessageExoType::FromRight
        {
            x2 -= shift;
        }
        let mut area = Area::create(x2 - x1, dim.height);
        area.text_delta_x = text_delta_x;
        let ug = ug.apply_translate(UTranslate::dx(x1));
        ug.start_group(&group_attributes(
            &self.message.core,
            self.message.participant,
            self.message.participant,
            &self.args,
        ));
        comp.draw(&ug, &area, ug.is_background_layer());
        ug.close_group();
    }
}
