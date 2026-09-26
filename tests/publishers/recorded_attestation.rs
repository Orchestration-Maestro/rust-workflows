//! attest-binaries.yml, the recorded attestation: verified through `gh` for
//! the signing workflow of the repository this run found it in, whatever its
//! name, and refused unless it covers the payload digest this job computed.

use crate::harness::{Fixture, refused, succeeds, workflow};

/// What `gh attestation verify --format json` answers for one attestation
/// over a subject digest.
const ATTESTATION: &str = concat!(
    r#"printf '%s' '[{"verificationResult":{"statement":"#,
    r#"{"subject":[{"digest":{"sha256":"DIGEST"}}]}}}]'"#
);

/// A fixture whose `gh` answers with one attestation over `digest`, asked to
/// verify the payload whose digest is sixty-four `a`.
fn attested(digest: &str) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.set("GITHUB_REPOSITORY", "example/consumer");
    fixture.set(
        "WORKFLOW_REPOSITORY",
        "Orchestration-Maestro/rust-workflows",
    );
    fixture.set("EXPECTED_DIGEST", &"a".repeat(64));
    fixture.stub("gh", &ATTESTATION.replace("DIGEST", digest));
    fixture
}

#[test]
fn the_recorded_attestation_is_verified_for_the_signing_workflow_of_this_repository() {
    let fixture = attested(&"a".repeat(64));
    succeeds(&fixture.run_body("rust-gate attest-binaries verify-recorded"));
    let trace = fixture.trace();
    assert!(
        trace.contains(
            "gh attestation verify rust-release/payload.tar.gz --repo example/consumer \
             --signer-workflow \
              Orchestration-Maestro/rust-workflows/.github/workflows/attest-binaries.yml \
             --format json"
        ),
        "{trace}"
    );
}

#[test]
fn a_renamed_repository_is_the_signer_its_run_names() {
    let mut fixture = attested(&"a".repeat(64));
    fixture.set(
        "WORKFLOW_REPOSITORY",
        "Orchestration-Maestro/maestro-rust-workflows",
    );
    succeeds(&fixture.run_body("rust-gate attest-binaries verify-recorded"));
    let trace = fixture.trace();
    assert!(
        trace.contains(
            "--signer-workflow \
             Orchestration-Maestro/maestro-rust-workflows/.github/workflows/attest-binaries.yml \
             --format json"
        ),
        "{trace}"
    );
    let data = workflow("attest-binaries");
    let steps = data["jobs"]["attest"]["steps"].as_array().unwrap();
    let step = steps
        .iter()
        .find(|step| step["id"] == "verify-recorded")
        .unwrap();
    assert_eq!(
        step["env"]["WORKFLOW_REPOSITORY"],
        "${{ job.workflow_repository }}"
    );
}

#[test]
fn both_attestation_calls_use_a_normalized_host_and_scoped_tokens() {
    for host in ["github.com", "tenant.ghe.com", "ghe.example.invalid:8443"] {
        let mut fixture = attested(&"a".repeat(64));
        let server = format!("https://{host}");
        fixture.set("GITHUB_SERVER_URL", &server);
        fixture.set("GH_HOST", &server);
        fixture.set("EXPECTED_HOST", host);
        fixture.set("GH_TOKEN", "fixture-token-not-a-secret");
        fixture.set("GH_ENTERPRISE_TOKEN", "fixture-token-not-a-secret");
        fixture.set("ON_UNAVAILABLE", "skip");
        fixture.stub(
            "gh",
            &format!(
                r#"[[ "$GH_HOST" == "$EXPECTED_HOST" ]]
[[ "$GH_TOKEN" == fixture-token-not-a-secret ]]
[[ "$GH_ENTERPRISE_TOKEN" == fixture-token-not-a-secret ]]
if [[ "$1" == api ]]; then printf 'false\n'; else {}; fi"#,
                ATTESTATION.replace("DIGEST", &"a".repeat(64))
            ),
        );
        succeeds(&fixture.run_body("rust-gate attest-binaries validate"));
        succeeds(&fixture.run_body("rust-gate attest-binaries verify-recorded"));
        assert!(!fixture.trace().contains("fixture-token-not-a-secret"));
    }
    let data = workflow("attest-binaries");
    let steps = data["jobs"]["attest"]["steps"].as_array().unwrap();
    for id in ["validate", "verify-recorded"] {
        let step = steps.iter().find(|step| step["id"] == id).unwrap();
        for name in ["GH_TOKEN", "GH_ENTERPRISE_TOKEN"] {
            assert_eq!(step["env"][name], "${{ github.token }}");
        }
        assert!(
            step["env"].get("GH_HOST").is_none(),
            "the gate normalizes the host"
        );
    }
    for server in [
        "http://github.com",
        "https://",
        "https://github.com/path",
        "https://token@github.com",
        "https://github.com?query",
        "https://github.com#fragment",
    ] {
        let mut fixture = attested(&"a".repeat(64));
        fixture.set("GITHUB_SERVER_URL", server);
        fixture.set("ON_UNAVAILABLE", "skip");
        for step in ["validate", "verify-recorded"] {
            refused(
                &fixture.run_body(&format!("rust-gate attest-binaries {step}")),
                "GITHUB_SERVER_URL must be an HTTPS origin",
            );
        }
        assert!(fixture.calls().is_empty());
    }
}

#[test]
fn an_attestation_over_another_digest_or_none_at_all_is_refused() {
    let other = attested(&"b".repeat(64));
    let output = other.run_body("rust-gate attest-binaries verify-recorded");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Verified attestation does not cover the payload digest"),
        "{stderr}"
    );
    let silent = attested("");
    silent.stub("gh", "");
    let output = silent.run_body("rust-gate attest-binaries verify-recorded");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Attestation verification produced no result"),
        "{stderr}"
    );
}
