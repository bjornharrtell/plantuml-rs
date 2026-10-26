//! Shapes of elements and packages (`USymbol` and `USymbols`).

mod boxes;
mod cloud;
mod common;
pub(crate) mod figures;
mod folder;
mod hline;
pub mod usymbols;

pub use cloud::JavaRandom;
pub use figures::CircleInterface;

use crate::klimt::fashion::Fashion;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::actor::ActorStyle;
use crate::style::SName;
use std::rc::Rc;

/// A shape that can hold a label (`USymbol`).
pub trait USymbol {
    /// Style names added to the signature of the element (`getSNames`).
    fn snames(&self) -> Vec<SName>;

    /// The symbol around a label (`asSmall`).
    fn as_small(
        &self,
        name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock>;

    /// The symbol of a given size, used for packages (`asBig`).
    #[allow(clippy::too_many_arguments)]
    fn as_big(
        &self,
        label: Rc<dyn TextBlock>,
        label_alignment: HorizontalAlignment,
        stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock>;

    fn supp_height_because_of_shape(&self) -> i32 {
        0
    }

    fn supp_width_because_of_shape(&self) -> i32 {
        0
    }
}

/// `skinparam componentStyle` (`ComponentStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentStyle {
    Uml1,
    #[default]
    Uml2,
    Rectangle,
}

impl ComponentStyle {
    pub fn to_usymbol(self) -> Rc<dyn USymbol> {
        match self {
            ComponentStyle::Uml1 => usymbols::component1(),
            ComponentStyle::Uml2 => usymbols::component2(),
            ComponentStyle::Rectangle => usymbols::component_rectangle(),
        }
    }
}

/// `skinparam packageStyle` (`PackageStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageStyle {
    #[default]
    Folder,
    Rectangle,
    Node,
    Frame,
    Cloud,
    Database,
    Agent,
    Storage,
    Component1,
    Component2,
    Artifact,
    Card,
}

impl PackageStyle {
    pub fn from_string(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "folder" => Some(Self::Folder),
            "rectangle" | "rect" => Some(Self::Rectangle),
            "node" => Some(Self::Node),
            "frame" => Some(Self::Frame),
            "cloud" => Some(Self::Cloud),
            "database" => Some(Self::Database),
            "agent" => Some(Self::Agent),
            "storage" => Some(Self::Storage),
            "component1" => Some(Self::Component1),
            "component2" => Some(Self::Component2),
            "artifact" => Some(Self::Artifact),
            "card" => Some(Self::Card),
            _ => None,
        }
    }

    pub fn to_usymbol(self) -> Option<Rc<dyn USymbol>> {
        match self {
            Self::Node => Some(usymbols::node()),
            Self::Card => Some(usymbols::card()),
            Self::Database => Some(usymbols::database()),
            Self::Cloud => Some(usymbols::cloud()),
            Self::Frame => Some(usymbols::frame()),
            Self::Rectangle => Some(usymbols::rectangle()),
            Self::Folder => Some(usymbols::package()),
            _ => None,
        }
    }
}

impl ActorStyle {
    pub fn to_usymbol(self) -> Rc<dyn USymbol> {
        match self {
            ActorStyle::Stickman => usymbols::actor_stickman(),
            ActorStyle::StickmanBusiness => usymbols::actor_stickman_business(),
            ActorStyle::Awesome => usymbols::actor_awesome(),
            ActorStyle::Hollow => usymbols::actor_hollow(),
        }
    }
}

/// The symbol registered under `code`, such as `NODE` or `ACTOR_AWESOME` (`USymbols.all`).
fn by_code(code: &str) -> Option<Rc<dyn USymbol>> {
    use usymbols as u;
    Some(match code {
        "ACTION" => u::action(),
        "ACTOR_AWESOME" => u::actor_awesome(),
        "ACTOR_HOLLOW" => u::actor_hollow(),
        "ACTOR_STICKMAN" => u::actor_stickman(),
        "ACTOR_STICKMAN_BUSINESS" => u::actor_stickman_business(),
        "AGENT" => u::agent(),
        "ARCHIMATE" => u::archimate(),
        "ARTIFACT" => u::artifact(),
        "BOUNDARY" => u::boundary(),
        "CARD" => u::card(),
        "CLOUD" => u::cloud(),
        "COLLECTIONS" => u::collections(),
        "COMPONENT_RECTANGLE" => u::component_rectangle(),
        "COMPONENT1" => u::component1(),
        "COMPONENT2" => u::component2(),
        "CONTROL" => u::control(),
        "DATABASE" => u::database(),
        "ENTITY_DOMAIN" => u::entity_domain(),
        "FILE" => u::file(),
        "FOLDER" => u::folder(),
        "FRAME" => u::frame(),
        "GROUP" => u::group(),
        "HEXAGON" => u::hexagon(),
        "INTERFACE" => u::interface(),
        "LABEL" => u::label(),
        "NODE" => u::node(),
        "PACKAGE" => u::package(),
        "PARTITION" => u::partition(),
        "PERSON" => u::person(),
        "PROCESS" => u::process(),
        "QUEUE" => u::queue(),
        "RECTANGLE" => u::rectangle(),
        "STACK" => u::stack(),
        "STORAGE" => u::storage(),
        "USECASE" => u::usecase(),
        "USECASE_BUSINESS" => u::usecase_business(),
        _ => return None,
    })
}

/// `USymbols.fromString(s, actorStyle, componentStyle, packageStyle)`.
pub fn from_string(
    s: &str,
    actor_style: ActorStyle,
    component_style: ComponentStyle,
    package_style: PackageStyle,
) -> Option<Rc<dyn USymbol>> {
    if s.eq_ignore_ascii_case("package") {
        return package_style.to_usymbol();
    }
    if s.eq_ignore_ascii_case("actor") {
        return Some(actor_style.to_usymbol());
    }
    if s.eq_ignore_ascii_case("component") {
        return Some(component_style.to_usymbol());
    }
    if s.eq_ignore_ascii_case("entity") {
        return Some(usymbols::entity_domain());
    }
    if s.eq_ignore_ascii_case("circle") {
        return Some(usymbols::interface());
    }
    let code: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_uppercase();
    by_code(&code)
}

/// `USymbols.fromString(s, ...)` with the default styles (stickman, UML2, folder).
pub fn from_name(name: &str) -> Option<Rc<dyn USymbol>> {
    from_string(
        name,
        ActorStyle::Stickman,
        ComponentStyle::default(),
        PackageStyle::default(),
    )
}

/// `USymbols.fromString(symbol, skinParam)`: the symbol of a declaration keyword.
pub fn from_keyword(
    symbol: &str,
    actor_style: ActorStyle,
    component_style: ComponentStyle,
) -> Option<Rc<dyn USymbol>> {
    use usymbols as u;
    let keyword = symbol.to_ascii_lowercase();
    Some(match keyword.as_str() {
        "artifact" => u::artifact(),
        "folder" => u::folder(),
        "file" => u::file(),
        "package" => u::package(),
        "rectangle" => u::rectangle(),
        "person" => u::person(),
        "hexagon" => u::hexagon(),
        "label" => u::label(),
        "collections" => u::collections(),
        "node" => u::node(),
        "frame" => u::frame(),
        "cloud" => u::cloud(),
        "action" => u::action(),
        "process" => u::process(),
        "database" => u::database(),
        "queue" => u::queue(),
        "stack" => u::stack(),
        "storage" => u::storage(),
        "agent" => u::agent(),
        "actor/" => u::actor_stickman_business(),
        "actor" => actor_style.to_usymbol(),
        "component" => component_style.to_usymbol(),
        "boundary" => u::boundary(),
        "control" => u::control(),
        "entity" => u::entity_domain(),
        "card" => u::card(),
        "interface" | "()" => u::interface(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
