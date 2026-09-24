//! `rust-gate hygiene prepare`: the first step of `hygiene.yml`, the reusable
//! workflow of a repository without Rust. It names the checkout as the
//! project and gives the run a reports directory, the two things `validate`
//! does for `ci.yml`, since nothing here has an input to validate.

use crate::runner::{Outcome, Step, export, input};
use std::fs;
use std::path::PathBuf;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "hygiene",
    id: "prepare",
    summary: "Name the checkout and the reports directory",
    inputs: &["GITHUB_WORKSPACE", "RUNNER_TEMP"],
    tools: &[],
    reports: &[],
    run,
}];

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
