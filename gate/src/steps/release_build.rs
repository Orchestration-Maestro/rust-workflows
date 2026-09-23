//! `rust-gate build`: release tests, the auditable release build whose bytes
//! ship, verified packages and the per-member `CycloneDX` documents, with any
//! lockfile change during SBOM generation refused.

use crate::runner::{Cmd, Job, Outcome, Step};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "build",
    summary: "Release build, verified packages and SBOMs",
    inputs: &[],
    tools: &[
        "cargo auditable",
        "cargo cyclonedx",
        "cargo package",
        "cargo test",
    ],
    reports: &[],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let temp = &job.temp;
    // `cargo test --release` rebuilds every bin target its integration tests
    // use, without the auditable wrapper, and would overwrite the binaries
    // built below. It therefore runs first; the build whose bytes ship is the
    // last thing to touch them before they are compared and staged.
    Cmd::new("cargo test --workspace --release --locked")
        .cwd(project)
        .run()?;
    // `cargo auditable` wraps the normal build and embeds the resolved
    // dependency list in a `.dep-v0` ELF section, so the shipped binary can be
    // audited after it leaves here even by someone who never sees this
    // repository's SBOM. Both release builds use it, or their digests would
    // differ and the reproducibility check would fail for a reason that has
    // nothing to do with reproducibility.
    Cmd::new("cargo auditable build --workspace --release --locked --message-format json")
        .cwd(project)
        .stdout_to(&temp.join("build.jsonl"))?;
    Cmd::new("cargo package --workspace --locked")
        .cwd(project)
        .run()?;
    let lockfile = project.join("Cargo.lock");
    let checked = std::fs::read(&lockfile).map_err(|error| format!("Cargo.lock: {error}"))?;
    std::fs::write(temp.join("checked.lock"), &checked)
        .map_err(|error| format!("cannot copy Cargo.lock: {error}"))?;
    // cargo-cyclonedx has no --locked; reject any resolution change explicitly.
    Cmd::new(
        "cargo cyclonedx --all --format json --spec-version 1.5 --target x86_64-unknown-linux-gnu",
    )
    .cwd(project)
    .run()?;
    if std::fs::read(&lockfile).map_err(|error| format!("Cargo.lock: {error}"))? != checked {
        return Err(
            "Cargo.lock changed while the SBOM was generated; commit a resolved lockfile".into(),
        );
    }
    Ok(())
}
