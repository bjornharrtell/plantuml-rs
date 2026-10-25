//! Commands of chart diagrams (`chart/command`).

use super::{
    AnnotationX, ChartAnnotation, ChartAxis, ChartDiagram, ChartSeries, GridMode, LabelPosition,
    LegendPosition, MarkerShape, Orientation, SeriesType, StackMode,
};
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::klimt::color::HColor;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::Stereotype;

type Cmd = Box<dyn Command<ChartDiagram>>;

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut ChartDiagram, &RegexResult) -> CommandExecutionResult + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut ChartDiagram, _, arg, _| f(d, arg),
    ))
}

/// `(<<.+?>>)` stereotype, `"name"`, `[data]` and color parts shared by the series commands.
fn optional_stereotype() -> IRegex {
    IRegex::optional(IRegex::named("STEREO", r"(<<.+?>>)"))
}

fn optional_name() -> IRegex {
    IRegex::optional(IRegex::named("NAME", r#""([^"]+)""#))
}

fn data() -> IRegex {
    IRegex::named("DATA", r"\[(.*)\]")
}

fn optional_color() -> IRegex {
    IRegex::optional(IRegex::named(
        "COLOR",
        r"#([0-9a-fA-F]{6}|[0-9a-fA-F]{3}|\w+)",
    ))
}

/// ` word` appended to the command, optional.
fn optional_word(name: &'static str, pattern: &'static str) -> IRegex {
    IRegex::optional(IRegex::group(vec![
        IRegex::space_one_or_more(),
        IRegex::named(name, pattern),
    ]))
}

/// Splits like Java's `String.split(regex)`: trailing empty parts are dropped.
fn java_split<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
    let mut parts: Vec<&str> = s.split(sep).collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

/// Labels of a category list; surrounding quotes are removed.
fn parse_labels(data: Option<&str>) -> Vec<String> {
    let Some(data) = data.filter(|d| !d.trim().is_empty()) else {
        return Vec::new();
    };
    java_split(data, ",")
        .into_iter()
        .map(|part| unquote(part.trim()))
        .collect()
}

fn unquote(label: &str) -> String {
    if label.len() >= 2 && label.starts_with('"') && label.ends_with('"') {
        label[1..label.len() - 1].to_string()
    } else {
        label.to_string()
    }
}

/// Values where an empty entry is a gap (`null`), as for bars and lines.
fn parse_values_with_gaps(data: Option<&str>) -> Option<Vec<Option<f64>>> {
    let Some(data) = data.filter(|d| !d.trim().is_empty()) else {
        return Some(Vec::new());
    };
    java_split(data, ",")
        .into_iter()
        .map(|part| {
            let part = part.trim();
            if part.is_empty() {
                Some(None)
            } else {
                part.parse::<f64>().ok().map(Some)
            }
        })
        .collect()
}

/// Values where every entry must be a number (`area` and `scatter`).
fn parse_values(data: Option<&str>) -> Option<Vec<Option<f64>>> {
    let Some(data) = data.filter(|d| !d.trim().is_empty()) else {
        return Some(Vec::new());
    };
    java_split(data, ",")
        .into_iter()
        .map(|part| part.trim().parse::<f64>().ok().map(Some))
        .collect()
}

/// Parses `(x, y), (x, y)` pairs into the two lists; `false` on any malformed pair.
fn parse_coordinate_pairs(data: &str, xs: &mut Vec<f64>, ys: &mut Vec<Option<f64>>) -> bool {
    if data.trim().is_empty() {
        return false;
    }
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    for pair in java_split(&cleaned, "),(") {
        let trimmed = pair
            .trim()
            .trim_start_matches(['[', '('])
            .trim_end_matches([']', ')']);
        let coords = java_split(trimmed, ",");
        if coords.len() != 2 {
            return false;
        }
        match (
            coords[0].trim().parse::<f64>(),
            coords[1].trim().parse::<f64>(),
        ) {
            (Ok(x), Ok(y)) => {
                xs.push(x);
                ys.push(Some(y));
            }
            _ => return false,
        }
    }
    true
}

/// Custom ticks `value:"label", ...`; `None` when the format is wrong.
fn parse_custom_ticks(ticks: &str) -> Option<Vec<(f64, String)>> {
    let mut result: Vec<(f64, String)> = Vec::new();
    if ticks.trim().is_empty() {
        return Some(result);
    }
    for pair in java_split(ticks, ",") {
        let pair = pair.trim();
        let colon = pair.find(':')?;
        let value_str = pair[..colon].trim();
        let label = pair[colon + 1..].trim();
        if !(label.len() > 1 && label.starts_with('"') && label.ends_with('"')) {
            return None;
        }
        let label = label[1..label.len() - 1].to_string();
        let value = value_str.parse::<f64>().ok()?;
        match result.iter_mut().find(|(v, _)| *v == value) {
            Some(entry) => entry.1 = label,
            None => result.push((value, label)),
        }
    }
    Some(result)
}

fn parse_color(color: &str) -> Result<HColor, CommandExecutionResult> {
    HColor::parse(&format!("#{color}"))
        .ok_or_else(|| CommandExecutionResult::error(format!("No such color: #{color}")))
}

/// The `Stereotype` of a series declaration, when present.
fn stereotype(arg: &RegexResult) -> Option<Stereotype> {
    arg.get("STEREO", 0).map(Stereotype::build)
}

/// Common tail of the series commands: color, secondary axis and labels.
fn apply_series_options(
    series: &mut ChartSeries,
    arg: &RegexResult,
) -> Result<(), CommandExecutionResult> {
    series.stereotype = stereotype(arg);
    if let Some(c) = arg.get("COLOR", 0) {
        series.color = Some(parse_color(c)?);
    }
    if arg.get("V2", 0).is_some() {
        series.use_secondary_axis = true;
    }
    if arg.get("LABELS", 0).is_some() {
        series.show_labels = true;
    }
    Ok(())
}

fn series_name(arg: &RegexResult, prefix: &str, count: usize) -> String {
    arg.get("NAME", 0)
        .map_or_else(|| format!("{prefix}{count}"), str::to_string)
}

/// Builds a series from `[values]` or from `(x, y)` pairs, with the given error texts.
fn series_from_data(
    name: &str,
    series_type: SeriesType,
    data: &str,
    kind: &str,
    strict: bool,
) -> Result<ChartSeries, CommandExecutionResult> {
    if data.contains('(') {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        if !parse_coordinate_pairs(data, &mut xs, &mut ys) {
            return Err(CommandExecutionResult::error(format!(
                "Invalid coordinate pair format in {kind} data"
            )));
        }
        return Ok(ChartSeries::with_pairs(name, series_type, xs, ys));
    }
    let values = if strict {
        parse_values(Some(data))
    } else {
        parse_values_with_gaps(Some(data))
    };
    match values {
        Some(v) => Ok(ChartSeries::new(name, series_type, v)),
        None => Err(CommandExecutionResult::error(format!(
            "Invalid number format in {kind} data"
        ))),
    }
}

fn bar_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("bar"),
            IRegex::space_zero_or_more(),
            optional_stereotype(),
            IRegex::space_zero_or_more(),
            optional_name(),
            IRegex::space_zero_or_more(),
            data(),
            IRegex::space_zero_or_more(),
            optional_color(),
            optional_word("LABELS", "(labels)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let Some(values) = parse_values_with_gaps(arg.get("DATA", 0)) else {
                return CommandExecutionResult::error("Invalid number format in bar data");
            };
            let name = series_name(arg, "bar", d.series_count());
            let mut series = ChartSeries::new(&name, SeriesType::Bar, values);
            if let Err(e) = apply_series_options(&mut series, arg) {
                return e;
            }
            d.add_series(series)
        },
    )
}

fn line_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("line"),
            IRegex::space_zero_or_more(),
            optional_stereotype(),
            IRegex::space_zero_or_more(),
            optional_name(),
            IRegex::space_zero_or_more(),
            data(),
            IRegex::space_zero_or_more(),
            optional_color(),
            optional_word("V2", "([vy]2)"),
            optional_word("LABELS", "(labels)"),
            optional_word("INTERPOLATE", "(interpolate)"),
            optional_word("CONNECT", "(connect)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let name = series_name(arg, "line", d.series_count());
            let data = arg.get("DATA", 0).unwrap_or("");
            let mut series = match series_from_data(&name, SeriesType::Line, data, "line", false) {
                Ok(s) => s,
                Err(e) => return e,
            };
            if let Err(e) = apply_series_options(&mut series, arg) {
                return e;
            }
            series.interpolate = arg.get("INTERPOLATE", 0).is_some();
            series.connect = arg.get("CONNECT", 0).is_some();
            d.add_series(series)
        },
    )
}

fn area_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("area"),
            IRegex::space_zero_or_more(),
            optional_stereotype(),
            IRegex::space_zero_or_more(),
            optional_name(),
            IRegex::space_zero_or_more(),
            data(),
            IRegex::space_zero_or_more(),
            optional_color(),
            optional_word("V2", "([vy]2)"),
            optional_word("LABELS", "(labels)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let name = series_name(arg, "area", d.series_count());
            let Some(values) = parse_values(arg.get("DATA", 0)) else {
                return CommandExecutionResult::error("Invalid number format in area data");
            };
            let mut series = ChartSeries::new(&name, SeriesType::Area, values);
            if let Err(e) = apply_series_options(&mut series, arg) {
                return e;
            }
            d.add_series(series)
        },
    )
}

fn scatter_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("scatter"),
            IRegex::space_zero_or_more(),
            optional_stereotype(),
            IRegex::space_zero_or_more(),
            optional_name(),
            IRegex::space_zero_or_more(),
            data(),
            IRegex::space_zero_or_more(),
            optional_color(),
            optional_word("V2", "([vy]2)"),
            optional_word("LABELS", "(labels)"),
            optional_word("MARKER", "<<(circle|square|triangle)>>"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let name = series_name(arg, "scatter", d.series_count());
            let data = arg.get("DATA", 0).unwrap_or("");
            let mut series =
                match series_from_data(&name, SeriesType::Scatter, data, "scatter", true) {
                    Ok(s) => s,
                    Err(e) => return e,
                };
            if let Err(e) = apply_series_options(&mut series, arg) {
                return e;
            }
            series.marker_shape = match arg.get("MARKER", 0).map(str::to_lowercase).as_deref() {
                Some("square") => MarkerShape::Square,
                Some("triangle") => MarkerShape::Triangle,
                _ => MarkerShape::Circle,
            };
            d.add_series(series)
        },
    )
}

fn h_axis_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("[hx]-axis"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("TITLE", r#""([^"]+)""#)),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("RANGE", r"(-?[0-9.]+)\s*-?->\s*(-?[0-9.]+)")),
            IRegex::space_zero_or_more(),
            IRegex::optional(data()),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf("spacing"),
                IRegex::space_one_or_more(),
                IRegex::named("SPACING", "([0-9]+)"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("LABELRIGHT", "(label-right)")),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("GRID", "(grid)")),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let title = arg.get("TITLE", 0);
            if let Some(spacing) = arg.get("SPACING", 0) {
                match spacing.parse::<i32>() {
                    Ok(s) if s > 0 => d.set_x_axis_tick_spacing(s),
                    Ok(_) => {
                        return CommandExecutionResult::error(
                            "X-axis spacing must be greater than 0",
                        )
                    }
                    Err(_) => {
                        return CommandExecutionResult::error(
                            "Invalid number format in spacing value",
                        )
                    }
                }
            }
            if arg.get("LABELRIGHT", 0).is_some() {
                d.set_x_axis_label_position(LabelPosition::Right);
            }
            if arg.get("GRID", 0).is_some() {
                d.set_x_grid_mode(GridMode::Major);
            }
            if let (Some(min), Some(max)) = (arg.get("RANGE", 0), arg.get("RANGE", 1)) {
                return match (min.parse::<f64>(), max.parse::<f64>()) {
                    (Ok(min), Ok(max)) => d.set_x_axis(title, Some(min), Some(max)),
                    _ => CommandExecutionResult::error("Invalid number format in axis range"),
                };
            }
            d.set_x_axis_title(title.map(str::to_string));
            d.set_x_axis_labels(parse_labels(arg.get("DATA", 0)))
        },
    )
}

fn v_axis_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::named("AXIS", "([vy]2?-axis)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("TITLE", r#""([^"]+)""#)),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("RANGE", r"(-?[0-9.]+)\s*-?->\s*(-?[0-9.]+)")),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("LABELS", r"\[([^\]]+)\]")),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf("ticks"),
                IRegex::space_one_or_more(),
                IRegex::named("TICKS", r"\[(.*)\]"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::group(vec![
                IRegex::leaf("spacing"),
                IRegex::space_one_or_more(),
                IRegex::named("SPACING", "([0-9.]+)"),
            ])),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("LABELTOP", "(label-top)")),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("GRID", "(grid)")),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        execute_v_axis,
    )
}

fn execute_v_axis(d: &mut ChartDiagram, arg: &RegexResult) -> CommandExecutionResult {
    let secondary = arg
        .get("AXIS", 0)
        .is_some_and(|a| a.starts_with("v2") || a.starts_with("y2"));
    let title = arg.get("TITLE", 0);
    if let Some(labels) = arg.get("LABELS", 0) {
        return d.set_y_axis_labels(parse_labels(Some(labels)));
    }
    let (min, max) = match (arg.get("RANGE", 0), arg.get("RANGE", 1)) {
        (Some(min), Some(max)) => match (min.parse::<f64>(), max.parse::<f64>()) {
            (Ok(min), Ok(max)) => (Some(min), Some(max)),
            _ => return CommandExecutionResult::error("Invalid number format in axis range"),
        },
        _ => (None, None),
    };
    let custom_ticks = match arg.get("TICKS", 0) {
        Some(ticks) => match parse_custom_ticks(ticks) {
            Some(t) => Some(t),
            None => {
                return CommandExecutionResult::error(
                    "Invalid tick format. Expected: [value:\"label\", ...]",
                )
            }
        },
        None => None,
    };
    let tick_spacing = match arg.get("SPACING", 0) {
        Some(spacing) => match spacing.parse::<f64>() {
            Ok(s) if s > 0.0 => Some(s),
            Ok(_) => return CommandExecutionResult::error("Tick spacing must be greater than 0"),
            Err(_) => {
                return CommandExecutionResult::error("Invalid number format in spacing value")
            }
        },
        None => None,
    };
    let result = if secondary {
        d.set_y2_axis(title, min, max)
    } else {
        d.set_y_axis(title, min, max)
    };
    let axis: Option<&mut ChartAxis> = if secondary {
        d.y2_axis_mut()
    } else {
        Some(d.y_axis_mut())
    };
    if let Some(axis) = axis {
        if let Some(ticks) = custom_ticks {
            axis.custom_ticks = Some(ticks);
        }
        if let Some(spacing) = tick_spacing {
            axis.tick_spacing = Some(spacing);
        }
        if arg.get("LABELTOP", 0).is_some() {
            axis.label_position = LabelPosition::Top;
        }
    }
    if arg.get("GRID", 0).is_some() {
        d.set_y_grid_mode(GridMode::Major);
    }
    result
}

fn legend_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("legend"),
            IRegex::space_one_or_more(),
            IRegex::named("POSITION", "(left|right|top|bottom)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let position = match arg.get("POSITION", 0).unwrap_or("").to_lowercase().as_str() {
                "left" => LegendPosition::Left,
                "right" => LegendPosition::Right,
                "top" => LegendPosition::Top,
                "bottom" => LegendPosition::Bottom,
                other => {
                    return CommandExecutionResult::error(format!(
                        "Invalid legend position: {other}"
                    ))
                }
            };
            d.set_legend_position(position);
            CommandExecutionResult::ok()
        },
    )
}

fn orientation_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("orientation"),
            IRegex::space_one_or_more(),
            IRegex::named("ORIENTATION", "(vertical|horizontal)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let value = arg.get("ORIENTATION", 0).unwrap_or("");
            let orientation = match value.to_lowercase().as_str() {
                "vertical" => Orientation::Vertical,
                "horizontal" => Orientation::Horizontal,
                _ => return CommandExecutionResult::error(format!("Invalid orientation: {value}")),
            };
            d.set_orientation(orientation);
            CommandExecutionResult::ok()
        },
    )
}

fn stack_mode_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("stackMode"),
            IRegex::space_one_or_more(),
            IRegex::named("MODE", "(grouped|stacked)"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let mode = arg.get("MODE", 0).unwrap_or("");
            let stack = match mode.to_lowercase().as_str() {
                "grouped" => StackMode::Grouped,
                "stacked" => StackMode::Stacked,
                _ => return CommandExecutionResult::error(format!("Invalid stack mode: {mode}")),
            };
            d.set_stack_mode(stack);
            CommandExecutionResult::ok()
        },
    )
}

fn annotation_command() -> Cmd {
    single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf("annotation"),
            IRegex::space_one_or_more(),
            IRegex::named("TEXT", r#""([^"]+)""#),
            IRegex::space_one_or_more(),
            IRegex::leaf("at"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\("),
            IRegex::space_zero_or_more(),
            IRegex::named("XPOS", "([^,]+)"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(","),
            IRegex::space_zero_or_more(),
            IRegex::named("YPOS", "([^)]+)"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\)"),
            IRegex::space_zero_or_more(),
            IRegex::optional(IRegex::named("ARROW", "<<(arrow)>>")),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let text = arg.get("TEXT", 0).unwrap_or("").to_string();
            let x_str = arg.get("XPOS", 0).unwrap_or("").trim().to_string();
            let y_str = arg.get("YPOS", 0).unwrap_or("").trim();
            let x_position = match x_str.parse::<f64>() {
                Ok(v) => AnnotationX::Number(v),
                Err(_) => AnnotationX::Label(x_str.clone()),
            };
            let Ok(y_position) = y_str.parse::<f64>() else {
                return CommandExecutionResult::error(format!(
                    "Y position must be a numeric value: {y_str}"
                ));
            };
            d.add_annotation(ChartAnnotation {
                text,
                x_position,
                y_position,
                show_arrow: arg.get("ARROW", 0).is_some(),
            });
            CommandExecutionResult::ok()
        },
    )
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = vec![
        h_axis_command(),
        v_axis_command(),
        bar_command(),
        line_command(),
        area_command(),
        scatter_command(),
        legend_command(),
        stack_mode_command(),
        orientation_command(),
        annotation_command(),
    ];
    cmds.extend(crate::command::common::common_commands());
    cmds
}
