//! Predefined style signatures (`StyleQueries`).

use super::{SName, StyleQuery};
use std::sync::LazyLock;

pub static ROOT: LazyLock<StyleQuery> = LazyLock::new(|| StyleQuery::of(&[SName::Root]));
pub static DOCUMENT: LazyLock<StyleQuery> = LazyLock::new(|| ROOT.clone().add(SName::Document));
pub static DOCUMENT_CAPTION: LazyLock<StyleQuery> =
    LazyLock::new(|| DOCUMENT.clone().add(SName::Caption));
pub static DOCUMENT_FOOTER: LazyLock<StyleQuery> =
    LazyLock::new(|| DOCUMENT.clone().add(SName::Footer));
pub static DOCUMENT_HEADER: LazyLock<StyleQuery> =
    LazyLock::new(|| DOCUMENT.clone().add(SName::Header));
pub static DOCUMENT_MAINFRAME: LazyLock<StyleQuery> =
    LazyLock::new(|| DOCUMENT.clone().add(SName::Mainframe));
pub static DOCUMENT_TITLE: LazyLock<StyleQuery> =
    LazyLock::new(|| DOCUMENT.clone().add(SName::Title));
pub static ELEMENT: LazyLock<StyleQuery> = LazyLock::new(|| ROOT.clone().add(SName::Element));
pub static ACTIVITYDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::ActivityDiagram));
pub static ACTIVITYDIAG_ACTIVITY: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Activity));
pub static ACTIVITYDIAG_ACTIVITYBAR: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::ActivityBar));
pub static ACTIVITYDIAG_ACTIVITY_ARROW: LazyLock<StyleQuery> = LazyLock::new(|| {
    ACTIVITYDIAG
        .clone()
        .add_all(&[SName::Activity, SName::Arrow])
});
pub static ACTIVITYDIAG_ACTIVITY_DIAMOND: LazyLock<StyleQuery> = LazyLock::new(|| {
    ACTIVITYDIAG
        .clone()
        .add_all(&[SName::Activity, SName::Diamond])
});
pub static ACTIVITYDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Arrow));
pub static ACTIVITYDIAG_CIRCLE: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Circle));
pub static ACTIVITYDIAG_CIRCLE_END: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add_all(&[SName::Circle, SName::End]));
pub static ACTIVITYDIAG_CIRCLE_SPOT: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add_all(&[SName::Circle, SName::Spot]));
pub static ACTIVITYDIAG_CIRCLE_START: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add_all(&[SName::Circle, SName::Start]));
pub static ACTIVITYDIAG_CIRCLE_STOP: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add_all(&[SName::Circle, SName::Stop]));
pub static ACTIVITYDIAG_GOTO: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Goto));
pub static ACTIVITYDIAG_GROUP: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Group));
pub static ACTIVITYDIAG_NOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Note));
pub static ACTIVITYDIAG_SWIMLANE: LazyLock<StyleQuery> =
    LazyLock::new(|| ACTIVITYDIAG.clone().add(SName::Swimlane));
pub static ACTOR_BUSINESS_TITLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Actor, SName::Business, SName::Title])
});
pub static ARROW: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Arrow));
pub static ARROW_CARDINALITY: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Arrow, SName::Cardinality]));
pub static CHARTDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::ChartDiagram));
pub static CHARTDIAG_ANNOTATION: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChartDiagram, SName::Annotation])
});
pub static CHARTDIAG_AREA: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Area));
pub static CHARTDIAG_AXIS: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Axis));
pub static CHARTDIAG_BAR: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Bar));
pub static CHARTDIAG_GRID: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Grid));
pub static CHARTDIAG_LEGEND: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Legend));
pub static CHARTDIAG_LINE: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Line));
pub static CHARTDIAG_SCATTER: LazyLock<StyleQuery> =
    LazyLock::new(|| CHARTDIAG.clone().add(SName::Scatter));
pub static CHENEER_ATTRIBUTE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenAttribute])
});
pub static CHENEER_ATTRIBUTE_TITLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenAttribute, SName::Title])
});
pub static CHENEER_CIRCLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::Circle])
});
pub static CHENEER_CIRCLE_TITLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::Circle, SName::Title])
});
pub static CHENEER_ENTITY: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenEntity])
});
pub static CHENEER_ENTITY_TITLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenEntity, SName::Title])
});
pub static CHENEER_RELATIONSHIP: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenRelationship])
});
pub static CHENEER_RELATIONSHIP_TITLE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ChenEerDiagram, SName::ChenRelationship, SName::Title])
});
pub static CIRCLE: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Circle));
pub static CIRCLE_END: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Circle, SName::End]));
pub static CIRCLE_START: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Circle, SName::Start]));
pub static CLASSDIAG_ARROW: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ClassDiagram, SName::Arrow])
});
pub static CLASSDIAG_CLASS: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ClassDiagram, SName::Class])
});
pub static CLASSDIAG_CLASS_GENERIC: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ClassDiagram, SName::Class, SName::Generic])
});
pub static CLASSDIAG_CLASS_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ClassDiagram, SName::Class, SName::Header])
});
pub static CLASSDIAG_CLASS_QUALIFIED: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ClassDiagram, SName::Class, SName::Qualified])
});
pub static COMPONENT: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::Component));
pub static COMPONENTDIAG_COMPONENT: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ComponentDiagram, SName::Component])
});
pub static COMPONENTDIAG_REQUIREMENT: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ComponentDiagram, SName::Requirement])
});
pub static COMPONENTDIAG_USECASE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ComponentDiagram, SName::Usecase])
});
pub static COMPONENTDIAG_USECASE_BUSINESS: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ComponentDiagram, SName::Usecase, SName::Business])
});
pub static COMPOSITE_TITLE: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Composite, SName::Title]));
pub static DIAMOND: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Diamond));
pub static EBNF: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Ebnf));
pub static FILESDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::FilesDiagram));
pub static FILESDIAG_NOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::FilesDiagram, SName::Note]));
pub static GANTTDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::GanttDiagram));
pub static GANTTDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Arrow));
pub static GANTTDIAG_MILESTONE: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Milestone));
pub static GANTTDIAG_NOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Note));
pub static GANTTDIAG_SEPARATOR: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Separator));
pub static GANTTDIAG_TASK: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Task));
pub static GANTTDIAG_TASK_UNSTARTED: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add_all(&[SName::Task, SName::Unstarted]));
pub static GANTTDIAG_TIMELINE: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Timeline));
pub static GANTTDIAG_UNDONE: LazyLock<StyleQuery> =
    LazyLock::new(|| GANTTDIAG.clone().add(SName::Undone));
pub static GITDIAG: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::GitDiagram));
pub static GROUP: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Group));
pub static MINDMAPDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::MindmapDiagram));
pub static MINDMAPDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| MINDMAPDIAG.clone().add(SName::Arrow));
pub static MINDMAPDIAG_NODE: LazyLock<StyleQuery> =
    LazyLock::new(|| MINDMAPDIAG.clone().add(SName::Node));
pub static MINDMAPDIAG_NODE_BOXLESS: LazyLock<StyleQuery> =
    LazyLock::new(|| MINDMAPDIAG.clone().add_all(&[SName::Node, SName::Boxless]));
pub static MINDMAPDIAG_NODE_LEAF: LazyLock<StyleQuery> =
    LazyLock::new(|| MINDMAPDIAG.clone().add_all(&[SName::Node, SName::LeafNode]));
pub static MINDMAPDIAG_NODE_LEAF_BOXLESS: LazyLock<StyleQuery> = LazyLock::new(|| {
    MINDMAPDIAG
        .clone()
        .add_all(&[SName::Node, SName::LeafNode, SName::Boxless])
});
pub static MINDMAPDIAG_NODE_ROOT: LazyLock<StyleQuery> =
    LazyLock::new(|| MINDMAPDIAG.clone().add_all(&[SName::Node, SName::RootNode]));
pub static MINDMAPDIAG_NODE_ROOT_BOXLESS: LazyLock<StyleQuery> = LazyLock::new(|| {
    MINDMAPDIAG
        .clone()
        .add_all(&[SName::Node, SName::RootNode, SName::Boxless])
});
pub static NODE: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Node));
pub static NODE_HEADER: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Header, SName::Node]));
pub static NODE_HEADER_HIGHLIGHT: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Header, SName::Node, SName::Highlight])
});
pub static NODE_HIGHLIGHT: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Node, SName::Highlight]));
pub static NODE_SEPARATOR: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Node, SName::Separator]));
pub static NOTE: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Note));
pub static NWDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::NwdiagDiagram));
pub static NWDIAG_GROUP: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::NwdiagDiagram, SName::Group])
});
pub static OBJECTDIAG_JSON: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ObjectDiagram, SName::Json])
});
pub static OBJECTDIAG_JSON_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ObjectDiagram, SName::Json, SName::Header])
});
pub static OBJECTDIAG_MAP: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::ObjectDiagram, SName::Map]));
pub static OBJECTDIAG_MAP_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ObjectDiagram, SName::Map, SName::Header])
});
pub static OBJECTDIAG_OBJECT: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ObjectDiagram, SName::Object])
});
pub static OBJECTDIAG_OBJECT_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::ObjectDiagram, SName::Object, SName::Header])
});
pub static PACKAGE_GROUP: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Package, SName::Group]));
pub static PACKAGE_TITLE: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Package, SName::Title]));
pub static PACKETDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::PacketdiagDiagram));
pub static PACKETDIAG_RECTANGLE: LazyLock<StyleQuery> =
    LazyLock::new(|| PACKETDIAG.clone().add(SName::Rectangle));
pub static PORT: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::Port));
pub static SEQUENCEDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::SequenceDiagram));
pub static SEQUENCEDIAG_ACTIVATIONBOX: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::ActivationBox));
pub static SEQUENCEDIAG_ACTOR: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Actor));
pub static SEQUENCEDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Arrow));
pub static SEQUENCEDIAG_BOUNDARY: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Boundary));
pub static SEQUENCEDIAG_BOX: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Box));
pub static SEQUENCEDIAG_COLLECTIONS: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Collections));
pub static SEQUENCEDIAG_CONTROL: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Control));
pub static SEQUENCEDIAG_DATABASE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Database));
pub static SEQUENCEDIAG_ENTITY: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Entity));
pub static SEQUENCEDIAG_GROUP: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Group));
pub static SEQUENCEDIAG_GROUPHEADER: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::GroupHeader));
pub static SEQUENCEDIAG_GROUP_HEADER: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add_all(&[SName::Group, SName::Header]));
pub static SEQUENCEDIAG_LIFELINE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::LifeLine));
pub static SEQUENCEDIAG_LIFELINE_DELAY: LazyLock<StyleQuery> = LazyLock::new(|| {
    SEQUENCEDIAG
        .clone()
        .add_all(&[SName::LifeLine, SName::Delay])
});
pub static SEQUENCEDIAG_LIFELINE_DESTROY: LazyLock<StyleQuery> = LazyLock::new(|| {
    SEQUENCEDIAG
        .clone()
        .add_all(&[SName::LifeLine, SName::Destroy])
});
pub static SEQUENCEDIAG_NEWPAGE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Newpage));
pub static SEQUENCEDIAG_NOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Note));
pub static SEQUENCEDIAG_NOTE_HNOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add_all(&[SName::Note, SName::Hnote]));
pub static SEQUENCEDIAG_NOTE_RNOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add_all(&[SName::Note, SName::Rnote]));
pub static SEQUENCEDIAG_PARTICIPANT: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Participant));
pub static SEQUENCEDIAG_PARTITION: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Partition));
pub static SEQUENCEDIAG_PARTITION_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    SEQUENCEDIAG
        .clone()
        .add_all(&[SName::Partition, SName::Header])
});
pub static SEQUENCEDIAG_QUEUE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Queue));
pub static SEQUENCEDIAG_REFERENCE: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Reference));
pub static SEQUENCEDIAG_REFERENCEHEADER: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::ReferenceHeader));
pub static SEQUENCEDIAG_REFERENCE_HEADER: LazyLock<StyleQuery> = LazyLock::new(|| {
    SEQUENCEDIAG
        .clone()
        .add_all(&[SName::Reference, SName::Header])
});
pub static SEQUENCEDIAG_SEPARATOR: LazyLock<StyleQuery> =
    LazyLock::new(|| SEQUENCEDIAG.clone().add(SName::Separator));
pub static SPOT_ABSTRACT_CLASS: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotAbstractClass])
});
pub static SPOT_ANNOTATION: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotAnnotation])
});
pub static SPOT_CLASS: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotClass]));
pub static SPOT_DATA_CLASS: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotDataClass])
});
pub static SPOT_ENTITY: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotEntity]));
pub static SPOT_ENUM: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotEnum]));
pub static SPOT_EXCEPTION: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotException])
});
pub static SPOT_INTERFACE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotInterface])
});
pub static SPOT_META_CLASS: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotMetaClass])
});
pub static SPOT_PROTOCOL: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotProtocol]));
pub static SPOT_RECORD: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotRecord]));
pub static SPOT_STEREOTYPE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::Spot, SName::SpotStereotype])
});
pub static SPOT_STRUCT: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add_all(&[SName::Spot, SName::SpotStruct]));
pub static STATE: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::State));
pub static STATEDIAG_STATE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::StateDiagram, SName::State])
});
pub static STATEDIAG_STATE_GROUP: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::StateDiagram, SName::State, SName::Group])
});
pub static STATEDIAG_STATE_NAME: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::StateDiagram, SName::State, SName::Name])
});
pub static STEREOTYPE: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::Stereotype));
pub static TIMINGDIAG: LazyLock<StyleQuery> =
    LazyLock::new(|| ELEMENT.clone().add(SName::TimingDiagram));
pub static TIMINGDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::Arrow));
pub static TIMINGDIAG_CONSTRAINTARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::ConstraintArrow));
pub static TIMINGDIAG_HIGHLIGHT: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::Highlight));
pub static TIMINGDIAG_NOTE: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::Note));
pub static TIMINGDIAG_TIMEGRID: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::Timegrid));
pub static TIMINGDIAG_TIMELINE: LazyLock<StyleQuery> =
    LazyLock::new(|| TIMINGDIAG.clone().add(SName::Timeline));
pub static VISIBILITYICON_IE_MANDATORY: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::VisibilityIcon, SName::IEMandatory])
});
pub static VISIBILITYICON_PACKAGE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::VisibilityIcon, SName::Package])
});
pub static VISIBILITYICON_PRIVATE: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::VisibilityIcon, SName::Private])
});
pub static VISIBILITYICON_PROTECTED: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::VisibilityIcon, SName::Protected])
});
pub static VISIBILITYICON_PUBLIC: LazyLock<StyleQuery> = LazyLock::new(|| {
    ELEMENT
        .clone()
        .add_all(&[SName::VisibilityIcon, SName::Public])
});
pub static WBSDIAG: LazyLock<StyleQuery> = LazyLock::new(|| ELEMENT.clone().add(SName::WbsDiagram));
pub static WBSDIAG_ARROW: LazyLock<StyleQuery> =
    LazyLock::new(|| WBSDIAG.clone().add(SName::Arrow));
pub static WBSDIAG_NODE: LazyLock<StyleQuery> = LazyLock::new(|| WBSDIAG.clone().add(SName::Node));
pub static WBSDIAG_NODE_BOXLESS: LazyLock<StyleQuery> =
    LazyLock::new(|| WBSDIAG.clone().add_all(&[SName::Node, SName::Boxless]));
pub static WBSDIAG_NODE_LEAF: LazyLock<StyleQuery> =
    LazyLock::new(|| WBSDIAG.clone().add_all(&[SName::Node, SName::LeafNode]));
pub static WBSDIAG_NODE_LEAF_BOXLESS: LazyLock<StyleQuery> = LazyLock::new(|| {
    WBSDIAG
        .clone()
        .add_all(&[SName::Node, SName::LeafNode, SName::Boxless])
});
pub static WBSDIAG_NODE_ROOT: LazyLock<StyleQuery> =
    LazyLock::new(|| WBSDIAG.clone().add_all(&[SName::Node, SName::RootNode]));
pub static WBSDIAG_NODE_ROOT_BOXLESS: LazyLock<StyleQuery> = LazyLock::new(|| {
    WBSDIAG
        .clone()
        .add_all(&[SName::Node, SName::RootNode, SName::Boxless])
});
