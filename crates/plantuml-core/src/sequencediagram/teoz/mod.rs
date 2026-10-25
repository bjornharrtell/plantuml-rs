//! The Teoz layout of sequence diagrams (`sequencediagram/teoz`).

pub mod communication;
pub mod living;
pub mod playing;

use super::model::{Event, Participant};
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::StringBounder;
use crate::real::Real;
use crate::skin::SkinParam;
use living::LivingSpaces;
use std::cell::RefCell;
use std::rc::Rc;

/// Frozen content of a diagram used while laying out.
#[derive(Debug)]
pub struct Model {
    pub participants: Vec<Rc<Participant>>,
    pub events: Vec<Event>,
    pub show_footbox: bool,
    pub englobers: Vec<(usize, Option<Rc<super::model::ParticipantEnglober>>)>,
    pub link_anchors: Vec<super::model::LinkAnchor>,
}

/// Vertical extent of a tile, with the position of its contact point (`YGauge`).
#[derive(Clone)]
pub struct YGauge {
    min: Real,
    max: Real,
    contact: Option<Real>,
    origin: Option<Real>,
}

impl YGauge {
    pub fn new(min: Real, max: Real) -> Self {
        Self {
            min,
            max,
            contact: None,
            origin: None,
        }
    }

    pub fn create(min: &Real, height: f64) -> YGauge {
        assert!(height >= 0.0);
        YGauge::new(min.clone(), min.add_at_least(height))
    }

    pub fn create_with_contact(current: &YGauge, contact_relative: f64, height: f64) -> YGauge {
        assert!(height >= 0.0 && contact_relative >= 0.0);
        let origin = current.max.clone();
        let contact = origin.add_at_least(contact_relative);
        let min = contact.add_fixed(-contact_relative);
        let max = contact.add_at_least(height - contact_relative);
        YGauge {
            min,
            max,
            contact: Some(contact),
            origin: Some(origin),
        }
    }

    pub fn create_parallel(current: &YGauge, contact_relative: f64, height: f64) -> YGauge {
        let (Some(contact), Some(origin)) = (&current.contact, &current.origin) else {
            return YGauge::create(&current.min, height);
        };
        contact.ensure_bigger_than(&origin.add_fixed(contact_relative));
        let min = contact.add_fixed(-contact_relative);
        let max = contact.add_at_least(height - contact_relative);
        max.ensure_bigger_than(&current.max);
        YGauge {
            min,
            max,
            contact: Some(contact.clone()),
            origin: Some(origin.clone()),
        }
    }

    pub fn create_propagating(current: &YGauge, height: f64) -> YGauge {
        assert!(height >= 0.0);
        let min = current.max.clone();
        YGauge {
            max: min.add_at_least(height),
            min,
            contact: current.contact.clone(),
            origin: current.origin.clone(),
        }
    }

    pub fn min(&self) -> &Real {
        &self.min
    }

    pub fn max(&self) -> &Real {
        &self.max
    }

    pub fn contact(&self) -> Option<&Real> {
        self.contact.as_ref()
    }

    pub fn origin(&self) -> Option<&Real> {
        self.origin.as_ref()
    }
}

/// Horizontal extent of the whole drawing (`Bordered`).
pub trait Bordered {
    fn border1(&self) -> f64;
    fn border2(&self) -> f64;
}

/// Something drawn along the vertical flow of events (`Tile`).
pub trait Tile {
    fn preferred_height(&self) -> f64;
    fn on_gauge_resolved(&self) {}
    fn y_gauge(&self) -> YGauge;
    fn add_constraints(&self);
    fn min_x(&self) -> Real;
    fn max_x(&self) -> Real;
    fn drawn_min_x(&self) -> Real {
        self.min_x()
    }
    fn drawn_max_x(&self) -> Real {
        self.max_x()
    }
    fn middle_x(&self) -> f64 {
        (self.min_x().value() + self.max_x().value()) / 2.0
    }
    /// Index in the diagram's events.
    fn event(&self) -> Option<usize>;
    fn contact_point_relative(&self) -> f64 {
        -1.0
    }
    fn zzz(&self) -> f64 {
        self.preferred_height() - self.contact_point_relative()
    }
    fn match_anchor(&self, _anchor: &str) -> bool {
        false
    }
    fn draw(&self, ug: &UGraphic);
}

/// Everything tiles need to be built and drawn (`TileArguments`).
pub struct TileArguments {
    pub string_bounder: Rc<dyn StringBounder>,
    pub living_spaces: Rc<LivingSpaces>,
    pub skin_param: Rc<SkinParam>,
    pub xorigin: Real,
    pub yorigin: Real,
    pub model: Rc<Model>,
    bordered: RefCell<Option<Rc<dyn Bordered>>>,
}

impl TileArguments {
    pub fn new(
        string_bounder: Rc<dyn StringBounder>,
        living_spaces: Rc<LivingSpaces>,
        skin_param: Rc<SkinParam>,
        xorigin: Real,
        yorigin: Real,
        model: Rc<Model>,
    ) -> Self {
        Self {
            string_bounder,
            living_spaces,
            skin_param,
            xorigin,
            yorigin,
            model,
            bordered: RefCell::new(None),
        }
    }

    pub fn set_bordered(&self, b: Rc<dyn Bordered>) {
        *self.bordered.borrow_mut() = Some(b);
    }

    pub fn border1(&self) -> f64 {
        self.bordered.borrow().as_ref().expect("bordered").border1()
    }

    pub fn border2(&self) -> f64 {
        self.bordered.borrow().as_ref().expect("bordered").border2()
    }

    pub fn sb(&self) -> &dyn StringBounder {
        self.string_bounder.as_ref()
    }
}
