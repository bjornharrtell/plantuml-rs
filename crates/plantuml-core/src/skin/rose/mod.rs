//! Default look of sequence diagram components (`skin/rose`).

pub mod arrow;
pub mod figure;
pub mod grouping;
pub mod line;
pub mod misc;
pub mod note;
pub mod participant;
pub mod self_arrow;
pub mod text;

use crate::klimt::color::colors::Colors;
use crate::klimt::creole::display::Display;
use crate::klimt::geom::HorizontalAlignment;
use crate::sequencediagram::model::{NotePosition, Participant};
use crate::skin::arrow::{ArrowConfiguration, ArrowDirection};
use crate::skin::component::{ArrowComponent, Component, ComponentType};
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::rc::Rc;

use crate::klimt::fashion::Fashion;
use crate::klimt::shape::EmptyBlock;
use crate::svek::{Boundary, Control, EntityDomain};
use arrow::ComponentRoseArrow;
use figure::{ComponentRoseFigure, ComponentRoseQueue};
use grouping::{
    ComponentRoseGroupingElse, ComponentRoseGroupingHeader, ComponentRoseGroupingSpace,
};
use line::ComponentRoseLine;
use misc::{ComponentRoseActiveLine, ComponentRoseDelayLine, ComponentRoseDestroy};
use note::{ComponentRoseNote, ComponentRoseNoteBox, ComponentRoseNoteHexagonal};
use participant::ComponentRoseParticipant;
use self_arrow::ComponentRoseSelfArrow;
use text::{ComponentRoseDelayText, ComponentRoseDivider, ComponentRoseNewpage};

/// Component factory of the default skin (`Rose`).
pub struct Rose;

impl Rose {
    /// Head or tail box of a participant (`createComponentParticipant`).
    pub fn create_component_participant(
        p: &Participant,
        ctype: ComponentType,
        skin: &Rc<SkinParam>,
        display: &Display,
    ) -> Rc<dyn Component> {
        let styles = p.used_styles();
        let padding = styles[0].padding();
        let margin = styles[0].margin();
        let min_width = styles[0].value(PName::MinimumWidth).as_double();
        let collections = matches!(
            ctype,
            ComponentType::CollectionsHead | ComponentType::CollectionsTail
        );
        let figure =
            |style: &Style| ComponentRoseFigure::core(style, &styles[1], display, skin.clone());
        let head = matches!(
            ctype,
            ComponentType::ActorHead
                | ComponentType::BoundaryHead
                | ComponentType::ControlHead
                | ComponentType::EntityHead
                | ComponentType::DatabaseHead
        );
        match ctype {
            ComponentType::ActorHead | ComponentType::ActorTail => {
                let fashion = styles[0].symbol_context(None);
                let core = figure(&styles[0]);
                return Rc::new(ComponentRoseFigure::new(
                    core,
                    skin.actor_style().text_block(fashion),
                    head,
                ));
            }
            ComponentType::BoundaryHead | ComponentType::BoundaryTail => {
                let fashion = styles[0].symbol_context(None);
                return Rc::new(ComponentRoseFigure::new(
                    figure(&styles[0]),
                    Rc::new(Boundary {
                        symbol_context: fashion,
                    }),
                    head,
                ));
            }
            ComponentType::ControlHead | ComponentType::ControlTail => {
                let fashion = styles[0].symbol_context(None);
                return Rc::new(ComponentRoseFigure::new(
                    figure(&styles[0]),
                    Rc::new(Control {
                        symbol_context: fashion,
                    }),
                    head,
                ));
            }
            ComponentType::EntityHead | ComponentType::EntityTail => {
                let fashion = styles[0].symbol_context(None);
                return Rc::new(ComponentRoseFigure::new(
                    figure(&styles[0]),
                    Rc::new(EntityDomain {
                        symbol_context: fashion,
                    }),
                    head,
                ));
            }
            ComponentType::DatabaseHead | ComponentType::DatabaseTail => {
                let bi_color = styles[0].symbol_context(None);
                let fashion = Fashion::new(
                    bi_color.back_color().cloned(),
                    bi_color.fore_color().cloned(),
                )
                .with_stroke(styles[0].stroke())
                .with_delta_shadow(bi_color.delta_shadow());
                let symbol = crate::decoration::symbol::database_small(
                    Rc::new(EmptyBlock {
                        width: 16.0,
                        height: 17.0,
                    }),
                    Rc::new(EmptyBlock {
                        width: 0.0,
                        height: 0.0,
                    }),
                    fashion,
                );
                return Rc::new(ComponentRoseFigure::new(figure(&styles[0]), symbol, head));
            }
            ComponentType::QueueHead | ComponentType::QueueTail => {
                let fashion = styles[0].symbol_context(None);
                return Rc::new(ComponentRoseQueue::new(&figure(&styles[0]), fashion));
            }
            _ => {}
        }
        Rc::new(ComponentRoseParticipant::new(
            &styles[0],
            &styles[1],
            display,
            skin.clone(),
            min_width,
            collections,
            padding,
            margin,
        ))
    }

    /// Straight message arrow (`createComponentArrow`).
    pub fn create_component_arrow(
        style: &Style,
        config: &ArrowConfiguration,
        skin: &Rc<SkinParam>,
        display: &Display,
    ) -> Rc<dyn ArrowComponent> {
        let style_width = style.wrap_width();
        let max_message_size = if style_width.max_width() > 0.0 {
            style_width
        } else {
            skin.max_message_size()
        };
        let direction = config.arrow_direction();
        if direction == ArrowDirection::SelfArrow {
            return Rc::new(ComponentRoseSelfArrow::new(
                style,
                display,
                config,
                skin.clone(),
                &max_message_size,
                !skin.strict_uml_style(),
            ));
        }
        let text_style = skin
            .style_builder()
            .merged_style(&queries::SEQUENCEDIAG_ARROW)
            .expect("arrow style");
        let value = text_style.value(PName::HorizontalAlignment).as_string();
        let mut alignment = text_style.horizontal_alignment();
        let reverse = config.is_reverse_define();
        if value.eq_ignore_ascii_case("first") {
            alignment = if direction == ArrowDirection::RightToLeftReverse {
                if reverse {
                    HorizontalAlignment::Left
                } else {
                    HorizontalAlignment::Right
                }
            } else if reverse {
                HorizontalAlignment::Right
            } else {
                HorizontalAlignment::Left
            };
        } else if value.eq_ignore_ascii_case("direction") {
            match direction {
                ArrowDirection::LeftToRightNormal => alignment = HorizontalAlignment::Left,
                ArrowDirection::RightToLeftReverse => alignment = HorizontalAlignment::Right,
                ArrowDirection::BothDirection => alignment = HorizontalAlignment::Center,
                ArrowDirection::SelfArrow => {}
            }
        } else if value.eq_ignore_ascii_case("reversedirection") {
            match direction {
                ArrowDirection::LeftToRightNormal => alignment = HorizontalAlignment::Right,
                ArrowDirection::RightToLeftReverse => alignment = HorizontalAlignment::Left,
                ArrowDirection::BothDirection => alignment = HorizontalAlignment::Center,
                ArrowDirection::SelfArrow => {}
            }
        }
        Rc::new(ComponentRoseArrow::new(
            style,
            display,
            config,
            alignment,
            skin.clone(),
            &max_message_size,
            !skin.strict_uml_style(),
            skin.is_true("responsemessagebelowarrow"),
        ))
    }

    /// Note shaped by `ctype` (`createComponentNote`).
    pub fn create_component_note(
        style: &Style,
        ctype: ComponentType,
        skin: &Rc<SkinParam>,
        display: &Display,
        colors: &Colors,
        position: Option<NotePosition>,
    ) -> Rc<dyn Component> {
        let (text_align, alignment) = if position == Some(NotePosition::OverSeveral) {
            let text_align = skin.note_text_alignment(Some(HorizontalAlignment::Left));
            let default = skin.note_text_alignment(Some(HorizontalAlignment::Center));
            let alignment = if text_align == default {
                text_align
            } else {
                HorizontalAlignment::Center
            };
            (text_align, alignment)
        } else {
            let text_align = skin.note_text_alignment(None);
            (text_align, text_align)
        };
        match ctype {
            ComponentType::Note => Rc::new(ComponentRoseNote::new(
                style,
                display,
                5.0,
                5.0,
                skin.clone(),
                text_align,
                alignment,
                colors,
            )),
            ComponentType::NoteHexagonal => Rc::new(ComponentRoseNoteHexagonal::new(
                style,
                display,
                skin.clone(),
                colors,
            )),
            ComponentType::NoteBox => Rc::new(ComponentRoseNoteBox::new(
                style,
                display,
                skin.clone(),
                colors,
            )),
            other => unimplemented!("note component {other:?}"),
        }
    }

    /// Header of a group (`GROUPING_HEADER_*`).
    pub fn create_grouping_header(
        teoz: bool,
        styles: &[Option<Style>; 2],
        title: &str,
        comment: Option<&str>,
        skin: &Rc<SkinParam>,
    ) -> Rc<dyn Component> {
        Rc::new(ComponentRoseGroupingHeader::new(
            teoz,
            styles[0].as_ref().expect("group style"),
            styles[1].as_ref().expect("group header style"),
            title,
            comment,
            skin.clone(),
        ))
    }

    /// Separator of a group (`GROUPING_ELSE_*`).
    pub fn create_grouping_else(
        teoz: bool,
        style: &Style,
        comment: Option<&str>,
        skin: &Rc<SkinParam>,
    ) -> Rc<dyn Component> {
        Rc::new(ComponentRoseGroupingElse::new(
            teoz,
            style,
            comment,
            skin.clone(),
        ))
    }

    pub fn create_grouping_space() -> Rc<dyn Component> {
        Rc::new(ComponentRoseGroupingSpace::new(7.0))
    }

    /// Lifeline, activation, destroy and delay components (`createComponent`).
    pub fn create_component(
        style: &Style,
        ctype: ComponentType,
        skin: &Rc<SkinParam>,
        display: Option<&Display>,
    ) -> Rc<dyn Component> {
        match ctype {
            ComponentType::ParticipantLine => {
                Rc::new(ComponentRoseLine::new(style, display, skin.clone()))
            }
            ComponentType::ActivationBoxCloseClose => {
                Rc::new(ComponentRoseActiveLine::new(style, true, true, display))
            }
            ComponentType::ActivationBoxCloseOpen => {
                Rc::new(ComponentRoseActiveLine::new(style, true, false, display))
            }
            ComponentType::ActivationBoxOpenClose => {
                Rc::new(ComponentRoseActiveLine::new(style, false, true, display))
            }
            ComponentType::ActivationBoxOpenOpen => {
                Rc::new(ComponentRoseActiveLine::new(style, false, false, display))
            }
            ComponentType::Divider => Rc::new(ComponentRoseDivider::new(
                style,
                display.expect("divider text"),
                skin.clone(),
            )),
            ComponentType::DelayText => Rc::new(ComponentRoseDelayText::new(
                style,
                display.expect("delay text"),
                skin.clone(),
            )),
            ComponentType::Newpage => Rc::new(ComponentRoseNewpage::new(style, skin.clone())),
            ComponentType::DelayLine => Rc::new(ComponentRoseDelayLine::new(style, skin.clone())),
            ComponentType::Destroy => Rc::new(ComponentRoseDestroy::new(style, skin.clone())),
            other => unimplemented!("component {other:?}"),
        }
    }
}
