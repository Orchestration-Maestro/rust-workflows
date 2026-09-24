//! `attest-binaries.yml`: signing only what the job verified itself.

use crate::harness::{
    Fixture, checksums, described, described_step, refused, root, succeeds, workflow,
};
use serde_json::json;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

#[test]
fn attestation_signs_only_bytes_it_verified_itself() {
    // This job holds `attestations: write` and `id-token: write`: the most
    // privileged code here. It must never sign an artifact it has not checked,
    // and the check has to happen in this job rather than be inherited from an
    // earlier one whose output could have been replaced in between.
    let revision = "a".repeat(40);
    let payload = b"release payload bytes";
    // The real digest of `payload`: the job computes it from the bytes, so a
    // made-up constant would test nothing.
    let digest = "6050124dee4359af2b8698c987f3a9a33bfd29f2869822273f38bdc2e894929d";

    let prepare = |fixture: &mut Fixture| -> PathBuf {
        let root = fixture.root.clone();
        let dir = root.join("rust-release");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("payload.tar.gz"), payload).unwrap();
        fs::write(
            dir.join("provenance.json"),
            json!({"revision": revision, "target": "x86_64-unknown-linux-gnu",
                   "binaries": ["fixture"]})
            .to_string(),
        )
        .unwrap();
        checksums(&dir);
        // The SBOM the workflow extracts from the verified tarball.
        fs::create_dir_all(root.join("sbom-source")).unwrap();
        fs::write(
            root.join("sbom-source/payload.cdx.json"),
            json!({"bomFormat": "CycloneDX", "specVersion": "1.5",
                   "metadata": {"component": {"name": "rust-release-payload"}}})
            .to_string(),
        )
        .unwrap();
        // tar hands back the SBOM the fixture prepared, standing in for
        // extraction from a tarball whose checksum was just confirmed.
        fixture.stub(
            "tar",
            r#"cp "${GITHUB_WORKSPACE}/sbom-source/payload.cdx.json" \
"${RUNNER_TEMP}/payload.cdx.json""#,
        );
        fixture.set("GITHUB_WORKSPACE", &root.display().to_string());
        fixture.set("REVISION", &revision);
        dir
    };

    let mut good = Fixture::new();
    prepare(&mut good);
    succeeds(&good.run("attest-binaries", "verify"));
    // Every digest in the manifest is checked, and strictly: a manifest line
    // that names no file is a failure, not a line to skip.
    assert!(
        good.trace().contains("sha256sum --check --strict"),
        "verification must check every digest strictly"
    );
    // The digest handed to the signer is the one this job computed, not one
    // copied out of the manifest it was given.
    let output = fs::read_to_string(good.root.join("output")).unwrap();
    assert!(output.contains(&format!("payload-digest={digest}")));
    succeeds(&good.run("attest-binaries", "payload-sbom"));

    // Each way the payload can be wrong must stop the job before it signs.
    let selector = "Invalid release checksum selector";
    for (damage, message) in [
        ("unusable digest", selector),
        ("manifest names another file", selector),
        (
            "payload is a symlink",
            "Duplicate checksum selector or symlink",
        ),
    ] {
        let mut fixture = Fixture::new();
        let dir = prepare(&mut fixture);
        match damage {
            "unusable digest" => {
                fs::write(dir.join("SHA256SUMS"), "notadigest  payload.tar.gz\n").unwrap();
            }
            "manifest names another file" => {
                fs::write(dir.join("SHA256SUMS"), format!("{digest}  other.tar.gz\n")).unwrap();
            }
            _ => {
                // A symlink would have the job hash, and then sign, whatever it
                // points at rather than the release it was handed.
                fs::remove_file(dir.join("payload.tar.gz")).unwrap();
                symlink("/etc/hostname", dir.join("payload.tar.gz")).unwrap();
            }
        }
        refused(&fixture.run("attest-binaries", "verify"), message);
    }

    // A revision that is not the checked-out one would bind provenance to a
    // commit the bytes do not come from.
    let sha = "revision must be an immutable commit SHA";
    for (wrong, message) in [
        (String::from("main"), sha),
        ("b".repeat(40), "Release revision does not match source"),
        ("a".repeat(39), sha),
    ] {
        let mut fixture = Fixture::new();
        prepare(&mut fixture);
        fixture.set("REVISION", &wrong);
        refused(&fixture.run("attest-binaries", "verify"), message);
    }

    // An SBOM that is not the payload's own must not be attested as if it were.
    let mut foreign = Fixture::new();
    prepare(&mut foreign);
    fs::write(
        foreign.root.join("sbom-source/payload.cdx.json"),
        json!({"bomFormat": "CycloneDX", "metadata": {"component": {"name": "something-else"}}})
            .to_string(),
    )
    .unwrap();
    succeeds(&foreign.run("attest-binaries", "verify"));
    refused(
        &foreign.run("attest-binaries", "payload-sbom"),
        "Payload SBOM is missing or unusable",
    );
}

#[test]
fn a_skipped_attestation_is_reported_and_never_reads_as_a_signed_release() {
    // Only a proven platform skip may leave an unsigned run green. Signing
    // and independent verification must all succeed before attested=true.
    let digest = "e".repeat(64);
    for (run, attest, sbom, verify, expected, accepted) in [
        ("true", "success", "success", "success", "true", true),
        ("false", "skipped", "skipped", "skipped", "false", true),
        ("true", "failure", "skipped", "skipped", "false", false),
        ("true", "success", "failure", "skipped", "false", false),
        ("true", "success", "success", "failure", "false", false),
        ("true", "success", "success", "cancelled", "false", false),
        ("true", "skipped", "skipped", "skipped", "false", false),
        ("false", "failure", "skipped", "skipped", "false", false),
        ("", "skipped", "skipped", "skipped", "false", false),
    ] {
        let mut fixture = Fixture::new();
        fixture.set("RUN", run);
        fixture.set("ATTEST", attest);
        fixture.set("SBOM", sbom);
        fixture.set("VERIFY", verify);
        fixture.set("DIGEST", &digest);
        let result = fixture.run("attest-binaries", "outcome");
        if accepted {
            succeeds(&result);
        } else {
            refused(
                &result,
                "Attestation signing or verification did not succeed",
            );
        }
        let output = fs::read_to_string(fixture.root.join("output")).unwrap();
        assert!(
            output.contains(&format!("attested={expected}")),
            "attest={attest} sbom={sbom} must report attested={expected}"
        );

        let summary = fs::read_to_string(fixture.root.join("summary")).unwrap();
        if expected == "true" {
            assert!(
                summary.contains(&digest),
                "a signed release must name its subject"
            );
        } else {
            // The summary has to say the release is unsigned in words a reader
            // cannot mistake for success, and must not quote a digest as though
            // something had vouched for it.
            assert!(summary.contains("Not attested."));
            assert!(summary.contains("carries no cryptographic provenance"));
            assert!(!summary.contains(&digest));
        }
    }
}

#[test]
fn attestation_tolerates_an_unavailable_platform_without_hiding_the_gap() {
    let data = workflow("attest-binaries");
    let inputs = &data["on"]["workflow_call"]["inputs"];
    // Default `skip` tolerates proven platform unavailability, not a failed
    // attempt to sign or an unconfirmed Cloud entitlement.
    assert_eq!(inputs["on-unavailable"]["default"], "skip");
    // Success is not evidence of provenance once skipping is allowed, so a
    // caller that requires an attestation must be able to gate on the fact.
    let outputs = &data["on"]["workflow_call"]["outputs"];
    assert!(outputs["attested"]["value"].as_str().is_some());
    let text = fs::read_to_string(root().join(".github/workflows/attest-binaries.yml")).unwrap();
    assert!(
        !text.contains("continue-on-error:"),
        "signing errors must block"
    );
    let steps = data["jobs"]["attest"]["steps"].as_array().unwrap();
    let validate = steps.iter().find(|step| step["id"] == "validate").unwrap();
    assert_eq!(validate["env"]["GH_TOKEN"], "${{ github.token }}");
    assert_eq!(
        validate["env"]["GH_ENTERPRISE_TOKEN"],
        "${{ github.token }}"
    );
    for id in [
        "verify",
        "payload-sbom",
        "attest",
        "sbom",
        "verify-recorded",
    ] {
        let step = steps.iter().find(|step| step["id"] == id).unwrap();
        assert!(
            step["if"]
                .as_str()
                .unwrap()
                .contains("steps.validate.outputs.run == 'true'")
        );
    }
    let outcome = steps.iter().find(|step| step["id"] == "outcome").unwrap();
    assert_eq!(outcome["if"], "${{ always() }}");
    assert_eq!(outcome["env"]["RUN"], "${{ steps.validate.outputs.run }}");
    assert_eq!(
        outcome["env"]["VERIFY"],
        "${{ steps.verify-recorded.outcome }}"
    );
    // A skipped attestation has to be stated in the run summary; a silent gap
    // would read exactly like a signed release. The outcome step is run and
    // its summary read in
    // a_skipped_attestation_is_reported_and_never_reads_as_a_signed_release.
}

#[test]
fn provenance_attestation_is_isolated_and_reverifies_the_payload() {
    // Attestation lives in its own callable workflow. GitHub validates every
    // job's scopes at startup even when `if:` skips the job, so declaring
    // `attestations: write` inside publication would force that grant on every
    // caller, and artifact attestations additionally require a private
    // repository owned by an enterprise. Only callers that opt in carry it.
    for name in ["publish-binaries", "publish-crate"] {
        let text =
            fs::read_to_string(root().join(format!(".github/workflows/{name}.yml"))).unwrap();
        assert!(
            !text.contains("attestations:"),
            "{name} must not impose attestation scope on every caller"
        );
    }
    let data = workflow("attest-binaries");
    let attest = &data["jobs"]["attest"];
    assert_eq!(attest["permissions"]["id-token"], "write");
    assert_eq!(attest["permissions"]["attestations"], "write");
    assert_eq!(attest["permissions"]["contents"], "read");
    for input in ["artifact-id", "revision"] {
        assert_eq!(
            data["on"]["workflow_call"]["inputs"][input]["required"], true,
            "{input} must be required so nothing is attested implicitly"
        );
    }
    // The attestation must describe bytes this job checked itself, not bytes a
    // previous job reported as good.
    let steps = attest["steps"].as_array().unwrap();
    let verify = steps
        .iter()
        .position(|step| step["id"] == "verify")
        .expect("attestation must re-verify the payload checksum");
    assert_eq!(
        steps[verify]["run"].as_str().map(str::trim),
        Some("rust-gate verify-payload"),
        "verification is the gate's own verify-payload command"
    );
    // What the verification declares: the digest tool it runs and the two
    // revisions it compares. The strict check itself is read from the trace
    // of a verified payload in attestation_signs_only_bytes_it_verified_itself.
    let registry = described();
    let verify_step = described_step(&registry, "rust-gate verify-payload")
        .expect("verify-payload is a registered command");
    assert!(
        verify_step.tools.iter().any(|tool| tool == "sha256sum"),
        "the verification step must check the payload digest"
    );
    let sign = steps
        .iter()
        .position(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/attest-build-provenance@"))
        })
        .expect("attestation step must be present");
    assert!(verify < sign, "verification must precede attestation");
    assert!(
        ["GITHUB_SHA", "REVISION"]
            .iter()
            .all(|name| verify_step.inputs.iter().any(|input| input == name)),
        "the attested revision must match the checked-out source"
    );
}

#[test]
fn only_confirmed_server_unavailability_can_skip_attestation() {
    for (policy, metadata, expected) in [
        ("skip", "{}", "true"),
        ("fail", "{}", "true"),
        ("skip", r#"{"installed_version":"3.20.0"}"#, "false"),
        ("skip", r#"{"installed_version":null}"#, "true"),
        ("skip", r#"{"installed_version":""}"#, "true"),
    ] {
        let mut fixture = Fixture::new();
        fixture.set("ON_UNAVAILABLE", policy);
        // Execute the actual JSON selector, with the local pinned jaq in place
        // of gh's embedded jq, against the documented metadata response shape.
        fixture.stub("gh", &format!("printf '%s' '{metadata}' | jaq -r \"$4\""));
        succeeds(&fixture.run_body("rust-gate attest-binaries validate"));
        assert_eq!(
            fs::read_to_string(fixture.root.join("output")).unwrap(),
            format!("run={expected}\n")
        );
        assert!(fixture.calls().contains("api meta --jq"));
    }
    for (response, message) in [
        (
            "true",
            "Artifact attestations are unavailable on GitHub Enterprise Server",
        ),
        (
            "unknown",
            "GitHub platform metadata did not identify the server type",
        ),
        (
            "",
            "GitHub platform metadata did not identify the server type",
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.set("ON_UNAVAILABLE", "fail");
        fixture.stub("gh", &format!("printf '%s' '{response}'"));
        refused(
            &fixture.run_body("rust-gate attest-binaries validate"),
            message,
        );
        assert!(!fixture.root.join("output").exists());
    }
    // Authentication, API, network and unknown failures cannot become skips.
    for policy in ["skip", "fail"] {
        for message in ["HTTP 403", "HTTP 404", "OIDC failure", "network failure"] {
            let mut fixture = Fixture::new();
            fixture.set("ON_UNAVAILABLE", policy);
            fixture.stub("gh", &format!("printf '%s\\n' '{message}' >&2; exit 7"));
            assert_eq!(
                fixture
                    .run_body("rust-gate attest-binaries validate")
                    .status
                    .code(),
                Some(7)
            );
            assert!(!fixture.root.join("output").exists());
        }
    }
}

#[test]
fn the_unavailability_policy_is_skip_or_fail() {
    let mut fixture = Fixture::new();
    fixture.stub("gh", "printf 'false\\n'");
    for value in ["skip", "fail"] {
        fixture.set("ON_UNAVAILABLE", value);
        succeeds(&fixture.run_body("rust-gate attest-binaries validate"));
    }
    for value in ["", "warn", "Skip"] {
        fixture.set("ON_UNAVAILABLE", value);
        refused(
            &fixture.run_body("rust-gate attest-binaries validate"),
            "on-unavailable must be skip or fail",
        );
    }
}
