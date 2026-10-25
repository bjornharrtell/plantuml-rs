//! Renderers of the bar, line, area and scatter series of a chart.

use super::{color_of, draw_value_label, merged, series_style, text_block};
use crate::chart::{ChartAxis, ChartSeries, MarkerShape};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPath, UPolygon, URectangle, UStroke, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{FontFace, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::rc::Rc;

/// Vertical position of `value` on a plot of the given height.
fn value_y(value: f64, axis: &ChartAxis, plot_height: f64) -> f64 {
    plot_height - (value - axis.min) / (axis.max - axis.min) * plot_height
}

/// Pixel of the zero line, kept inside the plot (`zeroY` of vertical bars).
fn clamped_zero_y(axis: &ChartAxis, plot_height: f64) -> f64 {
    0.0_f64
        .max(plot_height.min(plot_height - (0.0 - axis.min) / (axis.max - axis.min) * plot_height))
}

/// Bold 10pt font of the value labels of bars, lines and markers.
fn label_font() -> UFont {
    UFont::sans_serif(10).with_face(FontFace::BOLD)
}

fn bar_width_ratio(style: &Style) -> f64 {
    let ratio = style.value(PName::BarWidth).as_double();
    if ratio > 0.0 && ratio <= 1.0 {
        ratio
    } else {
        0.6
    }
}

/// Draws one bar series, or a group of them (`BarRenderer`).
pub(super) struct BarRenderer {
    pub(super) skin: Rc<SkinParam>,
    pub(super) plot_width: f64,
    pub(super) plot_height: f64,
    pub(super) category_count: usize,
    pub(super) axis: ChartAxis,
    pub(super) horizontal: bool,
}

impl BarRenderer {
    fn series_bar_style(&self, s: &ChartSeries) -> Style {
        series_style(&self.skin, &queries::CHARTDIAG_BAR, s.stereotype.as_ref())
    }

    pub(super) fn draw(&self, ug: &UGraphic, s: &ChartSeries, color: &HColor) {
        if self.horizontal {
            self.draw_horizontal(ug, s, color);
        } else {
            self.draw_vertical(ug, s, color);
        }
    }

    /// Top and height of the bar of a value, measured from the zero line.
    fn bar_geometry(&self, value: f64) -> (f64, f64) {
        let zero_y = clamped_zero_y(&self.axis, self.plot_height);
        let value_y = value_y(value, &self.axis, self.plot_height);
        if value < 0.0 {
            (zero_y, value_y - zero_y)
        } else {
            (value_y, zero_y - value_y)
        }
    }

    fn draw_vertical(&self, ug: &UGraphic, s: &ChartSeries, color: &HColor) {
        if self.category_count == 0 {
            return;
        }
        let category_width = self.plot_width / self.category_count as f64;
        let style = self.series_bar_style(s);
        let bar_width = category_width * bar_width_ratio(&style);
        let bar_offset = (category_width - bar_width) / 2.0;
        let line_color = color_of(&style, PName::LineColor);
        let stroke = UStroke::with_thickness(style.value(PName::LineThickness).as_double());
        for (i, value) in s.values.iter().take(self.category_count).enumerate() {
            let Some(value) = *value else { continue };
            let x = i as f64 * category_width + bar_offset;
            let (y, bar_height) = self.bar_geometry(value);
            if bar_height > 0.0 {
                self.draw_rect(ug, &line_color, stroke, color, x, y, bar_width, bar_height);
            }
            if s.show_labels {
                let label_y = if value < 0.0 {
                    y + bar_height + 5.0
                } else {
                    y - 5.0
                };
                draw_value_label(ug, &self.skin, value, x + bar_width / 2.0, label_y);
            }
        }
    }

    fn draw_horizontal(&self, ug: &UGraphic, s: &ChartSeries, color: &HColor) {
        if self.category_count == 0 {
            return;
        }
        let category_height = self.plot_height / self.category_count as f64;
        let style = self.series_bar_style(s);
        let bar_height = category_height * bar_width_ratio(&style);
        let bar_offset = (category_height - bar_height) / 2.0;
        let line_color = color_of(&style, PName::LineColor);
        let stroke = UStroke::with_thickness(style.value(PName::LineThickness).as_double());
        for (i, value) in s.values.iter().take(self.category_count).enumerate() {
            let Some(value) = *value else { continue };
            let y = i as f64 * category_height + bar_offset;
            let bar_width =
                ((value - self.axis.min) / (self.axis.max - self.axis.min) * self.plot_width).abs();
            if bar_height > 0.0 {
                self.draw_rect(
                    ug,
                    &line_color,
                    stroke,
                    color,
                    0.0,
                    y,
                    bar_width,
                    bar_height,
                );
            }
            if s.show_labels {
                draw_label_horizontal(ug, &self.skin, value, bar_width + 5.0, y + bar_height / 2.0);
            }
        }
    }

    /// Side-by-side bars of several series in each category (`drawGrouped`).
    pub(super) fn draw_grouped(&self, ug: &UGraphic, bars: &[&ChartSeries], colors: &[HColor]) {
        if self.category_count == 0 || bars.is_empty() {
            return;
        }
        let category_width = self.plot_width / self.category_count as f64;
        let group_width = category_width * 0.8;
        let bar_width = group_width / bars.len() as f64;
        let group_offset = (category_width - group_width) / 2.0;
        let style = merged(&self.skin, &queries::CHARTDIAG_BAR);
        let stroke = UStroke::with_thickness(style.value(PName::LineThickness).as_double());
        for category in 0..self.category_count {
            for (index, s) in bars.iter().enumerate() {
                let color = &colors[index];
                let Some(Some(value)) = s.values.get(category).copied() else {
                    continue;
                };
                let x = category as f64 * category_width + group_offset + index as f64 * bar_width;
                let (y, bar_height) = self.bar_geometry(value);
                let line_color = color_of(&style, PName::LineColor);
                if bar_height > 0.0 {
                    self.draw_rect(ug, &line_color, stroke, color, x, y, bar_width, bar_height);
                }
                if s.show_labels {
                    let label_y = if value < 0.0 {
                        y + bar_height + 5.0
                    } else {
                        y - 5.0
                    };
                    draw_value_label(ug, &self.skin, value, x + bar_width / 2.0, label_y);
                }
            }
        }
    }

    /// Bars of several series piled on each other (`drawStacked`).
    pub(super) fn draw_stacked(&self, ug: &UGraphic, bars: &[&ChartSeries], colors: &[HColor]) {
        if self.category_count == 0 || bars.is_empty() {
            return;
        }
        let category_width = self.plot_width / self.category_count as f64;
        let style = merged(&self.skin, &queries::CHARTDIAG_BAR);
        let bar_width = category_width * bar_width_ratio(&style);
        let bar_offset = (category_width - bar_width) / 2.0;
        let stroke = UStroke::with_thickness(style.value(PName::LineThickness).as_double());
        for category in 0..self.category_count {
            let zero_y = self.plot_height
                - (0.0 - self.axis.min) / (self.axis.max - self.axis.min) * self.plot_height;
            let mut cumulative_positive_y = zero_y;
            let mut cumulative_negative_y = zero_y;
            for (index, s) in bars.iter().enumerate() {
                let color = &colors[index];
                let Some(Some(value)) = s.values.get(category).copied() else {
                    continue;
                };
                let x = category as f64 * category_width + bar_offset;
                let top_y = value_y(value, &self.axis, self.plot_height);
                let (y, bar_height) = if value < 0.0 {
                    let bar_height = top_y - zero_y;
                    let y = cumulative_negative_y;
                    cumulative_negative_y += bar_height;
                    (y, bar_height)
                } else {
                    let bar_height = zero_y - top_y;
                    cumulative_positive_y -= bar_height;
                    (cumulative_positive_y, bar_height)
                };
                let line_color = color_of(&style, PName::LineColor);
                if bar_height > 0.0 {
                    self.draw_rect(ug, &line_color, stroke, color, x, y, bar_width, bar_height);
                }
                if s.show_labels {
                    let label_y = if value < 0.0 {
                        y + bar_height + 5.0
                    } else {
                        y - 5.0
                    };
                    draw_value_label(ug, &self.skin, value, x + bar_width / 2.0, label_y);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_rect(
        &self,
        ug: &UGraphic,
        line_color: &HColor,
        stroke: UStroke,
        fill: &HColor,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        ug.apply_color(line_color.clone())
            .apply_stroke(stroke)
            .apply_back_color(fill.clone())
            .apply_translate(UTranslate::new(x, y))
            .draw(URectangle::build(width, height));
    }
}

/// Value label of a horizontal bar, left aligned after the bar.
fn draw_label_horizontal(ug: &UGraphic, skin: &SkinParam, value: f64, x: f64, y: f64) {
    let black = HColor::BLACK;
    let fc = FontConfiguration::create(label_font(), black.clone(), black, None, 8);
    let text = super::format_value(value);
    let tb = text_block(skin, &text, &fc, HorizontalAlignment::Left);
    let dim = tb.calculate_dimension(ug.string_bounder());
    let label_y = y - dim.height / 2.0;
    tb.draw(&ug.apply_translate(UTranslate::new(x, label_y)));
}

/// Polyline through the values of a line series, optionally smoothed (`LineRenderer`).
pub(super) struct LineRenderer {
    pub(super) plot_width: f64,
    pub(super) plot_height: f64,
    pub(super) category_count: usize,
    pub(super) axis: ChartAxis,
    pub(super) x_axis: ChartAxis,
}

impl LineRenderer {
    pub(super) fn draw(&self, ug: &UGraphic, s: &ChartSeries, color: &HColor) {
        if self.category_count == 0 && !s.has_explicit_x_values() {
            return;
        }
        let stroke = UStroke::with_thickness(2.0);
        if s.interpolate {
            let curves = natural_spline(&self.points(s));
            if let Some(first) = curves.first() {
                let mut path = UPath::new();
                path.move_to(first.x0, first.y0);
                for c in &curves {
                    path.cubic_to(c.c1x, c.c1y, c.c2x, c.c2y, c.x1, c.y1);
                }
                ug.apply_color(color.clone())
                    .apply_stroke(stroke)
                    .apply_back_color(HColor::TRANSPARENT)
                    .draw(path);
                return;
            }
        }
        if s.connect {
            let points = self.points(s);
            for pair in points.windows(2) {
                let (p1, p2) = (pair[0], pair[1]);
                ug.apply_color(color.clone())
                    .apply_stroke(stroke)
                    .apply_translate(UTranslate::new(p1.0, p1.1))
                    .draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
            }
            return;
        }
        for (i, (a, b)) in s.values.iter().zip(s.values.iter().skip(1)).enumerate() {
            let (Some(v1), Some(v2)) = (*a, *b) else {
                continue;
            };
            let (x1, y1, x2, y2) = match &s.x_values {
                Some(xs) => (
                    self.x_axis.value_to_pixel(xs[i], 0.0, self.plot_width),
                    value_y(v1, &self.axis, self.plot_height),
                    self.x_axis.value_to_pixel(xs[i + 1], 0.0, self.plot_width),
                    value_y(v2, &self.axis, self.plot_height),
                ),
                None => {
                    let category_width = self.plot_width / self.category_count as f64;
                    if i >= self.category_count.saturating_sub(1) {
                        break;
                    }
                    (
                        (i as f64 + 0.5) * category_width,
                        value_y(v1, &self.axis, self.plot_height),
                        (i as f64 + 1.5) * category_width,
                        value_y(v2, &self.axis, self.plot_height),
                    )
                }
            };
            ug.apply_color(color.clone())
                .apply_stroke(stroke)
                .apply_translate(UTranslate::new(x1, y1))
                .draw(ULine::new(x2 - x1, y2 - y1));
        }
    }

    /// Pixel positions of the non-missing values (`points`).
    fn points(&self, s: &ChartSeries) -> Vec<(f64, f64)> {
        let range = self.axis.max - self.axis.min;
        let mut result = Vec::new();
        match &s.x_values {
            Some(xs) => {
                for (i, value) in s.values.iter().enumerate() {
                    if let Some(v) = value {
                        result.push((
                            self.x_axis.value_to_pixel(xs[i], 0.0, self.plot_width),
                            self.plot_height - (v - self.axis.min) / range * self.plot_height,
                        ));
                    }
                }
            }
            None => {
                let category_width = self.plot_width / self.category_count as f64;
                for (i, value) in s.values.iter().take(self.category_count).enumerate() {
                    if let Some(v) = value {
                        result.push((
                            (i as f64 + 0.5) * category_width,
                            self.plot_height - (v - self.axis.min) / range * self.plot_height,
                        ));
                    }
                }
            }
        }
        result
    }
}

/// One cubic Bezier piece of a smoothed line.
struct CubicPiece {
    x0: f64,
    y0: f64,
    c1x: f64,
    c1y: f64,
    c2x: f64,
    c2y: f64,
    x1: f64,
    y1: f64,
}

/// Natural cubic spline through the points (`NaturalSpline.through`).
///
/// Empty when the points are not strictly increasing in x.
fn natural_spline(pts: &[(f64, f64)]) -> Vec<CubicPiece> {
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let mut h = vec![0.0; n - 1];
    for i in 0..n - 1 {
        h[i] = pts[i + 1].0 - pts[i].0;
        if h[i] <= 0.0 {
            return Vec::new();
        }
    }
    let mut m = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    for i in 1..n - 1 {
        let b = 2.0 * (h[i - 1] + h[i]);
        let r = 6.0 * ((pts[i + 1].1 - pts[i].1) / h[i] - (pts[i].1 - pts[i - 1].1) / h[i - 1]);
        let denominator = b - h[i - 1] * c[i - 1];
        c[i] = h[i] / denominator;
        d[i] = (r - h[i - 1] * d[i - 1]) / denominator;
    }
    for i in (1..n - 1).rev() {
        m[i] = d[i] - c[i] * m[i + 1];
    }
    (0..n - 1)
        .map(|i| {
            let (x0, y0) = pts[i];
            let (x1, y1) = pts[i + 1];
            let slope = (y1 - y0) / h[i];
            let start = slope - h[i] * (2.0 * m[i] + m[i + 1]) / 6.0;
            let end = slope + h[i] * (m[i] + 2.0 * m[i + 1]) / 6.0;
            CubicPiece {
                x0,
                y0,
                c1x: x0 + h[i] / 3.0,
                c1y: y0 + start * h[i] / 3.0,
                c2x: x1 - h[i] / 3.0,
                c2y: y1 - end * h[i] / 3.0,
                x1,
                y1,
            }
        })
        .collect()
}

/// Filled area under a series, stacked on the previous areas (`AreaRenderer`).
pub(super) struct AreaRenderer {
    pub(super) skin: Rc<SkinParam>,
    pub(super) plot_width: f64,
    pub(super) plot_height: f64,
    pub(super) category_count: usize,
    pub(super) axis: ChartAxis,
}

impl AreaRenderer {
    /// `baseline` holds the top of the areas drawn before this one.
    pub(super) fn draw(
        &self,
        ug: &UGraphic,
        s: &ChartSeries,
        color: &HColor,
        baseline: Option<&[f64]>,
    ) {
        if self.category_count == 0 {
            return;
        }
        let values: Vec<f64> = s.values.iter().map(|v| v.unwrap_or(0.0)).collect();
        let count = values.len().min(self.category_count);
        let category_width = self.plot_width / self.category_count as f64;
        let top = |i: usize, value: f64| match baseline {
            Some(b) if i < b.len() => b[i] + value,
            _ => value,
        };
        let mut points = Vec::new();
        for (i, value) in values.iter().take(count).enumerate() {
            let x = (i as f64 + 0.5) * category_width;
            points.push((x, self.y(top(i, *value))));
        }
        for i in (0..count).rev() {
            let x = (i as f64 + 0.5) * category_width;
            let base = match baseline {
                Some(b) if i < b.len() => b[i],
                _ => self.axis.min,
            };
            points.push((x, self.y(base)));
        }
        ug.apply_color(color.clone())
            .apply_back_color(color.clone())
            .draw(UPolygon::new(points));
        let segments = values.len().min(self.category_count).saturating_sub(1);
        for i in 0..segments {
            let x1 = (i as f64 + 0.5) * category_width;
            let y1 = self.y(top(i, values[i]));
            let x2 = (i as f64 + 1.5) * category_width;
            let y2 = self.y(top(i + 1, values[i + 1]));
            ug.apply_color(color.clone())
                .apply_stroke(UStroke::with_thickness(2.0))
                .apply_translate(UTranslate::new(x1, y1))
                .draw(ULine::new(x2 - x1, y2 - y1));
        }
        if s.show_labels {
            for (i, value) in values.iter().take(count).enumerate() {
                let x = (i as f64 + 0.5) * category_width;
                let y = self.y(top(i, *value));
                draw_value_label(ug, &self.skin, *value, x, y - 8.0);
            }
        }
    }

    fn y(&self, value: f64) -> f64 {
        value_y(value, &self.axis, self.plot_height)
    }
}

/// Markers of a scatter series (`ScatterRenderer`).
pub(super) struct ScatterRenderer {
    pub(super) skin: Rc<SkinParam>,
    pub(super) plot_width: f64,
    pub(super) plot_height: f64,
    pub(super) category_count: usize,
    pub(super) axis: ChartAxis,
    pub(super) x_axis: ChartAxis,
}

impl ScatterRenderer {
    pub(super) fn draw(&self, ug: &UGraphic, s: &ChartSeries) {
        if self.category_count == 0 && !s.has_explicit_x_values() {
            return;
        }
        let style = series_style(
            &self.skin,
            &queries::CHARTDIAG_SCATTER,
            s.stereotype.as_ref(),
        );
        let marker_color = color_of(&style, PName::MarkerColor);
        let marker_size = match style.value(PName::MarkerSize).as_double() {
            v if v > 0.0 => v,
            _ => 8.0,
        };
        let mut shape = s.marker_shape;
        match style
            .value(PName::MarkerShape)
            .as_string()
            .to_lowercase()
            .as_str()
        {
            "circle" => shape = MarkerShape::Circle,
            "square" => shape = MarkerShape::Square,
            "triangle" => shape = MarkerShape::Triangle,
            _ => {}
        }
        for (i, value) in s.values.iter().enumerate() {
            let Some(yv) = *value else { continue };
            let (x, y) = match &s.x_values {
                Some(xs) => (
                    self.x_axis.value_to_pixel(xs[i], 0.0, self.plot_width),
                    value_y(yv, &self.axis, self.plot_height),
                ),
                None => {
                    if i >= self.category_count {
                        break;
                    }
                    let category_width = self.plot_width / self.category_count as f64;
                    (
                        (i as f64 + 0.5) * category_width,
                        value_y(yv, &self.axis, self.plot_height),
                    )
                }
            };
            draw_marker(ug, &marker_color, x, y, marker_size, shape);
            if s.show_labels {
                draw_value_label(ug, &self.skin, yv, x, y - marker_size / 2.0 - 8.0);
            }
        }
    }
}

fn draw_marker(ug: &UGraphic, color: &HColor, x: f64, y: f64, size: f64, shape: MarkerShape) {
    let ug = ug
        .apply_color(color.clone())
        .apply_back_color(color.clone());
    match shape {
        MarkerShape::Circle => ug
            .apply_translate(UTranslate::new(x - size / 2.0, y - size / 2.0))
            .draw(UEllipse::build(size, size)),
        MarkerShape::Square => ug
            .apply_translate(UTranslate::new(x - size / 2.0, y - size / 2.0))
            .draw(URectangle::build(size, size)),
        MarkerShape::Triangle => ug
            .apply_translate(UTranslate::new(x, y))
            .draw(UPolygon::new(vec![
                (0.0, -size / 2.0),
                (-size / 2.0, size / 2.0),
                (size / 2.0, size / 2.0),
            ])),
    }
}
