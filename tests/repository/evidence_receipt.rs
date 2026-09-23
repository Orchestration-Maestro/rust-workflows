//! The evidence receipt: produced only when every upstream result succeeded.

use crate::harness::{query, root, toolbelt_path};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Every combination of upstream results the evidence step can be handed.
const RESULTS: &[&str] = &[
    "success",
    "success success success",
    "failure",
    "skipped",
    "cancelled",
    "",
    "success failure",
    "success\nfailure",
    "success\r",
    "missing-context",
];

#[test]
fn ci_evidence_rejects_failed_skipped_and_missing_provenance() {
    let root = root();
    let jobs = if root.join("action.yml").is_file() {
        vec![("ci.yml", "test"), ("ci.yml", "required")]
    } else {
        vec![
            ("ci-internal.yml", "check"),
            ("ci-internal.yml", "required"),
        ]
    };
    for (workflow, job) in jobs {
        let script = evidence_step_contract(
            &root.join(".github/workflows").join(workflow),
            workflow,
            job,
        );
        for (index, results) in RESULTS.iter().enumerate() {
            evidence_outcome(&script, &format!("{workflow}-{job}-{index}"), results);
        }
    }
}

/// The evidence step always runs, reads the upstream outcomes, and its upload
/// is pinned and refuses an empty receipt; returns the step's body.
fn evidence_step_contract(path: &Path, workflow: &str, job: &str) -> String {
    let selector = format!(".jobs.{job}.steps[] | select(.id == \"evidence\")");
    let script = query(path, &format!("{selector} | .run"));
    assert!(
        !script.is_empty(),
        "Missing evidence step: {workflow}/{job}"
    );
    assert_eq!(query(path, &format!("{selector} | .if")), "${{ always() }}");
    let outcomes = query(path, &format!("{selector} | .env.CHECK_RESULTS"));
    assert!(outcomes.contains(".outcome") || outcomes.contains(".result"));
    let upload = format!(".jobs.{job}.steps[] | select(.id == \"upload-evidence\")");
    assert_eq!(
        query(path, &format!("{upload} | .uses")),
        "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
    );
    assert_eq!(
        query(path, &format!("{upload} | .if")),
        "${{ success() && steps.evidence.outcome == 'success' }}"
    );
    assert_eq!(
        query(path, &format!("{upload} | .with[\"if-no-files-found\"]")),
        "error"
    );
    script
}

/// One run of the evidence body: a receipt exists exactly when every result
/// succeeded, and it names the revision, the run and the scope.
fn evidence_outcome(script: &str, case: &str, results: &str) {
    let temp = std::env::temp_dir().join(format!("ci-evidence-{}-{case}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let summary = temp.join("summary.md");
    let mut command = Command::new("bash");
    command
        .args(["-c", script])
        .env_clear()
        .env("PATH", toolbelt_path())
        .env(
            "CHECK_RESULTS",
            if results == "missing-context" {
                "success"
            } else {
                results
            },
        )
        .env("RUNNER_TEMP", &temp)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("GITHUB_SERVER_URL", "https://github.invalid")
        .env("GITHUB_REPOSITORY", "test/repository")
        .env("GITHUB_RUN_ID", "42")
        .env("GITHUB_RUN_ATTEMPT", "2")
        .env("EVIDENCE_SCOPE", "Local contract-test stand-in");
    if results != "missing-context" {
        command.env("GITHUB_SHA", "a".repeat(40));
    }
    let output = command.output().unwrap();
    let receipt = temp.join("ci-evidence/result.md");
    let expected = matches!(results, "success" | "success success success");
    assert_eq!(
        output.status.success(),
        expected,
        "{case}: {results:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        receipt.is_file(),
        expected,
        "Invalid success evidence: {results:?}"
    );
    if expected {
        let text = fs::read_to_string(receipt).unwrap();
        assert!(text.contains(&"a".repeat(40)));
        assert!(text.contains("/test/repository/actions/runs/42/attempts/2"));
        assert!(text.contains("Local contract-test stand-in"));
        assert_eq!(text, fs::read_to_string(summary).unwrap());
    }
    fs::remove_dir_all(temp).unwrap();
}
