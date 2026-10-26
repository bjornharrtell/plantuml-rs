//! Data of a timing diagram: players, state changes, constraints, notes, highlights and
//! messages (`Player*`, `ChangeState`, `TimeConstraint`, `TimingNote`, `Highlight`,
//! `TimeMessage`, `TimeSeries`).

use super::ruler::TimingRuler;
use super::tick::{java_double, DeduceFormat, TimeTick};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPath, UPolygon, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::{HorizontalAlignment, Position};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, PName, SName, Style};
use std::cmp::Ordering;
use std::rc::Rc;

fn style_of(skin: &SkinParam, q: &crate::style::StyleQuery) -> Style {
    skin.style_builder()
        .merged_style(q)
        .expect("timing diagram style")
}

/// A change of state of a state player (`ChangeState`).
#[derive(Debug, Clone)]
pub struct ChangeState {
    pub when: TimeTick,
    pub comment: Option<String>,
    pub colors: Option<Colors>,
    pub states: Vec<String>,
}

impl ChangeState {
    pub fn state(&self) -> &str {
        &self.states[0]
    }

    pub fn back_color(&self, style: &Style) -> HColor {
        match self.colors.as_ref().and_then(|c| c.color(ColorType::Back)) {
            Some(c) => c.clone(),
            None => style.value(PName::BackGroundColor).as_color(),
        }
    }

    fn line_color(&self, style: &Style) -> HColor {
        match self.colors.as_ref().and_then(|c| c.color(ColorType::Line)) {
            Some(c) => c.clone(),
            None => style.value(PName::LineColor).as_color(),
        }
    }

    pub fn context(&self, style: &Style) -> Fashion {
        Fashion::new(Some(self.back_color(style)), Some(self.line_color(style)))
            .with_stroke(style.stroke())
    }

    pub fn is_blank(&self) -> bool {
        self.states[0] == "{...}"
    }

    pub fn is_completely_hidden(&self) -> bool {
        self.states[0] == "{hidden}"
    }

    pub fn is_flat(&self) -> bool {
        is_flat(&self.states[0])
    }
}

pub fn is_flat(state: &str) -> bool {
    state == "{-}"
}

/// A constraint arrow between two times (`TimeConstraint`).
#[derive(Debug, Clone)]
pub struct TimeConstraint {
    pub marginx: f64,
    pub tick1: TimeTick,
    pub tick2: TimeTick,
    label: Display,
    color: Option<HColor>,
}

impl TimeConstraint {
    pub fn new(
        marginx: f64,
        tick1: TimeTick,
        tick2: TimeTick,
        label: Option<&str>,
        color: Option<HColor>,
    ) -> Self {
        Self {
            marginx,
            tick1,
            tick2,
            label: Display::with_newlines(label),
            color,
        }
    }

    pub fn contains_strict(&self, other: &TimeTick) -> bool {
        self.tick1.compare(other) == Ordering::Less
            && self.tick2.compare(other) == Ordering::Greater
    }

    fn style(skin: &SkinParam) -> Style {
        style_of(skin, &queries::TIMINGDIAG_CONSTRAINTARROW)
    }

    fn text_block(&self, skin: &SkinParam) -> Rc<dyn TextBlock> {
        self.label.create_simple_block(
            &Self::style(skin).font_configuration(None),
            HorizontalAlignment::Left,
            &skin.skin_simple(),
        )
    }

    pub fn constraint_height(&self, sb: &dyn StringBounder, skin: &SkinParam) -> f64 {
        self.text_block(skin).calculate_dimension(sb).height + Self::top_margin()
    }

    pub fn top_margin() -> f64 {
        5.0
    }

    pub fn draw(&self, ug: &UGraphic, ruler: &TimingRuler, skin: &SkinParam) {
        let style = Self::style(skin);
        let arrow_color = self
            .color
            .clone()
            .unwrap_or_else(|| style.value(PName::LineColor).as_color());
        let ug = ug
            .apply_color(arrow_color.clone())
            .apply_back_color(arrow_color);
        let x1 = ruler.pos_in_pixel(&self.tick1) + self.marginx;
        let x2 = ruler.pos_in_pixel(&self.tick2) - self.marginx;
        let hline = |x1: f64, x2: f64| {
            ug.apply_translate(UTranslate::dx(x1))
                .apply_stroke(style.stroke())
                .draw(ULine::hline(x2 - x1));
        };
        let simple = ug.apply_stroke(UStroke::simple());
        if x2 - x1 > 20.0 {
            hline(x1 + 3.0, x2 - 3.0);
            simple.draw(polygon(false, (x1, 0.0)));
            simple.draw(polygon(true, (x2, 0.0)));
        } else {
            hline(x1 - 1.0, x2 + 1.0);
            simple.draw(polygon(true, (x1, 0.0)));
            simple.draw(polygon(false, (x2, 0.0)));
        }
        let text = self.text_block(skin);
        let dim = text.calculate_dimension(ug.string_bounder());
        let x = x1 + (x2 - x1 - dim.width) / 2.0;
        text.draw(&ug.apply_translate(UTranslate::new(
            x,
            -self.constraint_height(ug.string_bounder(), skin),
        )));
    }
}

/// Arrow head of a constraint; `right` points to the right.
fn polygon(right: bool, end: (f64, f64)) -> UPolygon {
    let (dx, dy) = (8.0, 4.0);
    let (pt1, pt2) = if right {
        ((end.0 - dx, end.1 + dy), (end.0 - dx, end.1 - dy))
    } else {
        ((end.0 + dx, end.1 + dy), (end.0 + dx, end.1 - dy))
    };
    UPolygon::new(vec![pt1, pt2, end])
}

/// A note above or below a player (`TimingNote`).
#[derive(Debug, Clone)]
pub struct TimingNote {
    pub when: Option<TimeTick>,
    pub note: Display,
    pub position: Position,
    pub stereotype: Option<Stereotype>,
}

const OPALE_MARGIN_X1: f64 = 6.0;
const OPALE_MARGIN_X2: f64 = 15.0;
const OPALE_MARGIN_Y: f64 = 5.0;

impl TimingNote {
    fn style(&self, skin: &SkinParam) -> Style {
        style_of(
            skin,
            &queries::TIMINGDIAG_NOTE.with_stereotype_obj(self.stereotype.as_ref()),
        )
    }

    fn text(&self, skin: &SkinParam, style: &Style) -> Rc<dyn TextBlock> {
        self.note.create_block(
            &style.font_configuration(None),
            skin.default_text_alignment(HorizontalAlignment::Left),
            &skin.skin_simple(),
            &crate::klimt::creole::LineBreakStrategy::NONE,
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        )
    }

    fn opale_dim(&self, sb: &dyn StringBounder, skin: &SkinParam) -> Dimension {
        let style = self.style(skin);
        let d = self.text(skin, &style).calculate_dimension(sb);
        Dimension::new(
            d.width + OPALE_MARGIN_X1 + OPALE_MARGIN_X2,
            d.height + 2.0 * OPALE_MARGIN_Y,
        )
    }

    fn margin_y() -> f64 {
        10.0
    }

    pub fn height(&self, sb: &dyn StringBounder, skin: &SkinParam) -> f64 {
        self.opale_dim(sb, skin).height + Self::margin_y()
    }

    pub fn draw(&self, ug: &UGraphic, skin: &SkinParam) {
        let ug = if self.position == Position::Bottom {
            ug.apply_translate(UTranslate::dy(Self::margin_y() / 2.0))
        } else {
            ug.clone()
        };
        let style = self.style(skin);
        let text = self.text(skin, &style);
        let dim = self.opale_dim(ug.string_bounder(), skin);
        let border = style.value(PName::LineColor).as_color();
        let back = style.value(PName::BackGroundColor).as_color();
        let ug = ug
            .apply_back_color(back)
            .apply_color(border)
            .apply_stroke(style.stroke());
        let mut poly = crate::skin::rose::note::polygon_normal(dim.width, dim.height, 0.0);
        poly.delta_shadow = style.shadowing();
        ug.draw(poly);
        ug.draw(crate::skin::rose::note::corner(dim.width, 0.0));
        text.draw(&ug.apply_translate(UTranslate::new(OPALE_MARGIN_X1, OPALE_MARGIN_Y)));
    }
}

/// Values of an analog player over time (`TimeSeries`).
#[derive(Debug, Clone, Default)]
pub struct TimeSeries {
    values: Vec<(TimeTick, f64)>,
    min_override: Option<f64>,
    max_override: Option<f64>,
    format: Option<DeduceFormat>,
}

impl TimeSeries {
    pub fn entries(&self) -> &[(TimeTick, f64)] {
        &self.values
    }

    pub fn put(&mut self, now: TimeTick, value: f64) {
        match self.values.binary_search_by(|(t, _)| t.compare(&now)) {
            Ok(i) => self.values[i].1 = value,
            Err(i) => self.values.insert(i, (now, value)),
        }
    }

    pub fn min(&self) -> f64 {
        if let Some(m) = self.min_override {
            return m;
        }
        self.values.iter().fold(0.0, |m, (_, v)| f64::min(m, *v))
    }

    pub fn max(&self) -> f64 {
        if let Some(m) = self.max_override {
            return m;
        }
        let max = self.values.iter().fold(0.0, |m, (_, v)| f64::max(m, *v));
        if max == 0.0 {
            10.0
        } else {
            max
        }
    }

    /// `Err` when a bound is not a number (Java would throw).
    pub fn set_bounds(&mut self, min: &str, max: &str) -> Result<(), String> {
        let lo = min.parse::<f64>().map_err(|e| e.to_string())?;
        let hi = max.parse::<f64>().map_err(|e| e.to_string())?;
        self.min_override = Some(lo);
        self.max_override = Some(hi);
        let f1 = DeduceFormat::from(min).ok_or("bad number format")?;
        let f2 = DeduceFormat::from(max);
        self.format = Some(f1.merge_with(f2.as_ref()));
        Ok(())
    }

    pub fn display_value(&self, value: f64) -> String {
        match &self.format {
            Some(f) => f.format(value),
            None => java_double(value),
        }
    }

    pub fn value_at(&self, tick: &TimeTick) -> f64 {
        if let Some((_, v)) = self
            .values
            .iter()
            .find(|(t, _)| t.compare(tick) == Ordering::Equal)
        {
            return *v;
        }
        let mut last: Option<&(TimeTick, f64)> = None;
        for ent in &self.values {
            if ent.0.compare(tick) == Ordering::Greater {
                let v2 = ent.1;
                let Some(l) = last else {
                    return v2;
                };
                let t2 = ent.0.time().to_f64();
                let (v1, t1) = (l.1, l.0.time().to_f64());
                let p = (tick.time().to_f64() - t1) / (t2 - t1);
                return v1 + (v2 - v1) * p;
            }
            last = Some(ent);
        }
        last.map_or(0.0, |l| l.1)
    }
}

/// Shape of a player (`TimingStyle` and the other player classes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Robust,
    Concise,
    Rectangle,
}

#[derive(Debug, Clone)]
pub struct StateData {
    pub kind: StateKind,
    pub changes: Vec<ChangeState>,
    pub constraints: Vec<TimeConstraint>,
    pub states_label: Vec<(String, String)>,
    pub initial_state: Option<String>,
    pub initial_colors: Option<Colors>,
}

#[derive(Debug, Clone)]
pub enum PlayerKind {
    State(StateData),
    Clock {
        period: super::tick::Decimal,
        pulse: super::tick::Decimal,
        offset: super::tick::Decimal,
    },
    Analog {
        series: TimeSeries,
        constraints: Vec<TimeConstraint>,
        initial_state: Option<f64>,
        ticks_every: Option<i32>,
    },
    Binary {
        constraints: Vec<TimeConstraint>,
        values: Vec<ChangeState>,
        initial_state: Option<ChangeState>,
    },
}

#[derive(Debug, Clone)]
pub struct Player {
    pub kind: PlayerKind,
    pub title: String,
    pub compact: bool,
    pub stereotype: Option<Stereotype>,
    pub general_background: Option<HColor>,
    pub suggested_height: i32,
    pub notes: Vec<TimingNote>,
}

impl Player {
    pub fn new(
        kind: PlayerKind,
        title: &str,
        compact: bool,
        stereotype: Option<Stereotype>,
        general_background: Option<HColor>,
        suggested_height: i32,
    ) -> Player {
        Player {
            kind,
            title: title.to_string(),
            compact,
            stereotype,
            general_background,
            suggested_height,
            notes: Vec::new(),
        }
    }

    pub fn new_state(
        kind: StateKind,
        title: &str,
        compact: bool,
        stereotype: Option<Stereotype>,
        general_background: Option<HColor>,
    ) -> Player {
        let data = StateData {
            kind,
            changes: Vec::new(),
            constraints: Vec::new(),
            states_label: Vec::new(),
            initial_state: None,
            initial_colors: None,
        };
        Player::new(
            PlayerKind::State(data),
            title,
            compact,
            stereotype,
            general_background,
            0,
        )
    }

    pub fn sname(&self) -> SName {
        match &self.kind {
            PlayerKind::State(d) => match d.kind {
                StateKind::Robust => SName::Robust,
                StateKind::Concise => SName::Concise,
                StateKind::Rectangle => SName::Rectangle,
            },
            PlayerKind::Clock { .. } => SName::Clock,
            PlayerKind::Analog { .. } => SName::Analog,
            PlayerKind::Binary { .. } => SName::Binary,
        }
    }

    pub fn style(&self, skin: &SkinParam) -> Style {
        style_of(
            skin,
            &queries::TIMINGDIAG
                .add(self.sname())
                .with_stereotype_obj(self.stereotype.as_ref()),
        )
    }

    pub fn add_note(
        &mut self,
        now: Option<TimeTick>,
        note: Display,
        position: Position,
        stereotype: Option<Stereotype>,
    ) {
        self.notes.push(TimingNote {
            when: now,
            note,
            position,
            stereotype,
        });
    }

    pub fn define_state(&mut self, code: &str, label: &str) -> Result<(), String> {
        match &mut self.kind {
            PlayerKind::State(d) => {
                match d.states_label.iter_mut().find(|(c, _)| c == code) {
                    Some(e) => e.1 = label.to_string(),
                    None => d.states_label.push((code.to_string(), label.to_string())),
                }
                Ok(())
            }
            _ => Err("This player does not accept state definitions".to_string()),
        }
    }

    pub fn set_state(
        &mut self,
        now: Option<TimeTick>,
        comment: Option<&str>,
        colors: Option<Colors>,
        states: &[String],
    ) -> Result<(), String> {
        match &mut self.kind {
            PlayerKind::State(d) => {
                let decoded: Vec<String> = states
                    .iter()
                    .map(|s| {
                        d.states_label
                            .iter()
                            .find(|(c, _)| c == s)
                            .map_or_else(|| s.clone(), |(_, l)| l.clone())
                    })
                    .collect();
                match now {
                    None => {
                        d.initial_state = Some(decoded[0].clone());
                        d.initial_colors = colors;
                    }
                    Some(when) => {
                        let change = ChangeState {
                            when,
                            comment: comment.map(str::to_string),
                            colors,
                            states: decoded,
                        };
                        // a sorted set keeps the first of two equal times
                        if let Err(i) = d.changes.binary_search_by(|c| c.when.compare(&change.when))
                        {
                            d.changes.insert(i, change);
                        }
                    }
                }
                Ok(())
            }
            PlayerKind::Analog {
                series,
                initial_state,
                ..
            } => {
                let value = states[0].parse::<f64>().unwrap_or_else(|e| {
                    log::error!("bad analog value {}: {e}", states[0]);
                    0.0
                });
                match now {
                    None => *initial_state = Some(value),
                    Some(t) => series.put(t, value),
                }
                if initial_state.is_none() {
                    *initial_state = Some(value);
                }
                Ok(())
            }
            PlayerKind::Binary {
                values,
                initial_state,
                ..
            } => {
                let convert = |v: &str| {
                    if v == "1" || v.eq_ignore_ascii_case("high") {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    }
                };
                let converted: Vec<String> = if states.len() == 1 {
                    vec![convert(&states[0])]
                } else {
                    vec![convert(&states[0]), convert(&states[1])]
                };
                match now {
                    None => {
                        *initial_state = Some(ChangeState {
                            when: TimeTick::new(
                                super::tick::Decimal::ZERO,
                                super::tick::TimingFormat::Decimal,
                            ),
                            comment: comment.map(str::to_string),
                            colors,
                            states: converted,
                        })
                    }
                    Some(when) => {
                        let cs = ChangeState {
                            when,
                            comment: comment.map(str::to_string),
                            colors,
                            states: converted,
                        };
                        // a sorted map keeps its first key and replaces the value
                        match values.binary_search_by(|c| c.when.compare(&cs.when)) {
                            Ok(i) => {
                                let key = values[i].when.clone();
                                values[i] = ChangeState { when: key, ..cs };
                            }
                            Err(i) => values.insert(i, cs),
                        }
                    }
                }
                Ok(())
            }
            PlayerKind::Clock { .. } => Err("A clock has no state".to_string()),
        }
    }

    pub fn create_constraint(
        &mut self,
        tick1: TimeTick,
        tick2: TimeTick,
        message: Option<&str>,
        color: Option<HColor>,
    ) -> Result<(), String> {
        let (offset, list) = match &mut self.kind {
            PlayerKind::State(d) => (
                match d.kind {
                    StateKind::Robust => 2.5,
                    StateKind::Concise => 1.0,
                    StateKind::Rectangle => 1.0,
                },
                &mut d.constraints,
            ),
            PlayerKind::Analog { constraints, .. } => (1.0, constraints),
            PlayerKind::Binary { constraints, .. } => (2.5, constraints),
            PlayerKind::Clock { .. } => return Err("A clock has no constraint".to_string()),
        };
        list.push(TimeConstraint::new(offset, tick1, tick2, message, color));
        Ok(())
    }
}

/// An arrow between two players (`TimeMessage`).
#[derive(Debug, Clone)]
pub struct TimeMessage {
    pub player1: usize,
    pub tick1: Option<TimeTick>,
    pub player2: usize,
    pub tick2: Option<TimeTick>,
    pub label: Display,
    pub specific_color: Option<HColor>,
}

impl TimeMessage {
    /// `applyStyle`: `;`-separated parts of `,`-separated keywords; only colors matter.
    pub fn apply_style(&mut self, arrow_style: Option<&str>) {
        let Some(style) = arrow_style else { return };
        for (i, part) in style.split(';').filter(|s| !s.is_empty()).enumerate() {
            for s in part.split(',').filter(|s| !s.is_empty()) {
                let l = s.to_lowercase();
                if matches!(
                    l.as_str(),
                    "dashed"
                        | "bold"
                        | "dotted"
                        | "hidden"
                        | "single"
                        | "plain"
                        | "node"
                        | "norank"
                ) || l.starts_with("thickness=")
                {
                    continue;
                }
                if i == 0 {
                    self.specific_color = Some(HColor::parse_or_white(s));
                }
            }
        }
    }
}

/// A highlighted period (`Highlight`).
#[derive(Debug, Clone)]
pub struct Highlight {
    pub from: TimeTick,
    pub to: TimeTick,
    pub caption: Display,
    pub colors: Colors,
}

impl Highlight {
    fn style(skin: &SkinParam) -> Style {
        style_of(skin, &queries::TIMINGDIAG_HIGHLIGHT)
    }

    fn back_color(&self, skin: &SkinParam) -> HColor {
        match self.colors.color(ColorType::Back) {
            Some(c) => c.clone(),
            None => Self::style(skin).value(PName::BackGroundColor).as_color(),
        }
    }

    fn line_color(&self, skin: &SkinParam) -> HColor {
        match self.colors.color(ColorType::Line) {
            Some(c) => c.clone(),
            None => Self::style(skin).value(PName::LineColor).as_color(),
        }
    }

    pub fn caption_block(&self, skin: &SkinParam) -> Rc<dyn TextBlock> {
        let fc = FontConfiguration::create(
            UFont::sans_serif(12),
            HColor::BLACK.with_dark(HColor::WHITE),
            skin.hyperlink_color(),
            Some(UStroke::simple()),
            skin.tab_size(),
        );
        self.caption
            .create_simple_block(&fc, HorizontalAlignment::Left, &skin.skin_simple())
    }

    pub fn draw_back(&self, ug: &UGraphic, ruler: &TimingRuler, height: f64, skin: &SkinParam) {
        let ug = ug
            .apply_color(HColor::TRANSPARENT)
            .apply_back_color(self.back_color(skin));
        let start = ruler.pos_in_pixel(&self.from);
        let end = ruler.pos_in_pixel(&self.to);
        ug.apply_translate(UTranslate::dx(start)).draw(
            crate::klimt::drawing::ugraphic::URectangle::build(end - start, height),
        );
    }

    pub fn draw_lines(&self, ug: &UGraphic, ruler: &TimingRuler, height: f64, skin: &SkinParam) {
        let ug = ug
            .apply_stroke(Self::style(skin).stroke())
            .apply_color(self.line_color(skin));
        let line = ULine::vline(height);
        let start = ruler.pos_in_pixel(&self.from);
        let end = ruler.pos_in_pixel(&self.to);
        ug.apply_translate(UTranslate::dx(start)).draw(line.clone());
        ug.apply_translate(UTranslate::dx(end)).draw(line);
    }
}

/// Shorthand to build path shapes.
pub fn path(points: &[(f64, f64)]) -> UPath {
    let mut p = UPath::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        if i == 0 {
            p.move_to(x, y);
        } else {
            p.line_to(x, y);
        }
    }
    p
}
