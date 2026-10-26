//! Arrow colors and what is written on the next arrow (`Rainbow`, `HtmlColorAndStyle`, `LinkRendering`).

use crate::klimt::color::colors::LinkStyle;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::skin::SkinParam;
use crate::style::{queries, PName, Style, Value};

/// One color of a (possibly multi-colored) arrow with its line style.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlColorAndStyle {
    pub arrow_color: HColor,
    pub arrow_head_color: HColor,
    pub style: LinkStyle,
}

impl HtmlColorAndStyle {
    pub fn new(arrow_color: HColor, style: LinkStyle, arrow_head_color: Option<HColor>) -> Self {
        Self {
            arrow_head_color: arrow_head_color.unwrap_or_else(|| arrow_color.clone()),
            arrow_color,
            style,
        }
    }

    /// `#red,dashed` (`HtmlColorAndStyle.build`): a color and a line style, both optional.
    pub fn build(skin: &SkinParam, definition: &str) -> Option<Self> {
        let style = skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ARROW);
        let mut arrow_color = style.map_or(HColor::BLACK, |s| s.value(PName::LineColor).as_color());
        let mut link_style = LinkStyle::NORMAL;
        for s in definition.split(',') {
            let tmp = LinkStyle::parse_or_normal(s);
            if !tmp.is_normal() {
                link_style = tmp;
                continue;
            }
            if let Some(c) = HColor::parse(s) {
                arrow_color = c;
            }
        }
        Some(Self::new(arrow_color, link_style, None))
    }
}

/// The colors of an arrow, usually only one (`Rainbow`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rainbow {
    colors: Vec<HtmlColorAndStyle>,
    color_arrow_separation_space: i32,
}

impl Rainbow {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_color(arrow_color: Option<HColor>, arrow_head_color: Option<HColor>) -> Self {
        match arrow_color {
            None => Self::none(),
            Some(c) => Self::of(HtmlColorAndStyle::new(
                c,
                LinkStyle::NORMAL,
                arrow_head_color,
            )),
        }
    }

    pub fn of(color: HtmlColorAndStyle) -> Self {
        Self {
            colors: vec![color],
            color_arrow_separation_space: 0,
        }
    }

    /// Line and head colors of an arrow style (`Rainbow.build(Style, HColorSet)`).
    pub fn from_style(style: &Style) -> Self {
        let color = style.value(PName::LineColor).as_color();
        let head = match style.value(PName::HeadColor) {
            Value::Null => color.clone(),
            v => v.as_color(),
        };
        Self::from_color(Some(color), Some(head))
    }

    /// `;`-separated list of colors, as in `-[#red;#blue]->`.
    pub fn build(skin: &SkinParam, color_string: &str, separation_space: i32) -> Self {
        let colors = color_string
            .split(';')
            .filter_map(|s| HtmlColorAndStyle::build(skin, s))
            .collect();
        Self {
            colors,
            color_arrow_separation_space: separation_space,
        }
    }

    pub fn size(&self) -> usize {
        self.colors.len()
    }

    pub fn colors(&self) -> &[HtmlColorAndStyle] {
        &self.colors
    }

    pub fn is_invisible(&self) -> bool {
        self.colors.iter().any(|c| c.style.is_invisible())
    }

    pub fn color_arrow_separation_space(&self) -> i32 {
        self.color_arrow_separation_space
    }

    pub fn with_default(self, default: Rainbow) -> Rainbow {
        if self.size() == 0 {
            default
        } else {
            self
        }
    }
}

/// Color and text of the arrow entering an instruction (`LinkRendering`).
#[derive(Debug, Clone)]
pub struct LinkRendering {
    rainbow: Rainbow,
    display: Display,
}

impl LinkRendering {
    pub fn create(rainbow: Rainbow) -> Self {
        Self {
            rainbow,
            display: Display::null(),
        }
    }

    pub fn none() -> Self {
        Self::create(Rainbow::none())
    }

    pub fn with_rainbow(&self, rainbow: Rainbow) -> Self {
        Self {
            rainbow,
            display: self.display.clone(),
        }
    }

    pub fn with_display(&self, display: Display) -> Self {
        Self {
            rainbow: self.rainbow.clone(),
            display,
        }
    }

    pub fn display(&self) -> &Display {
        &self.display
    }

    pub fn rainbow(&self) -> &Rainbow {
        &self.rainbow
    }

    /// The own colors, or `default` when none was given.
    pub fn rainbow_or(&self, default: &Rainbow) -> Rainbow {
        if self.rainbow.size() == 0 {
            default.clone()
        } else {
            self.rainbow.clone()
        }
    }

    pub fn is_none(&self) -> bool {
        self.display.is_null() && self.rainbow.size() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_rendering_none_until_something_is_set() {
        let none = LinkRendering::none();
        assert!(none.is_none());
        let labelled = none.with_display(Display::create(&["go"]));
        assert!(!labelled.is_none());
        let red = HColor::parse("#red").unwrap();
        let colored = none.with_rainbow(Rainbow::from_color(Some(red.clone()), None));
        assert!(!colored.is_none());
        assert_eq!(colored.rainbow_or(&Rainbow::none()).size(), 1);
        assert_eq!(none.rainbow_or(&colored.rainbow().clone()).size(), 1);
    }

    #[test]
    fn rainbow_from_definition() {
        let skin = SkinParam::new();
        let r = Rainbow::build(&skin, "#red;#blue,dashed", 3);
        assert_eq!(r.size(), 2);
        assert_eq!(r.color_arrow_separation_space(), 3);
        assert!(!r.is_invisible());
        assert_eq!(r.colors()[0].arrow_color, HColor::parse("red").unwrap());
        assert!(!r.colors()[1].style.is_normal());
        assert!(Rainbow::build(&skin, "hidden", 0).is_invisible());
        assert_eq!(Rainbow::none().with_default(r.clone()).size(), 2);
        assert_eq!(Rainbow::from_color(None, None).size(), 0);
    }

    #[test]
    fn rainbow_from_style() {
        let skin = SkinParam::new();
        let style = skin
            .style_builder()
            .merged_style(&queries::ACTIVITYDIAG_ARROW)
            .unwrap();
        let r = Rainbow::from_style(&style);
        assert_eq!(r.size(), 1);
        assert_eq!(r.colors()[0].arrow_head_color, r.colors()[0].arrow_color);
    }
}
