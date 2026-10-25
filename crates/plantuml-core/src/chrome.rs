//! Title, header, footer, legend, caption, main frame and warnings around a diagram
//! (`DiagramChromeFactory`).

use crate::diagram::DiagramBase;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::DisplayPositioned;
use crate::klimt::creole::LineBreakStrategy;
use crate::klimt::drawing::ugraphic::{
    find_limits, UGraphic, UPath, URectangle, UStroke, UText, UTranslate,
};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder, UFont};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{queries, Margins, SName, StyleQuery};
use crate::svek::decorate::{DecorateEntityImage, Group};
use std::rc::Rc;

fn group_of(class: &str, location: Option<crate::text::LineLocation>) -> Group {
    let mut g: Group = vec![("class", class.to_string())];
    if let Some(l) = location {
        g.push(("data-source-line", l.position.to_string()));
    }
    g
}

/// Wraps `raw` with everything the diagram declares around it.
pub fn create(
    raw: Rc<dyn TextBlock>,
    base: &DiagramBase,
    skin: &Rc<SkinParam>,
    title: &DisplayPositioned,
) -> Rc<dyn TextBlock> {
    let mut result = raw;
    result = add_warnings(result, &base.warnings);
    result = decorate_with_frame(result, base, skin);
    result = add_legend(result, base, skin);
    result = add_title(result, title, skin);
    result = add_caption(result, base, skin);
    add_header_and_footer(result, base, skin)
}

const LINE_SPACING: f64 = 10.0;
const CORNER_RADIUS: f64 = 5.0;

struct WarningBanner {
    warnings: Vec<Vec<String>>,
}

impl WarningBanner {
    fn font() -> FontConfiguration {
        FontConfiguration::black_blue_true(UFont::monospace(10))
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let mut width: f64 = 0.0;
        let mut height = 0.0;
        for w in &self.warnings {
            for s in w {
                let d = sb.calculate_dimension(&Self::font().font(), s);
                width = width.max(d.width);
                height += d.height;
            }
            height += LINE_SPACING;
        }
        if !self.warnings.is_empty() {
            height -= LINE_SPACING;
        }
        Dimension::new(width + 20.0, height + 10.0)
    }

    fn draw(&self, ug: &UGraphic, force_width: f64) {
        let sb = ug.string_bounder();
        let dim = self.calculate_dimension(sb);
        let effective_width = dim.width.max(force_width);
        let back = HColor::parse_or_white("ffffcc").with_dark(HColor::parse_or_white("774400"));
        let border = HColor::parse_or_white("ffdd88").with_dark(HColor::parse_or_white("aa5500"));
        let rect =
            URectangle::build(effective_width - 10.0, dim.height - 5.0).rounded(CORNER_RADIUS);
        ug.apply_back_color(back)
            .apply_color(border)
            .apply_stroke(UStroke::with_thickness(3.0))
            .apply_translate(UTranslate::new(3.0, 3.0))
            .draw(rect);
        let mut ug_text = ug
            .apply_color(HColor::BLACK)
            .apply_translate(UTranslate::new(10.0, 2.0));
        for w in &self.warnings {
            for s in w {
                let text = UText::new(s.clone(), Self::font());
                let height = sb.calculate_dimension(&Self::font().font(), s).height;
                ug_text = ug_text.apply_translate(UTranslate::dy(height));
                ug_text.draw(text);
            }
            ug_text = ug_text.apply_translate(UTranslate::dy(LINE_SPACING));
        }
    }
}

struct WithWarnings {
    banner: WarningBanner,
    original: Rc<dyn TextBlock>,
}

impl TextBlock for WithWarnings {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let banner = self.banner.calculate_dimension(sb);
        let original = self.original.calculate_dimension(sb);
        Dimension::new(
            banner.width.max(original.width),
            banner.height + original.height,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let total_width = self.calculate_dimension(sb).width;
        self.banner.draw(ug, total_width);
        let banner_height = self.banner.calculate_dimension(sb).height;
        self.original
            .draw(&ug.apply_translate(UTranslate::dy(banner_height)));
    }

    fn backcolor(&self) -> Option<HColor> {
        self.original.backcolor()
    }
}

fn add_warnings(original: Rc<dyn TextBlock>, warnings: &[Vec<String>]) -> Rc<dyn TextBlock> {
    if warnings.is_empty() {
        return original;
    }
    Rc::new(WithWarnings {
        banner: WarningBanner {
            warnings: warnings.to_vec(),
        },
        original,
    })
}

/// Rectangle with a titled tab around the whole diagram (`BigFrame`).
struct BigFrame {
    title: Rc<dyn TextBlock>,
    original: Rc<dyn TextBlock>,
    padding: Margins,
    symbol_context: Fashion,
}

impl BigFrame {
    fn effective_padding(&self, sb: &dyn StringBounder) -> Margins {
        let dim = self.title.calculate_dimension(sb);
        Margins {
            top: self.padding.top + dim.height + 10.0,
            ..self.padding
        }
    }

    fn original_limits(&self, sb: &dyn StringBounder) -> crate::klimt::geom::MinMax {
        let original = self.original.clone();
        find_limits(sb.to_rc(), false, move |ug| original.draw(ug))
    }
}

impl TextBlock for BigFrame {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let dim_title = self.title.calculate_dimension(sb);
        let padding = self.effective_padding(sb);
        let limits = self.original_limits(sb);
        let ww = if limits.min_x >= 0.0 {
            limits.max_x
        } else {
            limits.width()
        };
        let hh = if limits.min_y >= 0.0 {
            limits.max_y
        } else {
            limits.height()
        };
        Dimension::new(
            padding.left + (ww + 12.0).max(dim_title.width + 10.0) + padding.right,
            padding.top + dim_title.height + hh + padding.bottom,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let dim = self.calculate_dimension(sb);
        let ug = self.symbol_context.apply(ug);
        let dim_title = self.title.calculate_dimension(sb);
        let width_full = dim.width;
        let mut rect =
            URectangle::build(width_full, dim.height).rounded(self.symbol_context.round_corner());
        rect.delta_shadow = self.symbol_context.delta_shadow();
        ug.draw(rect);
        let (text_width, corner_size) = if dim_title.width == 0.0 {
            (width_full / 3.0, 7.0)
        } else {
            (dim_title.width + 10.0, 10.0)
        };
        let text_height = if dim_title.width == 0.0 {
            12.0
        } else {
            dim_title.height + 3.0
        };
        let mut line = UPath::new();
        line.move_to(text_width, 0.0);
        line.line_to(text_width, text_height - corner_size);
        line.line_to(text_width - corner_size, text_height);
        line.line_to(0.0, text_height);
        ug.apply_back_color(HColor::TRANSPARENT).draw(line);
        let width_title = self.title.calculate_dimension(sb).width;
        let ug_title = ug.apply_translate(UTranslate::new(3.0, 1.0));
        if width_full - width_title < 25.0 || !ug.is_finding_limits() {
            self.title.draw(&ug_title);
        }
    }
}

struct Framed {
    frame: Rc<dyn TextBlock>,
    original: Rc<dyn TextBlock>,
    title: Rc<dyn TextBlock>,
    margin: Margins,
    padding: Margins,
    back: Option<HColor>,
}

impl Framed {
    fn computed_padding(&self, sb: &dyn StringBounder) -> Margins {
        let dim = self.title.calculate_dimension(sb);
        Margins {
            top: self.padding.top + dim.height + 10.0,
            ..self.padding
        }
    }

    fn delta(&self, sb: &dyn StringBounder) -> UTranslate {
        let original = self.original.clone();
        let m = find_limits(sb.to_rc(), false, move |ug| original.draw(ug));
        UTranslate::new(
            if m.min_x < 0.0 { -m.min_x } else { 0.0 },
            if m.min_y < 0.0 { -m.min_y } else { 0.0 },
        )
    }
}

impl TextBlock for Framed {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.frame.calculate_dimension(sb);
        Dimension::new(
            self.margin.left + d.width + self.margin.right,
            self.margin.top + d.height + self.margin.bottom,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let padding = self.computed_padding(sb);
        let delta = self.delta(sb);
        self.frame
            .draw(&ug.apply_translate(UTranslate::new(self.margin.left, self.margin.top)));
        self.original.draw(&ug.apply_translate(UTranslate::new(
            self.margin.left + padding.left + delta.dx,
            self.margin.top + padding.top + delta.dy,
        )));
    }

    fn backcolor(&self) -> Option<HColor> {
        self.back.clone()
    }
}

fn decorate_with_frame(
    original: Rc<dyn TextBlock>,
    base: &DiagramBase,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    let main_frame = &base.main_frame.display;
    if main_frame.is_null() {
        return original;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::DOCUMENT_MAINFRAME)
        .expect("mainframe style");
    let title = main_frame.create_simple_block(
        &style.font_configuration(None),
        HorizontalAlignment::Center,
        &skin.skin_simple(),
    );
    let symbol_context = style.symbol_context(None);
    let frame: Rc<dyn TextBlock> = Rc::new(BigFrame {
        title: title.clone(),
        original: original.clone(),
        padding: style.padding(),
        symbol_context: symbol_context.clone(),
    });
    Rc::new(Framed {
        frame,
        original,
        title,
        margin: style.margin(),
        padding: style.padding(),
        back: symbol_context.back_color().cloned(),
    })
}

fn add_legend(
    original: Rc<dyn TextBlock>,
    base: &DiagramBase,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    let legend = &base.legend;
    if legend.is_null() {
        return original;
    }
    let query: StyleQuery = queries::DOCUMENT
        .clone()
        .add(SName::Legend)
        .add(base.diagram_type.style_name());
    let style = skin
        .style_builder()
        .merged_style(&query)
        .expect("legend style");
    let block = style.create_text_block_bordered(&legend.display, skin, &style.wrap_width());
    DecorateEntityImage::add(
        group_of("legend", legend.location),
        original,
        block,
        legend
            .horizontal_alignment
            .unwrap_or(HorizontalAlignment::Center),
        legend
            .vertical_alignment
            .unwrap_or(crate::klimt::geom::VerticalAlignment::Bottom),
    )
}

fn add_title(
    original: Rc<dyn TextBlock>,
    title: &DisplayPositioned,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    if title.is_null() {
        return original;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::DOCUMENT_TITLE)
        .expect("title style");
    let block = style.create_text_block_bordered(&title.display, skin, &LineBreakStrategy::NONE);
    DecorateEntityImage::add_top(
        group_of("title", title.location),
        original,
        block,
        HorizontalAlignment::Center,
    )
}

fn add_caption(
    original: Rc<dyn TextBlock>,
    base: &DiagramBase,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    let caption = &base.caption;
    if caption.is_null() {
        return original;
    }
    let style = skin
        .style_builder()
        .merged_style(&queries::DOCUMENT_CAPTION)
        .expect("caption style");
    let block = style.create_text_block_bordered(&caption.display, skin, &LineBreakStrategy::NONE);
    DecorateEntityImage::add_bottom(
        group_of("caption", caption.location),
        original,
        block,
        HorizontalAlignment::Center,
    )
}

fn ribbon(
    positioned: &DisplayPositioned,
    skin: &Rc<SkinParam>,
    query: &StyleQuery,
) -> Option<Rc<dyn TextBlock>> {
    if positioned.display.is_null() || positioned.display.size() == 0 {
        return None;
    }
    let style = skin.style_builder().merged_style(query)?;
    Some(style.create_text_block_bordered(&positioned.display, skin, &LineBreakStrategy::NONE))
}

fn add_header_and_footer(
    original: Rc<dyn TextBlock>,
    base: &DiagramBase,
    skin: &Rc<SkinParam>,
) -> Rc<dyn TextBlock> {
    let (header, footer) = (&base.header, &base.footer);
    if footer.is_null() && header.is_null() {
        return original;
    }
    let text_header = if header.is_null() {
        None
    } else {
        ribbon(header, skin, &queries::DOCUMENT_HEADER)
    };
    let text_footer = if footer.is_null() {
        None
    } else {
        ribbon(footer, skin, &queries::DOCUMENT_FOOTER)
    };
    DecorateEntityImage::add_top_and_bottom(
        original,
        group_of("header", header.location),
        text_header,
        header
            .horizontal_alignment
            .unwrap_or(HorizontalAlignment::Center),
        group_of("footer", footer.location),
        text_footer,
        footer
            .horizontal_alignment
            .unwrap_or(HorizontalAlignment::Center),
    )
}
