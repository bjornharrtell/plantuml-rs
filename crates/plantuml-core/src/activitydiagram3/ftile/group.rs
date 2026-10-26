//! A group or partition around a part of the flow (`FtileGroup`).

use super::factory::FtileRef;
use super::{same_ftile, Ftile, FtileGeometry, FtileGraphic};
use crate::decoration::usymbol::USymbol;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style};
use std::rc::Rc;

const HORIZONTAL_MARGIN: f64 = 10.0;
/// Extra space needed around the title, below it and under the content.
const TITLE_MARGIN: f64 = 10.0;
const BOTTOM_MARGIN: f64 = 15.0;

pub struct FtileGroup {
    inner: FtileRef,
    name: Rc<dyn TextBlock>,
    back: HColor,
    border: HColor,
    shadowing: f64,
    stroke: UStroke,
    round_corner: f64,
    symbol: Rc<dyn USymbol>,
    title_alignment: HorizontalAlignment,
}

impl FtileGroup {
    pub fn new(
        inner: FtileRef,
        title: Display,
        back: Option<HColor>,
        symbol: Rc<dyn USymbol>,
        style: &Style,
        skin: &SkinParam,
    ) -> Rc<Self> {
        let name = title.create_block(
            &style.font_configuration(None),
            HorizontalAlignment::Left,
            &skin.skin_simple(),
            &crate::klimt::creole::LineBreakStrategy::NONE,
            CreoleMode::SimpleLine,
            None,
            None,
            0.0,
            0.0,
        );
        Rc::new(Self {
            inner,
            name,
            back: back.unwrap_or_else(|| style.value(PName::BackGroundColor).as_color()),
            border: style.value(PName::LineColor).as_color(),
            shadowing: style.shadowing(),
            stroke: style.stroke(),
            round_corner: style.value(PName::RoundCorner).as_double(),
            symbol,
            title_alignment: HorizontalAlignment::Left,
        })
    }

    /// Geometry of the inner tile once its horizontal margin is added.
    fn inner_geometry(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let orig = self.inner.calculate_dimension(sb);
        if orig.has_point_out() {
            FtileGeometry::with_out(
                orig.width + 2.0 * HORIZONTAL_MARGIN,
                orig.height,
                orig.left() + HORIZONTAL_MARGIN,
                orig.in_y(),
                orig.out_y(),
            )
        } else {
            FtileGeometry::new(
                orig.width + 2.0 * HORIZONTAL_MARGIN,
                orig.height,
                orig.left() + HORIZONTAL_MARGIN,
                orig.in_y(),
            )
        }
    }

    fn title_height(&self, sb: &dyn StringBounder) -> f64 {
        let dim = self.name.calculate_dimension(sb);
        (dim.height + TITLE_MARGIN).max(25.0)
    }

    fn supp_width(&self, sb: &dyn StringBounder) -> f64 {
        let orig = self.inner_geometry(sb);
        let dim = self.name.calculate_dimension(sb);
        (dim.width + TITLE_MARGIN - orig.width).max(0.0)
    }
}

impl Ftile for FtileGroup {
    fn skin(&self) -> Rc<SkinParam> {
        self.inner.skin()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.inner.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let orig = self.inner_geometry(sb);
        let supp = self.supp_width(sb);
        let title = self.title_height(sb);
        let left = orig.left() + supp / 2.0;
        let height = orig.height + title + BOTTOM_MARGIN;
        if orig.has_point_out() {
            FtileGeometry::with_out(
                orig.width + supp,
                height,
                left,
                orig.in_y() + title,
                orig.out_y() + title,
            )
        } else {
            FtileGeometry::new(
                orig.width + supp,
                height,
                left,
                orig.in_y() + title,
            )
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let offset = UTranslate::new(
            self.supp_width(sb) / 2.0 + HORIZONTAL_MARGIN,
            self.title_height(sb),
        );
        if same_ftile(child, self.inner.as_ref()) {
            return Some(offset);
        }
        self.inner
            .translate_for(child, sb)
            .map(|t| t.compose(offset))
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let total = self.calculate_dimension(sb);
        let fashion = Fashion::new(Some(self.back.clone()), Some(self.border.clone()))
            .with_delta_shadow(self.shadowing)
            .with_stroke(self.stroke)
            .with_corner(self.round_corner, 0.0);
        let empty: Rc<dyn TextBlock> = Rc::new(EmptyBlock {
            width: 0.0,
            height: 0.0,
        });
        let big = self.symbol.as_big(
            self.name.clone(),
            self.title_alignment,
            empty,
            total.width,
            total.height,
            fashion,
            HorizontalAlignment::Center,
        );
        big.draw(g.ug());
        let offset = UTranslate::new(
            self.supp_width(sb) / 2.0 + HORIZONTAL_MARGIN,
            self.title_height(sb),
        );
        g.apply_translate(offset)
            .draw_ftile(self.inner.as_ref());
    }
}

