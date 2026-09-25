//! `ci.yml`'s `hooks` step, the local runs a commit hook makes, and the first
//! step of `hygiene.yml`: the organization's commit hooks over every file, a
//! step outside Actions, and the checkout a repository without Rust is.

use crate::harness::{Fixture, refused, root, succeeds, tool};
use std::fs;

#[test]
fn the_hooks_step_runs_prek_over_every_file_and_skips_what_ci_runs_itself() {
    let mut fixture = Fixture::new();
    // The hooks run on the toolbelt `rust-gate setup` installs, installed first.
    let tools = fixture.cached_mise("");
    fixture.stub(
        "prek",
        "printf '%s SKIP=%s\\n' \"$*\" \"$SKIP\" >> \"$CALLS\"\necho 'every hook passed'",
    );
    succeeds(&fixture.run("ci", "hooks"));
    assert!(tools.join("bin/mise").exists());
    assert!(
        fixture
            .calls()
            .contains("mise\ninstall --locked\nmise\nbin-paths\nprek\n"),
        "{}",
        fixture.calls()
    );
    assert!(
        fixture.calls().contains(
            "run --all-files --show-diff-on-failure --color never \
             SKIP=rustfmt,clippy,rust-gate-architecture,rust-gate-hygiene,rust-gate-rules,\
             rust-gate-guide"
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
    // A clippy.toml the repository writes itself may tighten the
    // organization's thresholds, never loosen them.
    let looser = fs::read_to_string(root().join("clippy.toml"))
        .unwrap()
        .replace("= 100", "= 150");
    fs::write(project.join("clippy.toml"), looser).unwrap();
    refused(
        &fixture.run_body("cd project && rust-gate architecture --local"),
        "source rules: 1 finding",
    );
}

#[test]
fn the_clippy_hook_hands_clippy_the_organization_thresholds_at_run_time() {
    // No clippy.toml in the repository: Clippy reads the organization's from
    // a scratch directory. One the repository wrote itself is Clippy's to
    // read, held no looser by the source rules.
    let fixture = Fixture::new();
    fixture.stub(
        "cargo",
        r#"printf 'CLIPPY_CONF_DIR=%s\n' "${CLIPPY_CONF_DIR:-}" >> "$CALLS"
if [[ -n "${CLIPPY_CONF_DIR:-}" ]]; then cat "$CLIPPY_CONF_DIR/clippy.toml" >> "$CALLS"; fi"#,
    );
    succeeds(&fixture.run_body("cd project && rust-gate clippy --local"));
    let calls = fixture.calls();
    assert!(
        calls.contains("clippy --workspace --all-targets --locked -- -D warnings\n"),
        "{calls}"
    );
    assert!(
        calls.contains("too-many-lines-threshold = 100\n"),
        "{calls}"
    );
    let project = fixture.root.join("project");
    fs::write(
        project.join("clippy.toml"),
        "too-many-lines-threshold = 80\n",
    )
    .unwrap();
    fs::write(fixture.root.join("calls"), "").unwrap();
    succeeds(&fixture.run_body("cd project && rust-gate clippy --local"));
    assert!(
        fixture.calls().contains("CLIPPY_CONF_DIR=\n"),
        "{}",
        fixture.calls()
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
