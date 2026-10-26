//! The crash test diagram (`crash` package): a fixed explanatory text.

use crate::command::{ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::rc::Rc;

/// `CrashDiagram`: the diagram type is BPM on purpose, like the Java class.
pub struct CrashDiagram {
    base: DiagramBase,
}

impl CrashDiagram {
    pub fn new(source: UmlSource) -> CrashDiagram {
        CrashDiagram {
            base: DiagramBase::new(source, DiagramType::Bpm),
        }
    }
}

impl Diagram for CrashDiagram {
    fn base(&self) -> &DiagramBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.base
    }

    fn text_block(&self, _sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        let display = Display::create(&[
            "<b>Crash Diagram</b>",
            "",
            "This diagram is only used to test crash handling.",
            "",
            "There are three types of crash:",
            "  \u{2022} During <b>parsing</b>",
            "  \u{2022} During <b>finalizing</b>",
            "  \u{2022} During <b>drawing</b>",
            "",
            "The purpose is to verify that PlantUML error recovery",
            "mechanisms work correctly, using a diagram whose",
            "behavior is predictable <i>(it crashes)</i>.",
            "",
            "To do so, you should the following directive:",
            "  \u{2022} \"\"crash when parsing\"\"",
            "  \u{2022} \"\"crash when finalizing\"\"",
            "  \u{2022} \"\"crash when drawing\"\"",
            "",
        ]);
        let fc = FontConfiguration::black_blue_true(UFont::sans_serif(14));
        display.create_block(
            &fc,
            HorizontalAlignment::Left,
            &skin.skin_simple(),
            &crate::klimt::creole::LineBreakStrategy::NONE,
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        )
    }
}

pub struct CrashDiagramFactory;

impl DiagramFactory for CrashDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Crash
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        Ok(Box::new(CrashDiagram::new(source.clone())))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crash_text_renders() {
        let svg = crate::render("@startcrash\nThis is working\n@endcrash").expect("renders");
        assert!(svg.contains(r#"data-diagram-type="BPM""#));
        assert!(svg.contains(">Crash Diagram</text>"));
        assert!(svg.contains("This diagram is only used to test crash handling."));
    }
}
