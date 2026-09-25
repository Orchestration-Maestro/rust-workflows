//! `rust-gate architecture --local` and `rust-gate hygiene --local`: a
//! `ci.yml` step run outside Actions, the way a commit hook runs it. The
//! current directory is the repository, its workspace and its project; the
//! reports go to a scratch directory the run removes, and the step prints its
//! findings as it does in CI. `rust-gate clippy --local` runs Clippy there
//! with the organization's thresholds, which a hook cannot hand it otherwise.

use crate::checks::organization_config::clippy_directory;
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
    Step {
        workflow: "local",
        id: "clippy --local",
        summary: "Clippy over the repository here with the organization's thresholds",
        inputs: &[],
        tools: &["cargo clippy"],
        reports: &[],
        run: clippy,
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

/// Run Clippy here, every warning denied, with the organization's
/// `clippy.toml` in a scratch directory unless the repository commits its own.
fn clippy() -> Outcome {
    let root = fs::canonicalize(".").map_err(|error| format!("clippy --local: {error}"))?;
    let scratch = env::temp_dir().join(format!("rust-gate-clippy-{}", process::id()));
    let outcome = clippy_directory(&root, &root, &scratch).and_then(|directory| {
        let mut clippy =
            Cmd::new("cargo clippy --workspace --all-targets --locked -- -D warnings").cwd(&root);
        if let Some(directory) = directory {
            clippy = clippy.env("CLIPPY_CONF_DIR", &directory);
        }
        clippy.run()
    });
    fs::remove_dir_all(&scratch).ok();
    outcome
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
