//! The shared payload verification: the revision, the checksum manifest and
//! the provenance are each checked before a release is handed on, and every
//! way they can be wrong is refused by name.

use crate::harness::{Fixture, checksums, refused, succeeds};
use serde_json::json;
use std::fs;
use std::os::unix::fs::symlink;

/// The body every workflow that verifies a payload runs.
const VERIFY: &str = "rust-gate verify-payload";

#[test]
fn artifact_integrity_revision_target_and_binary_selection_are_checked() {
    let mut f = Fixture::new();
    let release = f.root.join("rust-release");
    fs::create_dir(&release).unwrap();
    fs::write(release.join("payload.tar.gz"), "not an executable").unwrap();
    f.set("REVISION", &"a".repeat(40));
    f.set("REQUIRE_BINARIES", "true");
    let original = json!({"revision": "a".repeat(40), "target": "x86_64-unknown-linux-gnu",
                          "binaries": ["fixture"]});
    fs::write(release.join("provenance.json"), original.to_string()).unwrap();
    checksums(&release);
    succeeds(&f.run_body(VERIFY));
    for (key, value) in [
        ("revision", json!("b".repeat(40))),
        ("target", json!("wrong")),
        ("binaries", json!([])),
    ] {
        let mut provenance = original.clone();
        provenance[key] = value;
        fs::write(release.join("provenance.json"), provenance.to_string()).unwrap();
        checksums(&release);
        refused(
            &f.run_body(VERIFY),
            "Unexpected release revision, target or binary selection",
        );
    }
    fs::write(release.join("provenance.json"), original.to_string()).unwrap();
    checksums(&release);
    succeeds(&f.run_body(VERIFY));
}

#[test]
fn a_checksum_manifest_names_each_release_file_once_and_is_never_a_link() {
    let mut f = Fixture::new();
    let release = f.root.join("rust-release");
    fs::create_dir(&release).unwrap();
    fs::write(release.join("payload.tar.gz"), "payload").unwrap();
    fs::write(release.join("provenance.json"), "{}").unwrap();
    f.set("REVISION", &"a".repeat(40));
    let digest = "a".repeat(64);
    let selector = "Invalid release checksum selector";
    for (manifest, message) in [
        ("invalid checksum\n".to_owned(), selector),
        (format!("{digest}  ../outside\n"), selector),
        (
            format!("{digest}  payload.tar.gz\n"),
            "Missing required release checksums",
        ),
        (
            format!("{digest}  payload.tar.gz\n{digest}  payload.tar.gz\n"),
            "Duplicate checksum selector or symlink",
        ),
    ] {
        fs::write(release.join("SHA256SUMS"), manifest).unwrap();
        refused(&f.run_body(VERIFY), message);
    }
    fs::remove_file(release.join("SHA256SUMS")).unwrap();
    refused(&f.run_body(VERIFY), "Missing required release checksums");
    symlink("/etc/hostname", release.join("SHA256SUMS")).unwrap();
    refused(&f.run_body(VERIFY), "Checksum file must not be a symlink");
    fs::remove_file(release.join("SHA256SUMS")).unwrap();
    checksums(&release);
    f.set("REVISION", "main");
    refused(
        &f.run_body(VERIFY),
        "revision must be an immutable commit SHA",
    );
    f.set("REVISION", &"b".repeat(40));
    refused(
        &f.run_body(VERIFY),
        "Release revision does not match source",
    );
}
