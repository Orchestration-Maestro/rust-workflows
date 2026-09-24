//! `rust-gate architecture --local` and `rust-gate hygiene --local`: a
//! `ci.yml` step run outside Actions, the way a commit hook runs it. The
//! current directory is the repository, its workspace and its project; the
//! reports go to a scratch directory the run removes, and the step prints its
//! findings as it does in CI.

use crate::runner::{Cmd, Outcome, Step};
use std::env;
use std::fs;
use std::process;

/// What these commands declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "local",
        id: "architecture --local",
        summary: "The source rules over the repository here, as a commit hook runs them",
        inputs: &[],
        tools: &["rust-gate"],
        reports: &[],
        run: architecture,
    },
    Step {
        workflow: "local",
        id: "hygiene --local",
        summary: "Repository hygiene over the repository here, as a commit hook runs it",
        inputs: &[],
        tools: &["rust-gate"],
        reports: &[],
        run: hygiene,
    },
];

/// Run `architecture` here.
fn architecture() -> Outcome {
    locally("architecture")
}

/// Run `hygiene` here.
fn hygiene() -> Outcome {
    locally("hygiene")
}

/// Run `step` against the repository here, its reports in a scratch
/// directory.
fn locally(step: &str) -> Outcome {
    let root = fs::canonicalize(".").map_err(|error| format!("{step} --local: {error}"))?;
    let scratch = env::temp_dir().join(format!("rust-gate-{step}-{}", process::id()));
    fs::create_dir_all(&scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let outcome = Cmd::new("rust-gate")
        .arg(step)
        .env("GITHUB_WORKSPACE", &root)
        .env("PROJECT", &root)
        .env("REPORTS", &scratch)
        .env("RUNNER_TEMP", &scratch)
        .env("GITHUB_STEP_SUMMARY", &scratch.join("summary.md"))
        .cwd(&root)
        .run();
    fs::remove_dir_all(&scratch).ok();
    outcome
}
