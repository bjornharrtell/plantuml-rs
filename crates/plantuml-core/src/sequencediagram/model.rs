//! Sequence diagram elements (`sequencediagram/*`).

use crate::klimt::color::colors::Colors;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::HorizontalAlignment;
use crate::skin::arrow::ArrowConfiguration;
use crate::stereo::Stereotype;
use crate::style::queries as q;
use crate::style::{MergeStrategy, PName, Style, StyleBuilder, StyleQuery};
use crate::text::LineLocation;
use crate::url::Url;
use std::collections::BTreeSet;

/// Kinds of participants (`ParticipantType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticipantType {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Queue,
    Database,
    Collections,
}

impl ParticipantType {
    pub fn style_query(self) -> StyleQuery {
        match self {
            ParticipantType::Participant => q::SEQUENCEDIAG_PARTICIPANT.clone(),
            ParticipantType::Actor => q::SEQUENCEDIAG_ACTOR.clone(),
            ParticipantType::Boundary => q::SEQUENCEDIAG_BOUNDARY.clone(),
            ParticipantType::Control => q::SEQUENCEDIAG_CONTROL.clone(),
            ParticipantType::Entity => q::SEQUENCEDIAG_ENTITY.clone(),
            ParticipantType::Queue => q::SEQUENCEDIAG_QUEUE.clone(),
            ParticipantType::Database => q::SEQUENCEDIAG_DATABASE.clone(),
            ParticipantType::Collections => q::SEQUENCEDIAG_COLLECTIONS.clone(),
        }
    }

    /// Parses `participant`, `actor`, ... case-insensitively.
    pub fn parse(s: &str) -> Option<ParticipantType> {
        Some(match s.to_uppercase().as_str() {
            "PARTICIPANT" => ParticipantType::Participant,
            "ACTOR" => ParticipantType::Actor,
            "BOUNDARY" => ParticipantType::Boundary,
            "CONTROL" => ParticipantType::Control,
            "ENTITY" => ParticipantType::Entity,
            "QUEUE" => ParticipantType::Queue,
            "DATABASE" => ParticipantType::Database,
            "COLLECTIONS" => ParticipantType::Collections,
            _ => return None,
        })
    }
}

/// Parts of an entity that can be hidden (`EntityPortion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityPortion {
    Field,
    Method,
    Member,
    Circled,
    Stereotype,
}

/// A participant of the diagram (`Participant`).
#[derive(Debug, Clone)]
pub struct Participant {
    /// Position in creation order, also used as identity.
    pub id: usize,
    pub code: String,
    display: Display,
    pub participant_type: ParticipantType,
    initial_life: usize,
    stereotype: Option<Stereotype>,
    stereotype_position_top: bool,
    hidden_portions: BTreeSet<EntityPortion>,
    pub order: i32,
    style_builder: StyleBuilder,
    pub location: Option<LineLocation>,
    pub uid: String,
    colors: Colors,
    url: Option<Url>,
    live_backcolors: Vec<Fashion>,
}

impl Participant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        location: Option<LineLocation>,
        participant_type: ParticipantType,
        code: &str,
        display: Display,
        hidden_portions: BTreeSet<EntityPortion>,
        order: i32,
        style_builder: StyleBuilder,
        uid: String,
    ) -> Participant {
        assert!(!code.is_empty(), "empty participant code");
        assert!(!display.is_null() && display.size() > 0, "empty display");
        Participant {
            id,
            code: code.to_string(),
            display,
            participant_type,
            initial_life: 0,
            stereotype: None,
            stereotype_position_top: true,
            hidden_portions,
            order,
            style_builder,
            location,
            uid,
            colors: Colors::empty(),
            url: None,
            live_backcolors: Vec::new(),
        }
    }

    pub fn style_query(&self) -> StyleQuery {
        self.participant_type
            .style_query()
            .add_clickable(self.url.is_some())
    }

    /// Style of the box and style of the stereotype (`getUsedStyles`).
    pub fn used_styles(&self) -> [Style; 2] {
        let query = self
            .style_query()
            .with_stereotype_obj(self.stereotype.as_ref());
        let tmp = self
            .style_builder
            .merged_style(&query)
            .expect("participant style")
            .override_colors(&self.colors);
        let stereo_query = self
            .style_query()
            .for_stereotype_itself(self.stereotype.as_ref());
        let stereo = match self.style_builder.merged_style(&stereo_query) {
            Some(s) => tmp.merge_with(&s, MergeStrategy::OverwriteExistingValue),
            None => tmp.clone(),
        };
        [tmp, stereo]
    }

    /// Display with stereotype and optional underline (`getDisplay`).
    pub fn display(&self, underlined: bool) -> Display {
        let mut result = if underlined {
            self.display.underlined()
        } else {
            self.display.clone()
        };
        if let Some(st) = &self.stereotype {
            if !self.hidden_portions.contains(&EntityPortion::Stereotype) {
                let line = crate::klimt::creole::display::DisplayLine::Stereotype(st.clone());
                result = if self.stereotype_position_top {
                    result.add_first(line)
                } else {
                    result.add(line)
                };
            }
        }
        result
    }

    pub fn set_hidden_portions(&mut self, hidden: BTreeSet<EntityPortion>) {
        self.hidden_portions = hidden;
    }

    pub fn set_stereotype(&mut self, stereotype: Stereotype, position_top: bool) {
        assert!(self.stereotype.is_none(), "stereotype already set");
        self.stereotype = Some(stereotype);
        self.stereotype_position_top = position_top;
    }

    pub fn stereotype(&self) -> Option<&Stereotype> {
        self.stereotype.as_ref()
    }

    pub fn initial_life(&self) -> usize {
        self.initial_life
    }

    pub fn inc_initial_life(&mut self, colors: Fashion) {
        self.initial_life += 1;
        self.live_backcolors.push(colors);
    }

    pub fn live_specific_back_colors(&self, which: usize) -> &Fashion {
        &self.live_backcolors[which]
    }

    pub fn colors(&self) -> &Colors {
        &self.colors
    }

    pub fn set_colors(&mut self, colors: Colors) {
        self.colors = colors;
    }

    pub fn set_specific_back_color(&mut self, color: Option<HColor>) {
        if color.is_some() {
            self.colors = self
                .colors
                .add(crate::klimt::color::colors::ColorType::Back, color);
        }
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    pub fn set_url(&mut self, url: Url) {
        self.url = Some(url);
    }

    /// Attributes of the SVG group around the head, tail or lifeline.
    pub fn group_attributes(&self, kind: &str) -> Vec<(&'static str, String)> {
        let mut v = Vec::new();
        v.push((
            "class",
            match kind {
                "head" => "participant participant-head",
                "tail" => "participant participant-tail",
                _ => "participant-lifeline",
            }
            .to_string(),
        ));
        v.push((
            "data-qualified-name",
            crate::skin::rose::line::fix_group_value(&self.code),
        ));
        v.push(("data-entity-uid", self.uid.clone()));
        v.push(("id", format!("{}-{kind}", self.uid)));
        if let Some(l) = self.location {
            v.push(("data-source-line", l.position.to_string()));
        }
        v
    }
}

/// Kind of life event (`LifeEventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifeEventType {
    Activate,
    Deactivate,
    Destroy,
    Create,
}

/// Position of a note (`NotePosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    Left,
    Right,
    Over,
    OverSeveral,
    Bottom,
    Top,
}

impl NotePosition {
    pub fn parse(s: &str) -> Option<NotePosition> {
        Some(match s.to_uppercase().as_str() {
            "LEFT" => NotePosition::Left,
            "RIGHT" => NotePosition::Right,
            "OVER" => NotePosition::Over,
            "BOTTOM" => NotePosition::Bottom,
            "TOP" => NotePosition::Top,
            _ => return None,
        })
    }
}

/// Look of a note (`NoteStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteStyle {
    Normal,
    Hexagonal,
    Box,
}

impl NoteStyle {
    pub fn parse(s: &str) -> NoteStyle {
        if s.eq_ignore_ascii_case("hnote") {
            NoteStyle::Hexagonal
        } else if s.eq_ignore_ascii_case("rnote") {
            NoteStyle::Box
        } else {
            NoteStyle::Normal
        }
    }

    pub fn component_type(self) -> crate::skin::component::ComponentType {
        use crate::skin::component::ComponentType;
        match self {
            NoteStyle::Hexagonal => ComponentType::NoteHexagonal,
            NoteStyle::Box => ComponentType::NoteBox,
            NoteStyle::Normal => ComponentType::Note,
        }
    }

    pub fn style_query(self) -> StyleQuery {
        match self {
            NoteStyle::Hexagonal => q::SEQUENCEDIAG_NOTE_HNOTE.clone(),
            NoteStyle::Box => q::SEQUENCEDIAG_NOTE_RNOTE.clone(),
            NoteStyle::Normal => q::SEQUENCEDIAG_NOTE.clone(),
        }
    }
}

/// A note (`Note`).
#[derive(Debug, Clone)]
pub struct Note {
    /// Participant ids.
    pub p: Option<usize>,
    pub p2: Option<usize>,
    pub display: Display,
    pub position: NotePosition,
    pub note_style: NoteStyle,
    pub colors: Colors,
    pub url: Option<Url>,
    pub stereotype: Option<Stereotype>,
    pub parallel: bool,
    pub style_builder: StyleBuilder,
}

impl Note {
    pub fn new(
        p: Option<usize>,
        p2: Option<usize>,
        position: NotePosition,
        display: Display,
        style_builder: StyleBuilder,
    ) -> Note {
        Note {
            p,
            p2,
            display,
            position,
            note_style: NoteStyle::Normal,
            colors: Colors::empty(),
            url: None,
            stereotype: None,
            parallel: false,
            style_builder,
        }
    }

    pub fn with_position(&self, position: NotePosition) -> Note {
        Note {
            position,
            ..self.clone()
        }
    }

    pub fn style_query(&self) -> StyleQuery {
        self.note_style.style_query()
    }

    pub fn used_styles(&self) -> [Style; 1] {
        let mut query = self.style_query();
        if let Some(st) = &self.stereotype {
            query = query.with_stereotype_obj(Some(st));
        }
        [self
            .style_builder
            .merged_style(&query)
            .expect("note style")
            .override_colors(&self.colors)]
    }

    pub fn deals_with(&self, id: usize) -> bool {
        self.p == Some(id) || self.p2 == Some(id)
    }
}

/// A message between two participants or from/to the border (`AbstractMessage`).
#[derive(Debug, Clone)]
pub struct MessageCore {
    pub label: Display,
    pub arrow_configuration: ArrowConfiguration,
    pub message_number: Option<String>,
    pub location: Option<LineLocation>,
    pub uid: String,
    pub style_builder: StyleBuilder,
    pub stereotype: Option<Stereotype>,
    pub url: Option<Url>,
    pub parallel: bool,
    /// Index of the event this one is parallel with.
    pub parallel_brother: Option<usize>,
    pub life_event_types: Vec<LifeEventType>,
    first_is_activate: bool,
    no_activation_authorized: BTreeSet<usize>,
    pub notes: Vec<Note>,
    pub anchor: Option<String>,
    pub part1_anchor: Option<String>,
    pub part2_anchor: Option<String>,
}

impl MessageCore {
    pub fn new(
        style_builder: StyleBuilder,
        label: Display,
        arrow_configuration: ArrowConfiguration,
        message_number: Option<String>,
        location: Option<LineLocation>,
    ) -> MessageCore {
        MessageCore {
            label,
            arrow_configuration,
            message_number,
            location,
            uid: String::new(),
            style_builder,
            stereotype: None,
            url: None,
            parallel: false,
            parallel_brother: None,
            life_event_types: Vec::new(),
            first_is_activate: false,
            no_activation_authorized: BTreeSet::new(),
            notes: Vec::new(),
            anchor: None,
            part1_anchor: None,
            part2_anchor: None,
        }
    }

    pub fn style_query(&self) -> StyleQuery {
        q::SEQUENCEDIAG_ARROW.with_stereotype_obj(self.stereotype.as_ref())
    }

    pub fn used_style(&self) -> Style {
        let style = self
            .style_builder
            .merged_style(&self.style_query())
            .expect("arrow style");
        match self.arrow_configuration.color() {
            Some(c) => style.override_color(PName::LineColor, Some(c.clone())),
            None => style,
        }
    }

    /// Label with the autonumber prefix (`getLabelNumbered`).
    pub fn label_numbered(&self) -> Display {
        match &self.message_number {
            None => self.label.clone(),
            Some(n) => Display::empty()
                .add(crate::klimt::creole::display::DisplayLine::MessageNumber(
                    n.clone(),
                ))
                .add_all(&self.label),
        }
    }

    pub fn is_create(&self) -> bool {
        self.life_event_types.contains(&LifeEventType::Create)
    }

    pub fn is_activate(&self) -> bool {
        self.life_event_types.contains(&LifeEventType::Activate)
    }

    pub fn is_deactivate(&self) -> bool {
        self.life_event_types.contains(&LifeEventType::Deactivate)
    }

    pub fn is_destroy(&self) -> bool {
        self.life_event_types.contains(&LifeEventType::Destroy)
    }

    pub fn is_activate_and_deactivate(&self) -> bool {
        self.first_is_activate && (self.is_deactivate() || self.is_destroy())
    }

    /// Records a life event; false when activation is refused (`addLifeEvent`).
    pub fn add_life_event(
        &mut self,
        participant: usize,
        kind: LifeEventType,
        is_self_message_other_participant: bool,
    ) -> bool {
        if is_self_message_other_participant {
            return true;
        }
        if !self.life_event_types.contains(&kind) {
            self.life_event_types.push(kind);
        }
        if self.life_event_types.len() == 1 && self.is_activate() {
            self.first_is_activate = true;
        }
        if kind == LifeEventType::Activate && self.no_activation_authorized.contains(&participant) {
            return false;
        }
        if matches!(kind, LifeEventType::Deactivate | LifeEventType::Destroy) {
            self.no_activation_authorized.insert(participant);
        }
        true
    }

    pub fn has_url(&self) -> bool {
        self.notes.iter().any(|n| n.url.is_some()) || self.label.has_url() || self.url.is_some()
    }
}

/// Message between two participants (`Message`).
#[derive(Debug, Clone)]
pub struct Message {
    pub core: MessageCore,
    pub p1: usize,
    pub p2: usize,
    pub multicast: Vec<usize>,
}

impl Message {
    pub fn is_self_message(&self) -> bool {
        self.p1 == self.p2
    }

    pub fn compatible_for_create(&self, p: usize) -> bool {
        self.p1 != p && self.p2 == p
    }
}

/// Kind of arrow coming from or going to the border (`MessageExoType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageExoType {
    FromLeft,
    ToLeft,
    FromRight,
    ToRight,
}

impl MessageExoType {
    pub fn direction(self) -> i32 {
        match self {
            MessageExoType::FromLeft | MessageExoType::ToRight => 1,
            MessageExoType::ToLeft | MessageExoType::FromRight => -1,
        }
    }

    pub fn is_left_border(self) -> bool {
        matches!(self, MessageExoType::FromLeft | MessageExoType::ToLeft)
    }

    pub fn is_right_border(self) -> bool {
        matches!(self, MessageExoType::FromRight | MessageExoType::ToRight)
    }

    pub fn reverse(self) -> MessageExoType {
        match self {
            MessageExoType::FromLeft => MessageExoType::ToLeft,
            MessageExoType::ToRight => MessageExoType::FromRight,
            MessageExoType::FromRight => MessageExoType::ToRight,
            MessageExoType::ToLeft => MessageExoType::FromLeft,
        }
    }
}

/// Message from or to the border of the diagram (`MessageExo`).
#[derive(Debug, Clone)]
pub struct MessageExo {
    pub core: MessageCore,
    pub participant: usize,
    pub exo_type: MessageExoType,
    pub short_arrow: bool,
}

/// Activation, deactivation, destroy or create (`LifeEvent`).
#[derive(Debug, Clone)]
pub struct LifeEvent {
    pub participant: usize,
    pub kind: LifeEventType,
    pub specific_colors: Fashion,
    pub style_builder: StyleBuilder,
    /// Index of the message the event is attached to.
    pub message: Option<usize>,
}

impl LifeEvent {
    pub fn is_activate(&self) -> bool {
        self.kind == LifeEventType::Activate
    }

    pub fn is_deactivate_or_destroy(&self) -> bool {
        matches!(
            self.kind,
            LifeEventType::Deactivate | LifeEventType::Destroy
        )
    }

    pub fn is_destroy(&self, participant: usize) -> bool {
        self.participant == participant && self.kind == LifeEventType::Destroy
    }

    pub fn style(&self) -> Style {
        self.style_builder
            .merged_style(&q::SEQUENCEDIAG_LIFELINE_DESTROY)
            .expect("destroy style")
    }
}

/// Kind of grouping (`GroupingType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingType {
    Start,
    StartPartition,
    Else,
    End,
}

impl GroupingType {
    pub fn parse(s: &str) -> GroupingType {
        match s.to_lowercase().as_str() {
            "opt" | "alt" | "loop" | "par" | "par2" | "break" | "group" | "critical" => {
                GroupingType::Start
            }
            "partition" => GroupingType::StartPartition,
            "also" | "else" => GroupingType::Else,
            "end" => GroupingType::End,
            other => panic!("unknown grouping {other}"),
        }
    }

    pub fn is_start(self) -> bool {
        matches!(self, GroupingType::Start | GroupingType::StartPartition)
    }
}

/// Opening of a group, or one of its `else`/`end` leaves (`Grouping`).
#[derive(Debug, Clone)]
pub struct Grouping {
    pub title: String,
    pub comment: Option<String>,
    pub grouping_type: GroupingType,
    pub back_color_element: Option<HColor>,
    pub back_color_general: Option<HColor>,
    pub style_builder: StyleBuilder,
    /// For a leaf: index of its start event.
    pub start: Option<usize>,
    /// For a start: index of its parent start.
    pub parent: Option<usize>,
    /// For a start: indexes of its leaves.
    pub children: Vec<usize>,
    pub parallel: bool,
    /// For a leaf: whether its start is a `par2`.
    pub start_is_par2: bool,
    pub notes: Vec<Note>,
}

impl Grouping {
    pub fn is_par2(&self) -> bool {
        self.title == "par2"
    }

    pub fn style_query(&self) -> StyleQuery {
        if self.grouping_type == GroupingType::StartPartition {
            q::SEQUENCEDIAG_PARTITION.clone()
        } else {
            q::SEQUENCEDIAG_GROUP.clone()
        }
    }

    fn header_query(&self) -> StyleQuery {
        if self.grouping_type == GroupingType::StartPartition {
            q::SEQUENCEDIAG_PARTITION_HEADER.clone()
        } else {
            q::SEQUENCEDIAG_GROUPHEADER.clone()
        }
    }

    /// Body and header styles, with the explicit background colors applied.
    pub fn used_styles(&self, back_general: Option<&HColor>) -> [Option<Style>; 2] {
        let style = self.style_builder.merged_style(&self.style_query());
        let flat = self.style_builder.merged_style(&self.header_query());
        let header = if self.grouping_type == GroupingType::StartPartition {
            flat
        } else {
            let nested = self
                .style_builder
                .merged_style(&q::SEQUENCEDIAG_GROUP_HEADER);
            match (&flat, &nested) {
                (None, _) => nested,
                (Some(f), n) => Some(f.merge_nested_child_over(
                    n.as_ref(),
                    style.as_ref(),
                    MergeStrategy::OverwriteExistingValue,
                )),
            }
        };
        [
            style.map(|s| s.override_color(PName::BackGroundColor, back_general.cloned())),
            header
                .map(|s| s.override_color(PName::BackGroundColor, self.back_color_element.clone())),
        ]
    }
}

/// Reference over participants (`Reference`).
#[derive(Debug, Clone)]
pub struct Reference {
    pub participants: Vec<usize>,
    pub url: Option<Url>,
    pub back_color_general: Option<HColor>,
    pub back_color_element: Option<HColor>,
    pub strings: Display,
    pub style_builder: StyleBuilder,
    pub notes: Vec<Note>,
}

impl Reference {
    pub fn new(
        participants: &[usize],
        url: Option<Url>,
        strings: Display,
        back_color_general: Option<HColor>,
        back_color_element: Option<HColor>,
        style_builder: StyleBuilder,
    ) -> Reference {
        let mut uniq: Vec<usize> = Vec::new();
        for p in participants {
            if !uniq.contains(p) {
                uniq.push(*p);
            }
        }
        Reference {
            participants: uniq,
            url,
            back_color_general,
            back_color_element,
            strings,
            style_builder,
            notes: Vec::new(),
        }
    }

    pub fn used_styles(&self) -> [Style; 2] {
        let style = self
            .style_builder
            .merged_style(&q::SEQUENCEDIAG_REFERENCE)
            .expect("reference style");
        let flat = self
            .style_builder
            .merged_style(&q::SEQUENCEDIAG_REFERENCEHEADER);
        let nested = self
            .style_builder
            .merged_style(&q::SEQUENCEDIAG_REFERENCE_HEADER);
        let header = match flat {
            None => nested.expect("reference header style"),
            Some(f) => f.merge_nested_child_over(
                nested.as_ref(),
                Some(&style),
                MergeStrategy::OverwriteExistingValue,
            ),
        };
        [
            style,
            header.override_color(PName::BackGroundColor, self.back_color_element.clone()),
        ]
    }
}

/// Events of a sequence diagram, in source order (`Event`).
#[derive(Debug, Clone)]
pub enum Event {
    Message(Message),
    MessageExo(MessageExo),
    LifeEvent(LifeEvent),
    Note(Note),
    Notes(Vec<Note>),
    Divider {
        text: Display,
        style_builder: StyleBuilder,
        pos_y_end_level: f64,
    },
    Delay {
        text: Display,
        style_builder: StyleBuilder,
    },
    HSpace(i32),
    Newpage {
        style_builder: StyleBuilder,
    },
    Grouping(Grouping),
    Reference(Reference),
}

impl Event {
    /// True when the event concerns the participant (`dealWith`).
    pub fn deals_with(&self, id: usize) -> bool {
        match self {
            Event::Message(m) => m.p1 == id || m.p2 == id,
            Event::MessageExo(m) => m.participant == id,
            Event::LifeEvent(l) => l.participant == id,
            Event::Note(n) => n.deals_with(id),
            Event::Notes(ns) => ns.iter().any(|n| n.deals_with(id)),
            Event::Reference(r) => r.participants.contains(&id),
            _ => false,
        }
    }

    pub fn is_parallel(&self) -> bool {
        match self {
            Event::Message(m) => m.core.parallel,
            Event::MessageExo(m) => m.core.parallel,
            Event::Note(n) => n.parallel,
            Event::Grouping(g) => {
                if g.grouping_type.is_start() {
                    g.parallel || g.is_par2()
                } else {
                    g.start_is_par2
                }
            }
            _ => false,
        }
    }

    pub fn message_core(&self) -> Option<&MessageCore> {
        match self {
            Event::Message(m) => Some(&m.core),
            Event::MessageExo(m) => Some(&m.core),
            _ => None,
        }
    }
}

/// Automatic message numbering (`AutoNumber`).
#[derive(Debug, Clone, Default)]
pub struct AutoNumber {
    running: bool,
    current: Option<DottedNumber>,
    increment: i32,
    format: String,
    last: String,
}

impl AutoNumber {
    pub fn go(&mut self, start: DottedNumber, increment: i32, format: &str) {
        self.running = true;
        self.current = Some(start);
        self.increment = increment;
        self.format = format.to_string();
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn resume(&mut self, increment: Option<i32>, format: Option<&str>) {
        self.running = true;
        if let Some(i) = increment {
            self.increment = i;
        }
        if let Some(f) = format {
            self.format = f.to_string();
        }
    }

    pub fn increment_intermediate(&mut self, position: Option<usize>) {
        if let Some(c) = &mut self.current {
            match position {
                Some(p) => c.increment_intermediate_at(p),
                None => c.increment_intermediate(),
            }
        }
    }

    pub fn next_message_number(&mut self) -> Option<String> {
        if !self.running {
            return None;
        }
        let cur = self.current.as_mut()?;
        self.last = cur.format(&self.format);
        cur.increment_minor(self.increment);
        Some(self.last.clone())
    }

    pub fn current_message_number(&self, formatted: bool) -> String {
        if formatted {
            self.last.clone()
        } else {
            self.last.replace("<b>", "").replace("</b>", "")
        }
    }
}

/// `1.2.3`-style numbers (`DottedNumber`).
#[derive(Debug, Clone, PartialEq)]
pub struct DottedNumber {
    nums: Vec<i64>,
    separators: Vec<String>,
}

impl DottedNumber {
    pub fn create(value: &str) -> DottedNumber {
        let mut nums = Vec::new();
        let mut separators = Vec::new();
        let mut cur = String::new();
        let mut cur_digit: Option<bool> = None;
        let mut flush = |cur: &mut String, digit: Option<bool>| {
            if cur.is_empty() {
                return;
            }
            if digit == Some(true) {
                nums.push(cur.parse().unwrap_or(0));
            } else {
                separators.push(cur.clone());
            }
            cur.clear();
        };
        for c in value.chars() {
            let d = c.is_ascii_digit();
            if cur_digit != Some(d) {
                flush(&mut cur, cur_digit);
                cur_digit = Some(d);
            }
            cur.push(c);
        }
        flush(&mut cur, cur_digit);
        DottedNumber { nums, separators }
    }

    pub fn increment_minor(&mut self, step: i32) {
        if let Some(last) = self.nums.last_mut() {
            *last += i64::from(step);
        }
    }

    pub fn increment_intermediate(&mut self) {
        let pos = if self.nums.len() == 1 {
            0
        } else {
            self.nums.len() - 2
        };
        self.increment_intermediate_at(pos);
    }

    pub fn increment_intermediate_at(&mut self, pos: usize) {
        if pos >= self.nums.len() {
            return;
        }
        for n in &mut self.nums[pos + 1..] {
            *n = 1;
        }
        self.nums[pos] += 1;
    }

    /// Text of the number, `<b>` wrapped when composed (`format`).
    pub fn format(&self, pattern: &str) -> String {
        if self.nums.len() == 1 && self.separators.is_empty() {
            return format_decimal_pattern(pattern, self.nums[0]);
        }
        format!("<b>{self}</b>")
    }
}

impl std::fmt::Display for DottedNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, n) in self.nums.iter().enumerate() {
            write!(f, "{n}")?;
            if let Some(s) = self.separators.get(i) {
                f.write_str(s)?;
            }
        }
        Ok(())
    }
}

/// Minimal `java.text.DecimalFormat`: `0` pads, `#` is optional, `'` quotes.
pub fn format_decimal_pattern(pattern: &str, value: i64) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let is_digit_spec = |c: char| c == '0' || c == '#';
    let first = chars.iter().position(|c| is_digit_spec(*c));
    let Some(first) = first else {
        return unquote_literal(pattern);
    };
    let last = chars
        .iter()
        .rposition(|c| is_digit_spec(*c))
        .expect("found");
    let spec: String = chars[first..=last].iter().collect();
    let min_digits = spec.chars().filter(|c| *c == '0').count();
    let digits = value.abs().to_string();
    let mut num = String::new();
    for _ in digits.len()..min_digits {
        num.push('0');
    }
    num.push_str(&digits);
    if value < 0 {
        num.insert(0, '-');
    }
    let prefix: String = chars[..first].iter().collect();
    let suffix: String = chars[last + 1..].iter().collect();
    format!(
        "{}{num}{}",
        unquote_literal(&prefix),
        unquote_literal(&suffix)
    )
}

fn unquote_literal(s: &str) -> String {
    s.replace("''", "\u{0}")
        .replace('\'', "")
        .replace('\u{0}', "'")
}

/// Group of participants drawn in a box (`ParticipantEnglober`).
#[derive(Debug, Clone)]
pub struct ParticipantEnglober {
    pub parent: Option<std::rc::Rc<ParticipantEnglober>>,
    pub title: Display,
    pub box_color: Option<HColor>,
    pub stereotype: Option<Stereotype>,
}

impl ParticipantEnglober {
    /// From the outermost to this one (`getGenealogy`).
    pub fn genealogy(self: &std::rc::Rc<Self>) -> Vec<std::rc::Rc<ParticipantEnglober>> {
        let mut v = vec![self.clone()];
        let mut cur = self.parent.clone();
        while let Some(p) = cur {
            v.insert(0, p.clone());
            cur = p.parent.clone();
        }
        v
    }
}

/// Link between two anchors (`LinkAnchor`).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkAnchor {
    pub anchor1: String,
    pub anchor2: String,
    pub message: String,
}

/// Placement hint kept for notes over several participants.
pub const DEFAULT_ALIGNMENT: HorizontalAlignment = HorizontalAlignment::Left;
