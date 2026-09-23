//! Scorecard states distinguish selection, applicability and actual execution.

use crate::harness::{Fixture, succeeds, workflow};
use serde_json::Value;
use std::fs;

/// Control, changed selector, value, GitHub outcome, result and expected state.
const INACTIVE: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "mutation testing",
        "MUTATION_TEST",
        "false",
        "OUT_MUTANTS",
        "success",
        "disabled",
    ),
    (
        "unused dependencies",
        "UNUSED_DEPENDENCIES",
        "false",
        "OUT_UNUSED",
        "success",
        "disabled",
    ),
    (
        "sources, versions and licences",
        "LICENSE_POLICY",
        "off",
        "OUT_LICENCES",
        "success",
        "disabled",
    ),
    (
        "unsafe denied",
        "UNSAFE_POLICY",
        "deny",
        "OUT_QUALITY",
        "skipped",
        "not-run",
    ),
    (
        "SARIF reports",
        "SARIF_REPORTS",
        "true",
        "OUT_QUALITY",
        "failure",
        "failed",
    ),
    (
        "SARIF reports",
        "SARIF_REPORTS",
        "true",
        "OUT_SECRETS",
        "failure",
        "failed",
    ),
    (
        "SARIF reports",
        "SARIF_REPORTS",
        "true",
        "OUT_SECRETS",
        "skipped",
        "not-run",
    ),
    (
        "feature combinations",
        "FEATURES_APPLIED",
        "false",
        "OUT_FEATURES",
        "success",
        "not-applicable",
    ),
    (
        "mutation testing",
        "MUTANTS_APPLIED",
        "false",
        "OUT_MUTANTS",
        "success",
        "not-applicable",
    ),
    (
        "feature combinations",
        "FEATURES_APPLIED",
        "",
        "OUT_FEATURES",
        "success",
        "not-run",
    ),
    (
        "mutation testing",
        "MUTANTS_APPLIED",
        "",
        "OUT_MUTANTS",
        "success",
        "not-run",
    ),
    (
        "advisories",
        "MUTATION_TEST",
        "true",
        "OUT_AUDIT",
        "cancelled",
        "not-run",
    ),
    (
        "mutation testing",
        "MUTANTS_APPLIED",
        "false",
        "OUT_MUTANTS",
        "failure",
        "failed",
    ),
];

#[test]
fn the_scorecard_distinguishes_disabled_inapplicable_failed_and_unrun_controls() {
    for &(control, setting, value, outcome, result, expected) in INACTIVE {
        let mut f = Fixture::new();
        for key in [
            "OUT_QUALITY",
            "OUT_COVERAGE",
            "OUT_AUDIT",
            "OUT_SECRETS",
            "OUT_MSRV",
            "OUT_FEATURES",
            "OUT_LICENCES",
            "OUT_MUTANTS",
            "OUT_UNUSED",
            "OUT_STAGE",
        ] {
            f.set(key, "success");
        }
        for key in [
            "MUTATION_TEST",
            "UNUSED_DEPENDENCIES",
            "SARIF_REPORTS",
            "FEATURES_APPLIED",
            "MUTANTS_APPLIED",
        ] {
            f.set(key, "true");
        }
        f.set("UNSAFE_POLICY", "deny");
        f.set(setting, value);
        f.set(outcome, result);
        succeeds(&f.run("ci", "scorecard"));
        let card: Value = serde_json::from_str(
            &fs::read_to_string(f.root.join("reports/scorecard.json")).unwrap(),
        )
        .unwrap();
        let row = card["controls"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["control"] == control)
            .unwrap();
        assert_eq!(row["state"], expected, "{control}: {card}");
        assert_eq!(row["active"], false, "{control}: {card}");
        let markdown = fs::read_to_string(f.root.join("reports/scorecard.md")).unwrap();
        assert!(
            markdown
                .lines()
                .any(|line| line.contains(control) && line.contains(expected))
        );
    }
}

#[test]
fn mutation_testing_records_no_application_when_the_workspace_has_no_mutants() {
    let mut f = Fixture::new();
    f.set("MUTATION_TEST", "true");
    succeeds(&f.run_body("cd \"$PROJECT\"; cargo generate-lockfile --offline"));
    succeeds(&f.run("ci", "mutants"));
    let output = fs::read_to_string(f.root.join("output")).unwrap();
    assert!(output.ends_with("applied=false\n"), "{output}");
}

#[test]
fn the_scorecard_receives_application_results_from_the_executed_steps() {
    let ci = workflow("ci");
    let card = ci["jobs"]["checks"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["id"] == "scorecard")
        .unwrap();
    for (key, value) in [
        ("FEATURES_APPLIED", "${{ steps.features.outputs.applied }}"),
        ("MUTANTS_APPLIED", "${{ steps.mutants.outputs.applied }}"),
    ] {
        assert_eq!(card["env"][key], value, "{key}");
    }
}
