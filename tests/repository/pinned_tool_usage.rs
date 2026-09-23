//! Every job installs every pinned tool it invokes: no step reads a tool
//! that a step of its job did not install from the pinned table first.

use crate::harness::{described, described_step, root, tool_rows, workflow};
use std::fs;

#[test]
fn a_job_installs_every_pinned_tool_it_invokes() {
    // `attest-binaries` invoked jaq twice and installed it nowhere, so the job
    // that decides whether a release is safe to sign depended on whatever the
    // runner image happened to ship. A pin nobody installs is not a pin.
    let described = described();
    let mut checked = 0;
    for entry in fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let data = workflow(name.trim_end_matches(".yml"));
        let Some(jobs) = data["jobs"].as_object() else {
            continue;
        };
        for (job, body) in jobs {
            let Some(steps) = body["steps"].as_array() else {
                continue;
            };
            // A gate body invokes what its step declares; a Bash body, what
            // its lines show.
            let invoked_by = |tool: &str| {
                steps.iter().any(|step| {
                    let Some(run) = step["run"].as_str() else {
                        return false;
                    };
                    match described_step(&described, run) {
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
            };
            for tool in ["jaq"] {
                let invoked = invoked_by(tool);
                if !invoked {
                    continue;
                }
                // The pinned table, or the toolbelt bootstrap, which installs
                // every tool at the version and checksum mise.lock records.
                let installed = steps.iter().any(|step| {
                    tool_rows(step).iter().any(|row| row.name == tool)
                        || step["run"].as_str().is_some_and(|run| {
                            run.lines()
                                .any(|line| line.trim() == "scripts/bootstrap.sh")
                        })
                });
                assert!(
                    installed,
                    "{name}/{job} invokes {tool} without installing the pinned copy"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 3, "no jaq usage was found to check");
}
