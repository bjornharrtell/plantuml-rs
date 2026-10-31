//! Builders of tiles (`FtileFactory`, `VCompactFactory` and the `FtileFactoryDelegator*` chain).
//!
//! A delegator implements [`FtileFactory::inner`] and overrides only the methods it decorates;
//! every other method forwards to the inner factory. Later constructs (`if`, `while`, `fork`,
//! groups, notes...) add methods here and a delegator each, wrapped in [`default_factory`].

use super::box_style::BoxStyle;
use super::leaf::{CircleKind, FtileBox, FtileCircle};
use super::wrap::{
    ConnectionVerticalDown, FtileAssemblySimple, FtileDecorate, FtileMargedRight,
    FtileMargedVertically, FtileWithConnection,
};
use super::{Connection, Ftile};
use crate::activitydiagram3::rainbow::{LinkRendering, Rainbow};
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::colors::Colors;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, StyleBuilder};
use crate::url::Url;
use std::rc::Rc;

pub type FtileRef = Rc<dyn Ftile>;
pub type SwimlaneRef = Option<Rc<Swimlane>>;

pub trait FtileFactory {
    /// The factory this one decorates; the base factory has none.
    fn inner(&self) -> &dyn FtileFactory {
        unreachable!("base factory implements every method")
    }

    fn string_bounder(&self) -> Rc<dyn StringBounder> {
        self.inner().string_bounder()
    }

    fn skin(&self) -> Rc<SkinParam> {
        self.inner().skin()
    }

    fn start(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        self.inner().start(swimlane, colors)
    }

    fn stop(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        self.inner().stop(swimlane, colors)
    }

    fn end(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        self.inner().end(swimlane, colors)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: SwimlaneRef,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<Stereotype>,
        style_builder: Option<StyleBuilder>,
    ) -> FtileRef {
        self.inner()
            .activity(label, swimlane, style, colors, stereotype, style_builder)
    }

    fn add_url(&self, ftile: FtileRef, url: &Url) -> FtileRef {
        self.inner().add_url(ftile, url)
    }

    fn decorate_in(&self, ftile: FtileRef, link: LinkRendering) -> FtileRef {
        self.inner().decorate_in(ftile, link)
    }

    fn decorate_out(&self, ftile: FtileRef, link: LinkRendering) -> FtileRef {
        self.inner().decorate_out(ftile, link)
    }

    fn assembly(&self, tile1: FtileRef, tile2: FtileRef) -> FtileRef {
        self.inner().assembly(tile1, tile2)
    }
}

/// The base factory: builds the vertical, compact look (`VCompactFactory`).
pub struct VCompactFactory {
    skin: Rc<SkinParam>,
    string_bounder: Rc<dyn StringBounder>,
}

impl VCompactFactory {
    pub fn new(skin: Rc<SkinParam>, string_bounder: Rc<dyn StringBounder>) -> Self {
        Self {
            skin,
            string_bounder,
        }
    }
}

impl FtileFactory for VCompactFactory {
    fn string_bounder(&self) -> Rc<dyn StringBounder> {
        self.string_bounder.clone()
    }

    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn start(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        FtileCircle::create(&self.skin, swimlane, CircleKind::Start, colors)
    }

    fn stop(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        FtileCircle::create(&self.skin, swimlane, CircleKind::Stop, colors)
    }

    fn end(&self, swimlane: SwimlaneRef, colors: &Colors) -> FtileRef {
        FtileCircle::create(&self.skin, swimlane, CircleKind::EndCross, colors)
    }

    fn activity(
        &self,
        label: &Display,
        swimlane: SwimlaneRef,
        style: BoxStyle,
        colors: &Colors,
        stereotype: Option<Stereotype>,
        style_builder: Option<StyleBuilder>,
    ) -> FtileRef {
        FtileBox::create(
            &self.skin,
            colors,
            label,
            swimlane,
            style,
            stereotype,
            style_builder,
        )
    }

    fn add_url(&self, ftile: FtileRef, _url: &Url) -> FtileRef {
        // Links are not drawn yet (no `<a>` support in the SVG backend).
        ftile
    }

    fn decorate_in(&self, ftile: FtileRef, link: LinkRendering) -> FtileRef {
        FtileDecorate::decorate_in(ftile, link)
    }

    fn decorate_out(&self, ftile: FtileRef, link: LinkRendering) -> FtileRef {
        FtileDecorate::decorate_out(ftile, link)
    }

    fn assembly(&self, tile1: FtileRef, tile2: FtileRef) -> FtileRef {
        FtileAssemblySimple::new(tile1, tile2)
    }
}

/// Colors of the arrow entering `tile`, from its link or else the arrow style
/// (`getInLinkRenderingColor`).
pub fn in_link_rendering_color(skin: &SkinParam, tile: &dyn Ftile) -> Rainbow {
    let rainbow = tile.in_link_rendering().rainbow().clone();
    if rainbow.size() > 0 {
        return rainbow;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::ACTIVITYDIAG_ARROW)
        .expect("arrow style");
    Rainbow::from_style(&style)
}

/// Colour of the arrow leaving a tile: its own out link, else the arrow style (`Branch.getOut`).
pub fn out_link_rendering_color(skin: &SkinParam, tile: &dyn Ftile) -> Rainbow {
    let rainbow = tile.out_link_rendering().rainbow().clone();
    if rainbow.size() > 0 {
        return rainbow;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::ACTIVITYDIAG_ARROW)
        .expect("arrow style");
    Rainbow::from_style(&style)
}

/// Label of an arrow in the arrow font (`getTextBlock`).
pub fn arrow_text_block(skin: &SkinParam, display: &Display) -> Option<Rc<dyn TextBlock>> {
    if display.is_null() {
        return None;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::ACTIVITYDIAG_ARROW)
        .expect("arrow style");
    Some(display.create_block(
        &style.font_configuration(None),
        HorizontalAlignment::Left,
        &skin.skin_simple(),
        &crate::klimt::creole::LineBreakStrategy::NONE,
        CreoleMode::SimpleLine,
        None,
        None,
        0.0,
        0.0,
    ))
}

/// Puts the arrow between consecutive tiles (`FtileFactoryDelegatorAssembly`).
pub struct FtileFactoryDelegatorAssembly {
    factory: Box<dyn FtileFactory>,
}

impl FtileFactoryDelegatorAssembly {
    pub fn new(factory: Box<dyn FtileFactory>) -> Self {
        Self { factory }
    }
}

impl FtileFactory for FtileFactoryDelegatorAssembly {
    fn inner(&self) -> &dyn FtileFactory {
        self.factory.as_ref()
    }

    fn assembly(&self, tile1: FtileRef, tile2: FtileRef) -> FtileRef {
        let skin = self.skin();
        let sb = self.string_bounder();
        let sb = sb.as_ref();
        let mut height = 35.0;
        let text_block = arrow_text_block(&skin, tile2.in_link_rendering().display());
        if let Some(tb) = &text_block {
            height += tb.calculate_dimension(sb).height;
        }
        let tile1_and_space: FtileRef = FtileMargedVertically::new(tile1.clone(), 0.0, height);
        let mut result = self
            .factory
            .assembly(tile1_and_space.clone(), tile2.clone());
        let geo = tile1.calculate_dimension(sb);
        if !geo.has_point_out() {
            return result;
        }
        let translate1 = result
            .translate_for(tile1_and_space.as_ref(), sb)
            .expect("first tile");
        let p1 = geo.translate(translate1).point_out();
        let translate2 = result
            .translate_for(tile2.as_ref(), sb)
            .expect("second tile");
        let p2 = tile2
            .calculate_dimension(sb)
            .translate(translate2)
            .point_in();
        let color = in_link_rendering_color(&skin, tile2.as_ref());
        let connection =
            ConnectionVerticalDown::new(tile1, tile2, p1, p2, color, text_block.clone());
        let max_x = connection.max_x(sb);
        let connections: Vec<Rc<dyn Connection>> = vec![Rc::new(connection)];
        result = FtileWithConnection::wrap(result, connections);
        if text_block.is_some() {
            let width = result.calculate_dimension(sb).width;
            if width < max_x {
                result = FtileMargedRight::new(result, max_x);
            }
        }
        result
    }
}

/// The factory used by [`Swimlanes`](crate::activitydiagram3::swimlanes::Swimlanes), in the
/// order of `Swimlanes.getFtileFactory`.
pub fn default_factory(
    skin: Rc<SkinParam>,
    string_bounder: Rc<dyn StringBounder>,
) -> Box<dyn FtileFactory> {
    let factory: Box<dyn FtileFactory> = Box::new(VCompactFactory::new(skin, string_bounder));
    // Java wraps here, from the inside out: AddUrl, Assembly, If, Switch, While, Repeat,
    // CreateParallel, AddNote, CreateGroup. Only Assembly exists so far (AddUrl is folded into
    // the base factory).
    Box::new(FtileFactoryDelegatorAssembly::new(factory))
}
