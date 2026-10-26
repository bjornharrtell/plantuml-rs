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
    assert!(out.contains(r#"y="68""#), "{out}");
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
