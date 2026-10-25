//! Minimal XML tree and writer (`XmlNode`, `XmlWriter`). Attributes are written sorted by name,
//! matching the JVM's serialization that PlantUML's reference SVGs use.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum XmlChild {
    Node(XmlNode),
    Text(String),
    Comment(String),
    ProcessingInstruction(String, String),
    CData(String),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct XmlNode {
    pub tag: String,
    attrs: BTreeMap<String, String>,
    pub children: Vec<XmlChild>,
}

impl XmlNode {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.into(),
            attrs: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.attrs.insert(name.into(), value.into());
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    pub fn push(&mut self, child: XmlNode) {
        self.children.push(XmlChild::Node(child));
    }

    pub fn set_text(&mut self, text: &str) {
        self.children.clear();
        self.children.push(XmlChild::Text(text.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn to_xml(&self, indent: usize) -> String {
        let mut w = XmlWriter::new(indent);
        self.write(&mut w);
        w.out
    }

    fn write(&self, w: &mut XmlWriter) {
        w.start(&self.tag);
        for (k, v) in &self.attrs {
            w.attribute(k, v);
        }
        for c in &self.children {
            match c {
                XmlChild::Node(n) => n.write(w),
                XmlChild::Text(t) => w.text(t),
                XmlChild::Comment(t) => w.comment(t),
                XmlChild::ProcessingInstruction(t, d) => w.pi(t, d),
                XmlChild::CData(t) => w.cdata(t),
                XmlChild::Raw(t) => w.raw(t),
            }
        }
        w.end();
    }
}

struct XmlWriter {
    out: String,
    indent: usize,
    open: Vec<String>,
    pending_close: bool,
    inline_content: bool,
}

impl XmlWriter {
    fn new(indent: usize) -> Self {
        Self {
            out: String::new(),
            indent,
            open: Vec::new(),
            pending_close: false,
            inline_content: false,
        }
    }

    fn newline(&mut self) {
        if self.indent > 0 {
            self.out.push('\n');
        }
    }

    fn pad(&mut self, level: usize) {
        for _ in 0..level * self.indent {
            self.out.push(' ');
        }
    }

    fn close_pending(&mut self, opening_child: bool) {
        if self.pending_close {
            self.out.push('>');
            if opening_child {
                self.newline();
            }
            self.pending_close = false;
        }
    }

    fn start(&mut self, name: &str) {
        self.close_pending(true);
        self.pad(self.open.len());
        self.out.push('<');
        self.out.push_str(name);
        self.open.push(name.into());
        self.pending_close = true;
        self.inline_content = false;
    }

    fn attribute(&mut self, name: &str, value: &str) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        for c in value.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '"' => self.out.push_str("&quot;"),
                c => self.out.push(c),
            }
        }
        self.out.push('"');
    }

    fn text(&mut self, value: &str) {
        self.close_pending(false);
        self.inline_content = true;
        for c in value.chars() {
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                c => self.out.push(c),
            }
        }
    }

    fn comment(&mut self, value: &str) {
        self.close_pending(true);
        self.pad(self.open.len());
        let mut safe = value.replace("--", "- -");
        if safe.ends_with('-') {
            safe.push(' ');
        }
        self.out.push_str("<!--");
        self.out.push_str(&safe);
        self.out.push_str("-->");
        self.newline();
        self.inline_content = false;
    }

    fn pi(&mut self, target: &str, data: &str) {
        self.close_pending(true);
        self.pad(self.open.len());
        self.out.push_str("<?");
        self.out.push_str(target);
        if !data.is_empty() {
            self.out.push(' ');
            self.out.push_str(data);
        }
        self.out.push_str("?>");
        self.newline();
        self.inline_content = false;
    }

    fn cdata(&mut self, value: &str) {
        self.close_pending(true);
        self.pad(self.open.len());
        self.out.push_str("<![CDATA[");
        self.out.push_str(&value.replace("]]>", "]]]]><![CDATA[>"));
        self.out.push_str("]]>");
        self.newline();
        self.inline_content = false;
    }

    fn raw(&mut self, markup: &str) {
        self.close_pending(true);
        self.out.push_str(markup);
        self.inline_content = false;
    }

    fn end(&mut self) {
        let name = self.open.pop().expect("unbalanced xml");
        if self.pending_close {
            self.out.push_str("/>");
            self.newline();
            self.pending_close = false;
        } else {
            if !self.inline_content {
                self.pad(self.open.len());
            }
            self.out.push_str("</");
            self.out.push_str(&name);
            self.out.push('>');
            self.newline();
        }
        self.inline_content = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_sorted_attributes_and_escapes() {
        let mut root = XmlNode::new("svg");
        root.set("width", "1");
        root.set("a", "x<&\"");
        let mut t = XmlNode::new("text");
        t.set_text("a < b & c > d");
        root.push(t);
        root.push(XmlNode::new("defs"));
        root.children.push(XmlChild::Comment("a--b-".into()));
        let xml = root.to_xml(2);
        assert_eq!(
            xml,
            "<svg a=\"x&lt;&amp;&quot;\" width=\"1\">\n  <text>a &lt; b &amp; c > d</text>\n  <defs/>\n  <!--a- -b- -->\n</svg>\n"
        );
    }

    #[test]
    fn other_leaves_and_helpers() {
        let mut n = XmlNode::new("g");
        assert!(n.is_empty());
        n.children
            .push(XmlChild::ProcessingInstruction("pi".into(), "d".into()));
        n.children
            .push(XmlChild::ProcessingInstruction("pi2".into(), String::new()));
        n.children.push(XmlChild::CData("a]]>b".into()));
        n.children.push(XmlChild::Raw("<r/>".into()));
        n.set("k", "v");
        assert_eq!(n.attr("k"), Some("v"));
        assert_eq!(n.attr("z"), None);
        assert!(!n.is_empty());
        let xml = n.to_xml(0);
        assert_eq!(
            xml,
            "<g k=\"v\"><?pi d?><?pi2?><![CDATA[a]]]]><![CDATA[>b]]><r/></g>"
        );
    }
}
