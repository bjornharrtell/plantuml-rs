//! State common to all diagrams (`TitledDiagram`, `AbstractDiagram`).

use crate::command::UmlSource;
use crate::klimt::creole::display::{Display, DisplayPositioned};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::skin::SkinParam;
use crate::style::SName;
use crate::text::LineLocation;

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
    Gantt,
    Mindmap,
    Wbs,
    Json,
    Yaml,
    Salt,
    Unknown,
}

impl DiagramType {
    /// Value of `data-diagram-type` in the SVG.
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
            DiagramType::Gantt => "GANTT",
            DiagramType::Mindmap => "MINDMAP",
            DiagramType::Wbs => "WBS",
            DiagramType::Json => "JSON",
            DiagramType::Yaml => "YAML",
            DiagramType::Salt => "SALT",
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
            DiagramType::Composite => SName::StateDiagram,
            DiagramType::Timing => SName::TimingDiagram,
            DiagramType::Gantt => SName::GanttDiagram,
            DiagramType::Mindmap => SName::MindmapDiagram,
            DiagramType::Wbs => SName::WbsDiagram,
            DiagramType::Json => SName::JsonDiagram,
            DiagramType::Yaml => SName::YamlDiagram,
            DiagramType::Salt => SName::SaltDiagram,
            DiagramType::Unknown => SName::Root,
        }
    }
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
            skin_param: SkinParam::new(),
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
        }
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
        assert_eq!(DiagramType::Unknown.style_name(), SName::Root);
    }
}
