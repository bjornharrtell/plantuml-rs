//! The rows of the members a link can name (`MethodsOrFieldsArea.getPorts`), written as ports of
//! the layout node so that `Foo::field1 --> Bar::field3` leaves and reaches the field rows.

use super::types::PortGeometry;
use crate::abel::{Entity, Member};
use crate::klimt::font::StringBounder;
use crate::klimt::shape::InnerRect;

/// Whether some link named a member of this entity (`getPortShortNames().size() > 0`).
pub(crate) fn has_ports(entity: &Entity) -> bool {
    !entity.port_short_names().is_empty()
}

/// The ports of `members`, whose rows are found with `inner_position` (that of the whole image,
/// so the header offset is already applied). `members` must be in drawing order.
pub(crate) fn member_ports(
    entity: &Entity,
    members: &[Member],
    sb: &dyn StringBounder,
    inner_position: impl Fn(&str, &dyn StringBounder) -> Option<InnerRect>,
) -> Vec<PortGeometry> {
    let names = sort_short_names(entity.port_short_names());
    if names.is_empty() {
        return Vec::new();
    }
    let mut ports: Vec<PortGeometry> = Vec::new();
    for m in members {
        let display = m.display(false);
        let Some((name, score)) = elect(&display, &names) else {
            continue;
        };
        let Some((_x, y, _w, h)) = inner_position(m.raw(), sb) else {
            continue;
        };
        let id = super::bibliotekon::encode_port_name(&name);
        // `Ports.add` keeps the row with the highest score, and the first one on a tie.
        match ports.iter_mut().find(|p| p.id == id) {
            Some(existing) => {
                if score > existing.score {
                    existing.position = y;
                    existing.height = h;
                    existing.score = score;
                }
            }
            None => ports.push(PortGeometry::new(id, y, h, score)),
        }
    }
    ports.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ports
}

/// `MethodsOrFieldsArea.sortBySize`: longest short name first, then lexicographic, so `field1`
/// wins over `field`.
fn sort_short_names(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    names
}

/// `MethodsOrFieldsArea.getElected`: the first short name the member text contains, with Java's
/// score (100 for a word match, 50 for a substring). A trailing `()` on the name is not part of
/// the word, so `m1()` still matches the method row `m1()`.
fn elect(display: &str, short_names: &[String]) -> Option<(String, i32)> {
    for name in short_names {
        let word = name.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if !word.is_empty() {
            let pattern = format!(r"\b{}\b", regex::escape(word));
            if regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(display)) {
                return Some((name.clone(), 100));
            }
        }
        if display.contains(name.as_str()) {
            return Some((name.clone(), 50));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elect_prefers_the_whole_word_then_a_substring() {
        let names = vec!["field".to_string(), "field1".to_string()];
        // Longest first, word match beats substring.
        assert_eq!(
            elect("field1", &sort_short_names(names.clone())),
            Some(("field1".to_string(), 100))
        );
        assert_eq!(
            elect("field2", &sort_short_names(names.clone())),
            Some(("field".to_string(), 50))
        );
        // A method row keeps its parentheses in the display, the name may or may not have them.
        assert_eq!(
            elect("m1()", &["m1()".to_string()]),
            Some(("m1()".to_string(), 100))
        );
        assert_eq!(
            elect("m1()", &["m1".to_string()]),
            Some(("m1".to_string(), 100))
        );
        assert_eq!(elect("other", &names), None);
    }

    #[test]
    fn sort_by_size_is_longest_first_then_alphabetical() {
        assert_eq!(
            sort_short_names(vec!["b".into(), "long".into(), "a".into()]),
            vec!["long".to_string(), "a".to_string(), "b".to_string()]
        );
    }
}
