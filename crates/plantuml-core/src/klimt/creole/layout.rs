//! Placement of atoms into lines and blocks (`Sea`, `Fission`, `SheetBlock1`, `SheetBlock2`).

use super::atoms::{Atom, Neutron, NeutronType};
use super::parser::{create_sheet, Sheet};
use super::{CreoleMode, LineBreakStrategy, SkinSimple};
use crate::klimt::drawing::ugraphic::{UGraphic, UStroke, UTranslate};
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, MinMax};
use crate::klimt::shape::{Stencil, TextBlock};
use crate::style::Margins;
use std::cell::RefCell;
use std::rc::Rc;

/// A line as laid out: header, atoms and optional own alignment.
#[derive(Debug, Clone)]
struct LineStripe {
    atoms: Vec<Atom>,
    /// `None` uses the sheet alignment (stripes created by line breaking).
    align: Option<HorizontalAlignment>,
}

/// Position of an atom inside the block.
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f64,
    y: f64,
    dim: Dimension,
}

impl Position {
    fn max_y(&self) -> f64 {
        self.y + self.dim.height
    }
}

struct Layout {
    stripes: Vec<LineStripe>,
    /// Positions aligned with `stripes[i].atoms`.
    positions: Vec<Vec<Position>>,
    min_max: MinMax,
}

/// Breaks long lines at neutron boundaries (`Fission`).
fn split_stripe(
    atoms: &[Atom],
    has_header: bool,
    align: HorizontalAlignment,
    max_width: &LineBreakStrategy,
    sb: &dyn StringBounder,
) -> Vec<LineStripe> {
    let value = max_width.max_width().abs();
    if value == 0.0 {
        return vec![LineStripe {
            atoms: atoms.to_vec(),
            align: Some(align),
        }];
    }
    struct Line {
        remove_initial_spaces: bool,
        header: Option<Atom>,
        neutrons: Vec<Neutron>,
        width: f64,
    }
    impl Line {
        fn new(remove_initial_spaces: bool, header: Option<Atom>, sb: &dyn StringBounder) -> Self {
            let width = header.as_ref().map_or(0.0, |h| h.dimension(sb).width);
            Self {
                remove_initial_spaces,
                header,
                neutrons: Vec::new(),
                width,
            }
        }
        fn last_zwsp(&self) -> Option<usize> {
            self.neutrons
                .iter()
                .rposition(|n| n.kind == NeutronType::ZwspSeparator)
        }
        fn shorten(&mut self) -> Vec<Neutron> {
            match self.last_zwsp() {
                None => Vec::new(),
                Some(i) => self.neutrons.split_off(i),
            }
        }
        fn is_white(&self) -> bool {
            self.neutrons
                .iter()
                .all(|n| matches!(n.kind, NeutronType::ZwspSeparator | NeutronType::Whitespace))
        }
        fn remove_final_spaces(&mut self) {
            while self
                .neutrons
                .first()
                .is_some_and(|n| n.kind == NeutronType::ZwspSeparator)
            {
                self.neutrons.remove(0);
            }
            while self.neutrons.len() > 1
                && self.neutrons.last().is_some_and(|n| {
                    matches!(n.kind, NeutronType::Whitespace | NeutronType::ZwspSeparator)
                })
            {
                self.neutrons.pop();
            }
        }
        fn add(&mut self, n: Neutron, sb: &dyn StringBounder) {
            if n.kind == NeutronType::ZwspSeparator
                && self
                    .neutrons
                    .last()
                    .is_none_or(|l| l.kind == NeutronType::ZwspSeparator)
            {
                return;
            }
            if self.remove_initial_spaces
                && self.neutrons.is_empty()
                && n.kind == NeutronType::Whitespace
            {
                return;
            }
            self.width += n.width(sb);
            self.neutrons.push(n);
        }
        fn atoms(&self) -> Vec<Atom> {
            let mut v: Vec<Atom> = self.header.iter().cloned().collect();
            for n in &self.neutrons {
                if n.kind == NeutronType::ZwspSeparator {
                    continue;
                }
                if self.remove_initial_spaces && v.is_empty() && n.kind == NeutronType::Whitespace {
                    continue;
                }
                v.extend(n.atom.clone());
            }
            v
        }
    }

    let (header, body) = if has_header {
        (atoms.first().cloned(), &atoms[1..])
    } else {
        (None, atoms)
    };
    let mut lines = vec![Line::new(false, header.clone(), sb)];
    let mut all: std::collections::VecDeque<Neutron> =
        body.iter().flat_map(Atom::neutrons).collect();
    if all
        .back()
        .is_none_or(|n| n.kind != NeutronType::ZwspSeparator)
    {
        all.push_back(Neutron::zwsp());
    }
    while let Some(current) = all.pop_front() {
        let line = lines.last_mut().expect("at least one line");
        if current.kind == NeutronType::ZwspSeparator && line.width > value {
            all.push_front(current);
            let removed = line.shorten();
            for n in removed.into_iter().rev() {
                all.push_front(n);
            }
            let blank = header.clone().map(|h| Atom::Blank(Box::new(h)));
            lines.push(Line::new(true, blank, sb));
        } else {
            line.add(current, sb);
        }
    }
    for l in &mut lines {
        l.remove_final_spaces();
    }
    while lines.len() > 1 && lines.last().is_some_and(Line::is_white) {
        lines.pop();
    }
    lines
        .iter()
        .map(|l| LineStripe {
            atoms: l.atoms(),
            align: None,
        })
        .collect()
}

fn coef(a: HorizontalAlignment) -> f64 {
    match a {
        HorizontalAlignment::Center => 2.0,
        HorizontalAlignment::Right => 1.0,
        HorizontalAlignment::Left => 0.0,
    }
}

/// Text laid out from a sheet (`SheetBlock1`), also acting as a stencil.
pub struct SheetInner {
    sheet: Sheet,
    max_width: LineBreakStrategy,
    padding: Margins,
    margin_x1: f64,
    margin_x2: f64,
    pub minimum_width: f64,
    layout: RefCell<Option<Rc<Layout>>>,
}

impl SheetInner {
    fn layout(&self, sb: &dyn StringBounder) -> Rc<Layout> {
        if let Some(l) = self.layout.borrow().as_ref() {
            return l.clone();
        }
        let l = Rc::new(self.compute(sb));
        *self.layout.borrow_mut() = Some(l.clone());
        l
    }

    fn compute(&self, sb: &dyn StringBounder) -> Layout {
        let mut stripes = Vec::new();
        for s in &self.sheet.stripes {
            stripes.extend(split_stripe(
                &s.atoms(),
                s.has_header(),
                s.align,
                &self.max_width,
                sb,
            ));
        }
        let mut positions: Vec<Vec<Position>> = Vec::new();
        let mut widths = Vec::new();
        let mut min_max = MinMax::empty(true);
        let mut y = 0.0;
        for stripe in &stripes {
            if stripe.atoms.is_empty() {
                positions.push(Vec::new());
                widths.push(0.0);
                continue;
            }
            // Sea: atoms side by side, baseline aligned, then moved down to `y`.
            let mut x = 0.0;
            let mut pos: Vec<Position> = stripe
                .atoms
                .iter()
                .map(|a| {
                    let dim = a.dimension(sb);
                    let p = Position {
                        x,
                        y: -dim.height + a.starting_altitude(),
                        dim,
                    };
                    x += dim.width;
                    p
                })
                .collect();
            let min_y = pos.iter().map(|p| p.y).fold(f64::MAX, f64::min);
            let max_y = pos.iter().map(Position::max_y).fold(-f64::MAX, f64::max);
            let delta = y - min_y;
            for p in &mut pos {
                p.y += delta;
                min_max = min_max.add_point(p.x + p.dim.width, p.y + p.dim.height);
            }
            widths.push(x);
            y += max_y - min_y;
            positions.push(pos);
        }
        let max_w = widths.iter().copied().fold(0.0, f64::max);
        for (i, stripe) in stripes.iter().enumerate() {
            let diff = max_w - widths[i];
            let c = coef(stripe.align.unwrap_or(self.sheet.alignment));
            if diff > 0.0 && c > 0.0 {
                for p in &mut positions[i] {
                    p.x += diff / c;
                }
            }
        }
        Layout {
            stripes,
            positions,
            min_max,
        }
    }

    fn dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let l = self.layout(sb);
        let d = Dimension::new(l.min_max.width(), l.min_max.height());
        let extra = self.padding.bottom + self.padding.top;
        d.delta(extra, extra)
    }

    fn draw(&self, ug: &mut UGraphic<'_>) {
        let l = self.layout(ug.string_bounder());
        let mut ug = if self.padding.left > 0.0 || self.padding.top > 0.0 {
            ug.apply_translate(UTranslate::new(self.padding.left, self.padding.top))
        } else {
            ug.scope()
        };
        for (stripe, pos) in l.stripes.iter().zip(&l.positions) {
            for (atom, p) in stripe.atoms.iter().zip(pos) {
                atom.draw(&mut ug.apply_translate(UTranslate::new(p.x, p.y)));
            }
        }
    }
}

impl Stencil for SheetInner {
    fn starting_x(&self, _: &dyn StringBounder, _: f64) -> f64 {
        -self.margin_x1
    }
    fn ending_x(&self, sb: &dyn StringBounder, _: f64) -> f64 {
        self.dimension(sb).width + self.margin_x2
    }
}

struct Enlarged {
    inner: Rc<SheetInner>,
    left: f64,
    right: f64,
}

impl Stencil for Enlarged {
    fn starting_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        self.inner.starting_x(sb, y) - self.left
    }
    fn ending_x(&self, sb: &dyn StringBounder, y: f64) -> f64 {
        self.inner.ending_x(sb, y) + self.right
    }
}

/// Text block with horizontal-line support (`SheetBlock1` inside `SheetBlock2`).
pub struct SheetBlock {
    inner: Rc<SheetInner>,
    enlarge: (f64, f64),
    default_stroke: UStroke,
}

impl std::fmt::Debug for SheetBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SheetBlock({} stripes)", self.inner.sheet.stripes.len())
    }
}

impl SheetBlock {
    pub fn new(
        sheet: Sheet,
        max_width: LineBreakStrategy,
        padding: Margins,
        margin_x1: f64,
        margin_x2: f64,
    ) -> Self {
        Self {
            inner: Rc::new(SheetInner {
                sheet,
                max_width,
                padding,
                margin_x1,
                margin_x2,
                minimum_width: 0.0,
                layout: RefCell::new(None),
            }),
            enlarge: (0.0, 0.0),
            default_stroke: UStroke::with_thickness(1.5),
        }
    }

    pub fn with_minimum_width(mut self, w: f64) -> Self {
        let inner = Rc::get_mut(&mut self.inner).expect("not shared yet");
        inner.minimum_width = w;
        self
    }

    /// Horizontal lines reach `left`/`right` further (`enlargeMe`).
    pub fn enlarge_me(&self, left: f64, right: f64) -> SheetBlock {
        SheetBlock {
            inner: self.inner.clone(),
            enlarge: (left, right),
            default_stroke: self.default_stroke,
        }
    }

    fn alignment(&self) -> HorizontalAlignment {
        self.inner.sheet.alignment
    }
}

impl TextBlock for SheetBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.inner.dimension(sb)
    }

    fn draw(&self, ug: &mut UGraphic<'_>) {
        let stencil = Rc::new(Enlarged {
            inner: self.inner.clone(),
            left: self.enlarge.0,
            right: self.enlarge.1,
        });
        let mut ug = ug.apply_stencil(stencil, self.default_stroke);
        let min = self.inner.minimum_width;
        let dx = if min > 0.0 && self.alignment() != HorizontalAlignment::Left {
            let w = self.inner.dimension(ug.string_bounder()).width;
            match self.alignment() {
                HorizontalAlignment::Center => (min - w) / 2.0,
                _ => min - w,
            }
        } else {
            0.0
        };
        self.inner.draw(&mut ug.apply_translate(UTranslate::dx(dx)));
    }
}

/// `Display.create0` for plain text: lines to a laid out block.
pub fn create_text_block(
    lines: &[String],
    fc: &FontConfiguration,
    align: HorizontalAlignment,
    skin: &SkinSimple,
    max_width: LineBreakStrategy,
    mode: CreoleMode,
) -> Rc<SheetBlock> {
    let sheet = create_sheet(lines, fc, align, skin, mode);
    Rc::new(SheetBlock::new(sheet, max_width, skin.padding, 0.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::drawing::svg::{SvgGraphics, SvgOption};
    use crate::klimt::font::{EmbeddedFontBounder, UFont};

    fn fc() -> FontConfiguration {
        FontConfiguration::black_blue_true(UFont::sans_serif(14))
    }

    fn block(lines: &[&str], align: HorizontalAlignment, w: &str) -> Rc<SheetBlock> {
        let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        create_text_block(
            &lines,
            &fc(),
            align,
            &SkinSimple::default(),
            LineBreakStrategy::new(w),
            CreoleMode::Full,
        )
    }

    fn render(b: &dyn TextBlock) -> String {
        let mut svg = SvgGraphics::new(0, SvgOption::default());
        {
            let mut ug = UGraphic::new(&mut svg, &EmbeddedFontBounder);
            b.draw(&mut ug);
        }
        svg.to_svg()
    }

    #[test]
    fn single_line_dimension_and_draw() {
        let b = block(&["Hello"], HorizontalAlignment::Left, "");
        let d = b.calculate_dimension(&EmbeddedFontBounder);
        assert!((d.height - 17.609375).abs() < 1e-6, "{d:?}");
        assert!(d.width > 25.0 && d.width < 40.0, "{d:?}");
        let out = render(&*b);
        assert!(out.contains(">Hello</text>"), "{out}");
    }

    #[test]
    fn multi_line_alignment() {
        let b = block(&["a", "longer line"], HorizontalAlignment::Center, "");
        let d = b.calculate_dimension(&EmbeddedFontBounder);
        let one =
            block(&["a"], HorizontalAlignment::Left, "").calculate_dimension(&EmbeddedFontBounder);
        assert!((d.height - 2.0 * one.height).abs() < 1e-6);
        let out = render(&*b);
        assert!(
            out.contains(">a</text>") && out.contains(">longer line</text>"),
            "{out}"
        );
    }

    #[test]
    fn wrapping() {
        let b = block(
            &["alpha beta gamma delta epsilon"],
            HorizontalAlignment::Left,
            "60",
        );
        let d = b.calculate_dimension(&EmbeddedFontBounder);
        let one = block(&["alpha"], HorizontalAlignment::Left, "")
            .calculate_dimension(&EmbeddedFontBounder);
        assert!(d.height > 2.0 * one.height, "{d:?}");
        let out = render(&*b);
        assert!(
            out.contains(">alpha</text>") && out.contains(">epsilon</text>"),
            "{out}"
        );
    }

    #[test]
    fn lines_lists_and_tabs() {
        let b = block(
            &["* item", "# num", "----", "== t ==", "\ta\tb", "<space:4>x"],
            HorizontalAlignment::Left,
            "",
        );
        let out = render(&*b);
        assert!(out.contains("<line"), "{out}");
        assert!(out.contains("<ellipse"), "{out}");
        assert!(out.contains(">1.</text>"), "{out}");
        assert!(out.contains(">t</text>"), "{out}");
    }

    #[test]
    fn min_width_and_enlarge() {
        let b = block(&["x"], HorizontalAlignment::Center, "").enlarge_me(2.0, 2.0);
        let mut b2 = block(&["y", "----"], HorizontalAlignment::Right, "");
        Rc::get_mut(&mut b2).unwrap().inner = {
            let mut n = SheetBlock::new(
                create_sheet(
                    &["y".to_string(), "----".to_string()],
                    &fc(),
                    HorizontalAlignment::Right,
                    &SkinSimple::default(),
                    CreoleMode::Full,
                ),
                LineBreakStrategy::NONE,
                Margins::default(),
                1.0,
                1.0,
            )
            .with_minimum_width(100.0);
            n.enlarge = (0.0, 0.0);
            n.inner.clone()
        };
        assert!(render(&b).contains(">x</text>"));
        let out = render(&*b2);
        assert!(out.contains(">y</text>") && out.contains("<line"), "{out}");
        let st = b2.inner.clone();
        assert_eq!(st.starting_x(&EmbeddedFontBounder, 0.0), -1.0);
    }
}
