//! `rust-gate mutants`: cargo-mutants over the workspace, failing on any
//! surviving or timed-out mutant. A pull request mutates only the lines it
//! changed; a push or a tag mutates its own commit, which on a squash-merged
//! default branch is one pull request's change. Only a first commit, with no
//! parent, mutates the whole workspace.

use crate::runner::{Cmd, Job, Outcome, Step, flag, non_empty, optional, output, tee_line};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "mutants",
    summary: "Mutation testing",
    inputs: &["GITHUB_BASE_REF", "MUTATION_TEST"],
    tools: &["cargo mutants", "git", "jaq"],
    reports: &["mutants.json", "mutants.txt"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let temp = &job.temp;
    let report = job.report("mutants.txt")?;
    if !flag("MUTATION_TEST")? {
        tee_line("SKIPPED: mutation-test=false", &report, false)?;
        return output("applied", "false");
    }
    let base = optional("GITHUB_BASE_REF")?;
    // The checkout fetched the commit and its first parent: the base branch
    // of a pull request's merge commit, or the previous commit of a push. The
    // diff needs no network.
    let parent = Cmd::new("git rev-parse --verify -q HEAD^1")
        .cwd(project)
        .capture()
        .is_ok();
    if !base.is_empty() && !parent {
        return Err("pull request checkout must include the base parent".into());
    }
    let mut scope: Vec<String> = Vec::new();
    let change = if parent {
        let diff = temp.join("mutants.diff");
        Cmd::new("git diff --relative HEAD^1 HEAD -- .")
            .cwd(project)
            .stdout_to(&diff)?;
        scope.push("--in-diff".to_owned());
        scope.push(diff.display().to_string());
        if base.is_empty() {
            tee_line("scope: changes in the last commit", &report, false)?;
            Some("this commit")
        } else {
            tee_line(&format!("scope: changes against {base}"), &report, false)?;
            Some("this pull request")
        }
    } else {
        tee_line("scope: full workspace, a first commit", &report, false)?;
        None
    };
    let output_dir = temp.join("mutants");
    let verdict =
        Cmd::new("cargo mutants --no-shuffle --cargo-arg=--locked --colors=never --level=info")
            .args(&scope)
            .arg("--output")
            .arg(&output_dir)
            .cwd(project)
            .tee(&report, true);
    let outcomes = output_dir.join("mutants.out/outcomes.json");
    let saved = if outcomes.is_file() {
        std::fs::copy(&outcomes, job.report("mutants.json")?)
            .map(|_| ())
            .map_err(|error| format!("cannot copy the outcomes: {error}"))
    } else {
        Ok(())
    };
    verdict?;
    saved?;
    report_outcomes(&job, &outcomes, change)
}

/// Interpret only successful tool runs, after their raw outcomes were preserved.
/// `change` names the diff that was mutated, or is `None` for the whole workspace.
fn report_outcomes(job: &Job, outcomes: &std::path::Path, change: Option<&str>) -> Outcome {
    let report = job.report("mutants.txt")?;
    if !outcomes.exists() {
        let log = std::fs::read_to_string(&report)
            .map_err(|error| format!("{}: {error}", report.display()))?;
        // ponytail: pinned 25.3.1 reports these skips only as text; use a structured
        // skip when upstream provides one. Silence is never proof of no work.
        if log.lines().any(|line| {
            line.trim() == "WARN No mutants found under the active filters"
                || (change.is_some()
                    && matches!(
                        line.trim(),
                        "INFO Diff file is empty"
                            | "INFO Diff changes no Rust source files"
                            | "INFO No mutants to filter"
                    ))
        }) {
            let message = format!(
                "SKIPPED: no mutants apply to {}",
                change.unwrap_or("this workspace")
            );
            tee_line(&message, &report, true)?;
            return output("applied", "false");
        }
    }
    non_empty(outcomes).map_err(|_| "cargo-mutants produced no outcomes")?;
    Cmd::new("jaq -er")
        .arg(
            "\"caught=\\(.caught) missed=\\(.missed) timeout=\\(.timeout) unviable=\\(.unviable)\"",
        )
        .arg(outcomes)
        .tee(&report, true)?;
    Cmd::new("jaq -e")
        .arg(".missed == 0 and .timeout == 0")
        .arg(outcomes)
        .capture()
        .map(|_| ())
        .map_err(|_| "Surviving or timed-out mutants; strengthen the tests that should fail")?;
    let applied = Cmd::new("jaq -r")
        .arg("(.caught + .unviable) > 0")
        .arg(outcomes)
        .capture()?;
    output("applied", applied.trim())
}
