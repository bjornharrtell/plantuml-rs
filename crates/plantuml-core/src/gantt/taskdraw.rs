//! Drawing a row of the chart: a task bar, a milestone, a group or a separator
//! (`gantt/draw/TaskDraw*`, `RectangleTask`).

use super::calendar::is_zero_on_day;
use super::ctx::{gantt_query, Ctx};
use super::model::{unlinear, ArrowType, CenterBorderColor, GSide, Kind, TaskId};
use super::time::{Day, TimePoint};
use super::timescale::TimeScale;
use super::{Bounds, GanttDiagram};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{CreoleMode, LineBreakStrategy};
use crate::klimt::drawing::ugraphic::{
    UGraphic, ULine, UPath, UPolygon, URectangle, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{with_margin, EmptyBlock, TextBlock};
use crate::real::Real;
use crate::stereo::Stereotype;
use crate::style::{Margins, PName, SName, Style};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawKind {
    Regular,
    Diamond,
    Group,
    Separator,
}

/// A rectangle of the chart in absolute coordinates (`FingerPrint`).
#[derive(Debug, Clone, Copy)]
pub struct FingerPrint {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl FingerPrint {
    pub fn overlap(&self, other: &FingerPrint) -> f64 {
        if self.x >= other.x + other.width || other.x >= self.x + self.width {
            return 0.0;
        }
        if self.y >= other.y + other.height || other.y >= self.y + self.height {
            return 0.0;
        }
        self.y + self.height - other.y
    }
}

/// A segment of the x axis (`sequencediagram.graphic.Segment`).
#[derive(Debug, Clone, Copy)]
struct Seg {
    pos1: f64,
    pos2: f64,
}

impl Seg {
    fn length(&self) -> f64 {
        self.pos2 - self.pos1
    }

    fn contains(&self, y: f64) -> bool {
        y >= self.pos1 && y <= self.pos2
    }

    fn contains_seg(&self, o: &Seg) -> bool {
        self.contains(o.pos1) && self.contains(o.pos2)
    }

    /// `cutSegmentIfNeed`: this segment without the pauses.
    fn cut(&self, delays: &[Seg]) -> Vec<Seg> {
        let mut sorted = delays.to_vec();
        sorted.sort_by(|a, b| {
            a.pos1
                .partial_cmp(&b.pos1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut result = Vec::new();
        let mut pending = self.pos1;
        for pause in sorted {
            if (pause.pos1 - pending).abs() < 0.001 {
                pending = pause.pos2;
                continue;
            }
            if pause.pos1 < pending {
                continue;
            }
            if pause.pos1 > self.pos2 {
                if pending < self.pos2 {
                    result.push(Seg {
                        pos1: pending,
                        pos2: self.pos2,
                    });
                }
                return result;
            }
            if self.contains_seg(&pause) {
                result.push(Seg {
                    pos1: pending,
                    pos2: pause.pos1,
                });
                pending = pause.pos2;
            }
        }
        if pending < self.pos2 {
            result.push(Seg {
                pos1: pending,
                pos2: self.pos2,
            });
        }
        result
    }
}

/// One row of the chart, ready to be measured and drawn.
pub struct Draw {
    #[allow(dead_code)]
    pub id: TaskId,
    pub kind: DrawKind,
    pub y: Real,
    pub row: Option<TaskId>,
    pub hidden: bool,
    start: TimePoint,
    end: TimePoint,
    odd_start: bool,
    odd_end: bool,
    paused: Vec<Day>,
    colors: Option<CenterBorderColor>,
    completion: i32,
    right_arrow: bool,
    has_display_string: bool,
    style: Style,
    margin: Margins,
    padding: Margins,
    title: Rc<dyn TextBlock>,
    fc: FontConfiguration,
    line_color: HColor,
    back_color: HColor,
    back_undone: HColor,
    min_day: Day,
    max_day: Day,
    /// Extra pixels of the shape of a group whose label fits in its bar.
    group_extra: f64,
    opale: Option<Opale>,
    note_dim: Option<Dimension>,
    display_block: Option<Rc<dyn TextBlock>>,
}

struct Opale {
    block: Rc<dyn TextBlock>,
    background: HColor,
    border: HColor,
    shadowing: f64,
    stroke: UStroke,
}

const OPALE_MARGIN_X1: f64 = 6.0;
const OPALE_MARGIN_X2: f64 = 15.0;
const OPALE_MARGIN_Y: f64 = 5.0;

impl Opale {
    fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.block.calculate_dimension(sb);
        Dimension::new(
            d.width + OPALE_MARGIN_X1 + OPALE_MARGIN_X2,
            d.height + 2.0 * OPALE_MARGIN_Y,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let dim = self.dimension(ug.string_bounder());
        let ug = ug
            .apply_back_color(self.background.clone())
            .apply_color(self.border.clone())
            .apply_stroke(self.stroke);
        let mut polygon = crate::skin::rose::note::polygon_normal(dim.width, dim.height, 0.0);
        polygon.delta_shadow = self.shadowing;
        ug.draw(polygon);
        ug.draw(crate::skin::rose::note::corner(dim.width, 0.0));
        self.block
            .draw(&ug.apply_translate(UTranslate::new(OPALE_MARGIN_X1, OPALE_MARGIN_Y)));
    }
}

fn build_opale(ctx: &Ctx, note: &Display, stereotype: Option<&Stereotype>) -> Opale {
    let style = ctx.style_q(&gantt_query(&[SName::Note]).with_stereotype_obj(stereotype));
    let fc = style.font_configuration(None);
    let block = note.create_block(
        &fc,
        style.horizontal_alignment(),
        &ctx.simple,
        &LineBreakStrategy::NONE,
        CreoleMode::Full,
        None,
        None,
        0.0,
        0.0,
    );
    Opale {
        block: with_margin(block, style.padding()),
        background: style.value(PName::BackGroundColor).as_color(),
        border: style.value(PName::LineColor).as_color(),
        shadowing: style.shadowing(),
        stroke: style.stroke(),
    }
}

impl Draw {
    /// `TaskDrawRegistryData`'s creation of the draw of a task.
    pub fn build(
        g: &GanttDiagram,
        id: TaskId,
        b: &Bounds,
        ctx: &Ctx,
        ts: &TimeScale,
        y: Real,
    ) -> Draw {
        let task = &g.tasks[id];
        let (kind, query) = match &task.kind {
            Kind::Separator { .. } => (DrawKind::Separator, gantt_query(&[SName::Separator])),
            Kind::Group { .. } => (DrawKind::Group, gantt_query(&[SName::Task])),
            Kind::Impl(t) if t.diamond => (DrawKind::Diamond, gantt_query(&[SName::Milestone])),
            Kind::Impl(_) => (
                DrawKind::Regular,
                gantt_query(&[SName::Task]).with_stereotype_obj(task.stereotype.as_ref()),
            ),
        };
        let style = ctx.style_q(&query);
        let fc = style.font_configuration(None);
        let pretty = match &task.kind {
            Kind::Impl(t) if !g.hide_resource_name && !t.resources.is_empty() => {
                let parts: Vec<String> = t
                    .resources
                    .iter()
                    .map(|(name, pct)| {
                        if *pct != 100 {
                            format!("{{{name}:{pct}%}}")
                        } else {
                            format!("{{{name}}}")
                        }
                    })
                    .collect();
                format!("{} {}", task.code.display, parts.join(" "))
            }
            _ => task.code.display.clone(),
        };
        let title: Rc<dyn TextBlock> = match &task.kind {
            Kind::Separator { comment: None } => Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            }),
            Kind::Separator { comment: Some(c) } => ctx.block(c, &fc),
            _ => ctx.block(&pretty, &fc),
        };
        let (mut start, mut end) = (TimePoint(0), TimePoint(0));
        let (mut odd_start, mut odd_end) = (false, false);
        let mut hidden = false;
        if !matches!(task.kind, Kind::Separator { .. }) {
            start = g.start_for_drawing(id, b);
            end = g.end_for_drawing(id, b);
            hidden = g.is_hidden(id, b);
            if kind == DrawKind::Regular && b.printing {
                odd_start = TimePoint::of_start_of_day(b.min_day) == start;
                odd_end = TimePoint::of_start_of_day(b.max_day + 1) == end;
            }
        }
        let mut paused = Vec::new();
        if kind == DrawKind::Regular {
            let mut set: std::collections::BTreeSet<Day> = g.all_paused(id).into_iter().collect();
            let plan = g.task_default_plan(id);
            let mut tmp = start;
            while tmp <= end {
                if is_zero_on_day(plan.as_ref(), tmp.to_day()) {
                    set.insert(tmp.to_day());
                }
                tmp = tmp.increment();
            }
            paused = set.into_iter().collect();
        }
        let (colors, completion, note, note_stereotype) = match task.as_impl() {
            Some(t) => (
                t.colors(),
                t.completion,
                t.note.clone(),
                t.note_stereotype.clone(),
            ),
            None => (None, 100, None, None),
        };
        let unstarted = ctx.style(&[SName::Task, SName::Unstarted]);
        let line_color = unlinear(
            &unstarted.value(PName::LineColor).as_color(),
            &style.value(PName::LineColor).as_color(),
            completion,
        );
        let back_color = unlinear(
            &unstarted.value(PName::BackGroundColor).as_color(),
            &style.value(PName::BackGroundColor).as_color(),
            completion,
        );
        let right_arrow = g
            .constraints_for_task(id)
            .iter()
            .any(|c| c.is_there_right_arrow(id));
        let mut draw = Draw {
            id,
            kind,
            y,
            row: task.row,
            hidden,
            start,
            end,
            odd_start,
            odd_end,
            paused,
            colors,
            completion,
            right_arrow,
            has_display_string: task.display.is_some(),
            margin: style.margin(),
            padding: style.padding(),
            style,
            title,
            fc,
            line_color,
            back_color,
            back_undone: ctx
                .style(&[SName::Undone])
                .value(PName::BackGroundColor)
                .as_color(),
            min_day: b.min_day,
            max_day: b.max_day,
            group_extra: 0.0,
            opale: None,
            note_dim: None,
            display_block: None,
        };
        if kind == DrawKind::Group {
            let dim = draw.title.calculate_dimension(ctx.sb.as_ref());
            let (pos1, pos2) = draw.group_pos(ts);
            if pos2 - pos1 > dim.width {
                draw.group_extra = 2.0;
            }
        }
        if matches!(kind, DrawKind::Regular | DrawKind::Diamond) {
            if let Some(n) = &note {
                let o = build_opale(ctx, n, note_stereotype.as_ref());
                draw.note_dim = Some(o.dimension(ctx.sb.as_ref()));
                draw.opale = Some(o);
            }
        }
        if let (DrawKind::Diamond, Some(text)) = (kind, &task.display) {
            draw.display_block = Some(ctx.block(text, &draw.fc));
        }
        draw
    }

    // ---- rows ----

    pub fn true_row<'a>(&self, draws: &'a [Draw]) -> Option<&'a Draw> {
        if self.kind == DrawKind::Separator {
            return None;
        }
        self.row.map(|r| &draws[r])
    }

    pub fn y_of(&self, draws: &[Draw]) -> Real {
        match self.true_row(draws) {
            None => self.y.clone(),
            Some(r) => r.y_of(draws),
        }
    }

    // ---- sizes ----

    fn title_dim(&self, sb: &dyn StringBounder) -> Dimension {
        self.title.calculate_dimension(sb)
    }

    fn diamond_height(&self) -> f64 {
        let mut result = self.fc.size2d().trunc();
        if (result as i64) % 2 == 1 {
            result -= 1.0;
        }
        result
    }

    fn group_pos(&self, ts: &TimeScale) -> (f64, f64) {
        (
            ts.position(self.start) + 6.0,
            ts.position(self.end) + ts.width(self.end) - 6.0,
        )
    }

    pub fn shape_height(&self, sb: &dyn StringBounder) -> f64 {
        match self.kind {
            DrawKind::Regular => self.padding.top + self.title_dim(sb).height + self.padding.bottom,
            DrawKind::Diamond => self.title_dim(sb).height.max(self.diamond_height()),
            DrawKind::Group => self.title_dim(sb).height + self.group_extra,
            DrawKind::Separator => self.title_dim(sb).height,
        }
    }

    /// Height of a row of the chart (`getFullHeightTask`).
    pub fn full_height_task(&self, sb: &dyn StringBounder) -> f64 {
        match self.kind {
            DrawKind::Separator => {
                self.margin.top
                    + self.padding.top
                    + self.title_dim(sb).height
                    + self.padding.bottom
                    + self.margin.bottom
            }
            _ => self.margin.top + self.shape_height(sb) + self.margin.bottom,
        }
    }

    fn y_note_position(&self, sb: &dyn StringBounder) -> f64 {
        self.full_height_task(sb)
    }

    pub fn height_max(&self, sb: &dyn StringBounder) -> f64 {
        match &self.note_dim {
            None => self.full_height_task(sb),
            Some(d) => self.y_note_position(sb) + d.height,
        }
    }

    pub fn finger_print(
        &self,
        sb: &dyn StringBounder,
        ts: &TimeScale,
        draws: &[Draw],
    ) -> FingerPrint {
        let y = self.y_of(draws).value();
        let h = self.full_height_task(sb);
        match self.kind {
            DrawKind::Regular => {
                let (s, e) = (ts.position(self.start), ts.position(self.end));
                FingerPrint {
                    x: s,
                    y,
                    width: e - s,
                    height: h,
                }
            }
            DrawKind::Diamond => {
                let s = ts.position(self.start);
                FingerPrint {
                    x: s,
                    y,
                    width: s + h,
                    height: y + h,
                }
            }
            DrawKind::Group => {
                let s = ts.position(self.start);
                let e = ts.position(self.end) + ts.width(self.end);
                FingerPrint {
                    x: s,
                    y,
                    width: e - s,
                    height: h,
                }
            }
            DrawKind::Separator => {
                let end = ts.position(TimePoint::of_start_of_day(self.max_day + 1));
                FingerPrint {
                    x: 0.0,
                    y,
                    width: end,
                    height: y + h,
                }
            }
        }
    }

    pub fn finger_print_note(
        &self,
        sb: &dyn StringBounder,
        ts: &TimeScale,
        draws: &[Draw],
    ) -> Option<FingerPrint> {
        if self.kind != DrawKind::Regular {
            return None;
        }
        let dim = self.note_dim?;
        Some(FingerPrint {
            x: ts.position(self.start),
            y: self.y_of(draws).value() + self.y_note_position(sb),
            width: dim.width,
            height: dim.height,
        })
    }

    pub fn label_overflow(&self, sb: &dyn StringBounder, ts: &TimeScale, bars_width: f64) -> f64 {
        let dim = self.title_dim(sb);
        let note_overflow = |overflow: f64| -> f64 {
            match self.note_dim {
                Some(o) => {
                    let ne = ts.position(self.start) + o.width;
                    if ne - bars_width > overflow {
                        ne - bars_width
                    } else {
                        overflow
                    }
                }
                None => overflow,
            }
        };
        match self.kind {
            DrawKind::Regular => {
                let mut overflow = 0.0;
                let pos1 = ts.position(self.start) + 6.0;
                let pos2 = ts.position(self.end) - 6.0;
                if pos2 - pos1 <= dim.width {
                    let label_end = self.out_position(pos2) + dim.width;
                    if label_end > bars_width {
                        overflow = label_end - bars_width;
                    }
                }
                note_overflow(overflow)
            }
            DrawKind::Diamond => {
                let x2 = ts.position(self.start) + ts.width(self.start);
                let width = self.diamond_height();
                let delta = x2 - ts.position(self.start) - width;
                let label_start = x2 - delta / 2.0 + self.padding.left;
                let label_end = label_start + dim.width;
                let overflow = if label_end > bars_width {
                    label_end - bars_width
                } else {
                    0.0
                };
                note_overflow(overflow)
            }
            DrawKind::Group => {
                let (pos1, pos2) = self.group_pos(ts);
                if pos2 - pos1 > dim.width {
                    return 0.0;
                }
                let label_end = pos2 + 6.0 + dim.width;
                if label_end > bars_width {
                    label_end - bars_width
                } else {
                    0.0
                }
            }
            DrawKind::Separator => 0.0,
        }
    }

    fn out_position(&self, pos2: f64) -> f64 {
        if self.right_arrow {
            pos2 + 18.0
        } else {
            pos2 + 8.0
        }
    }

    // ---- arrows ----

    pub fn y_side(&self, draws: &[Draw], sb: &dyn StringBounder, side: GSide) -> f64 {
        let y1 = self.margin.top + self.y_of(draws).value();
        let y2 = y1 + self.shape_height(sb);
        if self.kind == DrawKind::Diamond && !self.has_display_string {
            return y1 + self.diamond_height() / 2.0;
        }
        if side.is_top() {
            return y1;
        }
        if side.is_bottom() {
            return y2;
        }
        (y1 + y2) / 2.0
    }

    pub fn x_of(&self, ts: &TimeScale, side: GSide, arrow_type: ArrowType) -> f64 {
        match self.kind {
            DrawKind::Regular => {
                let mut x = match side {
                    GSide::Left => ts.position(self.start),
                    GSide::Right => ts.position(self.end),
                    GSide::TopLeft | GSide::BottomLeft => {
                        let x = ts.position(self.start) + 8.0;
                        if x > ts.position(self.end) {
                            (ts.position(self.start)
                                + ts.position(self.start)
                                + ts.width(self.start))
                                / 2.0
                        } else {
                            x
                        }
                    }
                    GSide::TopRight | GSide::BottomRight => ts.position(self.end) - 8.0,
                };
                if arrow_type == ArrowType::Outgoing {
                    if side == GSide::Left {
                        x += self.margin.left;
                    } else if side == GSide::Right {
                        x -= self.margin.left;
                    }
                }
                x
            }
            DrawKind::Diamond => {
                let mut x = match side {
                    GSide::Left => ts.position(self.start),
                    GSide::Right => ts.position(self.start) + ts.width(self.start),
                    _ => {
                        (ts.position(self.start) + ts.position(self.start) + ts.width(self.start))
                            / 2.0
                    }
                };
                if arrow_type == ArrowType::Outgoing {
                    let width = self.diamond_height();
                    if side == GSide::Left {
                        x += width / 2.0;
                    } else if side == GSide::Right {
                        x -= width / 2.0;
                    }
                }
                x
            }
            DrawKind::Group => match side {
                GSide::Left => ts.position(self.start),
                GSide::Right => ts.position(self.end) + ts.width(self.end),
                GSide::TopLeft | GSide::BottomLeft => {
                    (ts.position(self.start) + ts.position(self.start) + ts.width(self.start)) / 2.0
                }
                GSide::TopRight | GSide::BottomRight => {
                    (ts.position(self.end) + ts.position(self.end) + ts.width(self.end)) / 2.0
                }
            },
            DrawKind::Separator => 0.0,
        }
    }

    // ---- drawing ----

    fn apply_colors(&self, ug: &UGraphic) -> UGraphic {
        if let Some(col) = &self.colors {
            if let Some(center) = &col.center {
                let ug = ug.apply_back_color(center.clone());
                return ug.apply_color(col.border.clone().unwrap_or_else(|| center.clone()));
            }
        }
        ug.apply_color(self.line_color.clone())
            .apply_back_color(self.back_color.clone())
    }

    pub fn draw_title(&self, ug: &UGraphic, ts: &TimeScale) {
        let sb = ug.string_bounder();
        let dim = self.title_dim(sb);
        match self.kind {
            DrawKind::Regular => {
                let ug = ug.apply_translate(UTranslate::dy(self.margin.top + self.padding.top));
                let pos1 = ts.position(self.start) + 6.0;
                let pos2 = ts.position(self.end) - 6.0;
                let pos = if pos2 - pos1 > dim.width {
                    pos1
                } else {
                    self.out_position(pos2)
                };
                self.title.draw(&ug.apply_translate(UTranslate::dx(pos)));
            }
            DrawKind::Diamond => {
                let ug = ug.apply_translate(UTranslate::dy(self.margin.top));
                let h = (self.shape_height(sb) - dim.height) / 2.0;
                let x1 = ts.position(self.start);
                let x2 = x1 + ts.width(self.start);
                let width = self.diamond_height();
                let delta = x2 - x1 - width;
                let x = x2 - delta / 2.0 + self.padding.left;
                self.title.draw(&ug.apply_translate(UTranslate::new(x, h)));
            }
            DrawKind::Group => {
                let (pos1, pos2) = self.group_pos(ts);
                let (pos, y) = if pos2 - pos1 > dim.width {
                    (pos1 + (pos2 - pos1 - dim.width) / 2.0, 0.0)
                } else {
                    (pos2 + 6.0, self.full_height_task(sb) - dim.height)
                };
                self.title
                    .draw(&ug.apply_translate(UTranslate::new(pos, y)));
            }
            DrawKind::Separator => {
                let dx = self.margin.left + self.padding.left;
                let dy = self.margin.top + self.padding.top;
                self.title
                    .draw(&ug.apply_translate(UTranslate::new(dx, dy)));
            }
        }
    }

    pub fn draw_u(&self, ug: &UGraphic, ts: &TimeScale) {
        let sb = ug.string_bounder();
        match self.kind {
            DrawKind::Regular => {
                let start_pos = ts.position(self.start);
                self.draw_note(
                    &ug.apply_translate(UTranslate::new(start_pos, self.y_note_position(sb))),
                );
                self.draw_regular_shape(ug, ts, sb);
            }
            DrawKind::Diamond => {
                let x1 = ts.position(self.start);
                self.draw_note(&ug.apply_translate(UTranslate::new(x1, self.y_note_position(sb))));
                let ug = ug
                    .apply_translate(UTranslate::dy(self.margin.top))
                    .apply_translate(UTranslate::dx(x1));
                match &self.display_block {
                    None => {
                        let x2 = x1 + ts.width(self.start);
                        let width = self.diamond_height();
                        let delta = x2 - x1 - width;
                        let ug = ug.apply_translate(UTranslate::dx(delta / 2.0));
                        let h = width;
                        let mut poly = UPolygon::new(Vec::new());
                        poly.add_point(h / 2.0, 0.0);
                        poly.add_point(h, h / 2.0);
                        poly.add_point(h / 2.0, h);
                        poly.add_point(0.0, h / 2.0);
                        self.apply_colors(&ug).draw(poly);
                    }
                    Some(block) => block.draw(&ug),
                }
            }
            DrawKind::Group => self.draw_group_shape(ug, ts, sb),
            DrawKind::Separator => self.draw_separator(ug, ts, sb),
        }
    }

    fn draw_note(&self, ug: &UGraphic) {
        if let Some(o) = &self.opale {
            o.draw(ug);
        }
    }

    fn draw_group_shape(&self, ug: &UGraphic, ts: &TimeScale, sb: &dyn StringBounder) {
        let start_pos = ts.position(self.start) + self.margin.left;
        let end_pos = ts.position(self.end) + ts.width(self.end) - self.margin.right;
        let height = 10.0;
        let ug = ug
            .apply_translate(UTranslate::dy(self.full_height_task(sb) - height))
            .apply_color(HColor::BLACK)
            .apply_back_color(HColor::BLACK);
        let thick = 2.0;
        let y1 = (height - thick) / 2.0;
        let y2 = height - (height - thick) / 2.0;
        let dx = 6.0;
        let mut rect = UPath::new();
        rect.move_to(start_pos, 0.0);
        rect.line_to(start_pos + dx, y1);
        rect.line_to(end_pos - dx, y1);
        rect.line_to(end_pos, 0.0);
        rect.line_to(end_pos, height);
        rect.line_to(end_pos - dx, y2);
        rect.line_to(start_pos + dx, y2);
        rect.line_to(start_pos, height);
        rect.line_to(start_pos, 0.0);
        rect.close_path();
        ug.draw(rect);
    }

    fn draw_separator(&self, ug: &UGraphic, ts: &TimeScale, sb: &dyn StringBounder) {
        let width_title = self.title_dim(sb).width;
        let start = ts.position(TimePoint::of_start_of_day(self.min_day));
        let end = ts.position(TimePoint::of_start_of_day(self.max_day + 1));
        let ug = ug.apply_translate(UTranslate::new(0.0, self.margin.top));
        let back = self.style.value(PName::BackGroundColor).as_color();
        let text_height = self.title_dim(sb).height;
        if !back.is_transparent() {
            let height = self.padding.top + text_height + self.padding.bottom;
            if height > 0.0 {
                ug.apply_back_color(back)
                    .draw(URectangle::build(end - start, height));
            }
        }
        let line_color = self.style.value(PName::LineColor).as_color();
        let ug = ug
            .apply_color(line_color)
            .apply_translate(UTranslate::dy(self.padding.top + text_height / 2.0));
        if width_title == 0.0 {
            ug.draw(ULine::hline(end - start));
        } else {
            if self.padding.left > 1.0 {
                ug.draw(ULine::hline(self.padding.left));
            }
            let x1 = self.padding.left + self.margin.left + width_title + self.margin.right;
            let x2 = end - 1.0;
            ug.apply_translate(UTranslate::dx(x1))
                .draw(ULine::hline(x2 - x1));
        }
    }

    fn draw_regular_shape(&self, ug: &UGraphic, ts: &TimeScale, sb: &dyn StringBounder) {
        let ug = self.apply_colors(ug);
        let start_pos = ts.position(self.start) + self.margin.left;
        let end_pos = ts.position(self.end) - self.margin.right;
        let ug = ug.apply_translate(UTranslate::dy(self.margin.top));
        let round = self.style.value(PName::RoundCorner).as_double();
        let off: Vec<Seg> = self
            .paused
            .iter()
            .map(|day| {
                let pause = TimePoint::of_start_of_day(*day);
                let x1 = ts.position(pause);
                Seg {
                    pos1: x1,
                    pos2: x1 + ts.width(pause),
                }
            })
            .collect();
        let rect = RectangleTask::new(start_pos, end_pos, round, self.completion, &off);
        rect.draw(
            &ug,
            self.shape_height(sb),
            &self.back_undone,
            self.odd_start,
            self.odd_end,
        );
    }
}

/// The bar of a task, cut where the task pauses (`RectangleTask`).
struct RectangleTask {
    segments: Vec<Seg>,
    round: f64,
    completion: i32,
}

fn utoright(width: f64, height: f64, round: f64) -> UPath {
    let mut p = UPath::new();
    if round == 0.0 {
        p.move_to(0.0, 0.0);
        p.line_to(width, 0.0);
        p.line_to(width, height);
        p.line_to(0.0, height);
    } else {
        let half = round / 2.0;
        p.move_to(0.0, 0.0);
        p.line_to(width - half, 0.0);
        p.arc_to(half, half, 0.0, false, true, width, half);
        p.line_to(width, height - half);
        p.arc_to(half, half, 0.0, false, true, width - half, height);
        p.line_to(0.0, height);
    }
    p
}

fn utoleft(width: f64, height: f64, round: f64) -> UPath {
    let mut p = UPath::new();
    if round == 0.0 {
        p.move_to(width, height);
        p.line_to(0.0, height);
        p.line_to(0.0, 0.0);
        p.line_to(width, 0.0);
    } else {
        let half = round / 2.0;
        p.move_to(width, height);
        p.line_to(half, height);
        p.arc_to(half, half, 0.0, false, true, 0.0, height - half);
        p.line_to(0.0, half);
        p.arc_to(half, half, 0.0, false, true, half, 0.0);
        p.line_to(width, 0.0);
    }
    p
}

impl RectangleTask {
    fn new(start_pos: f64, end_pos: f64, round: f64, completion: i32, paused: &[Seg]) -> Self {
        let segments = if start_pos < end_pos {
            Seg {
                pos1: start_pos,
                pos2: end_pos,
            }
            .cut(paused)
        } else {
            vec![Seg {
                pos1: start_pos,
                pos2: start_pos + 1.0,
            }]
        };
        Self {
            segments,
            round,
            completion,
        }
    }

    fn draw(
        &self,
        ug: &UGraphic,
        height: f64,
        document_background: &HColor,
        odd_start: bool,
        odd_end: bool,
    ) {
        if self.round == 0.0 {
            self.draw_without_round(ug, height, document_background, odd_start, odd_end);
            return;
        }
        if self.segments.len() != 1 {
            self.draw_with_round(ug, height, document_background);
            return;
        }
        let segment = self.segments[0];
        let width = segment.length();
        let partial = URectangle::build(width, height).rounded(self.round);
        let completion = self.completion;
        if completion == 100 || completion == 0 {
            let ug = if completion == 0 {
                ug.apply_back_color(document_background.clone())
            } else {
                ug.clone()
            };
            let at = ug.apply_translate(UTranslate::dx(segment.pos1));
            if odd_start && !odd_end {
                at.draw(utoright(width, height, self.round));
            } else if !odd_start && odd_end {
                at.draw(utoleft(width, height, self.round));
            } else {
                at.draw(partial);
            }
        } else {
            let x1 = width * f64::from(completion) / 100.0;
            ug.apply_color(HColor::TRANSPARENT)
                .apply_translate(UTranslate::dx(segment.pos1))
                .draw(utoleft(x1, height, self.round));
            ug.apply_back_color(document_background.clone())
                .apply_color(HColor::TRANSPARENT)
                .apply_translate(UTranslate::dx(segment.pos1 + x1))
                .draw(utoright(
                    width * f64::from(100 - completion) / 100.0,
                    height,
                    self.round,
                ));
            ug.apply_back_color(HColor::TRANSPARENT)
                .apply_translate(UTranslate::dx(segment.pos1))
                .draw(partial);
        }
    }

    fn draw_with_round(&self, ug: &UGraphic, height: f64, document_background: &HColor) {
        let first = self.segments[0];
        ug.apply_translate(UTranslate::dx(first.pos1)).draw(utoleft(
            first.length(),
            height,
            self.round,
        ));
        for i in 1..self.segments.len() - 1 {
            let seg = self.segments[i];
            self.draw_partly(
                seg.length() * f64::from(self.completion) / 100.0,
                ug,
                seg,
                height,
                document_background,
                i,
                false,
                false,
            );
        }
        let last = self.segments[self.segments.len() - 1];
        ug.apply_translate(UTranslate::dx(last.pos1)).draw(utoright(
            last.length(),
            height,
            self.round,
        ));
        self.draw_intermediate_dotted(ug, height);
    }

    fn full_length(&self) -> f64 {
        self.segments.iter().map(Seg::length).sum()
    }

    fn draw_without_round(
        &self,
        ug: &UGraphic,
        height: f64,
        document_background: &HColor,
        odd_start: bool,
        odd_end: bool,
    ) {
        let sum = self.full_length();
        let lim = if self.completion == 100 {
            sum
        } else {
            sum * f64::from(self.completion) / 100.0
        };
        if self.segments.len() == 1 && !odd_start && !odd_end {
            self.draw_full(
                if self.completion == 100 { -1.0 } else { lim },
                ug,
                self.segments[0],
                height,
                document_background,
            );
            return;
        }
        let mut current = 0.0;
        for (i, segment) in self.segments.iter().enumerate() {
            let next = current + segment.length();
            let width_completion = if lim >= next {
                -1.0
            } else if current >= lim {
                0.0
            } else {
                lim - current
            };
            self.draw_partly(
                width_completion,
                ug,
                *segment,
                height,
                document_background,
                i,
                !odd_start && i == 0,
                !odd_end && i == self.segments.len() - 1,
            );
            current = next;
        }
        self.draw_intermediate_dotted(ug, height);
    }

    fn draw_intermediate_dotted(&self, ug: &UGraphic, height: f64) {
        let ug = ug.apply_stroke(UStroke::new(2.0, 3.0, 1.0));
        for i in 0..self.segments.len().saturating_sub(1) {
            let v1 = self.segments[i].pos2 + 3.0;
            let v2 = self.segments[i + 1].pos1 - 3.0;
            if v2 > v1 {
                let at = ug.apply_translate(UTranslate::dx(v1));
                let hline = ULine::hline(v2 - v1);
                at.draw(hline.clone());
                at.apply_translate(UTranslate::dy(height)).draw(hline);
            }
        }
    }

    fn draw_full(
        &self,
        width_completion: f64,
        ug: &UGraphic,
        segment: Seg,
        height: f64,
        document_background: &HColor,
    ) {
        let width = segment.length();
        let ug = ug.apply_translate(UTranslate::dx(segment.pos1));
        self.draw_background_rect(
            width_completion,
            &ug.apply_color(HColor::TRANSPARENT),
            document_background,
            width,
            height,
        );
        ug.apply_back_color(HColor::TRANSPARENT)
            .draw(URectangle::build(width, height));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_partly(
        &self,
        width_completion: f64,
        ug: &UGraphic,
        segment: Seg,
        height: f64,
        document_background: &HColor,
        i: usize,
        with_start_vline: bool,
        with_end_vline: bool,
    ) {
        let mut width_back = segment.length();
        if i != self.segments.len() - 1 {
            width_back += 1.0;
        }
        let ug = ug.apply_translate(UTranslate::dx(segment.pos1));
        if width_back > 0.0 {
            self.draw_background_rect(
                width_completion,
                &ug.apply_color(HColor::TRANSPARENT),
                document_background,
                width_back,
                height,
            );
        }
        let ug = ug.apply_back_color(HColor::TRANSPARENT);
        if with_start_vline {
            ug.draw(utoleft(segment.length(), height, 0.0));
        } else if with_end_vline {
            ug.draw(utoright(segment.length(), height, 0.0));
        } else {
            let hline = ULine::hline(segment.length());
            ug.draw(hline.clone());
            ug.apply_translate(UTranslate::dy(height)).draw(hline);
        }
    }

    fn draw_background_rect(
        &self,
        width_completion: f64,
        ug: &UGraphic,
        document_background: &HColor,
        width: f64,
        height: f64,
    ) {
        if width_completion == -1.0 || width_completion == 0.0 {
            let ug = if width_completion == 0.0 {
                ug.apply_back_color(document_background.clone())
            } else {
                ug.clone()
            };
            ug.draw(URectangle::build(width, height));
        } else {
            ug.draw(URectangle::build(width_completion, height));
            ug.apply_back_color(document_background.clone())
                .apply_translate(UTranslate::dx(width_completion))
                .draw(URectangle::build(width - width_completion, height));
        }
    }
}
