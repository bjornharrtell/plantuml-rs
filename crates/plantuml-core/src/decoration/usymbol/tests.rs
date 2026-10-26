use super::*;
use crate::klimt::color::HColor;
use crate::klimt::drawing::svg::SvgOption;
use crate::klimt::drawing::ugraphic::{render_svg, UGraphic, UShape};
use crate::klimt::font::{Dimension, EmbeddedFontBounder, StringBounder};
use crate::klimt::shape::{EmptyBlock, TextBlock, UHorizontalLine};
use crate::style::SName;

fn fashion() -> Fashion {
    Fashion::new(HColor::parse("#FEFECE"), HColor::parse("#181818"))
}

fn empty(width: f64, height: f64) -> Rc<dyn TextBlock> {
    Rc::new(EmptyBlock { width, height })
}

fn svg_of(block: &Rc<dyn TextBlock>) -> String {
    render_svg(
        SvgOption::default(),
        0,
        Rc::new(EmbeddedFontBounder),
        |ug| block.draw(ug),
    )
}

fn small(symbol: &Rc<dyn USymbol>, fashion: Fashion) -> Rc<dyn TextBlock> {
    symbol.as_small(
        Some(empty(0.0, 0.0)),
        empty(40.0, 20.0),
        empty(0.0, 0.0),
        fashion,
        HorizontalAlignment::Center,
    )
}

/// A label that occupies its area (the empty blocks leave no footprint).
struct Ink(f64, f64);

impl TextBlock for Ink {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(self.0, self.1)
    }

    fn draw(&self, ug: &UGraphic) {
        ug.draw(UShape::Empty(self.0, self.1));
    }
}

fn small_ink(symbol: &Rc<dyn USymbol>) -> Rc<dyn TextBlock> {
    symbol.as_small(
        None,
        Rc::new(Ink(40.0, 20.0)),
        empty(0.0, 0.0),
        fashion(),
        HorizontalAlignment::Center,
    )
}

fn big(symbol: &Rc<dyn USymbol>, fashion: Fashion) -> Rc<dyn TextBlock> {
    symbol.as_big(
        empty(30.0, 14.0),
        HorizontalAlignment::Center,
        empty(20.0, 10.0),
        100.0,
        60.0,
        fashion,
        HorizontalAlignment::Center,
    )
}

fn dim(block: &Rc<dyn TextBlock>) -> (f64, f64) {
    let d = block.calculate_dimension(&EmbeddedFontBounder);
    (d.width, d.height)
}

#[test]
fn small_dimensions() {
    let cases: Vec<(&str, (f64, f64))> = vec![
        ("rectangle", (60.0, 40.0)),
        ("agent", (60.0, 40.0)),
        ("label", (60.0, 40.0)),
        ("card", (60.0, 26.0)),
        ("collections", (60.0, 40.0)),
        ("storage", (60.0, 40.0)),
        ("action", (70.0, 40.0)),
        ("process", (80.0, 40.0)),
        ("stack", (90.0, 40.0)),
        ("component", (80.0, 50.0)),
        ("component1", (60.0, 40.0)),
        ("artifact", (70.0, 43.0)),
        ("file", (60.0, 40.0)),
        ("package", (70.0, 43.0)),
        ("folder", (70.0, 58.0)),
        ("frame", (80.0, 50.0)),
        ("node", (80.0, 50.0)),
        ("database", (60.0, 49.0)),
        ("queue", (60.0, 30.0)),
        ("cloud", (70.0, 50.0)),
        ("hexagon", (80.0, 30.0)),
        ("actor", (40.0, 81.0)),
        ("boundary", (49.0, 52.0)),
    ];
    for (name, expected) in cases {
        let symbol = from_name(name).unwrap_or_else(|| panic!("no symbol {name}"));
        let (w, h) = dim(&small(&symbol, fashion()));
        assert_eq!((w, h), expected, "{name}");
    }
}

#[test]
fn person_and_usecase_dimensions() {
    let person = usymbols::person();
    let (w, h) = dim(&small(&person, fashion()));
    assert_eq!(w, 60.0);
    assert!((h - (40.0 + 2400f64.sqrt() * 0.42)).abs() < 1e-9);

    let (w, h) = dim(&small_ink(&usymbols::usecase()));
    assert!(w > 40.0 && h > 20.0, "{w} {h}");
    let (wb, _) = dim(&small_ink(&usymbols::usecase_business()));
    assert!(wb > w);
}

#[test]
fn big_dimensions_are_the_requested_ones() {
    let names = [
        "rectangle",
        "label",
        "card",
        "collections",
        "storage",
        "action",
        "process",
        "stack",
        "component",
        "component1",
        "artifact",
        "file",
        "package",
        "folder",
        "frame",
        "node",
        "database",
        "queue",
        "cloud",
        "hexagon",
    ];
    for name in names {
        let symbol = from_name(name).unwrap();
        let block = big(&symbol, fashion());
        assert_eq!(dim(&block), (100.0, 60.0), "{name}");
        let out = svg_of(&block);
        assert!(!out.is_empty(), "{name}");
    }
}

#[test]
fn rectangle_outline_and_text_position() {
    let out = svg_of(&small(&usymbols::rectangle(), fashion()));
    assert!(
        out.contains(r#"height="40""#) && out.contains(r#"width="60""#),
        "{out}"
    );
    assert!(out.contains("#FEFECE") && out.contains("#181818"), "{out}");

    let rounded = fashion().with_corner(10.0, 0.0);
    let out = svg_of(&small(&usymbols::rectangle(), rounded));
    assert!(out.contains(r#"rx="5""#), "{out}");

    let diagonal = fashion().with_corner(0.0, 5.0);
    let out = svg_of(&small(&usymbols::rectangle(), diagonal));
    assert!(out.contains("<path"), "{out}");

    let shadow = fashion().with_delta_shadow(3.0);
    let out = svg_of(&small(&usymbols::rectangle(), shadow));
    assert!(out.contains("url(#"), "{out}");
}

#[test]
fn big_rectangle_and_card() {
    let out = svg_of(&big(&usymbols::rectangle(), fashion()));
    assert!(
        out.contains(r#"height="60""#) && out.contains(r#"width="100""#),
        "{out}"
    );

    let out = svg_of(&big(&usymbols::card(), fashion()));
    // Separator under the title: 14 + 10 + 4.
    assert!(
        out.contains(r#"y1="28""#) && out.contains(r#"x2="100""#),
        "{out}"
    );
}

#[test]
fn collections_draws_two_rectangles() {
    let out = svg_of(&small(&usymbols::collections(), fashion()));
    assert_eq!(out.matches("<rect").count(), 2, "{out}");
    assert!(
        out.contains(r#"x="4""#) && out.contains(r#"width="56""#),
        "{out}"
    );
}

#[test]
fn polygons_of_action_process_artifact_file_node() {
    let out = svg_of(&small(&usymbols::action(), fashion()));
    assert!(
        out.contains(r#"points="0,0,60,0,70,20,60,40,0,40""#),
        "{out}"
    );

    let out = svg_of(&small(&usymbols::process(), fashion()));
    assert!(
        out.contains(r#"points="0,0,70,0,80,20,70,40,0,40,10,20""#),
        "{out}"
    );

    let out = svg_of(&small(&usymbols::artifact(), fashion()));
    assert!(out.contains("<polygon") && out.contains("<line"), "{out}");

    let out = svg_of(&small(&usymbols::file(), fashion()));
    assert!(
        out.contains(r#"points="0,0,0,40,60,40,60,10,50,0,0,0""#),
        "{out}"
    );
    let rounded = svg_of(&small(&usymbols::file(), fashion().with_corner(10.0, 0.0)));
    assert!(
        rounded.contains("<path") && rounded.contains(" A5,5"),
        "{rounded}"
    );

    let out = svg_of(&small(&usymbols::node(), fashion()));
    assert!(
        out.contains(r#"points="0,10,10,0,80,0,80,40,70,50,0,50,0,10""#),
        "{out}"
    );
}

#[test]
fn components_have_their_plugs() {
    let out = svg_of(&small(&usymbols::component1(), fashion()));
    assert_eq!(out.matches("<rect").count(), 3, "{out}");
    assert!(out.contains(r#"x="-5""#), "{out}");

    let out = svg_of(&small(&usymbols::component2(), fashion()));
    assert_eq!(out.matches("<rect").count(), 4, "{out}");
    // Plug at width - 20.
    assert!(out.contains(r#"x="60""#), "{out}");
}

#[test]
fn folder_and_frame_tabs() {
    let out = svg_of(&small(&usymbols::package(), fashion()));
    // wtitle = max(30, 70 / 4) with an empty title, tab height 10.
    assert!(
        out.contains(r#"points="0,0,30,0,37,10,70,10,70,43,0,43,0,0""#),
        "{out}"
    );

    let out = svg_of(&small(
        &usymbols::folder(),
        fashion().with_corner(10.0, 0.0),
    ));
    assert!(out.contains("<path"), "{out}");

    let out = svg_of(&small(&usymbols::frame(), fashion()));
    assert!(out.contains("<rect") && out.contains("<path"), "{out}");
    // Empty title: tab 80 / 3 wide, 12 high, with a corner of 7.
    assert!(out.contains("M26.667,0"), "{out}");
}

#[test]
fn frame_big_draws_title_tab() {
    let out = svg_of(&big(&usymbols::frame(), fashion()));
    // Title is 30x14: tab 40 wide, 17 high, corner 10.
    assert!(out.contains("M40,0"), "{out}");
}

#[test]
fn database_and_queue_outlines() {
    let out = svg_of(&small(&usymbols::database(), fashion()));
    assert!(out.contains("M0,10"), "{out}");
    assert_eq!(out.matches("<path").count(), 2, "{out}");

    let out = svg_of(&small(&usymbols::queue(), fashion()));
    assert!(out.contains("M5,0"), "{out}");
    assert_eq!(out.matches("<path").count(), 2, "{out}");
}

/// A label made of a horizontal line.
struct LineBlock {
    style: char,
}

impl TextBlock for LineBlock {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(40.0, 20.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let line = UHorizontalLine::infinite(1.0, self.style, None);
        ug.apply_translate(crate::klimt::drawing::ugraphic::UTranslate::dy(10.0))
            .draw(UShape::HLine(line));
    }
}

fn with_line(symbol: &Rc<dyn USymbol>, style: char) -> String {
    let label: Rc<dyn TextBlock> = Rc::new(LineBlock { style });
    let block = symbol.as_small(
        None,
        label,
        empty(0.0, 0.0),
        fashion(),
        HorizontalAlignment::Center,
    );
    svg_of(&block)
}

#[test]
fn horizontal_lines_follow_the_shape() {
    // Database: the line is replaced by the curved bottom, 15 above the line.
    let out = with_line(&usymbols::database(), '-');
    assert_eq!(out.matches("<path").count(), 3, "{out}");
    let out = with_line(&usymbols::database(), '=');
    assert_eq!(out.matches("<path").count(), 4, "{out}");

    // Node: line and diagonal.
    let out = with_line(&usymbols::node(), '-');
    assert!(
        out.contains(r#"x2="70""#) || out.contains(r#"x2="80""#),
        "{out}"
    );
    let doubled = with_line(&usymbols::node(), '=');
    assert!(doubled.matches("<line").count() > out.matches("<line").count());

    // Queue: the line ends on the rounded side (default stroke style).
    let out = with_line(&usymbols::queue(), '-');
    assert!(out.contains("<line"), "{out}");

    // Usecase: lines are clipped by the ellipse.
    let out = with_line(&usymbols::usecase(), '-');
    assert!(out.contains("<ellipse") && out.contains("<line"), "{out}");
}

#[test]
fn cloud_is_reproducible() {
    let a = svg_of(&small(&usymbols::cloud(), fashion()));
    let b = svg_of(&small(&usymbols::cloud(), fashion()));
    assert_eq!(a, b);
    assert!(a.contains("<path") && a.contains(" C"), "{a}");
    let path = cloud::cloud_path(150.0, 150.0);
    assert!(path.segments.len() > 20);
    // Zero-length sides must not panic.
    let _ = cloud::cloud_path(16.0, 16.0);
    let _ = cloud::cloud_path(0.0, 0.0);
}

#[test]
fn figures_draw_their_shapes() {
    let out = svg_of(&small(&usymbols::actor_stickman(), fashion()));
    assert!(out.contains("<ellipse") && out.contains("<path"), "{out}");
    let out = svg_of(&small(&usymbols::actor_stickman_business(), fashion()));
    assert!(out.contains("<line"), "{out}");
    for name in [
        "actor_awesome",
        "actor_hollow",
        "control",
        "entity",
        "interface",
    ] {
        let out = svg_of(&small(&from_name(name).unwrap(), fashion()));
        assert!(out.contains("<"), "{name}");
    }
    let out = svg_of(&small(
        &usymbols::boundary(),
        fashion().with_delta_shadow(3.0),
    ));
    assert!(out.contains("<ellipse"), "{out}");
    let circle = svg_of(&small(&usymbols::interface(), fashion()));
    assert!(circle.contains(r#"rx="8""#), "{circle}");
}

#[test]
fn person_and_usecase_outlines() {
    let out = svg_of(&small(&usymbols::person(), fashion()));
    assert!(out.contains("<ellipse") && out.contains("<rect"), "{out}");
    let out = svg_of(&small_ink(&usymbols::usecase()));
    // Ellipse of alpha 0.5 around a 40x20 text, then 6 bigger.
    assert!(out.contains("<ellipse"), "{out}");
    let out = svg_of(&small_ink(&usymbols::usecase_business()));
    assert!(out.contains("<ellipse") && out.contains("<line"), "{out}");
}

#[test]
fn figures_have_no_big_form() {
    for name in ["actor", "person", "usecase", "boundary"] {
        let symbol = from_name(name).unwrap();
        assert_eq!(dim(&big(&symbol, fashion())), (100.0, 60.0), "{name}");
    }
}

#[test]
fn registry() {
    assert!(from_name("nothing").is_none());
    assert_eq!(
        from_name("USECASE/").unwrap().snames(),
        vec![SName::Usecase]
    );
    assert_eq!(
        from_name("Usecase_Business").unwrap().snames(),
        vec![SName::Usecase, SName::Business]
    );
    assert_eq!(from_name("package").unwrap().snames(), vec![SName::Package]);
    assert_eq!(
        from_name("circle").unwrap().snames(),
        vec![SName::Interface]
    );
    assert_eq!(from_name("entity").unwrap().snames(), vec![SName::Entity]);
    assert_eq!(
        from_name("component").unwrap().snames(),
        vec![SName::Component]
    );
    assert_eq!(
        from_string(
            "actor",
            ActorStyle::StickmanBusiness,
            ComponentStyle::Uml1,
            PackageStyle::Folder
        )
        .unwrap()
        .snames(),
        vec![SName::Business]
    );
    assert!(from_string(
        "package",
        ActorStyle::Stickman,
        ComponentStyle::Uml2,
        PackageStyle::Agent
    )
    .is_none());
    for (name, sname) in [
        ("ARCHIMATE", SName::Archimate),
        ("group", SName::Group),
        ("partition", SName::Partition),
        ("hexagon", SName::Hexagon),
        ("stack", SName::Stack),
    ] {
        assert_eq!(from_name(name).unwrap().snames(), vec![sname], "{name}");
    }
}

#[test]
fn keywords_and_styles() {
    for keyword in [
        "artifact",
        "folder",
        "file",
        "package",
        "rectangle",
        "person",
        "hexagon",
        "label",
        "collections",
        "node",
        "frame",
        "cloud",
        "action",
        "process",
        "database",
        "queue",
        "stack",
        "storage",
        "agent",
        "actor/",
        "actor",
        "component",
        "boundary",
        "control",
        "entity",
        "card",
        "interface",
        "()",
    ] {
        assert!(
            from_keyword(keyword, ActorStyle::Hollow, ComponentStyle::Rectangle).is_some(),
            "{keyword}"
        );
    }
    assert!(from_keyword("unknown", ActorStyle::Hollow, ComponentStyle::Uml2).is_none());
    assert_eq!(
        PackageStyle::from_string("RECT"),
        Some(PackageStyle::Rectangle)
    );
    assert_eq!(PackageStyle::from_string("nope"), None);
    for name in [
        "folder",
        "rectangle",
        "node",
        "frame",
        "cloud",
        "database",
        "agent",
        "storage",
        "component1",
        "component2",
        "artifact",
        "card",
    ] {
        let style = PackageStyle::from_string(name).unwrap();
        let symbol = style.to_usymbol();
        let expected = !matches!(
            name,
            "agent" | "storage" | "component1" | "component2" | "artifact"
        );
        assert_eq!(symbol.is_some(), expected, "{name}");
    }
    for style in [
        ComponentStyle::Uml1,
        ComponentStyle::Uml2,
        ComponentStyle::Rectangle,
    ] {
        assert_eq!(style.to_usymbol().snames(), vec![SName::Component]);
    }
    for style in [
        ActorStyle::Stickman,
        ActorStyle::StickmanBusiness,
        ActorStyle::Awesome,
        ActorStyle::Hollow,
    ] {
        assert!(!style.to_usymbol().snames().is_empty());
    }
}

#[test]
fn supplementary_sizes() {
    assert_eq!(usymbols::node().supp_height_because_of_shape(), 5);
    assert_eq!(usymbols::node().supp_width_because_of_shape(), 60);
    assert_eq!(usymbols::database().supp_height_because_of_shape(), 15);
    assert_eq!(usymbols::database().supp_width_because_of_shape(), 0);
    assert_eq!(usymbols::rectangle().supp_height_because_of_shape(), 0);
}

#[test]
fn legacy_small_forms() {
    let block =
        crate::decoration::symbol::database_small(empty(16.0, 17.0), empty(0.0, 0.0), fashion());
    assert_eq!(dim(&block), (36.0, 46.0));
    let block =
        crate::decoration::symbol::queue_small(empty(16.0, 17.0), empty(0.0, 0.0), fashion());
    assert_eq!(dim(&block), (36.0, 27.0));
}

#[test]
fn hexagon_small_only_places_text() {
    let symbol = usymbols::hexagon();
    let block = small(&symbol, fashion());
    let out = svg_of(&block);
    assert!(!out.contains("<path"), "{out}");
}
