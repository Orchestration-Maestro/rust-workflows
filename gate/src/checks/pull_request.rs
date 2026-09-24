//! A pull request as the gate reads it: the change against its base, taken
//! from the merge commit's first parent, the lines each file adds and the
//! lines it touches, and the type its conventional title declares.

use crate::runner::{Cmd, Failure};
use std::collections::BTreeMap;
use std::path::Path;

/// Every change of the pull request checked out at `root`, without context
/// lines: the merge commit against its first parent, the base branch.
pub(crate) fn pull_request_diff(root: &Path) -> Result<String, Failure> {
    Cmd::new("git -C")
        .arg(root)
        .args([
            "diff",
            "-U0",
            "--no-color",
            "--no-renames",
            "HEAD^1",
            "HEAD",
        ])
        .capture()
}

/// The lines each file of `diff` adds, by their number in the new file.
pub(crate) fn added_lines(diff: &str) -> BTreeMap<String, Vec<usize>> {
    let mut added: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut file: Option<String> = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file = path.strip_prefix("b/").map(str::to_owned);
        } else if let (Some(hunk), Some(path)) = (line.strip_prefix("@@ "), &file) {
            let (start, count) = new_range(hunk);
            added
                .entry(path.clone())
                .or_default()
                .extend(start..start + count);
        }
    }
    added.retain(|_, lines| !lines.is_empty());
    added
}

/// How many lines each file of `diff` adds and removes, together.
pub(crate) fn touched_lines(diff: &str) -> BTreeMap<String, usize> {
    let mut touched: BTreeMap<String, usize> = BTreeMap::new();
    let mut file = String::new();
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("--- a/") {
            path.clone_into(&mut file);
        } else if let Some(path) = line.strip_prefix("+++ b/") {
            path.clone_into(&mut file);
        } else if (line.starts_with('+') || line.starts_with('-'))
            && !line.starts_with("+++ ")
            && !line.starts_with("--- ")
        {
            *touched.entry(file.clone()).or_default() += 1;
        }
    }
    touched
}

/// The first line and the line count of a hunk's new side: `-a,b +c,d @@`.
fn new_range(hunk: &str) -> (usize, usize) {
    let new = hunk
        .split_whitespace()
        .find_map(|word| word.strip_prefix('+'))
        .unwrap_or_default();
    let (start, count) = new.split_once(',').unwrap_or((new, "1"));
    (start.parse().unwrap_or(0), count.parse().unwrap_or(0))
}

/// The type a conventional title declares: `feat` in `feat(api)!: add`.
pub(crate) fn title_type(title: &str) -> Option<&str> {
    let (header, _) = title.split_once(':')?;
    let header = header.strip_suffix('!').unwrap_or(header);
    let kind = header.split_once('(').map_or(header, |(kind, _)| kind);
    (!kind.is_empty() && kind.chars().all(|character| character.is_ascii_lowercase()))
        .then_some(kind)
}

#[cfg(test)]
mod tests {
    use super::{added_lines, title_type, touched_lines};

    /// A change to two files: one line replaced and two added, one deleted.
    const DIFF: &str = concat!(
        "diff --git a/src/lib.rs b/src/lib.rs\n",
        "--- a/src/lib.rs\n",
        "+++ b/src/lib.rs\n",
        "@@ -3 +3 @@ fn a() {\n",
        "-    old();\n",
        "+    new();\n",
        "@@ -9,0 +10,2 @@ fn b() {\n",
        "+    one();\n",
        "+    two();\n",
        "diff --git a/src/gone.rs b/src/gone.rs\n",
        "--- a/src/gone.rs\n",
        "+++ /dev/null\n",
        "@@ -1 +0,0 @@\n",
        "-fn gone() {}\n",
    );

    #[test]
    fn a_diff_gives_each_file_its_added_and_touched_lines() {
        let added = added_lines(DIFF);
        assert_eq!(added.len(), 1);
        assert_eq!(added["src/lib.rs"], [3, 10, 11]);
        let touched = touched_lines(DIFF);
        assert_eq!(touched["src/lib.rs"], 4);
        assert_eq!(touched["src/gone.rs"], 1);
    }

    #[test]
    fn a_title_declares_its_type_before_scope_and_bang() {
        assert_eq!(title_type("feat(api)!: add a product"), Some("feat"));
        assert_eq!(title_type("fix: refuse an overflow"), Some("fix"));
        assert_eq!(title_type("Update README"), None);
        assert_eq!(title_type("Docs: capitals are not a type"), None);
    }
}
