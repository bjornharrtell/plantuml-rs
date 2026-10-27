//! Description diagrams (component, C4) are laid out with the Graphviz flow's separation: PlantUML's
//! `DotStringFactory` keeps at least 60 px between ranks, where the Smetana flow leaves Graphviz's
//! 36 px default.

use plantuml_core::render;

fn entity_boxes(svg: &str) -> Vec<(f64, f64)> {
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
            Some((value("y")?, value("height")?))
        })
        .collect()
}

#[test]
fn description_ranks_are_sixty_pixels_apart() {
    let svg = render(
        r#"@startuml
rectangle "A" as a
rectangle "B" as b
rectangle "C" as c
a --> b
b --> c
@enduml"#,
    )
    .unwrap();
    let mut boxes = entity_boxes(&svg);
    boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(boxes.len(), 3, "three rectangles: {svg}");
    for pair in boxes.windows(2) {
        let (top, height) = pair[0];
        let below = pair[1].0;
        let gap = below - (top + height);
        assert!(
            (gap - 60.0).abs() < 1.0,
            "rank separation is {gap} px, not 60: {boxes:?}"
        );
    }
}
