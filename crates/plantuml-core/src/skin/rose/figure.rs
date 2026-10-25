//! Participants drawn as a figure with the label below or above: actor, boundary, control,
//! entity, database and queue (`ComponentRoseActor`, `ComponentRoseBoundary`, ...).

use crate::klimt::creole::display::Display;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::skin::component::{Area, Component, TextualCore};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use std::rc::Rc;

/// A figure and the text next to it; `head` puts the text under the figure's top.
pub struct ComponentRoseFigure {
    core: TextualCore,
    figure: Rc<dyn TextBlock>,
    head: bool,
}

impl ComponentRoseFigure {
    /// Text core shared by all of them.
    pub fn core(
        style: &Style,
        stereo: &Style,
        display: &Display,
        skin: Rc<SkinParam>,
    ) -> TextualCore {
        TextualCore::new(
            style,
            stereo,
            &LineBreakStrategy::NONE,
            Margins::new(0.0, 3.0, 0.0, 3.0),
            skin,
            display,
        )
    }

    pub fn new(core: TextualCore, figure: Rc<dyn TextBlock>, head: bool) -> Self {
        Self { core, figure, head }
    }

    fn text_middle_position(&self, sb: &dyn StringBounder) -> f64 {
        (self.preferred_width(sb) - self.core.text_width(sb)) / 2.0
    }
}

impl Component for ComponentRoseFigure {
    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let sb = ug.string_bounder();
        let dim = self.figure.calculate_dimension(sb);
        let delta = (self.preferred_width(sb) - dim.width) / 2.0;
        let text = self.core.text_block();
        let ug = if self.head {
            text.draw(
                &ug.apply_translate(UTranslate::new(self.text_middle_position(sb), dim.height)),
            );
            ug.apply_translate(UTranslate::dx(delta))
        } else {
            text.draw(&ug.apply_translate(UTranslate::dx(self.text_middle_position(sb))));
            ug.apply_translate(UTranslate::new(delta, self.core.text_height(sb)))
        };
        self.figure.draw(&ug);
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.figure.calculate_dimension(sb).height + self.core.text_height(sb)
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.figure
            .calculate_dimension(sb)
            .width
            .max(self.core.text_width(sb))
    }
}

/// Queue participant: the label lives inside the symbol (`ComponentRoseQueue`).
pub struct ComponentRoseQueue {
    symbol: Rc<dyn TextBlock>,
}

impl ComponentRoseQueue {
    pub fn new(core: &TextualCore, fashion: crate::klimt::fashion::Fashion) -> Self {
        let empty: Rc<dyn TextBlock> = Rc::new(EmptyBlock {
            width: 0.0,
            height: 0.0,
        });
        Self {
            symbol: crate::decoration::symbol::queue_small(
                core.text_block().clone(),
                empty,
                fashion,
            ),
        }
    }
}

impl Component for ComponentRoseQueue {
    fn draw_internal(&self, ug: &UGraphic, _area: &Area) {
        let dim = self.symbol.calculate_dimension(ug.string_bounder());
        let delta = (self.preferred_width(ug.string_bounder()) - dim.width) / 2.0;
        self.symbol.draw(&ug.apply_translate(UTranslate::dx(delta)));
    }

    fn preferred_height(&self, sb: &dyn StringBounder) -> f64 {
        self.symbol.calculate_dimension(sb).height
    }

    fn preferred_width(&self, sb: &dyn StringBounder) -> f64 {
        self.symbol.calculate_dimension(sb).width
    }
}
