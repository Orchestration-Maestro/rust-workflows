//! `rust-gate install-tools`: what it refuses, what it honours, and what
//! `ci.yml` asks it to install.

use crate::harness::{Fixture, refused, succeeds, tool_rows, workflow};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;

/// The stand-ins a download needs: curl leaves the file behind, sha256sum
/// answers as told, tar does nothing, install creates the target.
fn prepare(fixture: &Fixture, checksum_holds: bool) {
    // curl must leave the file behind, or verification has nothing to read.
    fixture.stub(
        "curl",
        r#"out=""
while [[ $# -gt 0 ]]; do
  [[ "$1" == --output ]] && { out=$2; shift 2; continue; }
  shift
done
printf 'downloaded' > "$out""#,
    );
    fixture.stub(
        "sha256sum",
        if checksum_holds {
            "cat > /dev/null; exit 0"
        } else {
            "cat > /dev/null; exit 1"
        },
    );
    fixture.stub("tar", "");
    fixture.stub(
        "install",
        r#"dst=${@: -1}
mkdir -p "$(dirname "$dst")"
printf '#!/bin/bash\nexit 0\n' > "$dst"
chmod +x "$dst""#,
    );
}

/// How many release assets the fixture fetched.
fn downloads(fixture: &Fixture) -> usize {
    fixture
        .calls()
        .lines()
        .filter(|line| line.contains("releases/download/"))
        .count()
}

/// Two tools: a bare binary and an archive member, with a comment and a
/// blank line between them.
const TABLE: &str = "jaq 01mf02/jaq/releases/download/v3.1.1/jaq-x86_64-unknown-linux-gnu \
    5922c7b67d9bd6841d6676d1f954410c6bf04b47203dcb661c4f052dfef7f454\n\
    # a comment, and a blank line, are allowed between tools\n\n\
    cargo-vet mozilla/cargo-vet/releases/download/v0.10.0/\
    cargo-vet-x86_64-unknown-linux-gnu.tar.xz \
    c7664d9db5dd2ff813f20303650ac8253fa712ff2a1ea9ce12bed71e346f1744 \
    cargo-vet-x86_64-unknown-linux-gnu/cargo-vet\n";

#[test]
fn tool_installation_verifies_every_download_and_fetches_nothing_unasked() {
    // `rust-gate install-tools` is how untrusted bytes become executables on
    // a self-hosted runner. A download whose checksum is not enforced is a
    // supply-chain entry point, and an optional tool fetched without being
    // asked for is an unreviewed dependency every consumer inherits.
    // One download per listed tool, each installed under its own name, the
    // archive member extracted and the bare binary copied, and the directory
    // handed to every later step.
    let mut listed = Fixture::new();
    prepare(&listed, true);
    listed.set("TOOLS", TABLE);
    succeeds(&listed.run("ci", "install"));
    assert_eq!(downloads(&listed), 2);
    let bin = listed.root.join("rust-tools/bin");
    assert!(bin.join("jaq").is_file() && bin.join("cargo-vet").is_file());
    assert!(
        listed
            .calls()
            .contains(" cargo-vet-x86_64-unknown-linux-gnu/cargo-vet\n")
    );
    assert!(
        fs::read_to_string(listed.root.join("path"))
            .unwrap()
            .contains("rust-tools/bin")
    );

    // A digest that does not match must stop the job before the bytes reach a
    // parser or the PATH. This is the check the whole command exists for.
    let mut tampered = Fixture::new();
    prepare(&tampered, false);
    tampered.set("TOOLS", TABLE);
    assert!(
        !tampered.run("ci", "install").status.success(),
        "a download failing its checksum must not be installed"
    );
    let calls = tampered.calls();
    assert!(
        !calls.contains("tar ") && !calls.contains("install "),
        "{calls}"
    );
    assert!(!tampered.root.join("path").exists());
}

#[test]
fn a_dropped_connection_is_retried_before_a_download_fails() {
    // A connection GitHub's release storage reset once failed a whole run
    // (curl exit 35). curl retries only timeouts and server errors unless it
    // is told to retry every error.
    let mut fixture = Fixture::new();
    prepare(&fixture, true);
    fixture.set("TOOLS", TABLE);
    succeeds(&fixture.run("ci", "install"));
    assert_eq!(downloads(&fixture), 2);
    for call in fixture
        .calls()
        .lines()
        .filter(|line| line.contains("releases/download/"))
    {
        assert!(call.contains("--retry 4 --retry-all-errors "), "{call}");
    }
}

#[test]
fn every_download_comes_directly_from_github_releases() {
    let mut fixture = Fixture::new();
    prepare(&fixture, true);
    fixture.set("TOOLS", TABLE);
    succeeds(&fixture.run("ci", "install"));
    assert_eq!(downloads(&fixture), 2);
    for url in fixture
        .calls()
        .lines()
        .filter(|line| line.contains("releases/download/"))
    {
        assert!(
            url.contains(" https://github.com/") && !url.contains("github.com//"),
            "download not from GitHub releases: {url}"
        );
    }
}

#[test]
fn a_tool_line_names_an_immutable_release_asset_with_its_own_digest() {
    // Every line must name an immutable release asset with its own digest. An
    // absolute URL, a tag or branch archive, a malformed digest, or a name or
    // member that escapes its directory is refused before anything is fetched.
    let digest = "020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d";
    let asset = "01mf02/jaq/releases/download/v3.1.1/jaq-x86_64-unknown-linux-gnu";
    for (line, message) in [
        (
            format!("jaq https://evil.example/jaq {digest}"),
            "Not a release asset path:",
        ),
        (
            format!("jaq 01mf02/jaq/archive/refs/tags/v3.1.1.tar.gz {digest}"),
            "Not a release asset path:",
        ),
        (format!("jaq {asset} notadigest"), "Invalid sha256 for"),
        (
            format!("jaq {asset} {digest} ../../bin/jaq"),
            "Invalid archive member for",
        ),
        (format!("../jaq {asset} {digest}"), "Invalid tool name:"),
        (
            format!("jaq ../{asset} {digest}"),
            "Not a release asset path:",
        ),
        (String::new(), "tools lists nothing to install"),
    ] {
        let mut fixture = Fixture::new();
        prepare(&fixture, true);
        fixture.set("TOOLS", &line);
        refused(&fixture.run("ci", "install"), message);
        assert_eq!(downloads(&fixture), 0, "fetched before refusing {line:?}");
    }
}

#[test]
fn ci_installs_its_toolbelt_once_and_each_optional_tool_behind_its_gate() {
    // In ci.yml the mandatory toolbelt is one unconditional call and each
    // optional gate's tool is its own call, conditional on exactly that gate:
    // nothing is fetched unasked. Every asset is an immutable release, and no
    // digest is shared between two tools.
    let ci = workflow("ci");
    let mut gated = BTreeMap::new();
    let mut digests = Vec::new();
    let mut mandatory = 0;
    for step in ci["jobs"]["checks"]["steps"].as_array().unwrap() {
        let rows = tool_rows(step);
        if rows.is_empty() {
            continue;
        }
        for row in &rows {
            assert!(
                !row.asset.starts_with("http") && row.asset.contains("/releases/download/"),
                "not a release asset: {}",
                row.asset
            );
            let archive = [".tar.gz", ".tgz", ".tar.xz"]
                .iter()
                .any(|suffix| row.asset.ends_with(suffix));
            assert_eq!(
                archive,
                row.member.is_some(),
                "an archive names its member and a bare binary none: {}",
                row.asset
            );
            digests.push(row.digest.clone());
        }
        match step["if"].as_str() {
            None => mandatory += rows.len(),
            Some(condition) => {
                assert_eq!(
                    rows.len(),
                    1,
                    "a gate must add exactly one tool: {condition}"
                );
                gated.insert(rows[0].name.clone(), condition.to_owned());
            }
        }
    }
    assert!(mandatory > 5, "a default run must install its own tools");
    let expected: BTreeMap<String, String> = [
        ("cargo-mutants", "${{ inputs.mutation-test }}"),
        ("cargo-semver-checks", "${{ inputs.api-compatibility }}"),
        ("clippy-sarif", "${{ inputs.sarif-reports }}"),
        ("cargo-machete", "${{ inputs.unused-dependencies }}"),
        ("cargo-vet", "${{ inputs.dependency-audit }}"),
    ]
    .into_iter()
    .map(|(tool, condition)| (tool.to_owned(), condition.to_owned()))
    .collect();
    assert_eq!(gated, expected);
    let unique: BTreeSet<_> = digests.iter().collect();
    assert_eq!(
        digests.len(),
        unique.len(),
        "a checksum is reused between tools"
    );
}
