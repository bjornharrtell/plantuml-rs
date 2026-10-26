//! Chen EER diagrams (`cheneer` package).

pub mod commands;

use crate::abel::Entity;
use crate::command::common::HasBase;
use crate::command::{create_system, CommandDiagram, ErrorUml, UmlSource};
use crate::cucadiagram::{CucaDiagram, CucaHolder};
use crate::diagram::{Diagram, DiagramBase, DiagramFactory, DiagramType};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use std::rc::Rc;

pub struct ChenDiagram {
    pub cuca: CucaDiagram,
    owners: Vec<Entity>,
}

impl ChenDiagram {
    pub fn new(source: UmlSource) -> ChenDiagram {
        let mut cuca = CucaDiagram::new(DiagramBase::new(source, DiagramType::ChenEer));
        cuca.set_namespace_separator(Some("."));
        ChenDiagram {
            cuca,
            owners: Vec::new(),
        }
    }

    /// `CucaDiagram.cleanId`: only the surrounding quotes are removed.
    pub fn clean_id(&self, id: Option<&str>) -> Option<String> {
        id.map(|id| {
            crate::string_utils::eventually_remove_starting_and_ending_double_quote(id).to_string()
        })
    }

    /// Pushes the owner of the following attributes (`pushOwner`).
    pub fn push_owner(&mut self, entity: Entity) {
        self.owners.push(entity);
    }

    /// Pops the owner of the following attributes (`popOwner`).
    pub fn pop_owner(&mut self) -> bool {
        self.owners.pop().is_some()
    }

    /// Owner of the following attributes, not necessarily the current group (`peekOwner`).
    pub fn peek_owner(&self) -> Option<Entity> {
        self.owners.last().cloned()
    }
}

impl CucaHolder for ChenDiagram {
    fn cuca(&self) -> &CucaDiagram {
        &self.cuca
    }

    fn cuca_mut(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl HasBase for ChenDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.cuca.base
    }
}

impl CommandDiagram for ChenDiagram {
    fn starting_pass(&mut self, _pass: crate::command::ParserPass) {
        self.cuca.starting_pass();
    }
}

impl Diagram for ChenDiagram {
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

pub struct ChenEerDiagramFactory;

impl DiagramFactory for ChenEerDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::ChenEer
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ChenDiagram::new(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abel::LeafType;

    fn parse(text: &str) -> ChenDiagram {
        let source = UmlSource::from_text(text);
        create_system(
            &commands::commands(),
            ChenDiagram::new(source.clone()),
            &source,
        )
        .unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn entities_attributes_and_associations() {
        let d = parse(
            "@startchen\nentity PERSON {\n  Name <<key>>\n}\nrelationship LIVES {\n}\nPERSON -1- LIVES\n@endchen",
        );
        assert_eq!(d.cuca.leafs().len(), 3);
        assert_eq!(d.cuca.links().len(), 2);
        let name = d
            .cuca
            .leafs()
            .into_iter()
            .find(|e| e.leaf_type() == Some(LeafType::ChenAttribute))
            .expect("attribute");
        assert_eq!(name.display().text_at(0), "Name");
    }

    #[test]
    fn subclass_discriminator_links() {
        let d = parse("@startchen\nentity P {\n}\nentity C {\n}\nP =>= d { C }\n@endchen");
        let center = d
            .cuca
            .leafs()
            .into_iter()
            .find(|e| e.leaf_type() == Some(LeafType::ChenCircle))
            .expect("discriminator");
        assert_eq!(center.display().text_at(0), "d");
        assert_eq!(d.cuca.links().len(), 2);
    }

    #[test]
    fn renders_chen_shapes() {
        let svg = crate::render(
            "@startchen\nentity \"Director\" as DIRECTOR {\n  \"No.\" as Number <<key>>\n}\nrelationship \"was-rented-to\" as RENTED {\n}\nDIRECTOR -1- RENTED\n@endchen",
        )
        .expect("renders");
        assert!(svg.contains(">Director<"));
        assert!(svg.contains("was-rented-to"));
        assert!(svg.contains("text-decoration=\"underline\""));
    }

    #[test]
    fn attribute_outside_owner_is_an_error() {
        let source = UmlSource::from_text("@startchen\nName\n@endchen");
        assert!(create_system(
            &commands::commands(),
            ChenDiagram::new(source.clone()),
            &source
        )
        .is_err());
    }
}
