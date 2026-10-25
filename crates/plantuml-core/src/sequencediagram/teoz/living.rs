//! Participants as columns of the drawing: lifelines, activation boxes, heads and tails
//! (`LivingSpace`, `LivingSpaces`, `LiveBoxes`, `MutingLine`).

use super::Model;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::real::Real;
use crate::sequencediagram::model::{
    Event, LifeEventType, Participant, ParticipantEnglober, ParticipantType,
};
use crate::skin::component::{Area, Component, ComponentType};
use crate::skin::rose::Rose;
use crate::skin::SkinParam;
use crate::style::{StyleBuilder, StyleQuery};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// How future activations are counted at an event (`EventsHistoryMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventsHistoryMode {
    IgnoreFutureDeactivate,
    IgnoreFutureActivate,
    ConsiderFutureDeactivate,
}

/// Sorted map of positions to values with replace-on-equal semantics (`TreeMap<Double, V>`).
#[derive(Debug, Clone, Default)]
struct PosMap<V: Clone>(Vec<(f64, V)>);

impl<V: Clone> PosMap<V> {
    fn put(&mut self, k: f64, v: V) {
        match self.0.binary_search_by(|(a, _)| a.total_cmp(&k)) {
            Ok(i) => self.0[i].1 = v,
            Err(i) => self.0.insert(i, (k, v)),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(f64, V)> {
        self.0.iter()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

/// Extends the line of a participant over a delay (`MutingLine`).
pub struct MutingLine {
    skin: Rc<SkinParam>,
    style_builder: StyleBuilder,
    participant: Rc<Participant>,
    delays: RefCell<PosMap<f64>>,
}

impl MutingLine {
    pub fn new(skin: Rc<SkinParam>, participant: Rc<Participant>) -> Self {
        Self {
            style_builder: skin.style_builder(),
            skin,
            participant,
            delays: RefCell::new(PosMap::default()),
        }
    }

    pub fn draw_line(&self, ug: &UGraphic, background: bool, create_y: f64, end_y: f64) {
        let delays = self.delays.borrow().clone();
        if delays.len() > 0 {
            let mut y = create_y;
            for (k, v) in delays.iter() {
                if *k >= create_y && k + v <= end_y {
                    self.draw_internal(ug, background, y, *k, ComponentType::ParticipantLine);
                    self.draw_internal(ug, background, *k, k + v, ComponentType::DelayLine);
                    y = k + v;
                }
            }
            self.draw_internal(ug, background, y, end_y, ComponentType::ParticipantLine);
        } else {
            self.draw_internal(
                ug,
                background,
                create_y,
                end_y,
                ComponentType::ParticipantLine,
            );
        }
    }

    fn draw_internal(
        &self,
        ug: &UGraphic,
        background: bool,
        y1: f64,
        y2: f64,
        ctype: ComponentType,
    ) {
        if y2 == y1 {
            return;
        }
        assert!(y2 > y1, "negative height");
        let query = ctype
            .style_query()
            .with_stereotype_obj(self.participant.stereotype());
        let style = self
            .style_builder
            .merged_style(&query)
            .expect("lifeline style");
        let display = self
            .participant
            .display(self.skin.value_is("sequenceParticipant", "underline"));
        let comp = Rose::create_component(&style, ctype, &self.skin, Some(&display));
        let dim = comp.preferred_dimension(ug.string_bounder());
        let area = Area::create(dim.width, y2 - y1);
        comp.draw(&ug.apply_translate(UTranslate::dy(y1)), &area, background);
    }

    pub fn delay_on(&self, y: f64, height: f64) {
        self.delays.borrow_mut().put(y, height);
    }
}

#[derive(Debug, Clone)]
struct Step {
    value: f64,
    destroy: bool,
    indent: usize,
    color: Option<Fashion>,
    style_builder: Option<StyleBuilder>,
}

/// Activation levels over the height of a lifeline (`Stairs`).
#[derive(Default)]
struct Stairs {
    values: Vec<Step>,
}

impl Stairs {
    fn add_step(&mut self, step: Step) {
        if let Some(last) = self.values.last() {
            if step.value < last.value {
                return;
            }
        }
        self.values.push(step);
    }

    fn max_indent(&self) -> usize {
        self.values.iter().map(|s| s.indent).max().unwrap_or(0)
    }
}

struct LevelCache {
    /// Per event: (index, level after it).
    info: Vec<(usize, usize)>,
    next_life: Vec<i64>,
}

/// Activation boxes of a participant (`LiveBoxes`).
pub struct LiveBoxes {
    skin: Rc<SkinParam>,
    delays: RefCell<PosMap<f64>>,
    participant: Rc<Participant>,
    model: Rc<Model>,
    events_step: RefCell<HashMap<usize, f64>>,
    cache: RefCell<Option<Rc<LevelCache>>>,
}

fn parallel_with(events: &[Event], a: usize, b: usize) -> bool {
    if a == b {
        return true;
    }
    match events[a].message_core().and_then(|c| c.parallel_brother) {
        Some(brother) => brother == b || parallel_with(events, brother, b),
        None => false,
    }
}

impl LiveBoxes {
    pub fn new(participant: Rc<Participant>, model: Rc<Model>, skin: Rc<SkinParam>) -> Self {
        Self {
            skin,
            delays: RefCell::new(PosMap::default()),
            participant,
            model,
            events_step: RefCell::new(HashMap::new()),
            cache: RefCell::new(None),
        }
    }

    pub fn add_step(&self, event: usize, y: f64) {
        let ev = &self.model.events[event];
        if !ev.deals_with(self.participant.id) {
            return;
        }
        let mut y = y;
        let mut steps = self.events_step.borrow_mut();
        if let Event::LifeEvent(le) = ev {
            if le.kind == LifeEventType::Deactivate && steps.values().any(|v| *v == y) {
                y += 5.0;
            }
        }
        steps.insert(event, y);
    }

    fn cache(&self) -> Rc<LevelCache> {
        if let Some(c) = self.cache.borrow().as_ref() {
            return c.clone();
        }
        let events = &self.model.events;
        let p = self.participant.id;
        let mut info = Vec::with_capacity(events.len());
        let mut level = 0usize;
        for (i, e) in events.iter().enumerate() {
            if let Event::LifeEvent(le) = e {
                if le.participant == p && le.is_activate() {
                    level += 1;
                }
                if le.participant == p && le.is_deactivate_or_destroy() {
                    level = level.saturating_sub(1);
                }
            }
            info.push((i, level));
        }
        let mut next_life = vec![-1i64; events.len() + 1];
        for i in (0..events.len()).rev() {
            next_life[i] = match &events[i] {
                Event::LifeEvent(_) => i as i64,
                Event::Note(_) | Event::Message(_) | Event::MessageExo(_) => next_life[i + 1],
                _ => -1,
            };
        }
        let c = Rc::new(LevelCache { info, next_life });
        *self.cache.borrow_mut() = Some(c.clone());
        c
    }

    pub fn level_at(&self, event: usize, mode: EventsHistoryMode) -> usize {
        let cache = self.cache();
        let events = &self.model.events;
        let p = self.participant.id;
        let (index, mut level) = cache.info[event];
        let mut level_i = level as i64;
        match &events[index] {
            e @ (Event::Message(_) | Event::MessageExo(_)) => {
                let mut seen_activate = false;
                let mut seen_deactivate = false;
                let mut k = cache.next_life[index + 1];
                while k != -1 {
                    let Event::LifeEvent(le) = &events[k as usize] else {
                        break;
                    };
                    let same_message = le
                        .message
                        .is_some_and(|m| m == index || parallel_with(events, m, index));
                    if same_message {
                        if mode != EventsHistoryMode::IgnoreFutureActivate
                            && le.is_activate()
                            && e.deals_with(p)
                            && le.participant == p
                        {
                            seen_activate = true;
                            if seen_deactivate {
                                break;
                            }
                            level_i += 1;
                        }
                        if mode == EventsHistoryMode::ConsiderFutureDeactivate
                            && le.is_deactivate_or_destroy()
                            && e.deals_with(p)
                            && le.participant == p
                        {
                            seen_deactivate = true;
                            if seen_activate {
                                break;
                            }
                            level_i = (level_i - 1).max(0);
                        }
                    }
                    k = cache.next_life[k as usize + 1];
                }
            }
            Event::Note(_) => {
                let mut i = index + 1;
                loop {
                    while i < events.len() && matches!(events[i], Event::Note(_)) {
                        i += 1;
                    }
                    let Some(Event::LifeEvent(le)) = events.get(i) else {
                        break;
                    };
                    i += 1;
                    if le.participant != p || le.message.is_none() {
                        continue;
                    }
                    if mode != EventsHistoryMode::IgnoreFutureActivate && le.is_activate() {
                        level_i += 1;
                    }
                    if mode == EventsHistoryMode::ConsiderFutureDeactivate
                        && le.is_deactivate_or_destroy()
                    {
                        level_i = (level_i - 1).max(0);
                    }
                }
            }
            _ => {}
        }
        level = level_i.max(0) as usize;
        level
    }

    fn is_next_event_a_destroy(&self, event: usize) -> bool {
        let events = &self.model.events;
        if !matches!(events[event], Event::Message(_)) {
            return false;
        }
        for next in &events[event + 1..] {
            match next {
                Event::Note(_) => continue,
                Event::LifeEvent(le) => {
                    if le.is_destroy(self.participant.id) {
                        return true;
                    }
                }
                _ => return false,
            }
        }
        false
    }

    fn activate_color(&self, event: usize) -> Option<Fashion> {
        let events = &self.model.events;
        if let Event::LifeEvent(le) = &events[event] {
            if le.is_activate() {
                return Some(le.specific_colors.clone());
            }
        }
        if !matches!(events[event], Event::Message(_) | Event::MessageExo(_)) {
            return None;
        }
        for next in &events[event + 1..] {
            match next {
                Event::Note(_) => continue,
                Event::LifeEvent(le) if le.message == Some(event) => {
                    if le.is_activate() && le.participant == self.participant.id {
                        return Some(le.specific_colors.clone());
                    }
                }
                _ => return None,
            }
        }
        None
    }

    fn stairs(&self, create_y: f64, total_height: f64) -> Stairs {
        let events = &self.model.events;
        let p = self.participant.id;
        let steps = self.events_step.borrow();
        let mut stair = Stairs::default();
        let mut indent = 0usize;
        let mut last_message: Option<usize> = None;
        let mut position: Option<f64> = None;
        let mut seen_activate = false;
        let mut seen_deactivate = false;
        for (idx, event) in events.iter().enumerate() {
            if matches!(event, Event::Note(_)) {
                last_message = None;
                seen_activate = false;
                seen_deactivate = false;
            }
            let potential = steps.get(&idx).copied();
            if position.is_none() || last_message.is_none() {
                position = potential;
            } else if let Event::LifeEvent(le) = event {
                if event.deals_with(p) {
                    let lm = last_message.expect("checked");
                    if le.message.is_none()
                        || !le.message.is_some_and(|m| parallel_with(events, m, lm))
                    {
                        position = potential;
                    } else if (le.is_activate() && seen_deactivate)
                        || (le.kind == LifeEventType::Deactivate && seen_activate)
                    {
                        position = potential;
                    }
                    seen_activate |= le.is_activate();
                    seen_deactivate |= le.kind == LifeEventType::Deactivate;
                } else {
                    continue;
                }
            } else {
                position = potential;
            }
            if matches!(event, Event::Message(_) | Event::MessageExo(_)) {
                if last_message.is_some_and(|lm| parallel_with(events, idx, lm)) {
                    continue;
                }
                seen_activate = false;
                seen_deactivate = false;
                last_message = Some(idx);
            }
            if let Some(pos) = position {
                indent = self.level_at(idx, EventsHistoryMode::ConsiderFutureDeactivate);
                let color = self.activate_color(idx);
                let style_builder = match event {
                    Event::Message(m) => Some(m.core.style_builder.clone()),
                    Event::MessageExo(m) => Some(m.core.style_builder.clone()),
                    Event::LifeEvent(l) => Some(l.style_builder.clone()),
                    _ => None,
                };
                stair.add_step(Step {
                    value: create_y.max(pos),
                    destroy: self.is_next_event_a_destroy(idx),
                    indent,
                    color,
                    style_builder,
                });
            }
        }
        stair.add_step(Step {
            value: total_height,
            destroy: false,
            indent,
            color: None,
            style_builder: None,
        });
        stair
    }

    /// Highest nesting of activations (`getMaxValue`).
    pub fn max_value(&self) -> usize {
        let p = self.participant.id;
        let mut max = 0i64;
        let mut level = 0i64;
        for e in &self.model.events {
            if let Event::LifeEvent(le) = e {
                if le.participant == p && le.is_activate() {
                    level += 1;
                }
                max = max.max(level);
                if le.participant == p && le.is_deactivate_or_destroy() {
                    level -= 1;
                }
            }
        }
        max as usize
    }

    fn box_width(&self, sb: &dyn StringBounder) -> f64 {
        let style = self
            .skin
            .style_builder()
            .merged_style(&ComponentType::ActivationBoxCloseClose.style_query())
            .expect("activation style");
        Rose::create_component(
            &style,
            ComponentType::ActivationBoxCloseClose,
            &self.skin,
            None,
        )
        .preferred_width(sb)
    }

    pub fn max_position(&self, sb: &dyn StringBounder) -> f64 {
        self.box_width(sb) / 2.0 * self.max_value() as f64
    }

    pub fn delay_on(&self, y: f64, height: f64) {
        self.delays.borrow_mut().put(y, height);
    }

    fn component(
        &self,
        ctype: ComponentType,
        symbol: Option<&Fashion>,
        style_builder: Option<&StyleBuilder>,
    ) -> Rc<dyn Component> {
        let sb = style_builder
            .cloned()
            .unwrap_or_else(|| self.skin.style_builder());
        let query: StyleQuery = ctype
            .style_query()
            .with_stereotype_obj(self.participant.stereotype());
        let mut style = sb.merged_style(&query).expect("activation style");
        if let Some(f) = symbol {
            if let Some(b) = f.back_color() {
                style = style.override_color(crate::style::PName::BackGroundColor, Some(b.clone()));
            }
        }
        Rose::create_component(&style, ctype, &self.skin, None)
    }

    fn draw_destroy_if_needed(&self, ug: &UGraphic, background: bool, step: &Step) {
        if step.destroy {
            let cross = Rose::create_component(
                &self
                    .skin
                    .style_builder()
                    .merged_style(&ComponentType::Destroy.style_query())
                    .expect("destroy style"),
                ComponentType::Destroy,
                &self.skin,
                None,
            );
            let dim = cross.preferred_dimension(ug.string_bounder());
            cross.draw(
                &ug.apply_translate(UTranslate::new(
                    -dim.width / 2.0,
                    step.value - dim.height / 2.0,
                )),
                &Area::default(),
                background,
            );
        }
    }

    pub fn draw_boxes(&self, ug: &UGraphic, background: bool, create_y: f64, end_y: f64) {
        let stairs = self.stairs(create_y, end_y);
        let max = stairs.max_indent();
        for i in 1..=max {
            self.draw_one_level(ug, background, i, &stairs);
        }
        for step in &stairs.values {
            if step.indent == 0 {
                self.draw_destroy_if_needed(ug, background, step);
            }
        }
    }

    fn draw_one_level(&self, ug: &UGraphic, background: bool, level: usize, stairs: &Stairs) {
        let width = self.box_width(ug.string_bounder());
        let ug = ug.apply_translate(UTranslate::dx((level - 1) as f64 * width / 2.0));
        let delays: Vec<(f64, f64)> = self
            .delays
            .borrow()
            .iter()
            .map(|(k, v)| (*k, k + v))
            .collect();
        let mut pending = true;
        let mut y1 = f64::MAX;
        let mut symbol: Option<Fashion> = None;
        let mut style_builder: Option<StyleBuilder> = None;
        let n = stairs.values.len();
        for (i, step) in stairs.values.iter().enumerate() {
            let last = i + 1 == n;
            if pending && step.indent == level {
                y1 = step.value;
                symbol = step.color.clone();
                style_builder = step.style_builder.clone();
                pending = false;
            } else if !pending && (last || step.indent < level) {
                // cut the segment [y1, step.value] by the delays
                let segments = cut_segment(y1, step.value, &delays);
                let mut ctype = ComponentType::ActivationBoxCloseClose;
                if segments.len() > 1 {
                    ctype = ComponentType::ActivationBoxCloseOpen;
                }
                for (k, (a, b)) in segments.iter().enumerate() {
                    if k + 1 == segments.len() && ctype != ComponentType::ActivationBoxCloseClose {
                        ctype = ComponentType::ActivationBoxOpenClose;
                    }
                    let comp = self.component(ctype, symbol.as_ref(), style_builder.as_ref());
                    comp.draw(
                        &ug.apply_translate(UTranslate::new(-width / 2.0, *a)),
                        &Area::create(width, b - a),
                        background,
                    );
                    ctype = ComponentType::ActivationBoxOpenOpen;
                }
                if step.indent > 0 {
                    self.draw_destroy_if_needed(&ug, background, step);
                }
                pending = true;
            }
        }
    }
}

/// Splits `[y1, y2]` around the delay segments (`Segment.cutSegmentIfNeed`).
fn cut_segment(y1: f64, y2: f64, delays: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut result = Vec::new();
    let mut start = y1;
    let mut sorted = delays.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (d1, d2) in sorted {
        if d2 <= start || d1 >= y2 {
            continue;
        }
        if d1 > start {
            result.push((start, d1));
        }
        start = start.max(d2);
    }
    if start < y2 || result.is_empty() {
        result.push((start, y2));
    }
    result
}

/// A participant in the layout (`LivingSpace`).
pub struct LivingSpace {
    pub participant: Rc<Participant>,
    skin: Rc<SkinParam>,
    head_type: ComponentType,
    tail_type: ComponentType,
    muting_line: MutingLine,
    liveboxes: LiveBoxes,
    pos_b: Real,
    pos_c: RefCell<Option<Real>>,
    pos_d: RefCell<Option<Real>>,
    create: Cell<bool>,
    alive_changes: RefCell<PosMap<bool>>,
    englober: Option<Rc<ParticipantEnglober>>,
    margin_before: Cell<f64>,
    margin_after: Cell<f64>,
}

impl LivingSpace {
    pub fn new(
        participant: Rc<Participant>,
        englober: Option<Rc<ParticipantEnglober>>,
        skin: Rc<SkinParam>,
        position: Real,
        model: Rc<Model>,
    ) -> LivingSpace {
        use ComponentType::*;
        let (head_type, tail_type) = match participant.participant_type {
            ParticipantType::Participant => (ParticipantHead, ParticipantTail),
            ParticipantType::Actor => (ActorHead, ActorTail),
            ParticipantType::Boundary => (BoundaryHead, BoundaryTail),
            ParticipantType::Control => (ControlHead, ControlTail),
            ParticipantType::Entity => (EntityHead, EntityTail),
            ParticipantType::Queue => (QueueHead, QueueTail),
            ParticipantType::Database => (DatabaseHead, DatabaseTail),
            ParticipantType::Collections => (CollectionsHead, CollectionsTail),
        };
        LivingSpace {
            muting_line: MutingLine::new(skin.clone(), participant.clone()),
            liveboxes: LiveBoxes::new(participant.clone(), model, skin.clone()),
            participant,
            skin,
            head_type,
            tail_type,
            pos_b: position,
            pos_c: RefCell::new(None),
            pos_d: RefCell::new(None),
            create: Cell::new(false),
            alive_changes: RefCell::new(PosMap::default()),
            englober,
            margin_before: Cell::new(0.0),
            margin_after: Cell::new(0.0),
        }
    }

    pub fn level_at(&self, event: usize, mode: EventsHistoryMode) -> usize {
        self.liveboxes.level_at(event, mode)
    }

    pub fn add_step_for_livebox(&self, event: usize, y: f64) {
        self.liveboxes.add_step(event, y);
    }

    pub fn englober(&self) -> Option<&Rc<ParticipantEnglober>> {
        self.englober.as_ref()
    }

    fn underlined(&self) -> bool {
        self.skin.value_is("sequenceParticipant", "underline")
    }

    fn head_component(&self, ctype: ComponentType) -> Rc<dyn Component> {
        Rose::create_component_participant(
            &self.participant,
            ctype,
            &self.skin,
            &self.participant.display(self.underlined()),
        )
    }

    pub fn head_preferred_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.head_component(self.head_type).preferred_dimension(sb)
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.head_preferred_dimension(sb).width
    }

    pub fn draw_line_and_liveboxes(&self, ug: &UGraphic, background: bool, height: f64) {
        ug.start_group(&self.participant.group_attributes("lifeline"));
        self.draw_line(ug, background, height);
        ug.close_group();
        self.liveboxes
            .draw_boxes(ug, background, self.first_create_y(), height);
    }

    fn draw_line(&self, ug: &UGraphic, background: bool, height: f64) {
        let mut alive = !self.create.get();
        let mut alive_since = 0.0;
        for (k, v) in self.alive_changes.borrow().iter() {
            if !alive && *v {
                alive_since = *k;
            } else if alive && !*v {
                self.muting_line.draw_line(ug, background, alive_since, *k);
            }
            alive = *v;
        }
        if alive {
            self.muting_line
                .draw_line(ug, background, alive_since, height);
        }
    }

    fn first_create_y(&self) -> f64 {
        self.alive_changes
            .borrow()
            .iter()
            .find(|(_, v)| *v)
            .map_or(0.0, |(k, _)| *k)
    }

    pub fn draw_head(
        &self,
        ug: &UGraphic,
        background: bool,
        vertical: VerticalAlignment,
        horizontal: HorizontalAlignment,
    ) {
        self.draw_head_or_tail(ug, background, vertical, horizontal, self.head_type);
    }

    pub fn draw_tail(
        &self,
        ug: &UGraphic,
        background: bool,
        vertical: VerticalAlignment,
        horizontal: HorizontalAlignment,
    ) {
        self.draw_head_or_tail(ug, background, vertical, horizontal, self.tail_type);
    }

    fn draw_head_or_tail(
        &self,
        ug: &UGraphic,
        background: bool,
        vertical: VerticalAlignment,
        horizontal: HorizontalAlignment,
        ctype: ComponentType,
    ) {
        if self.create.get() && vertical == VerticalAlignment::Bottom {
            return;
        }
        let comp = self.head_component(ctype);
        let dim = comp.preferred_dimension(ug.string_bounder());
        let mut ug = ug.clone();
        if horizontal == HorizontalAlignment::Right {
            ug = ug.apply_translate(UTranslate::dx(-dim.width));
        }
        if vertical == VerticalAlignment::Center {
            ug = ug.apply_translate(UTranslate::dy(-dim.height / 2.0));
        }
        let area = Area::new(dim);
        let url = self.participant.url();
        let is_head = ctype == self.head_type;
        ug.start_group(
            &self
                .participant
                .group_attributes(if is_head { "head" } else { "tail" }),
        );
        if let Some(_u) = url {
            log::debug!(
                "link on participant {} not drawn yet",
                self.participant.code
            );
        }
        comp.draw(&ug, &area, background);
        ug.close_group();
    }

    pub fn pos_c(&self, sb: &dyn StringBounder) -> Real {
        if self.pos_c.borrow().is_none() {
            *self.pos_c.borrow_mut() = Some(self.pos_b.add_fixed(self.preferred_width(sb) / 2.0));
        }
        self.pos_c.borrow().clone().expect("set")
    }

    pub fn pos_c2(&self, sb: &dyn StringBounder) -> Real {
        let delta = self.liveboxes.max_position(sb);
        self.pos_c(sb).add_fixed(delta)
    }

    pub fn pos_d(&self, sb: &dyn StringBounder) -> Real {
        if self.pos_d.borrow().is_none() {
            *self.pos_d.borrow_mut() = Some(self.pos_b.add_fixed(self.preferred_width(sb)));
        }
        self.pos_d.borrow().clone().expect("set")
    }

    pub fn pos_b(&self) -> Real {
        self.pos_b.clone()
    }

    pub fn go_create_at(&self, y: f64) {
        self.alive_changes.borrow_mut().put(y, true);
        self.create.set(true);
    }

    pub fn go_create(&self) {
        self.create.set(true);
    }

    pub fn go_destroy(&self, y: f64) {
        self.alive_changes.borrow_mut().put(y, false);
    }

    pub fn delay_on(&self, y: f64, height: f64) {
        self.muting_line.delay_on(y, height);
        self.liveboxes.delay_on(y, height);
    }

    pub fn ensure_margin_before(&self, margin: f64) {
        assert!(margin >= 0.0);
        self.margin_before.set(self.margin_before.get().max(margin));
    }

    pub fn ensure_margin_after(&self, margin: f64) {
        assert!(margin >= 0.0);
        self.margin_after.set(self.margin_after.get().max(margin));
    }

    pub fn pos_a(&self) -> Real {
        self.pos_b.add_fixed(-self.margin_before.get())
    }

    pub fn pos_e(&self, sb: &dyn StringBounder) -> Real {
        self.pos_d(sb).add_fixed(self.margin_after.get())
    }

    pub fn is_created(&self) -> bool {
        self.create.get()
    }
}

/// Participants in order (`LivingSpaces`).
#[derive(Default)]
pub struct LivingSpaces {
    all: RefCell<Vec<(usize, Rc<LivingSpace>)>>,
}

impl LivingSpaces {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&self, participant: usize, space: Rc<LivingSpace>) {
        self.all.borrow_mut().push((participant, space));
    }

    pub fn get(&self, participant: usize) -> Rc<LivingSpace> {
        self.all
            .borrow()
            .iter()
            .find(|(p, _)| *p == participant)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| panic!("no living space for participant {participant}"))
    }

    pub fn try_get(&self, participant: usize) -> Option<Rc<LivingSpace>> {
        self.all
            .borrow()
            .iter()
            .find(|(p, _)| *p == participant)
            .map(|(_, s)| s.clone())
    }

    pub fn values(&self) -> Vec<Rc<LivingSpace>> {
        self.all.borrow().iter().map(|(_, s)| s.clone()).collect()
    }

    pub fn participants(&self) -> Vec<usize> {
        self.all.borrow().iter().map(|(p, _)| *p).collect()
    }

    pub fn size(&self) -> usize {
        self.all.borrow().len()
    }

    pub fn previous(&self, element: &Rc<LivingSpace>) -> Option<Rc<LivingSpace>> {
        let all = self.values();
        let i = all.iter().position(|s| Rc::ptr_eq(s, element))?;
        i.checked_sub(1).map(|j| all[j].clone())
    }

    pub fn next(&self, element: &Rc<LivingSpace>) -> Option<Rc<LivingSpace>> {
        let all = self.values();
        let i = all.iter().position(|s| Rc::ptr_eq(s, element))?;
        all.get(i + 1).cloned()
    }

    /// Keeps neighbors at least 10 apart (`addConstraints`).
    pub fn add_constraints(&self, sb: &dyn StringBounder) {
        let mut previous: Option<Rc<LivingSpace>> = None;
        for current in self.values() {
            if let Some(p) = &previous {
                let point1 = p.pos_e(sb);
                let point2 = current.pos_a();
                point2.ensure_bigger_than(&point1.add_fixed(10.0));
            }
            previous = Some(current);
        }
    }

    pub fn head_height(&self, sb: &dyn StringBounder) -> f64 {
        self.values()
            .iter()
            .map(|s| s.head_preferred_dimension(sb).height)
            .fold(0.0, f64::max)
    }

    pub fn draw_heads(&self, ug: &UGraphic, background: bool, vertical: VerticalAlignment) {
        let sb = ug.string_bounder();
        let head_height = self.head_height(sb);
        for ls in self.values() {
            let x = ls.pos_b().value();
            let mut y = 0.0;
            if vertical == VerticalAlignment::Bottom {
                y = head_height - ls.head_preferred_dimension(sb).height;
            }
            let u = ug.apply_translate(UTranslate::new(x, y));
            if vertical == VerticalAlignment::Top {
                ls.draw_tail(&u, background, vertical, HorizontalAlignment::Left);
            } else {
                ls.draw_head(&u, background, vertical, HorizontalAlignment::Left);
            }
        }
    }

    pub fn draw_life_lines(&self, ug: &UGraphic, background: bool, height: f64) {
        for ls in self.values() {
            let x = ls.pos_c(ug.string_bounder()).value();
            ls.draw_line_and_liveboxes(&ug.apply_translate(UTranslate::dx(x)), background, height);
        }
    }

    pub fn delay_on(&self, y: f64, height: f64) {
        for ls in self.values() {
            ls.delay_on(y, height);
        }
    }
}
