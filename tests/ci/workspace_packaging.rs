//! `ci.yml`'s build step packages the workspace members that may be
//! published, one `--package` each, on a Cargo that can package members
//! depending on each other, once every archive an earlier run left is
//! removed.

use crate::harness::{Fixture, refused, succeeds};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

/// One workspace member as `cargo metadata --no-deps` reports it: its name,
/// its `publish` (null for any registry, the registries it names, or an empty
/// list for `publish = false`) and the members it depends on.
type Member<'a> = (&'a str, Value, &'a [&'a str]);

/// What `cargo metadata --no-deps` prints for a workspace of library members
/// under the fixture's project.
fn workspace_metadata(fixture: &Fixture, members: &[Member<'_>]) -> String {
    let project = fixture.root.join("project");
    let packages: Vec<Value> = members
        .iter()
        .map(|(name, publish, siblings)| {
            let dependencies: Vec<Value> = siblings
                .iter()
                .map(|sibling| {
                    json!({"name": sibling, "req": "^0.1.0", "kind": null,
                           "path": project.join(sibling)})
                })
                .collect();
            json!({"name": name, "version": "0.1.0", "id": format!("{name}@0.1.0"),
                   "publish": publish, "edition": "2024", "features": {},
                   "manifest_path": project.join(name).join("Cargo.toml"),
                   "dependencies": dependencies,
                   "targets": [{"kind": ["lib"], "name": name.replace('-', "_"),
                                "src_path": project.join(name).join("src/lib.rs")}]})
        })
        .collect();
    let ids: Vec<&Value> = packages.iter().map(|package| &package["id"]).collect();
    json!({"packages": packages, "workspace_members": ids, "workspace_root": project}).to_string()
}

/// A `cargo` that describes the workspace `METADATA` holds and records every
/// other call.
const WORKSPACE_CARGO: &str = r#"[[ "$1" == metadata ]] && printf '%s' "$METADATA"; exit 0"#;

/// A `cargo` that describes the workspace `METADATA` holds and fails to
/// package it the way Cargo does when a member it packages depends on the
/// `publish = false` member `maestro-kernel`.
const PRIVATE_KERNEL_CARGO: &str = r#"case "$1" in
  metadata) printf '%s' "$METADATA" ;;
  package) echo 'error: no matching package named `maestro-kernel` found' >&2; exit 101 ;;
esac"#;

/// Point `CARGO_TARGET_DIR`, where Cargo leaves its package archives, at the
/// fixture's `target`, and return it.
fn target_dir(fixture: &mut Fixture) -> PathBuf {
    let target = fixture.root.join("target");
    fixture.set("CARGO_TARGET_DIR", &target.display().to_string());
    target
}

/// Leave the archive `file` in the package directory of `target`, the way a
/// target directory restored from CI's cache or kept by a local run holds
/// one, and return its path.
fn stale_archive(target: &Path, file: &str) -> PathBuf {
    let archive = target.join("package").join(file);
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    fs::write(&archive, "an archive an earlier run left").unwrap();
    archive
}

#[test]
fn packaging_uses_a_cargo_that_can_package_a_workspace() {
    // Before Cargo 1.90, `cargo package` looked for a member's sibling on
    // crates.io, so a workspace whose members depend on each other could not
    // package on 1.85 to 1.89. The release build keeps the selected compiler;
    // only the packaging runs on Cargo 1.90, and it names every member that
    // may be published.
    for (selected, packager) in [
        ("1.85.0", Some("1.90.0")),
        ("1.89.0", Some("1.90.0")),
        ("1.90.0", None),
        ("1.98.1", None),
    ] {
        let mut fixture = Fixture::new();
        fixture.set("RUSTUP_TOOLCHAIN", selected);
        target_dir(&mut fixture);
        let metadata = workspace_metadata(
            &fixture,
            &[
                ("maestro-arithmetic", Value::Null, &[]),
                ("maestro-sum", Value::Null, &["maestro-arithmetic"]),
            ],
        );
        fixture.set("METADATA", &metadata);
        fixture.stub("cargo", WORKSPACE_CARGO);
        fixture.stub("rustup", "");
        succeeds(&fixture.run("ci", "build"));
        let trace = fixture.trace();
        let package = trace
            .lines()
            .find(|line| {
                line.ends_with(
                    "cargo package --locked --package maestro-arithmetic --package maestro-sum",
                )
            })
            .unwrap_or_else(|| panic!("{selected}: every member must be packaged: {trace}"));
        if let Some(version) = packager {
            assert!(
                package.contains(&format!("RUSTUP_TOOLCHAIN={version} ")),
                "{selected}: {package}"
            );
            let install = format!("rustup toolchain install {version} --profile minimal");
            assert!(trace.contains(&install), "{selected}: {trace}");
        } else {
            assert!(
                !package.contains("RUSTUP_TOOLCHAIN="),
                "{selected}: {package}"
            );
            assert!(
                !trace.contains("rustup toolchain install"),
                "{selected}: {trace}"
            );
        }
    }
}

#[test]
fn a_private_workspace_whose_members_depend_on_each_other_builds_without_packaging() {
    // Cargo takes a member's dependency on another member from the members it
    // packages only when that one may be published, so a workspace whose
    // members are all `publish = false` and depend on each other cannot
    // package: Cargo looks the dependency up in the registry it packages for.
    // None of them can be published either, so nothing is packaged, not even
    // on an older compiler that would need Cargo 1.90, and the step says so;
    // the release tests, the build and the SBOMs still run. An archive an
    // earlier run left, restored from CI's cache or kept by a local run, is
    // removed: staging would ship it though nothing was packaged.
    let mut fixture = Fixture::new();
    fixture.set("RUSTUP_TOOLCHAIN", "1.85.0");
    let target = target_dir(&mut fixture);
    let stale = stale_archive(&target, "maestro-kernel-0.1.0.crate");
    let metadata = workspace_metadata(
        &fixture,
        &[
            ("maestro-kernel", json!([]), &[]),
            ("maestro-knowledge", json!([]), &["maestro-kernel"]),
        ],
    );
    fixture.set("METADATA", &metadata);
    fixture.stub("cargo", PRIVATE_KERNEL_CARGO);
    fixture.stub("rustup", "");
    let output = fixture.run("ci", "build");
    succeeds(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SKIPPED: no workspace member may be published, so none is packaged"),
        "{stdout}"
    );
    assert!(
        !stale.exists(),
        "the archive an earlier run left would be staged"
    );
    let trace = fixture.trace();
    assert!(!trace.contains("cargo package"), "{trace}");
    assert!(!trace.contains("rustup toolchain install"), "{trace}");
    for command in [
        "cargo test --workspace --release --locked",
        "cargo auditable build --workspace --release --locked",
        "cargo cyclonedx --all",
    ] {
        assert!(trace.contains(command), "{command} must still run: {trace}");
    }
}

#[test]
fn a_publishable_member_depending_on_a_private_one_still_fails_to_package() {
    // A publishable member's dependency on a `publish = false` one is looked
    // up in the registry Cargo packages for, where it is not, and the member
    // could not be published either. It is packaged all the same and the step
    // fails with Cargo's own message: leaving it out would drop a publishable
    // crate from the payload without a word.
    let mut fixture = Fixture::new();
    target_dir(&mut fixture);
    let metadata = workspace_metadata(
        &fixture,
        &[
            ("maestro-kernel", json!([]), &[]),
            ("maestro-knowledge", Value::Null, &["maestro-kernel"]),
        ],
    );
    fixture.set("METADATA", &metadata);
    fixture.stub("cargo", PRIVATE_KERNEL_CARGO);
    refused(
        &fixture.run("ci", "build"),
        "no matching package named `maestro-kernel` found",
    );
    let trace = fixture.trace();
    let packaged: Vec<&str> = trace
        .lines()
        .filter(|line| line.contains("cargo package"))
        .collect();
    assert_eq!(
        packaged,
        ["cargo package --locked --package maestro-knowledge"],
        "{trace}"
    );
}

#[test]
fn a_mixed_workspace_packages_only_the_members_that_may_be_published() {
    // A member whose `publish` is unset or names a registry, crates.io or any
    // other, may be published and is packaged; one with `publish = false` is
    // left out, even when it depends on a member that is packaged, and an
    // archive of it an earlier run left is removed rather than staged.
    let mut fixture = Fixture::new();
    let target = target_dir(&mut fixture);
    let stale = stale_archive(&target, "maestro-xtask-0.1.0.crate");
    let metadata = workspace_metadata(
        &fixture,
        &[
            ("maestro-arithmetic", Value::Null, &[]),
            ("maestro-checked", json!(["crates-io"]), &[]),
            ("maestro-internal", json!(["internal"]), &[]),
            ("maestro-xtask", json!([]), &["maestro-arithmetic"]),
        ],
    );
    fixture.set("METADATA", &metadata);
    fixture.stub("cargo", WORKSPACE_CARGO);
    succeeds(&fixture.run("ci", "build"));
    assert!(
        !stale.exists(),
        "the archive an earlier run left would be staged"
    );
    let trace = fixture.trace();
    let packaged: Vec<&str> = trace
        .lines()
        .filter(|line| line.contains("cargo package"))
        .collect();
    assert_eq!(
        packaged,
        [concat!(
            "cargo package --locked --package maestro-arithmetic ",
            "--package maestro-checked --package maestro-internal"
        )],
        "{trace}"
    );
}
