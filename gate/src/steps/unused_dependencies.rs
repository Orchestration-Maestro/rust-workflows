//! `rust-gate unused`: a declared dependency no source file uses fails the
//! run, through cargo-machete.

use crate::runner::{Cmd, Job, Outcome, Step, flag, tee_line};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "unused",
    summary: "Unused declared dependencies",
    inputs: &["UNUSED_DEPENDENCIES"],
    tools: &["cargo machete"],
    reports: &["unused-dependencies.txt"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("unused-dependencies.txt")?;
    if !flag("UNUSED_DEPENDENCIES")? {
        return tee_line("SKIPPED: unused-dependencies=false", &report, false);
    }
    Cmd::new("cargo machete")
        .cwd(&job.project)
        .tee(&report, false)
}
