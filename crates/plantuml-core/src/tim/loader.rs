//! Running the preprocessor over one diagram block (`TimLoader`, `BlockUml`).

use super::context::{PreprocessingArtifact, TContext};
use super::expr::TValue;
use super::memory::{TMemory, TVariableScope};
use super::resolver::Resolver;
use super::EaterException;
use crate::text::StringLocated;
use std::collections::HashMap;
use std::rc::Rc;

/// Predefined variables and environment (`Defines`).
#[derive(Debug, Clone, Default)]
pub struct Defines {
    pub environment: HashMap<String, String>,
    pub values: Vec<(String, TValue)>,
}

impl Defines {
    pub fn new() -> Self {
        let mut d = Defines::default();
        d.environment
            .insert("PLANTUML_VERSION".to_string(), crate::VERSION.to_string());
        d
    }

    pub fn define(&mut self, name: &str, value: &str) {
        self.values.push((name.to_string(), TValue::text(value)));
    }

    pub fn override_filename(&mut self, filename: &str) {
        self.environment
            .insert("filename".to_string(), filename.to_string());
        let no_ext = filename.rsplit_once('.').map_or(filename, |(n, _)| n);
        self.environment
            .insert("filenameNoExtension".to_string(), no_ext.to_string());
    }

    pub fn override_dir_path(&mut self, dir: &str) {
        self.environment
            .insert("dirpath".to_string(), dir.replace('\\', "/"));
    }
}

/// Result of preprocessing one block.
pub struct Preprocessed {
    pub lines: Vec<StringLocated>,
    pub artifact: PreprocessingArtifact,
    pub error: Option<EaterException>,
}

/// Splits lines holding `Jaws.BLOCK_E1_BREAKLINE` outside of `{{ }}` (`mutateExpandsBreakline`).
pub fn mutate_expands(lines: &mut Vec<StringLocated>) {
    let mut i = 0;
    while i < lines.len() {
        let sl = lines[i].clone();
        let chars: Vec<char> = sl.string().chars().collect();
        if !chars.contains(&'\u{e103}') {
            i += 1;
            continue;
        }
        let mut pending = String::new();
        let mut level = 0;
        let mut first_written = false;
        let mut inserted = 0;
        let mut j = 0;
        while j < chars.len() {
            let ch = chars[j];
            if ch == '{' && chars.get(j + 1) == Some(&'{') {
                level += 1;
            } else if ch == '}' && chars.get(j + 1) == Some(&'}') {
                level -= 1;
            }
            if level > 0 {
                pending.push(ch);
            } else if ch == '\u{e103}' {
                let piece = StringLocated::new(std::mem::take(&mut pending), sl.location());
                if !first_written {
                    lines[i] = piece;
                    first_written = true;
                } else {
                    lines.insert(i + 1 + inserted, piece);
                    inserted += 1;
                }
            } else {
                pending.push(ch);
            }
            j += 1;
        }
        let tail = StringLocated::new(pending, sl.location());
        if !first_written {
            lines[i] = tail;
        } else {
            lines.insert(i + 1 + inserted, tail);
            inserted += 1;
        }
        i += inserted + 1;
    }
}

pub fn preprocess(
    block: Vec<StringLocated>,
    defines: &Defines,
    resolver: Rc<dyn Resolver>,
    definitions: HashMap<String, Vec<String>>,
) -> Preprocessed {
    let raw = crate::preproc::remove_yaml_header_lines(block);
    let mut cx = TContext::new(defines.environment.clone(), resolver);
    cx.definitions = definitions;
    let mut global = TMemory::new_global();
    let first = raw
        .first()
        .cloned()
        .unwrap_or_else(|| StringLocated::new("", None));
    let mut error = None;
    for (name, value) in &defines.values {
        if let Err(e) =
            global.put_variable(name, value.clone(), Some(TVariableScope::Global), &first)
        {
            error = Some(e);
        }
    }
    if error.is_none() {
        if let Err(e) = cx.execute_lines(&mut global, &raw, None, false) {
            cx.result_list.push(e.location.clone());
            error = Some(e);
        }
    }
    let mut lines = std::mem::take(&mut cx.result_list);
    mutate_expands(&mut lines);
    Preprocessed {
        lines,
        artifact: cx.artifact,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tim::resolver::{MemoryResolver, NoResolver};

    fn run(src: &str) -> Vec<String> {
        let blocks = crate::preproc::split_blocks(src, &[]);
        let p = preprocess(
            blocks[0].clone(),
            &Defines::new(),
            Rc::new(NoResolver),
            HashMap::new(),
        );
        assert!(p.error.is_none(), "{:?}", p.error);
        p.lines.iter().map(|l| l.string().to_string()).collect()
    }

    #[test]
    fn variables_and_expressions() {
        let out = run("@startuml\n!$a = 2\n!$b = $a * 3 + 1\nA -> B : $a $b\n' c\n@enduml");
        assert_eq!(out, ["@startuml", "A -> B : 2 7", "@enduml"]);
    }

    #[test]
    fn conditionals_and_loops() {
        let out = run(
            "@startuml\n!$i = 0\n!while $i < 3\nX$i\n!$i = $i + 1\n!endwhile\n!if $i == 3\nyes\n!elseif 1\nno\n!else\nelse\n!endif\n@enduml",
        );
        assert_eq!(out, ["@startuml", "X0", "X1", "X2", "yes", "@enduml"]);
    }

    #[test]
    fn functions_and_procedures() {
        let out = run(
            "@startuml\n!function $double($x)\n!return $x * 2\n!endfunction\n!procedure $p($a, $b=5)\nP $a $b\n!endprocedure\nvalue $double(21)\n$p(1)\n$p(1, 2)\n!define SQ(x) x*x\nSQ(3)\n!define K 7\nK\n@enduml",
        );
        assert_eq!(
            out,
            [
                "@startuml",
                "value 42",
                "P 1 5",
                "P 1 2",
                "3*3",
                "7",
                "@enduml"
            ]
        );
    }

    #[test]
    fn builtins_and_json() {
        let out = run(
            "@startuml\n!$j = {\"a\": [1, 2], \"b\": \"x\"}\n%upper(\"ab\") %strlen(\"abc\") $j.b $j.a[1]\n!foreach $k in $j.a\nitem $k\n!endfor\n%substr(\"hello\", 1, 3) %strpos(\"abc\", \"c\")\n@enduml",
        );
        assert_eq!(
            out,
            [
                "@startuml",
                "AB 3 x 2",
                "item 1",
                "item 2",
                "ell 2",
                "@enduml"
            ]
        );
    }

    #[test]
    fn includes_and_subs() {
        let mut r = MemoryResolver::default();
        r.files.insert(
            "a.puml".to_string(),
            "from a\n!startsub X\nsubx\n!endsub\n".to_string(),
        );
        let blocks = crate::preproc::split_blocks(
            "@startuml\n!include a.puml\n!includesub a.puml!X\n@enduml",
            &[],
        );
        let p = preprocess(
            blocks[0].clone(),
            &Defines::new(),
            Rc::new(r),
            HashMap::new(),
        );
        assert!(p.error.is_none());
        let out: Vec<String> = p.lines.iter().map(|l| l.string().to_string()).collect();
        assert_eq!(out, ["@startuml", "from a", "subx", "subx", "@enduml"]);
    }

    #[test]
    fn errors_and_options() {
        let blocks = crate::preproc::split_blocks("@startuml\n!assert 1 == 2 : boom\n@enduml", &[]);
        let p = preprocess(
            blocks[0].clone(),
            &Defines::new(),
            Rc::new(NoResolver),
            HashMap::new(),
        );
        assert_eq!(p.error.unwrap().message, "Assertion error : boom");
        let blocks = crate::preproc::split_blocks(
            "@startuml\n!option handwritten\n!option nothing 1\n@enduml",
            &[],
        );
        let p = preprocess(
            blocks[0].clone(),
            &Defines::new(),
            Rc::new(NoResolver),
            HashMap::new(),
        );
        assert_eq!(p.artifact.warnings.len(), 1);
        assert!(p
            .artifact
            .options
            .contains_key(&crate::tim::context::OptionKey::Handwritten));
    }

    #[test]
    fn imports_zip_archives() {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, body) in [
            ("lib/a.iuml", "!include sub/b.iuml\nAlice -> Bob : A"),
            ("lib/sub/b.iuml", "Bob -> Carol : B"),
            ("d.puml", "@startuml\nDave -> Eve : D\n@enduml"),
        ] {
            zip.start_file(name, SimpleFileOptions::default())
                .expect("entry");
            zip.write_all(body.as_bytes()).expect("write");
        }
        let archive = zip.finish().expect("finish").into_inner();

        let mut r = MemoryResolver::default();
        r.binaries.insert("a.zip".to_string(), archive);
        let blocks = crate::preproc::split_blocks(
            "@startuml\n!import a.zip\n!include lib/a.iuml\n!include d.puml\n@enduml",
            &[],
        );
        let p = preprocess(
            blocks[0].clone(),
            &Defines::new(),
            Rc::new(r),
            HashMap::new(),
        );
        assert!(p.error.is_none(), "{:?}", p.error);
        let out: Vec<String> = p.lines.iter().map(|l| l.string().to_string()).collect();
        assert_eq!(
            out,
            [
                "@startuml",
                "Bob -> Carol : B",
                "Alice -> Bob : A",
                "Dave -> Eve : D",
                "@enduml"
            ]
        );
    }

    #[test]
    fn import_errors() {
        for (src, archives) in [
            ("!import missing.zip", vec![]),
            ("!import bad.zip", vec![("bad.zip", b"not a zip".to_vec())]),
        ] {
            let mut r = MemoryResolver::default();
            for (name, bytes) in archives {
                r.binaries.insert(name.to_string(), bytes);
            }
            let blocks = crate::preproc::split_blocks(&format!("@startuml\n{src}\n@enduml"), &[]);
            let p = preprocess(
                blocks[0].clone(),
                &Defines::new(),
                Rc::new(r),
                HashMap::new(),
            );
            assert_eq!(p.error.unwrap().message, "Cannot import");
        }
    }
}
