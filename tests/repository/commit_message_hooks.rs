//! The commit-msg hooks: a message opens with a conventional header and every
//! line stays within 80 columns, or prek refuses it before release-please
//! ever reads it.

use crate::harness::{Fixture, query, root, succeeds, temp_dir, tool};
use std::fs;
use std::process::Command;

/// Whether prek accepts `message` at the commit-msg stage, with its log. The
/// hooks run in a fresh repository holding a copy of this one's config: prek
/// stashes unstaged changes while it runs, which no test may suffer.
fn accepted(message: &str) -> (bool, String) {
    let repository = temp_dir("commit-message");
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let config = ".pre-commit-config.yaml";
    fs::copy(root().join(config), repository.join(config)).unwrap();
    fs::write(repository.join("message"), message).unwrap();
    let output = tool("prek")
        .args([
            "run",
            "--stage",
            "commit-msg",
            "--commit-msg-filename",
            "message",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    (output.status.success(), log)
}

#[test]
fn the_commit_message_hooks_refuse_a_bad_header_and_a_long_line() {
    // release-please writes the changelog from commit titles; a title it
    // cannot read is a release note nobody wrote.
    let (ok, log) = accepted("feat: add the commit message hooks\n\n- one body line\n");
    assert!(ok, "{log}");
    let too_wide = format!("fix: short\n\n{}\n", "x".repeat(81));
    for (message, hook) in [
        ("Add the hooks\n", "conventional-commit-header"),
        (
            "notes first\nfeat: header on the second line\n",
            "conventional-commit-header",
        ),
        ("Feat: a capital type\n", "conventional-commit-header"),
        (too_wide.as_str(), "eighty-columns"),
    ] {
        let (ok, log) = accepted(message);
        assert!(!ok, "{message:?} was accepted");
        assert!(
            log.contains(hook),
            "{message:?} was refused by another hook: {log}"
        );
    }
}

#[test]
fn the_commit_message_hooks_are_installed_with_the_pre_commit_shim() {
    let hooks = root().join(".pre-commit-config.yaml");
    assert_eq!(
        query(&hooks, ".default_install_hook_types | join(\",\")"),
        "pre-commit,commit-msg"
    );
    let ids = query(
        &hooks,
        ".repos[].hooks[] | select((.stages // []) | contains([\"commit-msg\"])) | .id",
    );
    assert_eq!(
        ids.lines().collect::<Vec<_>>(),
        ["conventional-commit-header", "eighty-columns"]
    );
}

#[test]
fn disposable_snapshot_hooks_reject_a_staged_bad_file() {
    let fixture = Fixture::new();
    succeeds(
        &tool("git")
            .args(["init", "-q"])
            .current_dir(&fixture.root)
            .output()
            .unwrap(),
    );
    fs::copy(
        root().join(".pre-commit-config.yaml"),
        fixture.root.join(".pre-commit-config.yaml"),
    )
    .unwrap();
    fs::write(fixture.root.join("bad.txt"), "trailing space \n").unwrap();
    succeeds(
        &tool("git")
            .args(["add", "bad.txt"])
            .current_dir(&fixture.root)
            .output()
            .unwrap(),
    );
    let output = tool("prek")
        .args(["run", "trailing-whitespace", "--all-files"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Failed"));
}
