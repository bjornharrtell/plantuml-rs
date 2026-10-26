//! The symbols of `USymbols`, one constructor per field.

use super::{boxes, cloud as cloud_symbol, figures, folder as folder_symbol, hline, USymbol};
use crate::skin::actor::ActorStyle;
use crate::style::SName;
use std::rc::Rc;

pub fn action() -> Rc<dyn USymbol> {
    boxes::action(SName::Action)
}

pub fn actor_awesome() -> Rc<dyn USymbol> {
    figures::actor(ActorStyle::Awesome)
}

pub fn actor_hollow() -> Rc<dyn USymbol> {
    figures::actor(ActorStyle::Hollow)
}

pub fn actor_stickman() -> Rc<dyn USymbol> {
    figures::actor(ActorStyle::Stickman)
}

pub fn actor_stickman_business() -> Rc<dyn USymbol> {
    figures::actor_business()
}

pub fn agent() -> Rc<dyn USymbol> {
    boxes::rectangle(SName::Agent)
}

pub fn archimate() -> Rc<dyn USymbol> {
    boxes::rectangle(SName::Archimate)
}

pub fn artifact() -> Rc<dyn USymbol> {
    boxes::artifact()
}

pub fn boundary() -> Rc<dyn USymbol> {
    figures::boundary()
}

pub fn card() -> Rc<dyn USymbol> {
    boxes::card()
}

pub fn cloud() -> Rc<dyn USymbol> {
    cloud_symbol::cloud()
}

pub fn collections() -> Rc<dyn USymbol> {
    boxes::collections()
}

pub fn component_rectangle() -> Rc<dyn USymbol> {
    boxes::rectangle(SName::Component)
}

pub fn component1() -> Rc<dyn USymbol> {
    boxes::component1()
}

pub fn component2() -> Rc<dyn USymbol> {
    boxes::component2()
}

pub fn control() -> Rc<dyn USymbol> {
    figures::control()
}

pub fn database() -> Rc<dyn USymbol> {
    hline::database()
}

pub fn entity_domain() -> Rc<dyn USymbol> {
    figures::entity_domain()
}

pub fn file() -> Rc<dyn USymbol> {
    boxes::file()
}

pub fn folder() -> Rc<dyn USymbol> {
    folder_symbol::folder(SName::Folder, false)
}

pub fn frame() -> Rc<dyn USymbol> {
    folder_symbol::frame(SName::Frame)
}

pub fn group() -> Rc<dyn USymbol> {
    folder_symbol::frame(SName::Group)
}

pub fn hexagon() -> Rc<dyn USymbol> {
    boxes::hexagon()
}

pub fn interface() -> Rc<dyn USymbol> {
    figures::interface()
}

pub fn label() -> Rc<dyn USymbol> {
    boxes::label()
}

pub fn node() -> Rc<dyn USymbol> {
    hline::node()
}

pub fn package() -> Rc<dyn USymbol> {
    folder_symbol::folder(SName::Package, true)
}

pub fn partition() -> Rc<dyn USymbol> {
    folder_symbol::frame(SName::Partition)
}

pub fn person() -> Rc<dyn USymbol> {
    figures::person()
}

pub fn process() -> Rc<dyn USymbol> {
    boxes::process(SName::Process)
}

pub fn queue() -> Rc<dyn USymbol> {
    hline::queue()
}

pub fn rectangle() -> Rc<dyn USymbol> {
    boxes::rectangle(SName::Rectangle)
}

pub fn stack() -> Rc<dyn USymbol> {
    boxes::stack()
}

pub fn storage() -> Rc<dyn USymbol> {
    boxes::storage()
}

pub fn usecase() -> Rc<dyn USymbol> {
    figures::usecase(false)
}

pub fn usecase_business() -> Rc<dyn USymbol> {
    figures::usecase(true)
}
