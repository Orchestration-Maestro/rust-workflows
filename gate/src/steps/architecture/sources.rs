//! DOC-001, LIB-001 and TST-001, read from the source: every file says what
//! its module is for, a library never prints, and a test never waits on
//! time.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::{is_identifier_byte, line_at, without_tests};
use crate::checks::rust_tests::{only_tests, waits};
use std::collections::BTreeSet;
use std::path::Path;

/// The macros that print to the process's own streams.
const PRINTS: &[&str] = &["print", "println", "eprint", "eprintln"];

/// DOC-001 over every module file of `trees`, each once.
pub(super) fn module_comments(trees: &[Tree], workspace: &Path) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for module in trees.iter().flat_map(|tree| &tree.modules) {
        let first = module.source.lines().find(|line| !line.trim().is_empty());
        let documented = first.is_some_and(|line| line.trim_start().starts_with("//!"));
        if seen.insert(&module.file) && !documented {
            found.push(Finding::new(
                "DOC-001",
                relative(workspace, &module.file),
                0,
                "the file does not open with a //! comment saying what the module is for"
                    .to_owned(),
            ));
        }
    }
    found
}

/// LIB-001 over the library trees of `trees`: every print macro outside
/// test items.
pub(super) fn library_prints(trees: &[Tree], workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for tree in trees.iter().filter(|tree| tree.is_library()) {
        for module in &tree.modules {
            let code = without_tests(&module.code);
            let prints = macro_calls(&code)
                .into_iter()
                .filter(|(_, name)| PRINTS.contains(name));
            for (offset, name) in prints {
                found.push(Finding::new(
                    "LIB-001",
                    relative(workspace, &module.file),
                    line_at(&code, offset),
                    format!(
                        "`{name}!` in a library; return the value or report through tracing, \
                         and let the binary print"
                    ),
                ));
            }
        }
    }
    found
}

/// TST-001 over one tree: every wait on time in its test code, the whole of
/// a test target and the test items of any other.
pub(super) fn test_sleeps(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let test_target = tree.kinds.iter().any(|kind| kind == "test");
    let mut found = Vec::new();
    for module in &tree.modules {
        let code = if test_target {
            module.code.clone()
        } else {
            only_tests(&module.code)
        };
        for wait in waits(&code) {
            let message = format!(
                "`{}` waits on time; wait on a fake clock or a synchronisation primitive",
                wait.path
            );
            let finding = Finding::new(
                "TST-001",
                relative(workspace, &module.file),
                wait.line,
                message,
            );
            found.push(finding.about(wait.function.as_deref().unwrap_or_default()));
        }
    }
    found
}

/// Every macro call of blanked code: the offset of its name and the name.
fn macro_calls(code: &str) -> Vec<(usize, &str)> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    for (bang, _) in code.match_indices('!') {
        let start = bytes
            .get(..bang)
            .unwrap_or_default()
            .iter()
            .rposition(|&byte| !is_identifier_byte(byte))
            .map_or(0, |offset| offset + 1);
        if let Some(name) = code.get(start..bang).filter(|name| !name.is_empty()) {
            found.push((start, name));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{library_prints, module_comments, test_sleeps};
    use crate::checks::findings::Finding;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    /// Every finding as its report line.
    fn lines(found: &[Finding]) -> Vec<String> {
        found.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_file_opening_without_a_module_comment_is_refused() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "\n//! Crate.\nmod a;\n"),
                ("a.rs", "#![forbid(unsafe_code)]\n"),
            ],
        );
        assert_eq!(
            lines(&module_comments(&[tree], Path::new("/w"))),
            [
                "DOC-001 src/a.rs: the file does not open with a //! comment saying what the \
                 module \
              is for"
            ]
        );
    }

    #[test]
    fn a_library_prints_nowhere_but_its_tests_and_a_binary_may() {
        let source = concat!(
            "pub fn f() {\n    println!(\"x\");\n",
            "    std::eprint!(\"y\");\n    format!(\"z\");\n}\n",
            "#[cfg(test)]\nmod tests {\n    fn t() { println!(\"ok\"); }\n}\n",
        );
        let library = sample(&["lib"], &[("lib.rs", source)]);
        let binary = sample(&["bin"], &[("main.rs", source)]);
        assert_eq!(
            lines(&library_prints(&[library, binary], Path::new("/w"))),
            [
                "LIB-001 src/lib.rs:2: `println!` in a library; return the value or report \
                 through tracing, and let the binary print",
                "LIB-001 src/lib.rs:3: `eprint!` in a library; return the value or report \
                 through tracing, and let the binary print",
            ]
        );
    }

    #[test]
    fn a_wait_on_time_in_test_code_is_refused_with_its_function() {
        let source = concat!(
            "fn live() { std::thread::sleep(d); }\n",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_waits_for_the_reply() {\n",
            "        tokio::time::sleep(d);\n    }\n}\n",
        );
        let library = sample(&["lib"], &[("lib.rs", source)]);
        let found = test_sleeps(&library, Path::new("/w"));
        assert_eq!(
            lines(&found),
            [
                "TST-001 src/lib.rs:6: `tokio::time::sleep` waits on time; wait on a fake clock or \
              a synchronisation primitive"
            ]
        );
        assert_eq!(found[0].item, "it_waits_for_the_reply");
        let test_target = sample(
            &["test"],
            &[("main.rs", "fn helper() { std::thread::sleep(d); }\n")],
        );
        assert_eq!(test_sleeps(&test_target, Path::new("/w")).len(), 1);
    }
}
