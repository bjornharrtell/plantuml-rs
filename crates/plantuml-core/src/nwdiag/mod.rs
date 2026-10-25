//! Network diagrams (`nwdiag`); only the helpers shared with other diagrams for now.

use std::collections::HashMap;

/// Parses `a = 1, b = "x y"` attribute lists (`NwDiagram.toSet`).
pub fn to_set(definition: Option<&str>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let Some(definition) = definition else {
        return result;
    };
    let re = fancy_regex::Regex::new(r#"\s*(\w+)\s*=\s*("([^"]*)"|[^\s,]+)"#).expect("valid");
    for caps in re.captures_iter(definition).flatten() {
        let value = caps
            .get(3)
            .or_else(|| caps.get(2))
            .map_or("", |m| m.as_str());
        result.insert(caps[1].to_string(), value.to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        let m = to_set(Some(r#"len=3, rotate = 90 , label="a b""#));
        assert_eq!(m["len"], "3");
        assert_eq!(m["rotate"], "90");
        assert_eq!(m["label"], "a b");
        assert!(to_set(None).is_empty());
    }
}
