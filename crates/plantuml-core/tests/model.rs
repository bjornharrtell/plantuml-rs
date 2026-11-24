//! The parsed model behind `create_psystem`, reached by downcasting the diagram.

use plantuml_core::classdiagram::ClassDiagram;
use plantuml_core::command::UmlSource;
use plantuml_core::diagram::Diagram;
use plantuml_core::mindmap::MindMapDiagram;
use plantuml_core::psystem::create_psystem;
use plantuml_core::statediagram::StateDiagram;

fn parse(text: &str) -> Box<dyn Diagram> {
    create_psystem(&UmlSource::from_text(text)).unwrap()
}

fn names(entities: &[plantuml_core::abel::Entity]) -> Vec<String> {
    entities.iter().map(|e| e.name()).collect()
}

#[test]
fn class_diagram_entities_and_links() {
    let diagram = parse("@startuml\nclass A\nclass B\nA <|-- B : extends\n@enduml\n");
    let class = diagram.as_any().downcast_ref::<ClassDiagram>().unwrap();
    assert_eq!(names(&class.cuca.leafs()), ["A", "B"]);
    let links = class.cuca.links();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].entity1().name(), "A");
    assert_eq!(links[0].entity2().name(), "B");
    assert_eq!(links[0].label().text_at(0), "extends");
}

#[test]
fn into_any_gives_the_owned_diagram() {
    let diagram = parse("@startuml\npackage p {\nclass A\n}\n@enduml\n");
    let class = diagram.into_any().downcast::<ClassDiagram>().unwrap();
    assert_eq!(names(&class.cuca.groups()), ["p"]);
}

#[test]
fn description_and_state_diagrams() {
    let diagram = parse("@startuml\n[Web] --> [Api]\n@enduml\n");
    let desc = diagram.as_any().downcast_ref::<ClassDiagram>().unwrap();
    assert!(desc.is_description());
    assert_eq!(names(&desc.cuca.leafs()), ["Web", "Api"]);

    let diagram = parse("@startuml\nstate Idle\nIdle --> Running : go\n@enduml\n");
    let state = diagram.as_any().downcast_ref::<StateDiagram>().unwrap();
    assert_eq!(names(&state.cuca.leafs()), ["Idle", "Running"]);
    assert_eq!(state.cuca.links()[0].label().text_at(0), "go");
}

#[test]
fn mind_map_ideas() {
    let diagram =
        parse("@startmindmap\n* root\n** right\n*** leaf\nleft side\n** left\n@endmindmap\n");
    let mindmap = diagram.as_any().downcast_ref::<MindMapDiagram>().unwrap();
    let map = &mindmap.mindmaps()[0];
    let ideas = &map.regular().ideas;
    assert_eq!(ideas[0].label.text_at(0), "root");
    assert_eq!(ideas[ideas[0].children[0]].label.text_at(0), "right");
    assert_eq!(ideas[2].label.text_at(0), "leaf");
    assert_eq!(map.reverse().ideas[1].label.text_at(0), "left");
}

/// Without Graphviz the diagrams it lays out still parse (above) and render, only empty.
#[cfg(not(feature = "graphviz"))]
#[test]
fn renders_without_graphviz() {
    let svg = plantuml_core::render("@startuml\nAlice -> Bob : hi\n@enduml\n").unwrap();
    assert!(svg.contains(">hi</text>"));
    let svg = plantuml_core::render("@startuml\nclass A\n@enduml\n").unwrap();
    assert!(!svg.contains(">A</text>"));
}
