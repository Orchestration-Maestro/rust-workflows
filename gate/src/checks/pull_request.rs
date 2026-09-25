//! A pull request as the gate reads it: the change against its base, taken
//! from the merge commit's first parent, the lines each file adds and the
//! lines it touches, the type its conventional title declares, and whether
//! its title and head branch follow the organization's conventions: PRL-003,
//! a title the commit-msg hook `conventional-commit-header` would accept,
//! since a squash merge makes it the commit release-please reads, and
//! PRL-004, a branch named `<type>/<name>` or one a bot names.

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

/// The types a conventional title or branch opens with, in the order the
/// Conventional Commits convention lists them.
const TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// The most characters a title's subject holds, as the commit-msg hook counts.
const SUBJECT: usize = 71;

/// The branches bots open pull requests from: one name, or a prefix.
const BOT_BRANCHES: &[&str] = &[
    "maestro/sync",
    "release-please--",
    "dependabot/",
    "gh-readonly-queue/",
];

/// The PRL-003 and PRL-004 findings of a pull request's `title` and head
/// `branch`, one line each, naming what is wrong and the form expected.
pub(crate) fn name_findings(title: &str, branch: &str) -> Vec<String> {
    let types = TYPES.join(", ");
    let mut findings = Vec::new();
    if !conventional_title(title) {
        findings.push(format!(
            "PRL-003 the title `{title}` is not a conventional header: write \
             `<type>(<scope>)!: <subject>`, the type one of {types}, the lowercase scope \
             and the `!` optional, the subject opening in lowercase within {SUBJECT} \
             characters"
        ));
    }
    if !conventional_branch(branch) {
        findings.push(format!(
            "PRL-004 the branch `{branch}` is not named `<type>/<name>`: the type one of \
             {types}, then segments in lowercase kebab-case separated by `/`, such as \
             feat/refuse-a-title"
        ));
    }
    findings
}

/// Whether `title` is a header the commit-msg hook `conventional-commit-header`
/// accepts: a type, a lowercase scope and a `!`, both optional, `: `, then a
/// subject opening with a lowercase letter, at most 71 characters long.
fn conventional_title(title: &str) -> bool {
    let Some((header, subject)) = title.split_once(": ") else {
        return false;
    };
    let header = header.strip_suffix('!').unwrap_or(header);
    let (kind, scope) = match header.split_once('(') {
        Some((kind, rest)) => match rest.strip_suffix(')') {
            Some(scope) => (kind, Some(scope)),
            None => return false,
        },
        None => (header, None),
    };
    let scope_ok = scope.is_none_or(|scope| {
        !scope.is_empty()
            && scope
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    });
    TYPES.contains(&kind)
        && scope_ok
        && subject.starts_with(|first: char| first.is_ascii_lowercase())
        && subject.chars().count() <= SUBJECT
        && !subject.contains('\n')
}

/// Whether `branch` is `<type>/<segment>[/<segment>...]`, each segment lowercase
/// kebab-case, or a branch a bot opens.
fn conventional_branch(branch: &str) -> bool {
    if BOT_BRANCHES.iter().any(|bot| {
        if bot.ends_with('/') || bot.ends_with('-') {
            branch.len() > bot.len() && branch.starts_with(bot)
        } else {
            branch == *bot
        }
    }) {
        return true;
    }
    let mut parts = branch.split('/');
    let kind = parts.next().unwrap_or_default();
    let segments: Vec<&str> = parts.collect();
    TYPES.contains(&kind) && !segments.is_empty() && segments.iter().all(|part| kebab(part))
}

/// Whether `segment` is lowercase letters and digits, runs joined by a single
/// `-`, `.` or `_`.
fn kebab(segment: &str) -> bool {
    segment.split(['-', '.', '_']).all(|run| {
        !run.is_empty()
            && run
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        added_lines, conventional_branch, conventional_title, name_findings, title_type,
        touched_lines,
    };

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

    #[test]
    fn a_title_is_held_to_the_commit_hook_header() {
        for good in [
            "feat: refuse a title",
            "fix(api)!: keep the old name",
            "build(deps-dev): bump serde from 1.0.1 to 1.0.2",
            "chore(main): release 2.4.0",
            &format!("docs: {}", "a".repeat(71)),
        ] {
            assert!(conventional_title(good), "{good}");
        }
        for bad in [
            "Update README",
            "feature: not a type",
            "feat: Capital subject",
            "feat(API): capital scope",
            "feat(): empty scope",
            "feat(api: unclosed scope",
            "feat:no space",
            "Feat: capital type",
            &format!("docs: {}", "a".repeat(72)),
        ] {
            assert!(!conventional_title(bad), "{bad}");
        }
    }

    #[test]
    fn a_branch_is_a_type_then_kebab_segments_or_a_bot() {
        for good in [
            "feat/refuse-a-title",
            "fix/a_b.c",
            "docs/adr/0010-spec-kit",
            "maestro/sync",
            "release-please--branches--main",
            "dependabot/cargo/serde-1.0.2",
            "gh-readonly-queue/main/pr-1",
        ] {
            assert!(conventional_branch(good), "{good}");
        }
        for bad in [
            "wip",
            "feature/x",
            "Feat/x",
            "feat/Upper",
            "feat/-x",
            "feat/a--b",
            "feat/",
            "feat//x",
            "maestro/sync-2",
            "dependabot/",
            "",
        ] {
            assert!(!conventional_branch(bad), "{bad}");
        }
    }

    #[test]
    fn each_finding_names_what_it_read_and_the_form() {
        assert!(name_findings("feat: a", "feat/a").is_empty());
        let findings = name_findings("Add a rule", "wip");
        assert_eq!(findings.len(), 2);
        assert!(findings[0].starts_with("PRL-003 the title `Add a rule` is not"));
        assert!(findings[1].starts_with("PRL-004 the branch `wip` is not named"));
    }
}
