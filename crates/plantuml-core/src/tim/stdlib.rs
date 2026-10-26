//! Standard library files bundled with the library, for `!include <lib/name>`.
//!
//! The sources are vendored from upstream projects, see `stdlib/README.md`.

/// Bundled stdlib files: lowercase name with extension, and content.
pub const FILES: &[(&str, &str)] = &[
    ("c4/c4.puml", include_str!("../../stdlib/c4/C4.puml")),
    (
        "c4/c4_component.puml",
        include_str!("../../stdlib/c4/C4_Component.puml"),
    ),
    (
        "c4/c4_container.puml",
        include_str!("../../stdlib/c4/C4_Container.puml"),
    ),
    (
        "c4/c4_context.puml",
        include_str!("../../stdlib/c4/C4_Context.puml"),
    ),
    (
        "c4/c4_deployment.puml",
        include_str!("../../stdlib/c4/C4_Deployment.puml"),
    ),
    (
        "c4/c4_dynamic.puml",
        include_str!("../../stdlib/c4/C4_Dynamic.puml"),
    ),
    (
        "c4/c4_sequence.puml",
        include_str!("../../stdlib/c4/C4_Sequence.puml"),
    ),
];

/// Normalizes a stdlib reference: strips the `<`/`>`, lowercases and adds the extension.
fn key(name: &str) -> String {
    let mut key = name
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_lowercase();
    if !key.ends_with(".puml") {
        key.push_str(".puml");
    }
    key
}

/// Content of a bundled stdlib file (`<C4/C4_Container>` or `C4_Container`), if present.
pub fn read(name: &str) -> Option<String> {
    let key = key(name);
    if let Some((_, content)) = FILES.iter().find(|(n, _)| *n == key) {
        return Some((*content).to_string());
    }
    // A bare name such as `<C4_Container>` stands for the only library bundled so far.
    if !key.contains('/') {
        let with_lib = format!("c4/{key}");
        if let Some((_, content)) = FILES.iter().find(|(n, _)| *n == with_lib) {
            return Some((*content).to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bundled_files() {
        assert!(read("C4/C4_Container")
            .unwrap()
            .contains("procedure Container"));
        assert!(read("<c4/c4_context>")
            .unwrap()
            .contains("procedure Person"));
        assert!(read("C4_Container.puml").is_some());
        assert!(read("C4/Nope").is_none());
        assert!(read("EBNF/Ebnf").is_none());
    }

    #[test]
    fn vendored_files_include_each_other_within_the_bundle() {
        let container = read("C4/C4_Container").unwrap();
        assert!(container.contains("!include <c4/C4_Context.puml>"));
    }
}
