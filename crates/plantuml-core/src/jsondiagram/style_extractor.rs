//! Picks the settings out of the source lines of a data diagram (`StyleExtractor`).

use crate::skin::SkinParam;

#[derive(Debug, Default)]
pub struct StyleExtractor {
    list: Vec<String>,
    style: Vec<String>,
    title: Option<String>,
    handwritten: bool,
    scale: Option<String>,
    new_skin: Option<String>,
}

impl StyleExtractor {
    pub fn new(lines: &[String]) -> Self {
        let mut r = StyleExtractor::default();
        let mut it = lines.iter();
        while let Some(line) = it.next() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            let early = r.list.len() <= 1;
            if s == "<style>" {
                for l in it.by_ref() {
                    if l.trim() == "</style>" {
                        break;
                    }
                    r.style.push(l.clone());
                }
            } else if early
                && (s.starts_with("!assume ")
                    || s.starts_with("!pragma ")
                    || s.starts_with("hide "))
            {
            } else if early && s.starts_with("scale ") {
                r.scale = Some(s.to_string());
            } else if early && s.starts_with("title ") {
                r.title = Some(s["title ".len()..].trim().to_string());
            } else if early && s.starts_with("skin ") {
                r.new_skin = Some(s["skin ".len()..].trim().to_string());
            } else if early && s.starts_with("skinparam ") {
                if s.contains("handwritten") && s.contains("true") {
                    r.handwritten = true;
                }
                if s.contains('{') {
                    for l in it.by_ref() {
                        if l.trim() == "}" {
                            break;
                        }
                    }
                }
            } else {
                r.list.push(line.clone());
            }
        }
        r
    }

    /// The remaining lines, `@start` and `@end` lines included.
    pub fn lines(&self) -> &[String] {
        &self.list
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn is_handwritten(&self) -> bool {
        self.handwritten
    }

    pub fn scale(&self) -> Option<&str> {
        self.scale.as_deref()
    }

    /// Loads the requested skin and the `<style>` blocks (`applyStyles`).
    pub fn apply_styles(&self, skin: &mut SkinParam) -> Result<(), String> {
        if let Some(name) = &self.new_skin {
            skin.set_default_skin(name)
                .map_err(|_| format!("Cannot find style {name}"))?;
        }
        if !self.style.is_empty() {
            let mut builder = skin.style_builder();
            let styles = builder
                .parse_style_text(&self.style.join("\n"))
                .map_err(|e| e.to_string())?;
            skin.mute_style(styles);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn extracts_settings() {
        let e = StyleExtractor::new(&lines(
            "@startjson\n\n!pragma x\nhide a\nscale 2\ntitle  My title \nskin rose\n\
             skinparam handwritten true\nskinparam foo {\na b\n}\n<style>\nnode {\n}\n</style>\n{}\ntitle late\n@endjson",
        ));
        assert_eq!(e.title(), Some("My title"));
        assert_eq!(e.scale(), Some("scale 2"));
        assert!(e.is_handwritten());
        assert_eq!(e.lines(), ["@startjson", "{}", "title late", "@endjson"]);
        let mut skin = SkinParam::new();
        e.apply_styles(&mut skin).unwrap();
    }

    #[test]
    fn bad_skin_and_style() {
        let e = StyleExtractor::new(&lines("@startjson\nskin nosuch\n@endjson"));
        let mut skin = SkinParam::new();
        assert_eq!(
            e.apply_styles(&mut skin),
            Err("Cannot find style nosuch".to_string())
        );
        let e = StyleExtractor::new(&lines("@startjson\n<style>\n</style>\n@endjson"));
        assert!(e.apply_styles(&mut SkinParam::new()).is_ok());
    }
}
