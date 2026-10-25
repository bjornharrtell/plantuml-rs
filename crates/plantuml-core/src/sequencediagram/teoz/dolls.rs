//! Boxes around participants (`Dolls`, `Doll`).

use super::TileArguments;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::real::{self, Real};
use crate::sequencediagram::model::ParticipantEnglober;
use crate::skin::component::{Area, Component, ComponentType};
use crate::skin::rose::Rose;
use crate::style::{PName, Style};
use std::rc::Rc;

/// One box and the participants it surrounds.
struct Doll {
    englober: Rc<ParticipantEnglober>,
    participants: Vec<usize>,
    args: Rc<TileArguments>,
}

impl Doll {
    fn used_style(&self) -> Style {
        let query = ComponentType::Englober
            .style_query()
            .with_stereotype_obj(self.englober.stereotype.as_ref());
        let style = self
            .args
            .skin_param
            .style_builder()
            .merged_style(&query)
            .expect("box style");
        style.override_color(PName::BackGroundColor, self.englober.box_color.clone())
    }

    fn component(&self) -> Rc<dyn Component> {
        Rose::create_component_englober(
            &self.used_style(),
            &self.englober.title,
            &self.args.skin_param,
        )
    }

    fn title_preferred_height(&self) -> f64 {
        self.component().preferred_height(self.args.sb())
    }

    fn first_living_space(&self) -> Rc<super::living::LivingSpace> {
        self.args.living_spaces.get(self.participants[0])
    }

    fn last_living_space(&self) -> Rc<super::living::LivingSpace> {
        self.args
            .living_spaces
            .get(*self.participants.last().expect("participant"))
    }

    fn pos_aa(&self, sb: &dyn StringBounder) -> Real {
        match self.args.living_spaces.previous(&self.first_living_space()) {
            None => self.args.xorigin.clone(),
            Some(previous) => previous.pos_d(sb),
        }
    }

    fn title_width(&self) -> f64 {
        self.component().preferred_width(self.args.sb())
    }

    fn min_x(&self) -> Real {
        self.first_living_space().pos_a()
    }

    fn max_x(&self, sb: &dyn StringBounder) -> Real {
        self.last_living_space().pos_e(sb).add_fixed(10.0)
    }
}

/// All boxes of the diagram, by englober.
pub struct Dolls {
    alls: Vec<Doll>,
    args: Rc<TileArguments>,
}

impl Dolls {
    pub fn new(args: &Rc<TileArguments>) -> Dolls {
        let mut alls: Vec<Doll> = Vec::new();
        for p in args.living_spaces.participants() {
            let Some(englober) = args.living_spaces.get(p).englober().cloned() else {
                continue;
            };
            for pe in englober.genealogy() {
                match alls.iter_mut().find(|d| Rc::ptr_eq(&d.englober, &pe)) {
                    Some(doll) => doll.participants.push(p),
                    None => alls.push(Doll {
                        englober: pe,
                        participants: vec![p],
                        args: args.clone(),
                    }),
                }
            }
        }
        Dolls {
            alls,
            args: args.clone(),
        }
    }

    pub fn size(&self) -> usize {
        self.alls.len()
    }

    fn parent_of(&self, doll: &Doll) -> Option<&Doll> {
        let parent = doll.englober.parent.as_ref()?;
        self.alls.iter().find(|d| Rc::ptr_eq(&d.englober, parent))
    }

    /// Height taken above the diagram by the titles of the boxes (`getOffsetForEnglobers`).
    pub fn offset_for_englobers(&self) -> f64 {
        let mut result: f64 = 0.0;
        for doll in &self.alls {
            let mut height = 0.0;
            let mut current = Some(doll);
            while let Some(d) = current {
                height += d.title_preferred_height();
                current = self.parent_of(d);
            }
            result = result.max(height);
        }
        result
    }

    pub fn add_constraints(&self) {
        let sb = self.args.sb();
        for doll in &self.alls {
            let title_width = doll.title_width();
            let x1 = doll.first_living_space().pos_b().value();
            let x2 = doll.last_living_space().pos_d(sb).value();
            let margin_x = (title_width + 10.0 - (x2 - x1)) / 2.0;
            if margin_x > 0.0 {
                doll.first_living_space().ensure_margin_before(margin_x);
                doll.last_living_space().ensure_margin_after(margin_x);
            }
        }
        let padding = self.args.skin_param.box_padding();
        for doll in &self.alls {
            doll.first_living_space()
                .pos_a_live()
                .ensure_bigger_than(&doll.pos_aa(sb).add_fixed(10.0 + padding.left));
        }
        for doll in &self.alls {
            let Some(next) = self.args.living_spaces.next(&doll.last_living_space()) else {
                continue;
            };
            next.pos_a_live().ensure_bigger_than(
                &doll
                    .last_living_space()
                    .pos_e_live(sb)
                    .add_fixed(20.0 + padding.left + padding.right),
            );
        }
    }

    pub fn draw_englobers(&self, ug: &UGraphic, height: f64) {
        let sb = ug.string_bounder();
        for doll in &self.alls {
            let x1 = doll.first_living_space().pos_a().value() - 4.0;
            let x2 = doll.last_living_space().pos_e(sb).value() + 4.0;
            let mut ug = ug.clone();
            let mut height = height;
            let mut current = self.parent_of(doll);
            while let Some(d) = current {
                let h = d.title_preferred_height();
                ug = ug.apply_translate(UTranslate::dy(h));
                height -= h;
                current = self.parent_of(d);
            }
            let area = Area::new(Dimension::new(x2 - x1, height));
            doll.component()
                .draw(&ug.apply_translate(UTranslate::new(x1, 1.0)), &area, true);
        }
    }

    pub fn min_x(&self) -> Real {
        let all: Vec<Real> = self.alls.iter().map(Doll::min_x).collect();
        real::min(&all)
    }

    pub fn max_x(&self) -> Real {
        let sb = self.args.sb();
        let all: Vec<Real> = self.alls.iter().map(|d| d.max_x(sb)).collect();
        real::max(&all)
    }
}
