//! Helpers shared by the symbols (`USymbol.Margin`, `TextBlockUtils.mergeTB`, `UGraphicStencil`).

use crate::klimt::drawing::ugraphic::{UGraphic, UStroke, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{RectangleStencil, TextBlock, VerticalBlock};
use std::rc::Rc;

/// Space around the label of a symbol (`USymbol.Margin`).
#[derive(Debug, Clone, Copy)]
pub struct Margin {
    pub x1: f64,
    pub x2: f64,
    pub y1: f64,
    pub y2: f64,
}

impl Margin {
    pub const fn new(x1: f64, x2: f64, y1: f64, y2: f64) -> Self {
        Self { x1, x2, y1, y2 }
    }

    pub fn add_dimension(&self, dim: Dimension) -> Dimension {
        Dimension::new(
            dim.width + self.x1 + self.x2,
            dim.height + self.y1 + self.y2,
        )
    }
}

/// A block defined by two closures, like the anonymous `TextBlock` of the Java symbols.
struct FnBlock<D, W> {
    dim: D,
    draw: W,
}

impl<D, W> TextBlock for FnBlock<D, W>
where
    D: Fn(&dyn StringBounder) -> Dimension,
    W: Fn(&UGraphic),
{
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        (self.dim)(sb)
    }

    fn draw(&self, ug: &UGraphic) {
        (self.draw)(ug)
    }
}

pub fn fn_block<D, W>(dim: D, draw: W) -> Rc<dyn TextBlock>
where
    D: Fn(&dyn StringBounder) -> Dimension + 'static,
    W: Fn(&UGraphic) + 'static,
{
    Rc::new(FnBlock { dim, draw })
}

/// Block of a fixed size, as returned by every `asBig`.
pub fn fn_block_sized<W>(width: f64, height: f64, draw: W) -> Rc<dyn TextBlock>
where
    W: Fn(&UGraphic) + 'static,
{
    fn_block(move |_| Dimension::new(width, height), draw)
}

/// `TextBlockUtils.mergeTB`: stereotype above label.
pub fn merge_tb(
    stereotype: &Rc<dyn TextBlock>,
    label: &Rc<dyn TextBlock>,
    alignment: HorizontalAlignment,
) -> Rc<dyn TextBlock> {
    Rc::new(VerticalBlock {
        blocks: vec![stereotype.clone(), label.clone()],
        alignment,
    })
}

/// `UGraphicStencil.create(ug, dim)`.
pub fn stencil_for(ug: &UGraphic, dim: Dimension) -> UGraphic {
    ug.apply_stencil(Rc::new(RectangleStencil(dim.width)), UStroke::simple())
}

/// Size of stereotype over label inside `margin`.
pub fn small_dimension(
    label: &Rc<dyn TextBlock>,
    stereotype: &Rc<dyn TextBlock>,
    margin: Margin,
    sb: &dyn StringBounder,
) -> Dimension {
    let dim_label = label.calculate_dimension(sb);
    let dim_stereo = stereotype.calculate_dimension(sb);
    margin.add_dimension(dim_stereo.merge_tb(dim_label))
}

/// Draws the outline of a symbol of the given size (`width`, `height`).
pub type ShapeFn = fn(&UGraphic, f64, f64, &Fashion);

/// Wraps the drawing context before the label is drawn, used to reshape horizontal lines.
pub type WrapFn = fn(&UGraphic, f64, f64) -> UGraphic;

/// How the common `asSmall` of a symbol is built.
#[derive(Clone, Copy)]
pub struct SmallSpec {
    pub margin: Margin,
    pub stencil: bool,
    /// `None` uses the stereotype alignment given by the caller.
    pub alignment: Option<HorizontalAlignment>,
    pub shape: ShapeFn,
    pub wrap: Option<WrapFn>,
    /// Shift of the text relative to the margin.
    pub text_shift: (f64, f64),
}

/// The `asSmall` shared by most symbols: outline sized from the margin, text at the margin.
pub fn small_standard(
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
    stereo_alignment: HorizontalAlignment,
    spec: SmallSpec,
) -> Rc<dyn TextBlock> {
    let (l, s) = (label.clone(), stereotype.clone());
    fn_block(
        move |sb| small_dimension(&l, &s, spec.margin, sb),
        move |ug| {
            let dim = small_dimension(&label, &stereotype, spec.margin, ug.string_bounder());
            let mut ug = ug.clone();
            if spec.stencil {
                ug = stencil_for(&ug, dim);
            }
            let ug = fashion.apply(&ug);
            (spec.shape)(&ug, dim.width, dim.height, &fashion);
            let tb = merge_tb(
                &stereotype,
                &label,
                spec.alignment.unwrap_or(stereo_alignment),
            );
            let ug = match spec.wrap {
                Some(wrap) => wrap(&ug, dim.width, dim.height),
                None => ug,
            };
            tb.draw(&ug.apply_translate(UTranslate::new(
                spec.margin.x1 + spec.text_shift.0,
                spec.margin.y1 + spec.text_shift.1,
            )));
        },
    )
}

/// Position of the stereotype (centered or at the right) and of the title below it, shared by
/// the `asBig` of rectangle-like symbols. `label_alignment` places the title.
#[allow(clippy::too_many_arguments)]
pub fn draw_big_texts(
    ug: &UGraphic,
    title: &Rc<dyn TextBlock>,
    label_alignment: HorizontalAlignment,
    stereotype: &Rc<dyn TextBlock>,
    width: f64,
    margin: Margin,
    stereo_alignment: HorizontalAlignment,
) {
    let dim_stereo = stereotype.calculate_dimension(ug.string_bounder());
    let (stereo_x, stereo_y) = if stereo_alignment == HorizontalAlignment::Right {
        (width - dim_stereo.width - margin.x1 / 2.0, margin.y1 / 2.0)
    } else {
        ((width - dim_stereo.width) / 2.0, 2.0)
    };
    stereotype.draw(&ug.apply_translate(UTranslate::new(stereo_x, stereo_y)));
    let dim_title = title.calculate_dimension(ug.string_bounder());
    let pos_title = match label_alignment {
        HorizontalAlignment::Left => 3.0,
        HorizontalAlignment::Right => width - dim_title.width - 3.0,
        HorizontalAlignment::Center => (width - dim_title.width) / 2.0,
    };
    title.draw(&ug.apply_translate(UTranslate::new(pos_title, 2.0 + dim_stereo.height)));
}

/// Centered stereotype at `y`, then centered title below it: the `asBig` of many symbols.
pub fn draw_big_centered(
    ug: &UGraphic,
    title: &Rc<dyn TextBlock>,
    stereotype: &Rc<dyn TextBlock>,
    width: f64,
    stereo_y: f64,
    title_y: f64,
) {
    let dim_stereo = stereotype.calculate_dimension(ug.string_bounder());
    let pos_stereo = (width - dim_stereo.width) / 2.0;
    stereotype.draw(&ug.apply_translate(UTranslate::new(pos_stereo, stereo_y)));
    let dim_title = title.calculate_dimension(ug.string_bounder());
    let pos_title = (width - dim_title.width) / 2.0;
    title.draw(&ug.apply_translate(UTranslate::new(pos_title, title_y + dim_stereo.height)));
}

/// Arguments of `USymbol.asBig`.
pub struct BigArgs {
    pub title: Rc<dyn TextBlock>,
    pub label_alignment: HorizontalAlignment,
    pub stereotype: Rc<dyn TextBlock>,
    pub width: f64,
    pub height: f64,
    pub fashion: Fashion,
    pub stereo_alignment: HorizontalAlignment,
}

pub type BigFn = fn(BigArgs) -> Rc<dyn TextBlock>;

/// A symbol described by a margin, an outline and an `asBig` function.
pub struct SimpleSymbol {
    pub snames: Vec<crate::style::SName>,
    pub spec: SmallSpec,
    pub big: BigFn,
    pub supp_height: i32,
    pub supp_width: i32,
}

impl super::USymbol for SimpleSymbol {
    fn snames(&self) -> Vec<crate::style::SName> {
        self.snames.clone()
    }

    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        small_standard(label, stereotype, context, stereo_alignment, self.spec)
    }

    fn as_big(
        &self,
        label: Rc<dyn TextBlock>,
        label_alignment: HorizontalAlignment,
        stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        (self.big)(BigArgs {
            title: label,
            label_alignment,
            stereotype,
            width,
            height,
            fashion: context,
            stereo_alignment,
        })
    }

    fn supp_height_because_of_shape(&self) -> i32 {
        self.supp_height
    }

    fn supp_width_because_of_shape(&self) -> i32 {
        self.supp_width
    }
}
