//! `rust-gate fuzz <step>`: a bounded fuzz regression on a nightly
//! toolchain, replaying the committed corpus before exploring for a fixed
//! budget.

use crate::checks::checkout_paths::project_directory;
use crate::checks::rust_versions::is_nightly;
use crate::checks::simple_names::simple;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, export, input, native_linux, summary};
use std::fs;
use std::path::Path;

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "fuzz",
        id: "validate",
        summary: "Validate consumer inputs",
        inputs: &[
            "DIRECTORY",
            "GITHUB_WORKSPACE",
            "MAX_TOTAL_TIME",
            "TARGET",
            "TOOLCHAIN",
        ],
        tools: &[],
        reports: &[],
        run: validate,
    },
    Step {
        workflow: "fuzz",
        id: "toolchain",
        summary: "Install nightly toolchain and cargo-fuzz",
        inputs: &["RUSTUP_TOOLCHAIN"],
        tools: &["cargo install", "rustup"],
        reports: &["toolchain.txt"],
        run: toolchain,
    },
    Step {
        workflow: "fuzz",
        id: "replay",
        summary: "Replay corpus and explore",
        inputs: &["MAX_TOTAL_TIME", "TARGET"],
        tools: &["cargo fuzz"],
        reports: &["fuzz.txt"],
        run: replay,
    },
];

/// Refuse any fuzz input the run cannot prove safe, then export the
/// resolved project, toolchain and reports directory to the rest of the job.
fn validate() -> Outcome {
    let project = project_directory()?;
    // A missing fuzz directory means there is nothing to replay, which must
    // be an error rather than a green run that fuzzed nothing.
    if !project.join("fuzz/fuzz_targets").is_dir() {
        return Err(
            "No fuzz/fuzz_targets directory; run cargo fuzz init before enabling this workflow"
                .into(),
        );
    }
    let toolchain = input("TOOLCHAIN")?;
    if !is_nightly(&toolchain) {
        return Err("toolchain must be nightly or nightly-YYYY-MM-DD".into());
    }
    let target = input("TARGET")?;
    if !(target.is_empty() || simple(&target, "_", "_-")) {
        return Err("target must be a simple fuzz target name".into());
    }
    let budget = input("MAX_TOTAL_TIME")?;
    if budget.is_empty() || !budget.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("max-total-time must be a whole number of seconds".into());
    }
    if !(10..=1800).contains(&budget.parse::<u64>().unwrap_or(u64::MAX)) {
        return Err("max-total-time must be between 10 and 1800 seconds".into());
    }
    let temp = input("RUNNER_TEMP")?;
    export(&[
        ("PROJECT", &project.display().to_string()),
        ("RUSTUP_TOOLCHAIN", &toolchain),
        ("REPORTS", &format!("{temp}/fuzz-reports")),
    ])
}

/// Install the nightly toolchain with rust-src and cargo-fuzz, and record
/// the compiler version.
fn toolchain() -> Outcome {
    let job = Job::current()?;
    native_linux()?;
    let reports = &job.reports;
    fs::create_dir_all(reports)
        .map_err(|error| format!("cannot create {}: {error}", reports.display()))?;
    let toolchain = input("RUSTUP_TOOLCHAIN")?;
    Cmd::new("rustup toolchain install")
        .arg(&toolchain)
        .args(["--profile", "minimal", "--component", "rust-src"])
        .run()?;
    Cmd::new("cargo install --locked cargo-fuzz --version 0.13.1").run()?;
    Cmd::new("rustup run")
        .arg(&toolchain)
        .args(["rustc", "--version"])
        .tee(&job.report("toolchain.txt")?, false)
}

/// Every fuzz target the project declares, one `.rs` file each under
/// `fuzz/fuzz_targets`, sorted.
fn fuzz_targets(project: &Path) -> Result<Vec<String>, Failure> {
    let entries = fs::read_dir(project.join("fuzz/fuzz_targets"))
        .map_err(|error| format!("fuzz/fuzz_targets: {error}"))?;
    let mut targets = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("fuzz/fuzz_targets: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let stem = name.strip_suffix(".rs").map(str::to_owned);
        targets.extend(stem.filter(|_| entry.file_type().is_ok_and(|kind| kind.is_file())));
    }
    targets.sort();
    Ok(targets)
}

/// Replay the committed corpus of every target, explore for the budget,
/// and summarize what ran.
fn replay() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let report = job.report("fuzz.txt")?;
    let requested = input("TARGET")?;
    let targets = if requested.is_empty() {
        fuzz_targets(project)?
    } else {
        vec![requested]
    };
    if targets.is_empty() {
        return Err("No fuzz targets found".into());
    }
    let budget = input("MAX_TOTAL_TIME")?;
    for name in &targets {
        println!("::group::{name}");
        // -runs=0 replays the committed corpus without generating new input,
        // so a previously fixed crash that regressed fails immediately and
        // deterministically, before any time is spent exploring.
        Cmd::new("cargo fuzz run")
            .arg(name)
            .args(["--", "-runs=0"])
            .cwd(project)
            .tee(&report, true)?;
        Cmd::new("cargo fuzz run")
            .arg(name)
            .arg("--")
            .arg(format!("-max_total_time={budget}"))
            .cwd(project)
            .tee(&report, true)?;
        println!("::endgroup::");
    }
    summary(&format!(
        "## Fuzz regression\n\nTargets: {}\nExploration budget: {budget}s per target\n\n\
         A clean run means no crash was reached within this budget.\n\
         It is not evidence that none exists.\n",
        targets.join(" ")
    ))
}
