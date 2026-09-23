//! The North Star: every control it promises runs, every gate names its proof,
//! and no lint is silenced.

use crate::harness::{described, root, test_sources};
use std::fs;

/// Every quality bar and evidence source promised by the North Star must be
/// held by a `ci.yml` step that declares it runs the tool, and proven by a
/// test that fails when the bar slips. A documented control that nothing
/// runs is the failure mode this test exists to prevent; never relax a row
/// to make a missing gate pass, wire the gate instead.
const PROMISED_CONTROLS: &[(&str, &[&str], &[&str])] = &[
    (
        "Gitleaks",
        &["gitleaks"],
        &["scanners_propagate_findings_execution_errors_and_missing_tools"],
    ),
    (
        "RustSec",
        &["cargo audit"],
        &["scanners_propagate_findings_execution_errors_and_missing_tools"],
    ),
    (
        "Clippy",
        &["cargo clippy"],
        &["clippy_denies_leftover_scaffolding_at_every_level"],
    ),
    (
        "rustdoc",
        &["cargo doc"],
        &["strict_rustdoc_fails_the_run_when_cargo_doc_does"],
    ),
    (
        "doctests",
        &["cargo test"],
        &["the_quality_gate_runs_doctests_and_strict_rustdoc"],
    ),
    (
        "coverage",
        &["cargo llvm-cov"],
        &["line_coverage_below_the_threshold_fails_the_run"],
    ),
    (
        "licen",
        &["cargo deny"],
        &["the_dependency_policy_holds_by_default_and_licences_only_with_a_list"],
    ),
    (
        "SBOM",
        &["cargo cyclonedx"],
        &["sbom_staging_rejects_malformed_data_and_emits_verifiable_payload"],
    ),
    (
        "mutation",
        &["cargo mutants"],
        &["mutation_testing_scopes_a_pull_request_to_its_diff"],
    ),
];

#[test]
fn north_star_promises_are_enforced_by_the_local_gate() {
    let root = root();
    let north_star = fs::read_to_string(root.join("docs/standards/northstar.md")).unwrap();
    let justfile = fs::read_to_string(root.join("justfile")).unwrap();
    // The example gate replays ci.yml's own step bodies, selected by id, so a
    // promised command may live in the workflow rather than in the justfile.
    // Only the steps the gate actually replays count, read from the same list
    // the gate runs.
    assert!(
        justfile.contains("-- --ignored") && justfile.contains("example_gate"),
        "just check no longer invokes the example gate"
    );
    // What the hosted gate runs is what its steps declare; what proves each
    // bar is a test that exists.
    let hosted: std::collections::BTreeSet<String> = described()
        .into_iter()
        .filter(|step| step.workflow == "ci")
        .flat_map(|step| step.tools)
        .collect();
    let mut defined = String::new();
    for path in test_sources() {
        defined.push_str(&fs::read_to_string(path).unwrap());
    }

    // Only the four-measure table is a promise; surrounding prose deliberately
    // describes what is *not* claimed and must not create a requirement.
    let table = north_star
        .split_once("Four axes")
        .expect("North Star must keep its four-axis table")
        .1
        .split_once("\n## ")
        .map_or_else(|| north_star.clone(), |(rows, _)| rows.to_owned());
    let promises: String = table
        .lines()
        .filter(|line| line.starts_with("| ") && !line.starts_with("| ---"))
        .filter(|line| !line.starts_with("| Area "))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    assert!(
        ["quality", "speed", "security", "maintainability"]
            .iter()
            .all(|area| promises.contains(area)),
        "North Star must keep all four measured areas"
    );

    let mut enforced = 0;
    for (promise, tools, proofs) in PROMISED_CONTROLS {
        if !promises.contains(&promise.to_lowercase()) {
            continue;
        }
        enforced += 1;
        for tool in *tools {
            assert!(
                hosted.contains(*tool),
                "North Star promises {promise:?} but no ci.yml step declares it runs {tool:?}"
            );
        }
        for proof in *proofs {
            assert!(
                defined.contains(&format!("fn {proof}()")),
                "North Star promises {promise:?} but its proof {proof} is not a test"
            );
        }
    }
    assert!(
        enforced >= 4,
        "North Star lost its enforceable controls; only {enforced} matched"
    );
}

#[test]
fn every_gate_names_the_test_that_proves_it() {
    // A gate that cannot fail is not a gate. The README lists every gate with
    // the test that makes it fail, so the proof is one name away from the
    // promise, and a renamed or deleted test breaks the promise visibly.
    let root = root();
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let section = readme
        .split_once("## 🔒 Gates")
        .expect("README must keep its Gates section")
        .1
        .split_once("\n## ")
        .map_or_else(String::new, |(rows, _)| rows.to_owned());
    let mut defined = String::new();
    for path in test_sources() {
        defined.push_str(&fs::read_to_string(path).unwrap());
    }
    let mut proof_column = None;
    let mut gates = 0;
    for line in section.lines().filter(|line| line.starts_with("| ")) {
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        if cells.first() == Some(&"Gate") {
            proof_column = cells.iter().position(|cell| *cell == "Proof");
            assert!(
                proof_column.is_some(),
                "a gate table without a Proof column: {line}"
            );
            continue;
        }
        if cells.iter().all(|cell| cell.starts_with("---")) {
            continue;
        }
        let column = proof_column.expect("a gate row before its table header");
        let proof = cells.get(column).copied().unwrap_or_default();
        let names: Vec<&str> = proof.split('`').skip(1).step_by(2).collect();
        assert!(!names.is_empty(), "gate {:?} names no proof test", cells[0]);
        for name in names {
            assert!(
                defined.contains(&format!("fn {name}(")),
                "gate {:?} cites `{name}`, which no test defines",
                cells[0]
            );
        }
        gates += 1;
    }
    assert!(
        gates >= 20,
        "only {gates} gate rows found; the scan drifted"
    );
}

#[test]
fn no_lint_is_silenced_in_the_gate_or_the_fixtures() {
    // A silenced lint is a gate somebody turned off for one line and nobody
    // will turn back on. The gate crate and the fixtures the gate is proven
    // against carry none; a comment may mention the attribute, code may not.
    let root = root();
    let mut pending = vec![root.join("gate/src"), root.join("examples")];
    let mut scanned = 0;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                scanned += 1;
                for (number, line) in fs::read_to_string(&path).unwrap().lines().enumerate() {
                    let code = line.trim_start();
                    if code.starts_with("//") {
                        continue;
                    }
                    assert!(
                        !["#[allow(", "#![allow(", "#[expect(", "#![expect("]
                            .iter()
                            .any(|attribute| code.contains(attribute)),
                        "{}:{} silences a lint",
                        path.display(),
                        number + 1
                    );
                }
            }
        }
    }
    assert!(
        scanned > 20,
        "only {scanned} Rust files scanned; the walk drifted"
    );
}
