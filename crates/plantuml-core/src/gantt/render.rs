//! Laying out and drawing the whole chart (`GanttDiagramMainBlock`, `GanttLayout`,
//! `TaskDrawRegistryData`, `GanttTaskTable`).

use super::ctx::{font_configuration, Ctx, TimelineStyle};
use super::header::Header;
use super::i18n;
use super::model::{ArrowType, Attr, GSide, GanttConstraint, Kind};
use super::taskdraw::Draw;
use super::time::{Day, TimePoint};
use super::{Bounds, Column, GanttDiagram};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    UGraphic, ULine, UPath, UPolygon, URectangle, UStroke, UTranslate,
};
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::shape::TextBlock;
use crate::real::{self, Real};
use crate::skin::SkinParam;
use crate::style::{PName, SName};
use std::rc::Rc;

/// Everything about one resource's row of loads at the bottom.
struct ResourceDraw {
    name: String,
    y: f64,
    /// Each day with the load of the resource on it.
    loads: Vec<(TimePoint, i32)>,
}

struct TableCell {
    texts: Vec<String>,
}

/// The table of tasks on the left (`GanttTaskTable`).
struct Table {
    edges: Vec<f64>,
    header_height: f64,
    headers: Vec<String>,
    rows: Vec<(usize, TableCell)>,
}

const CELL_PADDING: f64 = 5.0;

pub struct GanttBlock {
    ctx: Rc<Ctx>,
    st: Rc<TimelineStyle>,
    header: Header,
    draws: Vec<Draw>,
    constraints: Vec<ArrowData>,
    resources: Vec<ResourceDraw>,
    table: Table,
    b: Bounds,
    show_footbox: bool,
    hide_resource_footbox: bool,
    total_height_without_footer: f64,
    titles_width: f64,
    bars_width: f64,
    timeline_width: f64,
    total_height: f64,
    header_height: f64,
    footer_height: f64,
}

/// An arrow between two bars, with what is needed to draw it.
struct ArrowData {
    source_task: Option<usize>,
    dest_task: Option<usize>,
    source_attr: Attr,
    dest_attr: Attr,
    same_row: bool,
    style: crate::style::Style,
    hidden: bool,
}

/// Builds the block drawing the whole chart (`getTextBlock`).
pub fn main_block(
    g: &GanttDiagram,
    sb: &Rc<dyn StringBounder>,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    let ctx = Rc::new(Ctx::new(skin, sb));
    let st = Rc::new(TimelineStyle::new(&ctx));
    let b = g.bounds();
    let header = Header::create(g, b, ctx.clone(), st.clone());
    let sbr: &dyn StringBounder = sb.as_ref();

    // buildTaskAndResourceDraws
    let header_total = header.full_header_height();
    let origin = real::create_origin();
    let mut y: Real = origin.add_fixed(header_total);
    let mut draws: Vec<Draw> = Vec::new();
    for id in 0..g.tasks.len() {
        let draw = Draw::build(g, id, &b, &ctx, &header.ts, y.clone());
        if g.tasks[id].row.is_none() {
            y = y.add_at_least(draw.full_height_task(sbr));
        }
        draws.push(draw);
    }
    origin.compile_now();
    resolve_note_overlaps(&draws, &origin, sbr, &header.ts);
    let ts = header.ts.clone();

    let mut bottom = 0.0f64;
    for d in &draws {
        bottom = bottom.max(d.y_of(&draws).value() + d.height_max(sbr));
    }
    let mut yy = bottom;
    let mut resources = Vec::new();
    if yy == 0.0 {
        yy = header_total;
    } else if !g.hide_resource_footbox {
        for r in &g.resources {
            let mut loads = Vec::new();
            let mut i = TimePoint::of_start_of_day(b.min_day);
            let max = TimePoint::of_end_of_day_minus_one_second(b.max_day);
            while i <= max {
                loads.push((i, g.load_for_resource_total(&r.name, i)));
                i = i.increment();
            }
            resources.push(ResourceDraw {
                name: r.name.clone(),
                y: yy,
                loads,
            });
            yy += 16.0 * 2.0;
        }
    }
    let total_height_without_footer = yy;

    // GanttLayout
    let timeline_width = {
        let xmin = ts.position(TimePoint::of_start_of_day(b.min_day));
        let xmax = ts.position(TimePoint::of_start_of_day(b.max_day).add_days(1));
        xmax - xmin
    };
    let mut max_overflow = 0.0f64;
    for (i, d) in draws.iter().enumerate() {
        if g.is_hidden(i, &b) {
            continue;
        }
        max_overflow = max_overflow.max(d.label_overflow(sbr, &ts, timeline_width));
    }
    let bars_width = timeline_width + max_overflow;
    let header_height = header.time_header_height();
    let footer_height = if g.show_footbox {
        header.time_footer_height()
    } else {
        0.0
    };
    let total_height = total_height_without_footer + footer_height;

    let constraints = g
        .constraints
        .iter()
        .map(|c| arrow_data(g, c, &b, &ctx))
        .collect();
    let table = build_table(g, &ctx, &st, &draws, &b, header_total, sbr);

    Rc::new(GanttBlock {
        ctx,
        st,
        header,
        draws,
        constraints,
        resources,
        table,
        b,
        show_footbox: g.show_footbox,
        hide_resource_footbox: g.hide_resource_footbox,
        total_height_without_footer,
        titles_width: 0.0,
        bars_width,
        timeline_width,
        total_height,
        header_height,
        footer_height,
    })
}

fn resolve_note_overlaps(
    draws: &[Draw],
    origin: &Real,
    sb: &dyn StringBounder,
    ts: &super::timescale::TimeScale,
) {
    let mut notes: Vec<usize> = Vec::new();
    for (i, td) in draws.iter().enumerate() {
        let task_print = td.finger_print(sb, ts, draws);
        let note_print = td.finger_print_note(sb, ts, draws);
        if td.true_row(draws).is_none() {
            for &n in &notes {
                let other = draws[n].finger_print_note(sb, ts, draws);
                let delta = match (&other, &Some(task_print)) {
                    (Some(o), Some(t)) => o.overlap(t),
                    _ => 0.0,
                };
                if delta > 0.0 {
                    let bottom = draws[n].y_of(draws).add_at_least(draws[n].height_max(sb));
                    td.y_of(draws).ensure_bigger_than(&bottom);
                    origin.compile_now();
                }
            }
        }
        if note_print.is_some() {
            notes.push(i);
        }
    }
}

fn arrow_data(g: &GanttDiagram, c: &GanttConstraint, b: &Bounds, ctx: &Ctx) -> ArrowData {
    let mut style = ctx.style(&[SName::Arrow]);
    if let Some(color) = &c.forced_color {
        style = style.override_color(PName::LineColor, Some(color.clone()));
    }
    let stroke = c.style.stroke_over(style.stroke());
    style = style.override_stroke(&stroke);
    let hidden = b.printing && {
        let min = TimePoint::of_start_of_day(b.min_day);
        let max = TimePoint::of_end_of_day_minus_one_second(b.max_day);
        [&c.source, &c.dest].iter().any(|i| {
            let p = g.instant_precise(i);
            p < min || p > max
        })
    };
    ArrowData {
        source_task: c.source.task(),
        dest_task: c.dest.task(),
        source_attr: c.source.attribute,
        dest_attr: c.dest.attribute,
        same_row: g.same_row_as(&c.source, &c.dest),
        style,
        hidden,
    }
}

fn relative_day_num(point: TimePoint, min_day: Day) -> i64 {
    point.absolute_day_num() - TimePoint::of_start_of_day(min_day).absolute_day_num() + 1
}

fn build_table(
    g: &GanttDiagram,
    ctx: &Ctx,
    st: &TimelineStyle,
    draws: &[Draw],
    b: &Bounds,
    header_height: f64,
    sb: &dyn StringBounder,
) -> Table {
    let lang = &g.lang;
    let relative = g.is_relative_mode();
    let format_day = |p: TimePoint| -> String {
        if relative {
            i18n::day_number(lang, relative_day_num(p, g.min_day))
        } else {
            super::time::TimePointFormat::MonthAndDay.format(p, lang)
        }
    };
    let with_time = |p: TimePoint| -> String {
        let day = format_day(p);
        let (h, m, s) = (p.hour(), p.minute(), p.second());
        if h == 0 && m == 0 && s == 0 {
            return day;
        }
        if s == 0 {
            format!("{day} {h:02}:{m:02}")
        } else {
            format!("{day} {h:02}:{m:02}:{s:02}")
        }
    };
    let format_end = |p: TimePoint| -> String {
        if p.seconds_of_day() == 0 {
            format_day(p.minus_one_second())
        } else {
            with_time(p)
        }
    };
    let fc = &st.text_fc;
    let columns = g.displayed_columns.clone();
    let header_of = |c: Column| -> String {
        match c {
            Column::Task => i18n::task(lang),
            Column::Start => i18n::start(lang),
            Column::End => i18n::end(lang),
            Column::Duration => i18n::duration(lang),
        }
        .to_string()
    };
    let headers: Vec<String> = columns.iter().map(|c| header_of(*c)).collect();
    let mut rows = Vec::new();
    for (id, task) in g.tasks.iter().enumerate() {
        if !matches!(task.kind, Kind::Impl(_)) || g.is_hidden(id, b) || draws.get(id).is_none() {
            continue;
        }
        let (start, end) = (g.task_start(id), g.task_end(id));
        let texts = columns
            .iter()
            .map(|c| match c {
                Column::Task => task.code.display.clone(),
                Column::Start => with_time(start),
                Column::End => format_end(end),
                Column::Duration => i18n::duration_human_readable(lang, end.secs() - start.secs()),
            })
            .collect();
        rows.push((id, TableCell { texts }));
    }
    let width_of = |text: &str| ctx.block(text, fc).calculate_dimension(sb).width;
    let mut edges = vec![0.0];
    for (i, c) in columns.iter().enumerate() {
        let mut w = width_of(&headers[i]);
        for (_, row) in &rows {
            w = w.max(width_of(&row.texts[i]));
        }
        let _ = c;
        let last = *edges.last().expect("edge");
        edges.push(last + w + 2.0 * CELL_PADDING);
    }
    Table {
        edges,
        header_height,
        headers,
        rows,
    }
}

impl Table {
    fn width(&self) -> f64 {
        *self.edges.last().expect("edge")
    }
}

impl GanttBlock {
    fn draw_table(&self, ug: &UGraphic, total: f64) {
        let t = &self.table;
        let fc = &self.st.text_fc;
        let line = ug.apply_color(self.st.line_color.clone());
        let width = t.width();
        line.draw(ULine::hline(width));
        line.apply_translate(UTranslate::dy(t.header_height))
            .draw(ULine::hline(width));
        line.apply_translate(UTranslate::dy(total))
            .draw(ULine::hline(width));
        for x in &t.edges {
            line.apply_translate(UTranslate::dx(*x))
                .draw(ULine::vline(total));
        }
        let sb = ug.string_bounder();
        let cell = |x_left: f64, y_center: f64, text: &str| {
            let block = self.ctx.block(text, fc);
            let dim = block.calculate_dimension(sb);
            block.draw(&ug.apply_translate(UTranslate::new(
                x_left + CELL_PADDING,
                y_center - dim.height / 2.0,
            )));
        };
        for (i, h) in t.headers.iter().enumerate() {
            cell(t.edges[i], t.header_height / 2.0, h);
        }
        for (id, row) in &t.rows {
            let d = &self.draws[*id];
            let y_center = d.y_of(&self.draws).value() + d.full_height_task(sb) / 2.0;
            for (i, text) in row.texts.iter().enumerate() {
                cell(t.edges[i], y_center, text);
            }
        }
    }

    fn draw_arrow(&self, ug: &UGraphic, a: &ArrowData) {
        let sb = ug.string_bounder();
        let style = &a.style;
        let ug = ug
            .apply_stroke(style.stroke())
            .apply_color(style.value(PName::LineColor).as_color());
        let (Some(st), Some(dt)) = (a.source_task, a.dest_task) else {
            return;
        };
        let (start, end) = (&self.draws[st], &self.draws[dt]);
        let ts = &self.header.ts;
        let (at_start, at_end) = match (a.source_attr, a.dest_attr) {
            (Attr::End, Attr::Start) => (
                if a.same_row {
                    GSide::Right
                } else {
                    GSide::BottomRight
                },
                GSide::Left,
            ),
            (Attr::End, Attr::End) => (GSide::Right, GSide::Right),
            (Attr::Start, Attr::Start) => (GSide::Left, GSide::Left),
            _ => (
                if a.same_row {
                    GSide::Left
                } else {
                    GSide::BottomLeft
                },
                GSide::Right,
            ),
        };
        let mut x1 = start.x_of(ts, at_start, ArrowType::Outgoing);
        let mut y1 = start.y_side(&self.draws, sb, at_start);
        let x2 = end.x_of(ts, at_end, ArrowType::Incoming);
        let y2 = end.y_side(&self.draws, sb, at_end);
        if at_start.is_bottom() && y2 < y1 {
            y1 = start.y_side(&self.draws, sb, at_start.reverse_bottom_top());
        }
        let minimal_width = 6.0;
        let mut points: Vec<(f64, f64)> = Vec::new();
        if at_start.is_bottom() && at_end == GSide::Left {
            if x2 > x1 + minimal_width {
                points.extend([(x1, y1), (x1, y2), (x2, y2)]);
            } else {
                x1 = start.x_of(ts, GSide::Right, ArrowType::Outgoing);
                y1 = start.y_side(&self.draws, sb, GSide::Right);
                let y1b = end.y_of(&self.draws).value();
                points.extend([
                    (x1, y1),
                    (x1 + 6.0, y1),
                    (x1 + 6.0, y1b),
                    (x2 - 8.0, y1b),
                    (x2 - 8.0, y2),
                    (x2, y2),
                ]);
            }
        } else if at_start == GSide::Right && at_end == GSide::Right {
            let xmax = x1.max(x2) + 8.0;
            points.extend([(x1, y1), (xmax, y1), (xmax, y2), (x2, y2)]);
        } else if at_start == GSide::Left && at_end == GSide::Left {
            let xmin = x1.min(x2) - 8.0;
            points.extend([(x1, y1), (xmin, y1), (xmin, y2), (x2, y2)]);
        } else {
            points.extend([(x1, y1), (x1, y2), (x2, y2)]);
        }
        draw_garrows(
            &ug,
            &points,
            at_end,
            style.value(PName::LineColor).as_color(),
        );
    }

    fn draw_resource(&self, ug: &UGraphic, r: &ResourceDraw) {
        let sb = ug.string_bounder();
        let ug = ug.apply_translate(UTranslate::dy(r.y));
        let title_fc = resource_fc(13, HColor::BLACK);
        let title = self.ctx.block(&r.name, &title_fc);
        title.draw(&ug);
        let ts = &self.header.ts;
        let min = TimePoint::of_start_of_day(self.b.min_day);
        let max = TimePoint::of_end_of_day_minus_one_second(self.b.max_day);
        let line = ULine::hline(ts.position(max) - ts.position(min));
        ug.apply_color(HColor::BLACK)
            .apply_translate(UTranslate::dy(title.calculate_dimension(sb).height))
            .draw(line);
        let mut starting = -1.0;
        let mut total_load = 0;
        let mut active_days = 0;
        let mut is_red = false;
        for (i, load) in &r.loads {
            let breaking = ts.is_breaking(*i);
            if *load > 100 {
                is_red = true;
            }
            total_load += load;
            if *load > 0 {
                active_days += 1;
            }
            if breaking {
                if total_load > 0 {
                    let shown = (f64::from(total_load) / f64::from(active_days)).round() as i32;
                    let color = if is_red {
                        HColor::parse_or_white("red")
                    } else {
                        HColor::BLACK
                    };
                    let value = self.ctx.block(&shown.to_string(), &resource_fc(9, color));
                    if starting == -1.0 {
                        starting = ts.position(*i);
                    }
                    let ending = ts.position(*i) + ts.width(*i);
                    let start =
                        (starting + ending) / 2.0 - value.calculate_dimension(sb).width / 2.0;
                    value.draw(&ug.apply_translate(UTranslate::new(start, 16.0)));
                }
                starting = -1.0;
                total_load = 0;
                active_days = 0;
                is_red = false;
            } else if starting == -1.0 {
                starting = ts.position(*i);
            }
        }
    }
}

fn resource_fc(size: i32, color: HColor) -> crate::klimt::font::config::FontConfiguration {
    font_configuration(UFont::serif(size), false, color)
}

/// `GArrows`: a polyline with an arrow head.
fn draw_garrows(ug: &UGraphic, points: &[(f64, f64)], at_end: GSide, back: HColor) {
    let mut path = UPath::new();
    path.move_to(points[0].0, points[0].1);
    for p in &points[1..points.len() - 1] {
        path.line_to(p.0, p.1);
    }
    let last = points[points.len() - 1];
    let patched = match at_end {
        GSide::Left => (last.0 - 3.0, last.1),
        GSide::Right => (last.0 + 3.0, last.1),
        _ => last,
    };
    path.line_to(patched.0, patched.1);
    path.close_path();
    ug.draw(path);
    let ug = ug.apply_stroke(UStroke::simple()).apply_back_color(back);
    let (x, y) = last;
    let delta2 = 4.0;
    let mut head = UPolygon::new(Vec::new());
    match at_end {
        GSide::Left => {
            head.add_point(x - 4.0, y - delta2);
            head.add_point(x, y);
            head.add_point(x - 4.0, y + delta2);
            head.add_point(x - 4.0, y - delta2);
        }
        _ => {
            head.add_point(x + 4.0, y - delta2);
            head.add_point(x, y);
            head.add_point(x + 4.0, y + delta2);
            head.add_point(x + 4.0, y - delta2);
        }
    }
    ug.draw(head);
}

impl TextBlock for GanttBlock {
    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> Dimension {
        let margin = 20.0;
        Dimension::new(
            self.table.width() + self.titles_width + self.bars_width + margin,
            self.total_height,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let _sb = ug.string_bounder();
        let table_width = self.table.width();
        self.draw_table(ug, self.total_height_without_footer);
        let ug = ug.apply_translate(UTranslate::dx(table_width));
        let ug_orig = ug.clone();
        let back = self.st.back.clone();
        if !back.is_transparent() {
            let full_width = self.titles_width + self.timeline_width;
            ug.apply_back_color(back.clone())
                .draw(URectangle::build(full_width, self.header_height));
            if self.show_footbox {
                ug.apply_back_color(back)
                    .apply_translate(UTranslate::dy(self.total_height_without_footer))
                    .draw(URectangle::build(full_width, self.footer_height));
            }
        }
        self.header
            .draw_time_header(&ug, self.total_height_without_footer);
        for a in &self.constraints {
            if a.hidden {
                continue;
            }
            self.draw_arrow(&ug, a);
        }
        // task bars
        for (i, d) in self.draws.iter().enumerate() {
            if d.hidden {
                continue;
            }
            let _ = i;
            let move_y = d.y_of(&self.draws).value();
            d.draw_u(&ug.apply_translate(UTranslate::dy(move_y)), &self.header.ts);
        }
        // task titles
        for d in &self.draws {
            if d.hidden {
                continue;
            }
            let move_y = d.y_of(&self.draws).value();
            d.draw_title(
                &ug_orig.apply_translate(UTranslate::dy(move_y)),
                &self.header.ts,
            );
        }
        if !self.hide_resource_footbox {
            for r in &self.resources {
                self.draw_resource(&ug, r);
            }
        }
        if self.show_footbox {
            self.header.draw_time_footer(
                &ug.apply_translate(UTranslate::dy(self.total_height_without_footer)),
            );
        }
    }
}
