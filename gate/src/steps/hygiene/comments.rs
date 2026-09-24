//! HYG-001: a `TODO`, `FIXME`, `HACK` or `XXX` in a comment names its issue,
//! `#123` or the issue's URL, so what is left for later stays on a list
//! somebody reads.

use crate::checks::findings::Finding;
use crate::checks::rust_code::comments;
use std::fs;
use std::path::Path;

/// The words that mark work left for later.
const MARKERS: &[&str] = &["TODO", "FIXME", "HACK", "XXX"];

/// The extensions whose comments open with `#`.
const HASH_EXTENSIONS: &[&str] = &["sh", "bash", "toml", "yml", "yaml"];

/// HYG-001 over the comments of every Rust, shell, TOML, YAML and justfile
/// source among `files`.
pub(super) fn findings(workspace: &Path, files: &[String]) -> Vec<Finding> {
    let mut found = Vec::new();
    for file in files {
        let rust = extension(file).as_deref() == Some("rs");
        if !rust && !hash_commented(file) {
            continue;
        }
        let Ok(text) = fs::read_to_string(workspace.join(file)) else {
            continue;
        };
        let listed = if rust {
            comments(&text)
        } else {
            hash_comments(&text)
        };
        for (line, comment) in listed {
            if let Some(marker) = unlinked_marker(comment) {
                let message = format!(
                    "`{marker}` without an issue; link one, `#123` or its URL, or do it now"
                );
                found.push(Finding::new("HYG-001", file.clone(), line, message));
            }
        }
    }
    found
}

/// Whether a file's comments open with `#`: shell, TOML, YAML and justfiles.
fn hash_commented(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or_default();
    name.eq_ignore_ascii_case("justfile")
        || extension(file).is_some_and(|extension| HASH_EXTENSIONS.contains(&extension.as_str()))
}

/// A file's extension in lowercase.
fn extension(file: &str) -> Option<String> {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

/// Every `#` comment of a text, with its line: a `#` at the start of a line
/// or after whitespace, outside quotes.
fn hash_comments(text: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let mut quote = None;
        let mut previous = ' ';
        for (offset, character) in line.char_indices() {
            match quote {
                Some(open) if character == open && previous != '\\' => quote = None,
                None if character == '"' || character == '\'' => quote = Some(character),
                None if character == '#' && previous.is_whitespace() => {
                    found.push((number + 1, line.get(offset..).unwrap_or_default()));
                    break;
                }
                _ => {}
            }
            previous = character;
        }
    }
    found
}

/// The first marker a comment holds without naming an issue.
fn unlinked_marker(comment: &str) -> Option<&'static str> {
    let linked = comment.contains("/issues/")
        || comment.match_indices('#').any(|(offset, _)| {
            comment
                .get(offset + 1..)
                .is_some_and(|rest| rest.starts_with(|character: char| character.is_ascii_digit()))
        });
    if linked {
        return None;
    }
    MARKERS.iter().copied().find(|marker| {
        comment.match_indices(marker).any(|(offset, _)| {
            let before = comment
                .get(..offset)
                .and_then(|text| text.chars().next_back());
            let after = comment
                .get(offset + marker.len()..)
                .and_then(|text| text.chars().next());
            let word = |next: Option<char>| {
                next.is_some_and(|character| character.is_alphanumeric() || character == '_')
            };
            // A marker in backticks names the word; it leaves no work.
            let quoted = before == Some('`') && after == Some('`');
            !word(before) && !word(after) && !quoted
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_comments, unlinked_marker};

    #[test]
    fn markers_need_an_issue_and_must_stand_as_words() {
        assert_eq!(unlinked_marker("// TODO: split this"), Some("TODO"));
        assert_eq!(unlinked_marker("# FIXME(#42): wrong on Windows"), None);
        assert_eq!(
            unlinked_marker("// HACK, see https://github.com/o/r/issues/7"),
            None
        );
        assert_eq!(
            unlinked_marker("// TODOS and XXXL are words of their own"),
            None
        );
        assert_eq!(unlinked_marker("//! a `TODO` names the marker"), None);
    }

    #[test]
    fn hash_comments_skip_quoted_hashes_and_shell_counts() {
        let text = "echo \"# not\" # TODO one\nn=$# ; x=${#y}\n# two\n";
        assert_eq!(hash_comments(text), [(1, "# TODO one"), (3, "# two")]);
    }
}
