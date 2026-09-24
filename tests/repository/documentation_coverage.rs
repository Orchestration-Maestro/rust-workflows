//! The documentation: every report, input, output and secret has its row, every
//! link resolves, and every test the standards cite exists.

use crate::harness::{described, query, root, test_sources};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn every_report_the_run_produces_is_documented() {
    // A report nobody documented is a report nobody looks for. The list
    // drifts silently whenever a gate is added, so it comes from what the
    // `ci.yml` steps declare they write, not from a hand-kept list.
    let reference = fs::read_to_string(root().join("docs/ci.md")).unwrap();
    let mut produced: Vec<String> = described()
        .into_iter()
        .filter(|step| step.workflow == "ci")
        .flat_map(|step| step.reports)
        .collect();
    produced.sort();
    produced.dedup();
    assert!(
        produced.len() > 5,
        "the ci.yml steps declare almost no report; the declarations drifted"
    );
    for report in &produced {
        assert!(
            reference.contains(report.as_str()),
            "ci.yml writes {report} but docs/ci.md never mentions it"
        );
    }
}

#[test]
fn every_relative_link_in_the_repository_resolves() {
    // The earlier check only followed links ending in `.md`, so links to a
    // directory kept pointing at `docs/engineering-baseline/` for a whole
    // reorganisation without anything noticing.
    let root = root();
    let mut checked = 0;
    let mut broken = Vec::new();
    for path in markdown_files(&root) {
        let text = fs::read_to_string(&path).unwrap();
        let parent = path.parent().unwrap();
        for file in link_targets(&text) {
            checked += 1;
            if !parent.join(&file).exists() {
                broken.push(format!("{} -> {file}", path.display()));
            }
        }
        for span in path_tokens(&text) {
            checked += 1;
            if !parent.join(&span).exists() && !root.join(&span).exists() {
                broken.push(format!("{} -> `{span}`", path.display()));
            }
        }
    }
    assert!(
        checked > 20,
        "link extraction found almost nothing; the pattern has drifted"
    );
    assert!(
        broken.is_empty(),
        "broken relative links:\n  {}",
        broken.join("\n  ")
    );
}

/// Every Markdown file of the repository, outside the ignored directories.
fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut queue = vec![root.to_path_buf()];
    while let Some(directory) = queue.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if matches!(name.as_str(), ".git" | ".tools" | "target") {
                continue;
            }
            if path.is_dir() {
                queue.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    files
}

/// The relative file every Markdown link points at; anchors, absolute URLs
/// and mail links are out of scope.
fn link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("](") {
        rest = &rest[open + 2..];
        let Some(close) = rest.find(')') else { break };
        let target = &rest[..close];
        rest = &rest[close..];
        if target.starts_with("http") || target.starts_with('#') || target.contains(':') {
            continue;
        }
        let file = target.split('#').next().unwrap();
        if !file.is_empty() {
            targets.push(file.to_owned());
        }
    }
    targets
}

/// This reads backticked prose, where `link_targets` above reads Markdown
/// link syntax; the two rules stay apart so a refusal names which one caught
/// the path.
/// Prose names a path in backticks as often as it links it, and a renamed
/// directory kept its old name in that prose through the same
/// reorganisation. A backticked token that names a directory, or a path under
/// one of this repository's top-level directories, must exist from the file's
/// own directory or from the root.
fn path_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for (index, span) in text.split('`').enumerate() {
        let path_like = span.ends_with('/')
            || ["docs/", ".github/", "examples/", "tests/", "scripts/"]
                .iter()
                .any(|top| span.starts_with(top));
        let opaque = matches!(span, "./" | "///" | "target/" | ".tools/")
            || span.contains(char::is_whitespace)
            || span.contains([':', '$', '*', '{', '<', '~', '\\']);
        if index % 2 == 1 && path_like && !opaque {
            tokens.push(span.to_owned());
        }
    }
    tokens
}

#[test]
fn every_input_output_and_secret_is_documented() {
    // Four inputs had gone undocumented here before this test existed. An input needs a
    // table row, an output or a secret at least a backticked mention. The helper
    // actions are public surface too, documented in the same place.
    let root = root();
    let docs: String = ["docs/ci.md", "docs/publishing.md"]
        .iter()
        .map(|doc| fs::read_to_string(root.join(doc)).unwrap())
        .collect();
    let mut files: Vec<PathBuf> = fs::read_dir(root.join(".github/workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.extend(
        fs::read_dir(root.join(".github/actions"))
            .unwrap()
            .map(|entry| entry.unwrap().path().join("action.yml")),
    );
    let mut checked = 0;
    for path in files {
        for name in query(
            &path,
            "(.on.workflow_call.inputs // .inputs // {}) | keys[]",
        )
        .lines()
        {
            assert!(
                docs.contains(&format!("| `{name}` |")),
                "{}: input `{name}` has no row in docs/ci.md or docs/publishing.md",
                path.display()
            );
            checked += 1;
        }
        for name in query(
            &path,
            "(.on.workflow_call.secrets // {}) + (.outputs // {}) | keys[]",
        )
        .lines()
        {
            assert!(
                docs.contains(&format!("`{name}`")),
                "{}: `{name}` is not mentioned in docs/ci.md or docs/publishing.md",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked > 30,
        "only {checked} inputs found; the query drifted"
    );
}

#[test]
fn every_test_the_standards_cite_exists() {
    // The standards tables prove a rule by naming the test that enforces it. A
    // renamed test leaves a citation that proves nothing, and nothing else in
    // the gate reads those tables.
    let root = root();
    let mut defined = String::new();
    for path in test_sources() {
        defined.push_str(&fs::read_to_string(path).unwrap());
    }
    // A backticked identifier can also be a GitHub event, an input or a
    // variable; those live in the workflow files, so anything found there is
    // vocabulary, not a citation.
    let mut vocabulary = String::new();
    let mut pending = vec![root.join(".github")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(text) = fs::read_to_string(&path) {
                vocabulary.push_str(&text);
            }
        }
    }
    for file in [
        "action.yml",
        "justfile",
        "mise.toml",
        "scripts/bootstrap.sh",
    ] {
        if let Ok(text) = fs::read_to_string(root.join(file)) {
            vocabulary.push_str(&text);
        }
    }
    let mut cited = 0;
    for document in [
        "docs/standards/engineering.md",
        "docs/standards/security.md",
        "docs/standards/northstar.md",
        "CONTEXT.md",
        "CONTRIBUTING.md",
        "AGENTS.md",
    ] {
        let text = fs::read_to_string(root.join(document)).unwrap();
        for (index, span) in text.split('`').enumerate() {
            let looks_like_a_test = index % 2 == 1
                && span.len() >= 12
                && span.contains('_')
                && span.starts_with(|character: char| character.is_ascii_lowercase())
                && span.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                });
            if !looks_like_a_test || vocabulary.contains(span) {
                continue;
            }
            assert!(
                defined.contains(&format!("fn {span}(")),
                "{document} cites `{span}`, which no test defines"
            );
            cited += 1;
        }
    }
    assert!(
        cited >= 5,
        "only {cited} test citations found; the scan drifted"
    );
}
