//! A node with its subtree laid out as a finger (`FingerImpl`).

use super::box_block::BoxBlock;
use super::idea::{node_style, Branch, IdeaShape, MINDMAP_QUERIES};
use super::tetris::{SymetricalTee, Tetris};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::creole::CreoleMode;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, UStroke, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::{with_margin, EmptyBlock, TextBlock};
use crate::skin::{Rankdir, SkinParam};
use crate::style::{Margins, PName, Style, StyleBuilder};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An idea (the phalanx) and its children (the nail).
pub struct FingerImpl {
    label: Display,
    shape: IdeaShape,
    back_color: Option<HColor>,
    style: Style,
    style_arrow: Style,
    skin: Rc<SkinParam>,
    /// 1 to the right (or down), -1 to the left (or up).
    direction: f64,
    top_to_bottom: bool,
    draw_phalanx: Cell<bool>,
    nail: Vec<Rc<FingerImpl>>,
    tetris: RefCell<Option<Rc<Tetris>>>,
    phalanx: RefCell<Option<Rc<dyn TextBlock>>>,
}

impl FingerImpl {
    /// Builds the finger of the idea at `index` and of all its descendants (`build`).
    pub fn build(
        branch: &Branch,
        index: usize,
        skin: &Rc<SkinParam>,
        builder: &StyleBuilder,
        direction: bool,
    ) -> Rc<FingerImpl> {
        let idea = &branch.ideas[index];
        let (info, ancestors) = branch.infos(index);
        let style = node_style(builder, &MINDMAP_QUERIES, &info, &ancestors);
        let arrow_query = crate::style::queries::MINDMAPDIAG_ARROW
            .with_stereotype_obj(idea.stereotype.as_ref())
            .add_level(idea.level);
        let style_arrow = builder
            .merged_style(&arrow_query)
            .expect("arrow style of mindmap");
        let nail = idea
            .children
            .iter()
            .map(|&c| FingerImpl::build(branch, c, skin, builder, direction))
            .collect();
        Rc::new(FingerImpl {
            label: idea.label.clone(),
            shape: idea.shape,
            back_color: idea.back_color.clone(),
            style,
            style_arrow,
            skin: skin.clone(),
            direction: if direction { 1.0 } else { -1.0 },
            top_to_bottom: skin.rankdir == Rankdir::TopToBottom,
            draw_phalanx: Cell::new(true),
            nail,
            tetris: RefCell::new(None),
            phalanx: RefCell::new(None),
        })
    }

    pub fn do_not_draw_first_phalanx(&self) {
        self.draw_phalanx.set(false);
        *self.phalanx.borrow_mut() = None;
        *self.tetris.borrow_mut() = None;
    }

    fn margin(&self) -> Margins {
        self.style.margin()
    }

    fn phalanx(&self) -> Rc<dyn TextBlock> {
        if let Some(p) = self.phalanx.borrow().as_ref() {
            return p.clone();
        }
        let block = self.create_phalanx();
        *self.phalanx.borrow_mut() = Some(block.clone());
        block
    }

    fn create_phalanx(&self) -> Rc<dyn TextBlock> {
        if !self.draw_phalanx.get() {
            return Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            });
        }
        if self.shape == IdeaShape::Box {
            let boxed: Rc<dyn TextBlock> = Rc::new(BoxBlock::new(
                &self.style,
                &self.skin,
                &self.label,
                self.back_color.as_ref(),
            ));
            let margin = self.margin();
            return if self.top_to_bottom {
                with_margin(boxed, Margins::new(0.0, margin.right, 0.0, margin.left))
            } else {
                with_margin(boxed, Margins::new(margin.top, 0.0, margin.bottom, 0.0))
            };
        }
        let text = self.label.create_block(
            &self.style.font_configuration(None),
            self.style.horizontal_alignment(),
            &self.skin.skin_simple(),
            &self.style.wrap_width(),
            CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        );
        if self.direction > 0.0 {
            with_margin(text, Margins::new(1.0, 0.0, 1.0, 3.0))
        } else {
            with_margin(text, Margins::new(1.0, 3.0, 1.0, 0.0))
        }
    }

    fn tetris(&self, sb: &dyn StringBounder) -> Rc<Tetris> {
        if let Some(t) = self.tetris.borrow().as_ref() {
            return t.clone();
        }
        let mut tetris = Tetris::default();
        for child in &self.nail {
            tetris.add(child.as_symetrical_tee(sb));
        }
        tetris.balance();
        let tetris = Rc::new(tetris);
        *self.tetris.borrow_mut() = Some(tetris.clone());
        tetris
    }

    fn as_symetrical_tee(&self, sb: &dyn StringBounder) -> SymetricalTee {
        let thickness1 = self.phalanx_thickness(sb);
        let elongation1 = self.phalanx_elongation(sb);
        if self.nail.is_empty() {
            return SymetricalTee {
                thickness1,
                elongation1,
                thickness2: 0.0,
                elongation2: 0.0,
            };
        }
        SymetricalTee {
            thickness1,
            elongation1: elongation1 + self.x1(),
            thickness2: self.nail_thickness(sb),
            elongation2: self.x2() + self.nail_elongation(sb),
        }
    }

    fn x1(&self) -> f64 {
        if self.top_to_bottom {
            self.margin().top
        } else {
            self.margin().left
        }
    }

    fn x2(&self) -> f64 {
        if self.top_to_bottom {
            self.margin().bottom + 5.0
        } else {
            self.margin().right + 30.0
        }
    }

    pub fn x12(&self) -> f64 {
        self.x1() + self.x2()
    }

    pub fn phalanx_thickness(&self, sb: &dyn StringBounder) -> f64 {
        let dim = self.phalanx().calculate_dimension(sb);
        if self.top_to_bottom {
            dim.width
        } else {
            dim.height
        }
    }

    pub fn phalanx_elongation(&self, sb: &dyn StringBounder) -> f64 {
        let dim = self.phalanx().calculate_dimension(sb);
        if self.top_to_bottom {
            dim.height
        } else {
            dim.width
        }
    }

    pub fn nail_thickness(&self, sb: &dyn StringBounder) -> f64 {
        self.tetris(sb).height()
    }

    pub fn nail_elongation(&self, sb: &dyn StringBounder) -> f64 {
        self.tetris(sb).width()
    }

    pub fn full_thickness(&self, sb: &dyn StringBounder) -> f64 {
        self.phalanx_thickness(sb).max(self.nail_thickness(sb))
    }

    pub fn full_elongation(&self, sb: &dyn StringBounder) -> f64 {
        self.phalanx_elongation(sb) + self.nail_elongation(sb)
    }

    pub fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let phalanx = self.phalanx();
        let dim_phalanx: Dimension = phalanx.calculate_dimension(sb);
        if self.draw_phalanx.get() {
            let (pos_x, pos_y);
            if self.top_to_bottom {
                pos_x = -self.phalanx_thickness(sb) / 2.0;
                pos_y = if self.direction > 0.0 {
                    0.0
                } else {
                    -dim_phalanx.height
                };
            } else {
                pos_x = if self.direction > 0.0 {
                    0.0
                } else {
                    -dim_phalanx.width
                };
                pos_y = -self.phalanx_thickness(sb) / 2.0;
            }
            phalanx.draw(&ug.apply_translate(UTranslate::new(pos_x, pos_y)));
        }
        let p1 = if self.top_to_bottom {
            (0.0, self.direction * dim_phalanx.height)
        } else {
            (self.direction * dim_phalanx.width, 0.0)
        };
        let tetris = self.tetris(sb);
        for (child, stp) in self.nail.iter().zip(tetris.elements()) {
            let p2 = if self.top_to_bottom {
                (stp.y(), self.direction * (dim_phalanx.height + self.x12()))
            } else {
                (self.direction * (dim_phalanx.width + self.x12()), stp.y())
            };
            child.draw(&ug.apply_translate(UTranslate::new(p2.0, p2.1)));
            let link_color = self.link_color();
            if !link_color.is_transparent() {
                self.draw_line(
                    &ug.apply_color(link_color).apply_stroke(self.link_stroke()),
                    p1,
                    p2,
                );
            }
        }
    }

    fn link_color(&self) -> HColor {
        self.style_arrow.value(PName::LineColor).as_color()
    }

    fn link_stroke(&self) -> UStroke {
        self.style_arrow.stroke()
    }

    fn draw_line(&self, ug: &UGraphic, p1: (f64, f64), p2: (f64, f64)) {
        let mut path = UPath::new();
        path.move_to(p1.0, p1.1);
        if self.top_to_bottom {
            let delta1 = self.direction * 3.0;
            let delta2 = self.direction * 10.0;
            path.line_to(p1.0, p1.1 + delta1);
            path.cubic_to(
                p1.0,
                p1.1 + delta2,
                p2.0,
                p2.1 - delta2,
                p2.0,
                p2.1 - delta1,
            );
        } else {
            let delta1 = self.direction * 10.0;
            let delta2 = self.direction * 25.0;
            path.line_to(p1.0 + delta1, p1.1);
            path.cubic_to(
                p1.0 + delta2,
                p1.1,
                p2.0 - delta2,
                p2.1,
                p2.0 - delta1,
                p2.1,
            );
        }
        path.line_to(p2.0, p2.1);
        ug.draw(path);
    }
}
