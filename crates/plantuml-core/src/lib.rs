//! PlantUML rendering core. Pure library: no I/O, usable on `wasm32-unknown-unknown`.

pub mod command;
pub mod diagram;
pub mod export;
pub mod klimt;
pub mod real;
pub mod regex;
pub mod sequencediagram;
pub mod skin;
pub mod stereo;
pub mod style;
pub mod text;
pub mod url;

use std::fmt;

/// Errors returned by [`render`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The source does not contain a `@start...` block.
    NoDiagram,
    /// The diagram type is not implemented yet.
    Unsupported(String),
    /// The source could not be parsed.
    Syntax(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoDiagram => write!(f, "no @start block found"),
            Error::Unsupported(kind) => write!(f, "unsupported diagram type: {kind}"),
            Error::Syntax(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Returns the keyword following `@start` in the first diagram block, e.g. `uml`.
fn diagram_kind(source: &str) -> Option<&str> {
    let line = source
        .lines()
        .find(|l| l.trim_start().starts_with("@start"))?;
    let rest = line.trim_start().strip_prefix("@start")?;
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Renders the first diagram in `source` to SVG.
pub fn render(source: &str) -> Result<String, Error> {
    use crate::command::{create_system, UmlSource};
    use crate::export::{export_svg, ExportOptions};
    use crate::klimt::font::TableFontBounder;
    use crate::klimt::shape::TextBlock;
    use crate::sequencediagram::teoz::playing::SequenceBlock;
    use crate::sequencediagram::{commands, SequenceDiagram};
    use crate::style::Margins;
    use std::rc::Rc;

    let kind = diagram_kind(source).ok_or(Error::NoDiagram)?;
    log::debug!("diagram kind: {kind}");
    if !(kind.is_empty() || kind == "uml") {
        return Err(Error::Unsupported(kind.to_string()));
    }
    let start = source
        .lines()
        .position(|l| l.trim_start().starts_with("@start"))
        .ok_or(Error::NoDiagram)?;
    let text: String = source.lines().skip(start).collect::<Vec<_>>().join("\n");
    let source = UmlSource::from_text(&text);
    let diagram = create_system(
        &commands::commands(),
        SequenceDiagram::from_source(source.clone()),
        &source,
    )
    .map_err(|e| Error::Syntax(e.to_string()))?;
    let sb: Rc<dyn crate::klimt::font::StringBounder> = Rc::new(TableFontBounder);
    let skin = Rc::new(diagram.base.skin_param.clone());
    let block = SequenceBlock::new(&diagram, sb.clone(), skin.clone());
    let mut opts = ExportOptions::new(&skin, diagram.base.diagram_type, Margins::same(5.0));
    opts.back_color = diagram.base.calculate_back_color();
    let _ = (&block as &dyn TextBlock).calculate_dimension(sb.as_ref());
    Ok(export_svg(&block, sb, &opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_diagram() {
        assert_eq!(render("hello"), Err(Error::NoDiagram));
        assert_eq!(Error::NoDiagram.to_string(), "no @start block found");
    }

    #[test]
    fn unsupported_kind() {
        let err = render("  @startuml\nA -> B\n@enduml").unwrap_err();
        assert_eq!(err, Error::Unsupported("uml".into()));
        assert_eq!(err.to_string(), "unsupported diagram type: uml");
    }

    #[test]
    fn kind_stops_at_non_alphanumeric() {
        assert_eq!(diagram_kind("@startmindmap(x)"), Some("mindmap"));
        assert_eq!(diagram_kind("@start"), Some(""));
    }
}
