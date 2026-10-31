use crate::render;

fn svg(body: &str) -> String {
    render(&format!("@startuml\n{body}\n@enduml")).expect("renders")
}

#[test]
fn two_actions_are_joined_by_an_arrow() {
    let out = svg(":X;\n:Y;");
    assert!(out.contains(r#"data-diagram-type="ACTIVITY""#));
    assert_eq!(out.matches("<rect").count(), 2);
    assert_eq!(out.matches("<polygon").count(), 1);
    assert!(out.contains(">X</text>") && out.contains(">Y</text>"));
    // the 35 pixels of space are squeezed to 20 by the vertical compression
    let boxes: Vec<(f64, f64)> = out
        .match_indices("<rect")
        .map(|(i, _)| {
            let tag = &out[i..];
            let tag = &tag[..tag.find('>').unwrap()];
            (number(tag, "y").unwrap(), number(tag, "height").unwrap())
        })
        .collect();
    assert_eq!(boxes.len(), 2, "{out}");
    let gap = boxes[1].0 - (boxes[0].0 + boxes[0].1);
    assert!((gap - 20.0).abs() < 0.01, "gap {gap}: {out}");
}

#[test]
fn start_stop_and_end() {
    let out = svg("start\n:a;\nstop");
    assert_eq!(out.matches("<ellipse").count(), 3);
    let out = svg("start\n:a;\nend");
    assert!(out.contains("<ellipse") && out.matches("<line").count() >= 3);
    assert!(render("@startuml\nstart;\nstop;\n@enduml").is_ok());
}

#[test]
fn arrow_label_and_color() {
    let out = svg(":a;\n-> some text;\n:b;");
    assert!(out.contains(">some text</text>"));
    let out = svg(":a;\n-[#red]->\n:b;");
    assert!(out.contains("#F00") || out.contains("#FF0000"), "{out}");
    let out = svg(":a;\n-[#red]-> label;\n:b;");
    assert!(out.contains(">label</text>") && out.contains("#F00"));
    let out = svg(":a;\n-> first\nsecond;\n:b;");
    assert!(out.contains(">first</text>") && out.contains(">second</text>"));
    let out = svg(":a;\nlink #blue\n:b;");
    assert!(out.contains("#00F"));
    let out = svg(":a;\n-[#red;#blue]->\n:b;");
    assert!(out.contains("#F00") && out.contains("#00F"));
    let out = svg(":a;\n-[hidden]->\n:b;");
    assert!(!out.contains("<polygon"));
}

#[test]
fn multi_line_action_and_box_styles() {
    let out = svg(":first line\nsecond line;");
    assert!(out.contains(">first line</text>") && out.contains(">second line</text>"));
    for style in [
        "input",
        "output",
        "procedure",
        "load",
        "save",
        "continuous",
        "task",
        "object",
    ] {
        let out = svg(&format!(":x; <<{style}>>"));
        assert!(out.contains(">x</text>"), "{style}");
    }
    for style in [
        "objectSignal",
        "trigger",
        "sendSignal",
        "acceptEvent",
        "timeEvent",
    ] {
        let out = svg(&format!(":x; <<{style}>>"));
        assert!(out.contains(">x</text>"), "{style}");
    }
    assert!(svg("- item\n* other").contains(">other</text>"));
}

#[test]
fn colors_and_warnings() {
    let out = svg(":X; <<#palegreen>>");
    assert!(out.contains("#98FB98"));
    let out = svg("#red:X;");
    assert!(out.contains("deprecated"), "{out}");
    let out = svg("<<foo>>:X;");
    assert!(out.contains("ignored"));
    let out = svg("#red:first\nsecond;");
    assert!(out.contains("must"));
    let out = svg(":foo1; <<FontColor:red;BackGroundColor:blue;>>");
    assert!(out.contains("#00F") && out.contains(r##"fill="#F00""##));
}

#[test]
fn chrome_and_ignored_commands() {
    let out = svg("title My title\nhide footbox\n:X;");
    assert!(out.contains(">My title</text>"));
    let out = svg("skinparam backgroundColor yellow\n:X;");
    assert!(out.contains("#FF0"));
}

#[test]
fn shared_factory_order_keeps_other_types() {
    let out = render("@startuml\nA -> B: hi\n@enduml").unwrap();
    assert!(out.contains("SEQUENCE"));
    let out = render("@startuml\nclass A\n@enduml").unwrap();
    assert!(out.contains("CLASS"));
}

#[test]
fn syntax_error_is_reported() {
    assert!(render("@startuml\n:X;\nbogus words here\n@enduml").is_err());
}

#[test]
fn urls_are_parsed() {
    let out = svg(":X; [[http://example.com]]");
    assert!(out.contains(">X</text>"));
}

#[test]
fn groups_and_partitions_wrap_activities() {
    let out = svg("group G {\n:A;\n}");
    assert!(out.contains(">G</text>"), "{out}");
    assert!(out.contains(">A</text>"));
    let out = svg("partition \"My part\"\n:A;\n}");
    assert!(out.contains(">My part</text>"), "{out}");
    assert!(out.contains("Missing"), "{out}");
    let out = svg("group G {\n:A;\nend group");
    assert!(out.contains("deprecated"));
    assert!(out.contains(">A</text>"));
}

#[test]
fn unbalanced_group_close_is_an_error() {
    assert!(render("@startuml\n}\n@enduml").is_err());
}

#[test]
fn fork_draws_two_bars_and_branches() {
    let out = svg("fork\n:A;\nfork again\n:B;\nend fork");
    assert!(out.contains(">A</text>") && out.contains(">B</text>"));
    assert!(out.contains("#555"));
    assert_eq!(out.matches("fill=\"#555\"").count(), 2);
    let out = svg("fork\n-> right;\n:A;\nend fork");
    assert!(out.contains(">right</text>"));
}

#[test]
fn fork_again_outside_fork_is_an_error() {
    assert!(render("@startuml\nfork again\n@enduml").is_err());
    assert!(render("@startuml\nend fork\n@enduml").is_err());
}

#[test]
fn while_loop_renders_condition_and_body() {
    let out = svg("while (more?)\n:A;\nendwhile");
    assert!(out.contains(">more?</text>"));
    assert!(out.contains(">A</text>"));
    assert!(out.contains("<polygon"));
    let out = svg("#red:while (more?)\n:A;\nendwhile");
    assert!(out.contains("deprecated"));
}

#[test]
fn while_loop_keeps_the_loop_back_and_the_exit_apart() {
    let out = svg("while (more?)\n:A;\nendwhile\n:B;");
    let attr = |tag: &str, name: &str| number(tag, name).unwrap();
    let tags = |name: &str| -> Vec<String> {
        out.match_indices(name)
            .map(|(i, _)| out[i..out[i..].find('>').unwrap() + i].to_string())
            .collect()
    };
    let boxes = tags("<rect");
    let (body, next) = (&boxes[0], &boxes[1]);
    let body_bottom = attr(body, "y") + attr(body, "height");
    // horizontal runs of the loop-back and of the exit, by y
    let mut ys: Vec<f64> = tags("<line")
        .iter()
        .filter(|l| (attr(l, "y1") - attr(l, "y2")).abs() < 1e-9 && attr(l, "x1") != attr(l, "x2"))
        .map(|l| attr(l, "y1"))
        .filter(|y| *y > body_bottom)
        .collect();
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    assert_eq!(ys.len(), 2, "{out}");
    assert!((ys[0] - body_bottom - 10.0).abs() < 0.01, "{ys:?}");
    assert!((ys[1] - body_bottom - 22.0).abs() < 0.01, "{ys:?}");
    assert!((attr(next, "y") - ys[1] - 20.0).abs() < 0.01, "{out}");
    // both vertical runs of the loop carry a direction arrow in the middle
    assert!(out.matches("<polygon").count() >= 5, "{out}");
}

#[test]
fn elseif_conditions_sit_side_by_side_without_a_merge_diamond() {
    let out = svg("if (c?) then (yes)\n:A;\nelseif (d?) then (maybe)\n:B;\nelse (no)\n:C;\nendif");
    for text in ["c?", "d?", "yes", "maybe", "no", "A", "B", "C"] {
        assert!(out.contains(&format!(">{text}</text>")), "{text}: {out}");
    }
    // two six-point condition diamonds; a merge diamond would make three (and arrowheads have four points)
    let diamonds = out
        .match_indices("points=\"")
        .filter(|(i, _)| out[i + 8..].split('"').next().unwrap().split(',').count() == 12)
        .count();
    assert_eq!(diamonds, 2, "{out}");
}

#[test]
fn elseif_after_else_is_an_error() {
    assert!(render(
        "@startuml\nif (a?) then\n:A;\nelse\n:B;\nelseif (b?) then\n:C;\nendif\n@enduml"
    )
    .is_err());
}

#[test]
fn while_loop_draws_its_yes_and_no_labels() {
    let out = svg("while (more?) is (go)\n:A;\nendwhile (done)");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    // `no` sits to the left of the condition, `yes` under it, to the right of the centre line
    assert!(x("done") < x("more?"), "{out}");
    assert!(x("go") > x("more?"), "{out}");
}

#[test]
fn while_loop_draws_its_backward_box_on_the_right() {
    let out = svg("while (more?)\n:step;\nbackward :undo;\nendwhile");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    assert!(x("undo") > x("step"), "{out}");
    assert!(render("@startuml\n:a;\nbackward :b;\n@enduml").is_err());
}

#[test]
fn break_leaves_the_loop_through_the_exit_channel() {
    let out = svg("while (more?)\nif (bad?) then (yes)\nbreak\nendif\n:work;\nendwhile\n:done;");
    for text in ["more?", "bad?", "work", "done"] {
        assert!(out.contains(&format!(">{text}</text>")), "{text}: {out}");
    }
    // the exit channel is the leftmost vertical line; the break joins it with a horizontal one
    let xs: Vec<f64> = out
        .match_indices("<line")
        .map(|(i, _)| &out[i..out[i..].find('>').unwrap() + i])
        .filter(|l| (number(l, "y1").unwrap() - number(l, "y2").unwrap()).abs() < 1e-9)
        .map(|l| number(l, "x2").unwrap())
        .collect();
    let channel = xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!(
        xs.iter().filter(|x| (**x - channel).abs() < 0.01).count() >= 2,
        "{xs:?}"
    );
    assert!(render("@startuml\nbreak\n@enduml").is_ok());
}

#[test]
fn if_else_places_both_branches_under_the_condition_and_merges_them() {
    let out = svg("if (c?) then (yes)\n:A;\nelse (no)\n:B;\nendif\n:C;");
    let polygons: Vec<Vec<f64>> = out
        .match_indices("points=\"")
        .map(|(i, _)| {
            out[i + 8..]
                .split('"')
                .next()
                .unwrap()
                .split(',')
                .map(|n| n.parse().unwrap())
                .collect()
        })
        .collect();
    let hexagon = polygons.iter().find(|p| p.len() == 12).expect("condition");
    let bottom = hexagon[7];
    let boxes: Vec<(f64, f64)> = out
        .match_indices("<rect")
        .map(|(i, _)| {
            let tag = &out[i..out[i..].find('>').unwrap() + i];
            (number(tag, "x").unwrap(), number(tag, "y").unwrap())
        })
        .collect();
    // both branches start 10 below the condition, one on each side of its centre
    assert!((boxes[0].1 - bottom - 10.0).abs() < 0.01 && (boxes[1].1 - bottom - 10.0).abs() < 0.01);
    assert!(boxes[0].0 < hexagon[0] && boxes[1].0 > hexagon[0], "{out}");
    // the merge diamond has four points
    assert!(polygons
        .iter()
        .any(|p| p.len() == 8 && (p[0] - p[4]).abs() < 24.1 && p[1] < p[3]));
}

#[test]
fn if_without_else_routes_the_other_path_round_the_branch() {
    let out = svg("if (ok?) then (yes)\n:A;\nendif\n:B;");
    let y_of = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "y").unwrap()
    };
    // the `yes` label sits under the condition, above the branch
    assert!(
        y_of("yes") > y_of("ok?") && y_of("yes") < y_of("A"),
        "{out}"
    );
    // the path past the branch is one polyline: out of the east vertex, down, back into the merge
    let verticals = out
        .match_indices("<line")
        .map(|(i, _)| &out[i..out[i..].find('>').unwrap() + i])
        .filter(|l| number(l, "x1") == number(l, "x2"))
        .count();
    assert!(verticals >= 4, "{out}");
}

#[test]
fn switch_places_the_cases_in_a_row_under_the_test_and_merges_them() {
    let out = svg("switch (k?)\ncase (a)\n:A;\ncase (b)\n:B;\ncase (c)\n:C;\nendswitch\n:D;");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    assert!(x("A") < x("B") && x("B") < x("C"), "{out}");
    // the labels sit on the arrows into the cases
    for label in ["a", "b", "c"] {
        assert!(out.contains(&format!(">{label}</text>")), "{label}: {out}");
    }
    let one = svg("switch (k?)\ncase (a)\n:A;\nendswitch\n:D;");
    assert!(
        one.contains(">A</text>") && one.contains(">a</text>"),
        "{one}"
    );
}

#[test]
fn repeat_loop_draws_its_backward_box_and_its_labels() {
    let out = svg("repeat\n:step;\nbackward :undo;\nrepeat while (again?) is (yes) not (done)");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    assert!(x("undo") > x("step"), "{out}");
    assert!(
        x("yes") > x("again?") && out.contains(">done</text>"),
        "{out}"
    );
    // a break in a repeat loop leads to the diamond after it
    let out = svg("repeat\nif (bad?) then (yes)\nbreak\nendif\nrepeat while (again?)\n:after;");
    for text in ["bad?", "again?", "after"] {
        assert!(out.contains(&format!(">{text}</text>")), "{text}: {out}");
    }
}

#[test]
fn break_in_an_if_keeps_the_exit_path_20px_above_the_next_tile() {
    let out = svg("while (more?)\nif (bad?) then (yes)\nbreak\nendif\n:work;\nendwhile");
    let lines: Vec<(f64, f64, f64, f64)> = out
        .match_indices("<line")
        .map(|(i, _)| &out[i..out[i..].find('>').unwrap() + i])
        .map(|l| {
            let n = |k| number(l, k).unwrap();
            (n("x1"), n("y1"), n("x2"), n("y2"))
        })
        .collect();
    let work = out.find(">work</text>").unwrap();
    let work_box = out[..work].rfind("<rect").unwrap();
    let top = number(
        &out[work_box..out[work_box..].find('>').unwrap() + work_box],
        "y",
    )
    .unwrap();
    // the arrow into the next tile is 20 long: the else path ends on the bottom of the `if` tile
    assert!(
        lines
            .iter()
            .any(|l| l.0 == l.2 && (l.3 - top).abs() < 0.01 && (l.3 - l.1 - 20.0).abs() < 0.01),
        "{out}"
    );
}

#[test]
fn notes_sit_beside_the_latest_activity() {
    let out = svg(":A;\nnote right: on the right\n:B;\nnote left: on the left");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    assert!(x("on the right") > x("A"), "{out}");
    assert!(x("on the left") < x("B"), "{out}");
    // a multi-line note and a floating one
    let out = svg(":A;\nnote left\n  first\n  second\nend note\nfloating note right: loose");
    for text in ["first", "second", "loose"] {
        assert!(out.contains(&format!(">{text}</text>")), "{text}: {out}");
    }
}

#[test]
fn a_note_inside_a_branch_belongs_to_the_activity_before_it() {
    let out = svg("if (a?) then (yes)\n:A;\nnote right: about A\nelse (no)\n:B;\nendif");
    let x = |t: &str| {
        let i = out
            .find(&format!(">{t}</text>"))
            .unwrap_or_else(|| panic!("{t}: {out}"));
        number(&out[out[..i].rfind("<text").unwrap()..i], "x").unwrap()
    };
    // beside A, still left of B
    assert!(
        x("about A") > x("A") && x("about A") < x("B") + 60.0,
        "{out}"
    );
    // a note after the `if` sits beside the condition, not beside the whole tile
    let out = svg("if (a?) then (yes)\n:A;\nendif\nnote left: after");
    assert!(out.contains(">after</text>"), "{out}");
}

#[test]
fn endwhile_outside_loop_is_an_error() {
    assert!(render("@startuml\nendwhile\n@enduml").is_err());
}

#[test]
fn repeat_loop_renders_body_and_condition() {
    let out = svg("repeat\n:A;\nrepeat while (more?)");
    assert!(out.contains(">A</text>") && out.contains(">more?</text>"));
    let out = svg("repeat :R;\n:A;\nrepeat while (more?)");
    assert!(out.contains(">R</text>") && out.contains(">A</text>"));
    let out = svg("#red:repeat\n:A;\nrepeat while (more?)");
    assert!(out.contains("deprecated"));
}

#[test]
fn repeat_while_outside_loop_is_an_error() {
    assert!(render("@startuml\nrepeat while (x?)\n@enduml").is_err());
}

#[test]
fn if_else_renders_two_branches() {
    let out = svg("if (c?) then (yes)\n:A;\nelse (no)\n:B;\nendif");
    assert!(out.contains(">c?</text>") && out.contains(">A</text>") && out.contains(">B</text>"));
    assert!(out.contains(">yes</text>") && out.contains(">no</text>"));
    let out = svg("if (c?) is (v) then\n:A;\nelse\n:B;\nendif");
    assert!(out.contains(">v</text>"));
    let out = svg("if (c?) then (yes)\n:A;\nelseif (d?) then (maybe)\n:B;\nelse (no)\n:C;\nendif");
    assert!(out.contains(">A</text>") && out.contains(">B</text>") && out.contains(">C</text>"));
}

#[test]
fn endif_outside_if_is_an_error() {
    assert!(render("@startuml\nendif\n@enduml").is_err());
    assert!(render("@startuml\nelse\n@enduml").is_err());
}

#[test]
fn switch_renders_cases_and_labels() {
    let out = svg("switch (x?)\ncase (a)\n:A;\ncase (b)\n:B;\nendswitch");
    assert!(out.contains(">x?</text>"));
    assert!(out.contains(">a</text>") && out.contains(">b</text>"));
    assert!(out.contains(">A</text>") && out.contains(">B</text>"));
    let out = svg("switch (x?)\ncase (a)\n:A;\ncase (b)\n:B;\ncase (c)\n:C;\nendswitch");
    assert!(out.contains(">C</text>"));
}

#[test]
fn switch_commands_outside_switch_are_errors() {
    assert!(render("@startuml\ncase (a)\n@enduml").is_err());
    assert!(render("@startuml\nendswitch\n@enduml").is_err());
}

#[test]
fn legacy_start_stop_and_arrows_render() {
    let out = svg(
        "(*) --> Verify\nif \"\" then\n-> [incorrect] Send\n--> (*)\nelse\n--> [correct] Get\n--> ===Y1===\nendif\n===Y1=== --> Print\n--> ===Y2===\n--> Give\n--> (*)",
    );
    for text in [
        "Verify",
        "Send",
        "Get",
        "Print",
        "Give",
        "incorrect",
        "correct",
    ] {
        assert!(out.contains(text), "missing {text}: {out}");
    }
}

fn number(tag: &str, name: &str) -> Option<f64> {
    let key = format!(" {name}=\"");
    tag.split_once(&key)?.1.split_once('"')?.0.parse().ok()
}

/// Every polygon of the drawing as its points.
fn polygons(svg: &str) -> Vec<Vec<(f64, f64)>> {
    svg.match_indices("<polygon")
        .map(|(i, _)| {
            let tag = &svg[i..];
            let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
            let points = tag.split_once("points=\"").map(|(_, v)| v).unwrap_or("");
            let points = points.split_once('"').map(|(v, _)| v).unwrap_or("");
            let numbers: Vec<f64> = points
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter_map(|v| v.parse().ok())
                .collect();
            numbers
                .chunks(2)
                .filter(|p| p.len() == 2)
                .map(|p| (p[0], p[1]))
                .collect()
        })
        .collect()
}

fn text_x(svg: &str, content: &str) -> Option<(f64, f64)> {
    let i = svg.find(&format!(">{content}</text>"))?;
    let start = svg[..i].rfind("<text")?;
    let tag = &svg[start..i];
    Some((number(tag, "x")?, number(tag, "textLength")?))
}

#[test]
fn if_labels_sit_beside_the_condition_and_stay_on_top() {
    let out = svg("if (Graphviz installed?) then (yes)\n  :a;\nelse (no)\n  :b;\nendif");
    let hexagon = polygons(&out)
        .into_iter()
        .find(|p| p.len() == 6)
        .expect("condition hexagon");
    let min_x = hexagon.iter().map(|p| p.0).fold(f64::MAX, f64::min);
    let max_x = hexagon.iter().map(|p| p.0).fold(f64::MIN, f64::max);
    let (yes_x, yes_w) = text_x(&out, "yes").expect("yes label");
    let (no_x, _) = text_x(&out, "no").expect("no label");
    // Java right-aligns `yes` on the west vertex and left-aligns `no` on the east vertex.
    assert!(
        (yes_x + yes_w - min_x).abs() < 1.0,
        "yes label at {yes_x}+{yes_w}, vertex at {min_x}"
    );
    assert!(
        (no_x - max_x).abs() < 1.0,
        "no label at {no_x}, vertex at {max_x}"
    );
    // The condition shape must be drawn before the labels, or it covers the `no` text.
    let hex_at = out.find("<polygon").expect("polygon");
    let no_at = out.find(">no</text>").expect("no label");
    assert!(no_at > hex_at, "the condition covers the no label");
}

#[test]
fn merge_diamond_meets_the_return_arrows() {
    let out = svg("if (c?) then (yes)\n  :a;\nelse (no)\n  :b;\nendif");
    let diamond = polygons(&out)
        .into_iter()
        .find(|p| p.len() == 4)
        .expect("merge diamond");
    let min_x = diamond.iter().map(|p| p.0).fold(f64::MAX, f64::min);
    let max_x = diamond.iter().map(|p| p.0).fold(f64::MIN, f64::max);
    let min_y = diamond.iter().map(|p| p.1).fold(f64::MAX, f64::min);
    let max_y = diamond.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    let middle_y = (min_y + max_y) / 2.0;

    let horizontal: Vec<(f64, f64, f64, f64)> = out
        .match_indices("<line")
        .filter_map(|(i, _)| {
            let tag = &out[i..];
            let tag = &tag[..tag.find('>')?];
            let (y1, y2) = (number(tag, "y1")?, number(tag, "y2")?);
            if (y1 - y2).abs() > 0.01 || (y1 - middle_y).abs() > 0.01 {
                return None;
            }
            Some((
                number(tag, "x1")?,
                number(tag, "y1")?,
                number(tag, "x2")?,
                number(tag, "y2")?,
            ))
        })
        .collect();
    let left = horizontal
        .iter()
        .any(|l| (l.2 - min_x).abs() < 0.5 && l.2 > l.0);
    let right = horizontal
        .iter()
        .any(|l| (l.2 - max_x).abs() < 0.5 && l.2 < l.0);
    assert!(
        left && right,
        "return arrows do not reach the diamond ({min_x}..{max_x} at y={middle_y}): {horizontal:?}"
    );
}
