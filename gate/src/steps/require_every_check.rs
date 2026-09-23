//! `rust-gate required`: the one status a branch protection can require.
//! It fails on a failed, cancelled or skipped `checks` job, so reports can
//! never convert a failure into success.

use crate::runner::{Outcome, Step, input, summary};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "required",
    summary: "Require every check",
    inputs: &["RESULT"],
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
    Ok(())
}
