//! Archives loaded with `!import` (`PathSystem`'s imported folders).
//!
//! `!import archive.zip` makes the entries of the archive available to later
//! `!include`/`!includesub` statements, resolved relative to the including
//! entry's folder and, failing that, to the archive root.

use std::collections::HashMap;
use std::io::{Cursor, Read};

/// Entries of one imported archive, keyed by normalized relative path.
pub struct ImportedArchive {
    files: HashMap<String, String>,
}

impl ImportedArchive {
    /// Parses a ZIP archive held in memory; returns `None` for a non-archive.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
        let mut files = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok()?;
            if entry.is_dir() {
                continue;
            }
            let name = normalize(entry.name())?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content).ok()?;
            files.insert(name, String::from_utf8_lossy(&content).into_owned());
        }
        Some(Self { files })
    }

    /// Looks up `name` relative to `dir`, then relative to the archive root.
    /// Returns the entry path, its content and its parent folder.
    pub fn read(&self, name: &str, dir: Option<&str>) -> Option<(String, String, String)> {
        let mut candidates = Vec::with_capacity(2);
        if let Some(dir) = dir {
            if let Some(path) = normalize(&format!("{dir}/{name}")) {
                candidates.push(path);
            }
        }
        if let Some(path) = normalize(name) {
            if !candidates.contains(&path) {
                candidates.push(path);
            }
        }
        for path in candidates {
            if let Some(content) = self.files.get(&path) {
                let parent = path.rsplit_once('/').map(|(p, _)| p.to_string());
                return Some((path, content.clone(), parent.unwrap_or_default()));
            }
        }
        None
    }
}

/// Normalizes a slash-separated relative path, resolving `.` and `..`.
fn normalize(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    let path = path.replace('\\', "/");
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            other => out.push(other),
        }
    }
    Some(out.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize("lib/../c.iuml").as_deref(), Some("c.iuml"));
        assert_eq!(normalize("./lib//a.iuml").as_deref(), Some("lib/a.iuml"));
        assert_eq!(normalize("../x").as_deref(), None);
    }
}
