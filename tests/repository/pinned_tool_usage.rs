//! Every job installs every pinned tool it invokes: no step reads a tool
//! that a step of its job did not install from the pinned table first.

use crate::harness::{Described, described, described_step, tool_rows, workflow_steps};
use serde_json::Value;
use std::collections::BTreeMap;

/// Whether a step of `steps` invokes `tool`: a gate body, what its step
/// declares; a Bash body, what its lines show.
fn invokes(described: &[Described], steps: &[Value], tool: &str) -> bool {
    steps.iter().any(|step| {
        let Some(run) = step["run"].as_str() else {
            return false;
        };
        match described_step(described, run) {
            Some(found) => found.tools.iter().any(|name| name == tool),
            None => run.lines().any(|line| {
                let line = line.trim_start();
                !line.starts_with('#')
                    && (line.starts_with(&format!("{tool} "))
                        || line.contains(&format!("$({tool} "))
                        || line.contains(&format!("| {tool} "))
                        || line.contains(&format!("={tool} ")))
            }),
        }
    })
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
fn a_job_installs_every_pinned_tool_it_invokes() {
    // `attest-binaries` invoked jaq twice and installed it nowhere, so the job
    // that decides whether a release is safe to sign depended on whatever the
    // runner image happened to ship. A pin nobody installs is not a pin.
    let described = described();
    let mut checked = 0;
    let mut jobs: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
    for (name, job, step) in workflow_steps() {
        jobs.entry((name, job)).or_default().push(step);
    }
    for ((name, job), steps) in jobs {
        for tool in ["jaq"]
            .into_iter()
            .filter(|tool| invokes(&described, &steps, tool))
        {
            assert!(
                installs(&steps, tool),
                "{name}.yml/{job} invokes {tool} without installing the pinned copy"
            );
            checked += 1;
        }
    }
    assert!(checked > 3, "no jaq usage was found to check");
}
