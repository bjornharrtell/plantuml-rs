//! Source lines and groups of lines, ported from `text/StringLocated` and `utils/BlocLines`.

/// Where a line comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineLocation {
    /// 1-based position in the source.
    pub position: usize,
}

/// One source line with its location (`StringLocated`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLocated {
    s: String,
    location: Option<LineLocation>,
}

/// Whitespace trimmed by PlantUML (`StringUtils.trin`).
fn is_trimmable(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n'
}

impl StringLocated {
    pub fn new(s: impl Into<String>, location: Option<LineLocation>) -> Self {
        Self {
            s: s.into(),
            location,
        }
    }

    pub fn string(&self) -> &str {
        &self.s
    }

    pub fn location(&self) -> Option<LineLocation> {
        self.location
    }

    pub fn trimmed(&self) -> StringLocated {
        let t = self.s.trim_matches(is_trimmable);
        if t.len() == self.s.len() {
            self.clone()
        } else {
            StringLocated::new(t, self.location)
        }
    }

    pub fn append(&self, tail: &str) -> StringLocated {
        StringLocated::new(format!("{}{}", self.s, tail), self.location)
    }

    pub fn substring(&self, start: usize) -> StringLocated {
        StringLocated::new(&self.s[start..], self.location)
    }

    pub fn substring_range(&self, start: usize, end: usize) -> StringLocated {
        StringLocated::new(&self.s[start..end], self.location)
    }

    pub fn len(&self) -> usize {
        self.s.len()
    }

    pub fn is_empty(&self) -> bool {
        self.s.is_empty()
    }
}

/// How multiline commands treat comment lines (`MultilinesStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultilinesStrategy {
    RemoveStartingQuote,
    KeepStartingQuote,
}

impl MultilinesStrategy {
    fn clean(self, lines: &mut Vec<StringLocated>) {
        if self == MultilinesStrategy::RemoveStartingQuote {
            lines.retain(|l| !l.trimmed().string().starts_with('\''));
        }
    }
}

/// Group of source lines handed to a command (`BlocLines`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlocLines {
    lines: Vec<StringLocated>,
}

impl BlocLines {
    pub fn create() -> Self {
        Self::default()
    }

    pub fn from(lines: Vec<StringLocated>) -> Self {
        Self { lines }
    }

    pub fn single(line: StringLocated) -> Self {
        Self { lines: vec![line] }
    }

    pub fn single_string(s: &str) -> Self {
        Self::single(StringLocated::new(s, None))
    }

    pub fn from_strings(strings: &[&str]) -> Self {
        Self {
            lines: strings
                .iter()
                .map(|s| StringLocated::new(*s, None))
                .collect(),
        }
    }

    pub fn add(&self, s: StringLocated) -> Self {
        let mut lines = self.lines.clone();
        lines.push(s);
        Self { lines }
    }

    pub fn add_string(&self, s: &str) -> Self {
        self.add(StringLocated::new(s, None))
    }

    pub fn size(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn at(&self, i: usize) -> &StringLocated {
        &self.lines[i]
    }

    pub fn first(&self) -> Option<&StringLocated> {
        self.lines.first()
    }

    pub fn last(&self) -> &StringLocated {
        self.lines.last().expect("not empty")
    }

    pub fn iter(&self) -> impl Iterator<Item = &StringLocated> {
        self.lines.iter()
    }

    pub fn as_strings(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.s.clone()).collect()
    }

    pub fn clean_list(&self, strategy: MultilinesStrategy) -> Self {
        let mut lines = self.lines.clone();
        strategy.clean(&mut lines);
        Self { lines }
    }

    pub fn trim(&self) -> Self {
        Self {
            lines: self.lines.iter().map(StringLocated::trimmed).collect(),
        }
    }

    pub fn remove_empty_lines(&self) -> Self {
        Self {
            lines: self
                .lines
                .iter()
                .filter(|l| !l.is_empty())
                .cloned()
                .collect(),
        }
    }

    /// Drops common leading blanks of every line (`removeEmptyColumns`).
    pub fn remove_empty_columns(&self) -> Self {
        fn removable(lines: &[StringLocated]) -> bool {
            let mut any = false;
            for l in lines.iter().filter(|l| !l.is_empty()) {
                any = true;
                if !matches!(l.s.chars().next(), Some(' ' | '\t')) {
                    return false;
                }
            }
            any
        }
        let mut lines = self.lines.clone();
        while removable(&lines) {
            for l in &mut lines {
                if !l.is_empty() {
                    *l = l.substring(1);
                }
            }
        }
        Self { lines }
    }

    pub fn sub_list(&self, start: usize, end: usize) -> Self {
        Self {
            lines: self.lines[start..end].to_vec(),
        }
    }

    /// Removes `start` lines at the beginning and `end` at the end (`subExtract`).
    pub fn sub_extract(&self, start: usize, end: usize) -> Self {
        self.sub_list(start, self.lines.len() - end)
    }

    /// Joins a lone `{` on the second line to the first (`eventuallyMoveBracket`).
    pub fn eventually_move_bracket(&self) -> Self {
        if self.size() < 2 {
            return self.clone();
        }
        let first = self.lines[0].trimmed();
        let second = self.lines[1].trimmed();
        if !first.string().ends_with('{') && second.string() == "{" {
            let mut lines = vec![self.lines[0].append(" {")];
            lines.extend(self.lines[2..].iter().cloned());
            return Self { lines };
        }
        self.clone()
    }

    pub fn override_last_line(&self, last: &str) -> Self {
        let mut lines = self.lines.clone();
        if let Some(l) = lines.last_mut() {
            *l = StringLocated::new(last, l.location);
        }
        Self { lines }
    }

    /// Replaces the first line and removes `remove_at_end` chars from the last.
    pub fn remove_starting_and_ending(&self, data: &str, remove_at_end: usize) -> Self {
        if self.lines.is_empty() {
            return self.clone();
        }
        let mut lines = self.lines.clone();
        lines[0] = StringLocated::new(data, None);
        if remove_at_end > 0 {
            let n = lines.len() - 1;
            let end = lines[n].len() - remove_at_end;
            lines[n] = lines[n].substring_range(0, end);
        }
        Self { lines }
    }

    pub fn location(&self) -> Option<LineLocation> {
        self.lines.first().and_then(StringLocated::location)
    }

    /// Removes the leading indentation of the reference line from the following ones.
    pub fn trim_smart(&self, reference_line: usize) -> Self {
        if self.lines.len() <= reference_line {
            return self.clone();
        }
        let indent = |s: &str| s.chars().take_while(|c| matches!(c, ' ' | '\t')).count();
        let n = indent(self.lines[reference_line].string());
        let mut lines = self.lines.clone();
        for l in lines.iter_mut().skip(reference_line) {
            if l.is_empty() {
                continue;
            }
            let k =
                l.s.chars()
                    .take(n)
                    .take_while(|c| matches!(c, ' ' | '\t'))
                    .count();
            if k > 0 {
                *l = l.substring(k);
            }
        }
        Self { lines }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_located() {
        let s = StringLocated::new("  a b \t", Some(LineLocation { position: 3 }));
        assert_eq!(s.trimmed().string(), "a b");
        assert_eq!(s.trimmed().location().unwrap().position, 3);
        assert_eq!(StringLocated::new("x", None).trimmed().string(), "x");
        assert_eq!(s.append("!").string(), "  a b \t!");
        assert_eq!(s.substring(2).string(), "a b \t");
        assert_eq!(s.substring_range(2, 5).string(), "a b");
        assert_eq!(s.len(), 7);
        assert!(!s.is_empty());
    }

    #[test]
    fn bloc_lines_basics() {
        let b = BlocLines::from_strings(&["a", "'c", "b"]);
        assert_eq!(b.size(), 3);
        assert_eq!(
            b.clean_list(MultilinesStrategy::RemoveStartingQuote).size(),
            2
        );
        assert_eq!(
            b.clean_list(MultilinesStrategy::KeepStartingQuote).size(),
            3
        );
        assert_eq!(b.first().unwrap().string(), "a");
        assert_eq!(b.last().string(), "b");
        assert_eq!(b.at(1).string(), "'c");
        assert_eq!(b.as_strings(), ["a", "'c", "b"]);
        assert_eq!(b.sub_extract(1, 1).as_strings(), ["'c"]);
        assert_eq!(b.add_string("z").size(), 4);
        assert_eq!(
            BlocLines::create()
                .add(StringLocated::new("q", None))
                .size(),
            1
        );
        assert!(BlocLines::create().is_empty());
        assert_eq!(b.iter().count(), 3);
        assert_eq!(BlocLines::single_string("k").size(), 1);
        assert!(b.location().is_none());
    }

    #[test]
    fn bloc_lines_transforms() {
        let b = BlocLines::from_strings(&["  a", "", "  b"]);
        assert_eq!(b.trim().as_strings(), ["a", "", "b"]);
        assert_eq!(b.remove_empty_lines().size(), 2);
        assert_eq!(b.remove_empty_columns().as_strings(), ["a", "", "b"]);
        let c = BlocLines::from_strings(&["x", "  y", "    z"]);
        assert_eq!(c.trim_smart(1).as_strings(), ["x", "y", "  z"]);
        assert_eq!(c.trim_smart(9).size(), 3);
        let d = BlocLines::from_strings(&["class A", "{", "x", "}"]);
        assert_eq!(
            d.eventually_move_bracket().as_strings(),
            ["class A {", "x", "}"]
        );
        assert_eq!(c.eventually_move_bracket().size(), 3);
        assert_eq!(c.override_last_line("w").last().string(), "w");
        let e = BlocLines::from_strings(&["start", "body", "end>>"]);
        assert_eq!(
            e.remove_starting_and_ending("S", 2).as_strings(),
            ["S", "body", "end"]
        );
        assert_eq!(
            BlocLines::create()
                .remove_starting_and_ending("S", 2)
                .size(),
            0
        );
        assert_eq!(e.sub_list(1, 2).as_strings(), ["body"]);
    }
}
