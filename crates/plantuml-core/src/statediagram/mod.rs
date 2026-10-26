//! State diagrams (`statediagram` package).

mod commands;

use crate::abel::{Entity, GroupType, LeafType};
use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, ErrorUml, ParserPass, UmlSource};
use crate::cucadiagram::{CucaDiagram, CucaHolder};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::creole::display::Display;
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::plasma::Quark;
use crate::skin::SkinParam;
use crate::text::LineLocation;
use std::rc::Rc;

pub struct StateDiagram {
    pub cuca: CucaDiagram,
}

impl StateDiagram {
    pub fn new(source: UmlSource) -> StateDiagram {
        let mut cuca = CucaDiagram::new(DiagramBase::new(source, DiagramType::State));
        cuca.set_namespace_separator(Some("."));
        StateDiagram { cuca }
    }

    /// `checkConcurrentStateOk`.
    pub fn check_concurrent_state_ok(&self, code: &Quark<Entity>) -> bool {
        let Some(existing) = code.data() else {
            return true;
        };
        let current = self.cuca.current_group();
        let parent = existing.parent_container();
        if current.group_type() == Some(GroupType::ConcurrentState)
            && Some(&current) != parent.as_ref()
        {
            return false;
        }
        if let Some(p) = &parent {
            if p.group_type() == Some(GroupType::ConcurrentState) && &current != p {
                return false;
            }
        }
        true
    }

    fn special(&mut self, loc: Option<LineLocation>, prefix: &str, leaf: LeafType) -> Entity {
        let g = self.cuca.current_group();
        let id = if g.is_root() {
            prefix.to_string()
        } else {
            format!("{prefix}{}", g.name())
        };
        let quark = self.cuca.quark_in_context(true, &id).expect("special name");
        if quark.data().is_none() {
            self.cuca.really_create_leaf(
                loc,
                quark.clone(),
                Display::with_newlines(Some("")),
                leaf,
            );
        }
        quark.data().expect("created")
    }

    pub fn get_start(&mut self, loc: Option<LineLocation>) -> Entity {
        self.special(loc, "*start*", LeafType::CircleStart)
    }

    pub fn get_end(&mut self, loc: Option<LineLocation>) -> Entity {
        self.special(loc, "*end*", LeafType::CircleEnd)
    }

    pub fn get_historical(&mut self, loc: Option<LineLocation>) -> Entity {
        self.special(loc, "*historical*", LeafType::PseudoState)
    }

    pub fn get_deep_history(&mut self, loc: Option<LineLocation>) -> Entity {
        self.special(loc, "*deephistory*", LeafType::DeepHistory)
    }

    fn history_of(
        &mut self,
        loc: Option<LineLocation>,
        id_short: &str,
        prefix: &str,
        leaf: LeafType,
    ) -> Entity {
        let quark = self
            .cuca
            .quark_in_context(true, id_short)
            .expect("state name");
        let display = Display::with_newlines(Some(quark.name()));
        self.cuca
            .goto_group(loc, quark, display, GroupType::State, None);
        let g = self.cuca.current_group();
        let ident = self
            .cuca
            .quark_in_context(true, &format!("{prefix}{}", g.name()))
            .expect("history name");
        let result = match ident.data() {
            Some(e) => e,
            None => {
                self.cuca
                    .really_create_leaf(loc, ident, Display::with_newlines(Some("")), leaf)
            }
        };
        self.end_group();
        result
    }

    pub fn get_historical_of(&mut self, loc: Option<LineLocation>, id_short: &str) -> Entity {
        self.history_of(loc, id_short, "*historical*", LeafType::PseudoState)
    }

    pub fn get_deep_history_of(&mut self, loc: Option<LineLocation>, id_short: &str) -> Entity {
        self.history_of(loc, id_short, "*deephistory*", LeafType::DeepHistory)
    }

    /// Starts a new region of the current state (`concurrentState`).
    pub fn concurrent_state(&mut self, loc: Option<LineLocation>, direction: char) -> bool {
        let cur = self.cuca.current_group();
        cur.set_concurrent_separator(direction);
        if cur.group_type() == Some(GroupType::ConcurrentState) {
            self.cuca.end_group();
        }
        let tmp = self.cuca.unique_sequence2("CONC");
        let quark = self.cuca.quark_in_context(true, &tmp).expect("conc name");
        self.cuca.goto_group(
            loc,
            quark,
            Display::create(&[""]),
            GroupType::ConcurrentState,
            None,
        );
        self.cuca
            .current_group()
            .set_concurrent_separator(direction);
        true
    }

    pub fn end_group(&mut self) -> bool {
        if self.cuca.current_group().group_type() == Some(GroupType::ConcurrentState) {
            self.cuca.end_group();
        }
        self.cuca.end_group()
    }

    /// Creates the enclosing states of `a.b.c` (`ensureParentState`).
    pub fn ensure_parent_state(&mut self, loc: Option<LineLocation>, current: &Quark<Entity>) {
        let mut current = current.clone();
        while let Some(parent) = current.parent() {
            if parent.data().is_some() {
                return;
            }
            let group = self
                .cuca
                .create_group(loc, parent.clone(), GroupType::State);
            group.set_display(Display::with_newlines(Some(parent.name())));
            current = parent;
        }
    }

    fn concurrent_parent(ent: &Entity) -> Option<Entity> {
        let mut parent = ent.parent_container();
        while let Some(p) = parent {
            if p.group_type() == Some(GroupType::ConcurrentState) {
                return Some(p);
            }
            parent = p.parent_container();
        }
        None
    }
}

impl CucaHolder for StateDiagram {
    fn cuca(&self) -> &CucaDiagram {
        &self.cuca
    }
    fn cuca_mut(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl HasBase for StateDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.cuca.base
    }
}

impl CommandDiagram for StateDiagram {
    fn required_passes(&self) -> Vec<ParserPass> {
        vec![ParserPass::One, ParserPass::Two, ParserPass::Three]
    }

    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }

    fn check_final_error(&self) -> Option<String> {
        for link in self.cuca.links() {
            let p1 = Self::concurrent_parent(&link.entity1());
            let p2 = Self::concurrent_parent(&link.entity2());
            if p1 != p2 {
                return Some(format!(
                    "State within concurrent state cannot be linked out of this concurrent state (between {} and {})",
                    link.entity1().name(),
                    link.entity2().name()
                ));
            }
        }
        None
    }
}

impl Diagram for StateDiagram {
    fn base(&self) -> &DiagramBase {
        &self.cuca.base
    }

    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.cuca.base
    }

    fn default_margins(&self) -> crate::style::Margins {
        crate::style::Margins::new(0.0, 5.0, 5.0, 0.0)
    }

    fn text_block(&self, sb: &Rc<dyn StringBounder>, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        crate::sdot::build_state_image(&self.cuca, skin, sb)
    }
}

pub struct StateDiagramFactory;

impl DiagramFactory for StateDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::State
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            StateDiagram::new(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> StateDiagram {
        let source = UmlSource::from_text(text);
        create_system(
            &commands::commands(),
            StateDiagram::new(source.clone()),
            &source,
        )
        .unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn states_links_and_pseudo_states() {
        let d = parse("@startuml\n[*] --> A\nstate B : text\nA --> B : go\nB --> [*]\n@enduml");
        assert_eq!(d.cuca.leafs().len(), 4);
        assert_eq!(d.cuca.links().len(), 3);
        let b = d.cuca.first_with_name("B").and_then(|q| q.data()).unwrap();
        assert_eq!(b.get().bodier.raw_body(), ["text"]);
    }

    #[test]
    fn composite_and_concurrent() {
        let d = parse("@startuml\nstate P {\n state A\n --\n state B\n}\n@enduml");
        let p = d.cuca.first_with_name("P").and_then(|q| q.data()).unwrap();
        assert!(p.is_group());
        assert_eq!(p.groups().len(), 1);
    }

    fn render(text: &str) -> String {
        crate::render(text).expect("renders")
    }

    #[test]
    fn renders_composite_states_and_headers() {
        let svg = render(
            "@startuml\nstate Outer {\n state Inner\n}\n[*] --> Outer\nInner --> [*]\n@enduml",
        );
        assert!(svg.contains("data-diagram-type=\"STATE\""));
        assert!(svg.contains("class=\"cluster\""));
        assert!(svg.contains("<path"));
        assert!(svg.contains(">Outer</text>") && svg.contains(">Inner</text>"));
    }

    #[test]
    fn smetana_renders_autarkic_composite_as_one_entity() {
        let svg = render("@startuml\n!pragma layout smetana\nstate P {\n state A\n}\n@enduml");
        assert!(svg.contains("data-qualified-name=\"P\""));
        assert!(svg.contains("class=\"entity\"") && svg.contains(">A</text>"));
        assert!(!svg.contains("class=\"cluster\""));
    }

    #[test]
    fn hiding_a_state_hides_its_links() {
        let svg = render(
            "@startuml\nstate A\nstate B $x\nstate C\nA --> B : go\nB --> C : next\n\
             A --> C\nhide $x\n@enduml",
        );
        assert!(!svg.contains(">B</text>"));
        assert!(!svg.contains(">go</text>") && !svg.contains(">next</text>"));
        assert_eq!(svg.matches("class=\"link\"").count(), 1);
    }

    #[test]
    fn link_between_regions_is_an_error() {
        let source = UmlSource::from_text(
            "@startuml\nstate P {\n state A\n --\n state B\n}\nA --> B\n@enduml",
        );
        assert!(create_system(
            &commands::commands(),
            StateDiagram::new(source.clone()),
            &source
        )
        .is_err());
    }
}
