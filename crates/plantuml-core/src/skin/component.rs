//! Skin components (`skin/Component`, `AbstractComponent`, `AbstractTextualComponent`).

use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::{CreoleMode, LineBreakStrategy};
use crate::klimt::drawing::ugraphic::{UGraphic, UStroke, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::skin::SkinParam;
use crate::style::{Margins, PName, Style};
use std::rc::Rc;

/// Space given to a component when drawn (`Area`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Area {
    pub dimension_to_use: Dimension,
    pub delta_x1: f64,
    pub live_delta_size: f64,
    pub level: i32,
    pub text_delta_x: f64,
}

impl Area {
    pub fn new(dimension_to_use: Dimension) -> Area {
        Area {
            dimension_to_use,
            ..Area::default()
        }
    }

    pub fn create(width: f64, height: f64) -> Area {
        Area::new(Dimension::new(width, height))
    }
}

/// Visual element of a skin (`Component`).
pub trait Component {
    fn preferred_width(&self, sb: &dyn StringBounder) -> f64;
    fn preferred_height(&self, sb: &dyn StringBounder) -> f64;

    fn preferred_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        Dimension::new(self.preferred_width(sb), self.preferred_height(sb))
    }

    /// Offset applied before drawing (`getPaddingX`).
    fn padding_x(&self) -> f64 {
        0.0
    }

    /// Offset applied before drawing (`getPaddingY`).
    fn padding_y(&self) -> f64 {
        0.0
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area);

    fn draw_background_internal(&self, _ug: &UGraphic, _area: &Area) {}

    /// Draws the background pass when `background`, else the foreground (`drawU`).
    fn draw(&self, ug: &UGraphic, area: &Area, background: bool) {
        let ug = ug.apply_translate(UTranslate::new(self.padding_x(), self.padding_y()));
        if background {
            self.draw_background_internal(&ug, area);
        } else {
            self.draw_internal(&ug, area);
        }
    }
}

/// Anchor points of an arrow component (`ArrowComponent`).
pub trait ArrowComponent: Component {
    fn start_point(&self, sb: &dyn StringBounder, dimension_to_use: Dimension) -> (f64, f64);
    fn end_point(&self, sb: &dyn StringBounder, dimension_to_use: Dimension) -> (f64, f64);
    fn y_point(&self, sb: &dyn StringBounder) -> f64;
    fn pos_arrow(&self, sb: &dyn StringBounder) -> f64;
}

/// Style access shared by components (`AbstractComponent`).
#[derive(Clone)]
pub struct ComponentBase {
    pub style: Style,
    pub skin: Rc<SkinParam>,
}

impl ComponentBase {
    pub fn new(style: Style, skin: Rc<SkinParam>) -> Self {
        Self { style, skin }
    }

    pub fn color_font(&self) -> HColor {
        self.style.value(PName::FontColor).as_color()
    }

    pub fn color_line(&self) -> HColor {
        self.style.value(PName::LineColor).as_color()
    }

    pub fn color_back_ground(&self) -> HColor {
        self.style.value(PName::BackGroundColor).as_color()
    }

    pub fn round_corner(&self) -> f64 {
        f64::from(self.style.value(PName::RoundCorner).as_int())
    }

    pub fn diagonal_corner(&self) -> f64 {
        f64::from(self.style.value(PName::DiagonalCorner).as_int())
    }

    pub fn stroke(&self) -> UStroke {
        self.style.stroke()
    }

    pub fn shadowing(&self) -> f64 {
        self.style.shadowing()
    }

    pub fn font(&self) -> UFont {
        self.style.font()
    }

    pub fn font_configuration(&self) -> FontConfiguration {
        self.style.font_configuration(None)
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.style.horizontal_alignment()
    }
}

/// Component showing a text with padding (`AbstractTextualComponent`).
#[derive(Clone)]
pub struct TextualCore {
    pub base: ComponentBase,
    text_block: Rc<dyn TextBlock>,
    padding: Margins,
}

impl TextualCore {
    pub fn new(
        style: &Style,
        stereo: &Style,
        max_message_size: &LineBreakStrategy,
        padding: Margins,
        skin: Rc<SkinParam>,
        display: &Display,
    ) -> Self {
        let base = ComponentBase::new(style.clone(), skin.clone());
        let fc = base.font_configuration();
        let horizontal_alignment = base.horizontal_alignment();
        let font_for_stereotype = stereo.font();
        let color_for_stereotype = stereo.value(PName::FontColor).as_color();
        let display = display.without_stereotype_if_needed(style);
        let text_block: Rc<dyn TextBlock> = if display.size() == 1 && display.text_at(0).is_empty()
        {
            Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            })
        } else {
            display.create_block(
                &fc,
                horizontal_alignment,
                &skin.skin_simple(),
                max_message_size,
                CreoleMode::Full,
                Some(font_for_stereotype),
                Some(color_for_stereotype),
                padding.left,
                padding.right,
            )
        };
        Self {
            base,
            text_block,
            padding,
        }
    }

    pub fn text_block(&self) -> &Rc<dyn TextBlock> {
        &self.text_block
    }

    pub fn pure_text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(sb).width
    }

    pub fn text_width(&self, sb: &dyn StringBounder) -> f64 {
        self.pure_text_width(sb) + self.padding.left + self.padding.right
    }

    pub fn text_height(&self, sb: &dyn StringBounder) -> f64 {
        self.text_block.calculate_dimension(sb).height + self.padding.top + self.padding.bottom
    }

    pub fn padding(&self) -> Margins {
        self.padding
    }
}

/// Kinds of components created by the skin factory (`ComponentType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    Arrow,
    ActorHead,
    ActorTail,
    BoundaryHead,
    BoundaryTail,
    ControlHead,
    ControlTail,
    EntityHead,
    EntityTail,
    QueueHead,
    QueueTail,
    DatabaseHead,
    DatabaseTail,
    CollectionsHead,
    CollectionsTail,
    ActivationBoxCloseClose,
    ActivationBoxCloseOpen,
    ActivationBoxOpenClose,
    ActivationBoxOpenOpen,
    DelayText,
    Destroy,
    DelayLine,
    ParticipantLine,
    GroupingElseLegacy,
    GroupingElseTeoz,
    GroupingHeaderLegacy,
    GroupingHeaderTeoz,
    GroupingSpace,
    Newpage,
    Note,
    NoteHexagonal,
    NoteBox,
    Divider,
    Reference,
    Englober,
    ParticipantHead,
    ParticipantTail,
}

impl ComponentType {
    pub fn is_arrow(self) -> bool {
        self == ComponentType::Arrow
    }

    /// Style signature used by this component (`getStyleQuery`).
    pub fn style_query(self) -> crate::style::StyleQuery {
        use crate::style::queries as q;
        use ComponentType::*;
        match self {
            ParticipantHead | ParticipantTail => q::SEQUENCEDIAG_PARTICIPANT.clone(),
            ParticipantLine => q::SEQUENCEDIAG_LIFELINE.clone(),
            ActivationBoxCloseClose
            | ActivationBoxCloseOpen
            | ActivationBoxOpenClose
            | ActivationBoxOpenOpen => q::SEQUENCEDIAG_ACTIVATIONBOX.clone(),
            Destroy => q::SEQUENCEDIAG_LIFELINE_DESTROY.clone(),
            Divider => q::SEQUENCEDIAG_SEPARATOR.clone(),
            Englober => q::SEQUENCEDIAG_BOX.clone(),
            Newpage => q::SEQUENCEDIAG_NEWPAGE.clone(),
            Note => q::SEQUENCEDIAG_NOTE.clone(),
            DelayText | DelayLine => q::SEQUENCEDIAG_LIFELINE_DELAY.clone(),
            other => panic!("no style query for {other:?}"),
        }
    }
}
