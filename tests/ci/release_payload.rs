//! `ci.yml`: the release build, the payload and its bills of materials, and the
//! example gate that replays the real steps against every fixture.

use crate::harness::{
    Fixture, GATE_STEPS, described, described_step, refused, step, succeeds, tool, tool_rows,
    workflow,
};
use serde_json::{Value, json};
use std::fs;

/// The example gate: `ci.yml`'s own step bodies, read by id, replayed against
/// every owned fixture with the pinned toolbelt. `just check` is its one
/// caller; plain `cargo test` skips it because it needs the toolbelt and builds
/// three projects.
#[test]
#[ignore = "needs the pinned toolbelt; just check runs it"]
fn example_gate_replays_ci_step_bodies_against_every_fixture() {
    for (example, package) in [
        ("binary", "bounded-sum"),
        ("library", "bounded-arithmetic"),
        ("workspace", "workspace-arithmetic"),
    ] {
        let mut f = Fixture::example(example);
        for id in GATE_STEPS {
            let output = f.run("ci", id);
            assert!(
                output.status.success(),
                "{example}/{id} failed\n--- stdout\n{}\n--- stderr\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // Owned fixtures additionally require every mutant to be caught.
        let mutants: Value =
            serde_json::from_str(&fs::read_to_string(f.root.join("reports/mutants.json")).unwrap())
                .unwrap();
        assert!(
            mutants["unviable"] == 0 && mutants["caught"].as_u64().unwrap_or(0) > 0,
            "{example}: every mutant must be caught: {mutants}"
        );
        // Mirrors the reusable workflow's default license-policy=auto: the gate
        // applies when the fixture commits deny.toml and is skipped otherwise.
        // The CI step also checks advisories, which reads the registry index, so
        // its body is not replayed offline.
        let project = f.root.join("project");
        if project.join("deny.toml").is_file() {
            let deny = tool("cargo")
                .args([
                    "deny",
                    "--offline",
                    "--config",
                    "deny.toml",
                    "check",
                    "licenses",
                    "bans",
                    "sources",
                ])
                .current_dir(&project)
                .envs(&f.env)
                .output()
                .unwrap();
            succeeds(&deny);
        } else {
            // The default policy the step generates, run for real against the
            // fixture's lockfile, offline: it must parse and it must pass.
            f.set("DENY_CONFIG", "");
            f.set("CARGO_NET_OFFLINE", "true");
            succeeds(&f.run("ci", "licenses"));
        }
        for (key, value) in [
            ("PACKAGE", package),
            ("DRY_RUN", "true"),
            ("REF", "refs/heads/local-check"),
            ("REGISTRY", ""),
        ] {
            f.set(key, value);
        }
        succeeds(&f.run("publish-crate", "package"));
        if example == "binary" {
            // The one crates.io dependency must reach the payload SBOM, and the
            // release binary must carry its embedded dependency list: this is
            // what the fixture exists to prove. The hierarchical merge nests
            // each member's components under the member, so the whole document
            // is searched.
            let sbom: Value = serde_json::from_str(
                &fs::read_to_string(f.root.join("reports/payload.cdx.json")).unwrap(),
            )
            .unwrap();
            assert!(
                serde_json::to_string(&sbom)
                    .unwrap()
                    .contains("\"name\":\"anyhow\""),
                "the payload SBOM must name the fixture dependency"
            );
            let hardening = fs::read_to_string(f.root.join("reports/hardening.txt")).unwrap();
            assert!(
                hardening
                    .contains("bounded-sum reproducible pie relro bind-now noexec-stack auditable"),
                "{hardening}"
            );
        }
        println!(
            "PASS: {example} formatting, Clippy, tests, coverage, build, package and SBOM staging"
        );
    }
}

#[test]
fn the_release_build_must_be_reproducible_and_auditable_or_fail() {
    // Every guarantee this step makes is a refusal, so each one is provoked. A
    // reproducibility check that cannot fail proves nothing about the build.
    let prepare = |f: &Fixture| {
        let target = f.root.join("target");
        fs::create_dir_all(target.join("release")).unwrap();
        fs::write(target.join("release/app"), "first").unwrap();
        fs::write(
            f.root.join("build.jsonl"),
            json!({"reason": "compiler-artifact", "executable": target.join("release/app"),
                   "target": {"name": "app"}})
            .to_string(),
        )
        .unwrap();
        f.root.join("target")
    };

    // A rebuild producing different bytes must fail: that is the whole point.
    let mut differs = Fixture::new();
    let target = prepare(&differs);
    differs.set("CARGO_TARGET_DIR", &target.display().to_string());
    differs.stub(
        "cargo",
        r#"mkdir -p "$RUNNER_TEMP/rust-target-verify/release"
printf 'second' > "$RUNNER_TEMP/rust-target-verify/release/app""#,
    );
    refused(
        &differs.run("ci", "hardening"),
        "is not reproducible across build directories",
    );

    // Identical bytes, but no embedded dependency list: installing cargo-auditable
    // and forgetting to build through it produces exactly this, silently.
    let mut plain = Fixture::new();
    let target = prepare(&plain);
    plain.set("CARGO_TARGET_DIR", &target.display().to_string());
    plain.stub(
        "cargo",
        r#"mkdir -p "$RUNNER_TEMP/rust-target-verify/release"
printf 'first' > "$RUNNER_TEMP/rust-target-verify/release/app""#,
    );
    plain.stub(
        "readelf",
        r#"case "$1" in
  -h) echo '  Type:                              DYN (Position-Independent Executable file)' ;;
  -l) echo '  GNU_RELRO      0x000000'; echo '  GNU_STACK      0x000000'; echo '      RW ' ;;
  -d) echo ' 0x000000018 (BIND_NOW)' ;;
  -S) echo '  [ 1] .text             PROGBITS' ;;
esac"#,
    );
    // Installing the tool and forgetting to build through it produces a
    // normal binary and no error, so the section itself is checked and named.
    refused(
        &plain.run("ci", "hardening"),
        "carries no embedded dependency list; build through cargo auditable",
    );
}

#[test]
fn release_payload_carries_both_sbom_formats_and_auditable_binaries() {
    // Consumer tooling is split between the two SBOM formats. Shipping one
    // leaves half of them with no bill of materials they can read. The
    // staging step declares the native SPDX generator; a conversion from the
    // merged CycloneDX document would lose nested components and the whole
    // relationship graph, so no step declares `cyclonedx convert`, and the
    // valid run in
    // sbom_staging_rejects_malformed_data_and_emits_verifiable_payload reads
    // the exact command from the trace.
    let steps = described();
    let stage = described_step(&steps, "rust-gate stage").unwrap();
    assert!(stage.tools.iter().any(|tool| tool == "cargo sbom"));
    assert!(stage.reports.iter().any(|report| report == "*.spdx.json"));
    // Both release builds go through cargo-auditable. Only one would make the
    // two binaries differ and fail the reproducibility check for the wrong
    // reason, and a plain `cargo build` is a binary with no dependency list.
    for id in ["build", "hardening"] {
        let step = described_step(&steps, &format!("rust-gate {id}")).unwrap();
        assert!(
            step.tools.iter().any(|tool| tool == "cargo auditable"),
            "{id} must build through cargo-auditable"
        );
    }
    assert!(
        steps
            .iter()
            .filter(|step| step.workflow == "ci")
            .all(|step| step.tools.iter().all(|tool| tool != "cargo build")),
        "no ci.yml step may build without the auditable wrapper"
    );
    // The wrapper is pinned and installed like every other tool.
    let installed = workflow("ci")["jobs"]["checks"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(tool_rows)
        .any(|row| {
            row.asset
                .contains("cargo-auditable/releases/download/v0.7.6")
        });
    assert!(
        installed,
        "cargo-auditable must be installed at its pinned release"
    );
    // `cargo test --release` rebuilds the bin targets its integration tests use
    // without the wrapper. Run after the auditable build, it silently replaced
    // the shipped binary with a plain one, and the hardening step then failed
    // every consumer that has a binary and an integration test. The order is
    // read from the trace of a build against a cargo that records its calls.
    assert_eq!(step("ci", "build").trim(), "rust-gate build");
    let f = Fixture::new();
    f.stub("cargo", "");
    succeeds(&f.run("ci", "build"));
    let trace = f.trace();
    let tests = trace
        .find("cargo test --workspace --release --locked")
        .expect("release tests must run");
    let auditable = trace
        .find("cargo auditable build")
        .expect("the auditable build must run");
    assert!(
        tests < auditable,
        "release tests must run before the auditable build whose bytes ship"
    );
    assert_eq!(
        trace.matches("cargo auditable build").count(),
        1,
        "the build step builds once through the wrapper"
    );
}

#[test]
fn packaging_uses_a_cargo_that_can_package_a_workspace() {
    // Before Cargo 1.90, `cargo package --workspace` looked for a member's
    // sibling on crates.io, so a workspace whose members depend on each other
    // could not package on 1.85 to 1.89. The release build keeps the selected
    // compiler; only the packaging runs on Cargo 1.90.
    for (selected, packager) in [
        ("1.85.0", Some("1.90.0")),
        ("1.89.0", Some("1.90.0")),
        ("1.90.0", None),
        ("1.98.1", None),
    ] {
        let mut f = Fixture::new();
        f.set("RUSTUP_TOOLCHAIN", selected);
        f.stub("cargo", "");
        f.stub("rustup", "");
        succeeds(&f.run("ci", "build"));
        let trace = f.trace();
        let package = trace
            .lines()
            .find(|line| line.ends_with("cargo package --workspace --locked"))
            .expect("the workspace must be packaged");
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
fn sbom_staging_rejects_malformed_data_and_emits_verifiable_payload() {
    for valid in [false, true] {
        let mut f = Fixture::new();
        let target = f.root.join("target");
        fs::create_dir(&target).unwrap();
        f.set("CARGO_TARGET_DIR", &target.display().to_string());
        f.set("REVISION", &"a".repeat(40));
        let metadata = json!({"workspace_members": ["fixture"], "packages": [{
            "id": "fixture", "name": "fixture", "manifest_path": f.root.join("project/Cargo.toml")
        }]});
        fs::write(f.root.join("metadata.json"), metadata.to_string()).unwrap();
        fs::write(f.root.join("build.jsonl"), "").unwrap();
        let bom = json!({"bomFormat": "CycloneDX", "specVersion": "1.5", "version": 1,
            "metadata": {"component": {"name": "fixture", "type": "library"}},
            "components": if valid { json!([]) } else { json!([1]) }});
        fs::write(f.root.join("project/fixture.cdx.json"), bom.to_string()).unwrap();
        let output = f.run("ci", "stage");
        if valid {
            succeeds(&output);
            // Both formats come from the one member list: the SPDX document
            // is generated natively per member, never converted, and every
            // member with a CycloneDX document has an SPDX one beside it.
            let trace = f.trace();
            assert!(
                trace.contains("cargo sbom --cargo-package fixture --output-format spdx_json_2_3"),
                "SPDX must be generated natively per member"
            );
            assert!(
                !trace.contains("cyclonedx convert"),
                "SPDX must not come from a lossy CycloneDX conversion"
            );
            let payload = f.root.join("rust-release");
            for entry in fs::read_dir(&payload).unwrap() {
                let name = entry.unwrap().file_name().to_string_lossy().into_owned();
                if let Some(member) = name.strip_suffix(".cdx.json") {
                    let spdx = fs::read_to_string(payload.join(format!("{member}.spdx.json")))
                        .unwrap_or_else(|_| panic!("{member} has no SPDX document"));
                    assert!(
                        spdx.contains("SPDX-2.3"),
                        "{member}: not an SPDX 2.3 document"
                    );
                }
            }
            succeeds(&f.run_body("rust-gate verify-payload"));
            f.set("REQUIRE_BINARIES", "true");
            assert!(!f.run_body("rust-gate verify-payload").status.success());
        } else {
            refused(
                &output,
                "Invalid CycloneDX JSON envelope or component metadata",
            );
            assert!(!f.root.join("rust-release/payload.tar.gz").exists());
        }
    }
}
