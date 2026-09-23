//! The toolbelt the gate runs on: every pinned tool linked from `.tools/bin`
//! to its locked build, and every line of Bash left in the repository under
//! `ShellCheck`.

use crate::harness::{action, capture, command_line, root};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;

#[test]
fn every_toolbelt_link_points_at_the_locked_build() {
    // mise.lock is the pin. A tool absent at its locked version, or a link in
    // .tools/bin that points at any other build, means `just setup` has not
    // run since the pins changed, and the gate would quietly test the wrong
    // tool.
    let root = root();
    let missing = capture(command_line("mise ls --missing").current_dir(&root));
    assert!(
        missing.trim().is_empty(),
        "not installed at the locked version; run just setup:\n{missing}"
    );
    let mut checked = 0;
    for directory in capture(command_line("mise bin-paths").current_dir(&root)).lines() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let executable =
                path.is_file() && path.metadata().unwrap().permissions().mode() & 0o111 != 0;
            if !executable {
                continue;
            }
            let name = path.file_name().unwrap();
            assert_eq!(
                fs::canonicalize(root.join(".tools/bin").join(name)).ok(),
                fs::canonicalize(&path).ok(),
                "{} in .tools/bin is not the locked build; run just setup",
                name.to_string_lossy()
            );
            checked += 1;
        }
    }
    assert!(checked > 5, "only {checked} tools linked; the walk drifted");
}

#[test]
fn every_bash_line_left_in_the_repository_passes_shellcheck() {
    // Bash survives in three places: the Just recipes, bootstrap.sh and the
    // action's shell steps. actionlint reads none of them, so ShellCheck runs
    // over each one here, the recipes taken from Just's own dump.
    let root = root();
    let dump = capture(command_line("just --dump --dump-format json").current_dir(&root));
    let recipes: Value = serde_json::from_str(&dump).unwrap();
    let mut checked = 0;
    for (name, recipe) in recipes["recipes"].as_object().unwrap() {
        let body: Vec<String> = recipe["body"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| {
                line.as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<String>()
            })
            .collect();
        shellcheck(&body.join("\n"), &format!("recipe {name}"));
        checked += 1;
    }
    shellcheck(
        &fs::read_to_string(root.join("scripts/bootstrap.sh")).unwrap(),
        "scripts/bootstrap.sh",
    );
    checked += 1;
    checked += 1;
    // The gate action's build step is Bash the workflows call by SHA.
    for entry in fs::read_dir(root.join(".github/actions")).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        for step in action(&name)["runs"]["steps"].as_array().unwrap() {
            if let Some(run) = step["run"].as_str() {
                shellcheck(run, &format!("action {name}"));
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 4,
        "only {checked} Bash bodies checked; the walk drifted"
    );
}

/// `ShellCheck` over one body, as Bash, the way a workflow step is held.
fn shellcheck(body: &str, label: &str) {
    let mut child = command_line("shellcheck --shell=bash -")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("shellcheck must be installed: just setup");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "ShellCheck rejects {label}:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
