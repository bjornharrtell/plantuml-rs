//! String helpers (`StringUtils`).

use std::sync::LazyLock;

fn is_double_quote(c: char) -> bool {
    matches!(c, '"' | '\u{201c}' | '\u{201d}' | '\u{00ab}' | '\u{00bb}')
}

/// Removes a surrounding pair of quotes, or `(..)`, `[..]`, `:..:` as allowed by `format`.
pub fn eventually_remove_starting_and_ending_double_quote_with(s: &str, format: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2 {
        return s.to_string();
    }
    let (first, last) = (chars[0], chars[chars.len() - 1]);
    let inner = || chars[1..chars.len() - 1].iter().collect::<String>();
    if format.contains('"') && is_double_quote(first) && is_double_quote(last) {
        return inner();
    }
    if format.contains('(') && first == '(' && last == ')' {
        return inner();
    }
    if format.contains('[') && first == '[' && last == ']' {
        return inner();
    }
    if format.contains(':') && first == ':' && last == ':' {
        return inner();
    }
    s.to_string()
}

/// `eventuallyRemoveStartingAndEndingDoubleQuote(s)` with the default format.
pub fn eventually_remove_starting_and_ending_double_quote(s: &str) -> String {
    eventually_remove_starting_and_ending_double_quote_with(s, "\"([:")
}

fn is_trimmable(c: char) -> bool {
    c <= ' ' || c == '\u{a0}'
}

/// Trims control characters and non-breaking spaces (`trin`).
pub fn trin(s: &str) -> &str {
    s.trim_matches(is_trimmable)
}

static SPLIT_COMMA: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| crate::regex::compile("([%pLN_.]+|[%g][^%g]+[%g])"));

/// Splits `A, "B C", D` into its items, unquoted (`splitComma`).
pub fn split_comma(s: &str) -> Vec<String> {
    let s = trin(s);
    SPLIT_COMMA
        .find_iter(s)
        .filter_map(Result::ok)
        .map(|m| eventually_remove_starting_and_ending_double_quote(m.as_str()))
        .collect()
}

/// `getUid`: prefix and a four digit number.
pub fn get_uid(prefix: &str, n: i32) -> String {
    format!("{prefix}{n:04}")
}

/// Lowercase hexadecimal MD5 of a text (`SignatureUtils.getMD5Hex`).
pub fn md5_hex(s: &str) -> String {
    use md5::{Digest, Md5};
    let digest = Md5::digest(s.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes() {
        assert_eq!(
            eventually_remove_starting_and_ending_double_quote("\"a b\""),
            "a b"
        );
        assert_eq!(
            eventually_remove_starting_and_ending_double_quote("(a)"),
            "a"
        );
        assert_eq!(
            eventually_remove_starting_and_ending_double_quote("[a]"),
            "a"
        );
        assert_eq!(
            eventually_remove_starting_and_ending_double_quote(":a:"),
            "a"
        );
        assert_eq!(eventually_remove_starting_and_ending_double_quote("a"), "a");
        assert_eq!(
            eventually_remove_starting_and_ending_double_quote_with("(a)", "\""),
            "(a)"
        );
    }

    #[test]
    fn trin_and_split() {
        assert_eq!(trin(" \t x\u{a0}"), "x");
        assert_eq!(split_comma(" A, \"B C\" ,D.e "), vec!["A", "B C", "D.e"]);
    }
}
