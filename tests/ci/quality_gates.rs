//! `ci.yml`: the lint, documentation, coverage and analysis gates, each proven
//! to fail when its tool does.

use crate::harness::{Fixture, refused, succeeds, workflow};
use serde_json::{Value, json};
use std::fs;

#[test]
fn clippy_denies_leftover_scaffolding_at_every_level() {
    // todo!() and dbg!() are scaffolding. Clippy allows both by default, so the
    // gate denies them itself, at every Clippy level a caller can select.
    for level in ["default", "pedantic", "nursery"] {
        let mut f = Fixture::new();
        f.set("GITHUB_WORKSPACE", &f.root.display().to_string());
        f.set("CLIPPY_LEVEL", level);
        f.set("METADATA", &workspace_metadata(&f));
        f.stub("cargo", CARGO);
        succeeds(&f.run("ci", "quality"));
        let calls = f.calls();
        let clippy = calls
            .lines()
            .find(|line| line.starts_with("clippy --workspace") && line.contains(" -- "))
            .unwrap_or_else(|| panic!("no clippy gate at level {level}: {calls}"));
        assert!(
            clippy.contains("-D warnings -D clippy::todo -D clippy::dbg_macro"),
            "level {level}: {clippy}"
        );
        if level != "default" {
            assert!(
                clippy.contains("-D clippy::pedantic"),
                "level {level}: {clippy}"
            );
        }
    }
    // The unsafe ban is one more lint on the same command line, so it reaches
    // the workspace members only: a dependency that uses unsafe stays green.
    for (policy, banned) in [("allow", false), ("deny", true)] {
        let mut f = Fixture::new();
        f.set("GITHUB_WORKSPACE", &f.root.display().to_string());
        f.set("UNSAFE_POLICY", policy);
        f.set("METADATA", &workspace_metadata(&f));
        f.stub("cargo", CARGO);
        succeeds(&f.run("ci", "quality"));
        let calls = f.calls();
        let clippy = calls
            .lines()
            .find(|line| line.starts_with("clippy --workspace") && line.contains(" -- "))
            .unwrap_or_else(|| panic!("no clippy gate with unsafe-policy={policy}: {calls}"));
        assert_eq!(
            clippy.contains("-D unsafe_code"),
            banned,
            "unsafe-policy={policy}: {clippy}"
        );
        assert!(
            !calls.contains("RUSTFLAGS"),
            "the ban must not reach dependencies"
        );
    }
}

/// The metadata a stubbed `cargo metadata` answers: one member, its manifest
/// and one source file, all inside the fixture's checkout.
fn workspace_metadata(f: &Fixture) -> String {
    json!({"workspace_members": ["fixture"], "packages": [{
        "id": "fixture", "manifest_path": f.root.join("project/Cargo.toml"),
        "targets": [{"src_path": f.root.join("project/src/lib.rs")}]}]})
    .to_string()
}

/// The `cargo` a quality test needs: it answers `cargo metadata` with the
/// fixture's own workspace, and under `nextest` leaves the `JUnit` document a
/// real run would have left, which the step copies beside the other reports.
const CARGO: &str = r#"[[ "$1" == metadata ]] && printf '%s' "$METADATA"
[[ "$1" == clippy ]] && printf '{"reason":"build-finished","success":true}\n'
[[ "$1" == nextest ]] && printf '<testsuites tests="1"/>\n' > "$REPORTS/tests.xml"
exit 0"#;

#[test]
fn sarif_reports_are_written_only_when_asked_and_never_empty() {
    // SARIF is an opt-in second pass over the Clippy output. Off, the converter
    // never runs; on, its report must exist and carry bytes, because an empty
    // SARIF uploads as a clean scan.
    let mut f = Fixture::new();
    f.set("GITHUB_WORKSPACE", &f.root.display().to_string());
    f.set("METADATA", &workspace_metadata(&f));
    f.stub("cargo", CARGO);
    f.stub(
        "clippy-sarif",
        r#"out=""
while [[ $# -gt 0 ]]; do
  [[ "$1" == --output ]] && { out=$2; shift 2; continue; }
  shift
done
printf '{}' > "$out""#,
    );
    succeeds(&f.run("ci", "quality"));
    assert!(
        !f.calls().contains("clippy-sarif"),
        "the SARIF converter ran without being asked"
    );
    assert!(!f.root.join("reports/clippy.sarif").exists());
    f.set("SARIF_REPORTS", "true");
    succeeds(&f.run("ci", "quality"));
    assert!(f.root.join("reports/clippy.sarif").is_file());
    fs::remove_file(f.root.join("reports/clippy.sarif")).unwrap();
    f.stub("clippy-sarif", "exit 0");
    assert!(
        !f.run("ci", "quality").status.success(),
        "an empty SARIF report must fail the step"
    );
}

#[test]
fn sarif_reports_are_on_by_default_and_upload_in_their_own_workflow() {
    // Every run writes SARIF. Showing it in code scanning needs
    // security-events: write, which only the separate upload workflow asks
    // for, so an ordinary CI caller keeps a read-only token. A fork pull
    // request's token cannot write security events, so the upload skips it.
    let ci = workflow("ci");
    assert_eq!(
        ci["on"]["workflow_call"]["inputs"]["sarif-reports"]["default"],
        true
    );
    let upload = workflow("upload-sarif");
    let input = &upload["on"]["workflow_call"]["inputs"]["artifact-name"];
    assert_eq!(input["required"], true);
    let job = &upload["jobs"]["upload"];
    assert_eq!(
        job["permissions"],
        json!({"contents": "read", "security-events": "write"})
    );
    assert_eq!(
        job["if"],
        "${{ github.event_name != 'pull_request_target' && \
         (!github.event.pull_request || \
         github.event.pull_request.head.repo.full_name == github.repository) }}"
    );
    let steps = job["steps"].as_array().unwrap();
    assert!(
        steps[0]["uses"]
            .as_str()
            .unwrap()
            .starts_with("actions/download-artifact@")
    );
    assert_eq!(
        steps[0]["with"]["name"],
        "${{ inputs.artifact-name }}-reports"
    );
    let uploads: Vec<(&str, &str, &str)> = steps[1..]
        .iter()
        .map(|step| {
            (
                step["uses"].as_str().unwrap().split('@').next().unwrap(),
                step["with"]["sarif_file"].as_str().unwrap(),
                step["with"]["category"].as_str().unwrap(),
            )
        })
        .collect();
    let action = "github/codeql-action/upload-sarif";
    assert_eq!(
        uploads,
        [
            (action, "reports/clippy.sarif", "clippy"),
            (action, "reports/secrets.sarif", "gitleaks"),
        ]
    );
}

#[test]
fn strict_rustdoc_fails_the_run_when_cargo_doc_does() {
    // Strict rustdoc is the last cargo command of the quality step: an
    // undocumented public item or a broken intra-doc link fails cargo doc,
    // and that failure is the step's, with the lint flags cargo doc was given.
    let mut f = Fixture::new();
    f.set("GITHUB_WORKSPACE", &f.root.display().to_string());
    f.set("METADATA", &workspace_metadata(&f));
    f.stub(
        "cargo",
        r#"[[ "$1" == metadata ]] && printf '%s' "$METADATA"
[[ "$1" == nextest ]] && printf '<testsuites tests="1"/>\n' > "$REPORTS/tests.xml"
[[ "$1" == clippy ]] && printf '{"reason":"build-finished","success":true}\n'
[[ "$1" == doc ]] && { printf 'RUSTDOCFLAGS=%s\n' "$RUSTDOCFLAGS" >> "$CALLS"; exit 7; }
exit 0"#,
    );
    assert_eq!(f.run("ci", "quality").status.code(), Some(7));
    let calls = f.calls();
    assert!(
        calls.contains("RUSTDOCFLAGS=-D warnings -D missing_docs"),
        "{calls}"
    );
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("clippy "))
            .count(),
        1
    );
    assert!(
        fs::read_to_string(f.root.join("reports/clippy.json"))
            .unwrap()
            .contains("build-finished")
    );
}

#[test]
fn line_coverage_below_the_threshold_fails_the_run() {
    // The threshold is handed to cargo-llvm-cov as --fail-under-lines, so a
    // shortfall is the tool's own non-zero exit and the step's. A run that
    // leaves no LCOV report behind fails as well: the report is what the
    // scorecard and a reviewer read.
    let mut f = Fixture::new();
    f.set("COVERAGE", "90");
    f.stub("cargo", "exit 3");
    assert_eq!(f.run("ci", "coverage").status.code(), Some(3));
    assert!(f.calls().contains("--fail-under-lines 90"));
    f.stub("cargo", "exit 0");
    assert!(
        !f.run("ci", "coverage").status.success(),
        "a missing coverage report must fail the step"
    );
    f.stub(
        "cargo",
        r#"out=""
while [[ $# -gt 0 ]]; do
  [[ "$1" == --output-path ]] && { out=$2; shift 2; continue; }
  shift
done
printf 'TN:\nend_of_record\n' > "$out""#,
    );
    succeeds(&f.run("ci", "coverage"));
    assert!(f.root.join("reports/coverage.lcov").is_file());
}

#[test]
fn every_declared_feature_is_compiled_and_a_broken_one_fails() {
    // A default build proves one combination. The gate builds each declared
    // feature on its own, so a feature nobody selects cannot quietly stop
    // compiling; a workspace declaring none spends no compile saying so.
    let metadata = |features: Value| {
        json!({"workspace_members": ["p"],
               "packages": [{"id": "p", "name": "fixture", "features": features}]})
        .to_string()
    };
    let f = Fixture::new();
    f.stub("cargo", "exit 0");

    // No feature: nothing to build, and the run says which it is.
    fs::write(f.root.join("metadata.json"), metadata(json!({}))).unwrap();
    let outcome = f.run("ci", "features");
    succeeds(&outcome);
    assert!(
        fs::read_to_string(f.root.join("output"))
            .unwrap()
            .ends_with("applied=false\n")
    );
    assert!(
        String::from_utf8_lossy(&outcome.stdout).contains("SKIPPED: the workspace declares no"),
        "a workspace without features must say so"
    );
    assert!(
        !f.calls().contains("hack"),
        "nothing to check must cost no compile: {}",
        f.calls()
    );

    // Declared features: each one is built, and the report names them.
    fs::write(
        f.root.join("metadata.json"),
        metadata(json!({"default": ["tls"], "tls": [], "vendored": []})),
    )
    .unwrap();
    succeeds(&f.run("ci", "features"));
    assert!(
        fs::read_to_string(f.root.join("output"))
            .unwrap()
            .ends_with("applied=true\n")
    );
    assert!(
        f.calls()
            .contains("hack check --workspace --locked --each-feature"),
        "each feature must be built on its own: {}",
        f.calls()
    );
    let report = fs::read_to_string(f.root.join("reports/features.txt")).unwrap();
    assert_eq!(report, "default\ntls\nvendored", "the report names them");

    // The build is the gate: a combination that does not compile fails here.
    f.stub("cargo", "exit 101");
    assert_eq!(f.run("ci", "features").status.code(), Some(101));
}

#[test]
fn unused_dependencies_and_recorded_audits_fail_the_run_when_their_tool_does() {
    // Both gates are one tool each. Off, they say SKIPPED in their report and
    // run nothing; on, the tool's own verdict is the step's, and cargo-vet
    // without a committed ledger fails rather than auditing nothing.
    let mut f = Fixture::new();
    f.stub("cargo", "exit 3");
    succeeds(&f.run("ci", "unused"));
    succeeds(&f.run("ci", "vet"));
    assert!(f.calls().is_empty(), "a gate that is off must run nothing");
    for report in ["unused-dependencies.txt", "dependency-audit.txt"] {
        assert!(
            fs::read_to_string(f.root.join("reports").join(report))
                .unwrap()
                .contains("SKIPPED"),
            "{report} must say the gate was off"
        );
    }
    f.set("UNUSED_DEPENDENCIES", "true");
    assert_eq!(f.run("ci", "unused").status.code(), Some(3));
    assert!(f.calls().contains("machete"));
    f.set("DEPENDENCY_AUDIT", "true");
    refused(
        &f.run("ci", "vet"),
        "dependency-audit=true requires a committed supply-chain/config.toml; run cargo vet init",
    );
    fs::create_dir_all(f.root.join("project/supply-chain")).unwrap();
    fs::write(f.root.join("project/supply-chain/config.toml"), "").unwrap();
    assert_eq!(f.run("ci", "vet").status.code(), Some(3));
    assert!(f.calls().contains("vet --locked"));
    f.stub("cargo", "echo audited");
    succeeds(&f.run("ci", "unused"));
    succeeds(&f.run("ci", "vet"));
    assert!(
        fs::read_to_string(f.root.join("reports/dependency-audit.txt"))
            .unwrap()
            .contains("audited")
    );
}

#[test]
fn the_quality_gate_runs_doctests_and_strict_rustdoc() {
    // Doctests are the examples a reader copies, and rustdoc under
    // `-D missing_docs` is what makes every public item explained. Neither
    // runs by luck: both are commands of the quality step, read from the
    // trace of a run against a cargo that answers metadata and nothing else.
    let mut f = Fixture::new();
    f.set("METADATA", &workspace_metadata(&f));
    f.stub(
        "cargo",
        r#"[[ "$1" == metadata ]] && printf '%s' "$METADATA"
[[ "$1" == nextest ]] && printf '<testsuites tests="1"/>\n' > "$REPORTS/tests.xml"
[[ "$*" == *message-format=json* ]] && echo '{}'
exit 0"#,
    );
    succeeds(&f.run("ci", "quality"));
    let trace = f.trace();
    assert!(
        trace.contains("cargo test --workspace --doc --locked"),
        "doctests must run explicitly: {trace}"
    );
    assert!(
        trace.contains(
            "RUSTDOCFLAGS='-D warnings -D missing_docs' cargo doc --workspace --no-deps --locked"
        ),
        "rustdoc must run strictly: {trace}"
    );
}
