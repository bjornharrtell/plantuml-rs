//! Symbols drawn as a single outline around the label: rectangle, label, card, collections,
//! storage, hexagon, action, process, stack, component, artifact and file.

use super::common::*;
use super::USymbol;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    UGraphic, ULine, UPath, UPolygon, URectangle, UShape, UTranslate,
};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::style::SName;
use std::rc::Rc;

const CENTER: HorizontalAlignment = HorizontalAlignment::Center;

fn sized(a: &BigArgs) -> Dimension {
    Dimension::new(a.width, a.height)
}

fn rect_with_shadow(width: f64, height: f64, round: f64, shadowing: f64) -> URectangle {
    let mut rect = URectangle::build(width, height).rounded(round);
    rect.delta_shadow = shadowing;
    rect
}

// ---- rectangle ----------------------------------------------------------------------------

const RECTANGLE_MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

fn draw_rect(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let diagonal = fashion.diagonal_corner();
    if diagonal > 0.0 {
        let mut path = UPath::new();
        path.move_to(diagonal, 0.0);
        path.line_to(width - diagonal, 0.0);
        path.line_to(width, diagonal);
        path.line_to(width, height - diagonal);
        path.line_to(width - diagonal, height);
        path.line_to(diagonal, height);
        path.line_to(0.0, height - diagonal);
        path.line_to(0.0, diagonal);
        path.line_to(diagonal, 0.0);
        path.delta_shadow = fashion.delta_shadow();
        ug.draw(path);
    } else {
        ug.draw(rect_with_shadow(
            width,
            height,
            fashion.round_corner(),
            fashion.delta_shadow(),
        ));
    }
}

fn rectangle_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let dim = sized(&a);
        let ug = a.fashion.apply(ug);
        draw_rect(&ug, dim.width, dim.height, &a.fashion);
        draw_big_texts(
            &ug,
            &a.title,
            a.label_alignment,
            &a.stereotype,
            a.width,
            RECTANGLE_MARGIN,
            a.stereo_alignment,
        );
    })
}

/// `USymbolRectangle`.
pub fn rectangle(sname: SName) -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![sname],
        spec: SmallSpec {
            margin: RECTANGLE_MARGIN,
            stencil: true,
            alignment: None,
            shape: draw_rect,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: rectangle_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- label --------------------------------------------------------------------------------

fn draw_nothing(_: &UGraphic, _: f64, _: f64, _: &Fashion) {}

fn label_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_big_texts(
            &ug,
            &a.title,
            a.label_alignment,
            &a.stereotype,
            a.width,
            RECTANGLE_MARGIN,
            a.stereo_alignment,
        );
    })
}

/// `USymbolLabel`.
pub fn label() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Label],
        spec: SmallSpec {
            margin: RECTANGLE_MARGIN,
            stencil: true,
            alignment: None,
            shape: draw_nothing,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: label_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- card ---------------------------------------------------------------------------------

fn draw_card_top(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion, top: f64) {
    ug.draw(rect_with_shadow(
        width,
        height,
        fashion.round_corner(),
        fashion.delta_shadow(),
    ));
    if top != 0.0 {
        ug.apply_translate(UTranslate::dy(top))
            .draw(ULine::hline(width));
    }
}

fn draw_card(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    draw_card_top(ug, width, height, fashion, 0.0);
}

fn card_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        let dim_stereo = a.stereotype.calculate_dimension(ug.string_bounder());
        let dim_title = a.title.calculate_dimension(ug.string_bounder());
        draw_card_top(
            &ug,
            a.width,
            a.height,
            &a.fashion,
            dim_title.height + dim_stereo.height + 4.0,
        );
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 2.0, 2.0);
    })
}

/// `USymbolCard`.
pub fn card() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Card],
        spec: SmallSpec {
            margin: Margin::new(10.0, 10.0, 3.0, 3.0),
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_card,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: card_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- collections --------------------------------------------------------------------------

const COLLECTIONS_DELTA: f64 = 4.0;
const COLLECTIONS_MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

fn draw_collections(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let mut small = URectangle::build(width - COLLECTIONS_DELTA, height - COLLECTIONS_DELTA)
        .rounded(fashion.round_corner());
    small.delta_shadow = fashion.delta_shadow();
    ug.apply_translate(UTranslate::new(COLLECTIONS_DELTA, COLLECTIONS_DELTA))
        .draw(small.clone());
    small.delta_shadow = 0.0;
    ug.draw(small);
}

fn collections_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_collections(&ug, a.width, a.height, &a.fashion);
        draw_big_texts(
            &ug,
            &a.title,
            CENTER,
            &a.stereotype,
            a.width,
            COLLECTIONS_MARGIN,
            a.stereo_alignment,
        );
    })
}

/// `USymbolCollections`.
pub fn collections() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Collections],
        spec: SmallSpec {
            margin: COLLECTIONS_MARGIN,
            stencil: true,
            alignment: None,
            shape: draw_collections,
            wrap: None,
            text_shift: (-COLLECTIONS_DELTA / 2.0, -COLLECTIONS_DELTA / 2.0),
        },
        big: collections_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- storage ------------------------------------------------------------------------------

fn draw_storage(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    ug.draw(rect_with_shadow(
        width,
        height,
        70.0,
        fashion.delta_shadow(),
    ));
}

fn storage_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_storage(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 5.0, 7.0);
    })
}

/// `USymbolStorage`.
pub fn storage() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Storage],
        spec: SmallSpec {
            margin: RECTANGLE_MARGIN,
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_storage,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: storage_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- hexagon ------------------------------------------------------------------------------

const HEXAGON_MARGIN_Y: f64 = 5.0;

fn draw_hexagon(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let dx = width / 8.0;
    let mut shape = UPath::new();
    shape.move_to(0.0, height / 2.0);
    shape.line_to(dx, 0.0);
    shape.line_to(width - dx, 0.0);
    shape.line_to(width, height / 2.0);
    shape.line_to(width - dx, height);
    shape.line_to(dx, height);
    shape.line_to(0.0, height / 2.0);
    shape.delta_shadow = fashion.delta_shadow();
    ug.draw(shape);
}

fn hexagon_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_hexagon(&ug, a.width, a.height, &a.fashion);
        draw_big_texts(
            &ug,
            &a.title,
            a.label_alignment,
            &a.stereotype,
            a.width,
            RECTANGLE_MARGIN,
            a.stereo_alignment,
        );
    })
}

fn hexagon_dimension(
    label: &Rc<dyn TextBlock>,
    stereotype: &Rc<dyn TextBlock>,
    sb: &dyn StringBounder,
) -> Dimension {
    let full = stereotype
        .calculate_dimension(sb)
        .merge_tb(label.calculate_dimension(sb));
    Dimension::new(full.width * 2.0, full.height + 2.0 * HEXAGON_MARGIN_Y)
}

struct Hexagon;

impl USymbol for Hexagon {
    fn snames(&self) -> Vec<SName> {
        vec![SName::Hexagon]
    }

    /// The outline itself is drawn by the caller; only the text is placed here.
    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        _context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let (l, s) = (label.clone(), stereotype.clone());
        fn_block(
            move |sb| hexagon_dimension(&l, &s, sb),
            move |ug| {
                let dim = hexagon_dimension(&label, &stereotype, ug.string_bounder());
                let tb = merge_tb(&stereotype, &label, stereo_alignment);
                tb.draw(&ug.apply_translate(UTranslate::new(dim.width / 4.0, HEXAGON_MARGIN_Y)));
            },
        )
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
        hexagon_big(BigArgs {
            title: label,
            label_alignment,
            stereotype,
            width,
            height,
            fashion: context,
            stereo_alignment,
        })
    }
}

/// `USymbolHexagon`.
pub fn hexagon() -> Rc<dyn USymbol> {
    Rc::new(Hexagon)
}

// ---- action and process -------------------------------------------------------------------

fn draw_action(ug: &UGraphic, width: f64, height: f64, _: &Fashion) {
    let mut shape = UPolygon::default();
    shape.add_point(0.0, 0.0);
    shape.add_point(width - 10.0, 0.0);
    shape.add_point(width, height / 2.0);
    shape.add_point(width - 10.0, height);
    shape.add_point(0.0, height);
    ug.draw(shape);
}

fn draw_process(ug: &UGraphic, width: f64, height: f64, _: &Fashion) {
    let mut shape = UPolygon::default();
    shape.add_point(0.0, 0.0);
    shape.add_point(width - 10.0, 0.0);
    shape.add_point(width, height / 2.0);
    shape.add_point(width - 10.0, height);
    shape.add_point(0.0, height);
    shape.add_point(10.0, height / 2.0);
    ug.draw(shape);
}

/// Title at the top and stereotype below, shifted by 4 (`USymbolAction.asBig`).
fn arrow_big(a: BigArgs, draw: fn(&UGraphic, f64, f64, &Fashion)) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        let sb = ug.string_bounder();
        let dim_title = a.title.calculate_dimension(sb);
        draw(&ug, a.width, a.height, &a.fashion);
        let pos_title = (a.width - dim_title.width) / 2.0;
        a.title
            .draw(&ug.apply_translate(UTranslate::new(pos_title, 2.0)));
        let dim_stereo = a.stereotype.calculate_dimension(sb);
        let pos_stereo = (a.width - dim_stereo.width) / 2.0;
        let h_title = if dim_title.width == 0.0 {
            10.0
        } else {
            dim_title.height
        };
        a.stereotype
            .draw(&ug.apply_translate(UTranslate::new(4.0 + pos_stereo, 2.0 + h_title)));
    })
}

fn action_big(a: BigArgs) -> Rc<dyn TextBlock> {
    arrow_big(a, draw_action)
}

fn process_big(a: BigArgs) -> Rc<dyn TextBlock> {
    arrow_big(a, draw_process)
}

/// `USymbolAction`.
pub fn action(sname: SName) -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![sname],
        spec: SmallSpec {
            margin: Margin::new(10.0, 20.0, 10.0, 10.0),
            stencil: true,
            alignment: None,
            shape: draw_action,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: action_big,
        supp_height: 0,
        supp_width: 0,
    })
}

/// `USymbolProcess`.
pub fn process(sname: SName) -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![sname],
        spec: SmallSpec {
            margin: Margin::new(20.0, 20.0, 10.0, 10.0),
            stencil: true,
            alignment: None,
            shape: draw_process,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: process_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- stack --------------------------------------------------------------------------------

fn draw_stack(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let border = 15.0;
    let round = fashion.round_corner();
    let rect = URectangle::build(width - 2.0 * border, height).rounded(round);
    ug.apply_color(HColor::TRANSPARENT)
        .apply_translate(UTranslate::dx(border))
        .draw(rect);
    let mut path = UPath::new();
    if round == 0.0 {
        path.move_to(0.0, 0.0);
        path.line_to(border, 0.0);
        path.line_to(border, height);
        path.line_to(width - border, height);
        path.line_to(width - border, 0.0);
        path.line_to(width, 0.0);
    } else {
        let r = round / 2.0;
        path.move_to(0.0, 0.0);
        path.line_to(border - r, 0.0);
        path.arc_to(r, r, 0.0, false, true, border, r);
        path.line_to(border, height - r);
        path.arc_to(r, r, 0.0, false, false, border + r, height);
        path.line_to(width - border - r, height);
        path.arc_to(r, r, 0.0, false, false, width - border, height - r);
        path.line_to(width - border, r);
        path.arc_to(r, r, 0.0, false, true, width - border + r, 0.0);
        path.line_to(width, 0.0);
    }
    path.delta_shadow = fashion.delta_shadow();
    ug.apply_back_color(HColor::TRANSPARENT).draw(path);
}

fn stack_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_stack(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 13.0, 13.0);
    })
}

/// `USymbolStack`.
pub fn stack() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Stack],
        spec: SmallSpec {
            margin: Margin::new(25.0, 25.0, 10.0, 10.0),
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_stack,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: stack_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- components ---------------------------------------------------------------------------

fn draw_component1(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    ug.draw(rect_with_shadow(
        width,
        height,
        fashion.round_corner(),
        fashion.delta_shadow(),
    ));
    let small = URectangle::build(10.0, 5.0);
    ug.apply_translate(UTranslate::new(-5.0, 5.0))
        .draw(small.clone());
    ug.apply_translate(UTranslate::new(-5.0, height - 10.0))
        .draw(small);
}

fn draw_component2(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let form = rect_with_shadow(
        width,
        height,
        fashion.round_corner(),
        fashion.delta_shadow(),
    );
    let small = URectangle::build(15.0, 10.0);
    let tiny = URectangle::build(4.0, 2.0);
    ug.draw(form);
    ug.apply_translate(UTranslate::new(width - 20.0, 5.0))
        .draw(small);
    ug.apply_translate(UTranslate::new(width - 22.0, 7.0))
        .draw(tiny.clone());
    ug.apply_translate(UTranslate::new(width - 22.0, 11.0))
        .draw(tiny);
}

fn component2_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_component2(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 13.0, 13.0);
    })
}

/// `USymbolComponent1`; its big form is the one of `USymbolComponent2`.
pub fn component1() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Component],
        spec: SmallSpec {
            margin: RECTANGLE_MARGIN,
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_component1,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: component2_big,
        supp_height: 0,
        supp_width: 0,
    })
}

/// `USymbolComponent2`.
pub fn component2() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Component],
        spec: SmallSpec {
            margin: Margin::new(15.0, 25.0, 20.0, 10.0),
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_component2,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: component2_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- artifact -----------------------------------------------------------------------------

fn draw_artifact(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    ug.draw(rect_with_shadow(
        width,
        height,
        fashion.round_corner(),
        fashion.delta_shadow(),
    ));
    let height_symbol = 14.0;
    let width_symbol = 12.0;
    let corner = 6.0;
    let mut polygon = UPolygon::default();
    polygon.add_point(0.0, 0.0);
    polygon.add_point(0.0, height_symbol);
    polygon.add_point(width_symbol, height_symbol);
    polygon.add_point(width_symbol, corner);
    polygon.add_point(width_symbol - corner, 0.0);
    polygon.add_point(0.0, 0.0);
    let x = width - width_symbol - 5.0;
    let y = 5.0;
    ug.apply_translate(UTranslate::new(x, y)).draw(polygon);
    ug.apply_translate(UTranslate::new(x + width_symbol - corner, y))
        .draw(ULine::vline(corner));
    ug.apply_translate(UTranslate::new(x + width_symbol, y + corner))
        .draw(ULine::hline(-corner));
}

fn artifact_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_artifact(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 2.0, 2.0);
    })
}

/// `USymbolArtifact`.
pub fn artifact() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::Artifact],
        spec: SmallSpec {
            margin: Margin::new(10.0, 20.0, 13.0, 10.0),
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_artifact,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: artifact_big,
        supp_height: 0,
        supp_width: 0,
    })
}

// ---- file ---------------------------------------------------------------------------------

fn draw_file(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    let corner = 10.0;
    let round = fashion.round_corner();
    let shadow = fashion.delta_shadow();
    if round == 0.0 {
        let mut polygon = UPolygon::default();
        polygon.add_point(0.0, 0.0);
        polygon.add_point(0.0, height);
        polygon.add_point(width, height);
        polygon.add_point(width, corner);
        polygon.add_point(width - corner, 0.0);
        polygon.add_point(0.0, 0.0);
        polygon.delta_shadow = shadow;
        ug.draw(UShape::Polygon(polygon));
    } else {
        let r = round / 2.0;
        let mut path = UPath::new();
        path.move_to(0.0, r);
        path.line_to(0.0, height - r);
        path.arc_to(r, r, 0.0, false, false, r, height);
        path.line_to(width - r, height);
        path.arc_to(r, r, 0.0, false, false, width, height - r);
        path.line_to(width, corner);
        path.line_to(width - corner, 0.0);
        path.line_to(r, 0.0);
        path.arc_to(r, r, 0.0, false, false, 0.0, r);
        path.delta_shadow = shadow;
        ug.draw(path);
    }
    let mut path = UPath::new();
    path.move_to(width - corner, 0.0);
    if round == 0.0 {
        path.line_to(width - corner, corner);
    } else {
        let r = round / 2.0;
        path.line_to(width - corner, corner - r);
        path.arc_to(r, r, 0.0, false, false, width - corner + r, corner);
    }
    path.line_to(width, corner);
    ug.draw(path);
}

fn file_big(a: BigArgs) -> Rc<dyn TextBlock> {
    fn_block_sized(a.width, a.height, move |ug| {
        let ug = a.fashion.apply(ug);
        draw_file(&ug, a.width, a.height, &a.fashion);
        draw_big_centered(&ug, &a.title, &a.stereotype, a.width, 2.0, 2.0);
    })
}

/// `USymbolFile`.
pub fn file() -> Rc<dyn USymbol> {
    Rc::new(SimpleSymbol {
        snames: vec![SName::File],
        spec: SmallSpec {
            margin: RECTANGLE_MARGIN,
            stencil: true,
            alignment: Some(CENTER),
            shape: draw_file,
            wrap: None,
            text_shift: (0.0, 0.0),
        },
        big: file_big,
        supp_height: 0,
        supp_width: 0,
    })
}
