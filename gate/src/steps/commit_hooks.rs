//! `rust-gate hooks`: the commit hooks of the repository's
//! `.pre-commit-config.yaml` over every file, the way a commit runs them,
//! through the pinned prek. The hooks CI runs as steps of their own, the
//! formatter, Clippy and the gate's rules, are skipped rather than run twice.

use crate::runner::{Cmd, Job, Outcome, Step, input};
use std::path::PathBuf;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "hooks",
    summary: "Commit hooks over every file",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["prek"],
    reports: &["hooks.txt"],
    run,
}];

/// The hooks CI runs as steps of their own.
const SKIPPED: &str = "rustfmt,clippy,rust-gate-architecture,rust-gate-hygiene";

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let root = PathBuf::from(input("GITHUB_WORKSPACE")?);
    let report = job.report("hooks.txt")?;
    Cmd::new("prek run --all-files --show-diff-on-failure --color never")
        .env("SKIP", SKIPPED)
        .cwd(&root)
        .tee(&report, false)
}
