//! What a step declares about itself, and the step this process runs: the
//! gate reads no input, runs no tool and writes no report the step did not
//! declare, so a declaration is an inventory a reader can trust and the
//! document generated from it cannot drift from the code.

use super::outcome::{Failure, Outcome};
use std::cell::Cell;

/// One step of one workflow, as data: what it reads, runs and writes, and
/// the function that does it.
pub(crate) struct Step {
    /// The workflow the step belongs to: `ci`, another workflow's own name,
    /// or `shared` for a command several workflows run.
    pub(crate) workflow: &'static str,
    /// The step id: the word after `rust-gate`, or after the workflow.
    pub(crate) id: &'static str,
    /// What the step does, in one line: its `name:` in the workflow.
    pub(crate) summary: &'static str,
    /// The variables the step reads: the inputs the workflow passes through
    /// `env:` and what earlier steps exported. The runner's own files and
    /// the job's directories are read by every step and not listed.
    pub(crate) inputs: &'static [&'static str],
    /// The programs the step runs; `cargo` with its subcommand.
    pub(crate) tools: &'static [&'static str],
    /// The reports the step writes; a leading `*` stands for any prefix.
    pub(crate) reports: &'static [&'static str],
    /// The step itself.
    pub(crate) run: fn() -> Outcome,
}

/// What a step uses: a variable, a program or a report.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// A variable read from the environment.
    Input,
    /// A program run.
    Tool,
    /// A report written under the reports directory.
    Report,
}

/// What every step reads without declaring it: the runner's own files, the
/// job's directories `validate` exported, and the trace file the contract
/// tests ask for.
const RUNNER_OWN: &[&str] = &[
    "GITHUB_ENV",
    "GITHUB_OUTPUT",
    "GITHUB_PATH",
    "GITHUB_STEP_SUMMARY",
    "PROJECT",
    "REPORTS",
    "RUNNER_TEMP",
    "RUST_GATE_TRACE",
];

thread_local! {
    /// The step this process runs, once entered.
    static CURRENT: Cell<Option<&'static Step>> = const { Cell::new(None) };
}

/// Enter `step`: from here on the runner refuses what it did not declare.
pub(crate) fn enter(step: &'static Step) -> Outcome {
    CURRENT.with(|current| {
        if current.get().is_some() {
            return Err(Failure::from("one process runs one step"));
        }
        current.set(Some(step));
        Ok(())
    })
}

/// Refuse `name` unless the step this process runs declared it under `kind`.
/// A process that entered no step, a unit test, is not checked.
pub(crate) fn declared(kind: Kind, name: &str) -> Result<(), String> {
    match CURRENT.with(Cell::get) {
        Some(step) => check(step, kind, name),
        None => Ok(()),
    }
}

/// Whether `step` declared `name` under `kind`, with the refusal when not.
fn check(step: &Step, kind: Kind, name: &str) -> Result<(), String> {
    let (set, verb) = match kind {
        Kind::Input => (step.inputs, "reads"),
        Kind::Tool => (step.tools, "runs"),
        Kind::Report => (step.reports, "writes"),
    };
    let implicit = matches!(kind, Kind::Input) && RUNNER_OWN.contains(&name);
    if implicit || set.iter().any(|declared| covers(declared, name)) {
        Ok(())
    } else {
        Err(format!(
            "step {} {verb} {name}, which it does not declare",
            step.id
        ))
    }
}

/// Whether a declared name covers `name`: equal, or a `*` prefix matched.
fn covers(declared: &str, name: &str) -> bool {
    declared
        .strip_prefix('*')
        .map_or(declared == name, |suffix| name.ends_with(suffix))
}

/// The tool a command line runs, as the toolbelt names it: the program, and
/// the subcommand when the program is `cargo`; a `+toolchain` word is not one.
pub(crate) fn tool_key(words: &str) -> String {
    let mut parts = words
        .split_whitespace()
        .filter(|word| !word.starts_with('+'));
    let program = parts.next().unwrap_or_default();
    match (program, parts.next()) {
        ("cargo", Some(subcommand)) => format!("cargo {subcommand}"),
        _ => program.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, Step, check, tool_key};

    /// A step the checks never run.
    fn never_run() -> super::Outcome {
        Err(super::Failure::from("the sample step never runs"))
    }

    static STEP: Step = Step {
        workflow: "ci",
        id: "sample",
        summary: "A sample",
        inputs: &["COVERAGE"],
        tools: &["cargo llvm-cov", "jaq"],
        reports: &["coverage.lcov", "*.spdx.json"],
        run: never_run,
    };

    #[test]
    fn a_step_uses_only_what_it_declared() {
        assert!(check(&STEP, Kind::Input, "COVERAGE").is_ok());
        assert!(check(&STEP, Kind::Tool, "jaq").is_ok());
        assert!(check(&STEP, Kind::Report, "member.spdx.json").is_ok());
        assert_eq!(
            check(&STEP, Kind::Input, "TOKEN").unwrap_err(),
            "step sample reads TOKEN, which it does not declare"
        );
        assert_eq!(
            check(&STEP, Kind::Tool, "cargo audit").unwrap_err(),
            "step sample runs cargo audit, which it does not declare"
        );
        assert_eq!(
            check(&STEP, Kind::Report, "audit.json").unwrap_err(),
            "step sample writes audit.json, which it does not declare"
        );
    }

    #[test]
    fn the_runner_s_own_files_and_the_job_directories_need_no_declaration() {
        for name in ["GITHUB_ENV", "GITHUB_OUTPUT", "REPORTS", "RUNNER_TEMP"] {
            assert!(check(&STEP, Kind::Input, name).is_ok(), "{name}");
        }
        assert!(check(&STEP, Kind::Report, "GITHUB_ENV").is_err());
    }

    #[test]
    fn a_tool_is_the_program_and_cargo_s_subcommand() {
        assert_eq!(tool_key("cargo llvm-cov --workspace"), "cargo llvm-cov");
        assert_eq!(tool_key("cargo +nightly miri test"), "cargo miri");
        assert_eq!(tool_key("rustup run"), "rustup");
        assert_eq!(tool_key("jaq"), "jaq");
        assert_eq!(tool_key(""), "");
    }

    #[test]
    fn one_process_runs_one_step() {
        assert!(super::enter(&STEP).is_ok());
        let again = super::enter(&STEP).unwrap_err();
        assert_eq!(again.message.as_deref(), Some("one process runs one step"));
    }
}
