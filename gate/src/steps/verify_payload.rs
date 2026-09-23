//! `rust-gate verify-payload`: a downloaded release payload checked against
//! its checksum manifest, its provenance and the checked-out source
//! revision, in the job that is about to publish or sign it, so nothing is
//! trusted from an earlier job whose output could have been replaced.

use crate::checks::checkout_paths::is_symlink;
use crate::checks::simple_names::is_hex;
use crate::runner::{Cmd, Outcome, Step, flag, input, output};
use std::collections::BTreeSet;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "shared",
    id: "verify-payload",
    summary: "Verify a downloaded payload before it is published or signed",
    inputs: &["GITHUB_SHA", "REQUIRE_BINARIES", "REVISION"],
    tools: &["jaq", "sha256sum"],
    reports: &[],
    run,
}];

/// Run the step. `REVISION` is the commit the payload was built from and
/// must be the checked-out one; `REQUIRE_BINARIES` refuses a payload whose
/// provenance lists no binaries, which a binary release must never be.
fn run() -> Outcome {
    let revision = input("REVISION")?;
    if !is_hex(&revision, 40) {
        return Err("revision must be an immutable commit SHA".into());
    }
    if revision != input("GITHUB_SHA")? {
        return Err("Release revision does not match source".into());
    }
    let require_binaries = flag("REQUIRE_BINARIES")?;
    let release = Path::new("rust-release");
    let manifest = release.join("SHA256SUMS");
    if is_symlink(&manifest) {
        return Err("Checksum file must not be a symlink".into());
    }
    // The manifest must name exactly the two release files, each once, and
    // neither may be a symlink: a link would have the job hash, and then
    // release, whatever it points at rather than the payload it was handed.
    let text =
        std::fs::read_to_string(&manifest).map_err(|_| "Missing required release checksums")?;
    let mut seen = BTreeSet::new();
    for line in text.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            return Err("Invalid release checksum selector".into());
        };
        let valid = is_hex(digest, 64) && (name == "payload.tar.gz" || name == "provenance.json");
        if !valid {
            return Err("Invalid release checksum selector".into());
        }
        if is_symlink(&release.join(name)) || !seen.insert(name.to_owned()) {
            return Err("Duplicate checksum selector or symlink".into());
        }
    }
    if seen.len() != 2 {
        return Err("Missing required release checksums".into());
    }
    Cmd::new("sha256sum --check --strict SHA256SUMS")
        .cwd(release)
        .run()?;
    Cmd::new("jaq -e --arg revision")
        .arg(&revision)
        .arg("--argjson")
        .arg("binaries")
        .arg(if require_binaries { "true" } else { "false" })
        .arg(
            "
          .revision == $revision and .target == \"x86_64-unknown-linux-gnu\"
          and (.binaries | type == \"array\"
            and all(.[]; type == \"string\" and test(\"^[A-Za-z0-9_][A-Za-z0-9_-]*$\")))
          and (($binaries | not) or (.binaries | length > 0))
        ",
        )
        .arg("provenance.json")
        .cwd(release)
        .capture()
        .map_err(|_| "Unexpected release revision, target or binary selection")?;
    // The digest handed on is computed from the verified bytes, not copied
    // out of the manifest the job was given.
    let digest = Cmd::new("sha256sum payload.tar.gz")
        .cwd(release)
        .capture()?;
    output(
        "payload-digest",
        digest.split_whitespace().next().unwrap_or_default(),
    )
}
