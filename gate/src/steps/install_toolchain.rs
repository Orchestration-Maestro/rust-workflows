//! `rust-gate tools`: the consumer's exact toolchain with the components the
//! gates need, and the reports directory every later step writes into.

use crate::runner::{Cmd, Outcome, Step, input, path};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "tools",
    summary: "Install the toolchain",
    inputs: &["RUSTUP_TOOLCHAIN"],
    tools: &["rustup"],
    reports: &[],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    Cmd::new("rustup toolchain install")
        .arg(input("RUSTUP_TOOLCHAIN")?)
        .args([
            "--profile",
            "minimal",
            "--component",
            "rustfmt,clippy,llvm-tools-preview",
        ])
        .run()?;
    let reports = path("REPORTS")?;
    std::fs::create_dir_all(&reports)
        .map_err(|error| format!("cannot create {}: {error}", reports.display()))?;
    Ok(())
}
