//! `ci.yml`: what `validate` accepts and refuses, and the registry it configures.

use crate::harness::{Fixture, refused, succeeds, workflow};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

#[test]
fn ci_accepts_any_exact_stable_from_the_msrv_and_keeps_matrix_artifacts_distinct() {
    // The five pins are what the consumer matrix proves; what CI accepts is any
    // exact stable release from the MSRV up: the project declares its compiler.
    let versions = ["1.85.0", "1.95.0", "1.96.1", "1.97.1", "1.98.1"];
    assert_eq!(
        workflow("ci-internal")["jobs"]["consumers"]["strategy"]["matrix"]["rust-version"],
        json!(versions)
    );
    assert_eq!(
        workflow("ci")["on"]["workflow_call"]["inputs"]["rust-version"]["default"],
        ""
    );
    let mut artifacts = std::collections::BTreeSet::new();
    for version in versions.into_iter().chain(["1.90.0", "1.99.3"]) {
        let mut fixture = Fixture::new();
        fixture.set("REQUESTED_TOOLCHAIN", version);
        succeeds(&fixture.run("ci", "validate"));
        let environment = fs::read_to_string(fixture.root.join("environment")).unwrap();
        assert!(
            environment
                .lines()
                .any(|line| line == format!("RUSTUP_TOOLCHAIN={version}"))
        );
        assert!(artifacts.insert(fs::read_to_string(fixture.root.join("output")).unwrap()));
    }
    let exact = "rust-version must be an exact stable version, such as 1.98.1";
    for (version, message) in [
        ("stable", exact),
        ("1.84.1", "rust-version must be at least the 1.85.0 MSRV"),
        ("1.98", exact),
        ("1.98.1\nINJECT=yes", exact),
    ] {
        let mut fixture = Fixture::new();
        fixture.set("REQUESTED_TOOLCHAIN", version);
        refused(&fixture.run("ci", "validate"), message);
    }
}

#[test]
fn matrix_versions_have_distinct_concurrency_groups() {
    let ci = workflow("ci");
    let caller = workflow("ci-internal");
    let consumers = &caller["jobs"]["consumers"];
    let matrix = &consumers["strategy"]["matrix"];
    let group = ci["concurrency"]["group"].as_str().unwrap();
    let mut groups = std::collections::BTreeSet::new();
    for example in matrix["example"].as_array().unwrap() {
        for version in matrix["rust-version"].as_array().unwrap() {
            let directory = consumers["with"]["working-directory"]
                .as_str()
                .unwrap()
                .replace("${{ matrix.example }}", example.as_str().unwrap());
            let version = consumers["with"]["rust-version"]
                .as_str()
                .unwrap()
                .replace("${{ matrix.rust-version }}", version.as_str().unwrap());
            let key = group
                .replace("${{ github.workflow }}", "Internal CI")
                .replace("${{ github.ref }}", "refs/pull/1/merge")
                .replace("${{ inputs.working-directory }}", &directory)
                .replace("${{ inputs.rust-version }}", &version)
                .replace(
                    "${{ inputs.artifact-key }}",
                    consumers["with"]["artifact-key"].as_str().unwrap(),
                );
            assert!(
                !key.contains("${{"),
                "unresolved concurrency expression: {key}"
            );
            assert!(groups.insert(key.clone()), "matrix collision: {key}");
        }
    }
    assert_eq!(groups.len(), 15);
    assert_eq!(
        ci["concurrency"]["cancel-in-progress"],
        "${{ github.event_name == 'pull_request' }}"
    );
}

#[test]
fn the_declared_msrv_must_be_real_and_reachable() {
    // A package that declares no MSRV, or one above the compiler in use, makes
    // the support policy unverifiable: the matrix would pass while telling a
    // consumer nothing about the versions it claims.
    let metadata =
        |packages: Value| json!({"workspace_members": ["p"], "packages": packages}).to_string();
    let run = |declared: Option<&str>, toolchain: &str| {
        let mut f = Fixture::new();
        let mut package = json!({"id": "p", "name": "fixture"});
        if let Some(value) = declared {
            package["rust_version"] = json!(value);
        }
        fs::write(f.root.join("metadata.json"), metadata(json!([package]))).unwrap();
        f.set("RUSTUP_TOOLCHAIN", toolchain);
        f.stub("rustup", "exit 0");
        f.stub("cargo", "exit 0");
        f.run("ci", "msrv")
    };

    succeeds(&run(Some("1.85.0"), "1.98.1"));
    succeeds(&run(Some("1.98.1"), "1.98.1"));
    // Declared above the compiler actually selected: the claim cannot hold.
    refused(&run(Some("1.99.0"), "1.98.1"), "declares rust-version");
    // Absent or unusable.
    refused(&run(None, "1.98.1"), "must declare rust-version (its MSRV)");
    for bad in ["stable", "1", "latest", "1.x"] {
        refused(
            &run(Some(bad), "1.98.1"),
            "declares an unusable rust-version:",
        );
    }
    // The comparison must be numeric per component, not lexical. Rust 1.9
    // really does precede 1.85, so this is allowed:
    succeeds(&run(Some("1.9.0"), "1.85.0"));
    // and this is not, even though "1.85.0" sorts before "1.9.0" as plain text.
    refused(&run(Some("1.85.0"), "1.9.0"), "declares rust-version");
}

#[test]
fn the_declared_msrv_is_the_compiler_the_workspace_is_checked_with() {
    // Validating the declaration proves only that a number was written down.
    // A workspace using an API newer than the version it claims to support
    // passes that reading and breaks for the consumer who believed it, so the
    // step installs the oldest compiler the declarations allow and builds.
    let package =
        |name: &str, version: &str| json!({"id": name, "name": name, "rust_version": version});
    let run = |packages: Value| {
        let f = Fixture::new();
        let members: Vec<&str> = packages
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["id"].as_str().unwrap())
            .collect();
        fs::write(
            f.root.join("metadata.json"),
            json!({"workspace_members": members, "packages": packages}).to_string(),
        )
        .unwrap();
        f.stub("rustup", "exit 0");
        f.stub("cargo", "exit 0");
        let outcome = f.run("ci", "msrv");
        (outcome, format!("{}\n{}", f.calls(), f.trace()))
    };

    // One declaration: it is the floor, and both commands name it.
    let (outcome, calls) = run(json!([package("p", "1.85")]));
    succeeds(&outcome);
    assert!(
        calls.contains("rustup\ntoolchain install 1.85.0 --profile minimal"),
        "the declared version must be installed: {calls}"
    );
    assert!(
        calls.contains("RUSTUP_TOOLCHAIN=1.85.0 cargo check --workspace --locked"),
        "the workspace must be checked at that version: {calls}"
    );

    // Several declarations: Cargo refuses a member above the active toolchain,
    // so the floor is the highest, and the run says the others ride on it.
    let (outcome, calls) = run(json!([package("a", "1.80"), package("b", "1.85")]));
    succeeds(&outcome);
    assert!(
        calls.contains("toolchain install 1.85.0 "),
        "the highest declaration is the floor: {calls}"
    );
    let said = String::from_utf8_lossy(&outcome.stdout);
    assert!(
        said.contains("REPORT: the workspace is checked at 1.85"),
        "a member carried by the floor must be named: {said}"
    );

    let (outcome, calls) = run(json!([package("a", "1.85"), package("b", "1.85.2")]));
    succeeds(&outcome);
    assert!(calls.contains("RUSTUP_TOOLCHAIN=1.85.2 cargo check --workspace --locked"));
    let (outcome, _) = run(json!([package("a", "1.85"), package("b", "1.85.0")]));
    succeeds(&outcome);
    assert!(!String::from_utf8_lossy(&outcome.stdout).contains("carried by that floor"));

    // The build is the point: a workspace that does not compile at its own
    // declared version fails here rather than passing on the declaration.
    let f = Fixture::new();
    fs::write(
        f.root.join("metadata.json"),
        json!({"workspace_members": ["p"], "packages": [package("p", "1.85")]}).to_string(),
    )
    .unwrap();
    f.stub("rustup", "exit 0");
    f.stub(
        "cargo",
        "echo 'error[E0658]: use of unstable library feature' >&2; exit 101",
    );
    assert!(!f.run("ci", "msrv").status.success());
}

#[test]
fn the_real_msrv_build_uses_the_exact_floor_and_rejects_newer_apis() {
    let mut f = Fixture::new();
    f.set("CARGO_NET_OFFLINE", "true");
    fs::write(
        f.root.join("project/Cargo.toml"),
        "[package]\nname = 'floor-fixture'\nversion = '0.1.0'\nedition = '2024'\n\
         rust-version = '1.85'\n",
    )
    .unwrap();
    fs::write(
        f.root.join("project/build.rs"),
        r#"fn main() {
    let compiler = std::env::var_os("RUSTC").unwrap();
    let output = std::process::Command::new(compiler).arg("--version").output().unwrap();
    let version = String::from_utf8(output.stdout).unwrap();
    println!("cargo:warning={version}");
    assert!(version.starts_with("rustc 1.85.0 "));
}
"#,
    )
    .unwrap();
    succeeds(&f.run_body(
        "cd \"$PROJECT\"; cargo metadata --format-version 1 --offline \
         > \"$RUNNER_TEMP/metadata.json\"",
    ));
    let result = f.run("ci", "msrv");
    succeeds(&result);
    assert!(String::from_utf8_lossy(&result.stderr).contains("rustc 1.85.0 "));
    fs::remove_file(f.root.join("project/build.rs")).unwrap();
    fs::write(
        f.root.join("project/src/lib.rs"),
        "pub fn newer_api() -> bool { 4_u32.is_multiple_of(2) }\n",
    )
    .unwrap();
    succeeds(&f.run_body("cd \"$PROJECT\"; cargo check --workspace --locked"));
    let result = f.run("ci", "msrv");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("is_multiple_of"));
}

#[test]
fn ci_rejects_unsafe_paths_and_symlinks() {
    let simple = "working-directory must be a simple relative path";
    let traverse = "working-directory must not traverse or contain option-like components";
    let mut f = Fixture::new();
    for (value, message) in [
        ("../outside", simple),
        ("/", simple),
        ("-bad", simple),
        ("project/../project", traverse),
        ("project;touch hacked", simple),
        (
            "missing",
            "working-directory does not exist inside checkout",
        ),
        ("project\nname", simple),
    ] {
        f.set("DIRECTORY", value);
        refused(&f.run("ci", "validate"), message);
    }
    symlink(f.root.parent().unwrap(), f.root.join("escape")).unwrap();
    f.set("DIRECTORY", "escape");
    refused(
        &f.run("ci", "validate"),
        "working-directory escapes checkout",
    );
    f.set("DIRECTORY", "project");
    fs::remove_file(f.root.join("project/Cargo.lock")).unwrap();
    symlink("/etc/passwd", f.root.join("project/Cargo.lock")).unwrap();
    refused(
        &f.run("ci", "validate"),
        "Cargo.lock must be a file inside checkout",
    );
    assert!(f.calls().is_empty());
}

#[test]
fn ci_validates_toolchain_threshold_and_artifact_identity() {
    // Every input that names or shapes an artifact changes its identity, and
    // every malformed value is refused before the job does any work.
    let mut f = Fixture::new();
    succeeds(&f.run("ci", "validate"));
    assert!(
        fs::read_to_string(f.root.join("environment"))
            .unwrap()
            .lines()
            .any(|line| line == "CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu")
    );
    let first = fs::read_to_string(f.root.join("output")).unwrap();
    let original = f.env.clone();
    fs::create_dir(f.root.join("other")).unwrap();
    for file in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        fs::copy(
            f.root.join("project").join(file),
            f.root.join("other").join(file),
        )
        .unwrap();
    }
    for (key, value) in [
        ("ARTIFACT_KEY", "other"),
        ("GITHUB_RUN_ATTEMPT", "2"),
        ("GITHUB_RUN_ID", "124"),
        ("DIRECTORY", "other"),
    ] {
        fs::write(f.root.join("output"), "").unwrap();
        f.env = original.clone();
        f.set(key, value);
        succeeds(&f.run("ci", "validate"));
        assert_ne!(first, fs::read_to_string(f.root.join("output")).unwrap());
    }
    f.env = original;
    let numeric = "coverage-threshold must be numeric";
    for (value, message) in [
        ("101", "coverage-threshold must be between 0 and 100"),
        ("nan", numeric),
        ("-1", numeric),
        ("80;touch hacked", numeric),
        ("", numeric),
    ] {
        f.set("COVERAGE", value);
        refused(&f.run("ci", "validate"), message);
    }
    f.set("COVERAGE", "80");
    for value in ["--bad", "x\ny", "x;touch hacked"] {
        f.set("ARTIFACT_KEY", value);
        refused(
            &f.run("ci", "validate"),
            "artifact-key must be 1-40 safe characters",
        );
    }
    f.set("ARTIFACT_KEY", "test");
    let pin = "rust-toolchain.toml must pin an exact stable version";
    for (channel, message) in [
        ("stable", pin),
        ("--help", pin),
        ("1.98.0\nextra", pin),
        ("1.84.0", "rust-version must be at least the 1.85.0 MSRV"),
    ] {
        fs::write(
            f.root.join("project/rust-toolchain.toml"),
            format!("[toolchain]\nchannel=\"{channel}\"\n"),
        )
        .unwrap();
        refused(&f.run("ci", "validate"), message);
    }
}

#[test]
fn ci_validates_the_licence_and_unsafe_policies_before_any_work() {
    // The two string policies are closed sets, and the licence policy decides
    // whether a missing deny.toml is acceptable before anything runs.
    let mut f = Fixture::new();
    for value in ["Auto", "strict", "", "auto;touch hacked", "off\nenforce"] {
        f.set("LICENSE_POLICY", value);
        assert!(
            !f.run("ci", "validate").status.success(),
            "license-policy must reject {value:?}"
        );
    }
    // A reusable workflow is adopted by many consumers: the default must never
    // fail a repository that has not committed a licence policy yet, while the
    // explicit `enforce` value must refuse to pass silently without one.
    for policy in ["auto", "off"] {
        f.set("LICENSE_POLICY", policy);
        succeeds(&f.run("ci", "validate"));
    }
    f.set("LICENSE_POLICY", "enforce");
    refused(
        &f.run("ci", "validate"),
        "license-policy=enforce requires a committed deny.toml",
    );
    symlink("/etc/hostname", f.root.join("project/deny.toml")).unwrap();
    refused(&f.run("ci", "validate"), "deny.toml escapes checkout");
    fs::remove_file(f.root.join("project/deny.toml")).unwrap();
    fs::write(f.root.join("project/deny.toml"), "[licenses]\n").unwrap();
    for policy in ["auto", "enforce"] {
        f.set("LICENSE_POLICY", policy);
        succeeds(&f.run("ci", "validate"));
        assert!(
            fs::read_to_string(f.root.join("environment"))
                .unwrap()
                .lines()
                .any(
                    |line| line.starts_with("DENY_CONFIG=") && line.ends_with("/project/deny.toml")
                ),
            "license-policy={policy} must select the consumer deny.toml"
        );
    }
    fs::remove_file(f.root.join("project/deny.toml")).unwrap();
    f.set("LICENSE_POLICY", "auto");
    for value in ["Deny", "forbid", "", "deny;touch hacked"] {
        f.set("UNSAFE_POLICY", value);
        assert!(
            !f.run("ci", "validate").status.success(),
            "unsafe-policy must reject {value:?}"
        );
    }
}

#[test]
fn unused_dependencies_reaches_the_next_step_only_through_exports() {
    for value in ["false", "true"] {
        let mut f = Fixture::new();
        f.set("UNUSED_DEPENDENCIES", value);
        succeeds(&f.run("ci", "validate"));
        // A step-local env block does not survive into the next GitHub step.
        f.env.remove("UNUSED_DEPENDENCIES");
        let exported = fs::read_to_string(f.root.join("environment")).unwrap();
        for line in exported.lines() {
            let (key, value) = line.split_once('=').unwrap();
            f.set(key, value);
        }
        fs::create_dir_all(&f.env["REPORTS"]).unwrap();
        succeeds(&f.run("ci", "unused"));
        let report =
            fs::read_to_string(Path::new(&f.env["REPORTS"]).join("unused-dependencies.txt"))
                .unwrap();
        assert_eq!(report.contains("SKIPPED:"), value == "false");
        assert_eq!(f.trace().contains("cargo machete"), value == "true");
    }
}

#[test]
fn unused_dependencies_is_validated_before_any_export() {
    for value in ["", "False", "0", "true\nINJECT=yes"] {
        let mut f = Fixture::new();
        f.set("UNUSED_DEPENDENCIES", value);
        refused(
            &f.run("ci", "validate"),
            "UNUSED_DEPENDENCIES must be true or false",
        );
        assert!(!f.root.join("environment").exists());
        assert!(!f.root.join("output").exists());
    }
}

#[test]
fn every_boolean_input_is_validated_before_any_export() {
    // validate exports these for the later steps, so a value it did not check
    // would reach GITHUB_ENV as written.
    for name in ["MUTATION_TEST", "SARIF_REPORTS", "DEPENDENCY_AUDIT"] {
        for value in ["", "False", "true\nINJECT=yes"] {
            let mut f = Fixture::new();
            f.set(name, value);
            refused(
                &f.run("ci", "validate"),
                &format!("{name} must be true or false"),
            );
            assert!(!f.root.join("environment").exists(), "{name}={value:?}");
            assert!(!f.root.join("output").exists(), "{name}={value:?}");
        }
    }
}
