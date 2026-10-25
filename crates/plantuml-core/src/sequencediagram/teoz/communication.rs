//! Message between two participants (`CommunicationTile`).

use super::living::{EventsHistoryMode, LivingSpace};
use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::real::{self, Real};
use crate::sequencediagram::model::{Event, Message};
use crate::skin::component::{Area, ArrowComponent};
use crate::skin::rose::Rose;
use std::cell::RefCell;
use std::rc::Rc;

pub const LIVE_DELTA_SIZE: f64 = 5.0;

pub struct CommunicationTile {
    args: Rc<TileArguments>,
    event: usize,
    message: Message,
    living_space1: Rc<LivingSpace>,
    living_space2: Rc<LivingSpace>,
    y_gauge: YGauge,
    cached: [RefCell<Option<Rc<dyn ArrowComponent>>>; 2],
}

impl CommunicationTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<CommunicationTile> {
        let Event::Message(message) = &args.model.events[event] else {
            panic!("not a message");
        };
        let message = message.clone();
        let ls1 = args.living_spaces.get(message.p1);
        let ls2 = args.living_spaces.get(message.p2);
        if message.core.is_create() {
            ls2.go_create();
        }
        let mut tile = CommunicationTile {
            args,
            event,
            message,
            living_space1: ls1,
            living_space2: ls2,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
            cached: [RefCell::new(None), RefCell::new(None)],
        };
        let comp = tile.component();
        let sb = tile.args.sb();
        let contact_relative = comp.y_point(sb);
        let height = tile.preferred_height();
        tile.y_gauge = if tile.message.core.parallel {
            YGauge::create_parallel(current_y, contact_relative, height)
        } else {
            YGauge::create_with_contact(current_y, contact_relative, height)
        };
        Rc::new(tile)
    }

    pub fn is_reverse(&self) -> bool {
        let sb = self.args.sb();
        self.living_space1.pos_c(sb).value() > self.living_space2.pos_c(sb).value()
    }

    fn is_create(&self) -> bool {
        self.message.core.is_create()
    }

    fn arrow_thickness(&self) -> f64 {
        1.0
    }

    fn component(&self) -> Rc<dyn ArrowComponent> {
        let reverse = self.is_reverse();
        let idx = usize::from(reverse);
        if let Some(c) = self.cached[idx].borrow().as_ref() {
            return c.clone();
        }
        let mut config = self.message.core.arrow_configuration.clone();
        if reverse {
            config = config.reverse();
        }
        config = config.with_thickness(self.arrow_thickness());
        let comp = Rose::create_component_arrow(
            &self.message.core.used_style(),
            &config,
            &self.args.skin_param,
            &self.message.core.label_numbered(),
        );
        *self.cached[idx].borrow_mut() = Some(comp.clone());
        comp
    }

    fn point1(&self) -> Real {
        self.living_space1.pos_c(self.args.sb())
    }

    fn point2(&self) -> Real {
        let sb = self.args.sb();
        if self.message.core.is_create() {
            if self.is_reverse() {
                return self.living_space2.pos_d(sb);
            }
            return self.living_space2.pos_b();
        }
        self.living_space2.pos_c(sb)
    }
}

impl Tile for CommunicationTile {
    fn preferred_height(&self) -> f64 {
        let comp = self.component();
        let sb = self.args.sb();
        let mut height = comp.preferred_dimension(sb).height;
        if self.is_create() {
            height = height.max(self.living_space2.head_preferred_dimension(sb).height);
        }
        height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn on_gauge_resolved(&self) {
        let y = self.y_gauge.min().value();
        if self.is_create() {
            self.living_space2.go_create_at(y);
        }
        let sb = self.args.sb();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let arrow_y = comp.start_point(sb, dim).1;
        self.living_space1
            .add_step_for_livebox(self.event, y + arrow_y);
        self.living_space2
            .add_step_for_livebox(self.event, y + arrow_y);
    }

    fn add_constraints(&self) {
        let sb = self.args.sb();
        let width = self.component().preferred_dimension(sb).width;
        let mut p1 = self.point1();
        let mut p2 = self.point2();
        if self.is_reverse() {
            let level1 = self
                .living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            let level2 = self
                .living_space2
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            if level1 > 0 {
                p1 = p1.add_fixed(-LIVE_DELTA_SIZE);
            }
            p2 = p2.add_fixed(level2 as f64 * LIVE_DELTA_SIZE);
            p1.ensure_bigger_than(&p2.add_fixed(width));
        } else {
            let level2 = self
                .living_space2
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            if level2 > 0 {
                p2 = p2.add_fixed(-LIVE_DELTA_SIZE);
            }
            p2.ensure_bigger_than(&p1.add_fixed(width));
        }
    }

    fn min_x(&self) -> Real {
        if self.is_reverse() {
            self.point2()
        } else {
            self.point1()
        }
    }

    fn max_x(&self) -> Real {
        let sb = self.args.sb();
        let width = self.component().preferred_dimension(sb).width;
        let (left, right) = if self.is_reverse() {
            (self.point2(), self.point1())
        } else {
            (self.point1(), self.point2())
        };
        real::max(&[right, left.add_fixed(width)])
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
        if self.message.core.part1_anchor.is_some() || self.message.core.part2_anchor.is_some() {
            return;
        }
        let sb = ug.string_bounder();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let mut x1 = self.point1().value();
        let mut x2 = self.point2().value();
        let background = ug.is_background_layer();
        let area;
        let mut ug = ug;
        if self.is_reverse() {
            let level1 = self
                .living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            let level2 = self
                .living_space2
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            if level1 == 1 {
                x1 -= LIVE_DELTA_SIZE;
            } else if level1 > 2 {
                x1 += LIVE_DELTA_SIZE * (level1 - 2) as f64;
            }
            x2 += LIVE_DELTA_SIZE * level2 as f64;
            area = Area::create(x1 - x2, dim.height);
            ug = ug.apply_translate(UTranslate::dx(x2));
            if self.is_create() {
                self.living_space2.draw_head(
                    &ug,
                    background,
                    VerticalAlignment::Top,
                    HorizontalAlignment::Right,
                );
            }
        } else {
            let level1 = self
                .living_space1
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
            let mut level2 = self
                .living_space2
                .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate)
                as i64;
            if level2 > 0 {
                level2 -= 2;
            }
            x1 += LIVE_DELTA_SIZE * level1 as f64;
            x2 += LIVE_DELTA_SIZE * level2 as f64;
            area = Area::create(x2 - x1, dim.height);
            ug = ug.apply_translate(UTranslate::dx(x1));
            if self.is_create() {
                self.living_space2.draw_head(
                    &ug.apply_translate(UTranslate::dx(area.dimension_to_use.width)),
                    background,
                    VerticalAlignment::Top,
                    HorizontalAlignment::Left,
                );
            }
        }
        ug.start_group(&message_group_attributes(&self.message, &self.args));
        comp.draw(&ug, &area, background);
        ug.close_group();
    }
}

/// Attributes of the SVG group around a message (`groupTypeMessage`).
pub fn message_group_attributes(m: &Message, args: &TileArguments) -> Vec<(&'static str, String)> {
    group_attributes(&m.core, m.p1, m.p2, args)
}

/// Same for any message given its core and both participants.
pub fn group_attributes(
    core: &crate::sequencediagram::model::MessageCore,
    p1: usize,
    p2: usize,
    args: &TileArguments,
) -> Vec<(&'static str, String)> {
    let mut v = vec![("class", "message".to_string())];
    let uid = |id: usize| {
        args.model
            .participants
            .iter()
            .find(|p| p.id == id)
            .expect("participant")
            .uid
            .clone()
    };
    v.push(("data-entity-1", uid(p1)));
    v.push(("data-entity-2", uid(p2)));
    v.push(("id", core.uid.clone()));
    if let Some(l) = core.location {
        v.push(("data-source-line", l.position.to_string()));
    }
    v
}
