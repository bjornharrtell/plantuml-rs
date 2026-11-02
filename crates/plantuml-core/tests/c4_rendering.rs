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
    let pixels = sprite_pixels(&svg);
    assert!(pixels.len() > 10, "C4 person sprite is a bitmap");
    // opaque pixels are white; transparent ones keep whatever colour the sprite is drawn in
    assert!(
        pixels
            .iter()
            .filter(|p| p[3] > 0)
            .all(|p| p[..3] == [255, 255, 255]),
        "dark sprite"
    );
}

/// The RGBA pixels of the first `<image>`: base64 -> PNG -> the single stored deflate block.
fn sprite_pixels(svg: &str) -> Vec<[u8; 4]> {
    let uri = svg
        .split_once("data:image/png;base64,")
        .expect("an image")
        .1
        .split('"')
        .next()
        .unwrap();
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bytes = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in uri.bytes().filter(|c| *c != b'=') {
        acc = (acc << 6) | table.iter().position(|t| *t == c).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((acc >> bits) as u8);
        }
    }
    let idat = bytes.windows(4).position(|w| w == b"IDAT").unwrap() + 4;
    // zlib header (2) + stored block header (5) + the filter byte of the first row
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap()) as usize;
    let data = &bytes[idat + 7..];
    let row = width * 4 + 1;
    data.chunks(row)
        .take(height)
        .flat_map(|r| r[1..].chunks(4).map(|p| [p[0], p[1], p[2], p[3]]))
        .collect()
}

#[test]
fn sprite_uses_inline_font_colour() {
    let svg = render("@startuml\nsprite $dot [1x1/16] {\nF\n}\nrectangle \"<color:red><$dot></color>\" as r\n@enduml").unwrap();
    assert_eq!(sprite_pixels(&svg), vec![[255, 0, 0, 255]], "{svg}");
}
