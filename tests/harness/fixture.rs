//! The fixture: one temporary checkout, one environment table, and a step run
//! against stand-ins the way a hosted runner would run it.

use super::gate_declarations::gate_bin;
use super::repository::{root, toolbelt_path};
use super::workflow_yaml::step;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct Fixture {
    pub(crate) root: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rust-workflows-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("project")).unwrap();
        fs::create_dir(path.join("bin")).unwrap();
        // Where every step writes its report; the tools step creates it in a run.
        fs::create_dir(path.join("reports")).unwrap();
        fs::write(
            path.join("project/Cargo.toml"),
            "[package]\nname=\"fixture\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        // A real Cargo package always has a target. Without one, `cargo metadata`
        // and any SBOM generator refuse the manifest, and the fixture would be
        // exercising a shape no consumer can actually have.
        fs::create_dir(path.join("project/src")).unwrap();
        fs::write(path.join("project/src/lib.rs"), "//! Fixture crate.\n").unwrap();
        fs::write(path.join("project/Cargo.lock"), "version = 4\n").unwrap();
        fs::write(
            path.join("project/rust-toolchain.toml"),
            "[toolchain]\nchannel=\"1.98.1\"\n",
        )
        .unwrap();
        let mut fixture = Self {
            root: path,
            env: BTreeMap::new(),
        };
        for (key, value) in [
            ("DIRECTORY", "project"),
            ("RUSTUP_TOOLCHAIN", "1.98.1"),
            // What `install-tools` and `verify-payload` read.
            ("TOOLS", ""),
            ("REQUIRE_BINARIES", "false"),
            ("GITHUB_BASE_REF", ""),
            ("LICENSE_ALLOWLIST", ""),
            ("ARTIFACT_KEY", "test"),
            ("COVERAGE", "80"),
            ("LICENSE_POLICY", "auto"),
            ("MUTATION_TEST", "false"),
            ("SARIF_REPORTS", "false"),
            ("UNUSED_DEPENDENCIES", "false"),
            ("UNSAFE_POLICY", "allow"),
            ("DEPENDENCY_AUDIT", "false"),
            ("CLIPPY_LEVEL", "default"),
            ("GITHUB_RUN_ID", "123"),
            ("GITHUB_RUN_ATTEMPT", "1"),
            // GitHub always sets this; a step reading it under `set -u` is fine in
            // production but needs it here, or the fixture fails for a reason the
            // workflow does not have.
            ("GITHUB_REF_TYPE", "branch"),
            ("GITHUB_REPOSITORY", "Orchestration-Maestro/rust-workflows"),
            ("GITHUB_SERVER_URL", "https://github.com"),
            ("DRY_RUN", "true"),
            ("EVENT", "pull_request"),
            ("REF", "refs/pull/1/merge"),
            ("PROTECTED", "false"),
            ("PACKAGE", "fixture"),
            ("TOKEN", ""),
        ] {
            fixture.set(key, value);
        }
        fixture.set("GITHUB_SHA", &"a".repeat(40));
        for (key, file) in [
            ("GITHUB_WORKSPACE", ""),
            ("RUNNER_TEMP", ""),
            ("GITHUB_OUTPUT", "output"),
            ("GITHUB_ENV", "environment"),
            ("GITHUB_STEP_SUMMARY", "summary"),
            ("GITHUB_PATH", "path"),
            ("PROJECT", "project"),
            ("REPORTS", "reports"),
            ("CALLS", "calls"),
            ("RUST_GATE_TRACE", "trace"),
        ] {
            fixture.set(key, &fixture.root.join(file).display().to_string());
        }
        fixture.set(
            "PATH",
            &format!(
                "{}:{}:{}",
                fixture.root.join("bin").display(),
                gate_bin().display(),
                toolbelt_path()
            ),
        );
        fixture
    }

    /// The real adapter: the same environment table as `new()`, but the project
    /// is a copy of one owned fixture, no command is stubbed, and the owned
    /// floors apply: the owned coverage floor is 90 rather than the public 80,
    /// and every optional gate that can run offline is on.
    pub(crate) fn example(name: &str) -> Self {
        let mut fixture = Self::new();
        let project = fixture.root.join("project");
        fs::remove_dir_all(&project).unwrap();
        copy_tree(&root().join("examples").join(name), &project);
        for (key, value) in [
            ("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu"),
            ("COVERAGE", "90"),
            ("UNSAFE_POLICY", "deny"),
            ("MUTATION_TEST", "true"),
            ("UNUSED_DEPENDENCIES", "true"),
        ] {
            fixture.set(key, value);
        }
        fixture.set(
            "CARGO_TARGET_DIR",
            &fixture.root.join("target").display().to_string(),
        );
        fixture
    }

    pub(crate) fn set(&mut self, key: &str, value: &str) {
        self.env.insert(key.to_owned(), value.to_owned());
    }

    /// Run one step body with exactly the fixture's environment table on top of
    /// a bare login environment, the way a hosted runner does. The test process
    /// itself is a child of `cargo test`, which injects `CARGO_PKG_*` and other
    /// variables of its own; leaked into a body, `CARGO_PKG_NAME` alone makes
    /// cargo-machete mistake its subcommand name for a path.
    pub(crate) fn run(&self, name: &str, id: &str) -> Output {
        self.run_body(&step(name, id))
    }

    pub(crate) fn run_body(&self, body: &str) -> Output {
        let mut command = Command::new("bash");
        command.env_clear();
        for key in [
            "HOME",
            "USER",
            "LANG",
            "TERM",
            "TMPDIR",
            "RUSTUP_HOME",
            "CARGO_HOME",
        ] {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
        command
            .args(["-c", body])
            .current_dir(&self.root)
            .envs(&self.env)
            .output()
            .unwrap()
    }

    pub(crate) fn stub(&self, command: &str, body: &str) {
        let path = self.root.join("bin").join(command);
        fs::write(
            &path,
            format!(
                "#!/bin/bash\nset -euo pipefail\n\
                 printf '%s\\n' '{command}' \"$*\" >> \"$CALLS\"\n{body}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Every command the gate ran in this fixture, one line each, environment
    /// first: the plan the tests read instead of the gate's source.
    pub(crate) fn trace(&self) -> String {
        fs::read_to_string(self.root.join("trace")).unwrap_or_default()
    }

    pub(crate) fn calls(&self) -> String {
        fs::read_to_string(self.root.join("calls")).unwrap_or_default()
    }

    pub(crate) fn trusted(&mut self) {
        for (key, value) in [
            ("DRY_RUN", "false"),
            ("EVENT", "push"),
            ("REF", "refs/tags/v0.1.0"),
            ("PROTECTED", "true"),
            (
                "ENVIRONMENT_JSON",
                r#"{"protection_rules":[{"type":"required_reviewers",
                    "reviewers":[{"type":"User","reviewer":{"id":42}}]}],
                    "deployment_branch_policy":{"protected_branches":false,
                    "custom_branch_policies":true}}"#,
            ),
            (
                "POLICIES_JSON",
                r#"{"total_count":1,"branch_policies":[{"type":"tag","name":"v*"}]}"#,
            ),
            ("RELEASE_JSON", r#"{"tagName":"v0.1.0","assets":[]}"#),
            ("UPLOAD_STATUS", "0"),
            ("VIEW_STATUS", "0"),
            ("POLICIES_STATUS", "0"),
            ("COMMIT_STATUS", "0"),
        ] {
            self.set(key, value);
        }
        self.set("REVISION", &"a".repeat(40));
        self.set(
            "COMMIT_JSON",
            &format!("{{\"sha\":\"{}\"}}", "a".repeat(40)),
        );
        self.stub(
            "gh",
            r#"case "$1 $2" in
  'api '*'/environments/release') printf '%s\n' "$ENVIRONMENT_JSON" ;;
  'api '*'/deployment-branch-policies?per_page=100')
    printf '%s\n' "$POLICIES_JSON"; exit "$POLICIES_STATUS" ;;
  'api '*'/commits/refs%2Ftags%2Fv0.1.0') printf '%s\n' "$COMMIT_JSON"; exit "$COMMIT_STATUS" ;;
  'release view') printf '%s\n' "$RELEASE_JSON"; exit "$VIEW_STATUS" ;;
  'release upload') exit "$UPLOAD_STATUS" ;;
  *) exit 88 ;;
esac"#,
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Copy a fixture into the temporary root, leaving local build output and
/// generated SBOMs behind.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "target" || name.ends_with(".cdx.json") || name.contains(":Zone.Identifier") {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A step body that pipes through `tee` reports on stdout, so a failure shows
/// both streams.
/// The step refused, and said why with `message` on stderr.
pub(crate) fn refused(output: &Output, message: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "the step did not refuse: {stderr}"
    );
    assert!(
        stderr.contains(message),
        "expected {message:?} in: {stderr}"
    );
}

pub(crate) fn succeeds(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(crate) fn checksums(release: &Path) {
    let output = Command::new("sha256sum")
        .args(["payload.tar.gz", "provenance.json"])
        .current_dir(release)
        .output()
        .unwrap();
    succeeds(&output);
    fs::write(release.join("SHA256SUMS"), output.stdout).unwrap();
}
