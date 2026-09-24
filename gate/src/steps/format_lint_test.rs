//! `rust-gate quality`: formatting, Clippy with every warning denied, the
//! tests, the doc tests and strict rustdoc, after every workspace manifest
//! and source path is confirmed to lie inside the checkout.
//!
//! The tests run under nextest, which gives each one its own process. Two
//! tests that share a global, a current directory or a temporary path pass
//! under one process and fail under another, and the version that catches them
//! is the one worth having. It also writes `JUnit`, which `cargo test` does not,
//! so report consumers can read results rather than scrape logs. Doc tests stay
//! on `cargo test --doc`: nextest does not run them, and dropping them silently
//! would trade a runner for a gap.

use crate::checks::checkout_paths::canonical;
use crate::checks::inputs::{UnsafePolicy, clippy_level, unsafe_policy};
use crate::checks::nextest_profile::nextest_profile;
use crate::runner::{Cmd, Job, Outcome, Step, flag, non_empty, path};
use std::fs;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "quality",
    summary: "Formatting, Clippy and tests",
    inputs: &[
        "CLIPPY_LEVEL",
        "GITHUB_WORKSPACE",
        "SARIF_REPORTS",
        "UNSAFE_POLICY",
    ],
    tools: &[
        "cargo clippy",
        "cargo doc",
        "cargo fmt",
        "cargo metadata",
        "cargo nextest",
        "cargo test",
        "clippy-sarif",
        "jaq",
    ],
    reports: &["clippy.json", "clippy.sarif", "tests.xml"],
    run,
}];

/// The manifest and every source path of every workspace member.
const MEMBER_PATHS: &str =
    ".workspace_members as $members | .packages[] | select(.id as $id | $members | index($id)) |
  .manifest_path, .targets[].src_path";

/// The tests, one process each, with their results as `JUnit` beside the other
/// reports. nextest reads its profile from a file rather than a flag, so the
/// gate writes the one it wants into the runner's temporary directory instead
/// of asking the consumer to commit `.config/nextest.toml`. The profile names
/// the report by absolute path, which nextest accepts: the alternative is a
/// path relative to a store directory under whichever target directory Cargo
/// happened to use, found by a second reading that can disagree with the first.
/// The path is a TOML literal string, so a temporary directory holding a quote
/// fails on the parse rather than writing the report somewhere else.
fn run_the_tests(job: &Job) -> Outcome {
    let report = job.report("tests.xml")?;
    let profile = job.temp.join("nextest.toml");
    let body = nextest_profile(
        "gate",
        &format!("junit = {{ path = '{}' }}\n", report.display()),
    );
    fs::write(&profile, body)
        .map_err(|error| format!("cannot write {}: {error}", profile.display()))?;
    Cmd::new("cargo nextest run --workspace --locked --profile gate --config-file")
        .arg(&profile)
        .cwd(&job.project)
        .run()?;
    non_empty(&report).map_err(|_| "cargo-nextest produced no JUnit report".into())
}

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let temp = &job.temp;
    let metadata = temp.join("metadata.json");
    Cmd::new("cargo metadata --format-version 1 --locked")
        .cwd(project)
        .stdout_to(&metadata)?;
    let root = canonical(&path("GITHUB_WORKSPACE")?)?;
    let members = Cmd::new("jaq -er")
        .arg(MEMBER_PATHS)
        .arg(&metadata)
        .capture()?;
    for line in members.lines() {
        let inside = canonical(Path::new(line))
            .is_ok_and(|resolved| resolved.starts_with(&root) && resolved != root);
        if !inside {
            return Err("Workspace manifests and sources must remain inside checkout".into());
        }
    }
    Cmd::new("cargo fmt --all --check").cwd(project).run()?;
    // Lints passed after `--` reach the selected workspace members only, so
    // denying unsafe here never fails on a dependency that legitimately uses
    // it. RUSTFLAGS would apply to the whole graph and break almost any real
    // project. A member that genuinely needs unsafe keeps an explicit,
    // greppable `#[allow(unsafe_code)]` instead of silently compiling. A
    // leftover todo!() or dbg!() is scaffolding, never release code.
    let mut lints = vec![
        "-D",
        "warnings",
        "-D",
        "clippy::todo",
        "-D",
        "clippy::dbg_macro",
    ];
    if unsafe_policy()? == UnsafePolicy::Deny {
        lints.extend(["-D", "unsafe_code"]);
    }
    // Clippy is the static analyser for Rust; raising the level widens the
    // rule set rather than adding a separate tool. nursery implies pedantic
    // because the unstable group is only meaningful on top of it.
    for group in clippy_level()?.denied() {
        lints.extend(["-D", group]);
    }
    let clippy_json = job.report("clippy.json")?;
    let verdict =
        Cmd::new("cargo clippy --workspace --all-targets --locked --message-format=json --")
            .args(&lints)
            .cwd(project)
            .stdout_to(&clippy_json);
    // Emit diagnostics even when Clippy fails; a converter must not replace
    // the compiler's original failure with its own exit status.
    let reports = clippy_reports(&job, &clippy_json);
    verdict?;
    reports?;
    run_the_tests(&job)?;
    Cmd::new("cargo test --workspace --doc --locked")
        .cwd(project)
        .run()?;
    // An undocumented public item is a maintenance defect, not a style
    // preference: the next reader has no statement of intent to check the
    // implementation against. Broken intra-doc links fail here too.
    Cmd::new("cargo doc --workspace --no-deps --locked")
        .env("RUSTDOCFLAGS", "-D warnings -D missing_docs")
        .cwd(project)
        .run()?;
    Ok(())
}

/// Render the captured diagnostics and optionally convert the same invocation
/// to SARIF before its verdict is propagated.
fn clippy_reports(job: &Job, clippy_json: &Path) -> Outcome {
    Cmd::new("jaq -r")
        .arg("select(.reason == \"compiler-message\") | .message.rendered // empty")
        .arg(clippy_json)
        .run()?;
    if flag("SARIF_REPORTS")? {
        let sarif = job.report("clippy.sarif")?;
        Cmd::new("clippy-sarif --input")
            .arg(clippy_json)
            .arg("--output")
            .arg(&sarif)
            .run()?;
        non_empty(&sarif)?;
    }
    Ok(())
}
