//! `ci.yml`: the dependency policy, direct crates.io access and the scanners.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;
use std::path::Path;

#[test]
fn the_dependency_policy_holds_by_default_and_licences_only_with_a_list() {
    // A project with no deny.toml still gets the line every project must hold:
    // approved registries only, no git dependency, no wildcard version. Licences
    // are checked only when a list exists, and the report says which applied.
    let record = r#"[[ "$1" == deny ]] || exit 0"#;
    let mut absent = Fixture::new();
    absent.set("DENY_CONFIG", "");
    absent.stub("cargo", record);
    succeeds(&absent.run("ci", "licenses"));
    let report = fs::read_to_string(absent.root.join("reports/licenses.txt")).unwrap();
    assert!(report.contains("licences NOT APPLIED"), "{report}");
    assert!(
        report.contains("default policy: approved registries only"),
        "{report}"
    );
    let config = fs::read_to_string(absent.root.join("default-deny.toml")).unwrap();
    assert_eq!(
        config,
        "[bans]\nwildcards = \"deny\"\n\n[sources]\nunknown-registry = \"deny\"\n\
         unknown-git = \"deny\"\n\
         allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]\n"
    );
    // RUNNER_TEMP carries a trailing slash in the fixture, so match the file
    // name and the check list rather than the exact path string.
    assert!(
        absent
            .calls()
            .contains("/default-deny.toml check bans sources\n"),
        "calls:\n{}",
        absent.calls()
    );

    // Turned off deliberately: recorded, and nothing runs.
    let mut off = Fixture::new();
    off.set("LICENSE_POLICY", "off");
    off.set("DENY_CONFIG", "");
    off.stub("cargo", record);
    succeeds(&off.run("ci", "licenses"));
    assert!(
        fs::read_to_string(off.root.join("reports/licenses.txt"))
            .unwrap()
            .contains("SKIPPED")
    );
    assert!(off.calls().is_empty());

    // With a policy committed, the tool runs and its failure is the gate.
    let mut enforced = Fixture::new();
    enforced.set("DENY_CONFIG", "/somewhere/deny.toml");
    enforced.stub(
        "cargo",
        r#"[[ "$1" == deny ]] || exit 0
echo "checking"; exit 1"#,
    );
    assert!(
        !enforced.run("ci", "licenses").status.success(),
        "a committed policy that fails must fail the job"
    );
}

#[test]
fn the_organization_allowlist_adds_licences_and_a_committed_policy_wins() {
    // Without a deny.toml, the organization list is the only licence policy;
    // a committed deny.toml wins over it, and a malformed list is refused
    // before cargo-deny runs.
    let record = r#"[[ "$1" == deny ]] || exit 0"#;
    // The organization list adds licences to the default policy.
    let mut org = Fixture::new();
    org.set("DENY_CONFIG", "");
    org.set("LICENSE_ALLOWLIST", "MIT,Apache-2.0");
    org.stub("cargo", record);
    succeeds(&org.run("ci", "licenses"));
    let config = fs::read_to_string(org.root.join("default-deny.toml")).unwrap();
    assert!(
        config.ends_with("\n[licenses]\nallow = [\"MIT\", \"Apache-2.0\"]\n"),
        "{config}"
    );
    assert!(config.contains("unknown-git = \"deny\""));
    assert!(
        org.calls()
            .contains("/default-deny.toml check bans sources licenses\n"),
        "calls:\n{}",
        org.calls()
    );
    assert!(
        fs::read_to_string(org.root.join("reports/licenses.txt"))
            .unwrap()
            .contains("organization licence allowlist: MIT,Apache-2.0")
    );

    // A committed policy wins over the organization list and the default.
    let mut own = Fixture::new();
    own.set("DENY_CONFIG", "/somewhere/deny.toml");
    own.set("LICENSE_ALLOWLIST", "MIT");
    own.stub("cargo", record);
    succeeds(&own.run("ci", "licenses"));
    assert!(
        own.calls().contains(
            "deny --config /somewhere/deny.toml check licenses bans sources advisories\n"
        )
    );
    assert!(!own.root.join("default-deny.toml").exists());

    // A malformed list is refused before cargo-deny runs.
    let mut bad = Fixture::new();
    bad.set("DENY_CONFIG", "");
    bad.set("LICENSE_ALLOWLIST", "MIT; GPL-3.0");
    bad.stub("cargo", record);
    refused(
        &bad.run("ci", "licenses"),
        "LICENSE_ALLOWLIST must be comma-separated SPDX identifiers",
    );
    assert!(bad.calls().is_empty());
}

#[test]
fn ci_configures_direct_crates_io_without_writing_credentials() {
    let mut fixture = Fixture::new();
    fixture.set("CARGO_REGISTRY_TOKEN", "synthetic-unused-token");
    let output = fixture.run("ci", "registry");
    succeeds(&output);
    let environment = fs::read_to_string(fixture.root.join("environment")).unwrap();
    let home = environment
        .lines()
        .find_map(|line| line.strip_prefix("CARGO_HOME="))
        .expect("CARGO_HOME exported");
    assert!(Path::new(home).starts_with(&fixture.root));
    let config = fs::read_to_string(Path::new(home).join("config.toml")).unwrap();
    assert_eq!(config, "[registries.crates-io]\nprotocol = \"sparse\"\n");
    assert!(environment.contains("CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse\n"));
    assert!(
        environment.contains("CARGO_REGISTRIES_CRATES_IO_INDEX=sparse+https://index.crates.io/\n")
    );
    assert!(!Path::new(home).join("credentials.toml").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for written in [environment.as_str(), config.as_str(), stdout.as_ref()] {
        assert!(!written.contains("synthetic-unused-token"));
    }
}

#[test]
fn scanners_propagate_findings_execution_errors_and_missing_tools() {
    let fixture = Fixture::new();
    fixture.stub("git", "tar -cf - --files-from /dev/null");
    for (command, id) in [("cargo", "audit"), ("gitleaks", "secrets")] {
        for status in [1, 2, 127] {
            fixture.stub(command, &format!("exit {status}"));
            assert_eq!(fixture.run("ci", id).status.code(), Some(status));
            assert!(fixture.calls().contains(command));
        }
        fixture.stub(command, "exit 0");
        assert!(
            !fixture.run("ci", id).status.success(),
            "Missing scanner report must fail"
        );
    }
}

#[test]
fn the_toolchain_step_installs_the_exact_pin_with_the_gate_components() {
    let fixture = Fixture::new();
    fixture.stub("rustup", "");
    succeeds(&fixture.run("ci", "tools"));
    let trace = fixture.trace();
    assert!(
        trace.contains(
            "rustup toolchain install 1.98.1 --profile minimal \
             --component rustfmt,clippy,llvm-tools-preview"
        ),
        "{trace}"
    );
    assert!(fixture.root.join("reports").is_dir());
    fixture.stub("rustup", "exit 3");
    assert_eq!(fixture.run("ci", "tools").status.code(), Some(3));
}
