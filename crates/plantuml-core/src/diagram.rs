//! State common to all diagrams (`TitledDiagram`, `AbstractDiagram`).

use crate::command::UmlSource;
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::Margins;
use crate::style::SName;
use crate::text::LineLocation;
use std::rc::Rc;

/// The kind of diagram (`DiagramType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramType {
    Sequence,
    State,
    Class,
    Object,
    Activity,
    Description,
    Composite,
    Timing,
    Help,
    Bpm,
    Ditaa,
    Dot,
    Jcckit,
    Salt,
    Flow,
    Creole,
    Math,
    Latex,
    Definition,
    Gantt,
    Chronology,
    Nwdiag,
    Mindmap,
    Wbs,
    Wire,
    Json,
    Git,
    Board,
    Yaml,
    Hcl,
    Ebnf,
    Regex,
    Files,
    ChenEer,
    Chart,
    Packet,
    Sprites,
    Crash,
    Unknown,
}

impl DiagramType {
    /// Value of `data-diagram-type` in the SVG (the Java constant name).
    pub fn svg_name(self) -> &'static str {
        match self {
            DiagramType::Sequence => "SEQUENCE",
            DiagramType::State => "STATE",
            DiagramType::Class => "CLASS",
            DiagramType::Object => "OBJECT",
            DiagramType::Activity => "ACTIVITY",
            DiagramType::Description => "DESCRIPTION",
            DiagramType::Composite => "COMPOSITE",
            DiagramType::Timing => "TIMING",
            DiagramType::Help => "HELP",
            DiagramType::Bpm => "BPM",
            DiagramType::Ditaa => "DITAA",
            DiagramType::Dot => "DOT",
            DiagramType::Jcckit => "JCCKIT",
            DiagramType::Salt => "SALT",
            DiagramType::Flow => "FLOW",
            DiagramType::Creole => "CREOLE",
            DiagramType::Math => "MATH",
            DiagramType::Latex => "LATEX",
            DiagramType::Definition => "DEFINITION",
            DiagramType::Gantt => "GANTT",
            DiagramType::Chronology => "CHRONOLOGY",
            DiagramType::Nwdiag => "NWDIAG",
            DiagramType::Mindmap => "MINDMAP",
            DiagramType::Wbs => "WBS",
            DiagramType::Wire => "WIRE",
            DiagramType::Json => "JSON",
            DiagramType::Git => "GIT",
            DiagramType::Board => "BOARD",
            DiagramType::Yaml => "YAML",
            DiagramType::Hcl => "HCL",
            DiagramType::Ebnf => "EBNF",
            DiagramType::Regex => "REGEX",
            DiagramType::Files => "FILES",
            DiagramType::ChenEer => "CHEN_EER",
            DiagramType::Chart => "CHART",
            DiagramType::Packet => "PACKET",
            DiagramType::Sprites => "SPRITES",
            DiagramType::Crash => "CRASH",
            DiagramType::Unknown => "UNKNOWN",
        }
    }

    /// Element name used in the style sheet (`getStyleName`).
    pub fn style_name(self) -> SName {
        match self {
            DiagramType::Sequence => SName::SequenceDiagram,
            DiagramType::State => SName::StateDiagram,
            DiagramType::Class => SName::ClassDiagram,
            DiagramType::Object => SName::ObjectDiagram,
            DiagramType::Activity => SName::ActivityDiagram,
            DiagramType::Description => SName::ComponentDiagram,
            DiagramType::Mindmap => SName::MindmapDiagram,
            DiagramType::Wbs => SName::WbsDiagram,
            DiagramType::Gantt | DiagramType::Chronology => SName::GanttDiagram,
            DiagramType::Salt => SName::SaltDiagram,
            DiagramType::Yaml | DiagramType::Hcl => SName::YamlDiagram,
            DiagramType::Json => SName::JsonDiagram,
            DiagramType::Timing => SName::TimingDiagram,
            DiagramType::Nwdiag => SName::NwdiagDiagram,
            DiagramType::Ebnf => SName::Ebnf,
            DiagramType::Regex => SName::Regex,
            DiagramType::Chart => SName::ChartDiagram,
            DiagramType::Packet => SName::PacketdiagDiagram,
            _ => SName::ActivityDiagram,
        }
    }

    /// Diagram types a `@startxxx` line may stand for (`findStartTypes`).
    pub fn find_start_types(text: &str) -> Vec<DiagramType> {
        let chars: Vec<char> = text.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if c.is_whitespace() {
                continue;
            }
            if *c != '@' && *c != '\\' {
                return Vec::new();
            }
            let pos = i + 1;
            if chars.len() < pos + 5 || !check("start", &chars, pos) {
                return Vec::new();
            }
            let p = pos + 5;
            if p >= chars.len() {
                return Vec::new();
            }
            return types_at(&chars, p);
        }
        Vec::new()
    }
}

fn check(key: &str, text: &[char], p: usize) -> bool {
    let key: Vec<char> = key.chars().collect();
    if p + key.len() > text.len() {
        return false;
    }
    key.iter()
        .enumerate()
        .all(|(i, k)| text[p + i].to_ascii_lowercase() == *k)
}

fn types_at(text: &[char], p: usize) -> Vec<DiagramType> {
    use DiagramType::*;
    let table: &[(&str, &[DiagramType])] = &[
        ("bpm", &[Bpm]),
        ("board", &[Board]),
        ("chart", &[Chart]),
        ("creole", &[Creole]),
        ("chronology", &[Chronology]),
        ("chen", &[ChenEer]),
        ("crash", &[Crash]),
        ("dot", &[Dot]),
        ("ditaa", &[Ditaa]),
        ("def", &[Definition]),
        ("ebnf", &[Ebnf]),
        ("flow", &[Flow]),
        ("files", &[Files]),
        ("gantt", &[Gantt]),
        ("git", &[Git]),
        ("hcl", &[Hcl]),
        ("jcckit", &[Jcckit]),
        ("json", &[Json]),
        ("latex", &[Latex]),
        ("math", &[Math]),
        ("mindmap", &[Mindmap]),
        ("nwdiag", &[Nwdiag]),
        ("project", &[Gantt]),
        ("packetdiag", &[Packet]),
        ("regex", &[Regex]),
        ("salt", &[Salt]),
        ("sprites", &[Sprites]),
        (
            "uml",
            &[
                Sequence,
                State,
                Class,
                Object,
                Activity,
                Description,
                Composite,
                Timing,
                Help,
                Sprites,
            ],
        ),
        ("wire", &[Wire]),
        ("wbs", &[Wbs]),
        ("yaml", &[Yaml]),
    ];
    let first = text[p].to_ascii_lowercase();
    for (key, types) in table {
        if key.starts_with(first) && check(key, text, p) {
            return types.to_vec();
        }
    }
    vec![Unknown]
}

/// Title, header, footer, legend and caption of a diagram.
#[derive(Debug, Clone)]
pub struct DiagramBase {
    pub source: UmlSource,
    pub diagram_type: DiagramType,
    pub skin_param: SkinParam,
    pub title: DisplayPositioned,
    pub caption: DisplayPositioned,
    pub legend: DisplayPositioned,
    pub header: DisplayPositioned,
    pub footer: DisplayPositioned,
    pub main_frame: DisplayPositioned,
    pub scale: Option<Scale>,
    pub pragma: std::collections::HashMap<String, String>,
    /// Warnings shown in a banner, one entry per warning with its lines.
    pub warnings: Vec<Vec<String>>,
    pub hide_empty_description: bool,
}

/// Requested scaling of the image (`Scale`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    Factor(f64),
    Width(f64),
    Height(f64),
    MaxWidth(f64),
    MaxHeight(f64),
    WidthAndHeight(f64, f64),
    MaxWidthAndHeight(f64, f64),
}

impl Scale {
    /// Factor to apply to an image of the given size (`getScale`).
    pub fn factor(&self, width: f64, height: f64) -> f64 {
        match *self {
            Scale::Factor(f) => f,
            Scale::Width(w) => w / width,
            Scale::Height(h) => h / height,
            Scale::MaxWidth(w) => (w / width).min(1.0),
            Scale::MaxHeight(h) => (h / height).min(1.0),
            Scale::WidthAndHeight(w, h) => (w / width).min(h / height),
            Scale::MaxWidthAndHeight(w, h) => (w / width).min(h / height).min(1.0),
        }
    }
}

impl DiagramBase {
    pub fn new(source: UmlSource, diagram_type: DiagramType) -> Self {
        Self {
            source,
            diagram_type,
            skin_param: {
                let mut skin = SkinParam::new();
                skin.diagram_style = Some(diagram_type.style_name());
                skin
            },
            title: DisplayPositioned::none(
                Some(HorizontalAlignment::Center),
                Some(VerticalAlignment::Top),
            ),
            caption: DisplayPositioned::none(
                Some(HorizontalAlignment::Center),
                Some(VerticalAlignment::Bottom),
            ),
            legend: DisplayPositioned::none(
                Some(HorizontalAlignment::Center),
                Some(VerticalAlignment::Bottom),
            ),
            header: DisplayPositioned::none(Some(HorizontalAlignment::Center), None),
            footer: DisplayPositioned::none(Some(HorizontalAlignment::Center), None),
            main_frame: DisplayPositioned::none(None, None),
            scale: None,
            pragma: std::collections::HashMap::new(),
            warnings: Vec::new(),
            hide_empty_description: false,
        }
    }

    pub fn add_warning(&mut self, lines: Vec<String>) {
        self.warnings.push(lines);
    }

    pub fn pragma_is_true(&self, key: &str) -> bool {
        self.pragma
            .get(&key.to_lowercase())
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
    }

    /// Sets a title; blank titles are ignored (`setTitle`).
    pub fn set_title(&mut self, title: DisplayPositioned) {
        if title.is_null() || title.display.is_white() {
            return;
        }
        self.title = title;
    }

    pub fn update_footer(
        &mut self,
        location: Option<LineLocation>,
        display: Display,
        align: HorizontalAlignment,
    ) {
        self.footer = self
            .footer
            .with_display(display)
            .with_horizontal_alignment(align)
            .with_location(location);
    }

    pub fn update_header(
        &mut self,
        location: Option<LineLocation>,
        display: Display,
        align: HorizontalAlignment,
    ) {
        self.header = self
            .header
            .with_display(display)
            .with_horizontal_alignment(align)
            .with_location(location);
    }

    /// `skinparam key value` (key lowercased like `TitledDiagram.setParam`).
    pub fn set_param(&mut self, key: &str, value: &str) {
        self.skin_param.set_param(&key.to_lowercase(), value);
    }

    /// Background of the image (`calculateBackColor`).
    pub fn calculate_back_color(&self) -> crate::klimt::color::HColor {
        let query = crate::style::queries::DOCUMENT.add(self.diagram_type.style_name());
        self.skin_param
            .style_builder()
            .merged_style(&query)
            .map_or(crate::klimt::color::HColor::TRANSPARENT, |s| {
                s.value(crate::style::PName::BackGroundColor).as_color()
            })
    }

    /// Source title found by the `title` command (`UmlSource.getTitle`).
    pub fn source_title(&self) -> Display {
        let re = regex::Regex::new(r"(?i)^\s*title\s+(.+)$").expect("valid");
        for l in &self.source.lines {
            if let Some(m) = re.captures(l.string()) {
                return Display::create(&[&m[1]]);
            }
        }
        Display::empty()
    }
}

/// A parsed diagram that can be drawn (`Diagram`, `UgDiagram`).
pub trait Diagram {
    fn base(&self) -> &DiagramBase;
    fn base_mut(&mut self) -> &mut DiagramBase;

    /// Lays the content out (`getTextBlock`).
    fn text_block(&self, sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock>;

    fn default_margins(&self) -> Margins {
        Margins::same(10.0)
    }

    /// Title shown above the drawing.
    fn title(&self) -> DisplayPositioned {
        self.base().title.clone()
    }
}

/// Builds the diagrams of one type from their source (`PSystemFactory`).
pub trait DiagramFactory {
    fn diagram_type(&self) -> DiagramType;

    /// Parses `source`; an error means the source is not of this type.
    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, crate::command::ErrorUml>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_scale() {
        let mut d = DiagramBase::new(
            UmlSource::from_text("@startuml\ntitle Hello\n@enduml"),
            DiagramType::Sequence,
        );
        assert!(d.title.is_null());
        d.set_title(DisplayPositioned::single(
            Display::create(&[" "]),
            None,
            None,
        ));
        assert!(d.title.is_null());
        d.set_title(DisplayPositioned::single(
            Display::create(&["T"]),
            None,
            None,
        ));
        assert!(!d.title.is_null());
        assert_eq!(d.source_title().text_at(0), "Hello");
        d.update_header(None, Display::create(&["h"]), HorizontalAlignment::Right);
        d.update_footer(None, Display::create(&["f"]), HorizontalAlignment::Left);
        assert_eq!(
            d.header.horizontal_alignment,
            Some(HorizontalAlignment::Right)
        );
        assert_eq!(d.footer.display.text_at(0), "f");
        d.set_param("PADDING", "3");
        assert_eq!(d.skin_param.get_value("padding"), Some("3"));
        assert_eq!(d.diagram_type.svg_name(), "SEQUENCE");
        assert_eq!(DiagramType::Class.style_name(), SName::ClassDiagram);
        assert_eq!(
            d.calculate_back_color().rgba(),
            crate::klimt::color::HColor::WHITE.rgba()
        );
        for (s, w, h, e) in [
            (Scale::Factor(2.0), 100.0, 50.0, 2.0),
            (Scale::Width(200.0), 100.0, 50.0, 2.0),
            (Scale::Height(25.0), 100.0, 50.0, 0.5),
            (Scale::MaxWidth(50.0), 100.0, 50.0, 0.5),
            (Scale::MaxHeight(100.0), 100.0, 50.0, 1.0),
            (Scale::WidthAndHeight(200.0, 50.0), 100.0, 50.0, 1.0),
            (Scale::MaxWidthAndHeight(200.0, 500.0), 100.0, 50.0, 1.0),
        ] {
            assert_eq!(s.factor(w, h), e);
        }
        assert_eq!(DiagramType::Unknown.style_name(), SName::ActivityDiagram);
        assert_eq!(
            DiagramType::find_start_types("@startuml"),
            vec![
                DiagramType::Sequence,
                DiagramType::State,
                DiagramType::Class,
                DiagramType::Object,
                DiagramType::Activity,
                DiagramType::Description,
                DiagramType::Composite,
                DiagramType::Timing,
                DiagramType::Help,
                DiagramType::Sprites
            ]
        );
        assert_eq!(
            DiagramType::find_start_types(" @startPacketdiag x"),
            vec![DiagramType::Packet]
        );
        assert!(DiagramType::find_start_types("x").is_empty());
        assert_eq!(
            DiagramType::find_start_types("@startzzz"),
            vec![DiagramType::Unknown]
        );
    }
}
