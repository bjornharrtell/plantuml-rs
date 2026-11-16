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

fn inheritance(group: &str, children: usize) -> String {
    let mut src = format!("@startuml\n{group}class P\n");
    for i in 0..children {
        src.push_str(&format!("class C{i}\nP <|-- C{i}\n"));
    }
    src.push_str("@enduml\n");
    src
}

#[test]
fn group_inheritance_shares_one_triangle() {
    let plain = plantuml_core::render(&inheritance("", 3)).unwrap();
    let grouped = plantuml_core::render(&inheritance("skinparam groupInheritance 2\n", 3)).unwrap();
    assert_eq!(polygons(&plain), 3);
    assert_eq!(polygons(&grouped), 1);
}

#[test]
fn group_inheritance_needs_enough_arrows() {
    let few = plantuml_core::render(&inheritance("skinparam groupInheritance 4\n", 3)).unwrap();
    assert_eq!(polygons(&few), 3);
    let one = plantuml_core::render(&inheritance("skinparam groupInheritance 1\n", 3)).unwrap();
    assert_eq!(polygons(&one), 3);
}
