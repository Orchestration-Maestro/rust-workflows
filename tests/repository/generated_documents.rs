//! Generated documents: every table between generated markers, and the
//! diagram's control count, is what `just docs` writes from its source, so a
//! reader never has to keep one in step with the other by hand.

use crate::harness::{root, succeeds, temp_dir, tool};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

/// Every file `just docs` rewrites, besides the steps document.
fn generated(dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        PathBuf::from("README.md"),
        PathBuf::from(".github/assets/how-it-works.svg"),
    ];
    for entry in fs::read_dir(dir.join("docs")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|kind| kind == "md") {
            files.push(path.strip_prefix(dir).unwrap().to_path_buf());
        }
    }
    files
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

/// A copy of the documents and of every source they are generated from.
fn sandbox() -> PathBuf {
    let dir = temp_dir("generated-documents");
    for file in ["justfile", "README.md", "mise.toml"] {
        fs::copy(root().join(file), dir.join(file)).unwrap();
    }
    copy_tree(&root().join("docs"), &dir.join("docs"));
    copy_tree(
        &root().join(".github/workflows"),
        &dir.join(".github/workflows"),
    );
    copy_tree(&root().join(".github/assets"), &dir.join(".github/assets"));
    let scorecard = "gate/src/steps/quality_scorecard";
    copy_tree(&root().join(scorecard), &dir.join(scorecard));
    dir
}

fn render(dir: &Path) -> Output {
    tool("just")
        .arg("--justfile")
        .arg(dir.join("justfile"))
        .arg("--working-directory")
        .arg(dir)
        .arg("_tables")
        .output()
        .unwrap()
}

#[test]
fn every_generated_table_is_what_its_source_says() {
    let dir = sandbox();
    succeeds(&render(&dir));
    for file in generated(&root()) {
        assert_eq!(
            fs::read_to_string(root().join(&file)).unwrap(),
            fs::read_to_string(dir.join(&file)).unwrap(),
            "{} is stale: run just docs",
            file.display()
        );
    }
}

#[test]
fn a_changed_source_changes_its_table() {
    let dir = sandbox();
    let fuzz = dir.join(".github/workflows/fuzz.yml");
    let text = fs::read_to_string(&fuzz).unwrap();
    let described = text
        .find("      target:\n        description: >-\n")
        .unwrap();
    let start = described + "      target:\n        description: >-\n".len();
    let end = start + text[start..].find("\n        type:").unwrap();
    fs::write(
        &fuzz,
        format!(
            "{}          Changed at its source{}",
            &text[..start],
            &text[end..]
        ),
    )
    .unwrap();
    succeeds(&render(&dir));
    let ci = fs::read_to_string(dir.join("docs/ci.md")).unwrap();
    assert!(
        ci.contains("| `target` | Empty | Changed at its source |"),
        "{ci}"
    );
}

#[test]
fn a_pinned_tool_without_its_description_is_refused() {
    // A tool mise.toml pins without its `# tool:` line would vanish from the
    // README's toolbelt, so rendering stops instead.
    let dir = sandbox();
    let mise = dir.join("mise.toml");
    let text = fs::read_to_string(&mise).unwrap();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("# tool: jaq |"))
        .collect();
    fs::write(&mise, kept.join("\n")).unwrap();
    let output = render(&dir);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("pins jaq without a '# tool:' line"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
