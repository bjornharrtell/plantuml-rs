//! `ref over A, B` frames (`ReferenceTile`).

use super::{Tile, TileArguments, YGauge};
use crate::klimt::creole::display::{Display, DisplayLine};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::real::Real;
use crate::sequencediagram::model::{Event, NotePosition, Reference};
use crate::skin::component::{Area, Component};
use crate::skin::rose::Rose;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ReferenceTile {
    args: Rc<TileArguments>,
    event: usize,
    reference: Reference,
    extent: RefCell<Option<(Real, Real)>>,
    note_left: Option<Rc<dyn Component>>,
    note_right: Option<Rc<dyn Component>>,
    y_gauge: YGauge,
}

impl ReferenceTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<ReferenceTile> {
        let Event::Reference(reference) = &args.model.events[event] else {
            panic!("not a reference");
        };
        let reference = reference.clone();
        let mut note_left = None;
        let mut note_right = None;
        for note in &reference.notes {
            let c = Rose::create_component_note(
                &note.used_styles()[0],
                note.note_style.component_type(),
                &args.skin_param,
                &note.display,
                &note.colors,
                None,
            );
            if note.position == NotePosition::Right {
                note_right = Some(c);
            } else {
                note_left = Some(c);
            }
        }
        let mut tile = ReferenceTile {
            args,
            event,
            reference,
            extent: RefCell::new(None),
            note_left,
            note_right,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
        };
        tile.y_gauge = YGauge::create(current_y.max(), tile.preferred_height());
        Rc::new(tile)
    }

    fn component(&self) -> Rc<dyn Component> {
        let strings = Display::empty()
            .add(DisplayLine::Text("ref".to_string()))
            .add_all(&self.reference.strings);
        Rose::create_component_reference(
            &self.reference.used_styles(),
            &strings,
            &self.args.skin_param,
        )
    }

    fn init(&self) -> (Real, Real) {
        if let Some(e) = self.extent.borrow().as_ref() {
            return e.clone();
        }
        let sb = self.args.sb();
        let mut first: Option<Real> = None;
        let mut last: Option<Real> = None;
        for p in &self.reference.participants {
            let ls = self.args.living_spaces.get(*p);
            let pos = ls.pos_c(sb).value();
            if first.as_ref().is_none_or(|f| pos < f.value()) {
                first = Some(ls.pos_b());
            }
            if last.as_ref().is_none_or(|l| pos > l.value()) {
                last = Some(ls.pos_d(sb));
            }
        }
        let first = first.expect("participants");
        let mut last = last.expect("participants");
        let dim = self.component().preferred_dimension(sb);
        if self.reference.participants.len() == 1 {
            last = last.add_at_least(0.0);
        }
        last.ensure_bigger_than(&first.add_fixed(dim.width));
        *self.extent.borrow_mut() = Some((first.clone(), last.clone()));
        (first, last)
    }
}

impl Tile for ReferenceTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        let (first, _) = self.init();
        match &self.note_left {
            Some(n) => first.add_fixed(-n.preferred_width(self.args.sb())),
            None => first,
        }
    }

    fn max_x(&self) -> Real {
        let (_, last) = self.init();
        match &self.note_right {
            Some(n) => last.add_fixed(n.preferred_width(self.args.sb())),
            None => last,
        }
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn stable_min_x(&self) -> Vec<Real> {
        vec![self.drawn_min_x()]
    }

    fn stable_max_x(&self) -> Vec<Real> {
        vec![self.drawn_max_x()]
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let sb = ug.string_bounder();
        let (first, last) = self.init();
        let background = ug.is_background_layer();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let area = Area::create(last.value() - first.value(), dim.height);
        comp.draw(
            &ug.apply_translate(UTranslate::dx(first.value())),
            &area,
            background,
        );
        if let Some(n) = &self.note_left {
            let (w, h) = (n.preferred_width(sb), n.preferred_height(sb));
            n.draw(
                &ug.apply_translate(UTranslate::dx(first.value() - w)),
                &Area::create(w, h),
                background,
            );
        }
        if let Some(n) = &self.note_right {
            let (w, h) = (n.preferred_width(sb), n.preferred_height(sb));
            n.draw(
                &ug.apply_translate(UTranslate::dx(last.value())),
                &Area::create(w, h),
                background,
            );
        }
    }
}
