//! Reading of the source before the preprocessor: line splitting, comment headers,
//! backslash continuation and blocks (`ReadLineReader`, `UncommentReadLine`,
//! `ReadFilterMergeLines`, `BlockUmlBuilder`, `StartUtils`).

use crate::text::{LineLocation, StringLocated};

/// Lines of `content`, numbered from 0 (`ReadLineReader`).
pub fn read_lines(content: &str) -> Vec<StringLocated> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = content.chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        any = true;
        match c {
            '\n' => lines.push(std::mem::take(&mut cur)),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if any && !cur.is_empty() {
        lines.push(cur);
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            StringLocated::new(
                l.replace('\u{2013}', "-"),
                Some(LineLocation { position: i }),
            )
        })
        .collect()
}

fn starts_with_directive_keyword(text: &str, from: usize, keyword: &str) -> bool {
    let rest = &text[from..];
    let t = rest.trim_start_matches(char::is_whitespace);
    let Some(after) = t.strip_prefix(['@', '\\']) else {
        return false;
    };
    after.starts_with(keyword)
}

pub fn is_start_directive(s: &str) -> bool {
    let t = s.trim_start_matches(char::is_whitespace);
    let Some(after) = t.strip_prefix(['@', '\\']) else {
        return false;
    };
    after.chars().count() > 5 && after.starts_with("start")
}

pub fn is_end_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, 0, "end")
}

pub fn is_pause_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, 0, "pause")
}

pub fn is_unpause_directive(s: &str) -> bool {
    starts_with_directive_keyword(s, 0, "unpause")
}

pub fn is_exit(s: &str) -> bool {
    s.trim() == "!exit"
}

/// Text before `@start` when it only holds a comment-like prefix (`beforeStartUml`).
fn before_start_uml(s: &str) -> Option<String> {
    let mut inside = false;
    for (i, c) in s.char_indices() {
        if starts_with_directive_keyword(s, i, "start") {
            return Some(s[..i].to_string());
        }
        if inside {
            if c == '>' {
                inside = false;
            }
            continue;
        }
        if c == '<' {
            inside = true;
        } else if c == '~' || c.is_alphanumeric() || c == '_' {
            return None;
        }
    }
    None
}

fn ends_with_backslash(s: &str) -> bool {
    s.ends_with('\\') && !s.ends_with("\\\\")
}

/// Skips comment lines and strips inner comments (`ReadFilterQuoteComment`).
fn next_without_comment(lines: &[StringLocated], pos: &mut usize) -> Option<StringLocated> {
    let mut long_comment = false;
    while *pos < lines.len() {
        let result = &lines[*pos];
        *pos += 1;
        let trim = result.string().replace('\t', " ");
        let trim = trim.trim();
        if long_comment && trim.ends_with("'/") {
            long_comment = false;
            continue;
        }
        if long_comment || trim.starts_with('\'') {
            continue;
        }
        if trim.starts_with("/'") && trim.ends_with("'/") {
            continue;
        }
        if trim.starts_with("/'") && !trim.contains("'/") {
            long_comment = true;
            continue;
        }
        return Some(crate::tim::iterator::remove_inner_comment(result));
    }
    None
}

/// Reads lines one by one with comment-prefix removal and continuation lines.
struct SourceReader {
    raw: Vec<StringLocated>,
    pos: usize,
    header_to_remove: Option<String>,
    paused: bool,
    manage_ending_backslash: bool,
}

impl SourceReader {
    fn read_uncommented(&mut self) -> Option<StringLocated> {
        let result = self.raw.get(self.pos)?.clone();
        self.pos += 1;
        let line = result.string();
        if let Some(tmp) = before_start_uml(line) {
            self.header_to_remove = Some(tmp);
        }
        if self.paused {
            let re = fancy_regex::Regex::new(r"^((?:\W|<[^<>]*>)*)[@\\]unpause").expect("valid");
            if let Ok(Some(c)) = re.captures(line) {
                self.header_to_remove = Some(c[1].to_string());
            }
        }
        if let Some(h) = &self.header_to_remove {
            if h.starts_with(line) {
                return Some(StringLocated::new("", result.location()));
            }
            if line.starts_with(h.as_str()) {
                return Some(result.substring(h.len()));
            }
        }
        Some(result)
    }

    fn read_line(&mut self) -> Option<StringLocated> {
        let mut result = self.read_uncommented()?;
        if is_start_directive(result.string()) && is_ditaa(result.string()) {
            self.manage_ending_backslash = false;
        }
        if is_end_directive(result.string()) {
            self.manage_ending_backslash = true;
        }
        while self.manage_ending_backslash && ends_with_backslash(result.string()) {
            // continuation lines come from the uncommented stream without comment lines
            let mut tmp: Vec<StringLocated> = Vec::new();
            let saved_pos = self.pos;
            while let Some(l) = self.read_uncommented() {
                tmp.push(l);
            }
            self.pos = saved_pos;
            let mut p = 0;
            let next = next_without_comment(&tmp, &mut p);
            self.pos = saved_pos + p;
            match next {
                None => break,
                Some(n) => {
                    let merged = format!(
                        "{}{}",
                        &result.string()[..result.string().len() - 1],
                        n.string()
                    );
                    result = StringLocated::new(merged, result.location());
                }
            }
        }
        Some(result)
    }
}

fn is_ditaa(s: &str) -> bool {
    s.trim().to_lowercase().starts_with("@startditaa")
        || s.trim().to_lowercase().starts_with("\\startditaa")
}

/// Splits the source into diagram blocks (`BlockUmlBuilder`).
pub fn split_blocks(text: &str, config: &[String]) -> Vec<Vec<StringLocated>> {
    let raw = read_lines(text);
    let mut reader = SourceReader {
        raw,
        pos: 0,
        header_to_remove: None,
        paused: false,
        manage_ending_backslash: true,
    };
    let mut pending: std::collections::VecDeque<StringLocated> = std::collections::VecDeque::new();
    let mut blocks: Vec<Vec<StringLocated>> = Vec::new();
    let mut current: Option<Vec<StringLocated>> = None;
    let append_re = regex::Regex::new(r"(?i)^\W*[@\\](append|a)\b").expect("valid");
    let mut paused = false;
    loop {
        let s = if let Some(c) = pending.pop_front() {
            c
        } else if let Some(s) = reader.read_line() {
            if is_start_directive(s.string()) && !config.is_empty() {
                let cfg: Vec<StringLocated> = config
                    .iter()
                    .map(|c| StringLocated::new(c.as_str(), s.location()))
                    .collect();
                let mut pos = 0;
                while let Some(l) = next_without_comment(&cfg, &mut pos) {
                    pending.push_back(l);
                }
            }
            s
        } else {
            break;
        };
        if is_start_directive(s.string()) {
            current = Some(Vec::new());
            paused = false;
        }
        if is_pause_directive(s.string()) {
            paused = true;
            reader.paused = true;
        }
        if is_exit(s.string()) {
            paused = true;
            reader.paused = true;
        }
        if let (Some(cur), false) = (&mut current, paused) {
            cur.push(s.clone());
        } else if paused {
            if let (Some(m), Some(cur)) = (append_re.find(s.string()), &mut current) {
                cur.push(StringLocated::new(
                    crate::string_utils::trin(&s.string()[m.end()..]),
                    s.location(),
                ));
            }
        }
        if is_unpause_directive(s.string()) {
            paused = false;
            reader.paused = false;
        }
        if is_end_directive(s.string()) && current.is_some() {
            if paused {
                if let Some(cur) = &mut current {
                    cur.push(s.clone());
                }
            }
            blocks.push(current.take().expect("block"));
            reader.paused = false;
        }
    }
    blocks
}

/// Removes a leading `---` YAML header after the start line (`removeYamlHeader`).
pub fn remove_yaml_header_lines(input: Vec<StringLocated>) -> Vec<StringLocated> {
    let is_sep = |l: &StringLocated| l.string() == "---";
    if input.len() > 1 && is_sep(&input[1]) {
        for i in 2..input.len() {
            if is_sep(&input[i]) {
                let mut result = vec![input[0].clone()];
                result.extend(input[i + 1..].iter().cloned());
                return result;
            }
        }
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(b: &[StringLocated]) -> Vec<String> {
        b.iter().map(|l| l.string().to_string()).collect()
    }

    #[test]
    fn lines_and_blocks() {
        let l = read_lines("a\r\nb\rc\n\nd");
        assert_eq!(strs(&l), ["a", "b", "c", "", "d"]);
        assert_eq!(l[4].location().unwrap().position, 4);
        let blocks = split_blocks(
            "x\n@startuml\nA -> B\n@enduml\n@startuml\nC\n@enduml\n",
            &[],
        );
        assert_eq!(blocks.len(), 2);
        assert_eq!(strs(&blocks[0]), ["@startuml", "A -> B", "@enduml"]);
        assert_eq!(blocks[1][1].location().unwrap().position, 5);
    }

    #[test]
    fn continuation_and_config() {
        let blocks = split_blocks(
            "@startuml\nA ->\\\n B\n@enduml\n",
            &["skinparam x 1".to_string()],
        );
        assert_eq!(
            strs(&blocks[0]),
            ["@startuml", "skinparam x 1", "A -> B", "@enduml"]
        );
    }

    #[test]
    fn pause() {
        let blocks = split_blocks("@startuml\nA\n@pause\nB\n@unpause\nC\n@enduml\n", &[]);
        assert_eq!(strs(&blocks[0]), ["@startuml", "A", "C", "@enduml"]);
    }

    #[test]
    fn yaml_header() {
        let l = read_lines("@startuml\n---\nk: v\n---\nA\n@enduml");
        assert_eq!(
            strs(&remove_yaml_header_lines(l)),
            ["@startuml", "A", "@enduml"]
        );
    }

    #[test]
    fn comment_prefix() {
        let blocks = split_blocks("' @startuml\n' A -> B\n' @enduml\n", &[]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0][1].string(), " A -> B");
    }
}
