//! Links that name a member (`Foo::field1 --> Bar::field3`) attach to that member's row.

use plantuml_core::render;

/// A `<rect>` of an entity: x, y, width, height.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn contains_y(&self, y: f64) -> bool {
        y > self.y && y < self.y + self.height
    }
}

fn class_boxes(svg: &str) -> Vec<Rect> {
    svg.match_indices("<rect")
        .filter_map(|(i, _)| {
            let tag = &svg[i..];
            let tag = &tag[..tag.find('>')?];
            if !tag.contains("fill=\"#F1F1F1\"") {
                return None;
            }
            let value = |name: &str| -> Option<f64> {
                tag.split_once(&format!(" {name}=\""))?
                    .1
                    .split_once('"')?
                    .0
                    .parse()
                    .ok()
            };
            Some(Rect {
                x: value("x")?,
                y: value("y")?,
                width: value("width")?,
                height: value("height")?,
            })
        })
        .collect()
}

/// The start and end point of every link path, in document order.
fn link_ends(svg: &str) -> Vec<((f64, f64), (f64, f64))> {
    svg.match_indices("<path d=\"M")
        .map(|(i, _)| {
            let d = &svg[i + "<path d=\"".len()..];
            let d = &d[..d.find('"').unwrap_or(d.len())];
            let numbers: Vec<f64> = d
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            let start = (numbers[0], numbers[1]);
            let end = (numbers[numbers.len() - 2], numbers[numbers.len() - 1]);
            (start, end)
        })
        .collect()
}

const TWO_FIELDS: &str = r#"@startuml
class Foo {
+ field1
+ field2
}

class Bar {
+ field3
+ field4
}

Foo::field1 --> Bar::field3 : foo
Foo::field2 --> Bar::field4 : bar
@enduml"#;

#[test]
fn member_links_attach_to_the_member_rows() {
    let svg = render(TWO_FIELDS).unwrap();
    let boxes = class_boxes(&svg);
    assert_eq!(boxes.len(), 2, "two class boxes");
    let (foo, bar) = (boxes[0], boxes[1]);
    let ends = link_ends(&svg);
    assert_eq!(ends.len(), 2, "two links: {svg}");
    for (start, end) in &ends {
        // The old behaviour attached both ends to the box border (Foo's bottom, Bar's top).
        assert!(
            foo.contains_y(start.1),
            "link starts on the top or bottom edge of Foo: {:?} in {:?}",
            start,
            foo
        );
        assert!(
            bar.contains_y(end.1),
            "link ends on the top or bottom edge of Bar: {:?} in {:?}",
            end,
            bar
        );
    }
    // With two links they take one side each, as the Graphviz port table makes it route around the box.
    let left = ends.iter().any(|(start, _)| (start.0 - foo.x).abs() < 2.0);
    let right = ends
        .iter()
        .any(|(start, _)| (start.0 - (foo.x + foo.width)).abs() < 2.0);
    assert!(
        left && right,
        "links do not leave Foo on both sides: {ends:?}"
    );
    // Each link leaves from its own row, and reaches the matching row of the other class.
    assert!(
        (ends[0].0 .1 - ends[1].0 .1).abs() > 1.0,
        "both links leave the same row: {ends:?}"
    );
    assert!(
        (ends[0].1 .1 - ends[1].1 .1).abs() > 1.0,
        "both links reach the same row: {ends:?}"
    );
}

#[test]
fn a_method_link_attaches_below_the_fields() {
    let svg = render(
        r#"@startuml
class Foo {
+ field1
+ method1()
}

class Bar {
+ field2
}

Foo::method1 --> Bar::field2
@enduml"#,
    )
    .unwrap();
    let boxes = class_boxes(&svg);
    assert_eq!(boxes.len(), 2);
    let field_link = render(
        r#"@startuml
class Foo {
+ field1
}

class Bar {
+ field2
}

Foo::field1 --> Bar::field2
@enduml"#,
    )
    .unwrap();
    let field_start = link_ends(&field_link)[0].0 .1;
    let method_start = link_ends(&svg)[0].0 .1;
    assert!(
        method_start > field_start + 1.0,
        "method row {method_start} is not below the field row {field_start}"
    );
    assert!(boxes[0].contains_y(method_start));
}

#[test]
fn object_member_links_attach_to_the_field_rows() {
    let svg = render(
        r#"@startuml
object Foo {
+ field1
+ field2
}
object Bar {
+ field3
}
Foo::field2 --> Bar::field3
@enduml"#,
    )
    .unwrap();
    let boxes = class_boxes(&svg);
    assert_eq!(boxes.len(), 2, "two object boxes: {svg}");
    let ends = link_ends(&svg);
    assert_eq!(ends.len(), 1);
    assert!(
        boxes[0].contains_y(ends[0].0 .1) && boxes[1].contains_y(ends[0].1 .1),
        "object link is not on a field row: {ends:?} in {boxes:?}"
    );
}

#[test]
fn a_link_to_a_missing_member_still_renders() {
    let svg = render(
        r#"@startuml
class Foo {
+ field1
}
class Bar
Foo::nope --> Bar
@enduml"#,
    )
    .unwrap();
    assert!(svg.contains("<svg"), "no svg: {svg}");
    assert_eq!(link_ends(&svg).len(), 1);
}
