//! Notes: free notes, merged notes and notes attached to a message (`NoteTile`, `NotesTile`,
//! `CommunicationTileNoteLeft/Right/Top/Bottom`, `CommunicationTileSelfNoteLeft/Right`).

use super::communication::LIVE_DELTA_SIZE;
use super::living::{EventsHistoryMode, LivingSpace};
use super::{Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UStroke, UTranslate};
use crate::real::{self, Real};
use crate::sequencediagram::model::{Event, Note, NotePosition};
use crate::skin::component::{Area, Component, ComponentType};
use crate::skin::rose::Rose;
use crate::style::{queries, PName};
use std::cell::RefCell;
use std::rc::Rc;

const SPACE_Y: f64 = 10.0;
/// `Rose.paddingY`.
const PADDING_Y: f64 = 5.0;

fn note_component(args: &TileArguments, note: &Note, with_position: bool) -> Rc<dyn Component> {
    Rose::create_component_note(
        &note.used_styles()[0],
        note.note_style.component_type(),
        &args.skin_param,
        &note.display,
        &note.colors,
        with_position.then_some(note.position),
    )
}

/// Plain note component, used by notes of messages with a box that is always `Note`.
fn folded_note_component(args: &TileArguments, note: &Note) -> Rc<dyn Component> {
    Rose::create_component_note(
        &note.used_styles()[0],
        ComponentType::Note,
        &args.skin_param,
        &note.display,
        &note.colors,
        None,
    )
}

/// Note standing on its own in the flow (`NoteTile`).
pub struct NoteTile {
    args: Rc<TileArguments>,
    event: usize,
    note: Note,
    ls1: Rc<LivingSpace>,
    ls2: Option<Rc<LivingSpace>>,
    y_gauge: YGauge,
    cached: RefCell<Option<Rc<dyn Component>>>,
}

impl NoteTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<NoteTile> {
        let Event::Note(note) = &args.model.events[event] else {
            panic!("not a note");
        };
        let note = note.clone();
        let mut ls1 = note.p.map(|p| args.living_spaces.get(p));
        let mut ls2 = note.p2.map(|p| args.living_spaces.get(p));
        if ls1.is_none() && ls2.is_none() {
            let all = args.living_spaces.values();
            ls1 = all.first().cloned();
            ls2 = all.last().cloned();
        }
        let ls1 = ls1.expect("living space");
        let mut tile = NoteTile {
            args,
            event,
            note,
            ls1,
            ls2,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
            cached: RefCell::new(None),
        };
        let contact = tile.contact_point_relative();
        let height = tile.preferred_height();
        let footprint = (tile.ls1.participant.id, tile.note.position);
        let in_run = current_y.note_anchors_in_run().map(<[_]>::to_vec);
        tile.y_gauge =
            if tile.note.parallel && in_run.as_ref().is_some_and(|r| r.contains(&footprint)) {
                YGauge::create_propagating(current_y, height)
                    .with_note_anchors_in_run(in_run.unwrap_or_default())
            } else if tile.note.parallel {
                let mut grown = in_run.unwrap_or_default();
                grown.push(footprint);
                YGauge::create_parallel(current_y, contact, height).with_note_anchors_in_run(grown)
            } else {
                YGauge::create_with_contact(current_y, contact, height)
                    .with_note_anchors_in_run(vec![footprint])
            };
        Rc::new(tile)
    }

    fn component(&self) -> Rc<dyn Component> {
        if let Some(c) = self.cached.borrow().as_ref() {
            return c.clone();
        }
        let c = note_component(&self.args, &self.note, true);
        *self.cached.borrow_mut() = Some(c.clone());
        c
    }

    fn used_width(&self) -> f64 {
        let sb = self.args.sb();
        let width = self.component().preferred_dimension(sb).width;
        if self.note.position == NotePosition::OverSeveral {
            if let Some(ls2) = &self.ls2 {
                let w = ls2.pos_d(sb).value() - self.ls1.pos_b().value();
                if width < w {
                    return w;
                }
            }
        }
        width
    }

    fn x(&self) -> Real {
        let sb = self.args.sb();
        let width = self.used_width();
        match self.note.position {
            NotePosition::Left => self.ls1.pos_c(sb).add_fixed(-width),
            NotePosition::Right => {
                let level = self
                    .ls1
                    .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
                self.ls1.pos_c(sb).add_fixed(level as f64 * LIVE_DELTA_SIZE)
            }
            NotePosition::OverSeveral => {
                let ls2 = self.ls2.as_ref().expect("second participant");
                real::middle(&self.ls1.pos_c(sb), &ls2.pos_c(sb)).add_fixed(-width / 2.0)
            }
            NotePosition::Over => self.ls1.pos_c(sb).add_fixed(-width / 2.0),
            other => panic!("unsupported note position {other:?}"),
        }
    }

    fn right_anchor(&self) -> &Rc<LivingSpace> {
        if self.note.position == NotePosition::OverSeveral {
            self.ls2.as_ref().unwrap_or(&self.ls1)
        } else {
            &self.ls1
        }
    }
}

impl Tile for NoteTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {
        let sb = self.args.sb();
        let overflow_left = self.ls1.pos_b().value() - self.min_x().value();
        if overflow_left > 0.0 {
            self.ls1.ensure_margin_before(overflow_left);
        }
        let anchor = self.right_anchor();
        let overflow_right = self.max_x().value() - anchor.pos_d(sb).value();
        if overflow_right > 0.0 {
            anchor.ensure_margin_after(overflow_right);
        }
    }

    fn min_x(&self) -> Real {
        let x = self.x();
        if self.note.position == NotePosition::OverSeveral {
            return real::min(&[x, self.ls1.pos_b()]);
        }
        x
    }

    fn max_x(&self) -> Real {
        let x = self.x().add_fixed(self.used_width());
        if self.note.position == NotePosition::OverSeveral {
            if let Some(ls2) = &self.ls2 {
                return real::max(&[x, ls2.pos_d(self.args.sb())]);
            }
        }
        x
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height / 2.0
    }

    fn stable_min_x(&self) -> Vec<Real> {
        let mut v = vec![self.x()];
        if self.note.position == NotePosition::OverSeveral {
            v.push(self.ls1.pos_b());
        }
        v
    }

    fn stable_max_x(&self) -> Vec<Real> {
        let mut v = vec![self.x().add_fixed(self.used_width())];
        if self.note.position == NotePosition::OverSeveral {
            if let Some(ls2) = &self.ls2 {
                v.push(ls2.pos_d(self.args.sb()));
            }
        }
        v
    }

    fn left_anchor(&self) -> Option<(usize, f64)> {
        let overhang = match self.note.position {
            NotePosition::Left => self.used_width(),
            NotePosition::Over => self.used_width() / 2.0,
            _ => 0.0,
        };
        Some((self.ls1.participant.id, overhang))
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let comp = self.component();
        let dim = comp.preferred_dimension(ug.string_bounder());
        let x = self.x().value();
        let area = Area::create(self.used_width(), dim.height);
        let ug = ug.apply_translate(UTranslate::dx(x));
        comp.draw(&ug, &area, ug.is_background_layer());
    }
}

/// Several notes sharing a row (`NotesTile`).
pub struct NotesTile {
    args: Rc<TileArguments>,
    event: usize,
    notes: Vec<Note>,
    y_gauge: YGauge,
}

impl NotesTile {
    pub fn new(args: Rc<TileArguments>, event: usize, current_y: &YGauge) -> Rc<NotesTile> {
        let Event::Notes(notes) = &args.model.events[event] else {
            panic!("not notes");
        };
        let mut tile = NotesTile {
            notes: notes.clone(),
            args,
            event,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
        };
        tile.y_gauge = YGauge::create_with_contact(
            current_y,
            tile.contact_point_relative(),
            tile.preferred_height(),
        );
        Rc::new(tile)
    }

    fn component(&self, note: &Note) -> Rc<dyn Component> {
        note_component(&self.args, note, true)
    }

    fn used_width(&self, note: &Note) -> f64 {
        self.component(note)
            .preferred_dimension(self.args.sb())
            .width
    }

    fn living_space(&self, note: &Note) -> Rc<LivingSpace> {
        self.args
            .living_spaces
            .get(note.p.expect("note participant"))
    }

    fn xcenter(&self, note: &Note) -> Real {
        self.living_space(note).pos_c(self.args.sb())
    }

    fn stacking_offset(&self, index: usize) -> f64 {
        let note = &self.notes[index];
        self.notes[..index]
            .iter()
            .filter(|o| o.position == note.position && o.p == note.p)
            .map(|o| self.used_width(o))
            .sum()
    }

    fn x(&self, index: usize) -> Real {
        let sb = self.args.sb();
        let note = &self.notes[index];
        let ls1 = self.living_space(note);
        let width = self.used_width(note);
        match note.position {
            NotePosition::Left => ls1
                .pos_c(sb)
                .add_fixed(-width - self.stacking_offset(index)),
            NotePosition::Right => {
                let level = ls1.level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
                let dx = level as f64 * LIVE_DELTA_SIZE;
                ls1.pos_c(sb).add_fixed(dx + self.stacking_offset(index))
            }
            NotePosition::OverSeveral => {
                let ls2 = self
                    .args
                    .living_spaces
                    .get(note.p2.expect("second participant"));
                real::middle(&ls1.pos_c(sb), &ls2.pos_c(sb)).add_fixed(-width / 2.0)
            }
            NotePosition::Over => ls1.pos_c(sb).add_fixed(-width / 2.0),
            other => panic!("unsupported note position {other:?}"),
        }
    }

    fn x2(&self, index: usize) -> Real {
        self.x(index).add_fixed(self.used_width(&self.notes[index]))
    }
}

impl Tile for NotesTile {
    fn preferred_height(&self) -> f64 {
        self.notes
            .iter()
            .map(|n| self.component(n).preferred_dimension(self.args.sb()).height)
            .fold(0.0, f64::max)
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {
        let n = self.notes.len();
        for i in 0..n.saturating_sub(1) {
            for j in i + 1..n {
                let center1 = self.xcenter(&self.notes[i]).value();
                let center2 = self.xcenter(&self.notes[j]).value();
                if center2 == center1 {
                    continue;
                }
                if center2 > center1 {
                    self.x(j).ensure_bigger_than(&self.x2(i));
                } else {
                    self.x(i).ensure_bigger_than(&self.x2(j));
                }
            }
        }
    }

    fn min_x(&self) -> Real {
        let reals: Vec<Real> = (0..self.notes.len()).map(|i| self.x(i)).collect();
        real::min(&reals)
    }

    fn max_x(&self) -> Real {
        let reals: Vec<Real> = (0..self.notes.len()).map(|i| self.x2(i)).collect();
        real::max(&reals)
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        self.preferred_height() / 2.0
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        for (i, note) in self.notes.iter().enumerate() {
            let comp = self.component(note);
            let dim = comp.preferred_dimension(ug.string_bounder());
            let area = Area::create(self.used_width(note), dim.height);
            let ug2 = ug.apply_translate(UTranslate::dx(self.x(i).value()));
            comp.draw(&ug2, &area, ug2.is_background_layer());
        }
    }
}

/// Where a note sits relative to the message it comments.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
    SelfLeft,
    SelfRight,
}

/// A tile followed by a note attached to its message.
pub struct NoteOnMessageTile {
    args: Rc<TileArguments>,
    side: Side,
    tile: Rc<dyn Tile>,
    event: usize,
    note: Note,
    living_space: Rc<LivingSpace>,
    is_create: bool,
    y_gauge: YGauge,
}

impl NoteOnMessageTile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        args: Rc<TileArguments>,
        side: Side,
        tile: Rc<dyn Tile>,
        event: usize,
        note: Note,
        living_space: Rc<LivingSpace>,
        is_create: bool,
    ) -> Rc<NoteOnMessageTile> {
        let inner = tile.y_gauge();
        let mut t = NoteOnMessageTile {
            args,
            side,
            tile,
            event,
            note,
            living_space,
            is_create,
            y_gauge: inner.clone(),
        };
        let max = inner.min().add_at_least(t.preferred_height());
        t.y_gauge = YGauge::with_contact(
            inner.min().clone(),
            max,
            inner.contact().cloned(),
            inner.origin().cloned(),
        );
        Rc::new(t)
    }

    fn component(&self) -> Rc<dyn Component> {
        match self.side {
            Side::Left | Side::Right => note_component(&self.args, &self.note, false),
            _ => folded_note_component(&self.args, &self.note),
        }
    }

    fn comp_width(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).width
    }

    fn note_position(&self) -> Real {
        let sb = self.args.sb();
        match self.side {
            Side::Left => self.living_space.pos_c(sb).add_fixed(-self.comp_width()),
            Side::Right => {
                if self.is_create {
                    return self.living_space.pos_d(sb);
                }
                let level = self
                    .living_space
                    .level_at(self.event, EventsHistoryMode::IgnoreFutureDeactivate);
                self.living_space
                    .pos_c(sb)
                    .add_fixed(level as f64 * LIVE_DELTA_SIZE)
            }
            Side::Top | Side::Bottom => self.tile.min_x(),
            Side::SelfLeft => self.tile.min_x().add_fixed(-self.comp_width()),
            Side::SelfRight => self.tile.max_x(),
        }
    }

    fn draw_line(&self, ug: &UGraphic, x1: f64, y1: f64, x2: f64, y2: f64) {
        let style = self
            .args
            .skin_param
            .style_builder()
            .merged_style(&queries::SEQUENCEDIAG)
            .expect("sequence style");
        let color = style.value(PName::LineColor).as_color();
        ug.apply_translate(UTranslate::new(x1, y1))
            .apply_color(color)
            .apply_stroke(UStroke::new(2.0, 2.0, 1.0))
            .draw(ULine::new(x2 - x1, y2 - y1));
    }

    fn middle_msg(&self) -> f64 {
        (self.tile.min_x().value() + self.tile.max_x().value()) / 2.0
    }
}

impl Tile for NoteOnMessageTile {
    fn preferred_height(&self) -> f64 {
        let h = self.component().preferred_dimension(self.args.sb()).height;
        match self.side {
            Side::Top | Side::Bottom => self.tile.preferred_height() + h + SPACE_Y,
            _ => self.tile.preferred_height().max(h),
        }
    }

    fn on_gauge_resolved(&self) {
        self.tile.on_gauge_resolved();
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {
        self.tile.add_constraints();
        let sb = self.args.sb();
        match self.side {
            Side::SelfLeft => {
                let overflow = self.living_space.pos_b().value() - self.min_x().value();
                if overflow > 0.0 {
                    self.living_space.ensure_margin_before(overflow);
                }
            }
            Side::SelfRight => {
                let overflow = self.max_x().value() - self.living_space.pos_d(sb).value();
                if overflow > 0.0 {
                    self.living_space.ensure_margin_after(overflow);
                }
            }
            _ => {}
        }
    }

    fn min_x(&self) -> Real {
        match self.side {
            Side::Left | Side::SelfLeft => self.note_position(),
            _ => self.tile.min_x(),
        }
    }

    fn max_x(&self) -> Real {
        let width = self.comp_width();
        match self.side {
            Side::Left | Side::SelfLeft => self.tile.max_x(),
            Side::Right | Side::SelfRight => self.note_position().add_fixed(width),
            Side::Top | Side::Bottom => {
                real::max(&[self.tile.max_x(), self.tile.min_x().add_fixed(width)])
            }
        }
    }

    fn drawn_min_x(&self) -> Real {
        match self.side {
            Side::SelfLeft => self.tile.drawn_min_x().add_fixed(-self.comp_width()),
            Side::SelfRight => self.tile.drawn_min_x(),
            _ => self.min_x(),
        }
    }

    fn drawn_max_x(&self) -> Real {
        match self.side {
            Side::SelfLeft => self.tile.drawn_max_x(),
            Side::SelfRight => self.tile.drawn_max_x().add_fixed(self.comp_width()),
            _ => self.max_x(),
        }
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        self.tile.contact_point_relative()
    }

    fn match_anchor(&self, anchor: &str) -> bool {
        self.tile.match_anchor(anchor)
    }

    fn stable_min_x(&self) -> Vec<Real> {
        match self.side {
            Side::Left | Side::SelfLeft => vec![self.drawn_min_x()],
            _ => Vec::new(),
        }
    }

    fn stable_max_x(&self) -> Vec<Real> {
        match self.side {
            Side::Right | Side::SelfRight => vec![self.drawn_max_x()],
            _ => Vec::new(),
        }
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let comp = self.component();
        let dim = comp.preferred_dimension(sb);
        let area = Area::create(dim.width, dim.height);
        let background = ug.is_background_layer();
        if self.side == Side::Top {
            self.tile
                .draw(&ug.apply_translate(UTranslate::dy(dim.height + SPACE_Y)));
        } else {
            self.tile.draw(ug);
        }
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let x_note = self.note_position().value();
        match self.side {
            Side::Bottom => {
                let y_note = self.tile.preferred_height();
                comp.draw(
                    &ug.apply_translate(UTranslate::new(x_note, y_note + SPACE_Y)),
                    &area,
                    background,
                );
                self.draw_line(
                    &ug,
                    self.middle_msg(),
                    self.tile.contact_point_relative(),
                    x_note + dim.width / 2.0,
                    y_note + SPACE_Y + PADDING_Y,
                );
            }
            Side::Top => {
                comp.draw(
                    &ug.apply_translate(UTranslate::dx(x_note)),
                    &area,
                    background,
                );
                self.draw_line(
                    &ug,
                    self.middle_msg(),
                    self.tile.contact_point_relative() + dim.height + SPACE_Y,
                    x_note + dim.width / 2.0,
                    dim.height - 2.0 * PADDING_Y,
                );
            }
            _ => comp.draw(
                &ug.apply_translate(UTranslate::dx(x_note)),
                &area,
                background,
            ),
        }
    }
}
