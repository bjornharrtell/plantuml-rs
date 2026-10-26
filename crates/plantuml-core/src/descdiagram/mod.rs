//! Description diagrams (`descdiagram` package).

pub(crate) mod commands;

use crate::classdiagram::ClassDiagram;
use crate::command::{create_system, ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramFactory, DiagramType};

/// Description and component diagrams (`DescriptionDiagramFactory`).
pub struct DescriptionDiagramFactory;

impl DiagramFactory for DescriptionDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Description
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ClassDiagram::new_description(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
