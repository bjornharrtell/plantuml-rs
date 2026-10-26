//! Choosing the diagram type of a source (`PSystemBuilder`).

use crate::command::{ErrorUml, UmlSource};
use crate::diagram::{Diagram, DiagramFactory, DiagramType};

/// All diagram factories, in the order they are tried.
pub fn factories() -> Vec<Box<dyn DiagramFactory>> {
    vec![
        Box::new(crate::sequencediagram::SequenceDiagramFactory),
        Box::new(crate::packetdiag::PacketDiagramFactory),
        Box::new(crate::classdiagram::ClassDiagramFactory),
        Box::new(crate::cheneer::ChenEerDiagramFactory),
        Box::new(crate::crash::CrashDiagramFactory),
        Box::new(crate::descdiagram::DescriptionDiagramFactory),
        Box::new(crate::statediagram::StateDiagramFactory),
        Box::new(crate::activitydiagram3::ActivityDiagramFactory3),
        Box::new(crate::chart::ChartDiagramFactory),
        Box::new(crate::salt::SaltDiagramFactory),
        Box::new(crate::jsondiagram::JsonDiagramFactory),
        Box::new(crate::jsondiagram::YamlDiagramFactory),
        Box::new(crate::jsondiagram::HclDiagramFactory),
        Box::new(crate::mindmap::MindMapDiagramFactory),
        Box::new(crate::wbs::WbsDiagramFactory),
        Box::new(crate::gantt::GanttDiagramFactory),
        Box::new(crate::timingdiagram::TimingDiagramFactory),
    ]
}

/// Parses `source` with the first factory accepting it; otherwise the best error.
pub fn create_psystem(source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
    let first = source.lines.first().map(|l| l.string()).unwrap_or_default();
    let types = DiagramType::find_start_types(first);
    let mut errors: Vec<ErrorUml> = Vec::new();
    for f in factories() {
        if !types.contains(&f.diagram_type()) {
            continue;
        }
        match f.create(source) {
            Ok(d) => return Ok(d),
            Err(e) => errors.push(e),
        }
    }
    // Java merges the errors; the one furthest in the source is the most informative.
    errors
        .into_iter()
        .max_by_key(|e| e.score)
        .map(Err)
        .unwrap_or_else(|| {
            Err(ErrorUml {
                kind: crate::command::ErrorUmlType::SyntaxError,
                message: "Unsupported diagram type".to_string(),
                score: 0,
                line: source.lines.first().cloned(),
            })
        })
}
