//! The size limits the repository holds its own code to beyond what
//! `rust-gate architecture` holds (SIZE-002 and SIZE-003 on every Rust file):
//! functions through Clippy's thresholds, and the width of shell scripts and
//! the justfile, which no module tree reaches.

use crate::harness::root;
use std::fs;
use std::path::{Path, PathBuf};

/// Every source file the limits apply to.
fn source_files(root: &Path, extensions: &[&str], names: &[&str]) -> Vec<PathBuf> {
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
            } else if extensions
                .iter()
                .any(|e| path.extension().is_some_and(|x| x == *e))
                || names.contains(&name.as_str())
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_crate_holds_the_complexity_limits() {
    // The lint levels live in each crate's Cargo.toml, so every `cargo clippy`
    // applies them, an editor's included; the thresholds live in the root
    // clippy.toml, the one file Clippy reads walking up from where cargo runs;
    // and the justfile's
    // `-D warnings` is what turns them into errors. A crate missing any of the
    // three would be linted at Clippy's looser defaults without turning red.
    let root = root();
    let crates = ["gate", "tests"];
    for crate_dir in crates {
        let manifest = fs::read_to_string(root.join(crate_dir).join("Cargo.toml")).unwrap();
        for line in [
            "[lints.clippy]",
            "pedantic = { level = \"warn\", priority = -1 }",
            "cognitive_complexity = \"warn\"",
            "too_many_lines = \"warn\"",
            "too_many_arguments = \"warn\"",
        ] {
            assert!(
                manifest.contains(line),
                "{crate_dir}/Cargo.toml lacks {line}"
            );
        }
    }
    let config = fs::read_to_string(root.join("clippy.toml")).unwrap();
    for line in [
        "cognitive-complexity-threshold = 15",
        "too-many-lines-threshold = 100",
        "too-many-arguments-threshold = 5",
    ] {
        assert!(config.contains(line), "clippy.toml lacks {line}");
    }
    // The justfile runs Clippy over every crate from one loop, `-D warnings`
    // turning the size lints into errors.
    let justfile = fs::read_to_string(root.join("justfile")).unwrap();
    let looped = justfile
        .lines()
        .find_map(|line| line.trim().strip_prefix("for manifest in "))
        .and_then(|rest| rest.strip_suffix("; do"))
        .unwrap_or_default();
    for crate_dir in crates {
        assert!(
            looped.split_whitespace().any(|word| word == crate_dir),
            "the justfile's crate loop must name {crate_dir}: {looped}"
        );
    }
    assert!(
        justfile.contains("cargo clippy --manifest-path \"$manifest/Cargo.toml\"")
            && justfile.contains("-- -D warnings"),
        "the justfile must run Clippy with -D warnings on every crate"
    );
}

#[test]
fn no_code_line_exceeds_one_hundred_columns() {
    // The width the editor configuration declares for Rust and shell, held
    // here because rustfmt wraps code but not a string literal, and nothing
    // wraps a justfile. Workflow YAML is not scanned: a pinned action and a
    // release asset with its digest are single tokens longer than any limit.
    let root = root();
    let mut over = Vec::new();
    let mut scanned = 0;
    for path in source_files(&root, &["rs", "sh"], &["justfile"]) {
        scanned += 1;
        for (number, line) in fs::read_to_string(&path).unwrap().lines().enumerate() {
            if line.chars().count() > 100 {
                over.push(format!(
                    "{}:{}",
                    path.strip_prefix(&root).unwrap().display(),
                    number + 1
                ));
            }
        }
    }
    assert!(
        scanned > 30,
        "only {scanned} files scanned; the walk drifted"
    );
    assert!(
        over.is_empty(),
        "lines over 100 columns:\n  {}",
        over.join("\n  ")
    );
}

#[test]
fn the_binary_refuses_every_way_to_panic() {
    // A refusal is a message and an exit status; a panic is neither. The
    // lints hold the binary to that, and its own unit tests keep `unwrap`.
    let root = root();
    let manifest = fs::read_to_string(root.join("gate/Cargo.toml")).unwrap();
    for lint in [
        "unwrap_used",
        "expect_used",
        "panic",
        "unreachable",
        "todo",
        "unimplemented",
        "dbg_macro",
    ] {
        let line = format!("{lint} = \"warn\"");
        assert!(manifest.contains(&line), "gate/Cargo.toml lacks {line}");
    }
    let config = fs::read_to_string(root.join("clippy.toml")).unwrap();
    for allowance in ["unwrap", "expect", "panic"] {
        let line = format!("allow-{allowance}-in-tests = true");
        assert!(config.contains(&line), "clippy.toml lacks {line}");
    }
}
