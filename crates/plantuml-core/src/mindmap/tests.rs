use crate::render;

fn svg(body: &str) -> String {
    render(&format!("@startmindmap\n{body}\n@endmindmap")).expect("renders")
}

#[test]
fn both_sides_with_plus_and_minus() {
    let s = svg("+ root\n++ right\n-- left\n++ more");
    for t in [">root<", ">right<", ">left<", ">more<"] {
        assert!(s.contains(t), "{t}");
    }
}

#[test]
fn top_to_bottom_and_default_direction() {
    let s = svg("top to bottom direction\nleft side\n* root\n** a\n** b");
    assert!(s.contains(">a<") && s.contains(">b<"));
}

#[test]
fn multiline_boxless_color_and_stereotype() {
    let s = svg("*[#lightblue] root\n**_ plain\n**:line one\nline two; <<st>>");
    assert!(s.contains("line one") && s.contains("line two"));
    assert!(s.contains("#ADD8E6"));
}

#[test]
fn several_roots_are_stacked() {
    let s = svg("* one\n** a\n* two\n** b");
    assert!(s.contains(">one<") && s.contains(">two<"));
}

#[test]
fn bad_indentation_is_an_error() {
    assert!(render("@startmindmap\n* r\n** a\n**** c\n@endmindmap").is_err());
}

#[test]
fn smart_level_with_tabs_and_spaces() {
    let mut d = super::MindMapDiagram::from_source(crate::command::UmlSource::from_text(
        "@startmindmap\n@endmindmap",
    ));
    assert_eq!(d.get_smart_level("*"), Ok(0));
    assert_eq!(d.get_smart_level("**"), Ok(1));
    assert_eq!(d.get_smart_level("\t*"), Ok(1));
    assert!(d.get_smart_level("x y").is_err());
}
