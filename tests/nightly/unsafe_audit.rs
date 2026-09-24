//! `unsafe-audit.yml`: Miri on the selected nightly, kept outside the stable
//! policy, and never a pass without a test executed under it.

use crate::harness::{Fixture, refused, root, succeeds, workflow};
use std::fmt::Write as _;
use std::fs;

#[test]
fn the_undefined_behaviour_audit_refuses_to_pass_without_running_anything() {
    // Miri only reports undefined behaviour on paths a test executes. A run that
    // executed nothing is not evidence of safety, and a narrow test-filter can
    // quietly exclude every unsafe block while still exiting zero.
    let write_source = |fixture: &Fixture, body: &str| {
        fs::create_dir_all(fixture.root.join("project/src")).unwrap();
        fs::write(fixture.root.join("project/src/lib.rs"), body).unwrap();
    };
    let write_miri = |fixture: &Fixture, passing: usize| {
        let mut lines = String::new();
        for i in 0..passing {
            let _ = writeln!(lines, "test case_{i} ... ok");
        }
        fs::write(fixture.root.join("reports/miri.txt"), lines).unwrap();
    };

    // unsafe present, nothing executed: the combination that proves nothing.
    let blind = Fixture::new();
    write_source(&blind, "pub fn f() { unsafe { } }\n");
    write_miri(&blind, 0);
    refused(
        &blind.run("unsafe-audit", "reach"),
        "This workspace contains unsafe code but the audit executed no tests; widen test-filter",
    );

    // unsafe present and tests ran: acceptable, and both counts reported.
    let covered = Fixture::new();
    write_source(&covered, "pub fn f() { unsafe { } }\n");
    write_miri(&covered, 3);
    succeeds(&covered.run("unsafe-audit", "reach"));
    let report = fs::read_to_string(covered.root.join("reports/miri-reach.txt")).unwrap();
    assert!(report.contains("unsafe occurrences: 1"));
    assert!(report.contains("tests executed under Miri: 3"));

    // No unsafe at all: nothing to reach, so an empty run is honest.
    let safe = Fixture::new();
    write_source(&safe, "pub fn f() -> u8 { 1 }\n");
    write_miri(&safe, 0);
    succeeds(&safe.run("unsafe-audit", "reach"));
    assert!(
        fs::read_to_string(safe.root.join("reports/miri-reach.txt"))
            .unwrap()
            .contains("unsafe occurrences: 0")
    );

    // `unsafely` is not `unsafe`: a word-boundary mistake here would fail every
    // safe workspace that happens to use the prefix.
    let lookalike = Fixture::new();
    write_source(&lookalike, "pub fn unsafely_named() {}\n");
    write_miri(&lookalike, 0);
    succeeds(&lookalike.run("unsafe-audit", "reach"));
}

#[test]
fn unsafe_audit_stays_outside_the_supported_stable_policy() {
    // Miri only exists on nightly. Keeping it in its own callable workflow is
    // what lets the five-pin stable policy stay absolute in ci.yml: a consumer
    // opts into nightly explicitly here instead of the stable policy being widened.
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    assert!(
        !ci.contains("nightly"),
        "the stable version policy must never admit nightly"
    );
    let data = workflow("unsafe-audit");
    let inputs = &data["on"]["workflow_call"]["inputs"];
    assert_eq!(inputs["toolchain"]["default"], "nightly-2026-09-14");
    assert_eq!(inputs["strict-provenance"]["default"], false);
    let job = &data["jobs"]["miri"];
    assert_eq!(job["runs-on"], "ubuntu-24.04");
    assert_eq!(job["permissions"]["contents"], "read");
    assert!(job["timeout-minutes"].as_i64().unwrap() >= 30);
    assert!(
        job["if"].as_str().unwrap().contains("pull_request_target"),
        "an entry workflow must reject untrusted pull-request contexts"
    );
    // A finding must fail the run: a consumer only gets here by asking for it.
    let text = fs::read_to_string(root().join(".github/workflows/unsafe-audit.yml")).unwrap();
    assert!(
        !text.contains("continue-on-error"),
        "undefined behaviour must not be reported as advisory"
    );
}

#[test]
fn the_unsafe_audit_toolchain_needs_miri_for_the_selected_nightly() {
    let mut fixture = Fixture::new();
    fixture.set("RUSTUP_TOOLCHAIN", "nightly-2026-09-14");
    fixture.stub("rustup", "");
    succeeds(&fixture.run_body("rust-gate unsafe-audit toolchain"));
    let trace = fixture.trace();
    for command in [
        "rustup toolchain install nightly-2026-09-14 --profile minimal --component miri",
        "rustup run nightly-2026-09-14 cargo miri --version",
    ] {
        assert!(trace.contains(command), "{command} not in {trace}");
    }
    fixture.stub("rustup", "exit 1");
    let refused = fixture.run_body("rust-gate unsafe-audit toolchain");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("Miri is unavailable for nightly-2026-09-14; select another nightly date"),
        "{stderr}"
    );
}

#[test]
fn miri_runs_the_workspace_tests_under_the_filter_and_strict_provenance() {
    let mut fixture = Fixture::new();
    fixture.set("STRICT_PROVENANCE", "true");
    fixture.set("TEST_FILTER", "parser::");
    fixture.stub("cargo", "");
    succeeds(&fixture.run("unsafe-audit", "miri"));
    let trace = fixture.trace();
    assert!(
        trace.contains(
            "MIRIFLAGS=-Zmiri-strict-provenance cargo miri test --workspace --locked \
             --all-features parser::"
        ),
        "{trace}"
    );
    assert!(fixture.root.join("reports/miri.txt").is_file());
}
