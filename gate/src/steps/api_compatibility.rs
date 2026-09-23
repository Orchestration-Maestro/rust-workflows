//! `rust-gate api`: a pull request that breaks a library's public API fails
//! unless its title declares the break. cargo-semver-checks compares the
//! workspace's libraries with the base branch as a minor release, so an
//! addition passes and a removal or an incompatible signature fails. A title
//! with `!` after its type, which release-please turns into a major release,
//! declares the break and skips the comparison.

use crate::checks::rust_versions::parse;
use crate::runner::{Cmd, Job, Outcome, Step, flag, input, optional, output, tee_line};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "api",
    summary: "Public API compatibility",
    inputs: &[
        "API_COMPATIBILITY",
        "GITHUB_BASE_REF",
        "PULL_REQUEST_TITLE",
        "RUSTUP_TOOLCHAIN",
    ],
    tools: &["cargo metadata", "cargo semver-checks", "git", "jaq"],
    reports: &["api-compatibility.txt"],
    run,
}];

/// Whether any workspace member has a target with a public API to compare.
const LIBRARY: &str =
    r#"[.packages[].targets[].kind[]] | any(. == "lib" or . == "rlib" or . == "proc-macro")"#;

/// The oldest compiler cargo-semver-checks 0.50 runs on.
const OLDEST_RUST: (u64, u64, u64) = (1, 93, 0);

/// Run the step. Every case that compares nothing says why and records that
/// the check did not apply, so the scorecard never counts it as a pass.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("api-compatibility.txt")?;
    let skip = |reason: &str| -> Outcome {
        tee_line(reason, &report, false)?;
        output("applied", "false")
    };
    if !flag("API_COMPATIBILITY")? {
        return skip("SKIPPED: api-compatibility=false");
    }
    let base = optional("GITHUB_BASE_REF")?;
    if base.is_empty() {
        return skip("NOT APPLICABLE: only a pull request is compared with its base");
    }
    if parse(&input("RUSTUP_TOOLCHAIN")?, false).is_none_or(|version| version < OLDEST_RUST) {
        return skip("NOT APPLICABLE: cargo-semver-checks needs Rust 1.93 or newer");
    }
    if declares_break(&optional("PULL_REQUEST_TITLE")?) {
        return skip("NOT APPLICABLE: the title declares a breaking change");
    }
    // The checkout fetched the merge commit and its first parent, the base
    // branch; cargo-semver-checks builds that parent as the baseline.
    let parent = Cmd::new("git rev-parse --verify -q HEAD^1")
        .cwd(&job.project)
        .capture()
        .is_ok();
    if !parent {
        return Err("pull request checkout must include the base parent".into());
    }
    let kinds = Cmd::new("cargo metadata --format-version 1 --no-deps --locked")
        .cwd(&job.project)
        .capture()?;
    let library = Cmd::new("jaq -r")
        .arg(LIBRARY)
        .stdin_bytes(kinds.as_bytes())
        .capture()?;
    if library.trim() != "true" {
        return skip("NOT APPLICABLE: no library target, so no public API");
    }
    tee_line(&format!("baseline: {base}"), &report, false)?;
    Cmd::new("cargo semver-checks --workspace --baseline-rev HEAD^1 --release-type minor")
        .cwd(&job.project)
        .tee(&report, true)?;
    output("applied", "true")
}

/// Whether a conventional title marks a breaking change: `!` right after its
/// type and optional scope, as in `feat!:` or `fix(api)!:`.
fn declares_break(title: &str) -> bool {
    title.split_once(':').is_some_and(|(header, _)| {
        header.strip_suffix('!').is_some_and(|kind| {
            let name = kind.split_once('(').map_or(kind, |(name, _)| name);
            !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::declares_break;

    #[test]
    fn only_a_bang_after_the_type_declares_a_break() {
        assert!(declares_break("feat!: drop the old API"));
        assert!(declares_break("fix(api)!: rename the error"));
        assert!(!declares_break("feat: add a product"));
        assert!(!declares_break("docs: mention feat!: in the guide"));
        assert!(!declares_break("Revert \"feat!: drop the old API\""));
        assert!(!declares_break("!: nothing"));
    }
}
