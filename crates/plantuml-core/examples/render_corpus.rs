//! Renders every `.puml` file below a directory in one process and reports the timing.
//!
//! Used by `scripts/bench.py` for the in-process figure.
//! Usage: `cargo run --release -p plantuml-core --example render_corpus -- <dir>`

use plantuml_core::tim::resolver::{BundledResolver, ResolvedFile, Resolver};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

/// Reads files relative to one directory and delegates themes and stdlib to the bundle.
struct DirResolver {
    base: PathBuf,
    bundled: BundledResolver,
}

impl Resolver for DirResolver {
    fn read_file(&self, name: &str, current_dir: Option<&str>) -> Option<ResolvedFile> {
        let path = match current_dir {
            Some(dir) if !dir.is_empty() => self.base.join(dir).join(name),
            _ => self.base.join(name),
        };
        let content = std::fs::read_to_string(&path).ok()?;
        Some(ResolvedFile {
            content,
            dir: path.parent().map(|p| p.to_string_lossy().into_owned()),
            identity: path.to_string_lossy().into_owned(),
        })
    }

    fn read_stdlib(&self, name: &str) -> Option<String> {
        self.bundled.read_stdlib(name)
    }

    fn read_theme(
        &self,
        name: &str,
        from: Option<&str>,
        current_dir: Option<&str>,
    ) -> Option<String> {
        self.bundled.read_theme(name, from, current_dir)
    }

    fn all_theme_names(&self) -> Vec<String> {
        self.bundled.all_theme_names()
    }

    fn file_exists(&self, path: &str) -> bool {
        self.base.join(path).is_file()
    }
}

fn puml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            puml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "puml") {
            out.push(path);
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: render_corpus <dir>");
    let mut files = Vec::new();
    puml_files(Path::new(&dir), &mut files);
    files.sort();
    let mut ok = 0usize;
    let mut err = 0usize;
    let mut bytes = 0usize;
    let start = Instant::now();
    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else {
            err += 1;
            continue;
        };
        let base = file.parent().unwrap_or(Path::new("."));
        let resolver = Rc::new(DirResolver {
            base: base.to_path_buf(),
            bundled: BundledResolver,
        });
        match plantuml_core::render_with(&source, resolver) {
            Ok(svg) => {
                ok += 1;
                bytes += svg.len();
            }
            Err(_) => err += 1,
        }
    }
    let elapsed = start.elapsed();
    println!(
        "files={} ok={} err={} bytes={} total_ms={:.1} mean_ms={:.2}",
        files.len(),
        ok,
        err,
        bytes,
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / files.len().max(1) as f64
    );
}
