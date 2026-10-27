//! `plantuml-cli`: render `.puml` files (or stdin) to SVG.
//!
//! The output contract is meant for scripts and agents as much as for humans: several inputs in one
//! run, `-o` to write files, `--check` for validation with a non-zero exit, and `--json` for
//! structured results (one object per input, newline-delimited when there are several).

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

mod resolver;

use resolver::FsResolver;

const USAGE: &str = "\
usage: plantuml-cli [OPTIONS] [FILE...]

Render PlantUML to SVG. With no FILE, or with `-`, the source is read from stdin.

options:
  -o, --out DIR   write <name>.svg into DIR instead of stdout (files only)
      --json      print one JSON object per input, without the SVG when writing files
      --check     validate only: write no SVG, exit 1 if any input fails
  -tsvg           accepted for PlantUML compatibility; SVG is the only format
  -h, --help      show this help
  -V, --version   show the version
";

#[derive(Debug, Default)]
struct Options {
    files: Vec<String>,
    out: Option<PathBuf>,
    json: bool,
    check: bool,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!(
                    "plantuml-cli {} (plantuml {})",
                    env!("CARGO_PKG_VERSION"),
                    plantuml_core::VERSION
                );
                std::process::exit(0);
            }
            "--json" => options.json = true,
            "--check" => options.check = true,
            "-tsvg" | "--svg" => {}
            "-o" | "--out" => {
                i += 1;
                let dir = args.get(i).ok_or("missing directory after --out")?;
                options.out = Some(PathBuf::from(dir));
            }
            "--" => {
                options.files.extend(args[i + 1..].iter().cloned());
                break;
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("unknown option {other}"));
            }
            other => options.files.push(other.to_string()),
        }
        i += 1;
    }
    if options.out.is_some() && options.files.iter().all(|f| f == "-") {
        return Err("--out needs file names, not stdin".to_string());
    }
    if options.out.is_some() && options.files.is_empty() {
        return Err("--out needs at least one file".to_string());
    }
    Ok(options)
}

/// Reads the source of one input (`None`/`-` is stdin).
fn read_input(path: Option<&str>) -> std::io::Result<String> {
    match path {
        None | Some("-") => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
        Some(p) => fs::read_to_string(p),
    }
}

/// Renders one input, resolving `!include` relative to its directory.
fn render(path: Option<&str>) -> Result<String, String> {
    let source = read_input(path).map_err(|e| e.to_string())?;
    let base = match path {
        None | Some("-") => std::env::current_dir().unwrap_or_default(),
        Some(p) => Path::new(p)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
    };
    let resolver = Rc::new(FsResolver::new(base));
    plantuml_core::render_with(&source, resolver).map_err(|e| e.to_string())
}

#[derive(Debug, Default, PartialEq)]
struct SvgMeta {
    diagram_type: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
}

/// What an agent needs to know about an SVG without parsing it: type and canvas size.
fn svg_meta(svg: &str) -> SvgMeta {
    let attribute = |name: &str| -> Option<String> {
        let key = format!("{name}=\"");
        let start = svg.find(&key)? + key.len();
        let rest = &svg[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    };
    SvgMeta {
        diagram_type: attribute("data-diagram-type"),
        width: attribute("width").and_then(|v| v.trim_end_matches("px").parse().ok()),
        height: attribute("height").and_then(|v| v.trim_end_matches("px").parse().ok()),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn number_j(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v}"),
        None => "null".to_string(),
    }
}

fn opt_string(value: Option<&str>) -> String {
    match value {
        Some(v) => format!("\"{}\"", json_escape(v)),
        None => "null".to_string(),
    }
}

/// The line of a `(line N)` suffix in an error message, when the core provides one.
fn error_line(message: &str) -> Option<u32> {
    let start = message.rfind("(line ")? + "(line ".len();
    let rest = &message[start..];
    let end = rest.find(')')?;
    rest[..end].parse().ok()
}

/// One JSON result for an input.
fn result_json(
    file: Option<&str>,
    result: &Result<String, String>,
    output: Option<&Path>,
    with_svg: bool,
) -> String {
    let meta = result.as_ref().map(|svg| svg_meta(svg)).unwrap_or_default();
    let mut fields = vec![
        format!("\"file\":{}", opt_string(file)),
        format!("\"ok\":{}", result.is_ok()),
        format!(
            "\"diagram_type\":{}",
            opt_string(meta.diagram_type.as_deref())
        ),
        format!("\"width\":{}", number_j(meta.width)),
        format!("\"height\":{}", number_j(meta.height)),
    ];
    match result {
        Ok(svg) => {
            if let Some(path) = output {
                fields.push(format!(
                    "\"output\":\"{}\"",
                    json_escape(&path.to_string_lossy())
                ));
            }
            if with_svg {
                fields.push(format!("\"svg\":\"{}\"", json_escape(svg)));
            }
        }
        Err(message) => {
            let mut error = format!("\"message\":\"{}\"", json_escape(message));
            if let Some(line) = error_line(message) {
                error.push_str(&format!(",\"line\":{line}"));
            }
            fields.push(format!("\"error\":{{{error}}}"));
        }
    }
    format!("{{{}}}", fields.join(","))
}

/// Writes the SVG next to an output directory, returning the path written.
fn write_output(dir: &Path, input: &str, svg: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stem = Path::new(input)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "diagram".to_string());
    let path = dir.join(format!("{stem}.svg"));
    fs::write(&path, svg).map_err(|e| e.to_string())?;
    Ok(path)
}

fn run(options: &Options, out: &mut impl Write) -> i32 {
    let inputs: Vec<Option<&str>> = if options.files.is_empty() {
        vec![None]
    } else {
        options.files.iter().map(|f| Some(f.as_str())).collect()
    };
    let mut failed = 0;
    for input in inputs {
        let result = render(input);
        let mut output = None;
        let mut written = result;
        if let Some(dir) = &options.out {
            if let Ok(svg) = &written {
                if !options.check {
                    match write_output(dir, input.unwrap_or("diagram"), svg) {
                        Ok(path) => output = Some(path),
                        Err(e) => written = Err(e),
                    }
                }
            }
        }
        if written.is_err() {
            failed += 1;
        }
        if options.json {
            let line = result_json(
                input,
                &written,
                output.as_deref(),
                !options.check && output.is_none(),
            );
            let _ = writeln!(out, "{line}");
        } else if let Ok(svg) = &written {
            if output.is_none() && !options.check {
                let _ = out.write_all(svg.as_bytes());
            }
        }
        if written.is_err() {
            if let Err(message) = &written {
                log::error!("{}: {message}", input.unwrap_or("<stdin>"));
            }
        }
    }
    i32::from(failed > 0)
}

fn main() -> ExitCode {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("plantuml-cli: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&options, &mut std::io::stdout()) {
        0 => ExitCode::SUCCESS,
        code => ExitCode::from(code as u8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, body: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("plantuml-cli-{}-{}", std::process::id(), name));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.puml"));
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(render(Some("/nonexistent/x.puml")).is_err());
    }

    #[test]
    fn unsupported_diagram_is_an_error() {
        let path = temp("unsupported", "@startbpm\nAlice -> Bob\n@endbpm\n");
        let err = render(path.to_str()).unwrap_err();
        assert!(err.contains("unsupported"), "{err}");
    }

    #[test]
    fn metadata_comes_from_the_svg() {
        let svg = render(Some(
            temp("meta", "@startuml\nA -> B\n@enduml").to_str().unwrap(),
        ))
        .unwrap();
        let meta = svg_meta(&svg);
        assert_eq!(meta.diagram_type.as_deref(), Some("SEQUENCE"));
        assert!(meta.width.unwrap() > 0.0 && meta.height.unwrap() > 0.0);
    }

    #[test]
    fn check_mode_reports_failures_without_svg() {
        let good = temp("check-good", "@startuml\nA -> B\n@enduml");
        let bad = temp("check-bad", "@startuml\nclass\n@enduml");
        let options = Options {
            files: vec![
                good.to_string_lossy().into_owned(),
                bad.to_string_lossy().into_owned(),
            ],
            check: true,
            json: true,
            out: None,
        };
        let mut out = Vec::new();
        assert_eq!(run(&options, &mut out), 1);
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("\"ok\":true"));
        assert!(text.contains("\"ok\":false"));
        assert!(!text.contains("\"svg\""));
    }

    #[test]
    fn out_writes_one_svg_per_input() {
        let a = temp("out-a", "@startuml\nA -> B\n@enduml");
        let b = temp("out-b", "@startuml\nclass C\n@enduml");
        let dir = std::env::temp_dir().join(format!("plantuml-cli-out-{}", std::process::id()));
        let options = Options {
            files: vec![
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            out: Some(dir.clone()),
            ..Options::default()
        };
        let mut out = Vec::new();
        assert_eq!(run(&options, &mut out), 0);
        assert!(out.is_empty());
        assert!(dir.join("out-a.svg").is_file());
        assert!(dir.join("out-b.svg").is_file());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_rejects_out_without_files() {
        assert!(parse_args(&["--out".into(), "dir".into()]).is_err());
        let options = parse_args(&["--json".into(), "a.puml".into(), "b.puml".into()]).unwrap();
        assert!(options.json);
        assert_eq!(options.files.len(), 2);
    }

    #[test]
    fn json_escapes_and_error_lines() {
        assert_eq!(json_escape("a\"b\n"), "a\\\"b\\n");
        assert_eq!(error_line("bad thing (line 12)"), Some(12));
        assert_eq!(error_line("bad thing"), None);
    }
}
