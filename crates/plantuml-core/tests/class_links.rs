//! Direction markers on link labels (`: label >` and `: label <`).

fn polygons(svg: &str) -> usize {
    svg.matches("<polygon").count()
}

#[test]
fn label_direction_draws_a_triangle() {
    let plain = plantuml_core::render("@startuml\nA -- B : crea\n@enduml\n").unwrap();
    let forward = plantuml_core::render("@startuml\nA -- B : crea >\n@enduml\n").unwrap();
    let backward = plantuml_core::render("@startuml\nA -- B : crea <\n@enduml\n").unwrap();
    assert_eq!(polygons(&forward), polygons(&plain) + 1);
    assert_eq!(polygons(&backward), polygons(&plain) + 1);
    assert!(forward.contains(">crea<"));
}

#[test]
fn direction_marker_without_text() {
    let svg = plantuml_core::render("@startuml\nA -- B : >\n@enduml\n").unwrap();
    assert_eq!(polygons(&svg), 1);
}

#[test]
fn each_line_of_a_multiline_label_has_its_own_marker() {
    let svg =
        plantuml_core::render("@startuml\nA -- B : one >\\n< two\\nplain\n@enduml\n").unwrap();
    assert_eq!(polygons(&svg), 2);
    assert!(svg.contains(">plain<"));
}
