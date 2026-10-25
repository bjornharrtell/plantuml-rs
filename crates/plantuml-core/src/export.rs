//! Turning a laid out block into an SVG document (`TextBlockExporter`).

use crate::diagram::{DiagramType, Scale};
use crate::klimt::color::HColor;
use crate::klimt::drawing::svg::{LengthAdjust, SvgGraphics, SvgOption};
use crate::klimt::drawing::ugraphic::{UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::Margins;
use std::cell::RefCell;
use std::rc::Rc;

/// Settings of one export (`TextBlockExporter.Builder`).
pub struct ExportOptions {
    pub back_color: HColor,
    pub margin: Margins,
    pub seed: i64,
    pub scale: Option<Scale>,
    pub dpi: i32,
    pub diagram_type: Option<DiagramType>,
    pub preserve_aspect_ratio: String,
    pub dimension_style: bool,
    pub length_adjust: LengthAdjust,
}

impl ExportOptions {
    pub fn new(skin: &SkinParam, diagram_type: DiagramType, margin: Margins) -> Self {
        Self {
            back_color: HColor::WHITE,
            margin,
            seed: 42,
            scale: None,
            dpi: skin.dpi(),
            diagram_type: Some(diagram_type),
            preserve_aspect_ratio: skin.preserve_aspect_ratio(),
            dimension_style: skin.svg_dimension_style(),
            length_adjust: match skin.get_value("lengthAdjust") {
                Some(v) if v.eq_ignore_ascii_case("spacingAndGlyphs") => {
                    LengthAdjust::SpacingAndGlyphs
                }
                Some(v) if v.eq_ignore_ascii_case("none") => LengthAdjust::None,
                _ => LengthAdjust::Spacing,
            },
        }
    }
}

/// Final size of the image: block plus margins.
pub fn final_dimension(
    block: &dyn TextBlock,
    sb: &dyn StringBounder,
    margin: &Margins,
) -> Dimension {
    let d = block.calculate_dimension(sb);
    Dimension::new(
        d.width + margin.left + margin.right,
        d.height + margin.top + margin.bottom,
    )
}

/// Draws `block` into an SVG document.
pub fn export_svg(
    block: &dyn TextBlock,
    sb: Rc<dyn StringBounder>,
    opts: &ExportOptions,
) -> String {
    let dim = final_dimension(block, sb.as_ref(), &opts.margin);
    let scale_factor =
        opts.scale.map_or(1.0, |s| s.factor(dim.width, dim.height)) * f64::from(opts.dpi) / 96.0;
    assert!(scale_factor > 0.0, "bad scale factor");
    let mut option = SvgOption {
        scale: scale_factor,
        min_width: dim.width,
        min_height: dim.height,
        backcolor: Some(opts.back_color.to_svg()),
        length_adjust: opts.length_adjust,
        preserve_aspect_ratio: opts.preserve_aspect_ratio.clone(),
        dimension_style: opts.dimension_style,
        ..SvgOption::default()
    };
    if let Some(t) = opts.diagram_type {
        option
            .root_attributes
            .push(("data-diagram-type".into(), t.svg_name().into()));
    }
    let svg = Rc::new(RefCell::new(SvgGraphics::new(opts.seed, option)));
    {
        let ug = UGraphic::new(svg.clone(), sb)
            .with_default_background(opts.back_color.clone())
            .apply_translate(UTranslate::new(opts.margin.left, opts.margin.top));
        block.draw(&ug);
    }
    let svg = Rc::try_unwrap(svg)
        .unwrap_or_else(|_| panic!("drawing context still in use"))
        .into_inner();
    svg.to_svg()
}
