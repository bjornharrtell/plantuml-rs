//! Rendering and parsing tests of chart diagrams.

use super::renderer::{format_value, java_fixed, java_scientific};
use super::{ChartAxis, LabelPosition};
use crate::render;

fn chart(body: &str) -> String {
    render(&format!("@startchart\n{body}\n@endchart")).unwrap()
}

fn chart_error(body: &str) -> String {
    match render(&format!("@startchart\n{body}\n@endchart")) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected an error for {body:?}"),
    }
}

#[test]
fn renders_bars_with_labels_colors_and_stereotypes() {
    let svg = chart(
        "bar \"Sales\" [25, , 50, -10] #ff0000 labels\n\
         bar <<warn>> \"Other\" [1, 2, 3, 4] #00f\n\
         bar [1,2]",
    );
    assert!(svg.contains("data-diagram-type=\"CHART\""));
    assert!(svg.contains(">50</text>"));
}

#[test]
fn renders_stacked_and_grouped_bars() {
    let body = "h-axis [a, b]\nbar [1, -2] labels\nbar [3, 4] labels\n";
    let grouped = chart(body);
    let stacked = chart(&format!("{body}stackMode stacked\n"));
    assert_ne!(grouped, stacked);
    let stacked_both = chart(&format!("{body}stackMode stacked\nbar [1, 1]\n"));
    assert!(stacked_both.contains("<rect"));
}

#[test]
fn renders_horizontal_bars_with_legend_and_grid() {
    let svg = chart(
        "orientation horizontal\n\
         legend top\n\
         y-axis [x, y, z]\n\
         h-axis \"Count\" 0 --> 100\n\
         bar \"A\" [10, 20, 30] labels\n\
         line \"L\" [5, 6, 7]\n\
         area \"Ar\" [1, 2, 3]\n\
         scatter \"S\" [1, 2, 3] labels <<circle>>\n",
    );
    assert!(svg.contains(">Count</text>"));
    assert!(svg.contains(">x</text>"));
}

#[test]
fn renders_horizontal_bars_with_grid_and_label_position() {
    let svg = chart(
        "orientation horizontal\nlegend bottom\nv-axis \"Items\" grid\nh-axis \"T\" label-right grid\nbar [1, 2]\n",
    );
    assert!(svg.contains("data-diagram-type=\"CHART\""));
}

#[test]
fn renders_vertical_legend_positions() {
    for pos in ["left", "right", "top", "bottom"] {
        let svg = chart(&format!(
            "legend {pos}\nv2-axis \"Two\" 0 --> 10\nbar \"B\" [1, 2] #123456\n\
             line \"L\" [1, 2] y2\nscatter \"S\" [1, 2] <<square>>\n\
             area <<st>> \"A\" [1, 2]\n"
        ));
        assert!(svg.contains(">B</text>"), "legend {pos}");
    }
}

#[test]
fn renders_legend_with_scatter_markers_of_each_shape() {
    for shape in ["circle", "square", "triangle"] {
        let svg = chart(&format!(
            "legend right\nscatter \"P\" [1, 2] <<{shape}>>\nscatter \"Q\" [2, 3] labels\nscatter \"R\" [3, 4] y2 <<triangle>>\n"
        ));
        assert!(svg.contains(">P</text>"));
    }
}

#[test]
fn renders_area_series_stacked_with_labels_and_secondary_axis() {
    let svg = chart(
        "v-axis \"V\" 0 --> 50\nv2-axis \"W\" 0 --> 20\n\
         area \"Base\" [10, 20, 30] labels\narea \"Top\" [5, 5, 5] labels\n\
         area \"Side\" [1, 2, 3] #abcdef y2\nlegend left\n",
    );
    assert!(svg.contains(">Base</text>"));
}

#[test]
fn renders_area_without_labels_and_with_color_only() {
    let svg = chart("area \"A\" [1, 2, 3] #ff8800\narea \"B\" [2, 1]\n");
    assert!(svg.contains("<polygon"));
}

#[test]
fn renders_lines_with_interpolation_connect_and_gaps() {
    let smooth = chart("line \"Smooth\" [1, 3, 2, 4] labels interpolate\n");
    assert!(smooth.contains("<path"));
    let connected = chart("line \"Gap\" [1, , 3, 4] #00ff00 connect\n");
    assert!(connected.contains("<line"));
    let plain = chart("line [1, , 3]\n");
    assert!(plain.contains("<line"));
    let single = chart("line \"One\" [5] interpolate\n");
    assert!(!single.contains("<path"));
}

#[test]
fn renders_coordinate_pair_lines_and_scatters() {
    let svg = chart(
        "h-axis \"x\" -5 --> 5\nv-axis \"y\" 0 --> 10 grid\nv-axis \"x\" grid\n\
         line \"P\" [(-5,1), (0,4), (5,9)] interpolate\n\
         scatter \"Q\" [(-1,2), (3,4)] labels <<triangle>>\n\
         line \"R\" [( -2 , 3 ) , (2,5)] #123 V2 connect\n",
    );
    assert!(svg.contains(">-5</text>") || svg.contains(">x</text>"));
}

#[test]
fn renders_pairs_with_numeric_h_axis_and_grid_ticks() {
    let svg = chart("h-axis \"x\" 0 --> 10 spacing 2 grid\nline [(0,0),(10,5)]\n");
    assert!(svg.contains("<line"));
}

#[test]
fn renders_pair_series_when_h_axis_range_is_auto() {
    let svg = chart("line [(1,1), (3,2), (2,5)]\nscatter [(4,1)]\n");
    assert!(svg.contains("<ellipse"));
}

#[test]
fn renders_single_coordinate_point_range() {
    let svg = chart("line [(4, 2)]\n");
    assert!(svg.contains("data-diagram-type=\"CHART\""));
}

#[test]
fn renders_numeric_h_axis_with_label_position_and_title() {
    let svg = chart("h-axis \"Time\" 0 --> 1\nline [0.5, 0.2]\n");
    assert!(svg.contains(">Time</text>"));
    let right = chart("h-axis \"Time\" 0 --> 1 label-right\nbar [1]\n");
    assert!(right.contains(">Time</text>"));
    let spaced = chart("h-axis \"Time\" 0 --> 1 spacing 3 label-right\nbar [1]\n");
    assert!(spaced.contains(">Time</text>"));
}

#[test]
fn renders_value_axis_variants() {
    let svg = chart(
        "v-axis \"Value\" -10 --> 10 label-top grid\n\
         v-axis ticks [0:\"zero\", 5:\"five\", 0:\"again\"]\n\
         v-axis spacing 5\n\
         v2-axis \"Second\" 0 --> 100 ticks [10:\"ten\"] spacing 20 label-top\n\
         v2-axis [a, b]\n\
         bar [1, -4, 9]\n",
    );
    assert!(svg.contains(">zero</text>") || svg.contains(">again</text>"));
    assert!(svg.contains(">Second</text>"));
}

#[test]
fn renders_value_axis_with_title_and_default_ticks() {
    let svg = chart("v-axis \"Vertical\" 0 --> 1000\nbar [100, 2000]\n");
    assert!(svg.contains(">Vertical</text>"));
}

#[test]
fn renders_axis_labels_and_y2_title_without_secondary_data() {
    let svg = chart("v2-axis \"Tail\" 0 --> 3\nbar [1]\n");
    assert!(svg.contains(">Tail</text>"));
}

#[test]
fn renders_secondary_axis_declared_with_tick_settings() {
    let svg = chart("y2-axis ticks [1:\"one\"] spacing 0.5 label-top\nline [1] y2\n");
    assert!(svg.contains(">one</text>"));
}

#[test]
fn renders_annotations_above_and_below_and_horizontal() {
    let svg = chart(
        "bar [1, 9, 5]\nh-axis [a, b, c]\n\
         annotation \"peak\" at (b, 9) <<arrow>>\n\
         annotation \"low\" at (a, 0.5) <<arrow>>\n\
         annotation \"plain\" at (2.5, 5)\n\
         annotation \"missing\" at (nope, 2) <<arrow>>\n",
    );
    assert!(svg.contains(">peak</text>"));
    assert!(svg.contains(">plain</text>"));
    let horizontal = chart(
        "orientation horizontal\nbar [1, 9]\ny-axis [a, b]\n\
         annotation \"h\" at (a, 2) <<arrow>>\nannotation \"v\" at (3, 4)\n",
    );
    assert!(horizontal.contains(">h</text>"));
}

#[test]
fn renders_annotations_with_numeric_positions_and_arrows() {
    let svg = chart(
        "bar [1, 9, 5]\nh-axis [a, b, c]\n\
         annotation \"near\" at (b, 9) <<arrow>>\n",
    );
    assert!(svg.contains(">near</text>"));
}

#[test]
fn renders_default_empty_chart() {
    assert!(chart_error("").contains("Empty description"));
}

#[test]
fn rejects_malformed_data() {
    assert!(chart_error("area [1, x]").contains("Invalid number format in area data"));
    assert!(chart_error("area [1, , 2]").contains("Invalid number format in area data"));
    assert!(chart_error("bar [1, x]").contains("Invalid number format in bar data"));
    assert!(chart_error("line [1, x]").contains("Invalid number format in line data"));
    assert!(chart_error("scatter [1, , 2]").contains("Invalid number format in scatter data"));
    assert!(chart_error("line [(1,2),(3)]").contains("Invalid coordinate pair format in line data"));
    assert!(
        chart_error("scatter [(1,2,3)]").contains("Invalid coordinate pair format in scatter data")
    );
    assert!(chart_error("line [(a,b)]").contains("Invalid coordinate pair format in line data"));
    assert!(chart_error("line [(,2)]").contains("Invalid coordinate pair format in line data"));
    assert!(chart_error("line [()]").contains("Invalid coordinate pair format in line data"));
}

#[test]
fn rejects_wrong_colors_and_ranges() {
    assert!(chart_error("bar [1] #zzzzzz").contains("No such color"));
    assert!(chart_error("h-axis \"x\" 1.2.3 --> 4").contains("Invalid number format in axis range"));
    assert!(chart_error("v-axis 1.2.3 --> 4").contains("Invalid number format in axis range"));
    assert!(chart_error("v-axis 0 --> 1 ticks [1 \"a\"]").contains("Invalid tick format"));
    assert!(chart_error("v-axis 0 --> 1 ticks [x:\"a\"]").contains("Invalid tick format"));
    assert!(chart_error("v-axis 0 --> 1 ticks [1:a]").contains("Invalid tick format"));
    assert!(chart_error("v-axis spacing 0").contains("Tick spacing must be greater than 0"));
    assert!(chart_error("v-axis spacing 1.2.3").contains("Invalid number format in spacing value"));
    assert!(chart_error("h-axis spacing 0").contains("X-axis spacing must be greater than 0"));
    assert!(chart_error("h-axis spacing 99999999999999")
        .contains("Invalid number format in spacing value"));
    assert!(
        chart_error("annotation \"a\" at (x, abc)").contains("Y position must be a numeric value")
    );
}

#[test]
fn rejects_mixed_and_misplaced_coordinate_pairs() {
    assert!(chart_error("h-axis [a, b]\nline [(1,2)]").contains("not categorical labels"));
    assert!(chart_error("line [(1,2)]\nh-axis [a]").contains("requires numeric h-axis"));
    assert!(chart_error("line [(1,2)]\nh-axis \"x\" 0 --> 5")
        .contains("h-axis range must be given before"));
    assert!(chart_error("h-axis \"x\" 0 --> 5\nline [(9,2)]").contains("outside h-axis range"));
    assert!(chart_error("h-axis \"x\" 5 --> 5\nline [(5,2)]")
        .contains("requires explicit h-axis range"));
    assert!(chart_error("line [(1,2)]\nbar [1]").contains("same data format"));
    assert!(chart_error("bar [1]\nline [(1,2)]").contains("same data format"));
    assert!(
        chart_error("h-axis \"x\" 0 --> 5\nline [1,2]\nline [(1,2)]").contains("same data format")
    );
    assert!(chart_error("area [(1,2)]").contains("Invalid number format in area data"));
}

#[test]
fn reports_syntax_errors_for_unknown_settings() {
    assert!(chart_error("orientation diagonal").contains("Syntax"));
    assert!(chart_error("stackMode mixed").contains("Syntax"));
    assert!(chart_error("annotation oops").contains("Syntax"));
}

#[test]
fn legend_and_orientation_and_stack_commands_apply() {
    let svg = chart("legend left\norientation vertical\nstackMode grouped\nbar [1]\n");
    assert!(svg.contains("data-diagram-type=\"CHART\""));
}

#[test]
fn axis_helpers_map_values_and_detect_ticks() {
    let axis = ChartAxis::with_range("t", 0.0, 10.0);
    assert_eq!(axis.value_to_pixel(5.0, 0.0, 100.0), 50.0);
    let flat = ChartAxis::with_range("t", 3.0, 3.0);
    assert_eq!(flat.value_to_pixel(9.0, 7.0, 100.0), 7.0);
    let mut auto = ChartAxis::default();
    auto.include_value(-4.0);
    auto.include_value(250.0);
    assert_eq!((auto.min, auto.max), (-4.0, 250.0));
    let mut fixed = ChartAxis::with_range("t", 0.0, 1.0);
    fixed.include_value(9.0);
    assert_eq!(fixed.max, 1.0);
    assert!(!auto.has_custom_ticks());
    auto.custom_ticks = Some(Vec::new());
    assert!(!auto.has_custom_ticks());
    auto.tick_spacing = Some(0.0);
    assert!(!auto.has_tick_spacing());
    auto.tick_spacing = Some(2.0);
    assert!(auto.has_tick_spacing());
    assert_eq!(auto.label_position, LabelPosition::Default);
}

#[test]
fn formats_numbers_like_java() {
    assert_eq!(java_fixed(0.125, 2), "0.13");
    assert_eq!(java_fixed(1.005, 2), "1.01");
    assert_eq!(java_fixed(-2.5, 0), "-3");
    assert_eq!(java_fixed(9.999, 2), "10.00");
    assert_eq!(java_fixed(0.0, 2), "0.00");
    assert_eq!(java_fixed(12.0, 0), "12");
    assert_eq!(java_fixed(-0.001, 2), "-0.00");
    assert_eq!(java_fixed(f64::INFINITY, 2), "inf");
    assert_eq!(java_fixed(2.25, 1), "2.3");
    assert_eq!(java_scientific(0.005), "5.00e-03");
    assert_eq!(java_scientific(-0.001), "-1.00e-03");
    assert_eq!(java_scientific(0.0099999), "1.00e-02");
    assert_eq!(java_scientific(0.0012345), "1.23e-03");
    assert_eq!(format_value(0.001), "1.00e-03");
    assert_eq!(format_value(3.0), "3");
    assert_eq!(format_value(-4.0), "-4");
    assert_eq!(format_value(2.5), "2.50");
}
