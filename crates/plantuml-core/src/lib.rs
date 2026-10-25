//! PlantUML rendering core. Pure library: no I/O, usable on `wasm32-unknown-unknown`.

pub mod command;
pub mod klimt;
pub mod real;
pub mod regex;
pub mod skin;
pub mod stereo;
pub mod style;
pub mod text;

use std::fmt;

/// Errors returned by [`render`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The source does not contain a `@start...` block.
    NoDiagram,
    /// The diagram type is not implemented yet.
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoDiagram => write!(f, "no @start block found"),
            Error::Unsupported(kind) => write!(f, "unsupported diagram type: {kind}"),
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
    let kind = diagram_kind(source).ok_or(Error::NoDiagram)?;
    log::debug!("diagram kind: {kind}");
    Err(Error::Unsupported(kind.to_string()))
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
