//! The model shared by class, object, state and component diagrams (`abel`).

pub mod bodier;
pub mod entity;
pub mod gender;
pub mod hide;
pub mod link;
pub mod member;

pub use bodier::Bodier;
pub use entity::Entity;
pub use link::{Link, LinkArg, LinkConstraint};
pub use member::Member;

use crate::klimt::color::colors::Colors;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::Position;
use crate::stereo::Stereotype;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupType {
    Root,
    Package,
    State,
    ConcurrentState,
    InnerActivity,
    ConcurrentActivity,
    Domain,
    Requirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeafType {
    EmptyPackage,
    AbstractClass,
    Class,
    Interface,
    Annotation,
    Protocol,
    Struct,
    Exception,
    Metaclass,
    Stereotype,
    LollipopFull,
    LollipopHalf,
    Note,
    Tips,
    Object,
    Map,
    Json,
    Association,
    Enum,
    Circle,
    Dataclass,
    Record,
    Usecase,
    UsecaseBusiness,
    Description,
    ArcCircle,
    Activity,
    Branch,
    SynchroBar,
    CircleStart,
    CircleEnd,
    PointForAssociation,
    ActivityConcurrent,
    State,
    StateConcurrent,
    PseudoState,
    DeepHistory,
    StateChoice,
    StateForkJoin,
    StateTransitionLabel,
    Junction,
    Block,
    Entity,
    Domain,
    Requirement,
    Portin,
    Portout,
    ChenEntity,
    ChenRelationship,
    ChenAttribute,
    ChenCircle,
    StillUnknown,
}

impl LeafType {
    /// `getLeafType`: accepts the keyword as written in the source.
    pub fn from_keyword(type_: &str) -> Option<LeafType> {
        let t = type_.to_uppercase();
        if t.starts_with("ABSTRACT") {
            return Some(LeafType::AbstractClass);
        }
        if t.starts_with("DIAMOND") {
            return Some(LeafType::StateChoice);
        }
        if t.starts_with("STATIC") {
            return Some(LeafType::Class);
        }
        Some(match t.as_str() {
            "CLASS" => LeafType::Class,
            "INTERFACE" => LeafType::Interface,
            "ANNOTATION" => LeafType::Annotation,
            "PROTOCOL" => LeafType::Protocol,
            "STRUCT" => LeafType::Struct,
            "EXCEPTION" => LeafType::Exception,
            "METACLASS" => LeafType::Metaclass,
            "STEREOTYPE" => LeafType::Stereotype,
            "OBJECT" => LeafType::Object,
            "MAP" => LeafType::Map,
            "JSON" => LeafType::Json,
            "ENUM" => LeafType::Enum,
            "CIRCLE" => LeafType::Circle,
            "DATACLASS" => LeafType::Dataclass,
            "RECORD" => LeafType::Record,
            "ENTITY" => LeafType::Entity,
            "USECASE" => LeafType::Usecase,
            "DESCRIPTION" => LeafType::Description,
            "STATE" => LeafType::State,
            "BLOCK" => LeafType::Block,
            "DOMAIN" => LeafType::Domain,
            "REQUIREMENT" => LeafType::Requirement,
            "NOTE" => LeafType::Note,
            "BRANCH" => LeafType::Branch,
            "ACTIVITY" => LeafType::Activity,
            _ => return None,
        })
    }

    pub fn is_like_class(self) -> bool {
        use LeafType::*;
        matches!(
            self,
            Annotation
                | AbstractClass
                | Class
                | Interface
                | Enum
                | Entity
                | Protocol
                | Struct
                | Exception
                | Metaclass
                | Stereotype
                | Dataclass
                | Record
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPortion {
    Field,
    Method,
    Member,
    CircledCharacter,
    Stereotype,
}

impl EntityPortion {
    pub fn as_set(self) -> Vec<EntityPortion> {
        if self == EntityPortion::Member {
            vec![EntityPortion::Field, EntityPortion::Method]
        } else {
            vec![self]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPosition {
    Normal,
    EntryPoint,
    ExitPoint,
    InputPin,
    OutputPin,
    ExpansionInput,
    ExpansionOutput,
    Portin,
    Portout,
}

impl EntityPosition {
    pub const RADIUS: f64 = 6.0;

    pub fn is_normal(self) -> bool {
        self == EntityPosition::Normal
    }

    pub fn from_stereotype(label: &str) -> EntityPosition {
        match label.to_lowercase().as_str() {
            "<<entrypoint>>" => EntityPosition::EntryPoint,
            "<<exitpoint>>" => EntityPosition::ExitPoint,
            "<<inputpin>>" => EntityPosition::InputPin,
            "<<outputpin>>" => EntityPosition::OutputPin,
            "<<expansioninput>>" => EntityPosition::ExpansionInput,
            "<<expansionoutput>>" => EntityPosition::ExpansionOutput,
            _ => EntityPosition::Normal,
        }
    }

    pub fn is_port(self) -> bool {
        matches!(self, EntityPosition::Portin | EntityPosition::Portout)
    }

    pub fn use_port_p(self) -> bool {
        self.is_port() || matches!(self, EntityPosition::ExitPoint | EntityPosition::EntryPoint)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkArrow {
    #[default]
    NoneOrSeveral,
    DirectNormal,
    Backward,
}

impl LinkArrow {
    pub fn reverse(self) -> LinkArrow {
        match self {
            LinkArrow::DirectNormal => LinkArrow::Backward,
            LinkArrow::Backward => LinkArrow::DirectNormal,
            o => o,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteLinkStrategy {
    Normal,
    HalfPrintedFull,
    HalfNotPrinted,
}

/// A note attached to an entity or a link (`CucaNote`).
#[derive(Debug, Clone)]
pub struct CucaNote {
    pub display: Display,
    pub position: Position,
    pub colors: Colors,
    pub strategy: NoteLinkStrategy,
}

impl CucaNote {
    pub fn build(display: Display, position: Position, colors: Colors) -> CucaNote {
        CucaNote {
            display,
            position,
            colors,
            strategy: NoteLinkStrategy::Normal,
        }
    }

    pub fn with_strategy(&self, strategy: NoteLinkStrategy) -> CucaNote {
        CucaNote {
            strategy,
            ..self.clone()
        }
    }
}

/// A tooltip attached to a member (`Tip`).
#[derive(Debug, Clone)]
pub struct Tip {
    pub display: Display,
    pub colors: Colors,
    pub stereotype: Option<Stereotype>,
}

/// Group of entities forced together (`Together`).
#[derive(Debug)]
pub struct Together {
    pub parent: Option<Rc<Together>>,
}

/// A `$tag` on an entity (`Stereotag`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stereotag(pub String);

impl Stereotag {
    pub const SINGLE: &'static str = r"(\$[^%s{}%g<>$]+)";

    pub fn pattern() -> String {
        format!("({}([%s]+{})*)", Self::SINGLE, Self::SINGLE)
    }
}
