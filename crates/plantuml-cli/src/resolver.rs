//! Filesystem resolver for the CLI.

use plantuml_core::tim::resolver::{BundledResolver, ResolvedFile, Resolver};
use std::path::{Path, PathBuf};

/// Reads includes relative to the diagram's directory or the process cwd.
pub struct FsResolver {
    base: PathBuf,
    themes: BundledResolver,
}

impl FsResolver {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            themes: BundledResolver,
        }
    }

    fn path(&self, name: &str, current_dir: Option<&str>) -> PathBuf {
        let name = Path::new(name);
        if name.is_absolute() {
            return name.to_path_buf();
        }
        match current_dir {
            Some(dir) if !dir.is_empty() => self.base.join(dir).join(name),
            _ => self.base.join(name),
        }
    }

    fn bytes(&self, name: &str, current_dir: Option<&str>) -> Option<(Vec<u8>, PathBuf)> {
        let path = self.path(name, current_dir);
        let bytes = std::fs::read(&path).ok()?;
        Some((bytes, path))
    }
}

impl Resolver for FsResolver {
    fn read_file(&self, name: &str, current_dir: Option<&str>) -> Option<ResolvedFile> {
        let (bytes, path) = self.bytes(name, current_dir)?;
        let dir = path.parent().map(|p| p.to_string_lossy().into_owned());
        Some(ResolvedFile {
            content: String::from_utf8_lossy(&bytes).into_owned(),
            dir,
            identity: path.to_string_lossy().into_owned(),
        })
    }

    fn read_binary(&self, name: &str, current_dir: Option<&str>) -> Option<Vec<u8>> {
        self.bytes(name, current_dir).map(|(bytes, _)| bytes)
    }

    fn read_theme(
        &self,
        name: &str,
        from: Option<&str>,
        current_dir: Option<&str>,
    ) -> Option<String> {
        if let Some(from) = from {
            let path = self.path(from, current_dir).join(format!("{name}.puml"));
            return std::fs::read_to_string(path).ok();
        }
        self.themes.read_theme(name, None, current_dir)
    }

    fn all_theme_names(&self) -> Vec<String> {
        self.themes.all_theme_names()
    }

    fn file_exists(&self, path: &str) -> bool {
        self.path(path, None).is_file()
    }

    fn env(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn now_millis(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    }
}
