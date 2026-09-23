//! `rust-gate required`: the one status a branch protection can require.
//! It fails on a failed, cancelled or skipped `checks` job, so reports can
//! never convert a failure into success, and on a portability job that did
//! not pass when the caller named platforms.

use crate::runner::{Outcome, Step, input, optional, summary};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "required",
    summary: "Require every check",
    inputs: &["PORTABILITY", "RESULT", "RUNNERS"],
    tools: &[],
    reports: &[],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let result = input("RESULT")?;
    summary(&format!("Rust checks: {result}\n"))?;
    if result != "success" {
        return Err("Required Rust checks failed or were skipped".into());
    }
    // No platform named means the portability job was skipped on purpose.
    if optional("RUNNERS")?.is_empty() {
        return Ok(());
    }
    let portability = optional("PORTABILITY")?;
    summary(&format!("Portability: {portability}\n"))?;
    if portability != "success" {
        return Err("Portability checks failed or were skipped".into());
    }
    Ok(())
}
