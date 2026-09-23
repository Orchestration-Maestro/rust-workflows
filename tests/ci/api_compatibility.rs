//! `rust-gate api`: a pull request that breaks a library's public API fails
//! unless its title declares the break, the way release-please reads it.

use crate::harness::{Fixture, refused, succeeds, workflow};
use std::fs;

/// The stand-ins: git answers whether the checkout kept the base parent, cargo
/// describes the workspace's targets and plays cargo-semver-checks.
fn prepare(f: &mut Fixture, parent: &str, kinds: &str, semver: &str) {
    f.set("API_COMPATIBILITY", "true");
    f.set("GITHUB_BASE_REF", "main");
    f.set("PULL_REQUEST_TITLE", "feat: add a checked product");
    f.stub("git", &format!(r#"[[ "$1" == rev-parse ]] && {parent}"#));
    f.stub(
        "cargo",
        &format!(
            r#"case "$1" in
  metadata) printf '{{"packages":[{{"targets":[{{"kind":[{kinds}]}}]}}]}}\n' ;;
  semver-checks) printf 'Summary semver requires new major version\n'; {semver} ;;
esac"#
        ),
    );
}

fn report(f: &Fixture) -> String {
    fs::read_to_string(f.root.join("reports/api-compatibility.txt")).unwrap()
}

fn applied(f: &Fixture) -> String {
    fs::read_to_string(f.root.join("output")).unwrap_or_default()
}

const CHECK: &str = "semver-checks --workspace --baseline-rev HEAD^1 --release-type minor";

#[test]
fn an_undeclared_break_fails_the_pull_request() {
    // cargo-semver-checks compares the library against the base branch as a
    // minor release: any breaking change fails, additions pass.
    let mut broken = Fixture::new();
    prepare(&mut broken, "exit 0", r#""lib""#, "exit 100");
    assert!(!broken.run("ci", "api").status.success());
    assert!(broken.calls().contains(CHECK), "{}", broken.calls());
    assert!(report(&broken).contains("requires new major version"));

    let mut compatible = Fixture::new();
    prepare(&mut compatible, "exit 0", r#""lib""#, "exit 0");
    succeeds(&compatible.run("ci", "api"));
    assert!(compatible.calls().contains(CHECK));
    assert!(applied(&compatible).contains("applied=true"));
}

#[test]
fn a_declared_break_and_what_has_no_api_are_not_checked() {
    // Each case is reported as not applicable, never as a pass.
    let cases: [(&str, &str, &str, &str, &str); 5] = [
        (
            "PULL_REQUEST_TITLE",
            "feat(api)!: drop checked_sum",
            r#""lib""#,
            "exit 0",
            "declares",
        ),
        ("GITHUB_BASE_REF", "", r#""lib""#, "exit 0", "pull request"),
        (
            "API_COMPATIBILITY",
            "false",
            r#""lib""#,
            "exit 0",
            "api-compatibility=false",
        ),
        ("RUSTUP_TOOLCHAIN", "1.92.0", r#""lib""#, "exit 0", "1.93"),
        (
            "API_COMPATIBILITY",
            "true",
            r#""bin""#,
            "exit 0",
            "no library",
        ),
    ];
    for (key, value, kinds, parent, said) in cases {
        let mut f = Fixture::new();
        prepare(&mut f, parent, kinds, "exit 100");
        f.set(key, value);
        succeeds(&f.run("ci", "api"));
        assert!(!f.calls().contains("semver-checks"), "{key}={value}");
        assert!(report(&f).contains(said), "{key}={value}: {}", report(&f));
        assert!(applied(&f).contains("applied=false"), "{key}={value}");
    }
}

#[test]
fn a_pull_request_checkout_must_keep_the_base_parent() {
    let mut f = Fixture::new();
    prepare(&mut f, "exit 1", r#""lib""#, "exit 0");
    refused(
        &f.run("ci", "api"),
        "pull request checkout must include the base parent",
    );
}

#[test]
fn ci_reads_the_title_and_reports_the_check_to_the_scorecard() {
    let ci = workflow("ci");
    let input = &ci["on"]["workflow_call"]["inputs"]["api-compatibility"];
    assert_eq!(input["default"], false);
    let steps = ci["jobs"]["checks"]["steps"].as_array().unwrap();
    let api = steps.iter().find(|step| step["id"] == "api").unwrap();
    assert_eq!(api["if"], "${{ inputs.api-compatibility }}");
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
