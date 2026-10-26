use std::io::{Read, Write};
use std::process::ExitCode;

fn read_input(path: Option<&str>) -> std::io::Result<String> {
    match path {
        None | Some("-") => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
        Some(p) => std::fs::read_to_string(p),
    }
}

fn run(path: Option<&str>, out: &mut impl Write) -> Result<(), String> {
    let source = read_input(path).map_err(|e| e.to_string())?;
    let svg = plantuml_core::render(&source).map_err(|e| e.to_string())?;
    out.write_all(svg.as_bytes()).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    env_logger::init();
    let arg = std::env::args().nth(1);
    match run(arg.as_deref(), &mut std::io::stdout()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_an_error() {
        let mut out = Vec::new();
        assert!(run(Some("/nonexistent/x.puml"), &mut out).is_err());
        assert!(out.is_empty());
    }

    #[test]
    fn unsupported_diagram_is_an_error() {
        let path = std::env::temp_dir().join("plantuml-cli-test.puml");
        std::fs::write(&path, "@startgantt\n[a] lasts 1 day\n@endgantt\n").unwrap();
        let mut out = Vec::new();
        let err = run(path.to_str(), &mut out).unwrap_err();
        assert!(err.contains("unsupported"));
        std::fs::remove_file(path).ok();
    }
}
