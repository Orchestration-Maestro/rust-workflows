//! `rust-gate build`: release tests, the auditable release build whose bytes
//! ship, verified packages of the members that may be published and the
//! per-member `CycloneDX` documents, with any lockfile change during SBOM
//! generation refused.

use crate::checks::manifests::{cargo_packages, read_cargo_metadata};
use crate::checks::rust_versions::parse;
use crate::runner::{Cmd, Job, Outcome, Step, input, path};
use std::fs;
use std::path::Path;

/// The oldest Cargo whose `package` takes a member's dependency on another
/// member it packages from the workspace, where older ones look it up on
/// crates.io.
const WORKSPACE_PACKAGER: &str = "1.90.0";

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "build",
    summary: "Release build, verified packages and SBOMs",
    inputs: &["CARGO_TARGET_DIR", "RUSTUP_TOOLCHAIN"],
    tools: &[
        "cargo auditable",
        "cargo cyclonedx",
        "cargo metadata",
        "cargo package",
        "cargo test",
        "jaq",
        "rustup",
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
    package_publishable(&job)?;
    let lockfile = project.join("Cargo.lock");
    let checked = fs::read(&lockfile).map_err(|error| format!("Cargo.lock: {error}"))?;
    fs::write(temp.join("checked.lock"), &checked)
        .map_err(|error| format!("cannot copy Cargo.lock: {error}"))?;
    // cargo-cyclonedx has no --locked; reject any resolution change explicitly.
    Cmd::new(
        "cargo cyclonedx --all --format json --spec-version 1.5 --target x86_64-unknown-linux-gnu",
    )
    .cwd(project)
    .run()?;
    if fs::read(&lockfile).map_err(|error| format!("Cargo.lock: {error}"))? != checked {
        return Err(
            "Cargo.lock changed while the SBOM was generated; commit a resolved lockfile".into(),
        );
    }
    Ok(())
}

/// Package and verify every member that may be published, whose `publish` is
/// not `false`, once every archive an earlier run left is removed. Cargo
/// takes a member's dependency on another member from the members it packages
/// only when that one may be published, and otherwise looks it up in the
/// registry it packages for, so a member with a normal or build dependency on
/// a `publish = false` one cannot package. Leaving those members out lets a
/// private workspace build; a publishable member with such a dependency still
/// fails, as it could not be published either. A workspace with no member to
/// publish packages nothing, and says so.
fn package_publishable(job: &Job) -> Outcome {
    remove_earlier_archives(&path("CARGO_TARGET_DIR")?)?;
    let metadata = read_cargo_metadata(&job.project, &job.temp)?;
    let members: Vec<String> = cargo_packages(&metadata)?
        .into_iter()
        .filter(|member| member.publishable)
        .map(|member| member.name)
        .collect();
    if members.is_empty() {
        println!("SKIPPED: no workspace member may be published, so none is packaged");
        return Ok(());
    }
    let mut package = Cmd::new("cargo package --locked").cwd(&job.project);
    for member in &members {
        package = package.arg("--package").arg(member);
    }
    // The binaries keep the selected compiler; only packaging moves to a
    // Cargo that can package members depending on each other.
    let selected = parse(&input("RUSTUP_TOOLCHAIN")?, false);
    if selected.is_some_and(|version| Some(version) < parse(WORKSPACE_PACKAGER, false)) {
        Cmd::new("rustup toolchain install")
            .arg(WORKSPACE_PACKAGER)
            .args(["--profile", "minimal"])
            .run()?;
        package = package.env("RUSTUP_TOOLCHAIN", WORKSPACE_PACKAGER);
    }
    package.run()
}

/// Remove every archive in the package directory under `target`, the ones
/// staging copies. One an earlier run left, restored from CI's cache or kept
/// by a local run, would otherwise ship with the payload although this run
/// did not package it: a `publish = false` member's, or an older version's.
fn remove_earlier_archives(target: &Path) -> Outcome {
    let Ok(entries) = fs::read_dir(target.join("package")) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read the packages: {error}"))?;
        let archive = entry.path();
        if archive
            .extension()
            .is_some_and(|extension| extension == "crate")
        {
            fs::remove_file(&archive)
                .map_err(|error| format!("cannot remove {}: {error}", archive.display()))?;
        }
    }
    Ok(())
}
