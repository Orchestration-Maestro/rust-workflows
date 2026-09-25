//! `rust-gate hooks`: the commit hooks of the repository's
//! `.pre-commit-config.yaml` over every file, the way a commit runs them, on
//! the toolbelt `rust-gate setup` installs on a developer's machine. The
//! hooks CI runs as steps of their own, the formatter and the gate's rules,
//! and the pre-push hook that runs CI's steps, are skipped rather than run
//! twice. The home of the workflows runs
//! its own hooks in `just check`, on its pinned toolbelt.

use crate::checks::toolbelt::{install_toolbelt, toolbelt_path};
use crate::checks::workflow_home::is_workflow_home;
use crate::runner::{Cmd, Job, Outcome, Step, input, output, tee_line};
use std::path::PathBuf;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "hooks",
    summary: "Commit hooks over every file",
    inputs: &[
        "GITHUB_WORKSPACE",
        "HOME",
        "LOCALAPPDATA",
        "PATH",
        "SystemRoot",
        "XDG_CACHE_HOME",
    ],
    // On Windows, mise's download directory is made private through
    // PowerShell, as every private directory there is.
    tools: if cfg!(windows) {
        &[
            "cargo install",
            "curl",
            "mise",
            "powershell.exe",
            "prek",
            "tar",
        ]
    } else {
        &["cargo install", "curl", "mise", "prek", "tar"]
    },
    reports: &["hooks.txt"],
    run,
}];

/// The hooks CI runs as steps of their own, the one that runs CI's steps
/// before a push, and the two that rewrite the rule map and the Copilot guide:
/// CI never fails on a stale one, which the daily drift check reports instead.
const SKIPPED: &str = concat!(
    "rustfmt,rust-gate-ci,rust-gate-architecture,rust-gate-hygiene,",
    "rust-gate-rules,rust-gate-guide"
);

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
    let toolbelt = install_toolbelt()?;
    Cmd::new("prek run --all-files --show-diff-on-failure --color never")
        .env("PATH", &toolbelt_path(&toolbelt.bin)?)
        .env("SKIP", SKIPPED)
        .cwd(&root)
        .tee(&report, false)?;
    output("applied", "true")
}
