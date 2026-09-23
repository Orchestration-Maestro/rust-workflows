//! `rust-gate coverage`: line coverage in LCOV, failing below the threshold.

use crate::checks::inputs::coverage_threshold;
use crate::runner::{Cmd, Job, Outcome, Step, non_empty};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "coverage",
    summary: "Line coverage gate",
    inputs: &["COVERAGE"],
    tools: &["cargo llvm-cov"],
    reports: &["coverage.lcov"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let lcov = job.report("coverage.lcov")?;
    Cmd::new("cargo llvm-cov --workspace --locked --lcov --output-path")
        .arg(&lcov)
        .arg("--fail-under-lines")
        .arg(coverage_threshold()?)
        .cwd(&job.project)
        .run()?;
    non_empty(&lcov)
}
