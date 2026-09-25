//! The gate action: one pin at every call site, and a commit that ships it.

use crate::harness::{
    Fixture, action, helper_action, root, rust_files, succeeds, temp_dir, workflow, workflow_steps,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
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
fn the_pinned_gate_embeds_the_managed_files_this_commit_holds() {
    // The organization's bot renders a release's managed files with the gate
    // built at the release tag; every repository's CI checks them with the
    // gate action this commit pins. A managed file this repository is the
    // source of must read the same at the pinned commit, or the two renderings
    // disagree and every sync pull request fails, as v2.5.0's did. A change to
    // a source, the weekly tool moves included, lands before its repin can, so
    // it fails the release pull request alone and is named here until then.
    let (pins, _, _) = call_sites();
    let pin = pins.into_iter().next().unwrap();
    let render = fs::read_to_string(root().join("gate/src/steps/managed_files/render.rs")).unwrap();
    let sources: Vec<&str> = render
        .split("include_str!(\"../../../../")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert_eq!(sources.len(), 9, "{sources:?}");
    match pinned_gate_verdict(&root(), &pin, &sources) {
        Ok(Some(note)) => println!("{note}"),
        Ok(None) => {}
        Err(refusal) => panic!("{refusal}"),
    }
}

#[test]
fn a_source_changed_since_the_pinned_gate_fails_only_the_release() {
    let repository = temp_dir("pinned-gate");
    let git = |arguments: &[&str]| {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=gate",
                "-c",
                "user.email=gate@example.invalid",
            ])
            .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
            .args(["-c", "core.hooksPath=/dev/null"])
            .args(arguments)
            .current_dir(&repository)
            .output()
            .unwrap();
        succeeds(&output);
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    fs::write(repository.join("version.txt"), "1.0.0\n").unwrap();
    fs::write(repository.join(".editorconfig"), "root = true\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "chore: the first release"]);
    git(&["tag", "v1.0.0"]);
    let pin = git(&["rev-parse", "HEAD"]);
    let sources = [".editorconfig"];
    assert_eq!(pinned_gate_verdict(&repository, &pin, &sources), Ok(None));
    fs::write(repository.join(".editorconfig"), "root = false\n").unwrap();
    assert_eq!(
        pinned_gate_verdict(&repository, &pin, &sources),
        Ok(Some(format!(
            "REPIN: .editorconfig changed since the pinned gate {pin}: repin the gate before \
             the release"
        )))
    );
    fs::write(repository.join("version.txt"), "1.1.0\n").unwrap();
    assert_eq!(
        pinned_gate_verdict(&repository, &pin, &sources),
        Err(format!(
            ".editorconfig changed since the pinned gate {pin}: v1.1.0 is not tagged yet, so \
             this is its release; pin the gate action to a commit that holds them first"
        ))
    );
    fs::remove_dir_all(&repository).unwrap();
}

/// The `sources` of managed files in the repository at `root` that differ
/// from their copy at the gate commit `pin`: nothing when none does, a note
/// naming them while the version `version.txt` names is tagged, and a refusal
/// once it is not, which is the release pull request.
fn pinned_gate_verdict(root: &Path, pin: &str, sources: &[&str]) -> Result<Option<String>, String> {
    let differing: Vec<&str> = sources
        .iter()
        .copied()
        .filter(|source| {
            let pinned = Command::new("git")
                .args(["show", &format!("{pin}:{source}")])
                .current_dir(root)
                .output()
                .unwrap();
            !pinned.status.success()
                || Some(String::from_utf8_lossy(&pinned.stdout).into_owned())
                    != fs::read_to_string(root.join(source)).ok()
        })
        .collect();
    if differing.is_empty() {
        return Ok(None);
    }
    let changed = format!(
        "{} changed since the pinned gate {pin}",
        differing.join(", ")
    );
    let version = fs::read_to_string(root.join("version.txt")).unwrap();
    let tag = format!("v{}", version.trim());
    let tagged = Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/tags/{tag}"),
        ])
        .current_dir(root)
        .output()
        .unwrap()
        .status
        .success();
    if tagged {
        Ok(Some(format!(
            "REPIN: {changed}: repin the gate before the release"
        )))
    } else {
        Err(format!(
            "{changed}: {tag} is not tagged yet, so this is its release; pin the gate action \
             to a commit that holds them first"
        ))
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
                .is_some_and(|uses| uses.starts_with("actions/checkout@"))
        })
        .unwrap();
    assert_eq!(checkout["with"]["fetch-depth"], 0);
    assert_eq!(checkout["with"]["persist-credentials"], false);
}

#[test]
fn every_gate_reference_uses_the_private_provider_owner() {
    for (_, _, step) in workflow_steps() {
        if let Some(reference) = step["uses"].as_str()
            && reference.contains("/rust-workflows/.github/actions/")
        {
            assert!(reference.starts_with("Orchestration-Maestro/rust-workflows/"));
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

/// Every `uses:` of one of this repository's actions across the workflows:
/// the pins seen, the actions called, and the number of call sites.
fn call_sites() -> (BTreeSet<String>, BTreeSet<String>, usize) {
    let mut pins = BTreeSet::new();
    let mut called = BTreeSet::new();
    let mut sites = 0;
    for (_, _, step) in workflow_steps() {
        let Some((name, pin)) = step["uses"].as_str().and_then(helper_action) else {
            continue;
        };
        pins.insert(pin);
        called.insert(name);
        sites += 1;
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

#[test]
fn every_file_the_gate_reads_when_built_ships_in_the_action_archive() {
    // GitHub fetches the action as the repository's archive, which leaves out
    // every export-ignore path; a file the gate reads at build time and the
    // archive leaves out fails every call site's build.
    let mut read = 0;
    for source in rust_files(&root().join("gate/src")) {
        let text = fs::read_to_string(&source).unwrap();
        let directory = source.parent().unwrap();
        for (index, _) in text.match_indices("include_str!(\"") {
            let rest = &text[index + "include_str!(\"".len()..];
            let relative = rest.split('"').next().unwrap();
            let Ok(file) = directory.join(relative).canonicalize() else {
                // A fixture inside a test's string, not a file of the gate.
                continue;
            };
            let path = file.strip_prefix(root()).unwrap().display().to_string();
            let attribute = Command::new("git")
                .args(["check-attr", "export-ignore", "--", &path])
                .current_dir(root())
                .output()
                .unwrap();
            let attribute = String::from_utf8_lossy(&attribute.stdout).into_owned();
            assert!(
                attribute.ends_with(": unspecified\n"),
                "the gate reads {path} when built, and the action archive leaves it out: \
                 {attribute}"
            );
            read += 1;
        }
    }
    assert!(read >= 6, "only {read} files the gate reads were checked");
}
