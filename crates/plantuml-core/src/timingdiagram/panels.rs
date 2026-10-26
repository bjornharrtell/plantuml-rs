//! How each kind of player is drawn (`timingdiagram/graphic`).

use super::model::{
    is_flat, path, ChangeState, StateKind, TimeConstraint, TimeMessage, TimeSeries, TimingNote,
};
use super::ruler::{apply_for_vlines, TimingRuler};
use super::tick::TimeTick;
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPolygon, URectangle, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{HorizontalAlignment, Position};
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::cmp::Ordering;
use std::rc::Rc;

pub const MARGIN_Y: f64 = 8.0;
pub const MARGIN_X: f64 = 12.0;
pub const BOTTOM_MARGIN: f64 = 10.0;
pub const LEFT_PANEL_MIN_WIDTH: f64 = 5.0;
const MARGIN_X1: f64 = 5.0;

/// A point on a player where an arrow can start or end; two when the line jumps there
/// (`IntricatedPoint`).
#[derive(Debug, Clone, Copy)]
pub struct IntricatedPoint {
    pub a: (f64, f64),
    pub b: (f64, f64),
}

impl IntricatedPoint {
    pub fn translated(&self, t: UTranslate) -> IntricatedPoint {
        IntricatedPoint {
            a: (self.a.0 + t.dx, self.a.1 + t.dy),
            b: (self.b.0 + t.dx, self.b.1 + t.dy),
        }
    }
}

/// What every kind of panel shares (`Panels`).
pub struct PanelsBase {
    pub ruler: Rc<TimingRuler>,
    pub skin: Rc<SkinParam>,
    pub suggested_height: f64,
    pub style: Style,
    pub notes: Vec<TimingNote>,
    pub constraints: Vec<TimeConstraint>,
}

impl PanelsBase {
    fn draw_notes(&self, ug: &UGraphic, position: Position) {
        for note in &self.notes {
            if note.position == position {
                let x = note
                    .when
                    .as_ref()
                    .map_or(0.0, |w| self.ruler.pos_in_pixel(w));
                note.draw(&ug.apply_translate(UTranslate::dx(x)), &self.skin);
            }
        }
    }

    fn height_for_notes(&self, sb: &dyn StringBounder, position: Position) -> f64 {
        self.notes
            .iter()
            .filter(|n| n.position == position)
            .fold(0.0, |h, n| f64::max(h, n.height(sb, &self.skin)))
    }

    fn context(&self) -> Fashion {
        let line = self.style.value(PName::LineColor).as_color();
        let back = self.style.value(PName::BackGroundColor).as_color();
        Fashion::new(Some(back), Some(line)).with_stroke(self.style.stroke())
    }

    fn font_configuration(&self) -> FontConfiguration {
        self.style.font_configuration(None)
    }

    fn text_block(&self, value: &str) -> Rc<dyn TextBlock> {
        Display::with_newlines(Some(value)).create_simple_block(
            &self.font_configuration(),
            HorizontalAlignment::Left,
            &self.skin.skin_simple(),
        )
    }

    fn draw_hline_from_point(&self, ug: &UGraphic, start: (f64, f64), length: f64) {
        ug.apply_translate(UTranslate::new(start.0, start.1))
            .draw(ULine::hline(length));
    }

    fn draw_horizontal_between_times(&self, ug: &UGraphic, start: f64, end: f64) {
        let x1 = self.x_of_time(start);
        let x2 = self.ruler.width().min(self.x_of_time(end));
        ug.apply_translate(UTranslate::dx(x1))
            .draw(ULine::hline(x2 - x1));
    }

    fn x_of_time(&self, time: f64) -> f64 {
        self.ruler.pos_in_pixel_f64(time)
    }

    fn draw_vline(&self, ug: &UGraphic, x: f64, y: f64, vline: &ULine) {
        ug.apply_translate(UTranslate::new(x, y))
            .draw(vline.clone());
    }
}

/// The left and right panels of one player (`Panels`).
pub trait Panels {
    fn base(&self) -> &PanelsBase;
    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64);
    fn draw_right_panel(&self, ug: &UGraphic);
    fn full_height(&self, sb: &dyn StringBounder) -> f64;
    fn left_panel_width(&self, sb: &dyn StringBounder) -> f64;
    fn time_projection(&self, sb: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint>;

    fn constraint_delta_y(&self, _constraint: &TimeConstraint) -> f64 {
        0.0
    }

    fn height_for_constraints(&self, sb: &dyn StringBounder) -> f64 {
        self.base_height_for_constraints(sb)
    }

    fn base_height_for_constraints(&self, sb: &dyn StringBounder) -> f64 {
        let b = self.base();
        if b.constraints.is_empty() {
            return 0.0;
        }
        b.constraints.iter().fold(0.0, |r, c| {
            f64::max(
                r,
                c.constraint_height(sb, &b.skin) - self.constraint_delta_y(c),
            )
        })
    }

    fn draw_constraints(&self, ug: &UGraphic) {
        let b = self.base();
        for c in &b.constraints {
            c.draw(
                &ug.apply_translate(UTranslate::dy(self.constraint_delta_y(c))),
                &b.ruler,
                &b.skin,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// Clock

pub struct PanelsClock {
    pub base: PanelsBase,
    pub period: f64,
    pub pulse: f64,
    pub offset: f64,
}

impl PanelsClock {
    fn line_height(&self) -> f64 {
        self.base.suggested_height - 2.0 * MARGIN_Y
    }

    fn y_high(&self) -> f64 {
        MARGIN_Y
    }

    fn y_low(&self) -> f64 {
        MARGIN_Y + self.line_height()
    }

    fn is_high_at(&self, time: f64) -> bool {
        if self.period <= 0.0 {
            return false;
        }
        if time < self.offset {
            return false;
        }
        let vpulse = if self.pulse == 0.0 {
            self.period / 2.0
        } else {
            self.pulse
        };
        let phase = (time - self.offset) % self.period;
        let normalized = if phase < 0.0 {
            phase + self.period
        } else {
            phase
        };
        normalized < vpulse
    }
}

impl Panels for PanelsClock {
    fn base(&self) -> &PanelsBase {
        &self.base
    }

    fn draw_left_panel(&self, _: &UGraphic, _: f64) {}

    fn left_panel_width(&self, _: &dyn StringBounder) -> f64 {
        LEFT_PANEL_MIN_WIDTH
    }

    fn full_height(&self, _: &dyn StringBounder) -> f64 {
        self.base.suggested_height
    }

    fn time_projection(&self, _: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint> {
        let x = self.base.ruler.pos_in_pixel(tick);
        let y = if self.is_high_at(tick.time().to_f64()) {
            self.y_high()
        } else {
            self.y_low()
        };
        Some(IntricatedPoint {
            a: (x, y),
            b: (x, y),
        })
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        let b = &self.base;
        let ug = b.context().apply(ug);
        let vline = ULine::vline(self.line_height());
        let ug = ug.apply_translate(UTranslate::dy(MARGIN_Y));
        let mut value = 0.0;
        if self.offset != 0.0 {
            b.draw_horizontal_between_times(
                &ug.apply_translate(UTranslate::dy(vline.dy)),
                value,
                self.offset,
            );
            value += self.offset;
        }
        if b.x_of_time(value) > b.ruler.width() {
            return;
        }
        b.draw_vline(&ug, b.x_of_time(value), 0.0, &vline);
        let vpulse = if self.pulse == 0.0 {
            self.period / 2.0
        } else {
            self.pulse
        };
        let remain = self.period - vpulse;
        for _ in 0..1000 {
            b.draw_horizontal_between_times(&ug, value, value + vpulse);
            value += vpulse;
            if b.x_of_time(value) > b.ruler.width() {
                return;
            }
            b.draw_vline(&ug, b.x_of_time(value), 0.0, &vline);
            b.draw_horizontal_between_times(
                &ug.apply_translate(UTranslate::dy(vline.dy)),
                value,
                value + remain,
            );
            value += remain;
            if b.x_of_time(value) > b.ruler.width() {
                return;
            }
            b.draw_vline(&ug, b.x_of_time(value), 0.0, &vline);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Binary

pub struct PanelsBinary {
    pub base: PanelsBase,
    pub values: Vec<ChangeState>,
    pub initial_state: Option<ChangeState>,
}

const LOW: &str = "0";

impl PanelsBinary {
    fn y_pos(&self, sb: &dyn StringBounder, state: &str) -> f64 {
        if state.eq_ignore_ascii_case(LOW) {
            self.y_low(sb)
        } else {
            self.y_high(sb)
        }
    }

    fn y_high(&self, sb: &dyn StringBounder) -> f64 {
        MARGIN_Y + self.height_for_constraints(sb) + self.base.height_for_notes(sb, Position::Top)
    }

    fn y_low(&self, sb: &dyn StringBounder) -> f64 {
        self.height_for_constraints(sb)
            + self.base.height_for_notes(sb, Position::Top)
            + self.base.suggested_height
            - MARGIN_Y
    }
}

impl Panels for PanelsBinary {
    fn base(&self) -> &PanelsBase {
        &self.base
    }

    fn draw_left_panel(&self, _: &UGraphic, _: f64) {}

    fn left_panel_width(&self, _: &dyn StringBounder) -> f64 {
        LEFT_PANEL_MIN_WIDTH
    }

    fn full_height(&self, sb: &dyn StringBounder) -> f64 {
        self.height_for_constraints(sb)
            + self.base.height_for_notes(sb, Position::Top)
            + self.base.suggested_height
            + self.base.height_for_notes(sb, Position::Bottom)
    }

    fn time_projection(&self, sb: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint> {
        let x = self.base.ruler.pos_in_pixel(tick);
        let y = self.y_pos(sb, "1");
        Some(IntricatedPoint {
            a: (x, y),
            b: (x, y),
        })
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        let b = &self.base;
        let ug = b.context().apply(ug);
        let mut lastx = 0.0;
        let mut last_values: Vec<String> = match &self.initial_state {
            None => vec![LOW.to_string()],
            Some(s) => s.states.clone(),
        };
        let sb = ug.string_bounder();
        let yhigh = self.y_high(sb);
        let ylow = self.y_low(sb);
        let vline = ULine::vline(ylow - yhigh);
        for value in &self.values {
            let x = b.ruler.pos_in_pixel(&value.when);
            if last_values.len() == 1 {
                ug.apply_translate(UTranslate::new(lastx, self.y_pos(sb, &last_values[0])))
                    .draw(ULine::hline(x - lastx));
            } else {
                let mut tmpx = lastx;
                while tmpx < x {
                    ug.apply_translate(UTranslate::new(tmpx, yhigh))
                        .draw(vline.clone());
                    tmpx += 5.0;
                }
            }
            if last_values != value.states {
                ug.apply_translate(UTranslate::new(x, yhigh))
                    .draw(vline.clone());
            }
            if let Some(comment) = &value.comment {
                b.text_block(comment)
                    .draw(&ug.apply_translate(UTranslate::new(x + 2.0, yhigh)));
            }
            lastx = x;
            last_values = value.states.clone();
        }
        ug.apply_translate(UTranslate::new(lastx, self.y_pos(sb, &last_values[0])))
            .draw(ULine::hline(b.ruler.width() - lastx));
        self.draw_constraints(&ug.apply_translate(UTranslate::dy(self.height_for_constraints(sb))));
        b.draw_notes(&ug.apply_translate(UTranslate::dy(MARGIN_Y)), Position::Top);
        b.draw_notes(
            &ug.apply_translate(UTranslate::dy(
                self.height_for_constraints(sb)
                    + b.height_for_notes(sb, Position::Top)
                    + b.suggested_height
                    - MARGIN_Y / 2.0,
            )),
            Position::Bottom,
        );
    }
}

// ---------------------------------------------------------------------------------------
// Analog

pub struct PanelsAnalog {
    pub base: PanelsBase,
    pub series: TimeSeries,
    pub initial_state: Option<f64>,
    pub ticks_every: Option<i32>,
}

impl PanelsAnalog {
    fn y_pos(&self, sb: &dyn StringBounder, value: f64) -> f64 {
        let h = self.base.suggested_height;
        let y = (value - self.series.min()) * (h - 2.0 * MARGIN_Y)
            / (self.series.max() - self.series.min());
        self.height_for_constraints(sb) + h - MARGIN_Y - y
    }

    fn tick_range(&self) -> std::ops::RangeInclusive<i32> {
        (self.series.min().ceil() as i32)..=(self.series.max().floor() as i32)
    }

    fn label_block(&self, value: f64) -> Rc<dyn TextBlock> {
        self.base.text_block(&self.series.display_value(value))
    }

    fn width_label(&self, sb: &dyn StringBounder, value: f64) -> f64 {
        self.label_block(value).calculate_dimension(sb).width
    }

    fn max_width_for_ticks(&self, sb: &dyn StringBounder) -> f64 {
        match self.ticks_every {
            None => f64::max(
                self.width_label(sb, self.series.min()),
                self.width_label(sb, self.series.max()),
            ),
            Some(every) => {
                let mut result: f64 = 0.0;
                for i in self.tick_range() {
                    if every != 0 && i % every == 0 {
                        result = result.max(self.width_label(sb, f64::from(i)));
                    }
                }
                result
            }
        }
    }

    fn draw_scale_label(&self, ug: &UGraphic, value: f64, full_available_width: f64) {
        let label = self.label_block(value);
        let dim = label.calculate_dimension(ug.string_bounder());
        let ug = ug.apply_translate(UTranslate::dx(full_available_width - dim.width - 2.0));
        label.draw(&ug.apply_translate(UTranslate::dy(
            self.y_pos(ug.string_bounder(), value) - dim.height / 2.0,
        )));
    }

    fn draw_tick_hlines(&self, ug: &UGraphic, every: i32) {
        let ug = apply_for_vlines(ug, &self.base.style);
        let hline = ULine::hline(self.base.ruler.width());
        for i in self.tick_range() {
            if every != 0 && i % every == 0 {
                ug.apply_translate(UTranslate::dy(
                    self.y_pos(ug.string_bounder(), f64::from(i)),
                ))
                .draw(hline.clone());
            }
        }
    }
}

impl Panels for PanelsAnalog {
    fn base(&self) -> &PanelsBase {
        &self.base
    }

    fn full_height(&self, sb: &dyn StringBounder) -> f64 {
        self.height_for_constraints(sb) + self.base.suggested_height
    }

    fn time_projection(&self, sb: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint> {
        let x = self.base.ruler.pos_in_pixel(tick);
        let value = self.series.value_at(tick);
        let y = self.y_pos(sb, value);
        Some(IntricatedPoint {
            a: (x, y),
            b: (x, y),
        })
    }

    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64) {
        match self.ticks_every {
            None => {
                self.draw_scale_label(ug, self.series.min(), full_available_width);
                self.draw_scale_label(ug, self.series.max(), full_available_width);
            }
            Some(every) => {
                for i in self.tick_range() {
                    if every != 0 && i % every == 0 {
                        self.draw_scale_label(ug, f64::from(i), full_available_width);
                    }
                }
            }
        }
    }

    fn left_panel_width(&self, sb: &dyn StringBounder) -> f64 {
        LEFT_PANEL_MIN_WIDTH + self.max_width_for_ticks(sb)
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        if let Some(every) = self.ticks_every {
            self.draw_tick_hlines(ug, every);
        }
        let ug = self.base.context().apply(ug);
        let sb = ug.string_bounder();
        let mut lastx = 0.0;
        let mut last_value = self.initial_state.unwrap_or(0.0);
        for (tick, value) in self.series.entries() {
            let y1 = self.y_pos(sb, last_value);
            let y2 = self.y_pos(sb, *value);
            let x = self.base.ruler.pos_in_pixel(tick);
            ug.apply_translate(UTranslate::new(lastx, y1))
                .draw(ULine::new(x - lastx, y2 - y1));
            lastx = x;
            last_value = *value;
        }
        ug.apply_translate(UTranslate::new(lastx, self.y_pos(sb, last_value)))
            .draw(ULine::hline(self.base.ruler.width() - lastx));
        self.draw_constraints(&ug.apply_translate(UTranslate::dy(self.height_for_constraints(sb))));
    }
}

// ---------------------------------------------------------------------------------------
// Robust

const HISTOGRAM_BOTTOM_MARGIN: f64 = 12.0;
const HISTOGRAM_INITIAL_WIDTH: f64 = 40.0;

pub struct PanelsRobust {
    pub base: PanelsBase,
    changes: Vec<ChangeState>,
    all_states: Vec<String>,
    initial_state: Option<String>,
}

impl PanelsRobust {
    pub fn new(base: PanelsBase, some_states: Vec<String>) -> Self {
        let mut all_states = some_states;
        all_states.reverse();
        Self {
            base,
            changes: Vec::new(),
            all_states,
            initial_state: None,
        }
    }

    pub fn set_initial_state(&mut self, initial_state: Option<String>) {
        if let Some(s) = &initial_state {
            if !self.all_states.contains(s) {
                self.all_states.push(s.clone());
            }
        }
        self.initial_state = initial_state;
    }

    pub fn add_change(&mut self, change: ChangeState) {
        let hidden = change.is_completely_hidden();
        let states = change.states.clone();
        self.changes.push(change);
        if hidden {
            return;
        }
        for state in states {
            if !self.all_states.contains(&state) {
                self.all_states.push(state);
            }
        }
    }

    fn states_at(&self, tick: &TimeTick) -> Vec<String> {
        if self.changes.is_empty() {
            return Vec::new();
        }
        for (i, c) in self.changes.iter().enumerate() {
            match c.when.compare(tick) {
                Ordering::Equal => {
                    if i == 0 {
                        return match &self.initial_state {
                            None => vec![c.state().to_string()],
                            Some(init) => vec![init.clone(), c.state().to_string()],
                        };
                    }
                    return vec![
                        self.changes[i - 1].state().to_string(),
                        c.state().to_string(),
                    ];
                }
                Ordering::Greater => {
                    let idx = if i == 0 { 0 } else { i - 1 };
                    return vec![self.changes[idx].state().to_string()];
                }
                Ordering::Less => {}
            }
        }
        vec![self.changes[self.changes.len() - 1].state().to_string()]
    }

    fn points(&self, n: usize) -> Vec<(f64, f64)> {
        let change = &self.changes[n];
        let x = self.base.ruler.pos_in_pixel(&change.when);
        if change.states.len() == 2 {
            vec![
                (x, self.y_of_state(&change.states[0])),
                (x, self.y_of_state(&change.states[1])),
            ]
        } else {
            vec![(x, self.y_of_state(&change.states[0]))]
        }
    }

    fn pointx(&self, n: usize) -> f64 {
        self.base.ruler.pos_in_pixel(&self.changes[n].when)
    }

    fn point_min_y(&self, n: usize) -> f64 {
        let s = &self.changes[n].states;
        if s.len() == 2 {
            self.y_of_state(&s[0]).min(self.y_of_state(&s[1]))
        } else {
            self.y_of_state(&s[0])
        }
    }

    fn point_max_y(&self, n: usize) -> f64 {
        let s = &self.changes[n].states;
        if s.len() == 2 {
            self.y_of_state(&s[0]).max(self.y_of_state(&s[1]))
        } else {
            self.y_of_state(&s[0])
        }
    }

    fn y_of_state(&self, state: &str) -> f64 {
        let idx = self
            .all_states
            .iter()
            .position(|s| s == state)
            .map_or(-1, |i| i as i64);
        let nb = self.all_states.len() as i64 - 1 - idx;
        self.step_height() * nb as f64
    }

    fn y_of_tick(&self, when: &TimeTick) -> f64 {
        match self.states_at(when).last() {
            Some(s) => self.y_of_state(s),
            None => 0.0,
        }
    }

    fn step_height(&self) -> f64 {
        if self.base.suggested_height == 0.0 || self.all_states.len() <= 1 {
            return 20.0;
        }
        // integer division like Java
        (self.base.suggested_height as i64 / (self.all_states.len() as i64 - 1)) as f64
    }

    fn states_width(&self, sb: &dyn StringBounder) -> f64 {
        self.all_states.iter().fold(0.0, |r, s| {
            f64::max(r, self.base.text_block(s).calculate_dimension(sb).width)
        })
    }

    fn initial_point(&self) -> (f64, f64) {
        (
            -HISTOGRAM_INITIAL_WIDTH,
            self.y_of_state(self.initial_state.as_deref().unwrap_or("")),
        )
    }

    fn draw_hlines(&self, ug: &UGraphic) {
        let b = &self.base;
        if self.initial_state.is_some() {
            for pt in self.points(0) {
                b.draw_hline_from_point(ug, self.initial_point(), HISTOGRAM_INITIAL_WIDTH + pt.0);
            }
        }
        let n = self.changes.len();
        for i in 0..n {
            if self.changes[i].is_completely_hidden() {
                continue;
            }
            let x2 = if i < n - 1 {
                self.pointx(i + 1)
            } else {
                b.ruler.width()
            };
            let len = x2 - self.pointx(i);
            let points = self.points(i);
            if points.len() == 2 {
                self.draw_hblock(
                    &ug.apply_back_color(self.changes[i].back_color(&b.style)),
                    points[0],
                    points[1],
                    len,
                );
            }
            if i < n - 1 {
                for pt in &points {
                    b.draw_hline_from_point(ug, *pt, len);
                }
            }
        }
        if !self.changes[n - 1].is_completely_hidden() {
            for pt in self.points(n - 1) {
                b.draw_hline_from_point(ug, pt, b.ruler.width() - pt.0);
            }
        }
    }

    fn draw_hblock(&self, ug: &UGraphic, pt1: (f64, f64), pt2: (f64, f64), len: f64) {
        let min_y = pt1.1.min(pt2.1);
        let max_y = pt1.1.max(pt2.1);
        let ug = ug.apply_translate(UTranslate::new(pt1.0, min_y));
        ug.draw(URectangle::build(len, max_y - min_y));
        let mut x = 0.0;
        while x < len {
            ug.apply_translate(UTranslate::dx(x))
                .draw(ULine::vline(max_y - min_y));
            x += 5.0;
        }
    }

    fn draw_vlines(&self, ug: &UGraphic) {
        let b = &self.base;
        if self.initial_state.is_some() {
            let before = self.initial_point();
            let current = self.points(0)[0];
            b.draw_vline(
                ug,
                current.0,
                current.1,
                &ULine::vline(before.1 - current.1),
            );
        }
        for i in 1..self.changes.len() {
            if self.changes[i - 1].is_completely_hidden() || self.changes[i].is_completely_hidden()
            {
                continue;
            }
            let min_y = self.point_min_y(i).min(self.point_min_y(i - 1));
            let max_y = self.point_max_y(i).max(self.point_max_y(i - 1));
            b.draw_vline(ug, self.pointx(i), min_y, &ULine::vline(max_y - min_y));
        }
    }

    fn draw_labels(&self, ug: &UGraphic) {
        for i in 0..self.changes.len() {
            let pt = self.points(i)[0];
            let Some(comment) = &self.changes[i].comment else {
                continue;
            };
            let label = self.base.text_block(comment);
            let dim = label.calculate_dimension(ug.string_bounder());
            label.draw(&ug.apply_translate(UTranslate::new(pt.0 + 2.0, pt.1 - dim.height)));
        }
    }
}

impl Panels for PanelsRobust {
    fn base(&self) -> &PanelsBase {
        &self.base
    }

    fn time_projection(&self, sb: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint> {
        let x = self.base.ruler.pos_in_pixel(tick);
        let states = self.states_at(tick);
        if states.is_empty() {
            return None;
        }
        let hfc = self.height_for_constraints(sb);
        if states.len() == 1 {
            let y = self.y_of_state(&states[0]) + hfc;
            return Some(IntricatedPoint {
                a: (x, y),
                b: (x, y),
            });
        }
        let y1 = self.y_of_state(&states[0]) + hfc;
        let y2 = self.y_of_state(&states[1]) + hfc;
        Some(IntricatedPoint {
            a: (x, y1),
            b: (x, y2),
        })
    }

    fn draw_left_panel(&self, ug: &UGraphic, full_available_width: f64) {
        let sb = ug.string_bounder();
        let ug = ug.apply_translate(UTranslate::dy(self.height_for_constraints(sb)));
        let mut width = self.states_width(sb);
        if self.initial_state.is_some() {
            width += HISTOGRAM_INITIAL_WIDTH;
        }
        let ug = if full_available_width > width + 5.0 {
            ug.apply_translate(UTranslate::dx(full_available_width - width - 5.0))
        } else {
            ug.apply_translate(UTranslate::dx(full_available_width - width))
        };
        for state in &self.all_states {
            let label = self.base.text_block(state);
            let dim = label.calculate_dimension(sb);
            label.draw(&ug.apply_translate(UTranslate::dy(
                self.y_of_state(state) - dim.height / 2.0 + 1.0,
            )));
        }
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        if self.changes.is_empty() {
            return;
        }
        let ug = self.base.context().apply(ug);
        let ug = ug.apply_translate(UTranslate::dy(
            self.height_for_constraints(ug.string_bounder()),
        ));
        self.draw_hlines(&ug);
        self.draw_vlines(&ug);
        self.draw_labels(&ug);
        self.draw_constraints(&ug.apply_translate(UTranslate::dy(-TimeConstraint::top_margin())));
    }

    fn left_panel_width(&self, sb: &dyn StringBounder) -> f64 {
        if self.initial_state.is_none() {
            return self.states_width(sb);
        }
        self.states_width(sb) + HISTOGRAM_INITIAL_WIDTH
    }

    fn constraint_delta_y(&self, constraint: &TimeConstraint) -> f64 {
        let mut y = self.y_of_tick(&constraint.tick1);
        for change in &self.changes {
            if constraint.contains_strict(&change.when) {
                y = y.min(self.y_of_tick(&change.when));
            }
        }
        y
    }

    fn height_for_constraints(&self, sb: &dyn StringBounder) -> f64 {
        f64::max(10.0, self.base_height_for_constraints(sb))
    }

    fn full_height(&self, sb: &dyn StringBounder) -> f64 {
        let mut height = self.height_for_constraints(sb);
        if !self.all_states.is_empty() {
            height += self.step_height() * (self.all_states.len() as f64 - 1.0);
        }
        height += HISTOGRAM_BOTTOM_MARGIN;
        height + 6.0
    }
}

// ---------------------------------------------------------------------------------------
// Concise and rectangle

const DEFAULT_RIBBON_HEIGHT: f64 = 24.0;
const SHAPE_DELTA: f64 = 12.0;

pub struct PanelsState {
    pub base: PanelsBase,
    pub shape: StateKind,
    changes: Vec<ChangeState>,
    initial_state: Option<String>,
    initial_colors: Option<Colors>,
}

impl PanelsState {
    pub fn new(mut base: PanelsBase, shape: StateKind) -> Self {
        if base.suggested_height == 0.0 {
            base.suggested_height = DEFAULT_RIBBON_HEIGHT;
        }
        Self {
            base,
            shape,
            changes: Vec::new(),
            initial_state: None,
            initial_colors: None,
        }
    }

    pub fn add_change(&mut self, change: ChangeState) {
        self.changes.push(change);
    }

    pub fn set_initial_state(&mut self, state: Option<String>, colors: Option<Colors>) {
        self.initial_state = state;
        self.initial_colors = colors;
    }

    fn ribbon_height(&self) -> f64 {
        self.base.suggested_height
    }

    fn pos_in_pixel(&self, change: &ChangeState) -> f64 {
        self.base.ruler.pos_in_pixel(&change.when)
    }

    fn initial_width(&self, sb: &dyn StringBounder) -> f64 {
        match &self.initial_state {
            None => 0.0,
            Some(s) => self.base.text_block(s).calculate_dimension(sb).width + 2.0 * MARGIN_X,
        }
    }

    fn comment_top_block(&self, change: &ChangeState) -> Rc<dyn TextBlock> {
        match &change.comment {
            None => Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            }),
            Some(c) => self.base.text_block(c),
        }
    }

    fn height_for_top_comment(&self, sb: &dyn StringBounder) -> f64 {
        self.changes.iter().fold(0.0, |r, c| {
            f64::max(r, self.comment_top_block(c).calculate_dimension(sb).height)
        })
    }

    fn context_with_initial_colors(&self, change: &ChangeState) -> Fashion {
        let mut context = change.context(&self.base.style);
        if let Some(colors) = &self.initial_colors {
            if let Some(back) = colors.color(ColorType::Back) {
                context = context.with_back_color(Some(back.clone()));
            }
            if let Some(line) = colors.color(ColorType::Line) {
                context = context.with_fore_color(Some(line.clone()));
            }
        }
        context
    }

    fn draw_flat(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        change
            .context(&self.base.style)
            .apply(ug)
            .apply_translate(UTranslate::dy(self.ribbon_height() / 2.0))
            .draw(ULine::hline(len));
    }

    fn draw_hexa(&self, ug: &UGraphic, len: f64, change: &ChangeState) {
        let h = self.ribbon_height();
        let context = change.context(&self.base.style);
        match self.shape {
            StateKind::Rectangle => context.apply(ug).draw(URectangle::build(len, h)),
            _ => {
                let poly = UPolygon::new(vec![
                    (SHAPE_DELTA, 0.0),
                    (len - SHAPE_DELTA, 0.0),
                    (len, h / 2.0),
                    (len - SHAPE_DELTA, h),
                    (SHAPE_DELTA, h),
                    (0.0, h / 2.0),
                ]);
                context.apply(ug).draw(poly);
            }
        }
    }

    fn fill_then_outline(
        &self,
        ug: &UGraphic,
        context: &Fashion,
        fill: impl Into<crate::klimt::drawing::ugraphic::UShape>,
        outline: &[(f64, f64)],
    ) {
        context
            .clone()
            .with_fore_color(context.back_color().cloned())
            .apply(ug)
            .draw(fill);
        context.apply(ug).draw(path(outline));
    }

    fn draw_penta_a(&self, ug: &UGraphic, w: f64, change: &ChangeState) {
        let h = self.ribbon_height();
        let context = self.context_with_initial_colors(change);
        if self.shape == StateKind::Rectangle {
            self.fill_then_outline(
                ug,
                &context,
                URectangle::build(w, h),
                &[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)],
            );
        } else {
            let outline = [
                (0.0, 0.0),
                (w - SHAPE_DELTA, 0.0),
                (w, h / 2.0),
                (w - SHAPE_DELTA, h),
                (0.0, h),
            ];
            self.fill_then_outline(ug, &context, UPolygon::new(outline.to_vec()), &outline);
        }
    }

    fn draw_penta_b(&self, ug: &UGraphic, w: f64, change: &ChangeState) {
        let h = self.ribbon_height();
        let context = change.context(&self.base.style);
        if self.shape == StateKind::Rectangle {
            self.fill_then_outline(
                ug,
                &context,
                URectangle::build(w, h),
                &[(w, 0.0), (0.0, 0.0), (0.0, h), (w, h)],
            );
        } else {
            let fill = [
                (SHAPE_DELTA, 0.0),
                (w, 0.0),
                (w, h),
                (SHAPE_DELTA, h),
                (0.0, h / 2.0),
            ];
            let outline = [
                (w, 0.0),
                (SHAPE_DELTA, 0.0),
                (0.0, h / 2.0),
                (SHAPE_DELTA, h),
                (w, h),
            ];
            self.fill_then_outline(ug, &context, UPolygon::new(fill.to_vec()), &outline);
        }
    }

    fn draw_before_zero_state(&self, ug: &UGraphic) {
        let Some(initial) = &self.initial_state else {
            return;
        };
        let sb = ug.string_bounder();
        let iw = self.initial_width(sb);
        if self.changes.is_empty() {
            self.draw_single(
                &ug.apply_translate(UTranslate::dx(-iw)),
                iw + self.base.ruler.width(),
            );
        } else {
            let a = self.pos_in_pixel(&self.changes[0]);
            if is_flat(initial) {
                self.draw_flat(
                    &ug.apply_translate(UTranslate::dx(-iw)),
                    iw + a,
                    &self.changes[0],
                );
            } else {
                self.draw_penta_a(
                    &ug.apply_translate(UTranslate::dx(-iw)),
                    iw + a,
                    &self.changes[0],
                );
            }
        }
    }

    fn draw_before_zero_state_label(&self, ug: &UGraphic) {
        let Some(initial) = &self.initial_state else {
            return;
        };
        if is_flat(initial) {
            return;
        }
        let block = self.base.text_block(initial);
        let dim = block.calculate_dimension(ug.string_bounder());
        block.draw(&ug.apply_translate(UTranslate::new(-MARGIN_X - dim.width, -dim.height / 2.0)));
    }

    fn draw_single(&self, ug: &UGraphic, len: f64) {
        let Some(initial) = &self.initial_state else {
            return;
        };
        let st = &self.base.style;
        let back = st.value(PName::BackGroundColor).as_color();
        let line = st.value(PName::LineColor).as_color();
        let context = Fashion::new(Some(back.clone()), Some(back)).with_stroke(st.stroke());
        let ug = context.apply(ug);
        let border = ULine::hline(len);
        let height = self.ribbon_height();
        if is_flat(initial) {
            ug.apply_color(line)
                .apply_translate(UTranslate::dy(height / 2.0))
                .draw(border);
            return;
        }
        ug.draw(URectangle::build(len, height));
        let ug = ug.apply_color(line);
        ug.draw(border.clone());
        ug.apply_translate(UTranslate::dy(height)).draw(border);
    }

    fn draw_states(&self, ug: &UGraphic) {
        let n = self.changes.len();
        for i in 0..n.saturating_sub(1) {
            let a = self.pos_in_pixel(&self.changes[i]);
            let b = self.pos_in_pixel(&self.changes[i + 1]);
            let ug_a = ug.apply_translate(UTranslate::dx(a));
            if self.changes[i].is_flat() {
                self.draw_flat(&ug_a, b - a, &self.changes[i]);
            } else if !self.changes[i].is_completely_hidden() {
                self.draw_hexa(&ug_a, b - a, &self.changes[i]);
            }
        }
        if let Some(last) = self.changes.last() {
            let a = self.pos_in_pixel(last);
            let ug_a = ug.apply_translate(UTranslate::dx(a));
            if last.is_flat() {
                self.draw_flat(&ug_a, self.base.ruler.width() - a, last);
            } else if !last.is_completely_hidden() {
                self.draw_penta_b(&ug_a, self.base.ruler.width() - a, last);
            }
        }
    }

    fn draw_states_labels(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let n = self.changes.len();
        for (i, change) in self.changes.iter().enumerate() {
            let x = self.base.ruler.pos_in_pixel(&change.when);
            if !change.is_blank() && !change.is_completely_hidden() && !change.is_flat() {
                let state = self.base.text_block(change.state());
                let dim = state.calculate_dimension(sb);
                let xtext = if i == n - 1 {
                    x + MARGIN_X
                } else {
                    let x2 = self.base.ruler.pos_in_pixel(&self.changes[i + 1].when);
                    (x + x2) / 2.0 - dim.width / 2.0
                };
                state.draw(&ug.apply_translate(UTranslate::new(xtext, -dim.height / 2.0)));
            }
            let comment = self.comment_top_block(change);
            let dim = comment.calculate_dimension(sb);
            comment.draw(&ug.apply_translate(UTranslate::new(
                x + MARGIN_X,
                -self.ribbon_height() / 2.0 - dim.height,
            )));
        }
    }
}

impl Panels for PanelsState {
    fn base(&self) -> &PanelsBase {
        &self.base
    }

    fn time_projection(&self, sb: &dyn StringBounder, tick: &TimeTick) -> Option<IntricatedPoint> {
        let x = self.base.ruler.pos_in_pixel(tick);
        let y = self.height_for_constraints(sb)
            + self.base.height_for_notes(sb, Position::Top)
            + self.height_for_top_comment(sb)
            + self.ribbon_height() / 2.0;
        for change in &self.changes {
            if change.when.compare(tick) == Ordering::Equal {
                return Some(IntricatedPoint {
                    a: (x, y),
                    b: (x, y),
                });
            }
        }
        Some(IntricatedPoint {
            a: (x, y - self.ribbon_height() / 2.0),
            b: (x, y + self.ribbon_height() / 2.0),
        })
    }

    fn draw_left_panel(&self, _: &UGraphic, _: f64) {}

    fn left_panel_width(&self, sb: &dyn StringBounder) -> f64 {
        self.initial_width(sb)
    }

    fn draw_right_panel(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let b = &self.base;
        let ug_ribbon = ug.apply_translate(UTranslate::dy(
            self.height_for_constraints(sb)
                + self.height_for_top_comment(sb)
                + b.height_for_notes(sb, Position::Top),
        ));
        self.draw_before_zero_state(&ug_ribbon);
        self.draw_before_zero_state_label(
            &ug_ribbon.apply_translate(UTranslate::dy(self.ribbon_height() / 2.0)),
        );
        self.draw_states(&ug_ribbon);
        self.draw_states_labels(
            &ug_ribbon.apply_translate(UTranslate::dy(self.ribbon_height() / 2.0)),
        );
        self.draw_constraints(
            &ug.apply_translate(UTranslate::dy(self.height_for_constraints(sb) / 2.0)),
        );
        b.draw_notes(ug, Position::Top);
        b.draw_notes(
            &ug.apply_translate(UTranslate::dy(
                self.height_for_constraints(sb)
                    + self.ribbon_height()
                    + b.height_for_notes(sb, Position::Top),
            )),
            Position::Bottom,
        );
    }

    fn height_for_constraints(&self, sb: &dyn StringBounder) -> f64 {
        f64::max(5.0, self.base_height_for_constraints(sb))
    }

    fn full_height(&self, sb: &dyn StringBounder) -> f64 {
        self.height_for_constraints(sb)
            + self.height_for_top_comment(sb)
            + self.base.height_for_notes(sb, Position::Top)
            + self.ribbon_height()
            + self.base.height_for_notes(sb, Position::Bottom)
            + BOTTOM_MARGIN
    }
}

// ---------------------------------------------------------------------------------------
// Frame and arrows

/// The title of a player and the separator under it (`PlayerFrame`).
pub struct PlayerFrame {
    title: Option<Rc<dyn TextBlock>>,
    compact: bool,
    line_color: HColor,
    stroke: UStroke,
}

impl PlayerFrame {
    pub fn new(title: &str, skin: &SkinParam, compact: bool) -> Self {
        let display = Display::with_newlines(Some(title));
        let style = skin
            .style_builder()
            .merged_style(&queries::TIMINGDIAG)
            .expect("timing diagram style");
        let block = (!display.is_white()).then(|| {
            display.create_simple_block(
                &style.font_configuration(None),
                HorizontalAlignment::Left,
                &skin.skin_simple(),
            )
        });
        Self {
            title: block,
            compact,
            line_color: style.value(PName::LineColor).as_color(),
            stroke: style.stroke(),
        }
    }

    pub fn draw_frame_title(&self, ug: &UGraphic) {
        let Some(title) = &self.title else { return };
        title.draw(ug);
        if self.compact {
            return;
        }
        let ug = ug
            .apply_color(self.line_color.clone())
            .apply_stroke(self.stroke);
        let dim = title.calculate_dimension(ug.string_bounder());
        let width = dim.width + 1.0;
        let height = dim.height + 1.0;
        let coords = [-MARGIN_X1, height, width, height, width + 10.0, 0.0];
        let mut i = 0;
        while i + 2 < coords.len() {
            let (x1, y1, x2, y2) = (coords[i], coords[i + 1], coords[i + 2], coords[i + 3]);
            ug.apply_translate(UTranslate::new(x1, y1))
                .draw(ULine::new(x2 - x1, y2 - y1));
            i += 2;
        }
    }

    pub fn height(&self, sb: &dyn StringBounder) -> f64 {
        let height = self
            .title
            .as_ref()
            .map_or(0.0, |t| t.calculate_dimension(sb).height);
        if self.compact {
            height - 1.0
        } else {
            height + 1.0
        }
    }
}

/// The arrow of a message between two players (`TimeArrow`).
pub struct TimeArrow {
    start: (f64, f64),
    end: (f64, f64),
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

impl TimeArrow {
    /// The shortest of the four arrows between the ends of the two points.
    pub fn create(pt1: IntricatedPoint, pt2: IntricatedPoint) -> TimeArrow {
        let a1 = TimeArrow {
            start: pt1.a,
            end: pt2.a,
        };
        let a2 = TimeArrow {
            start: pt1.a,
            end: pt2.b,
        };
        let a3 = TimeArrow {
            start: pt1.b,
            end: pt2.a,
        };
        let a4 = TimeArrow {
            start: pt1.b,
            end: pt2.b,
        };
        let shorter = |x: TimeArrow, y: TimeArrow| {
            if distance(x.start, x.end) < distance(y.start, y.end) {
                x
            } else {
                y
            }
        };
        shorter(shorter(a1, a2), shorter(a3, a4))
    }

    fn angle(&self) -> f64 {
        (self.end.0 - self.start.0).atan2(self.end.1 - self.start.1)
    }

    fn on_circle(pt: (f64, f64), alpha: f64) -> (f64, f64) {
        let radius = 8.0;
        (pt.0 - alpha.sin() * radius, pt.1 - alpha.cos() * radius)
    }

    pub fn draw(&self, ug: &UGraphic, message: &TimeMessage, skin: &SkinParam) {
        let style = skin
            .style_builder()
            .merged_style(&queries::TIMINGDIAG_ARROW)
            .expect("timing diagram style");
        let color = message
            .specific_color
            .clone()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let angle = self.angle();
        let ug = ug.apply_color(color.clone()).apply_stroke(style.stroke());
        ug.apply_translate(UTranslate::new(self.start.0, self.start.1))
            .draw(ULine::new(
                self.end.0 - self.start.0,
                self.end.1 - self.start.1,
            ));
        let delta = 20.0 * std::f64::consts::PI / 180.0;
        let pt1 = Self::on_circle(self.end, angle + delta);
        let pt2 = Self::on_circle(self.end, angle - delta);
        let polygon = UPolygon::new(vec![pt1, pt2, self.end]);
        let ug = ug.apply_back_color(color);
        ug.draw(polygon);
        let text = message.label.create_simple_block(
            &style.font_configuration(None),
            HorizontalAlignment::Left,
            &skin.skin_simple(),
        );
        let x = (pt1.0 + pt2.0) / 2.0;
        let mut y = (pt1.1 + pt2.1) / 2.0;
        if self.start.1 < self.end.1 {
            y -= text.calculate_dimension(ug.string_bounder()).height;
        }
        text.draw(&ug.apply_translate(UTranslate::new(x, y)));
    }
}
