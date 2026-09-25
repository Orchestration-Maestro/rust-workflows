//! Every job installs every pinned tool it invokes: no step reads a tool
//! that a step of its job did not install from the pinned table first.

use crate::harness::{Described, described, described_step, tool_rows, workflow_steps};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Whether `step` invokes `tool`: a gate body, what its step declares; a Bash
/// body, what its lines show. A Cargo subcommand is named as Cargo runs it,
/// `cargo mutants` for the `cargo-mutants` row.
fn invokes(described: &[Described], step: &Value, tool: &str) -> bool {
    let Some(run) = step["run"].as_str() else {
        return false;
    };
    let tool = tool.strip_prefix("cargo-").map_or_else(
        || tool.to_owned(),
        |subcommand| format!("cargo {subcommand}"),
    );
    match described_step(described, run) {
        Some(found) => found.tools.contains(&tool),
        None => run.lines().any(|line| {
            let line = line.trim_start();
            !line.starts_with('#')
                && (line.starts_with(&format!("{tool} "))
                    || line.contains(&format!("$({tool} "))
                    || line.contains(&format!("| {tool} "))
                    || line.contains(&format!("={tool} ")))
        }),
    }
}

/// Whether a step of `steps` installs `tool`: the pinned table, or the
/// toolbelt bootstrap, which installs every tool at the version and checksum
/// mise.lock records.
fn installs(steps: &[Value], tool: &str) -> bool {
    steps.iter().any(|step| {
        tool_rows(step).iter().any(|row| row.name == tool)
            || step["run"].as_str().is_some_and(|run| {
                run.lines()
                    .any(|line| line.trim() == "scripts/bootstrap.sh")
            })
    })
}

#[test]
fn a_job_installs_every_pinned_tool_before_a_step_invokes_it() {
    // `attest-binaries` invoked jaq twice and installed it nowhere, so the job
    // that decides whether a release is safe to sign depended on whatever the
    // runner image happened to ship. A pin nobody installs is not a pin. And
    // ci.yml's `validate` read a ruleset run's settings through jaq one step
    // before the table that installs it: exit 127 on its first live run.
    let described = described();
    let mut jobs: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
    for (name, job, step) in workflow_steps() {
        jobs.entry((name, job)).or_default().push(step);
    }
    let pinned: BTreeSet<String> = jobs
        .values()
        .flatten()
        .flat_map(|step| tool_rows(step).into_iter().map(|row| row.name))
        .collect();
    let mut checked = 0;
    for ((name, job), steps) in jobs {
        for tool in &pinned {
            let Some(first) = steps
                .iter()
                .position(|step| invokes(&described, step, tool))
            else {
                continue;
            };
            assert!(
                installs(&steps[..first], tool),
                "{name}.yml/{job} invokes {tool} at step {first} without installing the pinned \
                 copy first"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 20,
        "only {checked} pinned tool usages were checked"
    );
}
