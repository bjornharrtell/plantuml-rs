//! Timing diagrams (`timingdiagram` package): clock, binary, analog, robust, concise and
//! rectangle players over a shared time axis.

mod commands;
mod model;
mod panels;
mod ruler;
mod tick;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::colors::Colors;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{with_margin, TextBlock};
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, Margins, PName};
use model::{Highlight, Player, PlayerKind, StateKind, TimeMessage, TimeSeries};
use panels::{
    IntricatedPoint, Panels, PanelsAnalog, PanelsBase, PanelsBinary, PanelsClock, PanelsRobust,
    PanelsState, PlayerFrame, TimeArrow,
};
use ruler::{TimeAxisStrategy, TimingRuler};
use std::collections::HashMap;
use std::rc::Rc;
use tick::{DateFormat, Decimal, TimeTick, TimingFormat};

const MARGIN_X1: f64 = 5.0;
const MARGIN_X2: f64 = 5.0;

/// The diagram being built by commands (`TimingDiagram`).
#[derive(Debug, Clone)]
pub struct TimingDiagram {
    pub base: DiagramBase,
    codes: Vec<(String, TimeTick)>,
    players: Vec<(String, Player)>,
    clocks: HashMap<String, Decimal>,
    messages: Vec<TimeMessage>,
    highlights: Vec<Highlight>,
    ruler: Rc<TimingRuler>,
    now: Option<TimeTick>,
    last_player: Option<usize>,
    time_axis_strategy: TimeAxisStrategy,
    compact_by_default: bool,
    date_format: Option<Rc<DateFormat>>,
}

impl TimingDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Timing),
            codes: Vec::new(),
            players: Vec::new(),
            clocks: HashMap::new(),
            messages: Vec::new(),
            highlights: Vec::new(),
            ruler: Rc::new(TimingRuler::default()),
            now: None,
            last_player: None,
            time_axis_strategy: TimeAxisStrategy::Automatic,
            compact_by_default: false,
            date_format: None,
        }
    }

    fn put_player(&mut self, code: &str, player: Player) -> usize {
        let idx = match self.players.iter().position(|(c, _)| c == code) {
            Some(i) => {
                self.players[i].1 = player;
                i
            }
            None => {
                self.players.push((code.to_string(), player));
                self.players.len() - 1
            }
        };
        idx
    }

    pub fn create_player_state(
        &mut self,
        kind: StateKind,
        code: &str,
        full: &str,
        compact: bool,
        stereotype: Option<Stereotype>,
        back: Option<HColor>,
    ) {
        let player = Player::new_state(
            kind,
            full,
            self.compact_by_default || compact,
            stereotype,
            back,
        );
        let idx = self.put_player(code, player);
        self.last_player = Some(idx);
    }

    pub fn create_player_clock(
        &mut self,
        code: &str,
        full: &str,
        period: Decimal,
        pulse: Decimal,
        offset: Decimal,
        stereotype: Option<Stereotype>,
    ) {
        let player = Player::new(
            PlayerKind::Clock {
                period,
                pulse,
                offset,
            },
            full,
            self.compact_by_default,
            stereotype,
            None,
            30,
        );
        self.put_player(code, player);
        self.clocks.insert(code.to_string(), period);
        self.ruler
            .add_time(TimeTick::new(period, TimingFormat::Decimal));
    }

    /// Creates an analog player and returns its index.
    pub fn create_player_analog(
        &mut self,
        code: &str,
        full: &str,
        stereotype: Option<Stereotype>,
    ) -> usize {
        let player = Player::new(
            PlayerKind::Analog {
                series: TimeSeries::default(),
                constraints: Vec::new(),
                initial_state: None,
                ticks_every: None,
            },
            full,
            self.compact_by_default,
            stereotype,
            None,
            100,
        );
        self.put_player(code, player)
    }

    pub fn create_player_binary(&mut self, code: &str, full: &str, stereotype: Option<Stereotype>) {
        let player = Player::new(
            PlayerKind::Binary {
                constraints: Vec::new(),
                values: Vec::new(),
                initial_state: None,
            },
            full,
            self.compact_by_default,
            stereotype,
            None,
            30,
        );
        self.put_player(code, player);
    }

    pub fn create_time_message(
        &mut self,
        player1: usize,
        tick1: Option<TimeTick>,
        player2: usize,
        tick2: Option<TimeTick>,
        label: Option<&str>,
    ) -> &mut TimeMessage {
        self.messages.push(TimeMessage {
            player1,
            tick1,
            player2,
            tick2,
            label: Display::with_newlines(label),
            specific_color: None,
        });
        self.messages.last_mut().expect("just pushed")
    }

    pub fn add_time(&mut self, time: TimeTick, code: Option<&str>) {
        self.now = Some(time.clone());
        self.ruler.add_time(time.clone());
        if let Some(code) = code {
            match self.codes.iter_mut().find(|(c, _)| c == code) {
                Some(e) => e.1 = time,
                None => self.codes.push((code.to_string(), time)),
            }
        }
    }

    pub fn code_value(&self, code: &str) -> Option<TimeTick> {
        self.codes
            .iter()
            .find(|(c, _)| c == code)
            .map(|(_, t)| t.clone())
    }

    pub fn update_now(&mut self, time: Option<TimeTick>) {
        self.now = time;
    }

    pub fn player_index(&self, code: &str) -> Option<usize> {
        self.players.iter().position(|(c, _)| c == code)
    }

    pub fn player_mut(&mut self, idx: usize) -> &mut Player {
        &mut self.players[idx].1
    }

    pub fn now(&self) -> Option<TimeTick> {
        self.now.clone()
    }

    pub fn clock_value(&self, clock: &str, nb: i64) -> Option<TimeTick> {
        let period = self.clocks.get(clock)?;
        Some(TimeTick::new(period.mul_int(nb), TimingFormat::Decimal))
    }

    pub fn set_last_player(&mut self, idx: usize) {
        self.last_player = Some(idx);
    }

    pub fn last_player(&self) -> Option<usize> {
        self.last_player
    }

    pub fn scale_in_pixels(&mut self, tick: i64, pixel: i64) {
        self.ruler.scale_in_pixels(tick, pixel);
    }

    pub fn set_time_axis_strategy(&mut self, s: TimeAxisStrategy) -> CommandExecutionResult {
        self.time_axis_strategy = s;
        CommandExecutionResult::ok()
    }

    pub fn highlight(
        &mut self,
        from: TimeTick,
        to: TimeTick,
        caption: Display,
        colors: Colors,
    ) -> CommandExecutionResult {
        self.highlights.push(Highlight {
            from,
            to,
            caption,
            colors,
        });
        CommandExecutionResult::ok()
    }

    pub fn go_compact_mode(&mut self) {
        self.compact_by_default = true;
    }

    pub fn use_date_format(&mut self, format: &str) -> CommandExecutionResult {
        match DateFormat::new(format) {
            Some(f) => {
                self.date_format = Some(Rc::new(f));
                CommandExecutionResult::ok()
            }
            None => CommandExecutionResult::error("Bad date format"),
        }
    }

    pub fn timing_format_date(&self) -> TimingFormat {
        match &self.date_format {
            None => TimingFormat::Date,
            Some(f) => TimingFormat::Custom(f.clone()),
        }
    }
}

impl HasBase for TimingDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for TimingDiagram {}

/// A player with its panels built for drawing.
struct BuiltPlayer {
    frame: PlayerFrame,
    panels: Box<dyn Panels>,
    compact: bool,
    general_background: Option<HColor>,
}

/// The whole drawing (`TimingDiagram.getTextBlock`).
struct TimingBlock {
    players: Vec<BuiltPlayer>,
    messages: Vec<TimeMessage>,
    highlights: Vec<Highlight>,
    codes: Vec<(String, TimeTick)>,
    ruler: Rc<TimingRuler>,
    skin: Rc<SkinParam>,
    strategy: TimeAxisStrategy,
    compact_by_default: bool,
}

fn build_player(player: &Player, skin: &Rc<SkinParam>, ruler: &Rc<TimingRuler>) -> Box<dyn Panels> {
    let style = player.style(skin);
    let base = |notes: bool, constraints: Vec<model::TimeConstraint>| PanelsBase {
        ruler: ruler.clone(),
        skin: skin.clone(),
        suggested_height: f64::from(player.suggested_height),
        style: style.clone(),
        notes: if notes {
            player.notes.clone()
        } else {
            Vec::new()
        },
        constraints,
    };
    match &player.kind {
        PlayerKind::Clock {
            period,
            pulse,
            offset,
        } => Box::new(PanelsClock {
            base: base(false, Vec::new()),
            period: period.to_f64(),
            pulse: pulse.to_f64(),
            offset: offset.to_f64(),
        }),
        PlayerKind::Analog {
            series,
            constraints,
            initial_state,
            ticks_every,
        } => Box::new(PanelsAnalog {
            base: base(false, constraints.clone()),
            series: series.clone(),
            initial_state: *initial_state,
            ticks_every: *ticks_every,
        }),
        PlayerKind::Binary {
            constraints,
            values,
            initial_state,
        } => Box::new(PanelsBinary {
            base: base(true, constraints.clone()),
            values: values.clone(),
            initial_state: initial_state.clone(),
        }),
        PlayerKind::State(d) => match d.kind {
            StateKind::Robust => {
                let labels = d.states_label.iter().map(|(_, l)| l.clone()).collect();
                let mut p = PanelsRobust::new(base(false, d.constraints.clone()), labels);
                p.set_initial_state(d.initial_state.clone());
                for c in &d.changes {
                    p.add_change(c.clone());
                }
                Box::new(p)
            }
            kind => {
                let mut p = PanelsState::new(base(true, d.constraints.clone()), kind);
                p.set_initial_state(d.initial_state.clone(), d.initial_colors.clone());
                for c in &d.changes {
                    p.add_change(c.clone());
                }
                Box::new(p)
            }
        },
    }
}

impl TimingBlock {
    fn black(&self) -> HColor {
        self.style().value(PName::LineColor).as_color()
    }

    fn style(&self) -> crate::style::Style {
        self.skin
            .style_builder()
            .merged_style(&queries::TIMINGDIAG)
            .expect("timing diagram style")
    }

    fn part1_max_width(&self, sb: &dyn StringBounder) -> f64 {
        self.players
            .iter()
            .fold(0.0, |w, p| f64::max(w, p.panels.left_panel_width(sb)))
    }

    fn width_total(&self, sb: &dyn StringBounder) -> f64 {
        self.part1_max_width(sb) + self.ruler.width() + MARGIN_X1 + MARGIN_X2
    }

    /// Translation to a player's panels; `None` gives the end of the last one.
    fn translate_for_player(&self, candidate: Option<usize>, sb: &dyn StringBounder) -> UTranslate {
        let mut y = 0.0;
        for (i, p) in self.players.iter().enumerate() {
            y += p.frame.height(sb);
            if candidate == Some(i) {
                return UTranslate::dy(y);
            }
            y += p.panels.full_height(sb);
        }
        UTranslate::dy(y)
    }

    fn translate_for_player_frame(
        &self,
        candidate: Option<usize>,
        sb: &dyn StringBounder,
    ) -> UTranslate {
        let mut y = 0.0;
        for (i, p) in self.players.iter().enumerate() {
            if candidate == Some(i) {
                return UTranslate::dy(y);
            }
            y += p.frame.height(sb);
            y += p.panels.full_height(sb);
        }
        UTranslate::dy(y)
    }

    fn height_inner(&self, sb: &dyn StringBounder) -> f64 {
        self.translate_for_player(None, sb).dy
    }

    fn height_total(&self, sb: &dyn StringBounder) -> f64 {
        self.height_inner(sb) + self.ruler.height(sb, &self.skin)
    }

    fn stroke(&self) -> crate::klimt::drawing::ugraphic::UStroke {
        self.style().stroke()
    }

    fn draw_border(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let border = ULine::vline(self.translate_for_player(None, sb).dy);
        let ug = ug.apply_color(self.black()).apply_stroke(self.stroke());
        ug.draw(border.clone());
        ug.apply_translate(UTranslate::dx(self.width_total(sb)))
            .draw(border);
    }

    fn draw_horizontal_separator(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let ug = ug
            .apply_color(self.black())
            .apply_stroke(self.stroke())
            .apply_translate(UTranslate::dx(-MARGIN_X1));
        ug.draw(ULine::hline(self.width_total(sb)));
    }

    fn draw_messages(&self, ug: &UGraphic, message: &TimeMessage) {
        let sb = ug.string_bounder();
        let (Some(t1), Some(t2)) = (&message.tick1, &message.tick2) else {
            return;
        };
        let tr1 = self.translate_for_player(Some(message.player1), sb);
        let tr2 = self.translate_for_player(Some(message.player2), sb);
        let p1: Option<IntricatedPoint> =
            self.players[message.player1].panels.time_projection(sb, t1);
        let p2 = self.players[message.player2].panels.time_projection(sb, t2);
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return;
        };
        TimeArrow::create(p1.translated(tr1), p2.translated(tr2)).draw(ug, message, &self.skin);
    }
}

impl TextBlock for TimingBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(self.width_total(sb), self.height_total(sb))
    }

    fn draw(&self, ug: &UGraphic) {
        self.ruler.ensure_not_empty();
        let sb = ug.string_bounder();
        let part1 = self.part1_max_width(sb);
        for (i, player) in self.players.iter().enumerate() {
            if let Some(bg) = &player.general_background {
                let ug_player = ug.apply_translate(self.translate_for_player_frame(Some(i), sb));
                let full_height = player.frame.height(sb) + player.panels.full_height(sb);
                let ug_player = ug_player
                    .apply_color(bg.clone())
                    .apply_back_color(bg.clone());
                ug_player.draw(URectangle::build(self.width_total(sb), full_height));
            }
        }
        let width_part1 = UTranslate::dx(part1);
        if !self.compact_by_default {
            self.draw_border(ug);
        }
        let ug = ug.apply_translate(UTranslate::dx(MARGIN_X1));
        let height_inner = self.height_inner(sb);
        for h in &self.highlights {
            h.draw_back(
                &ug.apply_translate(width_part1),
                &self.ruler,
                height_inner,
                &self.skin,
            );
        }
        self.ruler
            .draw_vlines(&ug.apply_translate(width_part1), height_inner, &self.skin);
        for (i, player) in self.players.iter().enumerate() {
            let ug_player = ug.apply_translate(self.translate_for_player(Some(i), sb));
            let ug_frame = ug.apply_translate(self.translate_for_player_frame(Some(i), sb));
            if !player.compact {
                self.draw_horizontal_separator(&ug_frame);
            }
            player.frame.draw_frame_title(&ug_frame);
            player.panels.draw_left_panel(&ug_player, part1);
            player
                .panels
                .draw_right_panel(&ug_player.apply_translate(width_part1));
        }
        let ug = ug.apply_translate(width_part1);
        self.ruler.draw_time_axis(
            &ug.apply_translate(self.translate_for_player(None, sb)),
            self.strategy,
            &self.codes,
            &self.skin,
        );
        for m in &self.messages {
            self.draw_messages(&ug, m);
        }
        for h in &self.highlights {
            h.draw_lines(&ug, &self.ruler, height_inner, &self.skin);
            let start = self.ruler.pos_in_pixel(&h.from);
            h.caption_block(&self.skin)
                .draw(&ug.apply_translate(UTranslate::new(start + 3.0, 2.0)));
        }
    }
}

impl Diagram for TimingDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        // the ruler is edited while drawing: work on a copy
        let ruler = Rc::new((*self.ruler).clone());
        let players = self
            .players
            .iter()
            .map(|(_, p)| BuiltPlayer {
                frame: PlayerFrame::new(&p.title, skin, p.compact),
                panels: build_player(p, skin, &ruler),
                compact: p.compact,
                general_background: p.general_background.clone(),
            })
            .collect();
        let block: Rc<dyn TextBlock> = Rc::new(TimingBlock {
            players,
            messages: self.messages.clone(),
            highlights: self.highlights.clone(),
            codes: self.codes.clone(),
            ruler,
            skin: skin.clone(),
            strategy: self.time_axis_strategy,
            compact_by_default: self.compact_by_default,
        });
        with_margin(block, Margins::new(10.0, 10.0, 10.0, 10.0))
    }
}

pub struct TimingDiagramFactory;

/// Whether `line` declares a timing player, possibly after `mode compact`.
fn declares_player(line: &str) -> bool {
    let mut words = line.split_whitespace();
    let mut first = words.next().unwrap_or("").to_ascii_lowercase();
    if first == "mode"
        && words
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("compact"))
    {
        first = words.next().unwrap_or("").to_ascii_lowercase();
    }
    matches!(
        first.as_str(),
        "clock" | "binary" | "analog" | "robust" | "concise" | "rectangle"
    )
}

impl DiagramFactory for TimingDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Timing
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let explicit = source.lines.first().is_some_and(|l| {
            l.string()
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("@starttiming")
        });
        // `@startuml` is shared with the other diagram types: only claim it when a
        // timing player is declared, and give a losing score otherwise so that a
        // more meaningful diagnostic from another factory wins.
        if !explicit && !source.lines.iter().any(|l| declares_player(l.string())) {
            return Err(ErrorUml {
                kind: crate::command::ErrorUmlType::SyntaxError,
                message: "No timing player defined".into(),
                score: -1,
                line: source.lines.get(1).cloned(),
            });
        }
        let diagram = create_system(
            &commands::commands(),
            TimingDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests;
