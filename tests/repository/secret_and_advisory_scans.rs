//! The scans the gate runs over this repository itself: Gitleaks over the
//! source tree, and the `RustSec` advisories when the network is allowed.

use crate::harness::{capture, command_line, root, temp_dir};
use std::fs;
use std::path::Path;

#[test]
fn the_source_tree_holds_no_secret() {
    // The same scan the workflow runs on a consumer, over a copy of this
    // checkout without its build output and toolbelt, so nothing generated
    // can hide a finding or plant one.
    let root = root();
    let temp = temp_dir("secrets");
    let source = temp.join("source");
    copy_source(&root, &source);
    fs::write(temp.join("gitleaks.toml"), "[extend]\nuseDefault = true\n").unwrap();
    {
        let format = "json";
        let report = temp.join(format!("secrets.{format}"));
        capture(
            command_line("gitleaks dir")
                .arg(&source)
                .arg("--config")
                .arg(temp.join("gitleaks.toml"))
                .arg("--gitleaks-ignore-path")
                .arg(temp.join("no-ignore"))
                .args(["--ignore-gitleaks-allow", "--redact=100", "--no-banner"])
                .args(["--report-format", format, "--report-path"])
                .arg(&report),
        );
        assert!(report.is_file(), "gitleaks left no {format} report");
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn advisories_are_clean_when_the_network_is_allowed() {
    // CHECK_NETWORK=1 fetches one advisory database snapshot and audits every
    // lockfile with it; yanked crates need the registry index, so cargo-deny's
    // advisories pass runs here too. Offline, the justfile says NOT RUN.
    if std::env::var("CHECK_NETWORK").is_ok_and(|value| value == "1") {
        let root = root();
        let temp = temp_dir("advisories");
        let db = temp.join("advisory-db");
        let lockfiles = [
            "tests/Cargo.lock",
            "examples/binary/Cargo.lock",
            "examples/library/Cargo.lock",
            "examples/workspace/Cargo.lock",
        ];
        for (index, lockfile) in lockfiles.iter().enumerate() {
            let fetch = if index == 0 {
                "cargo audit --json --db"
            } else {
                "cargo audit --no-fetch --json --db"
            };
            let report = capture(
                command_line(fetch)
                    .arg(&db)
                    .arg("--file")
                    .arg(lockfile)
                    .current_dir(&root),
            );
            assert!(
                !report.trim().is_empty(),
                "{lockfile}: cargo audit produced no report"
            );
        }
        {
            let manifest = "tests/Cargo.toml";
            capture(
                command_line("cargo deny --config deny.toml --manifest-path")
                    .arg(manifest)
                    .args(["check", "advisories"])
                    .current_dir(&root),
            );
        }
        fs::remove_dir_all(temp).unwrap();
    }
}

/// The checkout without `.git`, the toolbelt, build output, generated SBOMs
/// and Windows zone markers.
fn copy_source(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(name.as_str(), ".git" | ".tools" | "target")
            || name.ends_with(".cdx.json")
            || name.contains(":Zone.Identifier")
        {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_source(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
