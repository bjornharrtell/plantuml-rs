use plantuml_core::render;

fn attribute<'a>(tag: &'a str, name: &str) -> &'a str {
    tag.split_once(&format!("{name}=\""))
        .expect("attribute")
        .1
        .split_once('"')
        .unwrap()
        .0
}

#[test]
fn c4_person_is_white_and_description_words_have_gaps() {
    let svg = render(
        r#"@startuml
!include <C4/C4_Container>
title Internet banking
Person(customer, "Customer", "A customer of the bank")
System(banking, "Internet Banking System", "Shows balances and payments")
System_Boundary(internal, "Bank") {
  Container(api, "API", "Rust", "Serves requests")
  ContainerDb(db, "Database", "PostgreSQL", "Stores accounts")
}
Rel(customer, banking, "Uses")
Rel(banking, api, "Calls")
Rel(api, db, "Reads")
@enduml"#,
    )
    .unwrap();
    let text_tag = |word: &str| {
        let before = svg.split_once(&format!(">{word}</text>")).expect(word).0;
        before.rsplit_once("<text").unwrap().1
    };
    let a = attribute(text_tag("A"), "x").parse::<f64>().unwrap();
    let customer = attribute(text_tag("customer"), "x").parse::<f64>().unwrap();
    // At 14pt 'A' is ~9.36px; the next word must include a visible space, not touch it.
    assert!(
        customer - a > 12.0,
        "collapsed gap: A={a}, customer={customer}"
    );
    let sprite_runs: Vec<_> = svg
        .split("<rect")
        .skip(1)
        .map(|r| r.split_once('>').unwrap().0)
        .filter(|r| r.contains("height=\"1\""))
        .collect();
    assert!(sprite_runs.len() > 10, "C4 person sprite has pixel rows");
    for row in sprite_runs {
        let fill = attribute(row, "fill");
        assert!(fill == "#FFF" || fill == "#FFFFFF", "dark sprite: {row}");
    }
}

#[test]
fn sprite_uses_inline_font_colour() {
    let svg = render("@startuml\nsprite $dot [1x1/16] {\nF\n}\nrectangle \"<color:red><$dot></color>\" as r\n@enduml").unwrap();
    let row = svg
        .split("<rect")
        .skip(1)
        .map(|r| r.split_once('>').unwrap().0)
        .find(|r| r.contains("height=\"1\""))
        .expect("sprite pixel");
    assert!(
        matches!(attribute(row, "fill"), "#F00" | "#FF0000"),
        "{row}"
    );
}
