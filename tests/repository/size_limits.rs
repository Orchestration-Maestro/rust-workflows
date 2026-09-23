//! The size limits the repository holds its own code to: functions through
//! Clippy's thresholds, files by their non-documentation lines, and lines
//! by width.

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

/// Every scanned source with its line count, doc comments excluded: explaining
/// an item must never be the reason to split the module it lives in.
fn measured_sources(root: &Path) -> Vec<(String, usize)> {
    let mut sizes = Vec::new();
    for directory in ["gate/src", "tests", "examples"] {
        for path in source_files(&root.join(directory), &["rs"], &[]) {
            let lines = fs::read_to_string(&path)
                .unwrap()
                .lines()
                .filter(|line| {
                    !line.trim_start().starts_with("///") && !line.trim_start().starts_with("//!")
                })
                .count();
            sizes.push((
                path.strip_prefix(root).unwrap().display().to_string(),
                lines,
            ));
        }
    }
    assert!(
        sizes.len() > 30,
        "only {} files scanned; the walk drifted",
        sizes.len()
    );
    sizes
}

#[test]
fn no_source_file_exceeds_five_hundred_lines() {
    // The refusal. Past five hundred lines a file stops being readable whole,
    // whatever its seams look like. Three hundred is reported rather than
    // refused, because a ceiling that forces a split lets the split be chosen
    // by size instead of by what varies, which is how a module lands in a
    // shared layer with a single caller.
    let root = root();
    let over: Vec<String> = measured_sources(&root)
        .into_iter()
        .filter(|(_, lines)| *lines > 500)
        .map(|(name, lines)| format!("{name} ({lines} lines)"))
        .collect();
    assert!(
        over.is_empty(),
        "files over 500 lines:\n  {}",
        over.join("\n  ")
    );
}

#[test]
fn files_over_three_hundred_lines_are_reported() {
    // A report, not a gate: it names what is growing without refusing it, at
    // the threshold and with the intent of the `complexity` step a consumer
    // run already carries. `just check` prints the line this writes, and the
    // line is written even when the list is empty so nothing hides a zero.
    let root = root();
    let mut over: Vec<String> = measured_sources(&root)
        .into_iter()
        .filter(|(_, lines)| *lines > 300)
        .map(|(name, lines)| format!("{name} ({lines})"))
        .collect();
    over.sort();
    println!(
        "REPORT: {} source {} over 300 lines{}{}",
        over.len(),
        if over.len() == 1 { "file" } else { "files" },
        if over.is_empty() { "" } else { ": " },
        over.join(", ")
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
