//! Repository-wide workflow policy: pins, permissions, runners, trust
//! boundaries and the guards every workflow shares.

use crate::harness::{Fixture, root, step, succeeds, tool, workflow};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn public_workflow_files_exist() {
    for name in ["ci", "publish-binaries", "publish-crate"] {
        assert!(
            root()
                .join(format!(".github/workflows/{name}.yml"))
                .is_file()
        );
    }
}

#[test]
fn repository_uses_just_and_an_isolated_rust_test_harness() {
    for required in [
        "justfile",
        "tests/Cargo.toml",
        "tests/Cargo.lock",
        "tests/workflows.rs",
    ] {
        assert!(
            root().join(required).is_file(),
            "Missing validation entrypoint: {required}"
        );
    }
    assert!(
        !root().join("Cargo.toml").exists(),
        "No root Rust application"
    );
    for obsolete in ["scripts/check.sh", "scripts/setup-tools.sh"] {
        assert!(
            !root().join(obsolete).exists(),
            "Use Just instead of {obsolete}"
        );
    }
}

#[test]
fn permissions_timeouts_and_shell_policy_hold_in_every_workflow() {
    for name in ["ci", "publish-binaries", "publish-crate", "ci-internal"] {
        let data = workflow(name);
        assert!(data.get("permissions").is_some());
        for job in data["jobs"].as_object().unwrap().values() {
            assert!(job.get("permissions").is_some());
            if job.get("steps").is_some() {
                assert!(job.get("timeout-minutes").is_some());
            }
            for item in std::iter::once(job).chain(job["steps"].as_array().into_iter().flatten()) {
                assert!(item.get("continue-on-error").is_none());
                if let Some(command) = item["run"].as_str() {
                    body_follows_the_shell_policy(command);
                }
            }
        }
    }
}

/// Consumer input reaches a step through `env`, never through `${{ }}` inside
/// a body, which is the script-injection path; nothing is silenced; and shell
/// that does not stop on the first failure reports success for work it
/// skipped, so a body is fail-closed Bash, a bare cargo or rustup invocation,
/// or a gate command.
fn body_follows_the_shell_policy(command: &str) {
    assert!(!command.contains("${{"));
    assert!(!command.contains("|| true"));
    let has_commands = command.lines().any(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with('#')
    });
    if has_commands {
        assert!(
            command.starts_with("set -euo pipefail\n")
                || command.starts_with("cargo ")
                || command.starts_with("rustup ")
                || command.starts_with("rust-gate "),
            "shell must fail closed, be a bare cargo invocation, or call the gate: {command}"
        );
    }
}

#[test]
fn all_jobs_use_github_runners_without_caller_overrides() {
    for name in [
        "ci",
        "publish-binaries",
        "publish-crate",
        "ci-internal",
        "attest-binaries",
        "publish-evidence",
        "unsafe-audit",
        "fuzz",
    ] {
        let data = workflow(name);
        for input in ["runs-on", "publish-runs-on"] {
            assert!(
                data["on"]["workflow_call"]["inputs"].get(input).is_none(),
                "{name} allows a runner override: {input}"
            );
        }
        for (id, job) in data["jobs"].as_object().unwrap() {
            if job.get("steps").is_some() {
                assert_eq!(job["runs-on"], "ubuntu-24.04", "{name}/{id}");
            }
            if let Some(runner) = job["with"].get("runs-on") {
                assert_eq!(runner, "ubuntu-24.04", "{name}/{id}");
            }
        }
    }
}

#[test]
fn ci_entry_jobs_accept_fork_pull_requests() {
    // CI takes no secret and runs with a read-only token, and the organization
    // holds every external contributor's run for approval: a fork pull request
    // is checked like any other, so an outside contribution can pass the
    // required status. pull_request_target, which runs with the base
    // repository's privileges, stays refused.
    for (name, job) in [("ci", "checks"), ("ci-internal", "check")] {
        assert_eq!(
            workflow(name)["jobs"][job]["if"],
            "${{ github.event_name != 'pull_request_target' }}",
            "{name}/{job}"
        );
        assert!(
            workflow(name)["on"]["workflow_call"]["secrets"].is_null(),
            "{name} must take no secret to run fork pull requests"
        );
    }
}

#[test]
fn publisher_entry_jobs_reject_untrusted_pull_request_contexts() {
    for (name, job) in [
        ("publish-binaries", "preflight"),
        ("publish-crate", "preflight"),
        ("publish-evidence", "preflight"),
        ("attest-binaries", "attest"),
    ] {
        assert_eq!(
            workflow(name)["jobs"][job]["if"],
            "${{ github.event_name != 'pull_request_target' && \
             (!github.event.pull_request || \
             github.event.pull_request.head.repo.full_name == github.repository) }}",
            "{name}/{job} must reject fork PRs before a runner is allocated"
        );
    }
}

#[test]
fn the_path_guard_is_identical_in_every_workflow_that_takes_a_directory() {
    // A reusable workflow runs inside the consumer's checkout, so the three
    // workflows that take a working directory used to embed the same Bash
    // guard three times. The gate holds it once, and running each validate
    // step against the same bad directories has to produce the same refusal,
    // word for word: not a simple path, a traversal, a symlink out of the
    // checkout, a directory that is not there.
    let cases = [
        ("../escape", "simple relative path"),
        ("project/../escape", "traverse or contain option-like"),
        ("project/out", "escapes checkout"),
        ("project/none", "does not exist inside checkout"),
    ];
    let mut refusals: Vec<Vec<String>> = Vec::new();
    for (workflow, command) in [
        ("ci", "rust-gate validate"),
        ("fuzz", "rust-gate fuzz validate"),
        ("unsafe-audit", "rust-gate unsafe-audit validate"),
    ] {
        assert_eq!(step(workflow, "validate").trim(), command);
        let mut seen = Vec::new();
        for (directory, refusal) in cases {
            let mut f = Fixture::new();
            if directory == "project/out" {
                symlink("/", f.root.join("project/out")).unwrap();
            }
            f.set("DIRECTORY", directory);
            let output = f.run(workflow, "validate");
            assert!(!output.status.success(), "{workflow} accepted {directory}");
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            assert!(
                stderr.contains(refusal),
                "{workflow} refused {directory} for another reason: {stderr}"
            );
            seen.push(stderr);
        }
        refusals.push(seen);
    }
    assert!(
        refusals.iter().all(|seen| seen == &refusals[0]),
        "the three workflows refuse the same directory differently: {refusals:?}"
    );
}

#[test]
fn security_policy_documents_how_to_verify_a_release() {
    let text = fs::read_to_string(root().join("SECURITY.md")).unwrap();
    // A reader who cannot verify a release has no way to tell a real asset from
    // a substituted one, whatever the pipeline produced.
    for step in [
        "sha256sum --check",
        "gh attestation verify",
        "payload.spdx.json",
        "cargo audit bin",
    ] {
        assert!(text.contains(step), "SECURITY.md must document: {step}");
    }
    // The procedure has never been run against a published asset. Saying so is
    // part of the instruction, not a disclaimer to be dropped later.
    assert!(
        text.contains("No release has been published"),
        "unexercised verification steps must say they are unexercised"
    );
}

#[test]
fn the_consumer_matrix_calls_every_local_workflow_and_fixture() {
    let matrix = workflow("ci-internal");
    for name in ["ci", "publish-binaries", "publish-crate"] {
        assert!(
            matrix["jobs"]
                .as_object()
                .unwrap()
                .values()
                .any(|job| job["uses"] == format!("./.github/workflows/{name}.yml"))
        );
    }
    for example in ["binary", "library", "workspace"] {
        let base = root().join("examples").join(example);
        assert!(base.join("Cargo.lock").is_file());
        let result = tool("jaq")
            .args(["--from", "toml", "-r", ".toolchain.channel"])
            .arg(base.join("rust-toolchain.toml"))
            .output()
            .unwrap();
        succeeds(&result);
        assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), "1.98.1");
    }
    for member in ["binary", "library", "workspace/core", "workspace/app"] {
        let output = tool("jaq")
            .args(["--from", "toml", "."])
            .arg(root().join("examples").join(member).join("Cargo.toml"))
            .output()
            .unwrap();
        succeeds(&output);
        let manifest: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(manifest["package"]["edition"], "2024");
        assert_eq!(manifest["package"]["rust-version"], "1.85");
        // Fixtures depend on each other by path. The one exception is the
        // binary fixture's single crates.io dependency, there so the dependency
        // policy, the SBOMs and the embedded dependency list are checked against
        // a real lockfile; it also makes the hosted consumer matrix fetch one crate
        // directly from crates.io, as every consumer does.
        for (name, dependency) in manifest["dependencies"]
            .as_object()
            .into_iter()
            .flat_map(|d| d.iter())
        {
            assert!(
                dependency.get("path").is_some() || (member == "binary" && name == "anyhow"),
                "{member}: unexpected registry dependency {name}"
            );
        }
    }
}

#[test]
fn canonical_document_generation_and_native_status_remain_safe() {
    let recipes = fs::read_to_string(root().join("justfile")).unwrap();
    assert!(
        recipes.contains("[linux]\ndocs:"),
        "canonical docs require Linux"
    );
    assert!(recipes.contains("set windows-shell := [\"cmd.exe\", \"/d\", \"/s\", \"/c\"]"));
}
