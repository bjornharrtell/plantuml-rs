//! Tiles without children: actions, start, stop, end and empty tiles.

use super::box_style::BoxStyle;
use super::{Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::rainbow::{LinkRendering, Rainbow};
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::colors::Colors;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UEllipse, ULine, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::stereo::Stereotype;
use crate::style::{queries, Margins, PName, Style, StyleBuilder};
use std::rc::Rc;

fn swimlanes_of(s: &Option<Rc<Swimlane>>) -> Vec<Rc<Swimlane>> {
    s.iter().cloned().collect()
}

/// An action (`FtileBox`).
pub struct FtileBox {
    skin: Rc<SkinParam>,
    padding: Margins,
    tb: Rc<dyn TextBlock>,
    round_corner: f64,
    shadowing: f64,
    horizontal_alignment: HorizontalAlignment,
    minimum_width: f64,
    in_rendering: LinkRendering,
    swimlane: Option<Rc<Swimlane>>,
    box_style: BoxStyle,
    border_color: HColor,
    back_color: HColor,
    style: Style,
}

impl FtileBox {
    pub fn create(
        skin: &Rc<SkinParam>,
        colors: &Colors,
        label: &Display,
        swimlane: Option<Rc<Swimlane>>,
        box_style: BoxStyle,
        stereotype: Option<Stereotype>,
        style_builder: Option<StyleBuilder>,
    ) -> Rc<FtileBox> {
        let stereotype = stereotype.or_else(|| box_style.stereotype());
        let builder = style_builder.unwrap_or_else(|| skin.style_builder());
        let query = queries::ACTIVITYDIAG_ACTIVITY.with_stereotype_obj(stereotype.as_ref());
        let style = builder
            .merged_style(&query)
            .expect("activity style")
            .override_colors(colors);
        let style_arrow = builder
            .merged_style(&queries::ACTIVITYDIAG_ARROW)
            .expect("arrow style");
        let horizontal_alignment = style.horizontal_alignment();
        let padding = style.padding();
        let tb = label.create_block(
            &style.font_configuration(None),
            skin.default_text_alignment(horizontal_alignment),
            &skin.skin_simple(),
            &style.wrap_width(),
            CreoleMode::Full,
            None,
            None,
            padding.left,
            padding.left,
        );
        Rc::new(FtileBox {
            skin: skin.clone(),
            padding,
            tb,
            round_corner: style.value(PName::RoundCorner).as_double(),
            shadowing: style.shadowing(),
            horizontal_alignment,
            minimum_width: style.value(PName::MinimumWidth).as_double(),
            in_rendering: LinkRendering::create(Rainbow::from_style(&style_arrow)),
            swimlane,
            box_style,
            border_color: style.value(PName::LineColor).as_color(),
            back_color: style.value(PName::BackGroundColor).as_color(),
            style,
        })
    }
}

impl Ftile for FtileBox {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.in_rendering.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        swimlanes_of(&self.swimlane)
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let raw = self.tb.calculate_dimension(sb);
        let raw = raw
            .delta(
                self.padding.left + self.padding.right,
                self.padding.bottom + self.padding.top,
            )
            .at_least(self.minimum_width, 0.0);
        FtileGeometry::with_out(
            raw.width + self.box_style.shield(),
            raw.height,
            raw.width / 2.0,
            0.0,
            raw.height,
        )
    }

    fn draw(&self, g: &FtileGraphic) {
        let ug = g.ug();
        let dim = self.calculate_dimension(ug.string_bounder());
        let ug = ug
            .apply_color(self.border_color.clone())
            .apply_back_color(self.back_color.clone())
            .apply_stroke(self.style.stroke());
        self.box_style.draw_me(
            &ug,
            dim.width,
            dim.height,
            self.shadowing,
            self.round_corner,
        );
        let sb = ug.string_bounder();
        match self.horizontal_alignment {
            HorizontalAlignment::Left => self
                .tb
                .draw(&ug.apply_translate(UTranslate::new(self.padding.left, self.padding.top))),
            HorizontalAlignment::Right => {
                let d = self.tb.calculate_dimension(sb);
                self.tb.draw(&ug.apply_translate(UTranslate::new(
                    dim.width - d.width - self.padding.right,
                    self.padding.bottom,
                )));
            }
            HorizontalAlignment::Center => {
                let d = self.tb.calculate_dimension(sb);
                self.tb.draw(&ug.apply_translate(UTranslate::new(
                    (dim.width - d.width) / 2.0,
                    self.padding.bottom,
                )));
            }
        }
    }
}

/// Which circle a [`FtileCircle`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleKind {
    Start,
    Stop,
    EndCross,
}

/// The start, stop and end nodes (`FtileCircleStart`, `FtileCircleStop`, `FtileCircleEndCross`).
pub struct FtileCircle {
    skin: Rc<SkinParam>,
    swimlane: Option<Rc<Swimlane>>,
    kind: CircleKind,
    style: Style,
    colors: Colors,
}

const START_SIZE: f64 = 20.0;
const STOP_SIZE: f64 = 22.0;
const END_SIZE: f64 = 20.0;

impl FtileCircle {
    pub fn create(
        skin: &Rc<SkinParam>,
        swimlane: Option<Rc<Swimlane>>,
        kind: CircleKind,
        colors: &Colors,
    ) -> Rc<FtileCircle> {
        let query = match kind {
            CircleKind::Start => &queries::ACTIVITYDIAG_CIRCLE_START,
            CircleKind::Stop => &queries::ACTIVITYDIAG_CIRCLE_STOP,
            CircleKind::EndCross => &queries::ACTIVITYDIAG_CIRCLE_END,
        };
        let style = skin
            .style_builder()
            .merged_style(query)
            .expect("circle style");
        Rc::new(FtileCircle {
            skin: skin.clone(),
            swimlane,
            kind,
            style,
            colors: colors.clone(),
        })
    }

    fn size(&self) -> f64 {
        match self.kind {
            CircleKind::Start => START_SIZE,
            CircleKind::Stop => STOP_SIZE,
            CircleKind::EndCross => END_SIZE,
        }
    }
}

impl Ftile for FtileCircle {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        swimlanes_of(&self.swimlane)
    }

    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> FtileGeometry {
        let s = self.size();
        match self.kind {
            CircleKind::Start => FtileGeometry::with_out(s, s, s / 2.0, 0.0, s),
            _ => FtileGeometry::new(s, s, s / 2.0, 0.0),
        }
    }

    fn draw(&self, g: &FtileGraphic) {
        let ug = g.ug();
        let size = self.size();
        let back = self.colors.color_for(&self.style, PName::BackGroundColor);
        let line = self.colors.color_for(&self.style, PName::LineColor);
        let mut circle = UEllipse::build(size, size);
        circle.delta_shadow = self.style.shadowing();
        match self.kind {
            CircleKind::Start => {
                ug.apply_color(line)
                    .apply_back_color(back)
                    .apply_stroke(self.style.stroke())
                    .draw(circle);
            }
            CircleKind::Stop => {
                ug.apply_back_color(HColor::TRANSPARENT)
                    .apply_color(line.clone())
                    .apply_stroke(self.style.stroke())
                    .draw(circle);
                let delta = 5.0;
                let small = UEllipse::build(size - delta * 2.0, size - delta * 2.0);
                let ug = if line == back {
                    let middle = line.clone();
                    ug.apply_back_color(back).apply_color(middle)
                } else {
                    ug.apply_back_color(back).apply_color(line)
                };
                ug.apply_translate(UTranslate::new(delta, delta))
                    .draw(small);
            }
            CircleKind::EndCross => {
                let ug0 = ug.apply_color(line).apply_back_color(HColor::TRANSPARENT);
                ug0.apply_stroke(self.style.stroke()).draw(circle);
                let ug1 = ug0.apply_back_color(back);
                let thickness = 2.5;
                let size2 = (size - thickness) / 2f64.sqrt();
                let delta = (size - size2) / 2.0;
                let ug1 = ug1.apply_stroke(UStroke::with_thickness(thickness));
                ug1.apply_translate(UTranslate::new(delta, delta))
                    .draw(ULine::new(size2, size2));
                ug1.apply_translate(UTranslate::new(delta, size - delta))
                    .draw(ULine::new(size2, -size2));
            }
        }
    }
}

/// A tile of fixed size that draws nothing (`FtileEmpty`).
pub struct FtileEmpty {
    skin: Rc<SkinParam>,
    width: f64,
    height: f64,
    swimlane: Option<Rc<Swimlane>>,
}

impl FtileEmpty {
    pub fn new(
        skin: &Rc<SkinParam>,
        width: f64,
        height: f64,
        swimlane: Option<Rc<Swimlane>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            skin: skin.clone(),
            width,
            height,
            swimlane,
        })
    }
}

impl Ftile for FtileEmpty {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        swimlanes_of(&self.swimlane)
    }

    fn calculate_dimension(&self, _sb: &dyn StringBounder) -> FtileGeometry {
        FtileGeometry::with_out(self.width, self.height, self.width / 2.0, 0.0, self.height)
    }

    fn draw(&self, _g: &FtileGraphic) {}
}
