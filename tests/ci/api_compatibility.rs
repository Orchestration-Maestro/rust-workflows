//! `rust-gate api`: a pull request that breaks a library's public API fails
//! unless its title declares the break, the way release-please reads it.

use crate::harness::{Fixture, refused, succeeds, workflow};
use std::fs;

/// A workspace member for the stand-ins: its name, its target kind, and
/// whether the base branch already has its manifest.
type Member<'a> = (&'a str, &'a str, bool);

/// The stand-ins: git answers whether the checkout kept the base parent and
/// which manifests the base branch has, cargo describes the workspace and
/// plays cargo-semver-checks.
fn prepare(fixture: &mut Fixture, parent: &str, members: &[Member<'_>], semver: &str) {
    fixture.set("API_COMPATIBILITY", "true");
    fixture.set("GITHUB_BASE_REF", "main");
    fixture.set("PULL_REQUEST_TITLE", "feat: add a checked product");
    let in_base: Vec<String> = members
        .iter()
        .filter(|member| member.2)
        .map(|member| format!("HEAD^1:{}/Cargo.toml", member.0))
        .collect();
    fixture.stub(
        "git",
        &format!(
            r#"case "$1 $2" in
  "rev-parse --show-toplevel") pwd ;;
  "rev-parse --verify") {parent} ;;
  "show HEAD^1:"*) [[ " {} " == *" $2 "* ]] || exit 128
    member="${{2#HEAD^1:}}"
    printf '[package]\nname = "%s"\n' "${{BASE_NAME:-${{member%/Cargo.toml}}}}" ;;
  *) exit 1 ;;
esac"#,
            in_base.join(" ")
        ),
    );
    let packages: Vec<String> = members
        .iter()
        .map(|(name, kind, _)| {
            let manifest = format!(r#""manifest_path":"'"$PWD"'/{name}/Cargo.toml""#);
            format!(r#"{{"name":"{name}",{manifest},"targets":[{{"kind":["{kind}"]}}]}}"#)
        })
        .collect();
    fixture.stub(
        "cargo",
        &format!(
            r#"case "$1" in
  metadata) printf '%s\n' '{{"packages":[{}]}}' ;;
  semver-checks) printf 'Summary semver requires new major version\n'; {semver} ;;
esac"#,
            packages.join(",")
        ),
    );
}

fn report(fixture: &Fixture) -> String {
    fs::read_to_string(fixture.root.join("reports/api-compatibility.txt")).unwrap()
}

fn applied(fixture: &Fixture) -> String {
    fs::read_to_string(fixture.root.join("output")).unwrap_or_default()
}

const CHECK: &str = "semver-checks --baseline-rev HEAD^1 --release-type minor --package fixture";

#[test]
fn an_undeclared_break_fails_the_pull_request() {
    // cargo-semver-checks compares the library against the base branch as a
    // minor release: any breaking change fails, additions pass.
    let mut broken = Fixture::new();
    prepare(
        &mut broken,
        "exit 0",
        &[("fixture", "lib", true)],
        "exit 100",
    );
    assert!(!broken.run("ci", "api").status.success());
    assert!(broken.calls().contains(CHECK), "{}", broken.calls());
    assert!(report(&broken).contains("requires new major version"));

    let mut compatible = Fixture::new();
    prepare(
        &mut compatible,
        "exit 0",
        &[("fixture", "lib", true)],
        "exit 0",
    );
    succeeds(&compatible.run("ci", "api"));
    assert!(compatible.calls().contains(CHECK));
    assert!(applied(&compatible).contains("applied=true"));
}

#[test]
fn a_declared_break_and_what_has_no_api_are_not_checked() {
    // Each case is reported as not applicable, never as a pass.
    let cases: [(&str, &str, &[Member<'_>], &str, &str); 7] = [
        (
            "PULL_REQUEST_TITLE",
            "feat(api)!: drop checked_sum",
            &[("fixture", "lib", true)],
            "exit 0",
            "declares",
        ),
        (
            "GITHUB_BASE_REF",
            "",
            &[("fixture", "lib", true)],
            "exit 0",
            "pull request",
        ),
        (
            "API_COMPATIBILITY",
            "false",
            &[("fixture", "lib", true)],
            "exit 0",
            "api-compatibility=false",
        ),
        (
            "RUSTUP_TOOLCHAIN",
            "1.92.0",
            &[("fixture", "lib", true)],
            "exit 0",
            "1.93",
        ),
        (
            "API_COMPATIBILITY",
            "true",
            &[("fixture", "bin", true)],
            "exit 0",
            "no library",
        ),
        // A library the pull request adds has no base to be compared with.
        (
            "API_COMPATIBILITY",
            "true",
            &[("fresh", "lib", false)],
            "exit 0",
            "new in this pull request: fresh",
        ),
        // A library the pull request renames has no baseline under its new
        // name: the base manifest at its path names another package.
        (
            "BASE_NAME",
            "old-fixture",
            &[("fixture", "lib", true)],
            "exit 0",
            "new in this pull request: fixture",
        ),
    ];
    for (key, value, members, parent, said) in cases {
        let mut fixture = Fixture::new();
        prepare(&mut fixture, parent, members, "exit 100");
        fixture.set(key, value);
        succeeds(&fixture.run("ci", "api"));
        assert!(!fixture.calls().contains("semver-checks"), "{key}={value}");
        assert!(
            report(&fixture).contains(said),
            "{key}={value}: {}",
            report(&fixture)
        );
        assert!(applied(&fixture).contains("applied=false"), "{key}={value}");
    }
}

#[test]
fn only_the_libraries_the_base_branch_has_are_compared() {
    // A workspace that gains a member compares the members it already had,
    // and the report names the new one.
    let mut fixture = Fixture::new();
    prepare(
        &mut fixture,
        "exit 0",
        &[
            ("core", "lib", true),
            ("fresh", "lib", false),
            ("app", "bin", true),
        ],
        "exit 0",
    );
    succeeds(&fixture.run("ci", "api"));
    let calls = fixture.calls();
    assert!(
        calls.contains("semver-checks --baseline-rev HEAD^1 --release-type minor --package core"),
        "{calls}"
    );
    assert!(!calls.contains("--package fresh") && !calls.contains("--package app"));
    assert!(report(&fixture).contains("new in this pull request: fresh"));
    assert!(applied(&fixture).contains("applied=true"));
}

#[test]
fn a_pull_request_checkout_must_keep_the_base_parent() {
    let mut fixture = Fixture::new();
    prepare(
        &mut fixture,
        "exit 1",
        &[("fixture", "lib", true)],
        "exit 0",
    );
    refused(
        &fixture.run("ci", "api"),
        "pull request checkout must include the base parent",
    );
}

#[test]
fn ci_reads_the_title_and_reports_the_check_to_the_scorecard() {
    let ci = workflow("ci");
    let input = &ci["on"]["workflow_call"]["inputs"]["api-compatibility"];
    assert_eq!(input["default"], true);
    let steps = ci["jobs"]["checks"]["steps"].as_array().unwrap();
    // The step always runs, so a switched-off gate still writes its report.
    let api = steps.iter().find(|step| step["id"] == "api").unwrap();
    assert!(api.get("if").is_none());
    assert_eq!(
        api["env"]["PULL_REQUEST_TITLE"],
        "${{ github.event.pull_request.title }}"
    );
    let scorecard = steps.iter().find(|step| step["id"] == "scorecard").unwrap();
    assert_eq!(scorecard["env"]["OUT_API"], "${{ steps.api.outcome }}");
    assert_eq!(
        scorecard["env"]["API_APPLIED"],
        "${{ steps.api.outputs.applied }}"
    );
}
