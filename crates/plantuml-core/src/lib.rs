//! PlantUML rendering core. Pure library: no I/O, usable on `wasm32-unknown-unknown`.

pub mod abel;
pub mod activitydiagram3;
pub mod chart;
pub mod cheneer;
pub mod chrome;
pub mod classdiagram;
pub mod command;
pub mod crash;
pub mod cucadiagram;
pub mod decoration;
pub mod descdiagram;
pub mod diagram;
pub mod ebnf;
pub mod export;
pub mod gantt;
pub mod hcl;
pub mod json;
pub mod jsondiagram;
pub mod klimt;
pub mod mindmap;
pub mod nwdiag;
pub mod packetdiag;
pub mod plasma;
pub mod preproc;
pub mod psystem;
pub mod real;
pub mod regex;
pub mod salt;
pub mod sdot;
pub mod sequencediagram;
pub mod skin;
pub mod statediagram;
pub mod stereo;
pub mod string_utils;
pub mod style;
pub mod svek;
pub mod text;
pub mod tim;
pub mod timingdiagram;
pub mod url;
pub mod wbs;
pub mod yaml;

/// Version reported by `%version()`.
pub const VERSION: &str = "1.2026.0";

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

/// Renders the first diagram in `source` to SVG; nothing outside the source can be included.
pub fn render(source: &str) -> Result<String, Error> {
    render_with(
        source,
        std::rc::Rc::new(crate::tim::resolver::BundledResolver),
    )
}

/// Renders the first diagram in `source`, resolving includes and themes with `resolver`.
pub fn render_with(
    source: &str,
    resolver: std::rc::Rc<dyn crate::tim::resolver::Resolver>,
) -> Result<String, Error> {
    use crate::command::UmlSource;
    use crate::export::{export_svg, ExportOptions};
    use crate::klimt::font::TableFontBounder;
    use crate::tim::context::OptionKey;
    use crate::tim::loader::{preprocess, Defines};
    use std::rc::Rc;

    let blocks = crate::preproc::split_blocks(source, &[]);
    let block = blocks.into_iter().next().ok_or(Error::NoDiagram)?;
    let first_line = block
        .first()
        .map(|l| l.string().to_string())
        .unwrap_or_default();
    let kind = diagram_kind(&first_line)
        .ok_or(Error::NoDiagram)?
        .to_string();
    log::debug!("diagram kind: {kind}");
    let pre = preprocess(
        block,
        &Defines::new(),
        resolver,
        std::collections::HashMap::new(),
    );
    if let Some(e) = pre.error {
        let line = e.location.location().map_or(0, |l| l.position);
        return Err(Error::Syntax(format!("{} (line {line})", e.message)));
    }
    let source = UmlSource::from_lines(pre.lines);
    let types = crate::diagram::DiagramType::find_start_types(&first_line);
    if !crate::psystem::factories()
        .iter()
        .any(|f| types.contains(&f.diagram_type()))
    {
        return Err(Error::Unsupported(kind));
    }
    let mut diagram =
        crate::psystem::create_psystem(&source).map_err(|e| Error::Syntax(e.to_string()))?;
    for w in &pre.artifact.warnings {
        diagram.base_mut().add_warning(w.clone());
    }
    let sb: Rc<dyn crate::klimt::font::StringBounder> = Rc::new(TableFontBounder);
    let skin = Rc::new(diagram.base().skin_param.clone());
    let block = diagram.text_block(&sb, &skin);
    let base = diagram.base();
    let margin = {
        let style = skin
            .style_builder()
            .merged_style(&crate::style::queries::DOCUMENT);
        match style {
            Some(s) if s.has_value(crate::style::PName::Margin) => s.margin(),
            _ => diagram.default_margins(),
        }
    };
    let mut opts = ExportOptions::new(&skin, base.diagram_type, margin);
    opts.back_color = base.calculate_back_color();
    opts.scale = base.scale;
    opts.svg_title = pre.artifact.options.get(&OptionKey::SvgTitle).cloned();
    opts.svg_desc = pre.artifact.options.get(&OptionKey::SvgDesc).cloned();
    let block = crate::chrome::create(block, base, &skin, &diagram.title());
    Ok(export_svg(block.as_ref(), sb, &opts))
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
        let err = render("@startbpm\nAlice -> Bob\n@endbpm").unwrap_err();
        assert_eq!(err, Error::Unsupported("bpm".into()));
        assert_eq!(err.to_string(), "unsupported diagram type: bpm");
    }

    #[test]
    fn renders_a_sequence_diagram() {
        let svg = render("  @startuml\nA -> B: hi\n@enduml").unwrap();
        assert!(svg.contains("data-diagram-type=\"SEQUENCE\""));
        assert!(svg.contains(">hi</text>"));
    }

    #[test]
    fn renders_self_messages() {
        let svg = render("@startuml\nA -> A: one\nactivate A\nA <- A: two\n@enduml").unwrap();
        assert!(svg.contains(">one</text>") && svg.contains(">two</text>"));
    }

    #[test]
    fn renders_groups() {
        let svg = render(
            "@startuml\nalt#Gold #LightBlue ok\nA -> B\nelse #Pink bad\nB -> A\nelse\nloop 2\nA -> A\nend\nend\n\
             group My [label]\nA -> B\nend\ngroup\nA -> B\nend\npar2\nA -> B\nend\npartition P1\nA -> B\nend\n@enduml",
        )
        .unwrap();
        assert!(svg.contains(">alt</text>") && svg.contains(">[ok]</text>"));
        assert!(svg.contains(">[bad]</text>") && svg.contains(">P1</text>"));
        assert!(svg.contains(">My</text>") && svg.contains(">[label]</text>"));
    }

    #[test]
    fn renders_participant_figures() {
        for style in [
            "",
            "skinparam actorStyle awesome",
            "skinparam actorStyle hollow",
        ] {
            let svg = render(&format!(
                "@startuml\n{style}\nactor A\nboundary B\ncontrol C\nentity D\ndatabase E\nqueue F\ncollections G\nA -> B\nB -> C\nC -> D\nD -> E\nE -> F\nF -> G\n@enduml"
            ))
            .unwrap();
            assert!(
                svg.contains("<ellipse") && svg.contains(">E</text>"),
                "{style}"
            );
        }
    }

    #[test]
    fn renders_boxes_and_references() {
        let svg = render(
            "@startuml\nbox \"Outer\" <<s>> #LightBlue\nparticipant a\nbox Inner\nparticipant b\nend box\nend box\n\
             participant c\nref over a, c : one\nref#pink over b\nmulti\nline\nend ref\nnote right: rn\n\
             ref over c [[http://x.org]] : linked\nnote left: ln\n@enduml",
        )
        .unwrap();
        for text in ["Outer", "Inner", "ref", "one", "multi", "rn", "ln"] {
            assert!(svg.contains(&format!(">{text}</text>")), "{text}");
        }
        assert!(matches!(
            render("@startuml\nend box\n@enduml"),
            Err(Error::Syntax(_))
        ));
    }

    #[test]
    fn renders_border_messages() {
        let svg = render(
            "@startuml\nparticipant A\nparticipant B\n[-> A : l1\nA ->] : r1\n[<- B : l2\nB <-] : r2\n\
             ?-> B : short\nB ->? : short2\n[->o B\n[x-> B\nA ->x]\nA ->o]\nactivate A\n[-> A ++\n[<- A --\n\
             [--> A : dotted\n[->> A : async\nnote right: n\n@enduml",
        )
        .unwrap();
        for text in [
            "l1", "r1", "l2", "r2", "short", "short2", "dotted", "async", "n",
        ] {
            assert!(svg.contains(&format!(">{text}</text>")), "{text}");
        }
    }

    #[test]
    fn renders_chrome() {
        let svg = render(
            "@startuml\ntitle My title\nheader hd\nfooter ft\ncaption cp\nlegend\nleg\nendlegend\n\
             mainframe frame\nA -> B\nnote over A: n\n/ note over B: m\n@enduml",
        )
        .unwrap();
        for text in ["My title", "hd", "ft", "cp", "leg", "frame", "n", "m"] {
            assert!(svg.contains(&format!(">{text}</text>")), "{text}");
        }
        assert!(svg.contains("class=\"title\"") && svg.contains("class=\"legend\""));
        assert!(svg.contains("deprecated"), "warning banner");
    }

    #[test]
    fn renders_packet_diagrams() {
        let svg = render(
            "@startpacketdiag\ncolwidth = 16\nscale_direction = rtl\nsame_height = true\n0-7: Source Port\n\
             8-15: Destination Port [height=2]\n16-47: Long field that wraps [rotate=90]\n* next [len=4]\n@endpacketdiag",
        )
        .unwrap();
        assert!(svg.contains("data-diagram-type=\"PACKET\""));
        assert!(svg.contains(">Source Port</text>"));
        assert!(matches!(
            render("@startpacketdiag\ncolwidth = x\n@endpacketdiag"),
            Err(Error::Syntax(_))
        ));
    }

    #[test]
    fn empty_packages_and_boundaries_are_drawn() {
        let svg = render("@startuml\npackage foo {\n}\n@enduml").unwrap();
        assert!(svg.contains("class=\"cluster\""));
        assert!(svg.contains(">foo</text>"));
        let svg =
            render("@startuml\nrectangle \"== Domain 2 ==\" <<boundary>> as domain2 {\n}\n@enduml")
                .unwrap();
        assert!(svg.contains(">Domain 2</text>"));
        let svg = render(
            "@startuml\n!include <C4/C4_Component>\nContainer(api, \"API\", \"Rust\", \"Serves\")\n\
             System_Boundary(domain2, \"Domain 2\") {\n}\nRel(api, domain2, \"calls\")\n@enduml",
        )
        .unwrap();
        assert!(svg.contains(">Domain 2</text>"));
        assert!(svg.contains(">calls</text>"));
    }

    #[test]
    fn empty_packet_diagrams_do_not_panic() {
        // Java divides by zero on these two and shows an error diagram; the port draws them.
        assert!(render("@startpacketdiag\n4-3: error\n@endpacketdiag").is_ok());
        assert!(render("@startpacketdiag\n* A [len = 0]\n@endpacketdiag").is_ok());
    }

    #[test]
    fn renders_a_class_diagram() {
        let svg = render(
            "@startuml\nclass A {\n  + int x\n  - run()\n}\nclass B\nA <|-- B : extends\n@enduml",
        )
        .unwrap();
        assert!(svg.contains("data-diagram-type=\"CLASS\""));
        assert!(svg.contains("<!--class A-->") && svg.contains("<!--class B-->"));
        assert!(svg.contains(">A</text>") && svg.contains(">B</text>"));
        assert!(svg.contains(">int x</text>") && svg.contains(">run()</text>"));
        assert!(svg.contains("data-visibility-modifier=\"PUBLIC_FIELD\""));
        assert!(svg.contains("data-visibility-modifier=\"PRIVATE_METHOD\""));
        assert!(svg.contains("data-link-type=\"extension\""));
        assert!(svg.contains(">extends</text>"));
        assert!(svg.contains("<polygon"), "arrow end of the extension link");
    }

    #[test]
    fn renders_c4_models() {
        let svg = render(
            "@startuml\n!include <C4/C4_Container>\nPerson(user, \"User\")\nSystem(sys, \"System\")\nRel(user, sys, \"Uses\")\n@enduml",
        )
        .unwrap();
        assert!(svg.contains("data-diagram-type=\"DESCRIPTION\""));
        for text in ["User", "System", "Uses"] {
            assert!(svg.contains(&format!(">{text}</text>")), "{text}");
        }
    }

    #[test]
    fn sprite_definitions_parse_without_being_drawn() {
        let svg =
            render("@startuml\nsprite $p [4x4/16] {\n0123\n3210\n}\nclass A\n@enduml").unwrap();
        assert!(svg.contains(">A</text>"));
        assert!(matches!(
            render("@startuml\nsprite $p [4x4/16] {\n}\nclass A\n@enduml"),
            Err(Error::Syntax(_))
        ));
    }

    #[test]
    fn renders_notes() {
        let svg = render(
            "@startuml\nA -> B\nnote left of A: l\nnote right of B #red: r\nnote over A: o\n\
             hnote over B: h\nrnote over A, B: several\nnote across: all\nB -> B\nnote right: sr\nnote left: sl\n\
             A -> B\nnote top: t\nnote bottom\nb1\n--x--\nb2\nend note\n\
             alt\nA -> B\nnote left: gl\nend\nnote right: after\n& note over B: p\n@enduml",
        )
        .unwrap();
        for text in [
            "l", "r", "o", "h", "several", "all", "sr", "sl", "t", "b1", "b2", "after",
        ] {
            assert!(svg.contains(&format!(">{text}</text>")), "{text}");
        }
    }

    #[test]
    fn group_without_start_is_an_error() {
        assert!(matches!(
            render("@startuml\nA -> B\nend\n@enduml"),
            Err(Error::Syntax(_))
        ));
        assert!(matches!(
            render("@startuml\nalt #nosuchcolor\nA -> B\nend\n@enduml"),
            Err(Error::Syntax(_))
        ));
    }

    #[test]
    fn syntax_errors_are_reported() {
        let err = render("@startuml\nA -> B: hi\n???\n@enduml").unwrap_err();
        assert!(matches!(err, Error::Syntax(_)));
        assert!(err.to_string().contains("Syntax"));
    }

    #[test]
    fn kind_stops_at_non_alphanumeric() {
        assert_eq!(diagram_kind("@startmindmap(x)"), Some("mindmap"));
        assert_eq!(diagram_kind("@start"), Some(""));
    }

    #[test]
    fn bundled_theme_sets_the_background() {
        let svg = render("@startuml\n!theme amiga\nA -> B\n@enduml").expect("renders");
        assert!(svg.contains("#0B58A8"));
        assert!(render("@startuml\n!theme nonexistent\nA -> B\n@enduml").is_err());
    }
}
