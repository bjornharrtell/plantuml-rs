//! Color settings inside command syntax (`ColorParser`).

use super::colors::{ColorType, Colors};
use crate::regex::{IRegex, RegexResult};

const COLOR_REGEXP: &str = r"#\w+[-\\|/]?\w+";
const PART2: &str = r"#(?:\w+[-\\|/]?\w+;)?(?:(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?(?:;|(?![\w;:.])))+";

fn colors_regexp() -> String {
    format!("(?:{PART2})|(?:{COLOR_REGEXP})")
}

/// A color group of a command regex and how to read it back.
pub struct ColorParser {
    name: String,
    main_type: Option<ColorType>,
}

impl ColorParser {
    pub fn simple_color(main_type: ColorType) -> ColorParser {
        Self::simple_color_id(main_type, "COLOR")
    }

    pub fn simple_color_id(main_type: ColorType, id: &str) -> ColorParser {
        ColorParser {
            name: id.to_string(),
            main_type: Some(main_type),
        }
    }

    /// The optional color group (`#back;line:red`).
    pub fn regex(&self) -> IRegex {
        IRegex::named(&self.name, &format!("({})?", colors_regexp()))
    }

    /// Reads the settings captured by this parser; `None` when a color name is unknown.
    pub fn colors(&self, arg: &RegexResult) -> Option<Colors> {
        let main = self.main_type.expect("main type");
        match arg.get_lazzy(&self.name, 0) {
            None => Some(Colors::empty()),
            Some(data) => Colors::parse(data, main),
        }
    }
}

/// `ColorParser.exp1`: an optional color group named `COLOR`.
pub fn exp1() -> IRegex {
    IRegex::named("COLOR", &format!("({})?", colors_regexp()))
}

/// Optional single color named `BACKCOLOR`.
pub fn exp2() -> IRegex {
    IRegex::named("BACKCOLOR", &format!("({COLOR_REGEXP})?"))
}

pub fn exp3() -> IRegex {
    IRegex::named("BACKCOLOR2", &format!("({COLOR_REGEXP})?"))
}

pub fn exp4() -> IRegex {
    IRegex::named("COLOR", &format!("(?:({COLOR_REGEXP}):)?"))
}

pub fn exp6() -> IRegex {
    IRegex::named("COLOR", &format!("(?:({COLOR_REGEXP})\\|)?"))
}

pub fn exp7() -> IRegex {
    IRegex::named("COLOR", &format!("(?:({COLOR_REGEXP}))?"))
}

/// A mandatory color group named `COLOR`.
pub fn mandatory_color() -> IRegex {
    IRegex::named("COLOR", &format!("({})", colors_regexp()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rx(part: IRegex) -> IRegex {
        IRegex::concat(vec![
            IRegex::start(),
            IRegex::leaf("x"),
            part,
            IRegex::end(),
        ])
    }

    #[test]
    fn exp1_reads_colors() {
        let r = rx(exp1());
        let m = r.matcher("x#red").unwrap();
        assert_eq!(m.get("COLOR", 0), Some("#red"));
        let p = ColorParser::simple_color(ColorType::Back);
        let c = p.colors(&m).unwrap();
        assert!(c.color(ColorType::Back).is_some());
        let m = r.matcher("x#back:red;line:blue;").unwrap();
        let c = p.colors(&m).unwrap();
        assert!(c.color(ColorType::Line).is_some());
        assert!(p.colors(&r.matcher("x").unwrap()).unwrap().is_empty());
        assert!(p.regex().pattern().contains("COLOR") || !p.regex().pattern().is_empty());
        assert!(
            ColorParser::simple_color_id(ColorType::Line, "C2")
                .regex()
                .pattern()
                .len()
                > 5
        );
    }

    #[test]
    fn other_forms() {
        assert_eq!(
            rx(exp2()).matcher("x#red").unwrap().get("BACKCOLOR", 0),
            Some("#red")
        );
        assert!(rx(exp3())
            .matcher("x#red")
            .unwrap()
            .get("BACKCOLOR2", 0)
            .is_some());
        assert_eq!(
            rx(exp4()).matcher("x#red:").unwrap().get("COLOR", 0),
            Some("#red")
        );
        assert_eq!(
            rx(exp6()).matcher("x#red|").unwrap().get("COLOR", 0),
            Some("#red")
        );
        assert_eq!(
            rx(exp7()).matcher("x#red").unwrap().get("COLOR", 0),
            Some("#red")
        );
        assert_eq!(
            rx(mandatory_color())
                .matcher("x#red")
                .unwrap()
                .get("COLOR", 0),
            Some("#red")
        );
        assert!(rx(mandatory_color()).matcher("x").is_none());
    }
}
