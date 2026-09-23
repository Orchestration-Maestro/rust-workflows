//! The gate action: one pin at every call site, and a commit that ships it.

use crate::harness::{Fixture, action, helper_action, root, succeeds, workflow};
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

#[test]
fn helper_actions_are_pinned_to_one_commit_that_contains_them() {
    // A reusable workflow runs in the consumer's checkout, so the only way
    // every workflow gets the same gate binary is a composite action fetched
    // by commit SHA, and that commit must ship every action it is called for.
    let (pins, called, sites) = call_sites();
    assert!(sites >= 10, "only {sites} gate action call sites");
    assert_eq!(
        pins.len(),
        1,
        "every call site must pin the same commit: {pins:?}"
    );
    let pin = pins.into_iter().next().unwrap();
    let shipped = shipped_actions();
    assert_eq!(
        called, shipped,
        "every shipped action is called, every called action is shipped"
    );
    for name in &shipped {
        let status = Command::new("git")
            .args([
                "cat-file",
                "-e",
                &format!("{pin}:.github/actions/{name}/action.yml"),
            ])
            .current_dir(root())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "pinned commit {pin} does not contain {name}"
        );
    }
}

#[test]
fn repository_checkout_fetches_the_history_needed_to_verify_gate_pins() {
    let data = workflow("ci-internal");
    let checkout = data["jobs"]["check"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|s| s.starts_with("actions/checkout@"))
        })
        .unwrap();
    assert_eq!(checkout["with"]["fetch-depth"], 0);
    assert_eq!(checkout["with"]["persist-credentials"], false);
}

#[test]
fn every_gate_reference_uses_the_private_provider_owner() {
    for entry in fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let data = workflow(path.file_stem().unwrap().to_str().unwrap());
        for job in data["jobs"].as_object().unwrap().values() {
            for step in job["steps"].as_array().into_iter().flatten() {
                if let Some(reference) = step["uses"].as_str()
                    && reference.contains("/rust-workflows/.github/actions/")
                {
                    assert!(reference.starts_with("Orchestration-Maestro/rust-workflows/"));
                }
            }
        }
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
        let executable = std::path::Path::new(directory).join("rust-gate");
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

/// Every `uses:` of one of this repository's actions across the workflows:
/// the pins seen, the actions called, and the number of call sites.
fn call_sites() -> (BTreeSet<String>, BTreeSet<String>, usize) {
    let mut pins = BTreeSet::new();
    let mut called = BTreeSet::new();
    let mut sites = 0;
    for entry in fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let data = workflow(path.file_stem().unwrap().to_str().unwrap());
        for job in data["jobs"].as_object().unwrap().values() {
            for step in job["steps"].as_array().into_iter().flatten() {
                let Some((name, pin)) = step["uses"].as_str().and_then(helper_action) else {
                    continue;
                };
                pins.insert(pin);
                called.insert(name);
                sites += 1;
            }
        }
    }
    (pins, called, sites)
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
            spec["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name}: input {input} has no description"
        );
    }
    for step in data["runs"]["steps"].as_array().unwrap() {
        if let Some(reference) = step["uses"].as_str() {
            let pin = reference.rsplit_once('@').unwrap().1;
            assert!(
                pin.len() == 40 && pin.bytes().all(|b| b.is_ascii_hexdigit()),
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
