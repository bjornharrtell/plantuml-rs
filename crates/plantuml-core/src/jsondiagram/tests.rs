use crate::render;

#[test]
fn json_boxes_and_links() {
    let svg = render(
        "@startjson\n#highlight \"b\"\n{\"a\":1,\"b\":[true,false,null,{\"x\":\"y\"}],\"c\":{},\"d\":[]}\n@endjson",
    )
    .unwrap();
    assert!(svg.contains("data-diagram-type=\"JSON\""));
    assert!(svg.contains(">y</text>") && svg.contains("<ellipse"));
    assert!(svg.contains("<path"));
}

#[test]
fn scalars_and_empty_roots() {
    for s in ["42", "\"s\"", "null", "{}", "[]"] {
        let svg = render(&format!("@startjson\n{s}\n@endjson")).unwrap();
        assert!(svg.contains("<rect"), "{s}");
    }
}

#[test]
fn bad_data_gives_a_message() {
    let svg = render("@startjson\n{ nope\n@endjson").unwrap();
    assert!(
        svg.contains("Your") && svg.contains("JSON\u{a0}data"),
        "{svg}"
    );
}

#[test]
fn title_scale_and_style() {
    let svg = render(
        "@startyaml\ntitle My yaml\nscale 2\n<style>\nnode {\n  BackGroundColor: red\n}\n</style>\na: 1\nb:\n  - c\n  - d: 2\n@endyaml",
    )
    .unwrap();
    assert!(svg.contains("data-diagram-type=\"YAML\"") && svg.contains("My"));
    assert!(svg.contains("#F00"));
}

#[test]
fn hcl_renders() {
    let svg =
        render("@starthcl\nmodule \"m\" {\n  a = \"1\"\n  b = { c = \"d\" }\n}\n@endhcl").unwrap();
    assert!(svg.contains("data-diagram-type=\"HCL\"") && svg.contains(">d</text>"));
    let bad = render("@starthcl\n{{\n@endhcl").unwrap();
    assert!(bad.contains("HCL\u{a0}data"));
}
