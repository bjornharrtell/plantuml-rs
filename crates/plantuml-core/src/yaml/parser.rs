//! The YAML subset parser (`yaml/parser`): lines to a `Monomorph` tree to JSON.

use crate::json::JsonValue;

type Res<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineType {
    EmptyLine,
    NoKeyOnlyText,
    KeyOnly,
    KeyAndValue,
    KeyAndFlowSequence,
    KeyAndBlockStyle,
    KeyAndFoldedStyle,
    PlainElementList,
    PlainDash,
}

/// One analysed line (`YamlLine`).
#[derive(Debug, Clone)]
struct YamlLine {
    indent: usize,
    key: String,
    value: String,
    values: Vec<String>,
    list_item: bool,
    kind: LineType,
}

fn trim_java(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

fn unquote(s: &str) -> String {
    let b = s.as_bytes();
    if b.len() >= 2 {
        let (first, last) = (b[0], b[b.len() - 1]);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

fn remove_yaml_comment(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if s.starts_with('#') {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut in_quote = '\0';
    for i in 0..chars.len() {
        let c = chars[i];
        if c == '\'' || c == '"' {
            if in_quote == '\0' {
                in_quote = c;
            } else if c == in_quote {
                in_quote = '\0';
            }
        }
        if in_quote == '\0' && i + 1 < chars.len() && c == ' ' && chars[i + 1] == '#' {
            return chars[..i].iter().collect();
        }
    }
    s.to_string()
}

fn find_colon_separator(line: &str) -> Option<usize> {
    let mut in_quote = '\0';
    for (i, c) in line.char_indices() {
        if in_quote != '\0' {
            if c == in_quote {
                in_quote = '\0';
            }
        } else if c == '"' || c == '\'' {
            in_quote = c;
        } else if c == ':' {
            return Some(i);
        }
    }
    None
}

fn to_list(raw: &str) -> Vec<String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quoted = '\0';
    let mut field_start_with_quote = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_quoted != '\0' {
            if c == '\\' {
                if i + 1 < chars.len() {
                    current.push(chars[i + 1]);
                    i += 1;
                }
            } else if c == in_quoted {
                in_quoted = '\0';
            } else {
                current.push(c);
            }
        } else if !field_start_with_quote
            && trim_java(&current).is_empty()
            && (c == '\'' || c == '"')
        {
            in_quoted = c;
            field_start_with_quote = true;
            current.clear();
        } else if c == ',' {
            result.push(if field_start_with_quote {
                current.clone()
            } else {
                trim_java(&current).to_string()
            });
            current.clear();
            field_start_with_quote = false;
        } else if c == '\\' {
            if i + 1 < chars.len() {
                current.push(chars[i + 1]);
                i += 1;
            }
        } else {
            current.push(c);
        }
        i += 1;
    }
    result.push(if field_start_with_quote {
        current
    } else {
        trim_java(&current).to_string()
    });
    result
}

impl YamlLine {
    fn new(kind: LineType, indent: usize) -> Self {
        Self {
            indent,
            key: String::new(),
            value: String::new(),
            values: Vec::new(),
            list_item: false,
            kind,
        }
    }

    fn build(line: &str) -> YamlLine {
        let line = line.replace('\t', "    ");
        let mut count = line.chars().take_while(|&c| c == ' ').count();
        let rest = remove_yaml_comment(trim_java(&line[count..]));
        let mut trimmed = trim_java(&rest).to_string();
        if trimmed.is_empty() {
            return YamlLine::new(LineType::EmptyLine, 0);
        }
        if trimmed == "-" {
            let mut l = YamlLine::new(LineType::PlainDash, count + 1);
            l.list_item = true;
            return l;
        }
        let list_item = trimmed.starts_with("- ");
        if list_item {
            count += 2;
            trimmed = trimmed[2..].to_string();
        }
        let Some(colon) = find_colon_separator(&trimmed) else {
            let kind = if list_item {
                LineType::PlainElementList
            } else {
                LineType::NoKeyOnlyText
            };
            let mut l = YamlLine::new(kind, count);
            l.value = unquote(&trimmed);
            l.list_item = list_item;
            return l;
        };
        let raw_key = trim_java(&trimmed[..colon]).to_string();
        let raw_value = trim_java(&trimmed[colon + 1..]).to_string();
        let mut kind = LineType::KeyAndValue;
        if raw_value.is_empty() {
            kind = LineType::KeyOnly;
        } else if raw_value == "|" {
            kind = LineType::KeyAndBlockStyle;
        } else if raw_value == ">" {
            kind = LineType::KeyAndFoldedStyle;
        } else if raw_value.starts_with('[') && raw_value.ends_with(']') {
            let mut l = YamlLine::new(LineType::KeyAndFlowSequence, count);
            l.key = unquote(&raw_key);
            l.values = to_list(&raw_value[1..raw_value.len() - 1]);
            l.list_item = list_item;
            return l;
        }
        let mut l = YamlLine::new(kind, count);
        l.key = unquote(&raw_key);
        l.value = unquote(&raw_value);
        l.list_item = list_item;
        l
    }
}

#[derive(Debug, Clone)]
enum Mono {
    Undetermined,
    Scalar(String),
    List(Vec<usize>),
    Map(Vec<(String, usize)>),
}

/// The tree being built; nodes are addressed by index so that the builder can keep a stack
/// of the nodes still open (`Monomorph`, `YamlBuilder`).
struct Builder {
    arena: Vec<Mono>,
    indents: Vec<usize>,
    nodes: Vec<usize>,
}

impl Builder {
    fn new() -> Self {
        Self {
            arena: vec![Mono::Undetermined],
            indents: Vec::new(),
            nodes: vec![0],
        }
    }

    fn alloc(&mut self, m: Mono) -> usize {
        self.arena.push(m);
        self.arena.len() - 1
    }

    fn last(&self) -> usize {
        *self.nodes.last().expect("one node at least")
    }

    fn pop_node(&mut self) {
        self.nodes.pop();
    }

    fn add_in_list(&mut self, list: usize, element: usize) -> Res<()> {
        match &mut self.arena[list] {
            m @ Mono::Undetermined => *m = Mono::List(vec![element]),
            Mono::List(l) => l.push(element),
            _ => return Err("Monomorph is not of type LIST.".into()),
        }
        Ok(())
    }

    fn put_in_map(&mut self, map: usize, key: &str, value: usize) -> Res<()> {
        match &mut self.arena[map] {
            m @ Mono::Undetermined => *m = Mono::Map(vec![(key.to_string(), value)]),
            Mono::Map(entries) => match entries.iter_mut().find(|(k, _)| k == key) {
                Some(e) => e.1 = value,
                None => entries.push((key.to_string(), value)),
            },
            _ => return Err("Monomorph is not of type MAP.".into()),
        }
        Ok(())
    }

    fn scalar(&mut self, s: &str) -> usize {
        self.alloc(Mono::Scalar(s.to_string()))
    }

    fn list_of(&mut self, input: &[String]) -> usize {
        let items = input.iter().map(|s| self.scalar(s)).collect();
        self.alloc(Mono::List(items))
    }

    fn adjust_indentation(&mut self, indent: usize) -> Res<()> {
        let Some(&top) = self.indents.last() else {
            self.indents.push(indent);
            return Ok(());
        };
        if indent > top {
            self.indents.push(indent);
            return Ok(());
        }
        while self.indents.last().is_some_and(|&t| indent < t) {
            self.indents.pop();
            self.nodes.pop();
            let last = self.nodes.last().ok_or("indentation underflow")?;
            if matches!(self.arena[*last], Mono::List(_)) {
                self.nodes.pop();
            }
        }
        Ok(())
    }

    fn is_array_already_there(&self) -> bool {
        if self.nodes.len() < 2 {
            return false;
        }
        let potential = self.nodes[self.nodes.len() - 2];
        match &self.arena[potential] {
            Mono::List(l) => l.last() == self.nodes.last(),
            _ => false,
        }
    }

    fn new_list_element(&mut self) -> Res<usize> {
        if self.is_array_already_there() {
            self.pop_node();
        }
        let element = self.alloc(Mono::Undetermined);
        let last = self.last();
        self.add_in_list(last, element)?;
        self.nodes.push(element);
        Ok(element)
    }

    fn on_list_item_plain_dash(&mut self) -> Res<()> {
        self.new_list_element().map(|_| ())
    }

    fn on_key_and_value(&mut self, key: &str, value: &str) -> Res<()> {
        let v = self.scalar(value);
        let last = self.last();
        self.put_in_map(last, key, v)
    }

    fn on_key_and_flow_sequence(&mut self, key: &str, list: &[String]) -> Res<()> {
        let v = self.list_of(list);
        let last = self.last();
        self.put_in_map(last, key, v)
    }

    fn on_only_key(&mut self, key: &str) -> Res<()> {
        let e = self.alloc(Mono::Undetermined);
        let last = self.last();
        self.put_in_map(last, key, e)?;
        self.nodes.push(e);
        Ok(())
    }

    fn on_list_item_only_key(&mut self, key: &str) -> Res<()> {
        self.new_list_element()?;
        self.on_only_key(key)
    }

    fn on_list_item_only_value(&mut self, value: &str) -> Res<()> {
        if self.is_array_already_there() {
            self.pop_node();
        }
        let v = self.scalar(value);
        let last = self.last();
        self.add_in_list(last, v)
    }

    fn on_list_item_key_and_value(&mut self, key: &str, value: &str) -> Res<()> {
        self.new_list_element()?;
        self.on_key_and_value(key, value)
    }

    fn on_list_item_key_and_flow_sequence(&mut self, key: &str, values: &[String]) -> Res<()> {
        self.new_list_element()?;
        self.on_key_and_flow_sequence(key, values)
    }

    /// `MonomorphToJson.convert` of the root.
    fn to_json(&self, node: usize) -> Res<Option<JsonValue>> {
        Ok(match &self.arena[node] {
            Mono::Scalar(s) => Some(JsonValue::String(s.clone())),
            Mono::List(_) => Some(self.to_array(node)?),
            Mono::Map(_) => Some(self.to_object(node)?),
            Mono::Undetermined => None,
        })
    }

    fn to_array(&self, node: usize) -> Res<JsonValue> {
        let Mono::List(items) = &self.arena[node] else {
            return Err("not a list".into());
        };
        let mut out = Vec::new();
        for &i in items {
            out.push(match &self.arena[i] {
                Mono::Scalar(s) => JsonValue::String(s.clone()),
                Mono::Map(_) => self.to_object(i)?,
                _ => return Err("unsupported element".into()),
            });
        }
        Ok(JsonValue::Array(out))
    }

    fn to_object(&self, node: usize) -> Res<JsonValue> {
        let Mono::Map(entries) = &self.arena[node] else {
            return Err("not a map".into());
        };
        let mut out = Vec::new();
        for (k, i) in entries {
            let v = match &self.arena[*i] {
                Mono::Scalar(s) => JsonValue::String(s.clone()),
                Mono::Map(_) => self.to_object(*i)?,
                Mono::List(_) => self.to_array(*i)?,
                Mono::Undetermined => return Err("unsupported value".into()),
            };
            out.push((k.clone(), v));
        }
        Ok(JsonValue::Object(out))
    }
}

fn peek_next(lines: &[String], pos: usize) -> Option<YamlLine> {
    let mut i = 1;
    loop {
        let l = lines.get(pos + i)?;
        let next = YamlLine::build(l);
        if next.kind == LineType::EmptyLine {
            i += 1;
        } else {
            return Some(next);
        }
    }
}

fn peek_next_only_text(lines: &[String], pos: &mut usize) -> String {
    let mut result = String::new();
    loop {
        let Some(peek) = lines.get(*pos + 1) else {
            return result;
        };
        let next = YamlLine::build(peek);
        if next.kind == LineType::EmptyLine {
            *pos += 1;
            continue;
        }
        if next.kind == LineType::NoKeyOnlyText {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&next.value);
            *pos += 1;
            continue;
        }
        return result;
    }
}

fn block_style_string(lines: &[String], pos: &mut usize, indent: usize) -> String {
    let mut result = String::new();
    while let Some(line) = lines.get(*pos + 1) {
        let l = YamlLine::build(line);
        if l.kind != LineType::NoKeyOnlyText && l.kind != LineType::EmptyLine && l.indent <= indent
        {
            return result;
        }
        result.push_str(trim_java(line));
        result.push('\n');
        *pos += 1;
    }
    result
}

/// Parses the lines of a YAML document into JSON (`YamlParser` then `MonomorphToJson`).
/// `Ok(None)` when the document has no content.
pub fn parse_yaml(lines: &[String]) -> Result<Option<JsonValue>, String> {
    let mut b = Builder::new();
    let mut pos = 0;
    while pos < lines.len() {
        let line = YamlLine::build(&lines[pos]);
        match line.kind {
            LineType::EmptyLine => {
                pos += 1;
                continue;
            }
            LineType::NoKeyOnlyText => return Err("YamlLineType.NO_KEY_ONLY_TEXT".into()),
            LineType::PlainDash => {
                b.on_list_item_plain_dash()?;
                pos += 1;
                continue;
            }
            _ => {}
        }
        b.adjust_indentation(line.indent)?;
        if line.list_item {
            match line.kind {
                LineType::KeyOnly => b.on_list_item_only_key(&line.key)?,
                LineType::PlainElementList => b.on_list_item_only_value(&line.value)?,
                LineType::KeyAndValue => b.on_list_item_key_and_value(&line.key, &line.value)?,
                LineType::KeyAndFlowSequence => {
                    b.on_list_item_key_and_flow_sequence(&line.key, &line.values)?
                }
                _ => return Err(format!("wip3 {line:?}")),
            }
        } else {
            match line.kind {
                LineType::KeyOnly => match peek_next(lines, pos) {
                    Some(next) if next.indent > line.indent => {
                        if next.kind == LineType::NoKeyOnlyText {
                            let text = peek_next_only_text(lines, &mut pos);
                            b.on_key_and_value(&line.key, &text)?;
                        } else {
                            b.on_only_key(&line.key)?;
                        }
                    }
                    _ => b.on_key_and_value(&line.key, "")?,
                },
                LineType::KeyAndValue => b.on_key_and_value(&line.key, &line.value)?,
                LineType::KeyAndBlockStyle => {
                    let text = block_style_string(lines, &mut pos, line.indent);
                    b.on_key_and_value(&line.key, &text)?;
                }
                LineType::KeyAndFlowSequence => {
                    b.on_key_and_flow_sequence(&line.key, &line.values)?
                }
                LineType::KeyAndFoldedStyle => return Err(format!("wip4 {line:?}")),
                _ => return Err(format!("wip4 {line:?}")),
            }
        }
        pos += 1;
    }
    b.to_json(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Option<String>, String> {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        parse_yaml(&lines).map(|v| v.map(|v| v.to_string()))
    }

    #[test]
    fn maps_and_lists() {
        assert_eq!(
            parse("a: 1\nb:\n  c: 'x y'\n  d: [1, \"2,3\", z]\ne:\n  - p\n  - q: 1\n    r: 2\n")
                .unwrap()
                .unwrap(),
            r#"{"a":"1","b":{"c":"x y","d":["1","2,3","z"]},"e":["p",{"q":"1","r":"2"}]}"#
        );
    }

    #[test]
    fn list_of_maps_with_dashes() {
        assert_eq!(
            parse("-\n  name: A\n  hr: 65 # comment\n-\n  name: B\n")
                .unwrap()
                .unwrap(),
            r#"[{"name":"A","hr":"65"},{"name":"B"}]"#
        );
        assert_eq!(
            parse("- k:\n    z: 1\n- k: v\n- w: [a]\n- plain\n")
                .unwrap()
                .unwrap(),
            r#"[{"k":{"z":"1"}},{"k":"v"},{"w":["a"]},"plain"]"#
        );
    }

    #[test]
    fn multiline_and_block_styles() {
        assert_eq!(
            parse("a:\n  one\n  two\nb: |\n  l1\n  l2\nc:\n")
                .unwrap()
                .unwrap(),
            "{\"a\":\"one two\",\"b\":\"l1\\nl2\\n\",\"c\":\"\"}"
        );
    }

    #[test]
    fn errors_and_empty() {
        assert_eq!(parse("").unwrap(), None);
        assert_eq!(parse("# only\n\n").unwrap(), None);
        assert!(parse("just text").is_err());
        assert!(parse("a: >\n  x").is_err());
        assert!(parse("- a\nb: 1").is_err());
    }

    #[test]
    fn quoting_helpers() {
        assert_eq!(unquote("\"a\""), "a");
        assert_eq!(unquote("'a'"), "a");
        assert_eq!(unquote("a"), "a");
        assert_eq!(remove_yaml_comment("a: 'b #c' #d"), "a: 'b #c'");
        assert_eq!(find_colon_separator("'a:b': c"), Some(5));
        assert_eq!(to_list("a\\,b, 'c, d' , e"), vec!["a,b", "c, d ", "e"]);
    }
}
