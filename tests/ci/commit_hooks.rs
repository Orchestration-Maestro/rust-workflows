//! `ci.yml`'s `hooks` step, the local runs a commit hook makes, and the first
//! step of `hygiene.yml`: the organization's commit hooks over every file, a
//! step outside Actions, and the checkout a repository without Rust is.

use crate::harness::{Fixture, refused, succeeds, tool};
use std::fs;

#[test]
fn the_hooks_step_runs_prek_over_every_file_and_skips_what_ci_runs_itself() {
    let fixture = Fixture::new();
    fixture.stub(
        "prek",
        "printf '%s SKIP=%s\\n' \"$*\" \"$SKIP\" >> \"$CALLS\"\necho 'every hook passed'",
    );
    succeeds(&fixture.run("ci", "hooks"));
    assert!(
        fixture.calls().contains(
            "run --all-files --show-diff-on-failure --color never \
             SKIP=rustfmt,clippy,rust-gate-architecture,rust-gate-hygiene"
        ),
        "{}",
        fixture.calls()
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("reports/hooks.txt")).unwrap(),
        "every hook passed\n"
    );
    fixture.stub("prek", "echo 'trailing whitespace'\nexit 1");
    assert!(!fixture.run("ci", "hooks").status.success());
}

#[test]
fn a_step_runs_locally_the_way_a_commit_hook_runs_it() {
    let fixture = Fixture::with_sources(&[("src/lib.rs", "//! A crate.\n")]);
    let project = fixture.root.join("project");
    fs::copy(
        fixture.root.join("clippy.toml"),
        project.join("clippy.toml"),
    )
    .unwrap();
    for (path, text) in [("README.md", "# Fixture\n"), ("LICENSE", "MIT\n")] {
        fs::write(project.join(path), text).unwrap();
    }
    let git = |args: &[&str]| {
        succeeds(
            &tool("git")
                .args(args)
                .current_dir(&project)
                .output()
                .unwrap(),
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    succeeds(&fixture.run_body("cd project && rust-gate architecture --local"));
    succeeds(&fixture.run_body("cd project && rust-gate hygiene --local"));
    fs::write(project.join("src/lib.rs"), "//! A crate.\n// TODO: split\n").unwrap();
    refused(
        &fixture.run_body("cd project && rust-gate hygiene --local"),
        "hygiene: 1 finding",
    );
    fs::remove_file(project.join("clippy.toml")).unwrap();
    refused(
        &fixture.run_body("cd project && rust-gate architecture --local"),
        "source rules: 1 finding",
    );
}

#[test]
fn the_hygiene_workflow_names_its_checkout_and_reports_directory() {
    let fixture = Fixture::new();
    succeeds(&fixture.run("hygiene", "prepare"));
    let exported = fs::read_to_string(fixture.root.join("environment")).unwrap();
    let reports = fixture.root.join("hygiene-reports");
    assert!(
        exported.contains(&format!("PROJECT={}\n", fixture.env["GITHUB_WORKSPACE"])),
        "{exported}"
    );
    assert!(
        exported.contains(&format!("REPORTS={}\n", reports.display())),
        "{exported}"
    );
    assert!(fixture.root.join("hygiene-reports").is_dir());
}
