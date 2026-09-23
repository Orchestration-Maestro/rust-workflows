//! `rust-gate vet`: every dependency must have a recorded cargo-vet audit.
//! cargo-vet needs a committed ledger; without one it would offer to create
//! it, so the ledger is required explicitly rather than auditing nothing.

use crate::runner::{Cmd, Job, Outcome, Step, flag, tee_line};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "vet",
    summary: "Recorded dependency audits",
    inputs: &["DEPENDENCY_AUDIT"],
    tools: &["cargo vet"],
    reports: &["dependency-audit.txt"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("dependency-audit.txt")?;
    if !flag("DEPENDENCY_AUDIT")? {
        return tee_line("SKIPPED: dependency-audit=false", &report, false);
    }
    let project = &job.project;
    if !project.join("supply-chain/config.toml").is_file() {
        return Err(
            "dependency-audit=true requires a committed supply-chain/config.toml; \
             run cargo vet init"
                .into(),
        );
    }
    Cmd::new("cargo vet --locked")
        .cwd(project)
        .tee(&report, false)
}
