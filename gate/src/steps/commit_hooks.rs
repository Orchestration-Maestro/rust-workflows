//! `rust-gate hooks`: the commit hooks of the repository's
//! `.pre-commit-config.yaml` over every file, the way a commit runs them,
//! through the pinned prek. The hooks CI runs as steps of their own, the
//! formatter, Clippy and the gate's rules, are skipped rather than run twice.
//! The home of the workflows runs its own hooks in `just check`, on its
//! pinned toolbelt.

use crate::checks::workflow_home::is_workflow_home;
use crate::runner::{Cmd, Job, Outcome, Step, input, output, tee_line};
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

/// The hooks CI runs as steps of their own, and the two that rewrite the rule
/// map and the Copilot guide: CI never fails on a stale one, which the daily
/// drift check reports instead.
const SKIPPED: &str =
    "rustfmt,clippy,rust-gate-architecture,rust-gate-hygiene,rust-gate-rules,rust-gate-guide";

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let root = PathBuf::from(input("GITHUB_WORKSPACE")?);
    let report = job.report("hooks.txt")?;
    if is_workflow_home(&root) {
        tee_line(
            "NOT APPLICABLE: the home of the workflows runs its hooks in just check",
            &report,
            false,
        )?;
        return output("applied", "false");
    }
    Cmd::new("prek run --all-files --show-diff-on-failure --color never")
        .env("SKIP", SKIPPED)
        .cwd(&root)
        .tee(&report, false)?;
    output("applied", "true")
}
