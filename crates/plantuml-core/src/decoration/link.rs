//! Link ends and styles (`LinkDecor`, `LinkMiddleDecor`, `LinkType`).

use crate::klimt::color::colors::LinkStyle;
use crate::klimt::drawing::ugraphic::UStroke;

/// Decoration at one end of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkDecor {
    None,
    Extends,
    Composition,
    Aggregation,
    NotNavigable,
    Redefines,
    DefinedBy,
    Crowfoot,
    CircleCrowfoot,
    CircleLine,
    DoubleLine,
    LineCrowfoot,
    Arrow,
    ArrowTriangle,
    ArrowAndCircle,
    Circle,
    CircleFill,
    CircleConnect,
    Parenthesis,
    Square,
    CircleCross,
    Plus,
    HalfArrowUp,
    HalfArrowDown,
    SquareToBeRemoved,
}

const ALL: [LinkDecor; 25] = [
    LinkDecor::None,
    LinkDecor::Extends,
    LinkDecor::Composition,
    LinkDecor::Aggregation,
    LinkDecor::NotNavigable,
    LinkDecor::Redefines,
    LinkDecor::DefinedBy,
    LinkDecor::Crowfoot,
    LinkDecor::CircleCrowfoot,
    LinkDecor::CircleLine,
    LinkDecor::DoubleLine,
    LinkDecor::LineCrowfoot,
    LinkDecor::Arrow,
    LinkDecor::ArrowTriangle,
    LinkDecor::ArrowAndCircle,
    LinkDecor::Circle,
    LinkDecor::CircleFill,
    LinkDecor::CircleConnect,
    LinkDecor::Parenthesis,
    LinkDecor::Square,
    LinkDecor::CircleCross,
    LinkDecor::Plus,
    LinkDecor::HalfArrowUp,
    LinkDecor::HalfArrowDown,
    LinkDecor::SquareToBeRemoved,
];

impl LinkDecor {
    /// Keys that stand for this decor at the start of an arrow.
    fn decors1(self) -> &'static [&'static str] {
        use LinkDecor::*;
        match self {
            Extends => &["<|", "^"],
            Composition => &["*"],
            Aggregation => &["o"],
            NotNavigable => &["x"],
            Redefines => &["<||"],
            DefinedBy => &["<|:"],
            Crowfoot => &["}"],
            CircleCrowfoot => &["}o"],
            CircleLine => &["|o"],
            DoubleLine => &["||"],
            LineCrowfoot => &["}|"],
            Arrow => &["<", "<_"],
            ArrowTriangle => &["<<"],
            Circle => &["0"],
            CircleFill => &["@"],
            CircleConnect => &["0)"],
            Parenthesis => &[")"],
            Square => &["#"],
            Plus => &["+"],
            _ => &[],
        }
    }

    /// Keys that stand for this decor at the end of an arrow.
    fn decors2(self) -> &'static [&'static str] {
        use LinkDecor::*;
        match self {
            Extends => &["|>", "^"],
            Composition => &["*"],
            Aggregation => &["o"],
            NotNavigable => &["x"],
            Redefines => &["||>"],
            DefinedBy => &[":|>"],
            Crowfoot => &["{"],
            CircleCrowfoot => &["o{"],
            CircleLine => &["o|"],
            DoubleLine => &["||"],
            LineCrowfoot => &["|{"],
            Arrow => &[">", "_>"],
            ArrowTriangle => &[">>"],
            Circle => &["0"],
            CircleFill => &["@"],
            CircleConnect => &["(0"],
            Parenthesis => &["("],
            Square => &["#"],
            Plus => &["+"],
            HalfArrowUp => &["\\\\"],
            HalfArrowDown => &["//"],
            _ => &[],
        }
    }

    pub fn margin(self) -> i32 {
        use LinkDecor::*;
        match self {
            None => 2,
            Extends | Redefines | DefinedBy | SquareToBeRemoved => 30,
            Composition | Aggregation => 15,
            NotNavigable => 1,
            Crowfoot => 10,
            CircleCrowfoot => 14,
            CircleLine => 10,
            DoubleLine => 7,
            LineCrowfoot => 10,
            Arrow | ArrowTriangle | ArrowAndCircle => 10,
            _ => 0,
        }
    }

    pub fn is_fill(self) -> bool {
        use LinkDecor::*;
        matches!(self, Composition | Crowfoot | Arrow | ArrowTriangle)
    }

    pub fn arrow_size(self) -> f64 {
        use LinkDecor::*;
        match self {
            None | SquareToBeRemoved => 0.0,
            Extends | Redefines | DefinedBy => 2.0,
            Composition | Aggregation => 1.3,
            NotNavigable => 0.5,
            Crowfoot | CircleCrowfoot | CircleLine | LineCrowfoot | ArrowTriangle => 0.8,
            DoubleLine => 0.7,
            Arrow | ArrowAndCircle | Circle | CircleFill | CircleConnect | Square | CircleCross => {
                0.5
            }
            Parenthesis => 1.0,
            Plus | HalfArrowUp | HalfArrowDown => 1.5,
        }
    }

    pub fn is_extends_like(self) -> bool {
        matches!(
            self,
            LinkDecor::Extends | LinkDecor::Redefines | LinkDecor::DefinedBy
        )
    }

    /// `lookupDecors1`.
    pub fn lookup1(s: Option<&str>) -> LinkDecor {
        let Some(s) = s else { return LinkDecor::None };
        let s = crate::string_utils::trin(s);
        ALL.iter()
            .copied()
            .find(|d| d.decors1().contains(&s))
            .unwrap_or(LinkDecor::None)
    }

    /// `lookupDecors2`.
    pub fn lookup2(s: Option<&str>) -> LinkDecor {
        let Some(s) = s else { return LinkDecor::None };
        let s = crate::string_utils::trin(s);
        ALL.iter()
            .copied()
            .find(|d| d.decors2().contains(&s))
            .unwrap_or(LinkDecor::None)
    }

    /// `getRegexDecors1`.
    pub fn regex_decors1() -> String {
        build_regex(
            ALL.iter()
                .flat_map(|d| d.decors1().iter().copied())
                .collect(),
        )
    }

    /// `getRegexDecors2`.
    pub fn regex_decors2() -> String {
        build_regex(
            ALL.iter()
                .flat_map(|d| d.decors2().iter().copied())
                .collect(),
        )
    }
}

fn build_regex(mut keys: Vec<&'static str>) -> String {
    keys.sort();
    keys.dedup();
    keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
    let parts: Vec<String> = keys
        .iter()
        .map(|key| {
            let quoted = fancy_regex::escape(key).into_owned();
            let starts = key.starts_with('o');
            let ends = key.ends_with('o');
            if starts && ends {
                format!("\\b{quoted}\\b")
            } else if starts {
                format!("\\b{quoted}")
            } else if ends {
                format!("{quoted}\\b")
            } else {
                quoted
            }
        })
        .collect();
    format!("({})?", parts.join("|"))
}

/// Decoration in the middle of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkMiddleDecor {
    None,
    Circle,
    CircleCircled,
    CircleCircled1,
    CircleCircled2,
    Subset,
    Superset,
}

impl LinkMiddleDecor {
    pub fn inversed(self) -> Self {
        match self {
            LinkMiddleDecor::CircleCircled1 => LinkMiddleDecor::CircleCircled2,
            LinkMiddleDecor::CircleCircled2 => LinkMiddleDecor::CircleCircled1,
            o => o,
        }
    }
}

/// Both ends, the line style and the middle of a link.
#[derive(Debug, Clone, Copy)]
pub struct LinkType {
    decor1: LinkDecor,
    decor2: LinkDecor,
    middle: LinkMiddleDecor,
    style: LinkStyle,
}

impl PartialEq for LinkType {
    fn eq(&self, o: &Self) -> bool {
        self.decor1 == o.decor1 && self.decor2 == o.decor2 && self.style == o.style
    }
}

impl LinkType {
    pub fn new(decor1: LinkDecor, decor2: LinkDecor) -> Self {
        Self {
            decor1,
            decor2,
            middle: LinkMiddleDecor::None,
            style: LinkStyle::NORMAL,
        }
    }

    pub fn decor1(&self) -> LinkDecor {
        self.decor1
    }

    pub fn decor2(&self) -> LinkDecor {
        self.decor2
    }

    pub fn style(&self) -> LinkStyle {
        self.style
    }

    pub fn middle_decor(&self) -> LinkMiddleDecor {
        self.middle
    }

    pub fn is_double_decorated(&self) -> bool {
        self.decor1 != LinkDecor::None && self.decor2 != LinkDecor::None
    }

    pub fn looks_like_reverted_for_svg(&self) -> bool {
        self.decor1 == LinkDecor::None && self.decor2 != LinkDecor::None
    }

    pub fn looks_like_no_decor_at_all_svg(&self) -> bool {
        (self.decor1 == LinkDecor::None && self.decor2 == LinkDecor::None)
            || (self.decor1 != LinkDecor::None && self.decor2 != LinkDecor::None)
    }

    pub fn without_decors1(&self) -> Self {
        Self {
            decor1: LinkDecor::None,
            ..*self
        }
    }

    pub fn without_decors2(&self) -> Self {
        Self {
            decor2: LinkDecor::None,
            ..*self
        }
    }

    pub fn is_invisible(&self) -> bool {
        self.style.is_invisible()
    }

    pub fn go_dashed(&self) -> Self {
        Self {
            style: LinkStyle::DASHED,
            ..*self
        }
    }

    pub fn go_dotted(&self) -> Self {
        Self {
            style: LinkStyle::DOTTED,
            ..*self
        }
    }

    pub fn go_thickness(&self, thickness: f64) -> Self {
        Self {
            style: self.style.go_thickness(thickness),
            ..*self
        }
    }

    pub fn go_bold(&self) -> Self {
        Self {
            style: LinkStyle::BOLD,
            ..*self
        }
    }

    pub fn inversed(&self) -> Self {
        Self {
            decor1: self.decor2,
            decor2: self.decor1,
            middle: self.middle.inversed(),
            style: self.style,
        }
    }

    pub fn with_middle(&self, middle: LinkMiddleDecor) -> Self {
        Self { middle, ..*self }
    }

    pub fn invisible(&self) -> Self {
        Self {
            style: LinkStyle::INVISIBLE,
            ..*self
        }
    }

    pub fn is_extends(&self) -> bool {
        self.decor1 == LinkDecor::Extends || self.decor2 == LinkDecor::Extends
    }

    pub fn part1(&self) -> Self {
        Self {
            decor2: LinkDecor::None,
            ..*self
        }
    }

    pub fn part2(&self) -> Self {
        Self {
            decor1: LinkDecor::None,
            ..*self
        }
    }

    /// `getSpecificDecorationSvek` for the simplest strategy: the DOT edge has no arrow heads.
    pub fn specific_decoration_svek(&self) -> &'static str {
        "arrowtail=none,arrowhead=none"
    }

    pub fn stroke3(&self, default_thickness: Option<UStroke>) -> UStroke {
        if self.style.is_thickness_overridden() {
            return self.style.stroke();
        }
        let Some(d) = default_thickness else {
            return self.style.stroke();
        };
        if d.dash_visible == 0.0 && d.dash_space == 0.0 {
            return self.style.go_thickness(d.thickness).stroke();
        }
        d
    }

    pub fn with_lollipop_interface_eye2(&self) -> Self {
        self.without_decors1()
    }

    pub fn with_lollipop_interface_eye1(&self) -> Self {
        self.without_decors2()
    }

    fn has(&self, d: LinkDecor) -> bool {
        self.decor1 == d || self.decor2 == d
    }

    fn has_any(&self, ds: &[LinkDecor]) -> bool {
        ds.iter().any(|d| self.has(*d))
    }

    /// Value of `data-link-type`.
    pub fn link_type_name(&self) -> Option<&'static str> {
        use LinkDecor::*;
        if self.has(Composition) {
            return Some("composition");
        }
        if self.has(Aggregation) {
            return Some("aggregation");
        }
        if self.has(Extends) {
            return Some("extension");
        }
        if self.has(Redefines) {
            return Some("redefines");
        }
        if self.has(DefinedBy) {
            return Some("definedby");
        }
        if self.has_any(&[Arrow, ArrowTriangle]) {
            return Some("dependency");
        }
        if self.has(NotNavigable) {
            return Some("not_navigable");
        }
        if self.has_any(&[Crowfoot, CircleCrowfoot, LineCrowfoot]) {
            return Some("crowfoot");
        }
        if self.has_any(&[CircleLine, DoubleLine]) || (self.decor1 == None && self.decor2 == None) {
            return Some("association");
        }
        if self.has(Plus) {
            return Some("nested");
        }
        Option::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_and_regex() {
        assert_eq!(LinkDecor::lookup1(Some("<|")), LinkDecor::Extends);
        assert_eq!(LinkDecor::lookup2(Some("|>")), LinkDecor::Extends);
        assert_eq!(LinkDecor::lookup2(Some(">")), LinkDecor::Arrow);
        assert_eq!(LinkDecor::lookup1(None), LinkDecor::None);
        let r = LinkDecor::regex_decors2();
        assert!(r.starts_with('(') && r.ends_with(")?"));
        assert!(r.contains("\\bo\\b"));
    }

    #[test]
    fn type_names() {
        let t = LinkType::new(LinkDecor::None, LinkDecor::Arrow);
        assert_eq!(t.link_type_name(), Some("dependency"));
        assert!(t.looks_like_reverted_for_svg());
        assert_eq!(t.inversed().decor1(), LinkDecor::Arrow);
        assert_eq!(
            LinkType::new(LinkDecor::None, LinkDecor::None).link_type_name(),
            Some("association")
        );
    }
}
