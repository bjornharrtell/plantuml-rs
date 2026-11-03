//! Activity diagrams of the legacy syntax, `(*) --> a` (`activitydiagram` package). They are laid out
//! by Graphviz like state diagrams; the `:a;` syntax is `activitydiagram3`.

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

pub struct ActivityDiagram {
    pub cuca: CucaDiagram,
    last_entity_consulted: Option<Entity>,
    /// The `if` branches being filled, innermost last (`ConditionalContext`).
    contexts: Vec<Entity>,
}

impl ActivityDiagram {
    pub fn new(source: UmlSource) -> ActivityDiagram {
        let mut cuca = CucaDiagram::new(DiagramBase::new(source, DiagramType::Activity));
        cuca.set_namespace_separator(None);
        ActivityDiagram {
            cuca,
            last_entity_consulted: None,
            contexts: Vec::new(),
        }
    }

    /// `reallyCreateLeaf` and `updateLasts`.
    pub(crate) fn create_leaf(
        &mut self,
        loc: Option<LineLocation>,
        quark: Quark<Entity>,
        display: Display,
        leaf_type: LeafType,
    ) -> Entity {
        let result = self.cuca.really_create_leaf(loc, quark, display, leaf_type);
        if leaf_type != LeafType::Note {
            self.last_entity_consulted = Some(result.clone());
        }
        result
    }

    pub(crate) fn last_entity_consulted(&self) -> Option<Entity> {
        self.last_entity_consulted.clone()
    }

    pub(crate) fn set_last_entity_consulted(&mut self, entity: Option<Entity>) {
        self.last_entity_consulted = entity;
    }

    pub(crate) fn current_branch(&self) -> Option<Entity> {
        self.contexts.last().cloned()
    }

    pub(crate) fn start_if(&mut self, loc: Option<LineLocation>, code: Option<&str>) {
        let id = match code {
            Some(c) => c.to_string(),
            None => self.cuca.unique_sequence("#"),
        };
        let quark = self
            .cuca
            .quark_in_context(
                true,
                &crate::string_utils::eventually_remove_starting_and_ending_double_quote(&id),
            )
            .expect("branch name");
        let branch = self.create_leaf(loc, quark, Display::create(&[""]), LeafType::Branch);
        self.contexts.push(branch);
    }

    pub(crate) fn endif(&mut self) {
        self.contexts.pop();
    }

    pub(crate) fn get_start(&mut self, loc: Option<LineLocation>) -> Entity {
        let quark = self.cuca.quark_in_context(true, "start").expect("start");
        if quark.data().is_none() {
            self.create_leaf(
                loc,
                quark.clone(),
                Display::with_newlines(Some("start")),
                LeafType::CircleStart,
            );
        }
        quark.data().expect("created")
    }

    pub(crate) fn get_end(&mut self, loc: Option<LineLocation>, supp_id: Option<&str>) -> Entity {
        let id = supp_id.map_or("end".to_string(), |s| format!("end${s}"));
        let quark = self.cuca.quark_in_context(true, &id).expect("end");
        if quark.data().is_none() {
            self.create_leaf(
                loc,
                quark.clone(),
                Display::with_newlines(Some("end")),
                LeafType::CircleEnd,
            );
        }
        quark.data().expect("created")
    }

    pub(crate) fn goto_package(&mut self, loc: Option<LineLocation>, name: &str, display: &str) {
        let quark = self.cuca.quark_in_context(true, name).expect("partition");
        self.cuca.goto_group(
            loc,
            quark,
            Display::with_newlines(Some(display)),
            GroupType::Package,
            None,
        );
    }
}

impl CucaHolder for ActivityDiagram {
    fn cuca(&self) -> &CucaDiagram {
        &self.cuca
    }
    fn cuca_mut(&mut self) -> &mut CucaDiagram {
        &mut self.cuca
    }
}

impl HasBase for ActivityDiagram {
    fn base_mut(&mut self) -> &mut DiagramBase {
        &mut self.cuca.base
    }
}

impl CommandDiagram for ActivityDiagram {
    fn required_passes(&self) -> Vec<ParserPass> {
        vec![ParserPass::One]
    }

    fn starting_pass(&mut self, _pass: ParserPass) {
        self.cuca.starting_pass();
    }
}

impl Diagram for ActivityDiagram {
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

pub struct ActivityDiagramFactory;

impl DiagramFactory for ActivityDiagramFactory {
    fn diagram_type(&self) -> DiagramType {
        DiagramType::Activity
    }

    fn create(&self, source: &UmlSource) -> Result<Box<dyn Diagram>, ErrorUml> {
        let diagram = create_system(
            &commands::commands(),
            ActivityDiagram::new(source.clone()),
            source,
        )?;
        Ok(Box::new(diagram))
    }
}
