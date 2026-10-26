//! The time headers: days, weeks, months, quarters and years (`gantt/draw/header`).

use super::calendar::{is_zero_on_day, Pc};
use super::ctx::{font_configuration, Ctx, TimelineStyle};
use super::time::{Day, TimePoint, TimePointFormat, WeekNumberStrategy};
use super::timescale::{TimeScale, WorkingHours};
use super::{Bounds, GanttDiagram, PrintScale, WeeklyHeaderStrategy};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, URectangle, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::style::SName;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone)]
pub enum Kind {
    Simple,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Variant(WorkingHours),
}

/// A header, with the calendar data it shows (`TimeHeader` and its subclasses).
pub struct Header {
    kind: Kind,
    pub ts: Rc<TimeScale>,
    b: Bounds,
    ctx: Rc<Ctx>,
    st: Rc<TimelineStyle>,
    lang: String,
    plan: Pc,
    color_days: HashMap<TimePoint, HColor>,
    color_dow: HashMap<super::time::Dow, HColor>,
    name_days: HashMap<TimePoint, String>,
    separators: HashSet<Day>,
    hide_closed: bool,
    print_scale: PrintScale,
    week: WeekNumberStrategy,
    weekly_strategy: Option<WeeklyHeaderStrategy>,
    week_start_number: i32,
    closed_days: Rc<dyn Fn(Day) -> bool>,
    simple_delta: usize,
}

const DAY_WIDTH: f64 = 120.0;
const CLOSE_FACTOR: f64 = 0.2;

impl Header {
    /// `TimeHeaderFactory.createTimeHeader`.
    pub fn create(g: &GanttDiagram, b: Bounds, ctx: Rc<Ctx>, st: Rc<TimelineStyle>) -> Header {
        let start = TimePoint::of_start_of_day(b.min_day);
        let cell = st.cell_width();
        let scale = g.effective_scale();
        let (kind, ts) = if g.min_day == 0 {
            (Kind::Simple, TimeScale::wink(cell, scale, g.print_scale))
        } else if g.cal.has_working_time_ranges() {
            let wh = WorkingHours {
                ranges: g.cal.working_time_ranges.clone(),
                day_width: DAY_WIDTH,
                close_factor: CLOSE_FACTOR,
            };
            (
                Kind::Variant(wh.clone()),
                TimeScale::Variant {
                    min: b.min_day,
                    working_hours: wh,
                    open_close: g.cal.open_close.clone(),
                },
            )
        } else {
            match g.print_scale {
                PrintScale::Daily => (
                    Kind::Daily,
                    if g.hide_closed {
                        TimeScale::hide_closed(cell, start, scale, g.cal.open_close.clone())
                    } else {
                        TimeScale::daily(cell, start, scale, g.print_start, false)
                    },
                ),
                other => (
                    match other {
                        PrintScale::Weekly => Kind::Weekly,
                        PrintScale::Monthly => Kind::Monthly,
                        PrintScale::Quarterly => Kind::Quarterly,
                        _ => Kind::Yearly,
                    },
                    TimeScale::daily(cell, start, scale, g.print_start, true),
                ),
            }
        };
        let mut color_days = g.cal.color_days.clone();
        color_days.extend(g.cal.color_days_today.iter().map(|(k, v)| (*k, v.clone())));
        let open_close = g.cal.open_close.clone();
        let mut header = Header {
            kind,
            ts: Rc::new(ts),
            b,
            ctx,
            st,
            lang: g.lang.clone(),
            plan: g.cal.open_close.as_piecewise_constant(),
            color_days,
            color_dow: g.cal.color_days_of_week.clone(),
            name_days: g.cal.name_days.clone(),
            separators: g.cal.vertical_separator_before.clone(),
            hide_closed: g.hide_closed,
            print_scale: g.print_scale,
            week: g.week_number_strategy,
            weekly_strategy: g.weekly_header_strategy,
            week_start_number: g.week_starting_number,
            closed_days: Rc::new(move |d| open_close.is_closed(d)),
            simple_delta: 1,
        };
        header.simple_delta = header.init_simple_delta();
        header
    }

    fn init_simple_delta(&self) -> usize {
        if self.print_scale != PrintScale::Daily {
            return 1;
        }
        let mut day = self.b.min_day;
        let x1 = self.ts.position(TimePoint::of_start_of_day(day));
        let mut delta = 0;
        loop {
            delta += 1;
            day += 1;
            if self.ts.position(TimePoint::of_start_of_day(day)) >= x1 + 16.0 || delta > 100_000 {
                return delta;
            }
        }
    }

    // ---- sizes ----

    fn h1_daily(&self) -> f64 {
        self.st.font_size_month + 2.0
    }

    fn h2_daily(&self) -> f64 {
        self.h1_daily() + self.st.font_size_day + 2.0
    }

    fn h3_daily(&self) -> f64 {
        self.h2_daily() + self.st.font_size_day + 3.0
    }

    fn name_day_height(&self) -> f64 {
        if matches!(
            self.kind,
            Kind::Simple | Kind::Variant(_) | Kind::Quarterly | Kind::Yearly
        ) {
            return 0.0;
        }
        if self.name_days.is_empty() {
            0.0
        } else {
            self.st.font_size_day + 6.0
        }
    }

    pub fn time_header_height(&self) -> f64 {
        let s = &self.st;
        match self.kind {
            Kind::Simple => s.font_size_day + 6.0,
            Kind::Daily | Kind::Variant(_) => self.h3_daily(),
            Kind::Weekly => s.font_size_month + 4.0 + s.font_size_day + 1.0,
            Kind::Monthly => s.font_size_year + 2.0 + s.font_size_month + 2.0,
            Kind::Quarterly => s.font_size_year + s.font_size_month + 5.0,
            Kind::Yearly => s.font_size_year + 3.0,
        }
    }

    pub fn time_footer_height(&self) -> f64 {
        let s = &self.st;
        match self.kind {
            Kind::Simple => s.font_size_day + 6.0,
            Kind::Daily => s.font_size_day + s.font_size_day + s.font_size_month + 8.0,
            Kind::Variant(_) => 0.0,
            Kind::Weekly => s.font_size_month + 4.0,
            Kind::Monthly => s.font_size_year + s.font_size_month + 5.0,
            Kind::Quarterly => s.font_size_year + s.font_size_month + 5.0,
            Kind::Yearly => s.font_size_year + 3.0,
        }
    }

    pub fn full_header_height(&self) -> f64 {
        self.time_header_height() + self.name_day_height()
    }

    // ---- helpers ----

    fn is_zero_on_day(&self, t: TimePoint) -> bool {
        if matches!(self.kind, Kind::Simple) {
            return false;
        }
        is_zero_on_day(self.plan.as_ref(), t.to_day())
    }

    fn color_of(&self, t: TimePoint) -> Option<HColor> {
        self.color_days.get(&t).cloned()
    }

    fn is_bold(&self, d: Day) -> bool {
        self.separators.contains(&d)
    }

    fn days(&self) -> impl Iterator<Item = Day> {
        self.b.min_day..=self.b.max_day
    }

    fn pos(&self, d: Day) -> f64 {
        self.ts.position(TimePoint::of_start_of_day(d))
    }

    fn end_pos(&self) -> f64 {
        self.pos(self.b.max_day + 1)
    }

    fn draw_hline(&self, ug: &UGraphic, y: f64) {
        let xmin = self.pos(self.b.min_day);
        let xmax = self.end_pos();
        ug.apply_color(self.st.line_color.clone())
            .apply_translate(UTranslate::dy(y))
            .draw(ULine::hline(xmax - xmin));
    }

    fn draw_vline(&self, ug: &UGraphic, x: f64, y1: f64, y2: f64) {
        ug.apply_translate(UTranslate::new(x, y1))
            .draw(ULine::vline(y2 - y1));
    }

    fn fc(&self, param: SName, bold: bool, color: HColor) -> FontConfiguration {
        font_configuration(self.st.font(param), bold, color)
    }

    #[allow(clippy::too_many_arguments)]
    fn print_centered(
        &self,
        ug: &UGraphic,
        hide_if_too_big: bool,
        start: f64,
        end: f64,
        when: TimePoint,
        fc: &FontConfiguration,
        formats: &[TimePointFormat],
    ) {
        let available = end - start;
        for i in (0..formats.len()).rev() {
            let text = self.ctx.block(&formats[i].format(when, &self.lang), fc);
            let width = text.calculate_dimension(ug.string_bounder()).width;
            if (i == 0 && !hide_if_too_big) || width <= available {
                let diff = (available - width).max(0.0);
                text.draw(&ug.apply_translate(UTranslate::dx(start + diff / 2.0)));
                return;
            }
        }
    }

    fn draw_rectangle(&self, ug: &UGraphic, height: f64, x1: f64, x2: f64) {
        if height == 0.0 {
            return;
        }
        let ug = ug
            .apply_color(HColor::TRANSPARENT)
            .apply_translate(UTranslate::new(x1, self.full_header_height()));
        if x2 > x1 {
            ug.draw(URectangle::build(x2 - x1, height));
        }
    }

    fn draw_colors_background(&self, ug: &UGraphic, total: f64) {
        let height = total - self.full_header_height();
        let mut pending: Option<(HColor, f64, f64)> = None;
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.ts.position(wink);
            let x2 = x1 + self.ts.width(wink);
            let mut back = self.color_of(wink);
            if let Some(dow) = self.color_dow.get(&wink.to_day_of_week()) {
                back = Some(dow.clone());
            }
            if back.is_none() && self.is_zero_on_day(wink) {
                back = Some(self.st.closed_background.clone());
            }
            match back {
                None => {
                    if let Some((c, a, b)) = pending.take() {
                        self.draw_rectangle(&ug.apply_back_color(c), height, a, b);
                    }
                }
                Some(back) => {
                    if pending.as_ref().is_some_and(|p| p.0 != back) {
                        let (c, a, b) = pending.take().expect("pending");
                        self.draw_rectangle(&ug.apply_back_color(c), height, a, b);
                    }
                    match &mut pending {
                        None => pending = Some((back, x1, x2)),
                        Some(p) => p.2 = x2,
                    }
                }
            }
        }
        if let Some((c, a, b)) = pending {
            self.draw_rectangle(&ug.apply_back_color(c), height, a, b);
        }
    }

    fn vertical_separator_ug(&self, ug: &UGraphic) -> UGraphic {
        ug.apply_color(self.st.vsep_color.clone())
            .apply_stroke(self.st.vsep_stroke)
    }

    fn print_vertical_separators_base(&self, ug: &UGraphic, total: f64) {
        let ug = self.vertical_separator_ug(ug);
        for day in self.days() {
            if self.is_bold(day) {
                self.draw_vline(&ug, self.pos(day), self.full_header_height(), total);
            }
        }
    }

    fn print_named_days(&self, ug: &UGraphic) {
        if self.name_days.is_empty() {
            return;
        }
        let mut last: Option<&String> = None;
        let fc = self.fc(SName::Month, false, self.st.open_font.clone());
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            let name = self.name_days.get(&wink);
            if let Some(n) = name {
                if last != Some(n) {
                    let x1 = self.ts.position(wink);
                    let label = self.ctx.block(n, &fc);
                    label.draw(&ug.apply_translate(UTranslate::new(x1, self.time_header_height())));
                }
            }
            last = name;
        }
    }

    // ---- public drawing ----

    pub fn draw_time_header(&self, ug: &UGraphic, total: f64) {
        match &self.kind {
            Kind::Simple => {
                self.draw_colors_background(ug, total);
                self.draw_small_vlines_day(ug, total);
                self.print_vertical_separators_base(ug, total);
                self.draw_simple_day_counter(ug);
            }
            _ => {
                self.draw_colors_background(ug, total);
                self.draw_internal(ug, total);
            }
        }
    }

    pub fn draw_time_footer(&self, ug: &UGraphic) {
        match &self.kind {
            Kind::Simple => self.draw_simple_day_counter(&ug.apply_translate(UTranslate::dy(3.0))),
            Kind::Daily => {
                let h = self.st.font_size_day + 2.0;
                self.daily_texts_day_of_week(ug);
                self.daily_text_day_of_month(&ug.apply_translate(UTranslate::dy(h + 2.0)));
                self.daily_months(&ug.apply_translate(UTranslate::dy(2.0 * h + 3.0)));
            }
            Kind::Variant(_) => {}
            Kind::Weekly => {
                self.draw_hline(ug, 0.0);
                self.weekly_months(ug);
                self.draw_hline(ug, self.time_footer_height());
            }
            Kind::Monthly => {
                let h1 = self.st.font_size_year;
                let h2 = self.st.font_size_month;
                self.monthly_months(ug);
                self.years(
                    ug.apply_translate(UTranslate::dy(h2 + 2.0)).clone(),
                    false,
                    SName::Month,
                );
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h2 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            Kind::Quarterly => {
                let h1 = self.st.font_size_year;
                let h2 = self.st.font_size_month;
                self.quarters(ug);
                self.years(
                    ug.apply_translate(UTranslate::dy(h2 + 2.0)),
                    false,
                    SName::Month,
                );
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h2 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            Kind::Yearly => {
                self.years(ug.clone(), true, SName::Year);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, self.time_footer_height());
            }
        }
    }

    fn draw_internal(&self, ug: &UGraphic, total: f64) {
        let full = self.full_header_height();
        match &self.kind {
            Kind::Daily => {
                self.daily_months(ug);
                self.daily_texts_day_of_week(&ug.apply_translate(UTranslate::dy(self.h1_daily())));
                self.daily_text_day_of_month(&ug.apply_translate(UTranslate::dy(self.h2_daily())));
                self.daily_vertical_separators(ug, total);
                self.print_named_days(ug);
                self.draw_hline(ug, full);
                self.draw_hline(ug, total);
            }
            Kind::Variant(wh) => {
                self.variant_non_working(ug, total, wh);
                self.daily_months(ug);
                self.daily_texts_day_of_week(&ug.apply_translate(UTranslate::dy(self.h1_daily())));
                self.daily_text_day_of_month(&ug.apply_translate(UTranslate::dy(self.h2_daily())));
                self.variant_small_vlines(
                    &ug.apply_translate(UTranslate::dy(self.h1_daily())),
                    total,
                );
            }
            Kind::Weekly => {
                self.weekly_days_of_month(ug);
                self.weekly_vertical_separators(ug, total);
                self.weekly_months(ug);
                self.print_named_days(ug);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, self.weekly_h1());
                self.draw_hline(ug, full);
            }
            Kind::Monthly => {
                let h1 = self.st.font_size_year + 2.0;
                let h2 = h1 + self.st.font_size_month + 2.0;
                self.years(ug.clone(), false, SName::Month);
                self.monthly_months(&ug.apply_translate(UTranslate::dy(h1)));
                self.print_vertical_separators_base(ug, total);
                self.print_named_days(ug);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h1);
                self.draw_hline(ug, h2);
            }
            Kind::Quarterly => {
                let h1 = self.st.font_size_year;
                let h2 = self.st.font_size_month;
                self.years(ug.clone(), false, SName::Month);
                self.quarters(&ug.apply_translate(UTranslate::dy(h1 + 2.0)));
                self.print_vertical_separators_base(ug, total);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, h1 + 2.0);
                self.draw_hline(ug, h1 + 2.0 + h2 + 2.0);
            }
            Kind::Yearly => {
                self.years(ug.clone(), true, SName::Year);
                self.print_vertical_separators_base(ug, total);
                self.draw_hline(ug, 0.0);
                self.draw_hline(ug, full);
            }
            Kind::Simple => {}
        }
    }

    // ---- simple ----

    fn increment_day(&self, mut day: Day) -> Day {
        for _ in 0..self.simple_delta {
            day += if self.print_scale == PrintScale::Weekly {
                7
            } else {
                1
            };
        }
        day
    }

    fn simple_days(&self) -> Vec<Day> {
        let mut out = Vec::new();
        let mut day = self.b.min_day;
        while day <= self.b.max_day + 1 {
            out.push(day);
            day = self.increment_day(day);
        }
        out
    }

    fn draw_small_vlines_day(&self, ug: &UGraphic, total: f64) {
        let ug = ug
            .apply_color(self.st.line_color.clone())
            .apply_translate(UTranslate::dy(6.0));
        for day in self.simple_days() {
            ug.apply_translate(UTranslate::dx(self.pos(day)))
                .draw(ULine::vline(total + 2.0));
        }
    }

    fn draw_simple_day_counter(&self, ug: &UGraphic) {
        let fc = font_configuration(self.st.font(SName::Day), false, self.st.open_font.clone());
        for day in self.simple_days() {
            let wink = TimePoint::of_start_of_day(day);
            let value = if self.print_scale == PrintScale::Weekly {
                wink.absolute_day_num() / 7 + 1
            } else {
                wink.absolute_day_num() + 1
            };
            let num = self.ctx.block(&value.to_string(), &fc);
            let x1 = self.ts.position(wink);
            let x2 = if self.print_scale == PrintScale::Weekly {
                let w6 = wink.add_days(6);
                self.ts.position(w6) + self.ts.width(w6)
            } else {
                self.pos(self.increment_day(day))
            };
            let width = num.calculate_dimension(ug.string_bounder()).width;
            let delta = (x2 - x1) - width;
            if wink < TimePoint::of_start_of_day(self.b.max_day + 1) {
                num.draw(&ug.apply_translate(UTranslate::dx(x1 + delta / 2.0)));
            }
        }
    }

    // ---- daily and variant ----

    fn is_hidden(&self, wink: TimePoint) -> bool {
        self.hide_closed && (self.closed_days)(wink.to_day())
    }

    fn day_fc(&self, wink: TimePoint) -> FontConfiguration {
        let color = if self.is_zero_on_day(wink) {
            self.st.closed_font.clone()
        } else {
            self.st.open_font.clone()
        };
        self.fc(SName::Day, false, color)
    }

    fn daily_texts_day_of_week(&self, ug: &UGraphic) {
        let formats: &[TimePointFormat] = if matches!(self.kind, Kind::Variant(_)) {
            &[
                TimePointFormat::DayOfWeekShort,
                TimePointFormat::DayOfWeekLong,
            ]
        } else {
            &[TimePointFormat::DayOfWeekShort]
        };
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            if self.is_hidden(wink) {
                continue;
            }
            let x1 = self.ts.position(wink);
            let x2 = self.ts.position(wink.increment());
            self.print_centered(ug, false, x1, x2, wink, &self.day_fc(wink), formats);
        }
    }

    fn daily_text_day_of_month(&self, ug: &UGraphic) {
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            if self.is_hidden(wink) {
                continue;
            }
            let x1 = self.ts.position(wink);
            let x2 = self.ts.position(wink.increment());
            self.print_centered(
                ug,
                false,
                x1,
                x2,
                wink,
                &self.day_fc(wink),
                &[TimePointFormat::DayOfMonth],
            );
        }
    }

    fn daily_months(&self, ug: &UGraphic) {
        let fc = self.fc(SName::Month, true, self.st.open_font.clone());
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            if self.is_hidden(wink) {
                continue;
            }
            let x1 = self.ts.position(wink);
            if last.is_none_or(|l| l.month_year() != wink.month_year()) {
                if let Some(l) = last {
                    self.print_month_daily(ug, l, last_change, x1, &fc);
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let x1 = self.pos(self.b.max_day + 1);
        if x1 > last_change {
            if let Some(l) = last {
                self.print_month_daily(ug, l, last_change, x1, &fc);
            }
        }
    }

    fn print_month_daily(
        &self,
        ug: &UGraphic,
        t: TimePoint,
        start: f64,
        end: f64,
        fc: &FontConfiguration,
    ) {
        self.print_centered(
            ug,
            false,
            start,
            end,
            t,
            fc,
            &[
                TimePointFormat::MonthShort,
                TimePointFormat::MonthLong,
                TimePointFormat::MonthYearLong,
            ],
        );
    }

    fn daily_vertical_separators(&self, ug: &UGraphic, total: f64) {
        let sep = self.vertical_separator_ug(ug);
        let line = ug.apply_color(self.st.line_color.clone());
        let full = self.full_header_height();
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            let x = self.ts.position(wink);
            if self.is_bold(day) || self.ts.width(wink.decrement()) == 0.0 {
                self.draw_vline(&sep, x, full, total);
            } else {
                self.draw_vline(&line, x, full, total);
            }
        }
        self.draw_vline(&line, self.end_pos(), full, total);
    }

    fn variant_non_working(&self, ug: &UGraphic, height: f64, wh: &WorkingHours) {
        let back = self.st.closed_background.clone();
        let ug = ug
            .apply_color(back.clone())
            .apply_back_color(back)
            .apply_translate(UTranslate::dy(self.h3_daily()));
        for day in self.days() {
            let x1 = self.pos(day);
            let day_ug = ug.apply_translate(UTranslate::dx(x1));
            for (x, w) in wh.closed_rectangles() {
                day_ug
                    .apply_translate(UTranslate::dx(x))
                    .draw(URectangle::build(w, height));
            }
        }
    }

    fn variant_small_vlines(&self, ug: &UGraphic, total: f64) {
        let ug = ug.apply_color(self.st.line_color.clone());
        for day in self.b.min_day..=self.b.max_day + 1 {
            ug.apply_translate(UTranslate::dx(self.pos(day)))
                .draw(ULine::vline(total));
        }
    }

    // ---- weekly ----

    fn weekly_h1(&self) -> f64 {
        self.st.font_size_month + 4.0
    }

    fn weekly_months(&self, ug: &UGraphic) {
        let fc = self.fc(SName::Month, true, self.st.open_font.clone());
        let h1 = self.weekly_h1();
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.ts.position(wink);
            if last.is_none_or(|l| l.month_year() != wink.month_year()) {
                self.draw_vline(&ug.apply_color(self.st.line_color.clone()), x1, 0.0, h1);
                if let Some(l) = last {
                    self.print_centered(
                        ug,
                        false,
                        last_change,
                        x1,
                        l,
                        &fc,
                        &[TimePointFormat::MonthShort, TimePointFormat::MonthYearShort],
                    );
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let end = self.end_pos();
        self.draw_vline(&ug.apply_color(self.st.line_color.clone()), end, 0.0, h1);
        if let Some(l) = last {
            if end > last_change {
                self.print_centered(
                    ug,
                    false,
                    last_change,
                    end,
                    l,
                    &fc,
                    &[TimePointFormat::MonthShort, TimePointFormat::MonthYearShort],
                );
            }
        }
    }

    fn weekly_vertical_separators(&self, ug: &UGraphic, total: f64) {
        let line = ug.apply_color(self.st.line_color.clone());
        let h1 = self.weekly_h1();
        for day in self.days() {
            let wink = TimePoint::of_start_of_day(day);
            if wink.to_day_of_week() == self.week.first_day_of_week {
                self.draw_vline(&line, self.ts.position(wink), h1, total);
            }
        }
        self.draw_vline(&line, self.end_pos(), h1, total);
        self.print_vertical_separators_base(ug, total);
    }

    fn weekly_days_of_month(&self, ug: &UGraphic) {
        let fc = self.fc(SName::Day, false, self.st.open_font.clone());
        let mut counter = self.week_start_number;
        let ug = ug.apply_translate(UTranslate::dy(self.weekly_h1()));
        for day in self.b.min_day..self.b.max_day - 1 {
            let wink = TimePoint::of_start_of_day(day);
            if wink.to_day_of_week() == self.week.first_day_of_week {
                let num = match self.weekly_strategy {
                    Some(WeeklyHeaderStrategy::FromN) => {
                        counter += 1;
                        (counter - 1).to_string()
                    }
                    Some(WeeklyHeaderStrategy::DayOfMonth) => wink.day_of_month().to_string(),
                    _ => wink.week_of_year(self.week).to_string(),
                };
                let block = self.ctx.block(&num, &fc);
                block.draw(&ug.apply_translate(UTranslate::dx(self.ts.position(wink) + 5.0)));
            }
        }
    }

    // ---- monthly, quarterly, yearly ----

    fn monthly_months(&self, ug: &UGraphic) {
        let fc = self.fc(SName::Day, false, self.st.open_font.clone());
        let h2 = self.st.font_size_month;
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        let print = |l: TimePoint, a: f64, b: f64| {
            self.print_centered(
                ug,
                false,
                a,
                b,
                l,
                &fc,
                &[TimePointFormat::MonthShort, TimePointFormat::MonthLong],
            );
        };
        for day in self.b.min_day..self.b.max_day {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.ts.position(wink);
            if last.is_none_or(|l| l.month_year() != wink.month_year()) {
                self.draw_vline(
                    &ug.apply_color(self.st.line_color.clone()),
                    x1,
                    0.0,
                    h2 + 2.0,
                );
                if let Some(l) = last {
                    print(l, last_change, x1);
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let x1 = self.end_pos();
        if x1 > last_change {
            if let Some(l) = last {
                print(l, last_change, x1);
            }
        }
        self.draw_vline(
            &ug.apply_color(self.st.line_color.clone()),
            x1,
            0.0,
            h2 + 2.0,
        );
    }

    fn quarters(&self, ug: &UGraphic) {
        let h2 = self.st.font_size_month;
        let fc = self.fc(SName::Day, false, self.st.open_font.clone());
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        let print = |l: TimePoint, a: f64, b: f64| {
            self.print_centered(ug, false, a, b, l, &fc, &[TimePointFormat::Quarter]);
        };
        for day in self.b.min_day..self.b.max_day {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.ts.position(wink);
            if last.is_none_or(|l| l.quarter() != wink.quarter()) {
                self.draw_vline(
                    &ug.apply_color(self.st.line_color.clone()),
                    x1,
                    0.0,
                    h2 + 2.0,
                );
                if let Some(l) = last {
                    print(l, last_change, x1);
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let x1 = self.end_pos();
        if x1 > last_change {
            if let Some(l) = last {
                print(l, last_change, x1);
            }
        }
        self.draw_vline(
            &ug.apply_color(self.st.line_color.clone()),
            x1,
            0.0,
            h2 + 2.0,
        );
    }

    /// The row of years of the monthly, quarterly and yearly headers.
    fn years(&self, ug: UGraphic, hide_if_too_big: bool, font: SName) {
        let h1 = self.st.font_size_year;
        let fc = self.fc(font, true, self.st.open_font.clone());
        let inclusive = matches!(self.kind, Kind::Yearly);
        let last_day = if inclusive {
            self.b.max_day + 1
        } else {
            self.b.max_day
        };
        let mut last: Option<TimePoint> = None;
        let mut last_change = -1.0;
        for day in self.b.min_day..last_day {
            let wink = TimePoint::of_start_of_day(day);
            let x1 = self.ts.position(wink);
            if last.is_none_or(|l| l.year() != wink.year()) {
                self.draw_vline(
                    &ug.apply_color(self.st.line_color.clone()),
                    x1,
                    0.0,
                    h1 + 2.0,
                );
                if let Some(l) = last {
                    self.print_centered(
                        &ug,
                        hide_if_too_big,
                        last_change,
                        x1,
                        l,
                        &fc,
                        &[TimePointFormat::Year],
                    );
                }
                last_change = x1;
                last = Some(wink);
            }
        }
        let x1 = self.end_pos();
        if x1 > last_change {
            if let Some(l) = last {
                self.print_centered(
                    &ug,
                    hide_if_too_big,
                    last_change,
                    x1,
                    l,
                    &fc,
                    &[TimePointFormat::Year],
                );
            }
        }
        self.draw_vline(
            &ug.apply_color(self.st.line_color.clone()),
            x1,
            0.0,
            h1 + 2.0,
        );
    }
}
