//! Access to files, URLs and bundled resources for `!include` and `!theme`.

/// A file found by a [`Resolver`].
pub struct ResolvedFile {
    pub content: String,
    /// Directory against which includes inside this file are resolved.
    pub dir: Option<String>,
    /// Stable identity used to avoid including the same file twice.
    pub identity: String,
}

pub trait Resolver {
    /// Reads `name`, relative to `current_dir` first.
    fn read_file(&self, name: &str, current_dir: Option<&str>) -> Option<ResolvedFile>;
    /// Content of a standard library file `<lib/name>`.
    fn read_stdlib(&self, _name: &str) -> Option<String> {
        None
    }
    fn read_url(&self, _url: &str) -> Option<String> {
        None
    }
    /// Content of the theme `name` (bundled, or found in `from`).
    fn read_theme(
        &self,
        _name: &str,
        _from: Option<&str>,
        _current_dir: Option<&str>,
    ) -> Option<String> {
        None
    }
    fn all_theme_names(&self) -> Vec<String> {
        Vec::new()
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn env(&self, _name: &str) -> Option<String> {
        None
    }
    /// Milliseconds since the Unix epoch; zero when no clock is available.
    fn now_millis(&self) -> i64 {
        0
    }
    /// Names of the bundled stdlib folders.
    fn all_stdlib_names(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Resolver with nothing in it.
pub struct NoResolver;

impl Resolver for NoResolver {
    fn read_file(&self, _name: &str, _current_dir: Option<&str>) -> Option<ResolvedFile> {
        None
    }
}

/// Files held in memory, for tests.
#[derive(Default)]
pub struct MemoryResolver {
    pub files: std::collections::HashMap<String, String>,
}

impl Resolver for MemoryResolver {
    fn read_file(&self, name: &str, _current_dir: Option<&str>) -> Option<ResolvedFile> {
        self.files.get(name).map(|c| ResolvedFile {
            content: c.clone(),
            dir: None,
            identity: name.to_string(),
        })
    }

    fn file_exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }
}

/// Resolver with the themes bundled in the library and nothing else.
pub struct BundledResolver;

impl Resolver for BundledResolver {
    fn read_file(&self, _name: &str, _current_dir: Option<&str>) -> Option<ResolvedFile> {
        None
    }

    fn read_theme(
        &self,
        name: &str,
        from: Option<&str>,
        _current_dir: Option<&str>,
    ) -> Option<String> {
        if from.is_some() {
            return None;
        }
        super::themes::THEMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| (*c).to_string())
    }

    fn all_theme_names(&self) -> Vec<String> {
        super::themes::THEMES
            .iter()
            .map(|(n, _)| (*n).to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_themes() {
        let r = BundledResolver;
        assert!(r.read_theme("amiga", None, None).is_some());
        assert!(r.read_theme("nope", None, None).is_none());
        assert!(r.all_theme_names().len() >= 40);
    }
}
