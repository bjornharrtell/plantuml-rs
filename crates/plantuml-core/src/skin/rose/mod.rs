//! Default look of sequence diagram components (`skin/rose`).

pub mod arrow;
pub mod line;
pub mod misc;
pub mod participant;
pub mod text;

use crate::klimt::creole::display::Display;
use crate::klimt::geom::HorizontalAlignment;
use crate::sequencediagram::model::Participant;
use crate::skin::arrow::{ArrowConfiguration, ArrowDirection};
use crate::skin::component::{ArrowComponent, Component, ComponentType};
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style};
use std::rc::Rc;

use arrow::ComponentRoseArrow;
use line::ComponentRoseLine;
use misc::{ComponentRoseActiveLine, ComponentRoseDelayLine, ComponentRoseDestroy};
use participant::ComponentRoseParticipant;
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
        // Other participant shapes (actor, database, ...) fall back to the box for now.
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
