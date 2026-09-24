//! SIZE-002 and SIZE-003: a file holds at most the organization's lines of
//! code, doc comments not counted, and a line at most its columns. A file
//! past three hundred lines is reported without being refused.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::quality_config::Limits;
use std::collections::BTreeSet;
use std::path::Path;

/// Files longer than this many lines of code are reported.
const REPORTED: usize = 300;

/// SIZE-002 and SIZE-003 over every module file of `trees`, each once, and
/// the report lines for the files past three hundred lines.
pub(super) fn findings(
    trees: &[Tree],
    workspace: &Path,
    limits: Limits,
) -> (Vec<Finding>, Vec<String>) {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    let mut notes = Vec::new();
    for module in trees.iter().flat_map(|tree| &tree.modules) {
        if !seen.insert(&module.file) {
            continue;
        }
        let file = relative(workspace, &module.file);
        let lines = code_lines(&module.source);
        if lines > limits.file_lines {
            let message = format!(
                "{lines} lines of code, over {}; split it by what varies \
                 (doc comments are not counted)",
                limits.file_lines
            );
            found.push(Finding::new("SIZE-002", file.clone(), 0, message));
        } else if lines > REPORTED {
            notes.push(format!(
                "NOTE SIZE-002 {file}: {lines} lines of code, over {REPORTED}; \
                 reported, not refused"
            ));
        }
        for (number, line) in module.source.lines().enumerate() {
            let columns = line.chars().count();
            if columns > limits.line_columns {
                let message = format!(
                    "{columns} columns, over {}; wrap it, strings and comments included",
                    limits.line_columns
                );
                found.push(Finding::new("SIZE-003", file.clone(), number + 1, message));
            }
        }
    }
    (found, notes)
}

/// The lines of a source that are not documentation.
fn code_lines(source: &str) -> usize {
    source
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("///") && !line.starts_with("//!")
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use crate::checks::quality_config::Limits;
    use std::path::Path;

    #[test]
    fn long_files_and_wide_lines_are_refused_and_documentation_is_free() {
        let long = "fn f() {}\n".repeat(12);
        let documented = format!("{}fn g() {{}}\n", "/// Explained.\n".repeat(40));
        let wide = format!("fn h() {{ let _ = \"{}\"; }}\n", "x".repeat(30));
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", &long),
                ("documented.rs", &documented),
                ("wide.rs", &wide),
            ],
        );
        let limits = Limits {
            file_lines: 10,
            line_columns: 40,
        };
        let (found, notes) = findings(&[tree], Path::new("/w"), limits);
        let found: Vec<String> = found.iter().map(ToString::to_string).collect();
        assert_eq!(
            found,
            [
                "SIZE-002 src/lib.rs: 12 lines of code, over 10; split it by what varies \
                 (doc comments are not counted)",
                "SIZE-003 src/wide.rs:1: 52 columns, over 40; wrap it, strings and comments \
                 included",
            ]
        );
        assert!(notes.is_empty());
    }

    #[test]
    fn a_file_past_three_hundred_lines_is_reported_not_refused() {
        let source = "fn f() {}\n".repeat(301);
        let tree = sample(&["lib"], &[("lib.rs", &source)]);
        let (found, notes) = findings(&[tree], Path::new("/w"), Limits::default());
        assert!(found.is_empty());
        assert_eq!(
            notes,
            ["NOTE SIZE-002 src/lib.rs: 301 lines of code, over 300; reported, not refused"]
        );
    }
}
