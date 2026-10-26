use crate::render;

fn salt(body: &str) -> String {
    render(&format!("@startsalt\n{body}\n@endsalt\n")).expect("renders")
}

#[test]
fn table_with_widgets() {
    let svg =
        salt("{\n{#\nName | \"text\"\nMod: | { (X) a | () b | [X] c | [] d }\nGo | [Ok]\n}\n}");
    assert!(svg.contains("data-diagram-type=\"SALT\""));
    assert!(svg.contains("<ellipse"));
    assert!(svg.contains("rx=\"5\""));
}

#[test]
fn menu_tabs_tree_droplist_lines() {
    let svg = salt(
        "{\n{* File | Edit\n File | Open | Save | - | Exit }\n{/ A | B }\n{/ A\nB }\n{T\n+ root\n++ child | cell\n}\n^Pick^^one^two^\n--\n==\n~~\n..\n{S\nx\n}\n{SI\nx\n}\n{S-\nx\n}\n}",
    );
    assert!(svg.contains("Open"));
    assert!(svg.contains("child"));
    assert!(svg.contains("two"));
}

#[test]
fn border_images_and_titles() {
    let svg = salt(
        "{^\"Title\"\n{+\nN= north\nC= mid\nW= w\nE= e\nS= s\n}\n<<pic\nXX\nX.\n>>\n<<pic>>\n}",
    );
    assert!(svg.contains("north"));
    assert!(svg.contains("Title"));
    assert!(svg.contains("<rect"));
}

#[test]
fn merged_cells() {
    let svg = salt("{#\na | b\nc | *\n}");
    assert!(svg.contains(">c<"));
}

#[test]
fn errors() {
    assert!(render("@startsalt\nsalt\n{\n}\n@endsalt\n").is_err());
    assert!(render("@startsalt\njust text\n@endsalt\n").is_err());
    assert!(render("@startsalt\n{\n<<missing>>\n}\n@endsalt\n").is_err());
    assert!(render("@startsalt\n{\n{* A\n B | x }\n}\n@endsalt\n").is_err());
}
