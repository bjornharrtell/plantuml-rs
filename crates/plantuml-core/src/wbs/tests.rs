use crate::render;

fn svg(body: &str) -> String {
    render(&format!("@startwbs\n{body}\n@endwbs")).expect("renders")
}

#[test]
fn left_and_right_children() {
    let s = svg("* root\n** a\n*** a1\n** b\n--- x\n**< c");
    for t in [">root<", ">a<", ">a1<", ">b<", ">c<"] {
        assert!(s.contains(t), "{t}");
    }
}

#[test]
fn links_between_coded_elements() {
    let s = svg("* root\n**(a) A\n**(b) B\na -> b #red <<st>>");
    assert!(s.contains("<polygon"));
    assert!(render("@startwbs\n* r\n**(a) A\na -> zz\n@endwbs").is_err());
    assert!(render("@startwbs\n* r\n**(a) A\nzz -> a\n@endwbs").is_err());
}

#[test]
fn as_form_multiline_stereotype_and_old_order() {
    let s = svg(
        "* root\n**[#pink] \"Quoted\" as q\n**:l1\nl2; <<st>>\n**(c)[#pink]<:old\nx;\n** label <<st>>",
    );
    assert!(s.contains(">Quoted<") && s.contains(">l1<"));
}

#[test]
fn pseudo_node_and_autowidth() {
    let s = svg(
        "<style>\nwbsDiagram {\n node {\n width auto\n }\n}\n</style>\n* P\n**_\n*** A\n*** Longer name\n** B",
    );
    assert!(s.contains(">Longer name<"));
}

#[test]
fn errors() {
    assert!(render("@startwbs\n* a\n* b\n@endwbs").is_err());
    assert!(render("@startwbs\n* a\n**** x\n@endwbs").is_err());
}

#[test]
fn smart_level() {
    let mut d =
        super::WbsDiagram::from_source(crate::command::UmlSource::from_text("@startwbs\n@endwbs"));
    assert_eq!(d.get_smart_level("*"), Ok(0));
    d.add_idea_text(None, None, 0, "r", false, crate::mindmap::IdeaShape::Box);
    assert_eq!(d.get_smart_level("**"), Ok(1));
    assert_eq!(d.get_smart_level("\t*"), Ok(1));
    assert!(d.get_smart_level("x y").is_err());
}
