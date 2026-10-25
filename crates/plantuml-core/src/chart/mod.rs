//! Chart diagrams (`chart` package): bar, line, area and scatter series on axes.

mod commands;
mod renderer;
#[cfg(test)]
mod tests;

use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::UGraphic;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use renderer::ChartRenderer;
use std::rc::Rc;

/// Where the axis labels sit (`ChartAxis.LabelPosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPosition {
    #[default]
    Default,
    Top,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendPosition {
    #[default]
    None,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridMode {
    #[default]
    Off,
    Major,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackMode {
    #[default]
    Grouped,
    Stacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesType {
    Bar,
    Line,
    Area,
    Scatter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkerShape {
    #[default]
    Circle,
    Square,
    Triangle,
}

/// One axis of the chart (`ChartAxis`).
#[derive(Debug, Clone)]
pub struct ChartAxis {
    pub title: String,
    pub min: f64,
    pub max: f64,
    pub auto_scale: bool,
    /// Insertion-ordered `value -> label` pairs (`LinkedHashMap`).
    pub custom_ticks: Option<Vec<(f64, String)>>,
    pub tick_spacing: Option<f64>,
    pub label_position: LabelPosition,
}

impl Default for ChartAxis {
    fn default() -> Self {
        Self {
            title: String::new(),
            min: 0.0,
            max: 100.0,
            auto_scale: true,
            custom_ticks: None,
            tick_spacing: None,
            label_position: LabelPosition::Default,
        }
    }
}

impl ChartAxis {
    /// Axis with a fixed range (`ChartAxis(title, min, max)`).
    pub fn with_range(title: &str, min: f64, max: f64) -> Self {
        Self {
            title: title.to_string(),
            min,
            max,
            auto_scale: false,
            ..Self::default()
        }
    }

    pub fn set_min(&mut self, min: f64) {
        self.min = min;
        self.auto_scale = false;
    }

    pub fn set_max(&mut self, max: f64) {
        self.max = max;
        self.auto_scale = false;
    }

    pub fn value_to_pixel(&self, value: f64, pixel_min: f64, pixel_max: f64) -> f64 {
        if self.max == self.min {
            return pixel_min;
        }
        pixel_min + (value - self.min) / (self.max - self.min) * (pixel_max - pixel_min)
    }

    /// Grows the range to contain `value` while auto-scaled (`includeValue`).
    pub fn include_value(&mut self, value: f64) {
        if self.auto_scale {
            if value < self.min {
                self.min = value;
            }
            if value > self.max {
                self.max = value;
            }
        }
    }

    pub fn has_custom_ticks(&self) -> bool {
        self.custom_ticks.as_ref().is_some_and(|t| !t.is_empty())
    }

    pub fn has_tick_spacing(&self) -> bool {
        self.tick_spacing.is_some_and(|t| t > 0.0)
    }
}

/// One plotted series (`ChartSeries`). Values may be missing (`None`).
#[derive(Debug, Clone)]
pub struct ChartSeries {
    pub name: String,
    pub series_type: SeriesType,
    pub values: Vec<Option<f64>>,
    /// Present for coordinate-pair data (`x:y`).
    pub x_values: Option<Vec<f64>>,
    pub color: Option<HColor>,
    pub use_secondary_axis: bool,
    pub show_labels: bool,
    pub interpolate: bool,
    pub connect: bool,
    pub marker_shape: MarkerShape,
    pub stereotype: Option<Stereotype>,
}

impl ChartSeries {
    /// Index-based series.
    pub fn new(name: &str, series_type: SeriesType, values: Vec<Option<f64>>) -> Self {
        Self {
            name: name.to_string(),
            series_type,
            values,
            x_values: None,
            color: None,
            use_secondary_axis: false,
            show_labels: false,
            interpolate: false,
            connect: false,
            marker_shape: MarkerShape::Circle,
            stereotype: None,
        }
    }

    /// Coordinate-pair series.
    pub fn with_pairs(
        name: &str,
        series_type: SeriesType,
        x_values: Vec<f64>,
        values: Vec<Option<f64>>,
    ) -> Self {
        let mut s = Self::new(name, series_type, values);
        s.x_values = Some(x_values);
        s
    }

    /// Same series with other x positions (`withXValues`).
    pub fn with_x_values(&self, x_values: Vec<f64>) -> Self {
        let mut s = self.clone();
        s.x_values = Some(x_values);
        s
    }

    pub fn has_explicit_x_values(&self) -> bool {
        self.x_values.is_some()
    }
}

/// The x position of an annotation: a category label or a number.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationX {
    Label(String),
    Number(f64),
}

/// A text placed on the plot (`ChartAnnotation`).
#[derive(Debug, Clone)]
pub struct ChartAnnotation {
    pub text: String,
    pub x_position: AnnotationX,
    pub y_position: f64,
    pub show_arrow: bool,
}

/// A chart being built by commands (`ChartDiagram`).
#[derive(Debug, Clone)]
pub struct ChartDiagram {
    pub base: DiagramBase,
    x_axis_labels: Vec<String>,
    x_axis_title: Option<String>,
    x_axis_tick_spacing: Option<i32>,
    x_axis_label_position: LabelPosition,
    /// Numeric h-axis (horizontal bar charts and coordinate pairs).
    x_axis: ChartAxis,
    y_axis_labels: Vec<String>,
    series: Vec<ChartSeries>,
    y_axis: ChartAxis,
    y2_axis: Option<ChartAxis>,
    legend_position: LegendPosition,
    x_grid_mode: GridMode,
    y_grid_mode: GridMode,
    stack_mode: StackMode,
    orientation: Orientation,
    annotations: Vec<ChartAnnotation>,
}

impl ChartDiagram {
    pub fn from_source(source: UmlSource) -> Self {
        Self {
            base: DiagramBase::new(source, DiagramType::Chart),
            x_axis_labels: Vec::new(),
            x_axis_title: None,
            x_axis_tick_spacing: None,
            x_axis_label_position: LabelPosition::Default,
            x_axis: ChartAxis::default(),
            y_axis_labels: Vec::new(),
            series: Vec::new(),
            y_axis: ChartAxis::default(),
            y2_axis: None,
            legend_position: LegendPosition::None,
            x_grid_mode: GridMode::Off,
            y_grid_mode: GridMode::Off,
            stack_mode: StackMode::Grouped,
            orientation: Orientation::Vertical,
            annotations: Vec::new(),
        }
    }

    fn has_coordinate_pairs(&self) -> bool {
        self.series
            .first()
            .is_some_and(ChartSeries::has_explicit_x_values)
    }

    fn max_value_count(&self) -> usize {
        self.series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0)
    }

    /// Category names: the labels, or `1..n` for index-based series (`getCategories`).
    fn categories(&self, labels: &[String]) -> Vec<String> {
        if !labels.is_empty() || self.series.is_empty() || self.series[0].has_explicit_x_values() {
            return labels.to_vec();
        }
        (1..=self.max_value_count())
            .map(|i| i.to_string())
            .collect()
    }

    /// The h-axis actually used for drawing (`getHAxis`).
    fn h_axis(&self) -> ChartAxis {
        if !self.x_axis.auto_scale || !self.has_coordinate_pairs() {
            return self.x_axis.clone();
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in &self.series {
            for &x in s.x_values.iter().flatten() {
                min = min.min(x);
                max = max.max(x);
            }
        }
        if min == max {
            min -= 1.0;
            max += 1.0;
        }
        ChartAxis::with_range(&self.x_axis.title, min, max)
    }

    /// Series with x positions spread over the h-axis range (`getPlottedSeries`).
    fn plotted_series(&self) -> Vec<ChartSeries> {
        if self.x_axis.auto_scale
            || !self.x_axis_labels.is_empty()
            || self.series.is_empty()
            || self.has_coordinate_pairs()
        {
            return self.series.clone();
        }
        let count = self.max_value_count();
        let min = self.x_axis.min;
        let max = self.x_axis.max;
        self.series
            .iter()
            .map(|s| {
                let xs: Vec<f64> = (0..s.values.len())
                    .map(|i| {
                        if count == 1 {
                            (min + max) / 2.0
                        } else {
                            min + (max - min) * i as f64 / (count - 1) as f64
                        }
                    })
                    .collect();
                s.with_x_values(xs)
            })
            .collect()
    }

    fn renderer(&self, skin: &Rc<SkinParam>) -> ChartRenderer {
        let h_axis = self.h_axis();
        if self.orientation == Orientation::Horizontal {
            return ChartRenderer {
                skin: skin.clone(),
                x_axis_labels: self.categories(&self.y_axis_labels),
                x_axis_title: Some(self.y_axis.title.clone()),
                x_axis_tick_spacing: None,
                x_axis_label_position: self.x_axis_label_position,
                series: self.series.clone(),
                x_axis: h_axis.clone(),
                y_axis: h_axis,
                y2_axis: None,
                legend_position: self.legend_position,
                x_grid_mode: self.y_grid_mode,
                y_grid_mode: self.x_grid_mode,
                stack_mode: self.stack_mode,
                orientation: self.orientation,
                annotations: self.annotations.clone(),
            };
        }
        let h_title = if self.x_axis.title.is_empty() {
            self.x_axis_title.clone()
        } else {
            Some(self.x_axis.title.clone())
        };
        let categories = if self.x_axis.auto_scale {
            self.categories(&self.x_axis_labels)
        } else {
            self.x_axis_labels.clone()
        };
        ChartRenderer {
            skin: skin.clone(),
            x_axis_labels: categories,
            x_axis_title: h_title,
            x_axis_tick_spacing: self.x_axis_tick_spacing,
            x_axis_label_position: self.x_axis_label_position,
            series: self.plotted_series(),
            x_axis: h_axis,
            y_axis: self.y_axis.clone(),
            y2_axis: self.y2_axis.clone(),
            legend_position: self.legend_position,
            x_grid_mode: self.x_grid_mode,
            y_grid_mode: self.y_grid_mode,
            stack_mode: self.stack_mode,
            orientation: self.orientation,
            annotations: self.annotations.clone(),
        }
    }

    pub fn set_x_axis_labels(&mut self, labels: Vec<String>) -> CommandExecutionResult {
        if !labels.is_empty() && self.x_axis.auto_scale && self.has_coordinate_pairs() {
            return CommandExecutionResult::error(
                "Coordinate pair notation requires numeric h-axis (e.g., h-axis \"x\" -5 --> 5), not categorical labels",
            );
        }
        self.x_axis_labels = labels;
        CommandExecutionResult::ok()
    }

    pub fn set_x_axis_title(&mut self, title: Option<String>) {
        self.x_axis_title = title;
    }

    pub fn set_x_axis_label_position(&mut self, position: LabelPosition) {
        self.x_axis_label_position = position;
    }

    pub fn set_x_axis_tick_spacing(&mut self, spacing: i32) {
        self.x_axis_tick_spacing = Some(spacing);
    }

    pub fn set_x_axis(
        &mut self,
        title: Option<&str>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> CommandExecutionResult {
        if (min.is_some() || max.is_some()) && self.x_axis.auto_scale && self.has_coordinate_pairs()
        {
            return CommandExecutionResult::error(
                "h-axis range must be given before coordinate pair data",
            );
        }
        apply_range(&mut self.x_axis, title, min, max);
        CommandExecutionResult::ok()
    }

    pub fn x_axis_mut(&mut self) -> &mut ChartAxis {
        &mut self.x_axis
    }

    pub fn set_y_axis(
        &mut self,
        title: Option<&str>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> CommandExecutionResult {
        apply_range(&mut self.y_axis, title, min, max);
        CommandExecutionResult::ok()
    }

    pub fn y_axis_mut(&mut self) -> &mut ChartAxis {
        &mut self.y_axis
    }

    pub fn set_y_axis_labels(&mut self, labels: Vec<String>) -> CommandExecutionResult {
        self.y_axis_labels = labels;
        CommandExecutionResult::ok()
    }

    pub fn set_y2_axis(
        &mut self,
        title: Option<&str>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> CommandExecutionResult {
        let axis = self.y2_axis.get_or_insert_with(ChartAxis::default);
        apply_range(axis, title, min, max);
        CommandExecutionResult::ok()
    }

    pub fn y2_axis_mut(&mut self) -> Option<&mut ChartAxis> {
        self.y2_axis.as_mut()
    }

    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    /// Adds a series after checking it against the ones already declared (`addSeries`).
    pub fn add_series(&mut self, series: ChartSeries) -> CommandExecutionResult {
        if series.has_explicit_x_values() {
            if series.series_type != SeriesType::Line && series.series_type != SeriesType::Scatter {
                return CommandExecutionResult::error(
                    "Coordinate pair notation (x:y) is only supported for line and scatter charts",
                );
            }
            if !self.x_axis_labels.is_empty() {
                return CommandExecutionResult::error(
                    "Coordinate pair notation requires numeric h-axis (e.g., h-axis \"x\" -5 --> 5), not categorical labels",
                );
            }
            if !self.x_axis.auto_scale && self.x_axis.max == self.x_axis.min {
                return CommandExecutionResult::error(
                    "Coordinate pair notation requires explicit h-axis range (e.g., h-axis \"x\" -5 --> 5)",
                );
            }
            if let Some(first) = self.series.first() {
                if first.has_explicit_x_values() != series.has_explicit_x_values() {
                    return CommandExecutionResult::error(
                        "All series must use the same data format (either all coordinate pairs or all index-based)",
                    );
                }
            }
            let xs = series.x_values.clone().unwrap_or_default();
            for &x in &xs {
                if !self.x_axis.auto_scale && (x < self.x_axis.min || x > self.x_axis.max) {
                    return CommandExecutionResult::error(format!(
                        "X-coordinate {} is outside h-axis range [{}, {}]",
                        java_double(x),
                        java_double(self.x_axis.min),
                        java_double(self.x_axis.max)
                    ));
                }
            }
            for &x in &xs {
                self.x_axis.include_value(x);
            }
        } else if let Some(first) = self.series.first() {
            if first.has_explicit_x_values() != series.has_explicit_x_values() {
                return CommandExecutionResult::error(
                    "All series must use the same data format (either all coordinate pairs or all index-based)",
                );
            }
        }
        let secondary = series.use_secondary_axis && self.y2_axis.is_some();
        let values: Vec<f64> = series.values.iter().flatten().copied().collect();
        self.series.push(series);
        let axis = match (secondary, self.y2_axis.as_mut()) {
            (true, Some(y2)) => y2,
            _ => &mut self.y_axis,
        };
        for v in values {
            axis.include_value(v);
        }
        CommandExecutionResult::ok()
    }

    pub fn set_legend_position(&mut self, position: LegendPosition) {
        self.legend_position = position;
    }

    pub fn set_grid_mode(&mut self, mode: GridMode) {
        self.x_grid_mode = mode;
        self.y_grid_mode = mode;
    }

    pub fn set_x_grid_mode(&mut self, mode: GridMode) {
        self.x_grid_mode = mode;
    }

    pub fn set_y_grid_mode(&mut self, mode: GridMode) {
        self.y_grid_mode = mode;
    }

    pub fn set_stack_mode(&mut self, mode: StackMode) {
        self.stack_mode = mode;
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    pub fn add_annotation(&mut self, annotation: ChartAnnotation) {
        self.annotations.push(annotation);
    }
}

/// Applies the optional title and range of a `v-axis` or `h-axis` command.
fn apply_range(axis: &mut ChartAxis, title: Option<&str>, min: Option<f64>, max: Option<f64>) {
    if let Some(t) = title {
        axis.title = t.to_string();
    }
    if let Some(min) = min {
        axis.set_min(min);
    }
    if let Some(max) = max {
        axis.set_max(max);
    }
}

/// Formats a double the way Java's `Double.toString` does for the common cases.
fn java_double(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e7 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// Text block of the whole chart (`ChartDiagram.getTextBlock`).
struct ChartBlock {
    renderer: ChartRenderer,
}

impl TextBlock for ChartBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.renderer.calculate_dimension(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        self.renderer.draw_u(ug);
    }
}

impl Diagram for ChartDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        Rc::new(ChartBlock {
            renderer: self.renderer(skin),
        })
    }
}

impl HasBase for ChartDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }
}

impl CommandDiagram for ChartDiagram {}

pub struct ChartDiagramFactory;

impl DiagramFactory for ChartDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Chart
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ChartDiagram::from_source(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
