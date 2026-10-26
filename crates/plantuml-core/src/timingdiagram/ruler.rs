//! The time axis (`TimingRuler`).

use super::tick::{Decimal, TimeTick, TimingFormat};
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style, StyleQuery};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How the time axis is labelled (`TimeAxisStategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeAxisStrategy {
    Automatic,
    Hidden,
    Manual,
}

#[derive(Debug, Clone)]
pub struct TimingRuler {
    times: RefCell<Vec<TimeTick>>,
    tick_interval_in_pixels: Cell<i64>,
    forced_tick_unitary: Cell<i64>,
    format: RefCell<TimingFormat>,
    hcf: Cell<i64>,
}

impl Default for TimingRuler {
    fn default() -> Self {
        Self {
            times: RefCell::new(Vec::new()),
            tick_interval_in_pixels: Cell::new(50),
            forced_tick_unitary: Cell::new(0),
            format: RefCell::new(TimingFormat::Decimal),
            hcf: Cell::new(-1),
        }
    }
}

fn style(skin: &SkinParam, q: &StyleQuery) -> Style {
    skin.style_builder()
        .merged_style(q)
        .expect("timing diagram style")
}

/// Stroke and color of the dashed vertical grid lines (`applyForVLines`).
pub fn apply_for_vlines(ug: &UGraphic, style: &Style) -> UGraphic {
    let stroke = UStroke::new(3.0, 5.0, 0.5);
    let color = style.value(PName::LineColor).as_color();
    ug.apply_stroke(stroke).apply_color(color)
}

impl TimingRuler {
    pub fn ensure_not_empty(&self) {
        if self.times.borrow().is_empty() {
            self.insert(TimeTick::new(Decimal::ZERO, TimingFormat::Decimal));
        }
        if self.max().time().signum() > 0 && self.min().time().signum() < 0 {
            self.insert(TimeTick::new(Decimal::ZERO, TimingFormat::Decimal));
        }
    }

    /// Adds to the sorted set; a tick equal to an existing one is dropped (`TreeSet.add`).
    fn insert(&self, time: TimeTick) {
        let mut times = self.times.borrow_mut();
        match times.binary_search_by(|t| t.compare(&time)) {
            Ok(_) => {}
            Err(i) => times.insert(i, time),
        }
    }

    pub fn scale_in_pixels(&self, tick: i64, pixel: i64) {
        self.tick_interval_in_pixels.set(pixel);
        self.forced_tick_unitary.set(tick);
    }

    fn interval(&self) -> f64 {
        self.tick_interval_in_pixels.get() as f64
    }

    fn optimal_tick_unit(&self) -> i64 {
        let forced = self.forced_tick_unitary.get();
        if forced != 0 {
            return forced;
        }
        let hcf = self.highest_common_factor();
        let max_diagram_width = 4000.0;
        if hcf == 1 && self.diagram_width(hcf) > max_diagram_width {
            let range = self.max().time().to_f64() - self.min().time().to_f64();
            return (1.0 + (self.interval() * range / max_diagram_width) + 0.5).floor() as i64;
        }
        hcf
    }

    pub fn width(&self) -> f64 {
        if self.times.borrow().is_empty() {
            return 100.0;
        }
        self.diagram_width(self.optimal_tick_unit())
    }

    fn diagram_width(&self, tick_unitary: i64) -> f64 {
        let delta = self.max().time().to_f64() - self.min().time().to_f64();
        (delta / tick_unitary as f64 + 1.0) * self.interval()
    }

    fn highest_common_factor(&self) -> i64 {
        if self.hcf.get() == -1 {
            for tick in self.absolute_ticks() {
                if self.hcf.get() == -1 {
                    self.hcf.set(tick);
                } else {
                    self.hcf.set(gcd(self.hcf.get(), tick));
                }
            }
        }
        self.hcf.get()
    }

    /// Distinct non-zero absolute values, largest first.
    fn absolute_ticks(&self) -> Vec<i64> {
        let mut v: Vec<i64> = self
            .times
            .borrow()
            .iter()
            .map(|t| t.time().to_i64().abs())
            .filter(|&x| x > 0)
            .collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v.dedup();
        v
    }

    fn nb_tick(&self) -> i64 {
        if self.times.borrow().is_empty() {
            return 1;
        }
        let delta = self.max().time().to_i64() - self.min().time().to_i64();
        let unit = self.optimal_tick_unit();
        if unit == 0 {
            return 1000;
        }
        1000.min(1 + delta / unit)
    }

    pub fn pos_in_pixel(&self, when: &TimeTick) -> f64 {
        self.pos_in_pixel_internal(when.time().to_f64())
    }

    pub fn pos_in_pixel_f64(&self, time: f64) -> f64 {
        self.pos_in_pixel_internal(time)
    }

    fn pos_in_pixel_internal(&self, time: f64) -> f64 {
        let time = time - self.min().time().to_f64();
        time / self.optimal_tick_unit() as f64 * self.interval()
    }

    pub fn add_time(&self, time: TimeTick) {
        self.hcf.set(-1);
        if *time.format() != TimingFormat::Decimal {
            *self.format.borrow_mut() = time.format().clone();
        }
        self.insert(time);
    }

    fn time_text_block(&self, skin: &SkinParam, text: &str) -> Rc<dyn TextBlock> {
        let st = style(skin, &queries::TIMINGDIAG_TIMELINE);
        Display::with_newlines(Some(text)).create_simple_block(
            &st.font_configuration(None),
            HorizontalAlignment::Left,
            &skin.skin_simple(),
        )
    }

    pub fn draw_time_axis(
        &self,
        ug: &UGraphic,
        strategy: TimeAxisStrategy,
        codes: &[(String, TimeTick)],
        skin: &SkinParam,
    ) {
        if strategy == TimeAxisStrategy::Hidden {
            return;
        }
        let timeline = style(skin, &queries::TIMINGDIAG_TIMELINE);
        let color = timeline.value(PName::LineColor).as_color();
        let ug = ug.apply_stroke(timeline.stroke()).apply_color(color);
        if strategy == TimeAxisStrategy::Automatic {
            self.draw_automatic(&ug, skin);
        } else {
            self.draw_manual(&ug, codes, skin);
        }
    }

    fn first_positive_or_zero(&self) -> f64 {
        self.times
            .borrow()
            .iter()
            .find(|t| t.time().signum() >= 0)
            .map_or(0.0, |t| t.time().to_f64())
    }

    fn draw_manual(&self, ug: &UGraphic, codes: &[(String, TimeTick)], skin: &SkinParam) {
        let tick_height = 5.0;
        let line = ULine::vline(tick_height);
        let first = self.pos_in_pixel_internal(self.first_positive_or_zero());
        let mut nb = 0;
        while first + f64::from(nb) * self.interval() <= self.width() {
            nb += 1;
        }
        ug.apply_translate(UTranslate::dx(first))
            .draw(ULine::hline(f64::from(nb - 1) * self.interval()));
        let times = self.times.borrow().clone();
        for tick in &times {
            ug.apply_translate(UTranslate::dx(self.pos_in_pixel(tick)))
                .draw(line.clone());
            let label = self.label(tick, codes);
            if label.is_empty() {
                continue;
            }
            let text = self.time_text_block(skin, &label);
            let dim = text.calculate_dimension(ug.string_bounder());
            text.draw(&ug.apply_translate(UTranslate::new(
                self.pos_in_pixel(tick) - dim.width / 2.0,
                tick_height + 1.0,
            )));
        }
    }

    fn draw_automatic(&self, ug: &UGraphic, skin: &SkinParam) {
        let tick_height = 5.0;
        let line = ULine::vline(tick_height);
        let first = self.pos_in_pixel_internal(self.first_positive_or_zero());
        let mut nb = 0;
        while first + f64::from(nb) * self.interval() <= self.width() {
            ug.apply_translate(UTranslate::dx(first + f64::from(nb) * self.interval()))
                .draw(line.clone());
            nb += 1;
        }
        ug.apply_translate(UTranslate::dx(first))
            .draw(ULine::hline(f64::from(nb - 1) * self.interval()));
        let format = self.format.borrow().clone();
        for round in self.round_values() {
            let text = self.time_text_block(skin, &format.format_time(round));
            let dim = text.calculate_dimension(ug.string_bounder());
            text.draw(&ug.apply_translate(UTranslate::new(
                self.pos_in_pixel_internal(round as f64) - dim.width / 2.0,
                tick_height + 1.0,
            )));
        }
    }

    fn label(&self, tick: &TimeTick, codes: &[(String, TimeTick)]) -> String {
        for (code, t) in codes {
            if tick.same(t) {
                return code.clone();
            }
        }
        self.format.borrow().format_decimal(tick.time())
    }

    fn round_values(&self) -> Vec<i64> {
        let mut result: Vec<i64> = Vec::new();
        let forced = self.forced_tick_unitary.get();
        if forced == 0 {
            for tick in self.times.borrow().iter() {
                result.push(tick.time().to_i64());
            }
        } else {
            let nb = self.nb_tick();
            let min = self.min().time().to_i64();
            for i in 0..=nb {
                result.push(forced * i + min);
            }
        }
        result.sort_unstable();
        result.dedup();
        if let (Some(&first), Some(&last)) = (result.first(), result.last()) {
            if first < 0 && last > 0 {
                result.push(0);
                result.sort_unstable();
                result.dedup();
            }
        }
        result
    }

    pub fn draw_vlines(&self, ug: &UGraphic, height: f64, skin: &SkinParam) {
        let ug = apply_for_vlines(ug, &style(skin, &queries::TIMINGDIAG_TIMEGRID));
        let line = ULine::vline(height);
        for i in 0..=self.nb_tick() {
            ug.apply_translate(UTranslate::dx(self.interval() * i as f64))
                .draw(line.clone());
        }
    }

    pub fn height(&self, sb: &dyn StringBounder, skin: &SkinParam) -> f64 {
        self.time_text_block(skin, "0")
            .calculate_dimension(sb)
            .height
    }

    fn max(&self) -> TimeTick {
        self.times
            .borrow()
            .last()
            .cloned()
            .unwrap_or_else(|| TimeTick::new(Decimal::ZERO, TimingFormat::Decimal))
    }

    fn min(&self) -> TimeTick {
        self.times
            .borrow()
            .first()
            .cloned()
            .unwrap_or_else(|| TimeTick::new(Decimal::ZERO, TimingFormat::Decimal))
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a, b);
    let mut r = a;
    while r != 0 {
        if b == 0 {
            break;
        }
        r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}
