//! `rust-gate hygiene prepare`: the first step of `hygiene.yml`, the reusable
//! workflow of a repository without Rust. It names the checkout as the
//! project and gives the run a reports directory, the two things `validate`
//! does for `ci.yml`, since nothing here has an input to validate.
//! `rust-gate hygiene pull-request-names` holds a pull request's title and
//! head branch to PRL-003 and PRL-004, as `ci.yml`'s pull request rules do.

use crate::checks::pull_request::name_findings;
use crate::runner::{Failure, Outcome, Step, export, input, optional, summary};
use std::fs;
use std::path::PathBuf;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "hygiene",
        id: "prepare",
        summary: "Name the checkout and the reports directory",
        inputs: &["GITHUB_WORKSPACE", "RUNNER_TEMP"],
        tools: &[],
        reports: &[],
        run,
    },
    Step {
        workflow: "hygiene",
        id: "pull-request-names",
        summary: "Pull request title and branch PRL-003 and PRL-004",
        inputs: &["GITHUB_BASE_REF", "GITHUB_HEAD_REF", "PULL_REQUEST_TITLE"],
        tools: &[],
        reports: &[],
        run: names,
    },
];

/// Run the step.
fn run() -> Outcome {
    let workspace = input("GITHUB_WORKSPACE")?;
    let reports = PathBuf::from(input("RUNNER_TEMP")?).join("hygiene-reports");
    fs::create_dir_all(&reports).map_err(|error| format!("{}: {error}", reports.display()))?;
    export(&[
        ("PROJECT", &workspace),
        ("REPORTS", &reports.display().to_string()),
    ])
}

/// Run `pull-request-names`: nothing on a push, which has no pull request.
fn names() -> Outcome {
    if optional("GITHUB_BASE_REF")?.is_empty() {
        return Ok(());
    }
    let findings = name_findings(
        &optional("PULL_REQUEST_TITLE")?,
        &optional("GITHUB_HEAD_REF")?,
    );
    if findings.is_empty() {
        return summary("## Pull request names\n\nPRL-003 and PRL-004: no finding.\n");
    }
    summary(&format!(
        "## Pull request names\n\n{}\n",
        findings.join("\n")
    ))?;
    Err(Failure::from(format!(
        "pull-request-names: {}",
        findings.join("; ")
    )))
}
