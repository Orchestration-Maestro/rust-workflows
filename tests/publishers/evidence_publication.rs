//! Release evidence is verified and archived locally before any GitHub write.

use crate::harness::{Fixture, refused, succeeds, workflow};
use std::fs;

#[test]
fn evidence_publication_refuses_unverifiable_or_escaping_reports() {
    let revision = "a".repeat(40);
    let prepare = |f: &Fixture| {
        fs::create_dir_all(f.root.join("evidence")).unwrap();
        fs::write(
            f.root.join("evidence/scorecard.json"),
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
        let mut f = Fixture::new();
        prepare(&f);
        f.set("REVISION", &value);
        refused(&f.run("publish-evidence", "validate"), message);
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
    std::os::unix::fs::symlink("/etc/hostname", linked.root.join("evidence/link.json")).unwrap();
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
    let mut f = Fixture::new();
    f.trusted();
    refused(
        &f.run("publish-evidence", "upload"),
        "Release asset is missing or unsafe",
    );
    assert!(!f.calls().contains("release upload"));
    fs::write(f.root.join("evidence.tar.gz"), "locally verified archive").unwrap();
    succeeds(&f.run("publish-evidence", "upload"));
    assert!(f.calls().contains("release upload v0.1.0"));
    assert!(f.calls().contains("evidence.tar.gz"));
    assert!(!f.calls().contains("--clobber"));
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
    let mut f = Fixture::new();
    f.set("REVISION", &"a".repeat(40));
    fs::create_dir(f.root.join("evidence")).unwrap();
    fs::write(
        f.root.join("evidence/scorecard.json"),
        format!("{{\"revision\":\"{}\"}}", "a".repeat(40)),
    )
    .unwrap();
    succeeds(
        &std::process::Command::new("mkfifo")
            .arg(f.root.join("evidence/pipe"))
            .output()
            .unwrap(),
    );
    refused(
        &f.run("publish-evidence", "validate"),
        "Evidence entries must be regular files or directories",
    );
    assert!(!f.root.join("evidence.tar.gz").exists());
}
