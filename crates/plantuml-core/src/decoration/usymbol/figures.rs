//! Symbols made of a drawn figure: actors, boundary, control, entity, interface, person and
//! use case.

use super::common::*;
use super::USymbol;
use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{
    footprint, HLineDrawer, UEllipse, UGraphic, ULine, URectangle, UStroke, UTranslate,
};
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, Stencil, TextBlock, UHorizontalLine};
use crate::skin::actor::ActorStyle;
use crate::style::{Margins, SName};
use crate::svek::{Boundary, Control, EntityDomain};
use std::rc::Rc;

fn unsupported_big(symbol: &str, width: f64, height: f64) -> Rc<dyn TextBlock> {
    log::warn!("{symbol} has no big form");
    fn_block_sized(width, height, |_| {})
}

fn shadow_4(fashion: &Fashion) -> Fashion {
    fashion
        .clone()
        .with_delta_shadow(if fashion.is_shadowing() { 4.0 } else { 0.0 })
}

// ---- figure over its label (`USymbolSimpleAbstract`) --------------------------------------

type DrawingFn = fn(&Fashion) -> Rc<dyn TextBlock>;

/// Symbol drawing a figure above its label: actors, boundary, control, entity and interface.
pub struct FigureSymbol {
    snames: Vec<SName>,
    drawing: DrawingFn,
}

impl USymbol for FigureSymbol {
    fn snames(&self) -> Vec<SName> {
        self.snames.clone()
    }

    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let stickman = (self.drawing)(&context);
        let (l, s, m) = (label.clone(), stereotype.clone(), stickman.clone());
        fn_block(
            move |sb| {
                let dim_label = l.calculate_dimension(sb);
                let dim_stereo = s.calculate_dimension(sb);
                let dim_actor = m.calculate_dimension(sb);
                Dimension::new(
                    dim_stereo.width.max(dim_actor.width).max(dim_label.width),
                    dim_stereo.height + dim_actor.height + dim_label.height,
                )
            },
            move |ug| {
                let sb = ug.string_bounder();
                let dim_label = label.calculate_dimension(sb);
                let dim_stereo = stereotype.calculate_dimension(sb);
                let dim_stickman = stickman.calculate_dimension(sb);
                let total_width = dim_stereo
                    .width
                    .max(dim_stickman.width)
                    .max(dim_label.width);
                let ug = context.apply(ug);
                stickman.draw(&ug.apply_translate(UTranslate::new(
                    (total_width - dim_stickman.width) / 2.0,
                    dim_stereo.height,
                )));
                let label_x = (total_width - dim_label.width) / 2.0;
                let label_y = dim_stickman.height + dim_stereo.height;
                let ug2 = stencil_for(&ug, dim_label);
                label.draw(&ug2.apply_translate(UTranslate::new(label_x, label_y)));
                let stereo_x = (total_width - dim_stereo.width) / 2.0;
                stereotype.draw(&ug.apply_translate(UTranslate::dx(stereo_x)));
            },
        )
    }

    fn as_big(
        &self,
        _label: Rc<dyn TextBlock>,
        _label_alignment: HorizontalAlignment,
        _stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        _context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        unsupported_big("figure symbol", width, height)
    }
}

fn actor_drawing(style: ActorStyle) -> DrawingFn {
    match style {
        ActorStyle::Stickman => |f| ActorStyle::Stickman.text_block(f.clone()),
        ActorStyle::StickmanBusiness => |f| ActorStyle::StickmanBusiness.text_block(f.clone()),
        ActorStyle::Awesome => |f| ActorStyle::Awesome.text_block(f.clone()),
        ActorStyle::Hollow => |f| ActorStyle::Hollow.text_block(f.clone()),
    }
}

/// `USymbolActor` (`ACTOR_STICKMAN`, `ACTOR_AWESOME` and `ACTOR_HOLLOW`).
pub fn actor(style: ActorStyle) -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Actor],
        drawing: actor_drawing(style),
    })
}

/// `USymbolActorBusiness`.
pub fn actor_business() -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Business],
        drawing: actor_drawing(ActorStyle::StickmanBusiness),
    })
}

/// `USymbolBoundary`.
pub fn boundary() -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Boundary],
        drawing: |f| {
            Rc::new(Boundary {
                symbol_context: shadow_4(f),
            })
        },
    })
}

/// `USymbolControl`.
pub fn control() -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Control],
        drawing: |f| {
            Rc::new(Control {
                symbol_context: shadow_4(f),
            })
        },
    })
}

/// `USymbolEntityDomain`.
pub fn entity_domain() -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Entity],
        drawing: |f| {
            Rc::new(EntityDomain {
                symbol_context: shadow_4(f),
            })
        },
    })
}

/// Small circle of an interface (`CircleInterface2`).
pub struct CircleInterface {
    back_color: Option<HColor>,
    fore_color: Option<HColor>,
    delta_shadow: f64,
}

impl TextBlock for CircleInterface {
    fn calculate_dimension(&self, _: &dyn StringBounder) -> Dimension {
        Dimension::new(8.0 * 2.0 + 2.0, 8.0 * 2.0 + 2.0)
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug
            .apply_back_color(self.back_color.clone().unwrap_or(HColor::TRANSPARENT))
            .apply_color(self.fore_color.clone().unwrap_or(HColor::TRANSPARENT));
        let mut circle = UEllipse::build(8.0 * 2.0, 8.0 * 2.0);
        circle.delta_shadow = self.delta_shadow;
        ug.apply_translate(UTranslate::new(1.0, 1.0)).draw(circle);
    }
}

/// `USymbolInterface`.
pub fn interface() -> Rc<dyn USymbol> {
    Rc::new(FigureSymbol {
        snames: vec![SName::Interface],
        drawing: |f| {
            Rc::new(CircleInterface {
                back_color: f.back_color().cloned(),
                fore_color: f.fore_color().cloned(),
                delta_shadow: if f.is_shadowing() { 4.0 } else { 0.0 },
            })
        },
    })
}

// ---- person -------------------------------------------------------------------------------

const PERSON_MARGIN: Margin = Margin::new(10.0, 10.0, 10.0, 10.0);

fn person_head_size(body: Dimension) -> f64 {
    (body.width * body.height).sqrt() * 0.42
}

fn person_body_dimension(
    label: &Rc<dyn TextBlock>,
    stereotype: &Rc<dyn TextBlock>,
    sb: &dyn StringBounder,
) -> Dimension {
    small_dimension(label, stereotype, PERSON_MARGIN, sb)
}

struct Person;

impl USymbol for Person {
    fn snames(&self) -> Vec<SName> {
        vec![SName::Person]
    }

    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let (l, s) = (label.clone(), stereotype.clone());
        fn_block(
            move |sb| {
                let body = person_body_dimension(&l, &s, sb);
                body.delta(0.0, person_head_size(body))
            },
            move |ug| {
                let sb = ug.string_bounder();
                let body = person_body_dimension(&label, &stereotype, sb);
                let full = body.delta(0.0, person_head_size(body));
                let ug = stencil_for(ug, full);
                let ug = context.apply(&ug);
                let head_size = person_head_size(body);
                let shadow = context.delta_shadow();
                let mut head = UEllipse::build(head_size, head_size);
                let mut shape = URectangle::build(body.width, body.height).rounded(head_size);
                shape.delta_shadow = shadow;
                head.delta_shadow = shadow;
                let posx = (body.width - head_size) / 2.0;
                ug.apply_translate(UTranslate::dx(posx)).draw(head);
                ug.apply_translate(UTranslate::dy(head_size)).draw(shape);
                let tb = merge_tb(&stereotype, &label, stereo_alignment);
                tb.draw(&ug.apply_translate(UTranslate::new(
                    PERSON_MARGIN.x1,
                    PERSON_MARGIN.y1 + head_size,
                )));
            },
        )
    }

    fn as_big(
        &self,
        _label: Rc<dyn TextBlock>,
        _label_alignment: HorizontalAlignment,
        _stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        _context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        unsupported_big("person", width, height)
    }
}

/// `USymbolPerson`.
pub fn person() -> Rc<dyn USymbol> {
    Rc::new(Person)
}

// ---- use case -----------------------------------------------------------------------------

/// Smallest circle containing a set of points (`SmallestEnclosingCircle`).
struct Circle {
    center: (f64, f64),
    radius: f64,
}

impl Circle {
    fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    fn point(p: (f64, f64)) -> Self {
        Self {
            center: p,
            radius: 0.0,
        }
    }

    fn two(p1: (f64, f64), p2: (f64, f64)) -> Self {
        let center = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        Self {
            center,
            radius: Self::distance(p1, center),
        }
    }

    fn three(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Self {
        let num1 = p3.0 * p3.0 * (p1.1 - p2.1)
            + (p1.0 * p1.0 + (p1.1 - p2.1) * (p1.1 - p3.1)) * (p2.1 - p3.1)
            + p2.0 * p2.0 * (-p1.1 + p3.1);
        let den1 = 2.0 * (p3.0 * (p1.1 - p2.1) + p1.0 * (p2.1 - p3.1) + p2.0 * (-p1.1 + p3.1));
        let x = num1 / den1;
        let den2 = p3.1 - p2.1;
        let y = (p2.1 + p3.1) / 2.0 - (p3.0 - p2.0) / den2 * (x - (p2.0 + p3.0) / 2.0);
        let center = (x, y);
        Self {
            center,
            radius: Self::distance(center, p1),
        }
    }

    fn of_three(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Self {
        if p3.1 != p2.1 {
            Self::three(p1, p2, p3)
        } else {
            Self::three(p2, p1, p3)
        }
    }

    fn is_outside(&self, p: (f64, f64)) -> bool {
        Self::distance(self.center, p) > self.radius
    }
}

fn find_sec(n: usize, p: &[(f64, f64)], m: usize, b: &mut Vec<(f64, f64)>) -> Circle {
    let mut sec = Circle::point((0.0, 0.0));
    match m {
        1 => sec = Circle::point(b[0]),
        2 => sec = Circle::two(b[0], b[1]),
        3 => return Circle::of_three(b[0], b[1], b[2]),
        _ => {}
    }
    for (i, &pt) in p.iter().enumerate().take(n) {
        if sec.is_outside(pt) {
            if m < b.len() {
                b[m] = pt;
            } else {
                b.push(pt);
            }
            sec = find_sec(i, p, m + 1, b);
        }
    }
    sec
}

/// Ellipse of a given flatness around some points (`ContainingEllipse`).
struct ContainingEllipse {
    alpha: f64,
    points: Vec<(f64, f64)>,
}

impl ContainingEllipse {
    fn new(alpha: f64) -> Self {
        Self {
            alpha: if alpha.is_nan() { 1.0 } else { alpha },
            points: Vec::new(),
        }
    }

    fn append(&mut self, pt: (f64, f64)) {
        let pt = (pt.0, pt.1 / self.alpha);
        if !self.points.contains(&pt) {
            self.points.push(pt);
        }
    }

    fn circle(&self) -> Circle {
        let mut b = self.points.clone();
        find_sec(self.points.len(), &self.points, 0, &mut b)
    }

    fn width(&self) -> f64 {
        2.0 * self.circle().radius
    }

    fn height(&self) -> f64 {
        2.0 * self.circle().radius * self.alpha
    }

    fn center(&self) -> (f64, f64) {
        let c = self.circle().center;
        (c.0, c.1 * self.alpha)
    }
}

/// Text inside an ellipse that just contains it (`TextBlockInEllipse`).
struct TextBlockInEllipse {
    text: Rc<dyn TextBlock>,
    ellipse: UEllipse,
    center: (f64, f64),
}

impl TextBlockInEllipse {
    fn new(text: Rc<dyn TextBlock>, sb: &dyn StringBounder) -> Self {
        let dim = text.calculate_dimension(sb);
        let alpha = (dim.height / dim.width).clamp(0.2, 0.8);
        let drawn = text.clone();
        let points = footprint(sb.to_rc(), move |ug| drawn.draw(ug));
        let mut containing = ContainingEllipse::new(alpha);
        for pt in points {
            containing.append(pt);
        }
        Self {
            text,
            ellipse: UEllipse::build(containing.width(), containing.height()).bigger(6.0),
            center: containing.center(),
        }
    }

    fn draw(&self, ug: &UGraphic) {
        let dx = self.ellipse.width / 2.0 - self.center.0;
        let dy = self.ellipse.height / 2.0 - self.center.1;
        ug.draw(self.ellipse.clone());
        self.text
            .draw(&ug.apply_translate(UTranslate::new(dx, dy - 2.0)));
    }

    fn dimension(&self) -> Dimension {
        Dimension::new(self.ellipse.width, self.ellipse.height)
    }
}

/// Horizontal lines follow the border of the ellipse (`MyUGraphicEllipse`).
struct EllipseHLine {
    ellipse: UEllipse,
}

struct EllipseStencil {
    ellipse: UEllipse,
    dy: f64,
}

impl EllipseStencil {
    fn normalized(&self, y: f64) -> f64 {
        (y + self.dy).clamp(0.0, self.ellipse.height)
    }
}

impl Stencil for EllipseStencil {
    fn starting_x(&self, _: &dyn StringBounder, y: f64) -> f64 {
        self.ellipse.starting_x(self.normalized(y))
    }

    fn ending_x(&self, _: &dyn StringBounder, y: f64) -> f64 {
        self.ellipse.ending_x(self.normalized(y))
    }
}

impl HLineDrawer for EllipseHLine {
    fn draw_hline(&self, ug: &UGraphic, line: &UHorizontalLine, translate: UTranslate) {
        let stencil = EllipseStencil {
            ellipse: self.ellipse.clone(),
            dy: translate.dy,
        };
        line.draw_line_internal(
            &ug.apply_translate(translate),
            &stencil,
            0.0,
            UStroke::with_thickness(1.5),
        );
    }
}

/// `USymbolUsecase`.
struct Usecase {
    is_business: bool,
}

/// Slanted line across a business use case.
fn special_business(ug: &UGraphic, frontier: &UEllipse) {
    let beta = std::f64::consts::PI / 4.0;
    let (a, b) = (frontier.width / 2.0, frontier.height / 2.0);
    let point_x = |theta: f64| {
        let (x, y) = (a * theta.cos(), b * theta.sin());
        x * beta.cos() - y * beta.sin()
    };
    let theta1 = 20.0 * std::f64::consts::PI / 180.0;
    let z = point_x(theta1);
    let aa = a * beta.cos();
    let bb = b * beta.sin();
    let sum = 2.0 * aa * z / (aa * aa + bb * bb);
    let theta2 = -(sum - theta1.cos()).acos();
    let frontier2 = frontier.scale(0.99);
    let p1 = frontier2.point_at_angle(-theta1);
    let p2 = frontier2.point_at_angle(-theta2);
    ug.apply_translate(UTranslate::new(p1.0, p1.1))
        .draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
}

impl USymbol for Usecase {
    fn snames(&self) -> Vec<SName> {
        if self.is_business {
            vec![SName::Usecase, SName::Business]
        } else {
            vec![SName::Usecase]
        }
    }

    fn as_small(
        &self,
        _name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let tmp = merge_tb(&stereotype, &label, HorizontalAlignment::Center);
        let desc = if self.is_business {
            with_margin(tmp, Margins::new(0.0, 7.0, 0.0, 7.0))
        } else {
            tmp
        };
        let is_business = self.is_business;
        let d = desc.clone();
        fn_block(
            move |sb| TextBlockInEllipse::new(d.clone(), sb).dimension(),
            move |ug| {
                let ug = context.apply(ug);
                let ellipse = TextBlockInEllipse::new(desc.clone(), ug.string_bounder());
                let ug2 = ug.apply_hline_drawer(Rc::new(EllipseHLine {
                    ellipse: ellipse.ellipse.clone(),
                }));
                ellipse.draw(&ug2);
                if is_business {
                    special_business(&ug, &ellipse.ellipse);
                }
            },
        )
    }

    fn as_big(
        &self,
        _label: Rc<dyn TextBlock>,
        _label_alignment: HorizontalAlignment,
        _stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        _context: Fashion,
        _stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        unsupported_big("usecase", width, height)
    }
}

/// `USymbols.USECASE` and `USymbols.USECASE_BUSINESS`.
pub fn usecase(is_business: bool) -> Rc<dyn USymbol> {
    Rc::new(Usecase { is_business })
}
