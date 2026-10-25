//! Arrow appearance, ported from `skin/Arrow*`.

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, UStroke};
use crate::style::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowBody {
    Normal,
    Dotted,
    Dashed,
    Hidden,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowHead {
    Normal,
    CrossX,
    Async,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowPart {
    Full,
    TopPart,
    BottomPart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDecoration {
    None,
    Circle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    LeftToRightNormal,
    RightToLeftReverse,
    SelfArrow,
    BothDirection,
}

impl ArrowDirection {
    pub fn reverse(self) -> ArrowDirection {
        match self {
            ArrowDirection::LeftToRightNormal => ArrowDirection::RightToLeftReverse,
            ArrowDirection::RightToLeftReverse => ArrowDirection::LeftToRightNormal,
            _ => panic!("cannot reverse {self:?}"),
        }
    }
}

/// Head and part of one end of an arrow (`ArrowDressing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrowDressing {
    head: ArrowHead,
    part: ArrowPart,
}

impl ArrowDressing {
    pub const fn create() -> Self {
        Self {
            head: ArrowHead::None,
            part: ArrowPart::Full,
        }
    }

    pub const fn with_head(self, head: ArrowHead) -> Self {
        Self { head, ..self }
    }

    pub const fn with_part(self, part: ArrowPart) -> Self {
        Self { part, ..self }
    }

    pub fn head(&self) -> ArrowHead {
        self.head
    }

    pub fn part(&self) -> ArrowPart {
        self.part
    }
}

/// Complete description of an arrow (`ArrowConfiguration`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowConfiguration {
    body: ArrowBody,
    dressing1: ArrowDressing,
    dressing2: ArrowDressing,
    decoration1: ArrowDecoration,
    decoration2: ArrowDecoration,
    color: Option<HColor>,
    is_self: bool,
    thickness: f64,
    reverse_define: bool,
    inclination: i32,
}

impl ArrowConfiguration {
    fn base(
        dressing1: ArrowDressing,
        dressing2: ArrowDressing,
        is_self: bool,
        reverse_define: bool,
    ) -> Self {
        Self {
            body: ArrowBody::Normal,
            dressing1,
            dressing2,
            decoration1: ArrowDecoration::None,
            decoration2: ArrowDecoration::None,
            color: None,
            is_self,
            thickness: 1.0,
            reverse_define,
            inclination: 0,
        }
    }

    pub fn with_direction_normal() -> Self {
        Self::base(
            ArrowDressing::create(),
            ArrowDressing::create().with_head(ArrowHead::Normal),
            false,
            false,
        )
    }

    pub fn with_direction_both() -> Self {
        let d = ArrowDressing::create().with_head(ArrowHead::Normal);
        Self::base(d, d, false, false)
    }

    pub fn with_direction_self(reverse_define: bool) -> Self {
        Self::base(
            ArrowDressing::create(),
            ArrowDressing::create().with_head(ArrowHead::Normal),
            true,
            reverse_define,
        )
    }

    pub fn with_direction_reverse() -> Self {
        Self::with_direction_normal().reverse()
    }

    pub fn reverse(&self) -> Self {
        Self {
            dressing1: self.dressing2,
            dressing2: self.dressing1,
            decoration1: self.decoration2,
            decoration2: self.decoration1,
            ..self.clone()
        }
    }

    pub fn self_arrow(&self) -> Self {
        Self {
            is_self: true,
            ..self.clone()
        }
    }

    pub fn with_body(&self, body: ArrowBody) -> Self {
        Self {
            body,
            ..self.clone()
        }
    }

    /// Changes the head of the ends that have one (`withHead`).
    pub fn with_head(&self, head: ArrowHead) -> Self {
        let add = |d: ArrowDressing| {
            if d.head == ArrowHead::None {
                d
            } else {
                d.with_head(head)
            }
        };
        Self {
            dressing1: add(self.dressing1),
            dressing2: add(self.dressing2),
            ..self.clone()
        }
    }

    pub fn with_head1(&self, head: ArrowHead) -> Self {
        Self {
            dressing1: self.dressing1.with_head(head),
            ..self.clone()
        }
    }

    pub fn with_head2(&self, head: ArrowHead) -> Self {
        Self {
            dressing2: self.dressing2.with_head(head),
            ..self.clone()
        }
    }

    pub fn with_part(&self, part: ArrowPart) -> Self {
        if self.dressing2.head != ArrowHead::None {
            Self {
                dressing2: self.dressing2.with_part(part),
                ..self.clone()
            }
        } else {
            Self {
                dressing1: self.dressing1.with_part(part),
                ..self.clone()
            }
        }
    }

    pub fn with_decoration1(&self, d: ArrowDecoration) -> Self {
        Self {
            decoration1: d,
            ..self.clone()
        }
    }

    pub fn with_decoration2(&self, d: ArrowDecoration) -> Self {
        Self {
            decoration2: d,
            ..self.clone()
        }
    }

    pub fn with_color(&self, color: Option<HColor>) -> Self {
        Self {
            color,
            ..self.clone()
        }
    }

    pub fn with_thickness(&self, thickness: f64) -> Self {
        Self {
            thickness,
            ..self.clone()
        }
    }

    pub fn reverse_define(&self) -> Self {
        Self {
            reverse_define: !self.reverse_define,
            ..self.clone()
        }
    }

    pub fn with_inclination(&self, inclination: i32) -> Self {
        Self {
            inclination,
            ..self.clone()
        }
    }

    pub fn decoration1(&self) -> ArrowDecoration {
        self.decoration1
    }

    pub fn decoration2(&self) -> ArrowDecoration {
        self.decoration2
    }

    pub fn dressing1(&self) -> ArrowDressing {
        self.dressing1
    }

    pub fn dressing2(&self) -> ArrowDressing {
        self.dressing2
    }

    pub fn color(&self) -> Option<&HColor> {
        self.color.as_ref()
    }

    pub fn is_reverse_define(&self) -> bool {
        self.reverse_define
    }

    pub fn arrow_direction(&self) -> ArrowDirection {
        if self.is_self {
            return ArrowDirection::SelfArrow;
        }
        match (self.dressing1.head, self.dressing2.head) {
            (ArrowHead::None, h) if h != ArrowHead::None => ArrowDirection::LeftToRightNormal,
            (h, ArrowHead::None) if h != ArrowHead::None => ArrowDirection::RightToLeftReverse,
            _ => ArrowDirection::BothDirection,
        }
    }

    pub fn is_self_arrow(&self) -> bool {
        self.arrow_direction() == ArrowDirection::SelfArrow
    }

    pub fn is_dotted(&self) -> bool {
        self.body == ArrowBody::Dotted
    }

    pub fn is_hidden(&self) -> bool {
        self.body == ArrowBody::Hidden
    }

    pub fn head(&self) -> ArrowHead {
        if self.dressing2.head != ArrowHead::None {
            self.dressing2.head
        } else {
            self.dressing1.head
        }
    }

    pub fn is_async1(&self) -> bool {
        self.dressing1.head == ArrowHead::Async
    }

    pub fn is_async2(&self) -> bool {
        self.dressing2.head == ArrowHead::Async
    }

    pub fn part(&self) -> ArrowPart {
        if self.dressing2.head != ArrowHead::None {
            self.dressing2.part
        } else {
            self.dressing1.part
        }
    }

    pub fn inclination1(&self) -> i32 {
        if matches!(self.dressing2.head, ArrowHead::None | ArrowHead::CrossX) {
            self.inclination
        } else {
            0
        }
    }

    pub fn inclination2(&self) -> i32 {
        if matches!(
            self.dressing1.head,
            ArrowHead::None | ArrowHead::CrossX | ArrowHead::Normal
        ) {
            self.inclination
        } else {
            0
        }
    }

    /// Stroke from the style, dotted arrows get a default dash (`applyStroke(ug, style)`).
    pub fn apply_stroke(&self, ug: &UGraphic, style: &Style) -> UGraphic {
        let stroke = style.stroke();
        if self.is_dotted() && stroke.dash_visible == 0.0 && stroke.dash_space == 0.0 {
            return ug.apply_stroke(UStroke::new(2.0, 2.0, stroke.thickness));
        }
        ug.apply_stroke(stroke)
    }

    pub fn apply_thickness_only(&self, ug: &UGraphic) -> UGraphic {
        ug.apply_stroke(UStroke::with_thickness(self.thickness))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions() {
        let n = ArrowConfiguration::with_direction_normal();
        assert_eq!(n.arrow_direction(), ArrowDirection::LeftToRightNormal);
        let r = ArrowConfiguration::with_direction_reverse();
        assert_eq!(r.arrow_direction(), ArrowDirection::RightToLeftReverse);
        assert_eq!(
            r.arrow_direction().reverse(),
            ArrowDirection::LeftToRightNormal
        );
        assert_eq!(
            ArrowConfiguration::with_direction_both().arrow_direction(),
            ArrowDirection::BothDirection
        );
        let s = ArrowConfiguration::with_direction_self(true);
        assert!(s.is_self_arrow() && s.is_reverse_define());
        assert!(!s.reverse_define().is_reverse_define());
        assert!(n.self_arrow().is_self_arrow());
    }

    #[test]
    #[should_panic]
    fn cannot_reverse_self() {
        let _ = ArrowDirection::SelfArrow.reverse();
    }

    #[test]
    fn heads_and_parts() {
        let n = ArrowConfiguration::with_direction_normal();
        assert_eq!(n.head(), ArrowHead::Normal);
        let a = n.with_head(ArrowHead::Async);
        assert!(a.is_async2() && !a.is_async1());
        assert_eq!(
            a.with_head1(ArrowHead::Async).dressing1().head(),
            ArrowHead::Async
        );
        assert_eq!(
            n.with_head2(ArrowHead::CrossX).dressing2().head(),
            ArrowHead::CrossX
        );
        assert_eq!(n.with_part(ArrowPart::TopPart).part(), ArrowPart::TopPart);
        let rev = ArrowConfiguration::with_direction_reverse().with_part(ArrowPart::BottomPart);
        assert_eq!(rev.part(), ArrowPart::BottomPart);
        let both = ArrowConfiguration::with_direction_both().with_part(ArrowPart::TopPart);
        assert_eq!(both.dressing2().part(), ArrowPart::TopPart);
        assert_eq!(
            n.with_decoration1(ArrowDecoration::Circle).decoration1(),
            ArrowDecoration::Circle
        );
        assert_eq!(
            n.with_decoration2(ArrowDecoration::Circle).decoration2(),
            ArrowDecoration::Circle
        );
    }

    #[test]
    fn body_color_inclination() {
        let n = ArrowConfiguration::with_direction_normal();
        assert!(n.with_body(ArrowBody::Dotted).is_dotted());
        assert!(n.with_body(ArrowBody::Hidden).is_hidden());
        assert!(n.with_color(HColor::parse("red")).color().is_some());
        let i = n.with_inclination(3);
        assert_eq!((i.inclination1(), i.inclination2()), (0, 3));
        let j = ArrowConfiguration::with_direction_reverse().with_inclination(2);
        assert_eq!((j.inclination1(), j.inclination2()), (2, 2));
        assert_eq!(n.with_thickness(2.0), n.with_thickness(2.0));
    }

    #[test]
    fn stroke_application() {
        use crate::klimt::drawing::svg::SvgOption;
        use crate::klimt::drawing::ugraphic::{render_svg, ULine};
        use crate::klimt::font::EmbeddedFontBounder;
        use crate::style::{SName, StyleBuilder, StyleQuery};
        use std::rc::Rc;
        let style = StyleBuilder::default_skin()
            .merged_style(&StyleQuery::of(&[SName::Root]))
            .unwrap();
        let n = ArrowConfiguration::with_direction_normal();
        let out = render_svg(
            SvgOption::default(),
            0,
            Rc::new(EmbeddedFontBounder),
            |ug| {
                let ug = ug.apply_color(crate::klimt::color::HColor::BLACK);
                n.with_body(ArrowBody::Dotted)
                    .apply_stroke(&ug, &style)
                    .draw(ULine::hline(10.0));
                n.apply_thickness_only(&ug).draw(ULine::hline(5.0));
                n.apply_stroke(&ug, &style).draw(ULine::hline(7.0));
            },
        );
        assert!(out.contains("stroke-dasharray:2,2"), "{out}");
    }
}
