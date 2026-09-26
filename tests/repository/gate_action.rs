//! The gate action: every job builds it from the workflow's own commit.

use crate::harness::{Fixture, action, root, succeeds, workflow_steps};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[test]
fn every_job_builds_the_gate_from_the_workflows_own_commit() {
    // A reusable workflow runs in the consumer's checkout, and a ruleset's in
    // the target repository's; either way `job.workflow_sha` names the commit
    // of this repository the workflow runs from. Every job that runs a step
    // body checks that commit out, builds the gate from it, and only then
    // checks out the consumer, so a gate change ships without a pin to move.
    let mut jobs: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
    for (name, id, step) in workflow_steps() {
        jobs.entry((name, id)).or_default().push(step);
    }
    let mut sites = 0;
    for ((name, id), steps) in jobs {
        let runs_gate = steps.iter().any(|step| {
            step["run"]
                .as_str()
                .is_some_and(|body| body.trim_start().starts_with("rust-gate "))
        });
        let build = steps
            .iter()
            .position(|step| step["uses"] == "./.github/actions/gate");
        if !runs_gate {
            assert_eq!(build, None, "{name}/{id} builds a gate it never runs");
            continue;
        }
        assert_eq!(
            build,
            Some(1),
            "{name}/{id}: the gate's checkout, then its build"
        );
        let checkout = &steps[0];
        assert!(
            checkout["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/checkout@")),
            "{name}/{id}"
        );
        assert_eq!(
            checkout["with"],
            json!({
                "repository": "${{ job.workflow_repository }}",
                "ref": "${{ job.workflow_sha }}",
                "persist-credentials": false
            }),
            "{name}/{id}"
        );
        sites += 1;
    }
    assert!(sites >= 10, "only {sites} jobs build the gate");
    assert_eq!(shipped_actions(), BTreeSet::from(["gate".to_owned()]));
    for (name, id, step) in workflow_steps() {
        assert!(
            !step["uses"]
                .as_str()
                .is_some_and(|uses| uses.contains("rust-workflows/.github/actions/")),
            "{name}/{id} pins the gate instead of building it from its own commit"
        );
    }
}

#[test]
fn the_gate_is_rebuilt_without_reusing_consumer_modified_executables() {
    let data = action("gate");
    let steps = data["runs"]["steps"].as_array().unwrap();
    let body = steps
        .iter()
        .find_map(|step| {
            step["run"]
                .as_str()
                .filter(|body| body.contains("cargo build"))
        })
        .unwrap();
    let mut fixture = Fixture::new();
    fixture.set("GATE", root().join("gate").to_str().unwrap());
    fixture.set("CACHE_HIT", "true");
    let stale = fixture.root.join("rust-gate/bin/rust-gate");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "consumer-modified executable").unwrap();
    fixture.stub(
        "cargo",
        r#"[[ "$*" == 'build --release --locked --offline --target-dir '* ]]
target="${@: -1}"
[[ ! -e "$target" ]]
mkdir -p "$target/release"
printf 'rebuilt\n' > "$target/release/rust-gate""#,
    );
    let mut directories = BTreeSet::new();
    for _ in 0..2 {
        succeeds(&fixture.run_body(body));
        let paths = fs::read_to_string(fixture.root.join("path")).unwrap();
        let directory = paths.lines().last().unwrap();
        let executable = Path::new(directory).join("rust-gate");
        assert_eq!(fs::read_to_string(&executable).unwrap(), "rebuilt\n");
        assert!(
            directories.insert(directory.to_owned()),
            "a fresh build directory is required"
        );
        fs::write(executable, "consumer-modified executable").unwrap();
    }
    assert_eq!(fixture.calls().matches("cargo\n").count(), 2);
    assert!(steps.iter().all(|step| {
        !step["uses"]
            .as_str()
            .is_some_and(|uses| uses.starts_with("actions/cache"))
    }));
}

/// Every action under `.github/actions/`, each checked against the policy a
/// workflow step is held to.
fn shipped_actions() -> BTreeSet<String> {
    let mut shipped = BTreeSet::new();
    for entry in fs::read_dir(root().join(".github/actions")).unwrap() {
        let name = entry.unwrap().file_name().to_str().unwrap().to_owned();
        action_follows_the_step_policy(&name);
        shipped.insert(name);
    }
    shipped
}

/// A composite action whose inputs are described and whose steps are either
/// a pinned action or Bash under
/// the same policy as a workflow step.
fn action_follows_the_step_policy(name: &str) {
    let data = action(name);
    assert_eq!(data["runs"]["using"], "composite", "{name}");
    for (input, spec) in data["inputs"].as_object().into_iter().flatten() {
        assert!(
            spec["description"]
                .as_str()
                .is_some_and(|description| !description.is_empty()),
            "{name}: input {input} has no description"
        );
    }
    for step in data["runs"]["steps"].as_array().unwrap() {
        if let Some(reference) = step["uses"].as_str() {
            let pin = reference.rsplit_once('@').unwrap().1;
            assert!(
                pin.len() == 40 && pin.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "{name}: {reference} is not pinned to a commit"
            );
            continue;
        }
        assert_eq!(step["shell"], "bash", "{name}");
        let run = step["run"].as_str().unwrap();
        assert!(
            run.starts_with("set -euo pipefail\n")
                && !run.contains("${{")
                && !run.contains("|| true"),
            "{name}: the same shell policy as a workflow step"
        );
    }
}

#[test]
fn an_unknown_command_is_refused_by_name() {
    let fixture = Fixture::new();
    let output = fixture.run_body("rust-gate nonsense");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown gate command: nonsense"),
        "{stderr}"
    );
}
