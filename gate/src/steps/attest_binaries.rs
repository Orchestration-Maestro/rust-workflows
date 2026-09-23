//! `rust-gate attest-binaries <step>`: the steps around signing a payload
//! this job verified itself, and the honest record of whether it was signed.

use crate::runner::{Cmd, Failure, Outcome, Step, input, non_empty, output, path, summary};

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "attest-binaries",
        id: "validate",
        summary: "Validate consumer inputs",
        inputs: &["GITHUB_SERVER_URL", "ON_UNAVAILABLE"],
        tools: &["gh"],
        reports: &[],
        run: validate,
    },
    Step {
        workflow: "attest-binaries",
        id: "payload-sbom",
        summary: "Extract the verified bill of materials",
        inputs: &[],
        tools: &["jaq", "tar"],
        reports: &[],
        run: payload_sbom,
    },
    Step {
        workflow: "attest-binaries",
        id: "verify-recorded",
        summary: "Verify the recorded attestation",
        inputs: &["EXPECTED_DIGEST", "GITHUB_REPOSITORY", "GITHUB_SERVER_URL"],
        tools: &["gh", "jaq"],
        reports: &[],
        run: verify_recorded,
    },
    Step {
        workflow: "attest-binaries",
        id: "outcome",
        summary: "Record the attestation outcome",
        inputs: &["ATTEST", "DIGEST", "RUN", "SBOM", "VERIFY"],
        tools: &[],
        reports: &[],
        run: outcome,
    },
];

/// Only a successful metadata response identifying Enterprise Server proves
/// unavailability. Missing Cloud entitlement, API and authentication errors
/// remain blocking; run=true authorizes an attempt, not a capability claim.
fn validate() -> Outcome {
    let policy = input("ON_UNAVAILABLE")?;
    if !matches!(policy.as_str(), "skip" | "fail") {
        return Err("on-unavailable must be skip or fail".into());
    }
    let server = github_command(Cmd::new("gh api meta --jq"))?
        .arg("(.installed_version // \"\") | type == \"string\" and length > 0")
        .capture()?;
    match server.trim() {
        "true" if policy == "skip" => output("run", "false"),
        "true" => Err("Artifact attestations are unavailable on GitHub Enterprise Server".into()),
        "false" => output("run", "true"),
        _ => Err("GitHub platform metadata did not identify the server type".into()),
    }
}

/// The payload SBOM travels inside the verified tarball, so extracting it
/// here means the attested bill of materials is the one whose checksum was
/// just confirmed rather than a separately fetched copy.
fn payload_sbom() -> Outcome {
    let temp = path("RUNNER_TEMP")?;
    Cmd::new("tar -xzf rust-release/payload.tar.gz -C")
        .arg(&temp)
        .arg("./payload.cdx.json")
        .run()?;
    Cmd::new("jaq -e")
        .arg(".bomFormat == \"CycloneDX\" and .metadata.component.name == \"rust-release-payload\"")
        .arg(temp.join("payload.cdx.json"))
        .capture()
        .map(|_| ())
        .map_err(|_| "Payload SBOM is missing or unusable".into())
}

/// A signature nobody checks proves nothing. Verifying here means a broken
/// or unattached attestation fails the release instead of being discovered
/// by whoever consumes the artifact months later.
fn verify_recorded() -> Outcome {
    let repository = input("GITHUB_REPOSITORY")?;
    let result = path("RUNNER_TEMP")?.join("attestation.json");
    github_command(Cmd::new(
        "gh attestation verify rust-release/payload.tar.gz --repo",
    ))?
    .arg(&repository)
    .arg("--signer-workflow")
    .arg("Orchestration-Maestro/rust-workflows/.github/workflows/attest-binaries.yml")
    .args(["--format", "json"])
    .stdout_to(&result)?;
    non_empty(&result).map_err(|_| "Attestation verification produced no result")?;
    // The verified subject must be the payload this job hashed itself.
    Cmd::new("jaq -er --arg digest")
        .arg(input("EXPECTED_DIGEST")?)
        .arg(
            "[.[] | .verificationResult.statement.subject[]?.digest.sha256]\n\
             | index($digest) != null",
        )
        .arg(&result)
        .capture()
        .map(|_| ())
        .map_err(|_| "Verified attestation does not cover the payload digest".into())
}

/// Give both gh invocations a host, not GitHub's full server URL. The workflow
/// supplies the same job token through the Cloud and Enterprise variable names.
fn github_command(command: Cmd) -> Result<Cmd, Failure> {
    let server = input("GITHUB_SERVER_URL")?;
    let host = server
        .strip_prefix("https://")
        .filter(|host| {
            !host.is_empty()
                && !host.contains(['/', '?', '#', '@'])
                && !host.chars().any(char::is_whitespace)
        })
        .ok_or("GITHUB_SERVER_URL must be an HTTPS origin")?;
    Ok(command.env("GH_HOST", host))
}

/// Only complete signing and independent verification count as attested.
/// A proven platform skip stays visible; every other incomplete run fails.
fn outcome() -> Outcome {
    let run = input("RUN")?;
    let states = [input("ATTEST")?, input("SBOM")?, input("VERIFY")?];
    let attested = run == "true" && states.iter().all(|state| state == "success");
    let skipped = run == "false" && states.iter().all(|state| state == "skipped");
    output("attested", if attested { "true" } else { "false" })?;
    let body = if attested {
        format!(
            "Attested and independently verified for digest {}.\n",
            input("DIGEST")?
        )
    } else {
        let reason = if skipped {
            "GitHub metadata identified Enterprise Server, which does not support \
             artifact attestations."
        } else {
            "Signing or independent verification did not complete successfully."
        };
        format!(
            "Not attested. {reason}\n\n\
             This run carries no cryptographic provenance accepted by this workflow.\n"
        )
    };
    summary(&format!("## Build provenance\n\n{body}"))?;
    if attested || skipped {
        Ok(())
    } else {
        Err("Attestation signing or verification did not succeed".into())
    }
}
