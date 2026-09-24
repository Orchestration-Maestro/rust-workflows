//! Release evidence is verified and archived locally before any GitHub write.

use crate::harness::{Fixture, refused, succeeds, workflow};
use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

#[test]
fn evidence_publication_refuses_unverifiable_or_escaping_reports() {
    let revision = "a".repeat(40);
    let prepare = |fixture: &Fixture| {
        fs::create_dir_all(fixture.root.join("evidence")).unwrap();
        fs::write(
            fixture.root.join("evidence/scorecard.json"),
            format!("{{\"revision\":\"{revision}\"}}"),
        )
        .unwrap();
    };
    let mut good = Fixture::new();
    prepare(&good);
    good.set("REVISION", &revision);
    succeeds(&good.run("publish-evidence", "validate"));
    assert!(good.root.join("evidence.tar.gz").is_file());
    assert!(!good.calls().contains("gh"));

    for (value, message) in [
        (
            "main".to_owned(),
            "revision must be an immutable commit SHA",
        ),
        ("b".repeat(39), "revision must be an immutable commit SHA"),
        (
            "c".repeat(40),
            "Evidence revision does not match this source",
        ),
    ] {
        let mut fixture = Fixture::new();
        prepare(&fixture);
        fixture.set("REVISION", &value);
        refused(&fixture.run("publish-evidence", "validate"), message);
    }
    let mut missing = Fixture::new();
    missing.set("REVISION", &revision);
    fs::create_dir(missing.root.join("evidence")).unwrap();
    refused(
        &missing.run("publish-evidence", "validate"),
        "Reports are missing the run scorecard",
    );

    let mut linked = Fixture::new();
    prepare(&linked);
    linked.set("REVISION", &revision);
    symlink("/etc/hostname", linked.root.join("evidence/link.json")).unwrap();
    refused(
        &linked.run("publish-evidence", "validate"),
        "Evidence must not contain symlinks",
    );

    let mut mismatched = Fixture::new();
    prepare(&mismatched);
    mismatched.set("REVISION", &revision);
    fs::write(mismatched.root.join("evidence/scorecard.json"), "{}").unwrap();
    refused(
        &mismatched.run("publish-evidence", "validate"),
        "Scorecard revision does not match this source",
    );
}

#[test]
fn evidence_upload_requires_verified_files_and_the_existing_release() {
    let mut fixture = Fixture::new();
    fixture.trusted();
    refused(
        &fixture.run("publish-evidence", "upload"),
        "Release asset is missing or unsafe",
    );
    assert!(!fixture.calls().contains("release upload"));
    fs::write(
        fixture.root.join("evidence.tar.gz"),
        "locally verified archive",
    )
    .unwrap();
    succeeds(&fixture.run("publish-evidence", "upload"));
    assert!(fixture.calls().contains("release upload v0.1.0"));
    assert!(fixture.calls().contains("evidence.tar.gz"));
    assert!(!fixture.calls().contains("--clobber"));
}

#[test]
fn evidence_keeps_expiring_ci_artifacts_distinct_from_release_assets() {
    let data = workflow("publish-evidence");
    assert!(data["on"]["workflow_call"].get("secrets").is_none());
    assert!(
        data["on"]["workflow_call"]["inputs"]
            .get("repository-path")
            .is_none()
    );
    assert_eq!(data["jobs"]["publish"]["environment"], "release");
    assert_eq!(data["jobs"]["stage"]["permissions"]["contents"], "read");
    assert_eq!(data["jobs"]["publish"]["if"], "${{ !inputs.dry-run }}");
}

#[test]
fn evidence_refuses_special_files_before_creating_an_archive() {
    let mut fixture = Fixture::new();
    fixture.set("REVISION", &"a".repeat(40));
    fs::create_dir(fixture.root.join("evidence")).unwrap();
    fs::write(
        fixture.root.join("evidence/scorecard.json"),
        format!("{{\"revision\":\"{}\"}}", "a".repeat(40)),
    )
    .unwrap();
    succeeds(
        &Command::new("mkfifo")
            .arg(fixture.root.join("evidence/pipe"))
            .output()
            .unwrap(),
    );
    refused(
        &fixture.run("publish-evidence", "validate"),
        "Evidence entries must be regular files or directories",
    );
    assert!(!fixture.root.join("evidence.tar.gz").exists());
}
