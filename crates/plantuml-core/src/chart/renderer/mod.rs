//! Drawing of a chart: axes, grid, series, annotations and legend (`ChartRenderer`).

mod series;

use super::{
    AnnotationX, ChartAnnotation, ChartAxis, ChartSeries, GridMode, LabelPosition, LegendPosition,
    MarkerShape, Orientation, SeriesType, StackMode,
};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{CreoleMode, LineBreakStrategy};
use crate::klimt::drawing::ugraphic::{
    UEllipse, UGraphic, ULine, UPolygon, URectangle, UStroke, UText, UTranslate,
};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, FontFace, StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, MergeStrategy, PName, Style, StyleQuery};
use series::{AreaRenderer, BarRenderer, LineRenderer, ScatterRenderer};
use std::rc::Rc;

const MARGIN: f64 = 20.0;
const AXIS_LABEL_SPACE: f64 = 40.0;
const TITLE_SPACE: f64 = 30.0;
const TICK_SIZE: f64 = 5.0;
const LEGEND_MARGIN: f64 = 10.0;
const LEGEND_SYMBOL_SIZE: f64 = 12.0;
const LEGEND_TEXT_SPACING: f64 = 5.0;
const LEGEND_ITEM_SPACING: f64 = 15.0;

/// The style of `query`, which always matches the built-in skin.
pub(super) fn merged(skin: &SkinParam, query: &StyleQuery) -> Style {
    skin.style_builder()
        .merged_style(query)
        .expect("chart style matches")
}

/// Color of a property. As in Java, a value that is not defined reads as black.
pub(super) fn color_of(style: &Style, name: PName) -> HColor {
    style.value(name).as_color()
}

/// Style of a series query, with the stereotype-specific rules merged over the base style.
fn stereotype_style(
    skin: &SkinParam,
    query: &StyleQuery,
    stereotype: Option<&Stereotype>,
) -> Style {
    let Some(st) = stereotype else {
        return merged(skin, query);
    };
    let stereo_style = merged(skin, &query.for_stereotype_itself(Some(st)));
    match skin
        .style_builder()
        .merged_style(&query.with_stereotype_obj(Some(st)))
    {
        Some(style) => style.merge_with(&stereo_style, MergeStrategy::OverwriteExistingValue),
        None => stereo_style,
    }
}

/// Style of a series query, using only the stereotype-specific rules when present.
pub(super) fn series_style(
    skin: &SkinParam,
    query: &StyleQuery,
    stereotype: Option<&Stereotype>,
) -> Style {
    match stereotype {
        Some(st) => merged(skin, &query.with_stereotype_obj(Some(st))),
        None => merged(skin, query),
    }
}

/// A text block of `text` centered or aligned as requested, with the skin's creole settings.
pub(super) fn text_block(
    skin: &SkinParam,
    text: &str,
    font: &FontConfiguration,
    align: HorizontalAlignment,
) -> Rc<dyn TextBlock> {
    Display::with_newlines(Some(text)).create_block(
        font,
        align,
        &skin.skin_simple(),
        &LineBreakStrategy::NONE,
        CreoleMode::SimpleLine,
        None,
        None,
        0.0,
        0.0,
    )
}

/// Bold 10pt label drawn over a bar, point or area (`drawLabel`).
pub(super) fn draw_value_label(ug: &UGraphic, skin: &SkinParam, value: f64, x: f64, y: f64) {
    let font = UFont::sans_serif(10).with_face(FontFace::BOLD);
    let black = HColor::BLACK;
    let fc = FontConfiguration::create(font, black.clone(), black, None, 8);
    let tb = text_block(skin, &format_value(value), &fc, HorizontalAlignment::Center);
    let dim = tb.calculate_dimension(ug.string_bounder());
    let label_x = x - dim.width / 2.0;
    let label_y = y - dim.height - 2.0;
    tb.draw(&ug.apply_translate(UTranslate::new(label_x, label_y)));
}

/// Formats a number with `decimals` digits, rounding half up like Java's `%f`.
pub(super) fn java_fixed(v: f64, decimals: usize) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    let shortest = format!("{}", v.abs());
    let (int_part, frac_part) = shortest.split_once('.').unwrap_or((&shortest, ""));
    let int_len = int_part.len();
    let mut digits: Vec<u8> = int_part.bytes().chain(frac_part.bytes()).collect();
    let keep = int_len + decimals;
    let round_up = digits.len() > keep && digits[keep] >= b'5';
    digits.resize(keep, b'0');
    let mut int_len = int_len;
    if round_up {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                int_len += 1;
                break;
            }
            i -= 1;
            if digits[i] == b'9' {
                digits[i] = b'0';
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let int_text = String::from_utf8_lossy(&digits[..int_len]).to_string();
    if decimals == 0 {
        return format!("{sign}{int_text}");
    }
    let frac_text = String::from_utf8_lossy(&digits[int_len..]).to_string();
    format!("{sign}{int_text}.{frac_text}")
}

/// Scientific notation with two decimals and a signed two-digit exponent (`%.2e`).
pub(super) fn java_scientific(v: f64) -> String {
    let sign = if v < 0.0 { "-" } else { "" };
    let shortest = format!("{:e}", v.abs());
    let (mantissa, exp) = shortest.split_once('e').unwrap_or((&shortest, "0"));
    let mut exp: i32 = exp.parse().unwrap_or(0);
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    let round_up = digits.len() > 3 && digits[3] >= b'5';
    digits.resize(3, b'0');
    if round_up {
        let mut value: u32 = String::from_utf8_lossy(&digits).parse().unwrap_or(0) + 1;
        if value >= 1000 {
            value /= 10;
            exp += 1;
        }
        digits = format!("{value:03}").into_bytes();
    }
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!(
        "{sign}{}.{}{}e{exp_sign}{:02}",
        digits[0] as char,
        digits[1] as char,
        digits[2] as char,
        exp.abs()
    )
}

/// Number of an axis label with two decimals (`formatValue`).
pub(super) fn format_value(value: f64) -> String {
    format_number(value, 2)
}

/// Number of an axis tick with one decimal (`formatAxisValue`).
fn format_axis_value(value: f64) -> String {
    format_number(value, 1)
}

fn format_number(value: f64, decimals: usize) -> String {
    if value.abs() < 0.01 && value != 0.0 {
        return java_scientific(value);
    }
    if value == (value as i64) as f64 {
        return format!("{}", value as i64);
    }
    java_fixed(value, decimals)
}

/// Lays out and draws a whole chart (`ChartRenderer`).
pub(super) struct ChartRenderer {
    pub(super) skin: Rc<SkinParam>,
    pub(super) x_axis_labels: Vec<String>,
    pub(super) x_axis_title: Option<String>,
    pub(super) x_axis_tick_spacing: Option<i32>,
    pub(super) x_axis_label_position: LabelPosition,
    pub(super) series: Vec<ChartSeries>,
    pub(super) x_axis: ChartAxis,
    pub(super) y_axis: ChartAxis,
    pub(super) y2_axis: Option<ChartAxis>,
    pub(super) legend_position: LegendPosition,
    pub(super) x_grid_mode: GridMode,
    pub(super) y_grid_mode: GridMode,
    pub(super) stack_mode: StackMode,
    pub(super) orientation: Orientation,
    pub(super) annotations: Vec<ChartAnnotation>,
}

impl ChartRenderer {
    fn plot_width(&self) -> f64 {
        (self.x_axis_labels.len() as f64 * 60.0).max(400.0)
    }

    fn plot_height(&self) -> f64 {
        300.0
    }

    fn style(&self) -> Style {
        merged(&self.skin, &queries::CHARTDIAG)
    }

    fn axis_style(&self, horizontal: bool) -> Style {
        let axis = if horizontal {
            crate::style::SName::HAxis
        } else {
            crate::style::SName::VAxis
        };
        merged(&self.skin, &queries::CHARTDIAG_AXIS.add(axis))
    }

    /// Width and height of the chart (`calculateDimension`).
    pub(super) fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let legend = self.legend_dimension(sb);
        let mut width = MARGIN
            + AXIS_LABEL_SPACE
            + self.plot_width()
            + if self.y2_axis.is_some() {
                AXIS_LABEL_SPACE
            } else {
                0.0
            }
            + MARGIN;
        let mut height = MARGIN + TITLE_SPACE + self.plot_height() + AXIS_LABEL_SPACE + MARGIN;
        if self.x_axis_title.as_deref().is_some_and(|t| !t.is_empty()) {
            height += 20.0;
        }
        match self.legend_position {
            LegendPosition::Left | LegendPosition::Right => width += legend.width + LEGEND_MARGIN,
            LegendPosition::Top | LegendPosition::Bottom => height += legend.height + LEGEND_MARGIN,
            LegendPosition::None => {}
        }
        Dimension::new(width, height)
    }

    /// Draws the axes, grid, series, annotations and legend (`drawU`).
    pub(super) fn draw_u(&self, ug: &UGraphic) {
        let style = self.style();
        let line_color = style.value(PName::LineColor).as_color();
        let ug = ug
            .apply_color(line_color.clone())
            .apply_stroke(UStroke::with_thickness(1.0));
        let legend = self.legend_dimension(ug.string_bounder());
        let mut left_margin = MARGIN + AXIS_LABEL_SPACE;
        let mut top_margin = MARGIN + TITLE_SPACE;
        let plot_width = self.plot_width();
        let plot_height = self.plot_height();
        match self.legend_position {
            LegendPosition::Left => left_margin += legend.width + LEGEND_MARGIN,
            LegendPosition::Top => top_margin += legend.height + LEGEND_MARGIN,
            _ => {}
        }
        let horizontal = self.orientation == Orientation::Horizontal;
        let mut x_axis_y = top_margin + plot_height;
        if !horizontal && self.y_axis.min <= 0.0 && self.y_axis.max >= 0.0 {
            let zero_ratio = (0.0 - self.y_axis.min) / (self.y_axis.max - self.y_axis.min);
            x_axis_y = top_margin + plot_height * (1.0 - zero_ratio);
        }
        let mut y_axis_x = left_margin;
        if !horizontal && self.x_axis.min <= 0.0 && self.x_axis.max >= 0.0 {
            let zero_x = self.x_axis.value_to_pixel(0.0, 0.0, plot_width);
            y_axis_x = left_margin + zero_x;
        }
        if horizontal {
            self.draw_x_axis(
                &ug.apply_translate(UTranslate::new(left_margin, top_margin)),
                plot_height,
                plot_height,
            );
            self.draw_y_axis_horizontally(
                &ug.apply_translate(UTranslate::new(left_margin, top_margin + plot_height)),
                plot_width,
                &self.y_axis,
            );
        } else {
            self.draw_y_axis(
                &ug.apply_translate(UTranslate::new(y_axis_x, top_margin)),
                plot_height,
                &self.y_axis,
                true,
            );
            if let Some(y2) = &self.y2_axis {
                self.draw_y_axis(
                    &ug.apply_translate(UTranslate::new(left_margin + plot_width, top_margin)),
                    plot_height,
                    y2,
                    false,
                );
            }
            self.draw_x_axis(
                &ug.apply_translate(UTranslate::new(left_margin, x_axis_y)),
                plot_width,
                plot_height,
            );
        }
        let ug_plot = ug.apply_translate(UTranslate::new(left_margin, top_margin));
        self.draw_grid_lines(&ug_plot, plot_width, plot_height);
        self.draw_series(&ug_plot, plot_width, plot_height);
        self.draw_annotations(&ug_plot, plot_width, plot_height);
        self.draw_legend(&ug, left_margin, top_margin, plot_width, plot_height);
    }

    fn draw_y_axis(&self, ug: &UGraphic, height: f64, axis: &ChartAxis, left_side: bool) {
        let axis_style = self.axis_style(false);
        let line_thickness = axis_style.value(PName::LineThickness).as_double();
        let font = axis_style.font_configuration(None);
        let ug = ug
            .apply_color(color_of(&axis_style, PName::LineColor))
            .apply_stroke(UStroke::with_thickness(line_thickness));
        ug.draw(ULine::vline(height));
        let sb = ug.string_bounder();
        // Grid lines of the left axis are never drawn: the Java code only does so when the
        // h-axis is absent, and the renderer always has one.
        let tick_label = |label: &str, y: f64| {
            let tb = text_block(&self.skin, label, &font, HorizontalAlignment::Right);
            let text_height = tb.calculate_dimension(sb).height;
            if left_side {
                let text_width = tb.calculate_dimension(sb).width;
                ug.apply_translate(UTranslate::dy(y))
                    .draw(ULine::hline(-TICK_SIZE));
                tb.draw(&ug.apply_translate(UTranslate::new(
                    -TICK_SIZE - text_width - 5.0,
                    y - text_height / 2.0,
                )));
            } else {
                ug.apply_translate(UTranslate::dy(y))
                    .draw(ULine::hline(TICK_SIZE));
                tb.draw(
                    &ug.apply_translate(UTranslate::new(TICK_SIZE + 5.0, y - text_height / 2.0)),
                );
            }
        };
        if axis.has_custom_ticks() {
            for (value, label) in axis.custom_ticks.iter().flatten() {
                let y = height * (1.0 - (value - axis.min) / (axis.max - axis.min));
                if y < 0.0 || y > height {
                    continue;
                }
                tick_label(label, y);
            }
        } else if axis.has_tick_spacing() {
            let spacing = axis.tick_spacing.unwrap_or(1.0);
            let range = axis.max - axis.min;
            let mut value = (axis.min / spacing).ceil() * spacing;
            while value <= axis.max {
                if value > axis.max + spacing * 0.01 {
                    break;
                }
                let y = height * (1.0 - (value - axis.min) / range);
                tick_label(&format_value(value), y);
                value += spacing;
            }
        } else {
            for i in 0..=5 {
                let y = height * (1.0 - f64::from(i) / 5.0);
                let value = axis.min + (axis.max - axis.min) * f64::from(i) / 5.0;
                tick_label(&format_value(value), y);
            }
        }
        if !axis.title.is_empty() {
            if axis.label_position == LabelPosition::Top {
                self.draw_horizontal_axis_title(&ug, &axis.title, left_side);
            } else {
                self.draw_vertical_text(&ug, &axis.title, height, left_side);
            }
        }
    }

    fn grid_color(&self) -> HColor {
        color_of(
            &merged(&self.skin, &queries::CHARTDIAG_GRID),
            PName::LineColor,
        )
    }

    fn draw_vertical_text(&self, ug: &UGraphic, text: &str, height: f64, left_side: bool) {
        let font_config = self.axis_style(false).font_configuration(None);
        let orientation = if left_side { 90 } else { 270 };
        let mut utext = UText::new(text, font_config.clone());
        utext.orientation = orientation;
        let font = font_config.font();
        let sb = ug.string_bounder();
        let text_width = sb.calculate_dimension(&font, text).width;
        let text_height = sb.calculate_dimension(&font, text).height;
        let extra_spacing = 10.0;
        let x_pos = if left_side {
            -AXIS_LABEL_SPACE + text_height / 2.0 - extra_spacing
        } else {
            AXIS_LABEL_SPACE - text_height / 2.0 + extra_spacing
        };
        let y_pos = if left_side {
            height / 2.0 + text_width / 2.0
        } else {
            height / 2.0 - text_width / 2.0
        };
        ug.apply_translate(UTranslate::new(x_pos, y_pos))
            .draw(utext);
    }

    /// Title of a value axis placed above it (`drawHorizontalAxisTitle`).
    fn draw_horizontal_axis_title(&self, ug: &UGraphic, text: &str, left_side: bool) {
        let font = self.axis_style(false).font_configuration(None);
        let tb = text_block(&self.skin, text, &font, HorizontalAlignment::Center);
        let dim = tb.calculate_dimension(ug.string_bounder());
        let x_pos = if left_side {
            -AXIS_LABEL_SPACE / 2.0 - dim.width / 2.0
        } else {
            AXIS_LABEL_SPACE / 2.0 - dim.width / 2.0
        };
        let y_pos = -dim.height - 15.0;
        tb.draw(&ug.apply_translate(UTranslate::new(x_pos, y_pos)));
    }

    fn draw_x_axis(&self, ug: &UGraphic, width: f64, height: f64) {
        if self.orientation == Orientation::Horizontal {
            self.draw_categories_vertically_on_left(ug, height);
            return;
        }
        let axis_style = self.axis_style(true);
        let line_thickness = axis_style.value(PName::LineThickness).as_double();
        let font = axis_style.font_configuration(None);
        let ug = ug
            .apply_color(color_of(&axis_style, PName::LineColor))
            .apply_stroke(UStroke::with_thickness(line_thickness));
        ug.draw(ULine::hline(width));
        let sb = ug.string_bounder();
        let grid_color = self.grid_color();
        let grid_stroke = self.grid_stroke();
        if !self.x_axis_labels.is_empty() {
            let count = self.x_axis_labels.len() as f64;
            let category_width = width / count;
            let spacing = self
                .x_axis_tick_spacing
                .filter(|s| *s > 0)
                .map_or(1, |s| s as usize);
            if self.x_grid_mode != GridMode::Off {
                let (distance_to_top, distance_to_bottom) = if self.y_axis.min <= 0.0
                    && self.y_axis.max >= 0.0
                {
                    let zero_ratio = (0.0 - self.y_axis.min) / (self.y_axis.max - self.y_axis.min);
                    (height * (1.0 - zero_ratio), height * zero_ratio)
                } else {
                    (height, 0.0)
                };
                for i in 0..self.x_axis_labels.len() {
                    let grid_x = (i as f64 + 0.5) * category_width;
                    let grid_ug = ug
                        .apply_color(grid_color.clone())
                        .apply_stroke(grid_stroke)
                        .apply_translate(UTranslate::dx(grid_x));
                    if distance_to_top > 0.0 {
                        grid_ug.draw(ULine::vline(-distance_to_top));
                    }
                    if distance_to_bottom > 0.0 {
                        grid_ug.draw(ULine::vline(distance_to_bottom));
                    }
                }
            }
            for (i, label) in self.x_axis_labels.iter().enumerate() {
                let label_x = (i as f64 + 0.5) * category_width;
                if i % spacing == 0 {
                    ug.apply_translate(UTranslate::dx(label_x))
                        .draw(ULine::vline(TICK_SIZE));
                    let tb = text_block(&self.skin, label, &font, HorizontalAlignment::Center);
                    let text_width = tb.calculate_dimension(sb).width;
                    tb.draw(&ug.apply_translate(UTranslate::new(
                        label_x - text_width / 2.0,
                        TICK_SIZE + 5.0,
                    )));
                }
            }
        } else {
            let range = self.x_axis.max - self.x_axis.min;
            let tick_interval = match self.x_axis_tick_spacing.filter(|s| *s > 0) {
                Some(s) => f64::from(s),
                None => range / 10.0,
            };
            let start = (self.x_axis.min / tick_interval).floor() * tick_interval;
            let limit = self.x_axis.max + tick_interval * 0.01;
            let mut value = start;
            while tick_interval > 0.0 && value <= limit {
                if !(value < self.x_axis.min - tick_interval * 0.01 || value > limit) {
                    let x = self.x_axis.value_to_pixel(value, 0.0, width);
                    ug.apply_translate(UTranslate::dx(x))
                        .draw(ULine::vline(TICK_SIZE));
                    let tb = text_block(
                        &self.skin,
                        &format_axis_value(value),
                        &font,
                        HorizontalAlignment::Center,
                    );
                    let text_width = tb.calculate_dimension(sb).width;
                    tb.draw(
                        &ug.apply_translate(UTranslate::new(x - text_width / 2.0, TICK_SIZE + 5.0)),
                    );
                }
                value += tick_interval;
            }
        }
        self.draw_x_title(&ug, width, &font);
    }

    /// Title below (or right of) the h-axis (`drawXAxis` tail).
    fn draw_x_title(&self, ug: &UGraphic, width: f64, font: &FontConfiguration) {
        let Some(title) = self.x_axis_title.as_deref().filter(|t| !t.is_empty()) else {
            return;
        };
        if self.x_axis_label_position == LabelPosition::Right {
            let tb = text_block(&self.skin, title, font, HorizontalAlignment::Left);
            let text_height = tb.calculate_dimension(ug.string_bounder()).height;
            tb.draw(&ug.apply_translate(UTranslate::new(width + 10.0, -text_height / 2.0)));
        } else {
            let tb = text_block(&self.skin, title, font, HorizontalAlignment::Center);
            let title_width = tb.calculate_dimension(ug.string_bounder()).width;
            let title_y = TICK_SIZE + 25.0;
            tb.draw(&ug.apply_translate(UTranslate::new(width / 2.0 - title_width / 2.0, title_y)));
        }
    }

    fn grid_stroke(&self) -> UStroke {
        let grid = merged(&self.skin, &queries::CHARTDIAG_GRID);
        UStroke::with_thickness(grid.value(PName::LineThickness).as_double())
    }

    /// Category names listed down the left side of a horizontal chart.
    fn draw_categories_vertically_on_left(&self, ug: &UGraphic, height: f64) {
        ug.draw(ULine::vline(height));
        if self.x_axis_labels.is_empty() {
            return;
        }
        let font = self.plain_font();
        let category_height = height / self.x_axis_labels.len() as f64;
        for (i, label) in self.x_axis_labels.iter().enumerate() {
            let y = (i as f64 + 0.5) * category_height;
            ug.apply_translate(UTranslate::new(-TICK_SIZE, y))
                .draw(ULine::hline(TICK_SIZE));
            let tb = text_block(&self.skin, label, &font, HorizontalAlignment::Right);
            let dim = tb.calculate_dimension(ug.string_bounder());
            tb.draw(&ug.apply_translate(UTranslate::new(
                -TICK_SIZE - dim.width - 5.0,
                y - dim.height / 2.0,
            )));
        }
    }

    fn plain_font(&self) -> FontConfiguration {
        FontConfiguration::create(UFont::sans_serif(10), HColor::BLACK, HColor::BLACK, None, 8)
    }

    /// Category labels of a horizontal chart's value axis, and its title (`drawYAxisHorizontally`).
    fn draw_y_axis_horizontally(&self, ug: &UGraphic, width: f64, axis: &ChartAxis) {
        ug.draw(ULine::hline(width));
        let font = self.plain_font();
        let range = axis.max - axis.min;
        let sb = ug.string_bounder();
        for i in 0..=5 {
            let value = axis.min + (f64::from(i) * range / 5.0);
            let x = width * f64::from(i) / 5.0;
            ug.apply_translate(UTranslate::dx(x))
                .draw(ULine::vline(TICK_SIZE));
            let tb = text_block(
                &self.skin,
                &java_fixed(value, 0),
                &font,
                HorizontalAlignment::Center,
            );
            let text_width = tb.calculate_dimension(sb).width;
            tb.draw(&ug.apply_translate(UTranslate::new(x - text_width / 2.0, TICK_SIZE + 5.0)));
        }
        if !axis.title.is_empty() {
            let tb = text_block(&self.skin, &axis.title, &font, HorizontalAlignment::Center);
            let title_width = tb.calculate_dimension(sb).width;
            let title_y = TICK_SIZE + 25.0;
            tb.draw(&ug.apply_translate(UTranslate::new(width / 2.0 - title_width / 2.0, title_y)));
        }
    }

    fn draw_grid_lines(&self, ug: &UGraphic, plot_width: f64, plot_height: f64) {
        let grid_color = self.grid_color();
        let grid_stroke = self.grid_stroke();
        if self.x_grid_mode != GridMode::Off {
            let range = self.x_axis.max - self.x_axis.min;
            let tick_interval = match self.x_axis_tick_spacing.filter(|s| *s > 0) {
                Some(s) => f64::from(s),
                None => range / 10.0,
            };
            for value in ticks_between(self.x_axis.min, self.x_axis.max, tick_interval, false) {
                let x = self.x_axis.value_to_pixel(value, 0.0, plot_width);
                ug.apply_color(grid_color.clone())
                    .apply_stroke(grid_stroke)
                    .apply_translate(UTranslate::dx(x))
                    .draw(ULine::vline(plot_height));
            }
        }
        if self.y_grid_mode != GridMode::Off {
            let range = self.y_axis.max - self.y_axis.min;
            let tick_interval = if self.y_axis.has_tick_spacing() {
                self.y_axis.tick_spacing.unwrap_or(range / 10.0)
            } else {
                range / 10.0
            };
            for value in ticks_between(self.y_axis.min, self.y_axis.max, tick_interval, true) {
                let y = plot_height
                    * (1.0 - (value - self.y_axis.min) / (self.y_axis.max - self.y_axis.min));
                ug.apply_color(grid_color.clone())
                    .apply_stroke(grid_stroke)
                    .apply_translate(UTranslate::dy(y))
                    .draw(ULine::hline(plot_width));
            }
        }
    }

    fn draw_series(&self, ug: &UGraphic, plot_width: f64, plot_height: f64) {
        let has_coordinate_pairs = self
            .series
            .first()
            .is_some_and(ChartSeries::has_explicit_x_values);
        if (self.x_axis_labels.is_empty() && !has_coordinate_pairs) || self.series.is_empty() {
            return;
        }
        let categories = self.x_axis_labels.len();
        let skin = &self.skin;
        let bars: Vec<&ChartSeries> = self
            .series
            .iter()
            .filter(|s| s.series_type == SeriesType::Bar)
            .collect();
        let bar_colors: Vec<HColor> = bars
            .iter()
            .map(|s| {
                s.color
                    .clone()
                    .unwrap_or_else(|| color_of(&self.bar_style(s), PName::BackGroundColor))
            })
            .collect();
        if let Some(first) = bars.first() {
            let renderer = BarRenderer {
                skin: skin.clone(),
                plot_width,
                plot_height,
                category_count: categories,
                axis: self.axis_for(first),
                horizontal: self.orientation == Orientation::Horizontal,
            };
            if bars.len() == 1 {
                renderer.draw(ug, bars[0], &bar_colors[0]);
            } else if self.stack_mode == StackMode::Stacked {
                renderer.draw_stacked(ug, &bars, &bar_colors);
            } else {
                renderer.draw_grouped(ug, &bars, &bar_colors);
            }
        }
        for s in &self.series {
            match s.series_type {
                SeriesType::Line => {
                    let style =
                        stereotype_style(skin, &queries::CHARTDIAG_LINE, s.stereotype.as_ref());
                    let color = s
                        .color
                        .clone()
                        .unwrap_or_else(|| color_of(&style, PName::LineColor));
                    LineRenderer {
                        plot_width,
                        plot_height,
                        category_count: categories,
                        axis: self.axis_for(s),
                        x_axis: self.x_axis.clone(),
                    }
                    .draw(ug, s, &color);
                }
                SeriesType::Scatter => {
                    ScatterRenderer {
                        skin: skin.clone(),
                        plot_width,
                        plot_height,
                        category_count: categories,
                        axis: self.axis_for(s),
                        x_axis: self.x_axis.clone(),
                    }
                    .draw(ug, s);
                }
                SeriesType::Bar | SeriesType::Area => {}
            }
        }
        let areas: Vec<&ChartSeries> = self
            .series
            .iter()
            .filter(|s| s.series_type == SeriesType::Area)
            .collect();
        let area_colors: Vec<HColor> = areas
            .iter()
            .map(|s| {
                s.color.clone().unwrap_or_else(|| {
                    let style =
                        stereotype_style(skin, &queries::CHARTDIAG_AREA, s.stereotype.as_ref());
                    color_of(&style, PName::BackGroundColor)
                })
            })
            .collect();
        if let Some(first) = areas.first() {
            let renderer = AreaRenderer {
                skin: skin.clone(),
                plot_width,
                plot_height,
                category_count: categories,
                axis: self.axis_for(first),
            };
            let mut cumulative: Option<Vec<f64>> = None;
            for (area, color) in areas.iter().zip(&area_colors) {
                renderer.draw(ug, area, color, cumulative.as_deref());
                let values = area.values.iter().map(|v| v.unwrap_or(0.0));
                match cumulative.as_mut() {
                    None => cumulative = Some(values.collect()),
                    Some(total) => {
                        for (j, v) in values.enumerate() {
                            if let Some(t) = total.get_mut(j) {
                                *t += v;
                            } else {
                                total.push(v);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Axis a series is measured on: the secondary one when asked and declared.
    fn axis_for(&self, s: &ChartSeries) -> ChartAxis {
        match (&self.y2_axis, s.use_secondary_axis) {
            (Some(y2), true) => y2.clone(),
            _ => self.y_axis.clone(),
        }
    }

    fn bar_style(&self, s: &ChartSeries) -> Style {
        stereotype_style(&self.skin, &queries::CHARTDIAG_BAR, s.stereotype.as_ref())
    }

    fn draw_annotations(&self, ug: &UGraphic, plot_width: f64, plot_height: f64) {
        if self.annotations.is_empty() {
            return;
        }
        let style = merged(&self.skin, &queries::CHARTDIAG_ANNOTATION);
        let font = style.font_configuration(None);
        let arrow_color = color_of(&style, PName::LineColor);
        let arrow_thickness = style.value(PName::LineThickness).as_double();
        let sb = ug.string_bounder();
        let horizontal = self.orientation == Orientation::Horizontal;
        for annotation in &self.annotations {
            let mut x = 0.0;
            let mut y = 0.0;
            match &annotation.x_position {
                AnnotationX::Number(value) => {
                    let range = self.y_axis.max - self.y_axis.min;
                    if horizontal {
                        y = plot_height * (1.0 - (value - self.y_axis.min) / range);
                    } else {
                        x = plot_width * ((value - self.y_axis.min) / range);
                    }
                }
                AnnotationX::Label(label) => {
                    let Some(index) = self.x_axis_labels.iter().position(|l| l == label) else {
                        continue;
                    };
                    let count = self.x_axis_labels.len() as f64;
                    if horizontal {
                        y = plot_height * (index as f64 + 0.5) / count;
                    } else {
                        x = plot_width * (index as f64 + 0.5) / count;
                    }
                }
            }
            if horizontal {
                x = plot_width
                    * ((annotation.y_position - self.y_axis.min)
                        / (self.y_axis.max - self.y_axis.min));
            } else {
                y = plot_height
                    * (1.0
                        - (annotation.y_position - self.y_axis.min)
                            / (self.y_axis.max - self.y_axis.min));
            }
            let tb = text_block(
                &self.skin,
                &annotation.text,
                &font,
                HorizontalAlignment::Left,
            );
            let dim = tb.calculate_dimension(sb);
            if !annotation.show_arrow {
                tb.draw(
                    &ug.apply_translate(UTranslate::new(x - dim.width / 2.0, y - dim.height - 5.0)),
                );
                continue;
            }
            let min_space_above = dim.height + 40.0;
            if y > min_space_above {
                let text_y = y - dim.height - 40.0;
                tb.draw(&ug.apply_translate(UTranslate::new(x - dim.width / 2.0, text_y)));
                let arrow_start_y = text_y + dim.height + 5.0;
                let arrow_end_y = y - 5.0;
                let arrow_length = arrow_end_y - arrow_start_y;
                if arrow_length > 0.0 {
                    draw_arrow(
                        ug,
                        &arrow_color,
                        arrow_thickness,
                        x,
                        arrow_start_y,
                        ULine::new(0.0, arrow_length),
                        arrow_end_y,
                        &[(0.0, 0.0), (-5.0, -8.0), (5.0, -8.0)],
                    );
                }
            } else {
                let text_y = y + 40.0;
                tb.draw(&ug.apply_translate(UTranslate::new(x - dim.width / 2.0, text_y)));
                let arrow_start_y = text_y - 5.0;
                let arrow_end_y = y + 5.0;
                let arrow_length = arrow_start_y - arrow_end_y;
                if arrow_length > 0.0 {
                    draw_arrow(
                        ug,
                        &arrow_color,
                        arrow_thickness,
                        x,
                        arrow_start_y,
                        ULine::new(0.0, -arrow_length),
                        arrow_end_y,
                        &[(0.0, 0.0), (-5.0, 8.0), (5.0, 8.0)],
                    );
                }
            }
        }
    }

    fn legend_font(&self) -> FontConfiguration {
        merged(&self.skin, &queries::CHARTDIAG_LEGEND).font_configuration(None)
    }

    /// Size of the legend box (`calculateLegendDimension`).
    fn legend_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        if self.legend_position == LegendPosition::None || self.series.is_empty() {
            return Dimension::new(0.0, 0.0);
        }
        let font = self.legend_font();
        let vertical = matches!(
            self.legend_position,
            LegendPosition::Left | LegendPosition::Right
        );
        let mut max_width = 0.0;
        let mut total_height = 0.0;
        for s in &self.series {
            let dim = text_block(&self.skin, &s.name, &font, HorizontalAlignment::Left)
                .calculate_dimension(sb);
            if vertical {
                max_width = f64::max(max_width, dim.width);
                total_height += dim.height + LEGEND_ITEM_SPACING;
            } else {
                max_width +=
                    dim.width + LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING + LEGEND_ITEM_SPACING;
                total_height = f64::max(total_height, dim.height);
            }
        }
        if vertical {
            Dimension::new(
                max_width + LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING + LEGEND_MARGIN * 2.0,
                total_height,
            )
        } else {
            Dimension::new(max_width, total_height + LEGEND_MARGIN * 2.0)
        }
    }

    fn draw_legend(
        &self,
        ug: &UGraphic,
        left_margin: f64,
        top_margin: f64,
        plot_width: f64,
        plot_height: f64,
    ) {
        if self.legend_position == LegendPosition::None || self.series.is_empty() {
            return;
        }
        let font = self.legend_font();
        let (mut x, mut y) = match self.legend_position {
            LegendPosition::Left => (MARGIN, top_margin),
            LegendPosition::Right => (
                left_margin
                    + plot_width
                    + if self.y2_axis.is_some() {
                        AXIS_LABEL_SPACE
                    } else {
                        0.0
                    }
                    + LEGEND_MARGIN,
                top_margin,
            ),
            LegendPosition::Top => (left_margin, MARGIN),
            LegendPosition::Bottom => (
                left_margin,
                top_margin + plot_height + AXIS_LABEL_SPACE + LEGEND_MARGIN,
            ),
            LegendPosition::None => return,
        };
        let vertical = matches!(
            self.legend_position,
            LegendPosition::Left | LegendPosition::Right
        );
        for s in &self.series {
            match s.series_type {
                SeriesType::Bar => {
                    let color = s
                        .color
                        .clone()
                        .unwrap_or_else(|| color_of(&self.bar_style(s), PName::BackGroundColor));
                    self.legend_box(ug, &color, x, y);
                }
                SeriesType::Area => {
                    let style = stereotype_style(
                        &self.skin,
                        &queries::CHARTDIAG_AREA,
                        s.stereotype.as_ref(),
                    );
                    let color = s
                        .color
                        .clone()
                        .unwrap_or_else(|| color_of(&style, PName::BackGroundColor));
                    self.legend_box(ug, &color, x, y);
                }
                SeriesType::Line => {
                    let style = stereotype_style(
                        &self.skin,
                        &queries::CHARTDIAG_LINE,
                        s.stereotype.as_ref(),
                    );
                    let color = s
                        .color
                        .clone()
                        .unwrap_or_else(|| color_of(&style, PName::LineColor));
                    ug.apply_color(color)
                        .apply_stroke(UStroke::with_thickness(2.0))
                        .apply_translate(UTranslate::new(x, y + LEGEND_SYMBOL_SIZE / 2.0))
                        .draw(ULine::hline(LEGEND_SYMBOL_SIZE));
                }
                SeriesType::Scatter => {
                    let style = stereotype_style(
                        &self.skin,
                        &queries::CHARTDIAG_SCATTER,
                        s.stereotype.as_ref(),
                    );
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
                    draw_legend_marker(
                        ug,
                        &color_of(&style, PName::MarkerColor),
                        x + LEGEND_SYMBOL_SIZE / 2.0,
                        y + LEGEND_SYMBOL_SIZE / 2.0,
                        LEGEND_SYMBOL_SIZE * 0.7,
                        shape,
                    );
                }
            }
            let tb = text_block(&self.skin, &s.name, &font, HorizontalAlignment::Left);
            let text_dim = tb.calculate_dimension(ug.string_bounder());
            tb.draw(&ug.apply_translate(UTranslate::new(
                x + LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING,
                y,
            )));
            if vertical {
                y += text_dim.height + LEGEND_ITEM_SPACING;
            } else {
                x +=
                    LEGEND_SYMBOL_SIZE + LEGEND_TEXT_SPACING + text_dim.width + LEGEND_ITEM_SPACING;
            }
        }
    }

    fn legend_box(&self, ug: &UGraphic, color: &HColor, x: f64, y: f64) {
        ug.apply_color(color.clone())
            .apply_back_color(color.clone())
            .apply_translate(UTranslate::new(x, y))
            .draw(URectangle::build(LEGEND_SYMBOL_SIZE, LEGEND_SYMBOL_SIZE));
    }
}

/// Positions of the grid lines between `min` and `max` (`drawGridLines` loops).
///
/// `from_ceil` selects the start rounding used by the vertical axis.
/// A non-positive interval yields no line instead of looping forever.
fn ticks_between(min: f64, max: f64, interval: f64, from_ceil: bool) -> Vec<f64> {
    if interval <= 0.0 || !interval.is_finite() {
        return Vec::new();
    }
    let start = if from_ceil {
        (min / interval).ceil() * interval
    } else {
        (min / interval).floor() * interval
    };
    let limit = max + interval * 0.01;
    let mut values = Vec::new();
    let mut value = start;
    while value <= limit {
        if !(value < min - interval * 0.01 || value > limit) {
            values.push(value);
        }
        value += interval;
    }
    values
}

/// Line from the point of an annotation with its arrow head (`drawAnnotations`).
#[allow(clippy::too_many_arguments)]
fn draw_arrow(
    ug: &UGraphic,
    color: &HColor,
    thickness: f64,
    x: f64,
    start_y: f64,
    line: ULine,
    end_y: f64,
    head: &[(f64, f64)],
) {
    ug.apply_color(color.clone())
        .apply_stroke(UStroke::with_thickness(thickness))
        .apply_translate(UTranslate::new(x, start_y))
        .draw(line);
    ug.apply_color(color.clone())
        .apply_back_color(color.clone())
        .apply_translate(UTranslate::new(x, end_y))
        .draw(UPolygon::new(head.to_vec()));
}

/// Marker of a scatter series in the legend (`drawLegendScatterMarker`).
fn draw_legend_marker(
    ug: &UGraphic,
    color: &HColor,
    x: f64,
    y: f64,
    size: f64,
    shape: MarkerShape,
) {
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
