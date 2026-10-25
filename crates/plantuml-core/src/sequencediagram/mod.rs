//! Sequence diagrams (`sequencediagram` package).

pub mod commands;
pub mod model;
pub mod note_commands;
pub mod teoz;

use crate::command::{CommandDiagram, CommandExecutionResult};
use crate::diagram::{DiagramBase, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::fashion::Fashion;
use crate::skin::arrow::ArrowConfiguration;
use crate::stereo::Stereotype;
use crate::text::LineLocation;
use model::*;
use std::collections::BTreeSet;
use std::rc::Rc;

/// The diagram being built by commands (`SequenceDiagram`).
#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub base: DiagramBase,
    pub hide_unlinked_data: bool,
    participants: Vec<Participant>,
    next_participant_id: usize,
    events: Vec<Event>,
    pub titles: Vec<DisplayPositioned>,
    englobers: Vec<(usize, Option<Rc<ParticipantEnglober>>)>,
    uid_counter: usize,
    message_count: usize,
    last_event_with_deactivate: Option<usize>,
    last_delay: bool,
    pending_create: Option<usize>,
    activation_state: Vec<usize>,
    open_groupings: Vec<usize>,
    pub auto_number: AutoNumber,
    show_footbox: bool,
    participant_englober: Option<Rc<ParticipantEnglober>>,
    ignore_newpage: bool,
    count_newpage: usize,
    pub autoactivate: bool,
    hidden_portions: BTreeSet<EntityPortion>,
    link_anchors: Vec<LinkAnchor>,
}

impl SequenceDiagram {
    pub fn new(base: DiagramBase) -> Self {
        Self {
            base,
            hide_unlinked_data: false,
            participants: Vec::new(),
            next_participant_id: 0,
            events: Vec::new(),
            titles: Vec::new(),
            englobers: Vec::new(),
            uid_counter: 1,
            message_count: 0,
            last_event_with_deactivate: None,
            last_delay: false,
            pending_create: None,
            activation_state: Vec::new(),
            open_groupings: Vec::new(),
            auto_number: AutoNumber::default(),
            show_footbox: true,
            participant_englober: None,
            ignore_newpage: false,
            count_newpage: 0,
            autoactivate: false,
            hidden_portions: BTreeSet::new(),
            link_anchors: Vec::new(),
        }
    }

    pub fn from_source(source: crate::command::UmlSource) -> Self {
        Self::new(DiagramBase::new(source, DiagramType::Sequence))
    }

    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }

    pub fn participant(&self, id: usize) -> &Participant {
        self.participants
            .iter()
            .find(|p| p.id == id)
            .expect("unknown participant")
    }

    fn participant_mut(&mut self, id: usize) -> &mut Participant {
        self.participants
            .iter_mut()
            .find(|p| p.id == id)
            .expect("unknown participant")
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn link_anchors(&self) -> &[LinkAnchor] {
        &self.link_anchors
    }

    fn find_by_code(&self, code: &str) -> Option<usize> {
        self.participants
            .iter()
            .find(|p| p.code == code)
            .map(|p| p.id)
    }

    pub fn participants_contains_key(&self, code: &str) -> bool {
        self.find_by_code(code).is_some()
    }

    fn create_uid(&mut self) -> String {
        let uid = format!("part{}", self.uid_counter);
        self.uid_counter += 1;
        uid
    }

    fn add_with_order(&mut self, p: Participant) {
        let pos = self
            .participants
            .iter()
            .position(|q| p.order < q.order)
            .unwrap_or(self.participants.len());
        self.englobers
            .push((p.id, self.participant_englober.clone()));
        self.participants.insert(pos, p);
    }

    /// Existing participant or a new one displaying its code (`getOrCreateParticipant`).
    pub fn get_or_create_participant(
        &mut self,
        location: Option<LineLocation>,
        code: &str,
        display: Option<Display>,
    ) -> usize {
        if let Some(id) = self.find_by_code(code) {
            return id;
        }
        let display = display.unwrap_or_else(|| Display::with_newlines(Some(code)));
        let id = self.next_participant_id;
        self.next_participant_id += 1;
        let uid = self.create_uid();
        let p = Participant::new(
            id,
            location,
            ParticipantType::Participant,
            code,
            display,
            self.hidden_portions.clone(),
            0,
            self.base.skin_param.style_builder(),
            uid,
        );
        self.add_with_order(p);
        id
    }

    /// New participant; the code must be unused (`createNewParticipant`).
    pub fn create_new_participant(
        &mut self,
        location: Option<LineLocation>,
        participant_type: ParticipantType,
        code: &str,
        display: Option<Display>,
        order: i32,
    ) -> usize {
        assert!(
            self.find_by_code(code).is_none(),
            "participant {code} exists"
        );
        let display = match display {
            Some(d) if !d.is_null() => d,
            _ => Display::with_newlines(Some(code)),
        };
        let id = self.next_participant_id;
        self.next_participant_id += 1;
        let uid = self.create_uid();
        let p = Participant::new(
            id,
            location,
            participant_type,
            code,
            display,
            self.hidden_portions.clone(),
            order,
            self.base.skin_param.style_builder(),
            uid,
        );
        self.add_with_order(p);
        id
    }

    pub fn configure_participant(&mut self, id: usize, f: impl FnOnce(&mut Participant)) {
        f(self.participant_mut(id));
    }

    pub fn put_participant_in_last(&mut self, code: &str) {
        let id = self.find_by_code(code).expect("participant");
        let pos = self
            .participants
            .iter()
            .position(|p| p.id == id)
            .expect("in list");
        let p = self.participants.remove(pos);
        self.englobers.retain(|(i, _)| *i != id);
        self.add_with_order(p);
    }

    pub fn englober(&self, id: usize) -> Option<Rc<ParticipantEnglober>> {
        self.englobers
            .iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, e)| e.clone())
    }

    pub fn style_builder(&self) -> crate::style::StyleBuilder {
        self.base.skin_param.style_builder()
    }

    /// Appends a message and handles a pending `create` (`addMessage`).
    pub fn add_message(&mut self, mut event: Event) -> CommandExecutionResult {
        let parallel = event.is_parallel();
        let last_message = self.last_abstract_message();
        self.message_count += 1;
        let uid = format!("msg{}", self.message_count);
        let idx = self.events.len();
        match &mut event {
            Event::Message(m) => {
                m.core.uid = uid;
                if parallel {
                    m.core.parallel_brother = last_message;
                }
            }
            Event::MessageExo(m) => {
                m.core.uid = uid;
                if parallel {
                    m.core.parallel_brother = last_message;
                }
            }
            _ => panic!("not a message"),
        }
        let compat_check = |p: usize| match &event {
            Event::Message(m) => m.compatible_for_create(p),
            Event::MessageExo(m) => m.participant == p,
            _ => false,
        };
        self.last_event_with_deactivate = Some(idx);
        self.last_delay = false;
        let pending = self.pending_create.take();
        if let Some(le_idx) = pending {
            let Event::LifeEvent(le) = &self.events[le_idx] else {
                unreachable!()
            };
            let participant = le.participant;
            if !compat_check(participant) {
                self.events.push(event);
                let name = self.participant(participant).code.clone();
                return CommandExecutionResult::error(format!(
                    "After create command, you have to send a message to \"{name}\""
                ));
            }
            self.events.push(event);
            self.attach_life_event(idx, le_idx);
            return CommandExecutionResult::ok();
        }
        self.events.push(event);
        CommandExecutionResult::ok()
    }

    fn last_abstract_message(&self) -> Option<usize> {
        (0..self.events.len())
            .rev()
            .find(|i| matches!(self.events[*i], Event::Message(_) | Event::MessageExo(_)))
    }

    /// Records the life event on a message; false when refused (`addLifeEvent`).
    fn attach_life_event(&mut self, msg_idx: usize, le_idx: usize) -> bool {
        let (participant, kind) = match &mut self.events[le_idx] {
            Event::LifeEvent(le) => {
                le.message = Some(msg_idx);
                (le.participant, le.kind)
            }
            _ => unreachable!(),
        };
        match &mut self.events[msg_idx] {
            Event::Message(m) => {
                let skip = m.is_self_message() && participant != m.p1;
                m.core.add_life_event(participant, kind, skip)
            }
            Event::MessageExo(m) => m.core.add_life_event(participant, kind, false),
            _ => true,
        }
    }

    /// Whether the event (message, divider or group leaf) takes life events.
    fn event_deals_with(&self, idx: usize, p: usize) -> bool {
        self.events[idx].deals_with(p)
    }

    pub fn add_note(&mut self, note: Note, try_merge: bool) {
        if try_merge {
            if let Some(last) = self.events.last_mut() {
                match last {
                    Event::Note(n) => {
                        let first = n.clone();
                        *last = Event::Notes(vec![first, note]);
                        return;
                    }
                    Event::Notes(ns) => {
                        ns.push(note);
                        return;
                    }
                    _ => {}
                }
            }
        }
        self.events.push(Event::Note(note));
    }

    /// Index of the last event that can take a note (`getLastEventWithNote`).
    pub fn last_event_with_note(&self) -> Option<usize> {
        (0..self.events.len())
            .rev()
            .find(|i| match &self.events[*i] {
                Event::Message(_) | Event::MessageExo(_) | Event::Reference(_) => true,
                Event::Grouping(g) => !g.grouping_type.is_start(),
                _ => false,
            })
    }

    /// Attaches a note to an event; returns a warning when the position is not allowed.
    pub fn add_note_on_event(&mut self, idx: usize, note: Note) -> Option<String> {
        let name = format!("{:?}", note.position).to_lowercase();
        match &mut self.events[idx] {
            Event::Message(m) => add_note_message(&mut m.core.notes, note, None, &name),
            Event::MessageExo(m) => {
                let forced = match m.exo_type {
                    MessageExoType::FromLeft | MessageExoType::ToLeft => NotePosition::Right,
                    _ => NotePosition::Left,
                };
                add_note_message(&mut m.core.notes, note, Some(forced), &name)
            }
            Event::Reference(r) => {
                if !matches!(note.position, NotePosition::Left | NotePosition::Right) {
                    return Some(format!(
                        "'note {name}' is not supported on a 'ref': this note is ignored. Use 'note left' or 'note right'"
                    ));
                }
                r.notes.push(note);
                None
            }
            Event::Grouping(g) => {
                let warn = if matches!(note.position, NotePosition::Left | NotePosition::Right) {
                    None
                } else {
                    Some(format!(
                        "'note {name}' is not supported after 'end': use 'note left' or 'note right'"
                    ))
                };
                g.notes.push(note);
                warn
            }
            _ => None,
        }
    }

    pub fn newpage(&mut self, title: DisplayPositioned) {
        if self.ignore_newpage {
            return;
        }
        self.titles.push(title);
        self.events.push(Event::Newpage {
            style_builder: self.style_builder(),
        });
        self.count_newpage += 1;
    }

    pub fn ignore_newpage(&mut self) {
        self.ignore_newpage = true;
    }

    pub fn divider(&mut self, text: Display) {
        self.events.push(Event::Divider {
            text,
            style_builder: self.style_builder(),
            pos_y_end_level: 0.0,
        });
        self.last_event_with_deactivate = Some(self.events.len() - 1);
    }

    pub fn hspace(&mut self, pixel: i32) {
        self.events.push(Event::HSpace(pixel));
    }

    pub fn delay(&mut self, text: Display) {
        self.events.push(Event::Delay {
            text,
            style_builder: self.style_builder(),
        });
        self.last_delay = true;
    }

    /// Activation, deactivation, destroy or create; returns an error message (`activate`).
    pub fn activate(
        &mut self,
        participant: usize,
        kind: LifeEventType,
        back: Option<HColor>,
        line: Option<HColor>,
    ) -> Option<String> {
        if self.last_delay {
            return Some("You cannot Activate/Deactivate just after a ...".into());
        }
        let fashion = Fashion::new(back.clone(), line.clone());
        let le = LifeEvent {
            participant,
            kind,
            specific_colors: fashion.clone(),
            style_builder: self.style_builder(),
            message: None,
        };
        let le_idx = self.events.len();
        self.events.push(Event::LifeEvent(le));
        if kind == LifeEventType::Create {
            self.pending_create = Some(le_idx);
            return None;
        }
        let Some(last) = self.last_event_with_deactivate else {
            if kind == LifeEventType::Activate {
                self.participant_mut(participant).inc_initial_life(fashion);
                return None;
            }
            if self.participant(participant).initial_life() == 0 {
                return Some("You cannot deactivate here".into());
            }
            return None;
        };
        if !self.event_deals_with(last, participant) {
            return None;
        }
        let is_message = matches!(self.events[last], Event::Message(_) | Event::MessageExo(_));
        if kind == LifeEventType::Activate && is_message {
            self.activation_state.push(last);
        } else if kind == LifeEventType::Deactivate && !self.activation_state.is_empty() {
            self.activation_state.pop();
        }
        let ok = if is_message {
            self.attach_life_event(last, le_idx)
        } else {
            true
        };
        if ok {
            None
        } else {
            Some(format!(
                "Activate/Deactivate already done on {}",
                self.participant(participant).code
            ))
        }
    }

    /// Index of the last event taking life events (`getLastEventWithDeactivate`).
    pub fn last_event_with_deactivate(&self) -> Option<usize> {
        (0..self.events.len())
            .rev()
            .find(|i| match &self.events[*i] {
                Event::Message(_) | Event::MessageExo(_) | Event::Divider { .. } => true,
                Event::Grouping(g) => !g.grouping_type.is_start(),
                _ => false,
            })
    }

    /// The message currently being activated (`getActivatingMessage`).
    pub fn activating_message(&self) -> Option<usize> {
        self.activation_state.last().copied()
    }

    /// Opens, continues or closes a group; false when no group is open (`grouping`).
    #[allow(clippy::too_many_arguments)]
    pub fn grouping(
        &mut self,
        title: &str,
        comment: Option<&str>,
        grouping_type: GroupingType,
        back_color_general: Option<HColor>,
        back_color_element: Option<HColor>,
        parallel: bool,
    ) -> bool {
        if !grouping_type.is_start() && self.open_groupings.is_empty() {
            return false;
        }
        let top = self.open_groupings.first().copied();
        let idx = self.events.len();
        let mut g = Grouping {
            title: title.to_string(),
            comment: comment.map(str::to_string),
            grouping_type,
            back_color_element,
            back_color_general,
            style_builder: self.style_builder(),
            start: None,
            parent: None,
            children: Vec::new(),
            parallel: false,
            start_is_par2: false,
            notes: Vec::new(),
        };
        if grouping_type.is_start() {
            g.parent = top;
            if parallel {
                g.parallel = true;
            }
        } else {
            let start = top.expect("open group");
            g.start = Some(start);
            let Event::Grouping(s) = &self.events[start] else {
                unreachable!()
            };
            g.start_is_par2 = s.is_par2();
            if g.back_color_general.is_none() {
                g.back_color_general = s.back_color_general.clone();
            }
        }
        if let Some(start) = g.start {
            if let Event::Grouping(s) = &mut self.events[start] {
                s.children.push(idx);
            }
        }
        self.events.push(Event::Grouping(g));
        if grouping_type.is_start() {
            self.open_groupings.insert(0, idx);
        } else if grouping_type == GroupingType::End {
            self.open_groupings.remove(0);
            self.last_event_with_deactivate = Some(idx);
        }
        true
    }

    pub fn autonumber_go(&mut self, start: DottedNumber, increment: i32, format: &str) {
        self.auto_number.go(start, increment, format);
    }

    pub fn next_message_number(&mut self) -> Option<String> {
        self.auto_number.next_message_number()
    }

    pub fn is_show_footbox(&self) -> bool {
        if self.base.skin_param.strict_uml_style() {
            return false;
        }
        match self.base.skin_param.get_value("footbox") {
            None => self.show_footbox,
            Some(v) => !v.eq_ignore_ascii_case("hide"),
        }
    }

    pub fn set_show_footbox(&mut self, footbox: bool) {
        self.show_footbox = footbox;
    }

    pub fn box_start(
        &mut self,
        comment: Display,
        color: Option<HColor>,
        stereotype: Option<Stereotype>,
    ) {
        self.participant_englober = Some(Rc::new(ParticipantEnglober {
            parent: self.participant_englober.clone(),
            title: comment,
            box_color: color,
            stereotype,
        }));
    }

    pub fn end_box(&mut self) {
        let e = self.participant_englober.take().expect("box open");
        self.participant_englober = e.parent.clone();
    }

    pub fn is_box_pending(&self) -> bool {
        self.participant_englober.is_some()
    }

    pub fn nb_images(&self) -> usize {
        self.count_newpage + 1
    }

    pub fn add_reference(&mut self, r: Reference) {
        self.events.push(Event::Reference(r));
    }

    pub fn link_anchor(&mut self, a1: &str, a2: &str, message: &str) -> CommandExecutionResult {
        self.link_anchors.push(LinkAnchor {
            anchor1: a1.into(),
            anchor2: a2.into(),
            message: message.into(),
        });
        CommandExecutionResult::ok()
    }

    pub fn hide_or_show(&mut self, portions: &[EntityPortion], show: bool) {
        for p in portions {
            if show {
                self.hidden_portions.remove(p);
            } else {
                self.hidden_portions.insert(*p);
            }
        }
    }

    pub fn manage_variable(&self, labels: &Display) -> Display {
        labels.replace(
            "%autonumber%",
            &self.auto_number.current_message_number(false),
        )
    }

    pub fn remove_hidden_participants(&mut self) {
        let alone: Vec<usize> = self
            .participants
            .iter()
            .filter(|p| !self.events.iter().any(|e| e.deals_with(p.id)))
            .map(|p| p.id)
            .collect();
        self.participants.retain(|p| !alone.contains(&p.id));
        self.englobers.retain(|(i, _)| !alone.contains(i));
    }

    /// Body of `engine` for pages: the title of page `index`.
    pub fn title_for_page(&self, index: usize) -> &DisplayPositioned {
        if index > 0 {
            &self.titles[index - 1]
        } else {
            &self.base.title
        }
    }

    pub fn is_incomplete(&self) -> bool {
        self.participants.is_empty()
    }
}

fn add_note_message(
    notes: &mut Vec<Note>,
    note: Note,
    forced: Option<NotePosition>,
    name: &str,
) -> Option<String> {
    if !matches!(
        note.position,
        NotePosition::Left | NotePosition::Right | NotePosition::Bottom | NotePosition::Top
    ) {
        return Some(format!(
            "'note {name}' is not supported on a message: this note is ignored. Use left, right, top or bottom"
        ));
    }
    let note = match forced {
        Some(p) => note.with_position(p),
        None => note,
    };
    notes.push(note);
    None
}

impl crate::command::common::HasBase for SequenceDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

/// Creates sequence diagrams (`SequenceDiagramFactory`).
pub struct SequenceDiagramFactory;

impl crate::diagram::DiagramFactory for SequenceDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Sequence
    }

    fn create(
        &self,
        source: &crate::command::UmlSource,
    ) -> Result<Box<dyn crate::diagram::Diagram>, crate::command::ErrorUml> {
        let diagram = crate::command::create_system(
            &commands::commands(),
            SequenceDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

impl crate::diagram::Diagram for SequenceDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(
        &self,
        sb: &Rc<dyn crate::klimt::font::StringBounder>,
        skin: &Rc<crate::skin::SkinParam>,
    ) -> Rc<dyn crate::klimt::shape::TextBlock> {
        Rc::new(teoz::playing::SequenceBlock::new(
            self,
            sb.clone(),
            skin.clone(),
        ))
    }

    fn default_margins(&self) -> crate::style::Margins {
        crate::style::Margins::same(5.0)
    }
}

impl CommandDiagram for SequenceDiagram {
    fn check_final_error(&self) -> Option<String> {
        None
    }

    fn make_diagram_ready(&mut self) {
        if self.hide_unlinked_data {
            self.remove_hidden_participants();
        }
        let hidden = self.hidden_portions.clone();
        for p in &mut self.participants {
            p.set_hidden_portions(hidden.clone());
        }
    }
}

/// Builds a plain message (`new Message(...)`).
pub fn new_message(
    diagram: &SequenceDiagram,
    p1: usize,
    p2: usize,
    label: Display,
    config: ArrowConfiguration,
    number: Option<String>,
    location: Option<LineLocation>,
) -> Message {
    Message {
        core: MessageCore::new(diagram.style_builder(), label, config, number, location),
        p1,
        p2,
        multicast: Vec::new(),
    }
}
