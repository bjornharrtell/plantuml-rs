//! Folder-like symbols with a tab for the title: folder, package and frame.

use super::common::*;
use super::USymbol;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPath, UPolygon, URectangle, UTranslate};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::Dimension;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{EmptyBlock, TextBlock};
use crate::style::SName;
use std::rc::Rc;

const MARGIN_TITLE_X1: f64 = 3.0;
const MARGIN_TITLE_X2: f64 = 3.0;
const MARGIN_TITLE_X3: f64 = 7.0;
const MARGIN_TITLE_Y1: f64 = 3.0;
const MARGIN_TITLE_Y2: f64 = 3.0;

const FOLDER_MARGIN: Margin = Margin::new(10.0, 20.0, 13.0, 10.0);

/// Symbol of folders (`USymbolFolder`); a package is a folder that shows its title.
pub struct Folder {
    sname: SName,
    show_title: bool,
}

/// `USymbols.FOLDER` and `USymbols.PACKAGE`.
pub fn folder(sname: SName, show_title: bool) -> Rc<dyn USymbol> {
    Rc::new(Folder { sname, show_title })
}

fn folder_width_title(width: f64, dim_title: Dimension) -> f64 {
    if dim_title.width == 0.0 {
        30.0_f64.max(width / 4.0)
    } else {
        dim_title.width + MARGIN_TITLE_X1 + MARGIN_TITLE_X2
    }
}

fn folder_height_title(dim_title: Dimension) -> f64 {
    if dim_title.width == 0.0 {
        10.0
    } else {
        dim_title.height + MARGIN_TITLE_Y1 + MARGIN_TITLE_Y2
    }
}

fn draw_folder(ug: &UGraphic, width: f64, height: f64, dim_name: Dimension, fashion: &Fashion) {
    let round = fashion.round_corner();
    let wtitle = folder_width_title(width, dim_name);
    let htitle = folder_height_title(dim_name);
    let shadow = fashion.delta_shadow();
    if round == 0.0 {
        let mut poly = UPolygon::default();
        poly.add_point(0.0, 0.0);
        poly.add_point(wtitle, 0.0);
        poly.add_point(wtitle + MARGIN_TITLE_X3, htitle);
        poly.add_point(width, htitle);
        poly.add_point(width, height);
        poly.add_point(0.0, height);
        poly.add_point(0.0, 0.0);
        poly.delta_shadow = shadow;
        ug.draw(poly);
    } else {
        let r = round / 2.0;
        let mut path = UPath::new();
        path.move_to(r, 0.0);
        path.line_to(wtitle - r, 0.0);
        path.arc_to(r * 1.5, r * 1.5, 0.0, false, true, wtitle, r);
        path.line_to(wtitle + MARGIN_TITLE_X3, htitle);
        path.line_to(width - r, htitle);
        path.arc_to(r, r, 0.0, false, true, width, htitle + r);
        path.line_to(width, height - r);
        path.arc_to(r, r, 0.0, false, true, width - r, height);
        path.line_to(r, height);
        path.arc_to(r, r, 0.0, false, true, 0.0, height - r);
        path.line_to(0.0, r);
        path.arc_to(r, r, 0.0, false, true, r, 0.0);
        path.delta_shadow = shadow;
        ug.draw(path);
    }
    ug.apply_translate(UTranslate::dy(htitle))
        .draw(ULine::hline(wtitle + MARGIN_TITLE_X3));
}

impl Folder {
    fn dim_title(
        show_title: bool,
        title: &Rc<dyn TextBlock>,
        sb: &dyn crate::klimt::font::StringBounder,
    ) -> Dimension {
        if show_title {
            title.calculate_dimension(sb)
        } else {
            Dimension::new(40.0, 15.0)
        }
    }
}

impl USymbol for Folder {
    fn snames(&self) -> Vec<SName> {
        vec![self.sname]
    }

    fn as_small(
        &self,
        name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let show_title = self.show_title;
        let title = name.unwrap_or_else(|| {
            Rc::new(EmptyBlock {
                width: 0.0,
                height: 0.0,
            })
        });
        let (t, l, s) = (title.clone(), label.clone(), stereotype.clone());
        fn_block(
            move |sb| {
                let dim_name = Folder::dim_title(show_title, &t, sb);
                let dim_label = l.calculate_dimension(sb);
                let dim_stereo = s.calculate_dimension(sb);
                FOLDER_MARGIN.add_dimension(dim_name.merge_tb(dim_stereo).merge_tb(dim_label))
            },
            move |ug| {
                let sb = ug.string_bounder();
                let dim_name = Folder::dim_title(show_title, &title, sb);
                let dim = FOLDER_MARGIN.add_dimension(
                    dim_name
                        .merge_tb(stereotype.calculate_dimension(sb))
                        .merge_tb(label.calculate_dimension(sb)),
                );
                let ug = stencil_for(ug, dim);
                let ug = context.apply(&ug);
                draw_folder(&ug, dim.width, dim.height, dim_name, &context);
                let tb = merge_tb(&stereotype, &label, HorizontalAlignment::Center);
                if show_title {
                    title.draw(&ug.apply_translate(UTranslate::new(4.0, 3.0)));
                }
                tb.draw(&ug.apply_translate(UTranslate::new(
                    FOLDER_MARGIN.x1,
                    FOLDER_MARGIN.y1 + dim_name.height,
                )));
            },
        )
    }

    fn as_big(
        &self,
        label: Rc<dyn TextBlock>,
        _label_alignment: HorizontalAlignment,
        stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        fn_block_sized(width, height, move |ug| {
            let sb = ug.string_bounder();
            let ug = context.apply(ug);
            let dim_title = label.calculate_dimension(sb);
            draw_folder(&ug, width, height, dim_title, &context);
            label.draw(&ug.apply_translate(UTranslate::new(4.0, 2.0)));
            let dim_stereo = stereotype.calculate_dimension(sb);
            let pos_stereo = (width - dim_stereo.width) / 2.0;
            stereotype.draw(&ug.apply_translate(UTranslate::new(
                4.0 + pos_stereo,
                2.0 + folder_height_title(dim_title),
            )));
        })
    }
}

// ---- frame --------------------------------------------------------------------------------

const FRAME_MARGIN: Margin = Margin::new(15.0, 25.0, 20.0, 10.0);

/// Symbol of frames (`USymbolFrame`).
pub struct Frame {
    sname: SName,
}

/// `USymbols.FRAME`, `USymbols.GROUP` and `USymbols.PARTITION`.
pub fn frame(sname: SName) -> Rc<dyn USymbol> {
    Rc::new(Frame { sname })
}

fn frame_ypos(dim_title: Dimension) -> f64 {
    if dim_title.width == 0.0 {
        12.0
    } else {
        dim_title.height + 3.0
    }
}

fn draw_frame(ug: &UGraphic, width: f64, height: f64, dim_title: Dimension, fashion: &Fashion) {
    let mut rect = URectangle::build(width, height).rounded(fashion.round_corner());
    rect.delta_shadow = fashion.delta_shadow();
    ug.draw(rect);
    let (text_width, corner) = if dim_title.width == 0.0 {
        (width / 3.0, 7.0)
    } else {
        (dim_title.width + 10.0, 10.0)
    };
    let text_height = frame_ypos(dim_title);
    let mut line = UPath::new();
    line.move_to(text_width, 0.0);
    line.line_to(text_width, text_height - corner);
    line.line_to(text_width - corner, text_height);
    line.line_to(0.0, text_height);
    ug.apply_back_color(HColor::TRANSPARENT).draw(line);
}

fn draw_frame_shape(ug: &UGraphic, width: f64, height: f64, fashion: &Fashion) {
    draw_frame(ug, width, height, Dimension::new(0.0, 0.0), fashion);
}

impl USymbol for Frame {
    fn snames(&self) -> Vec<SName> {
        vec![self.sname]
    }

    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        small_standard(
            label,
            stereotype,
            context,
            stereo_alignment,
            SmallSpec {
                margin: FRAME_MARGIN,
                stencil: true,
                alignment: Some(HorizontalAlignment::Center),
                shape: draw_frame_shape,
                wrap: None,
                text_shift: (0.0, 0.0),
            },
        )
    }

    fn as_big(
        &self,
        label: Rc<dyn TextBlock>,
        _label_alignment: HorizontalAlignment,
        stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        fn_block_sized(width, height, move |ug| {
            let sb = ug.string_bounder();
            let ug = context.apply(ug);
            let dim_title = label.calculate_dimension(sb);
            draw_frame(&ug, width, height, dim_title, &context);
            // `SpecialText` only differs from a plain title when compressing.
            label.draw(&ug.apply_translate(UTranslate::new(3.0, 1.0)));
            let dim_stereo = stereotype.calculate_dimension(sb);
            let pos_stereo = (width - dim_stereo.width) / 2.0;
            stereotype.draw(&ug.apply_translate(UTranslate::new(
                4.0 + pos_stereo,
                2.0 + frame_ypos(dim_title),
            )));
        })
    }
}
