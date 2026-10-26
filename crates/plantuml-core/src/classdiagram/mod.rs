//! Class diagrams (`classdiagram` package).

mod association;
pub(crate) mod commands;
mod commands_group;
pub(crate) mod commands_misc;
pub(crate) mod commands_note;

use crate::abel::{Entity, Link};
use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, CommandExecutionResult, ErrorUml, UmlSource};
use crate::cucadiagram::CucaDiagram;
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::rc::Rc;

pub struct ClassDiagram {
    pub cuca: CucaDiagram,
    allow_mixing: bool,
    use_layout_explicit: i32,
    associations: Vec<association::Association>,
}

impl ClassDiagram {
    pub fn new(source: UmlSource) -> ClassDiagram {
        let mut cuca = CucaDiagram::new(DiagramBase::new(source, DiagramType::Class));
        cuca.set_namespace_separator(Some("."));
        ClassDiagram {
            cuca,
            allow_mixing: false,
            use_layout_explicit: 0,
            associations: Vec::new(),
        }
    }

    pub fn set_allow_mixing(&mut self, v: bool) {
        self.allow_mixing = v;
    }

    pub fn is_allow_mixing(&self) -> bool {
        self.allow_mixing
    }

    pub fn layout_new_line(&mut self) {
        self.use_layout_explicit += 1;
        self.cuca.inc_raw_layout();
    }

    /// `checkIfPackageHierarchyIsOk`.
    pub fn check_if_package_hierarchy_is_ok(&self, entity: &Entity) -> CommandExecutionResult {
        let mut current = entity.quark().parent();
        while let Some(c) = current {
            if c.is_root() {
                break;
            }
            if c.data().is_some_and(|d| !d.is_group()) {
                return CommandExecutionResult::error(format!(
                    "Bad hierarchy for class {}",
                    entity.qualified_name()
                ));
            }
            current = c.parent();
        }
        CommandExecutionResult::ok()
    }

    /// Joins packages holding only one package (`packSomePackage`).
    fn pack_some_package(&mut self) {
        let separator = self.cuca.namespace_separator().unwrap_or(".").to_string();
        loop {
            let mut changed = false;
            for group in self.cuca.groups() {
                if self.can_be_packed(&group) {
                    let child = group.groups().into_iter().next().expect("one child");
                    let appended = format!("{}{}", group.display().text_at(0), separator);
                    let new_display = child.display().append_first_line(&appended);
                    child.set_display(new_display);
                    group.set_packed();
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
    }

    fn can_be_packed(&self, g: &Entity) -> bool {
        if g.is_packed() || g.count_children() != 1 || !g.leafs().is_empty() {
            return false;
        }
        if self.cuca.links().iter().any(|l| l.contains(g)) {
            return false;
        }
        let child = g.groups().into_iter().next().expect("group child");
        child.count_children() != 0
    }

    /// The shape selected by a stereotype such as `<<Node>>` (`USymbols.fromString`).
    pub fn usymbol_for(
        &self,
        stereotype: &str,
    ) -> Option<std::rc::Rc<dyn crate::decoration::usymbol::USymbol>> {
        let skin = &self.cuca.base.skin_param;
        crate::decoration::usymbol::from_string(
            stereotype,
            skin.actor_style(),
            skin.component_style(),
            skin.package_style(),
        )
    }

    pub fn links(&self) -> Vec<Link> {
        self.cuca.links().to_vec()
    }
}

impl crate::cucadiagram::CucaHolder for ClassDiagram {
    fn cuca(&self) -> &CucaDiagram {
        &self.cuca
    }
    fn cuca_mut(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl HasBase for ClassDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.cuca.base
    }
}

impl CommandDiagram for ClassDiagram {
    fn starting_pass(&mut self, _pass: crate::command::ParserPass) {
        self.cuca.starting_pass();
    }

    fn check_final_error(&self) -> Option<String> {
        None
    }

    fn make_diagram_ready(&mut self) {
        let links = self.cuca.links().to_vec();
        for link in &links {
            if link.length() == 1 {
                for link2 in &links {
                    if link2.same_connections(link) && link2.length() != 1 {
                        link2.set_length(1);
                    }
                }
            }
        }
        if self
            .cuca
            .base
            .pragma
            .get("useintermediatepackages")
            .map(String::as_str)
            == Some("false")
        {
            self.pack_some_package();
        }
    }
}

impl Diagram for ClassDiagram {
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
        crate::sdot::build_image(&self.cuca, skin, sb)
    }
}

pub struct ClassDiagramFactory;

impl DiagramFactory for ClassDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Class
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ClassDiagram::new(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ClassDiagram {
        let source = UmlSource::from_text(text);
        create_system(
            &commands::commands(),
            ClassDiagram::new(source.clone()),
            &source,
        )
        .unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn classes_and_links() {
        let d = parse(
            "@startuml\nclass A\nclass B {\n-x : int\n+run()\n}\nA /\"-tailOnly\" --> B\nA <|-- C : label\n@enduml",
        );
        assert_eq!(d.cuca.leafs().len(), 3);
        let links = d.links();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].role1().as_deref(), Some("-tailOnly"));
        assert_eq!(
            links[0].type_().decor1(),
            crate::decoration::link::LinkDecor::Arrow
        );
        let b = d.cuca.first_with_name("B").and_then(|q| q.data()).unwrap();
        assert_eq!(b.get().bodier.raw_body().len(), 2);
    }

    #[test]
    fn notes_are_drawn_with_a_pointer() {
        let svg = crate::render(
            "@startuml\nclass A\nclass B\nA --> B\nnote right of A : hello\nnote bottom of B #red\n x\n y\nend note\nnote \"free\" as F\nnote on link : on the link\n@enduml",
        )
        .expect("renders");
        assert!(svg.contains("hello"));
        assert!(svg.contains("on the link"));
        assert!(svg.contains("free"));
        assert!(svg.contains("fill=\"#F00\""));
        // folded corner and pointer of the note on A
        assert!(svg.matches("<path").count() >= 4);
        assert_eq!(svg.matches("class=\"entity\"").count(), 5);
    }

    #[test]
    fn tips_point_at_members() {
        let svg = crate::render(
            "@startuml\nclass E {\n int field1\n doThing()\n}\nnote right of E::field1 #yellow\n tip one\nend note\nnote left of E::doThing\n tip two\nend note\n@enduml",
        )
        .expect("renders");
        assert!(svg.contains("tip one"));
        assert!(svg.contains("tip two"));
        assert!(svg.contains("fill=\"#FF0\""));
    }

    #[test]
    fn note_linked_twice_is_drawn_normally() {
        let svg = crate::render(
            "@startuml\nclass A\nclass B\nnote as N\n  n\nend note\nN .. A\nN .. B\n@enduml",
        )
        .expect("renders");
        assert!(svg.contains(">n<"));
        assert!(svg.contains("class=\"link\""));
    }

    #[test]
    fn extends_and_package_names() {
        let d = parse("@startuml\nclass a.B extends C\n@enduml");
        assert_eq!(d.cuca.leafs().len(), 2);
        assert_eq!(d.links().len(), 1);
    }
}
