//! `rust-gate unsafe-audit <step>`: the consumer's tests interpreted under
//! Miri on a nightly toolchain, and the proof that the audit reached the
//! unsafe code at all.

use crate::checks::checkout_paths::{committed_file, project_directory, rust_sources};
use crate::checks::rust_versions::is_nightly;
use crate::checks::simple_names::simple;
use crate::runner::{Cmd, Job, Outcome, Step, export, flag, input, native_linux, tee_line};

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "unsafe-audit",
        id: "validate",
        summary: "Validate consumer inputs",
        inputs: &["DIRECTORY", "GITHUB_WORKSPACE", "TEST_FILTER", "TOOLCHAIN"],
        tools: &[],
        reports: &[],
        run: validate,
    },
    Step {
        workflow: "unsafe-audit",
        id: "toolchain",
        summary: "Install nightly toolchain and Miri",
        inputs: &["RUSTUP_TOOLCHAIN"],
        tools: &["rustup"],
        reports: &["toolchain.txt"],
        run: toolchain,
    },
    Step {
        workflow: "unsafe-audit",
        id: "miri",
        summary: "Interpret tests under Miri",
        inputs: &["STRICT_PROVENANCE", "TEST_FILTER"],
        tools: &["cargo miri"],
        reports: &["miri.txt"],
        run: miri,
    },
    Step {
        workflow: "unsafe-audit",
        id: "reach",
        summary: "Confirm the audit reached the unsafe code",
        inputs: &[],
        tools: &[],
        reports: &["miri-reach.txt"],
        run: reach,
    },
];

/// Refuse any audit input the run cannot prove safe, then export the
/// resolved project, toolchain and directories to the rest of the job.
fn validate() -> Outcome {
    let project = project_directory()?;
    for name in ["Cargo.toml", "Cargo.lock"] {
        committed_file(&project, name)?;
    }
    // Only nightly is accepted here: this workflow exists precisely because
    // Miri does not run on a stable toolchain, and pointing it at one would
    // fail later with a confusing component error.
    let toolchain = input("TOOLCHAIN")?;
    if !is_nightly(&toolchain) {
        return Err("toolchain must be nightly or nightly-YYYY-MM-DD".into());
    }
    let filter = input("TEST_FILTER")?;
    if !(filter.is_empty() || simple(&filter, "_:", "_:-")) {
        return Err("test-filter must be a simple test path".into());
    }
    let temp = input("RUNNER_TEMP")?;
    export(&[
        ("PROJECT", &project.display().to_string()),
        ("RUSTUP_TOOLCHAIN", &toolchain),
        ("CARGO_TARGET_DIR", &format!("{temp}/miri-target")),
        ("REPORTS", &format!("{temp}/miri-reports")),
    ])
}

/// Install the nightly toolchain with Miri, and record both versions.
fn toolchain() -> Outcome {
    let job = Job::current()?;
    native_linux()?;
    let reports = &job.reports;
    std::fs::create_dir_all(reports)
        .map_err(|error| format!("cannot create {}: {error}", reports.display()))?;
    let toolchain = input("RUSTUP_TOOLCHAIN")?;
    // Miri is not published for every nightly, so a missing component here
    // means the selected date must move, not that the audit can be skipped.
    Cmd::new("rustup toolchain install")
        .arg(&toolchain)
        .args(["--profile", "minimal", "--component", "miri"])
        .run()
        .map_err(|_| format!("Miri is unavailable for {toolchain}; select another nightly date"))?;
    let report = job.report("toolchain.txt")?;
    Cmd::new("rustup run")
        .arg(&toolchain)
        .args(["rustc", "--version"])
        .tee(&report, false)?;
    Cmd::new("rustup run")
        .arg(&toolchain)
        .args(["cargo", "miri", "--version"])
        .tee(&report, true)
}

/// `--all-features` because `unsafe` hidden behind an optional feature is
/// exactly the code least likely to be exercised anywhere else. Undefined
/// behaviour is a hard failure: a consumer only reaches this workflow by
/// asking for it, so a finding must not be advisory.
fn miri() -> Outcome {
    let job = Job::current()?;
    let mut command = Cmd::new("cargo miri test --workspace --locked --all-features");
    if flag("STRICT_PROVENANCE")? {
        command = command.env("MIRIFLAGS", "-Zmiri-strict-provenance");
    }
    let filter = input("TEST_FILTER")?;
    if !filter.is_empty() {
        command = command.arg(&filter);
    }
    command
        .cwd(&job.project)
        .tee(&job.report("miri.txt")?, false)
}

/// Miri reports undefined behaviour only on paths a test actually runs, so
/// an audit that executed nothing is not evidence of safety. A narrow
/// test-filter can silently exclude every unsafe block; counting both sides
/// turns that from a green run into a visible failure.
fn reach() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let mut blocks = 0;
    for file in rust_sources(project)? {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        blocks += whole_word_occurrences(&source, "unsafe");
    }
    let executed = std::fs::read_to_string(job.earlier("miri.txt")).map_or(0, |log| {
        log.lines()
            .filter(|line| {
                line.strip_prefix("test ")
                    .and_then(|rest| rest.strip_suffix(" ... ok"))
                    .is_some_and(|name| !name.is_empty())
            })
            .count()
    });
    tee_line(
        &format!("unsafe occurrences: {blocks}\ntests executed under Miri: {executed}"),
        &job.report("miri-reach.txt")?,
        false,
    )?;
    if blocks > 0 && executed == 0 {
        return Err(
            "This workspace contains unsafe code but the audit executed no tests; widen test-filter"
                .into(),
        );
    }
    Ok(())
}

/// Occurrences of `word` in `text` as a whole word: `unsafely` is not
/// `unsafe`.
fn whole_word_occurrences(text: &str, word: &str) -> usize {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word)
        .filter(|(index, _)| {
            let before = text[..*index]
                .chars()
                .next_back()
                .is_none_or(|c| !is_word(c));
            let after = text[index + word.len()..]
                .chars()
                .next()
                .is_none_or(|c| !is_word(c));
            before && after
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::whole_word_occurrences;

    #[test]
    fn unsafe_is_counted_as_a_whole_word() {
        assert_eq!(
            whole_word_occurrences("pub fn f() { unsafe { } }\n", "unsafe"),
            1
        );
        assert_eq!(
            whole_word_occurrences("pub fn unsafely_named() {}\n", "unsafe"),
            0
        );
        assert_eq!(
            whole_word_occurrences("unsafe fn a() {} unsafe { b() }", "unsafe"),
            2
        );
    }
}
