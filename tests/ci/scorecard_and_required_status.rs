//! `ci.yml`: the scorecard, the required status and mutation testing.

use crate::harness::{Fixture, refused, root, succeeds, workflow};
use serde_json::Value;
use std::fs;

#[test]
fn the_scorecard_reports_what_ran_and_refuses_to_imply_more() {
    // The scorecard is the number a reader will quote. If it counted a gate that
    // was skipped, every claim made from it would be wrong, so both extremes are
    // exercised: nothing selected, and everything selected.
    let outcomes = [
        "OUT_QUALITY",
        "OUT_COVERAGE",
        "OUT_AUDIT",
        "OUT_SECRETS",
        "OUT_MSRV",
        "OUT_FEATURES",
        "OUT_LICENCES",
        "OUT_MUTANTS",
        "OUT_UNUSED",
        "OUT_STAGE",
        "OUT_API",
    ];
    let read = |f: &Fixture, key: &str| -> String {
        let text = fs::read_to_string(f.root.join("reports/scorecard.json")).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        value[key].to_string()
    };

    // Defaults only: a consumer who configures nothing.
    let mut bare = Fixture::new();
    bare.set("RUSTUP_TOOLCHAIN", "1.98.1");
    bare.set("DENY_CONFIG", "");
    for key in outcomes {
        bare.set(key, "success");
    }
    succeeds(&bare.run("ci", "scorecard"));
    let bare_active: i64 = read(&bare, "active").parse().unwrap();
    let available: i64 = read(&bare, "available").parse().unwrap();

    // Everything a caller can switch on.
    let mut full = Fixture::new();
    full.set("RUSTUP_TOOLCHAIN", "1.98.1");
    full.set(
        "DENY_CONFIG",
        &full.root.join("deny.toml").display().to_string(),
    );
    for key in outcomes {
        full.set(key, "success");
    }
    for key in [
        "MUTATION_TEST",
        "UNUSED_DEPENDENCIES",
        "SARIF_REPORTS",
        "API_COMPATIBILITY",
    ] {
        full.set(key, "true");
    }
    full.set("UNSAFE_POLICY", "deny");
    for key in ["FEATURES_APPLIED", "MUTANTS_APPLIED", "API_APPLIED"] {
        full.set(key, "true");
    }
    succeeds(&full.run("ci", "scorecard"));
    let full_active: i64 = read(&full, "active").parse().unwrap();

    assert!(
        bare_active < full_active,
        "a consumer who enables nothing must not score the same as one who enables everything"
    );
    assert_eq!(
        full_active, available,
        "enabling every gate must reach the advertised total"
    );

    // The badge is self-contained: a run must not depend on an external service
    // being reachable to render its own result.
    let svg = fs::read_to_string(full.root.join("reports/scorecard.svg")).unwrap();
    assert!(svg.starts_with("<svg"));
    // `xmlns` is an XML namespace identifier, never dereferenced. Anything that
    // would actually be fetched, an image href, a stylesheet or a font, would
    // make the badge depend on a service being reachable to render a local
    // result.
    for fetching in ["xlink:href", "<image", "@import", "<use", "src="] {
        assert!(
            !svg.contains(fetching),
            "the badge must not fetch anything: found {fetching}"
        );
    }
    assert_eq!(
        svg.matches("http").count(),
        svg.matches("xmlns=\"http://www.w3.org/2000/svg\"").count(),
        "the only URL in the badge may be the SVG namespace"
    );
    assert!(
        fs::read_to_string(full.root.join("reports/scorecard.md"))
            .unwrap()
            .contains(&full_active.to_string())
    );
}

#[test]
fn the_diagram_counts_the_same_controls_the_scorecard_does() {
    // The diagram in the README quotes a number of controls, and a reader
    // takes that number for the truth. It comes from the same list the run
    // scores itself against, so a control added or removed moves both or
    // fails here.
    let mut f = Fixture::new();
    f.set("RUSTUP_TOOLCHAIN", "1.98.1");
    f.set("DENY_CONFIG", "");
    for key in [
        "OUT_QUALITY",
        "OUT_COVERAGE",
        "OUT_AUDIT",
        "OUT_SECRETS",
        "OUT_MSRV",
        "OUT_FEATURES",
        "OUT_LICENCES",
        "OUT_MUTANTS",
        "OUT_UNUSED",
        "OUT_STAGE",
        "OUT_API",
    ] {
        f.set(key, "success");
    }
    succeeds(&f.run("ci", "scorecard"));
    let scorecard: Value =
        serde_json::from_str(&fs::read_to_string(f.root.join("reports/scorecard.json")).unwrap())
            .unwrap();
    let claim = format!("{} controls", scorecard["available"]);

    let diagram = fs::read_to_string(root().join(".github/assets/how-it-works.svg")).unwrap();
    assert!(
        diagram.contains(&claim),
        "the how-it-works diagram must say \"{claim}\", the total the scorecard reports"
    );
}

#[test]
fn a_failing_consumer_command_fails_the_step_with_its_own_status() {
    // The consumer's tool decides; the gate carries its exit status through
    // unchanged rather than folding every failure into 1.
    let f = Fixture::new();
    f.stub("cargo", "exit 42");
    assert_eq!(f.run("ci", "quality").status.code(), Some(42));
}

#[test]
fn the_required_status_fails_unless_every_result_succeeded() {
    // The one status a branch protection can require: green only when every
    // upstream result was a success, in ci.yml and in the consumer matrix alike.
    let mut f = Fixture::new();
    for status in ["failure", "cancelled", "skipped", ""] {
        f.set("RESULT", status);
        refused(
            &f.run("ci", "required"),
            "Required Rust checks failed or were skipped",
        );
    }
    f.set("RESULT", "success");
    succeeds(&f.run("ci", "required"));
    for key in [
        "CI_RESULT",
        "BINARY_RESULT",
        "CRATE_RESULT",
        "PORTABILITY_RESULT",
    ] {
        f.set(key, "success");
    }
    succeeds(&f.run("ci-internal", "required"));
    for key in [
        "CI_RESULT",
        "BINARY_RESULT",
        "CRATE_RESULT",
        "PORTABILITY_RESULT",
    ] {
        f.set(key, "skipped");
        assert!(!f.run("ci-internal", "required").status.success());
        f.set(key, "success");
    }
}

/// Stand-ins for a mutation run: git answers `rev-parse` with `parent` and
/// prints a one-file diff, cargo-mutants catches its one mutant.
fn prepare_mutants(f: &Fixture, parent: &str) {
    f.stub(
        "git",
        &format!(
            r#"case "$1" in
  rev-parse) {parent} ;;
  diff) printf 'diff --git a/src/lib.rs b/src/lib.rs\n' ;;
esac"#
        ),
    );
    f.stub(
        "cargo",
        r#"[[ "$1" == mutants ]] || exit 0
out=""
while [[ $# -gt 0 ]]; do [[ "$1" == --output ]] && out=$2; shift; done
mkdir -p "$out/mutants.out"
printf '{"caught":1,"missed":0,"timeout":0,"unviable":0}\n' > "$out/mutants.out/outcomes.json""#,
    );
}

#[test]
fn mutation_testing_scopes_a_pull_request_to_its_diff() {
    // A pull request mutates only what it changed, and the report says which
    // scope applied.
    let mut pr = Fixture::new();
    prepare_mutants(&pr, "exit 0");
    pr.set("MUTATION_TEST", "true");
    pr.set("GITHUB_BASE_REF", "main");
    succeeds(&pr.run("ci", "mutants"));
    assert!(pr.calls().contains("--in-diff"), "{}", pr.calls());
    assert!(
        fs::read_to_string(pr.root.join("mutants.diff"))
            .unwrap()
            .contains("src/lib.rs")
    );
    let report = fs::read_to_string(pr.root.join("reports/mutants.txt")).unwrap();
    assert!(
        report.contains("changes against main") && report.contains("caught=1"),
        "{report}"
    );
}

#[test]
fn mutation_testing_scopes_a_push_to_its_own_commit() {
    // Every merge to the default branch is one squashed pull request, so a
    // push or a release tag mutates its commit's diff, never the whole
    // workspace inside a job with a timeout. The checkout keeps the parent.
    let mut push = Fixture::new();
    prepare_mutants(&push, "exit 0");
    push.set("MUTATION_TEST", "true");
    succeeds(&push.run("ci", "mutants"));
    assert!(push.calls().contains("--in-diff"), "{}", push.calls());
    assert!(
        fs::read_to_string(push.root.join("reports/mutants.txt"))
            .unwrap()
            .contains("changes in the last commit")
    );
    let checkout = &workflow("ci")["jobs"]["checks"]["steps"][0];
    assert_eq!(checkout["with"]["fetch-depth"], 2, "{checkout}");

    // A repository's first commit has no parent: everything in it is new.
    let mut first = Fixture::new();
    prepare_mutants(&first, "exit 1");
    first.set("MUTATION_TEST", "true");
    succeeds(&first.run("ci", "mutants"));
    assert!(!first.calls().contains("--in-diff"));
    assert!(
        fs::read_to_string(first.root.join("reports/mutants.txt"))
            .unwrap()
            .contains("full workspace")
    );
}

#[test]
fn mutation_testing_accepts_real_diffs_without_applicable_mutants() {
    let source = "//! Updated.\npub fn answer() -> u32 { 42 }\n";
    for ((code, diff, reason), base, skipped) in [
        (source, "", "Diff file is empty"),
        (
            source,
            "--- a/README.md\n+++ b/README.md\n@@ -0,0 +1 @@\n+Documentation.\n",
            "Diff changes no Rust source files",
        ),
        (
            "pub const ANSWER: u32 = 42;\n",
            "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -0,0 +1 @@\n+pub const ANSWER: u32 = 42;\n",
            "No mutants to filter",
        ),
        (
            source,
            "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-//! Old.\n+//! Updated.\n",
            "No mutants to filter",
        ),
    ]
    .into_iter()
    .flat_map(|case| {
        [
            (
                case,
                "main",
                "SKIPPED: no mutants apply to this pull request",
            ),
            (case, "", "SKIPPED: no mutants apply to this commit"),
        ]
    }) {
        let mut f = Fixture::new();
        f.set("MUTATION_TEST", "true");
        f.set("GITHUB_BASE_REF", base);
        f.set("CARGO_TERM_COLOR", "always");
        f.set("CARGO_MUTANTS_TRACE_LEVEL", "error");
        fs::write(f.root.join("project/src/lib.rs"), code).unwrap();
        fs::write(f.root.join("project/README.md"), "Documentation.\n").unwrap();
        fs::write(f.root.join("changes.diff"), diff).unwrap();
        // Only Git is a stand-in: discovery and diff filtering use the pinned tool.
        f.stub(
            "git",
            "case \"$1\" in rev-parse) exit 0 ;; diff) cat \"$RUNNER_TEMP/changes.diff\" ;; esac",
        );
        succeeds(&f.run("ci", "mutants"));
        let report = fs::read_to_string(f.root.join("reports/mutants.txt")).unwrap();
        assert!(report.contains(reason), "{report}");
        assert!(report.contains(skipped), "{report}");
        assert!(!f.root.join("reports/mutants.json").exists());
        assert!(
            fs::read_to_string(f.root.join("output"))
                .unwrap()
                .ends_with("applied=false\n")
        );
    }
}

#[test]
fn mutation_testing_never_treats_an_unexplained_missing_report_as_a_skip() {
    // A diff message explains a missing report only when a diff was mutated:
    // a first commit, with no parent, mutates the whole workspace.
    for (base, git, cargo) in [
        ("", "exit 1", "echo 'INFO Diff file is empty' >&2"),
        ("main", "exit 0", "exit 0"),
        (
            "main",
            "exit 0",
            "echo 'unexpected: INFO Diff file is empty' >&2",
        ),
        (
            "main",
            "exit 0",
            "mkdir -p \"$RUNNER_TEMP/mutants/mutants.out\"\n\
             touch \"$RUNNER_TEMP/mutants/mutants.out/outcomes.json\"\n\
             echo 'INFO Diff file is empty' >&2",
        ),
    ] {
        let mut f = Fixture::new();
        f.set("MUTATION_TEST", "true");
        f.set("GITHUB_BASE_REF", base);
        f.stub("git", git);
        f.stub("cargo", cargo);
        refused(
            &f.run("ci", "mutants"),
            "cargo-mutants produced no outcomes",
        );
    }
    let mut f = Fixture::new();
    f.set("MUTATION_TEST", "true");
    f.set("GITHUB_BASE_REF", "main");
    f.stub("git", "exit 0");
    f.stub("cargo", "echo 'INFO Diff file is empty' >&2; exit 7");
    assert_eq!(f.run("ci", "mutants").status.code(), Some(7));
}

#[test]
fn mutation_testing_keeps_real_survivors_and_invalid_diffs_blocking() {
    let mut f = Fixture::new();
    f.set("MUTATION_TEST", "true");
    f.set("GITHUB_BASE_REF", "main");
    fs::write(
        f.root.join("project/src/lib.rs"),
        "pub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    succeeds(&f.run_body("cd \"$PROJECT\"; cargo generate-lockfile --offline"));
    fs::write(
        f.root.join("changes.diff"),
        "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n\
         -pub fn answer() -> u32 { 41 }\n+pub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    f.stub(
        "git",
        "case \"$1\" in rev-parse) exit 0 ;; diff) cat \"$RUNNER_TEMP/changes.diff\" ;; esac",
    );
    assert_eq!(f.run("ci", "mutants").status.code(), Some(2));
    let outcomes: Value = serde_json::from_str(
        &fs::read_to_string(f.root.join("mutants/mutants.out/outcomes.json")).unwrap(),
    )
    .unwrap();
    assert!(outcomes["missed"].as_u64().unwrap() > 0);
    assert!(
        f.root.join("reports/mutants.json").is_file(),
        "failed outcomes must be archived"
    );
    let archived: Value =
        serde_json::from_str(&fs::read_to_string(f.root.join("reports/mutants.json")).unwrap())
            .unwrap();
    assert_eq!(archived, outcomes);
    fs::write(f.root.join("changes.diff"), "not a unified diff\n").unwrap();
    assert!(!f.run("ci", "mutants").status.success());
}

#[test]
fn mutation_failures_keep_their_reports_and_original_status() {
    for (code, timeout) in [(3, 1), (4, 0)] {
        let mut f = Fixture::new();
        f.set("MUTATION_TEST", "true");
        f.stub(
            "cargo",
            &format!(
                "mkdir -p \"$RUNNER_TEMP/mutants/mutants.out\"\n\
             printf '{{\"caught\":0,\"missed\":0,\"timeout\":{timeout},\"unviable\":0}}' \
             > \"$RUNNER_TEMP/mutants/mutants.out/outcomes.json\"\nexit {code}"
            ),
        );
        assert_eq!(f.run("ci", "mutants").status.code(), Some(code));
        assert!(f.root.join("reports/mutants.json").is_file());
    }
}

#[test]
fn mutation_testing_refuses_survivors_missing_outcomes_and_a_shallow_checkout() {
    let outcomes = |missed: u32| {
        format!(
            r#"[[ "$1" == mutants ]] || exit 0
out=""
while [[ $# -gt 0 ]]; do [[ "$1" == --output ]] && out=$2; shift; done
mkdir -p "$out/mutants.out"
file="$out/mutants.out/outcomes.json"
printf '{{"caught":1,"missed":{missed},"timeout":0,"unviable":0}}\n' > "$file""#
        )
    };
    let mut flag = Fixture::new();
    flag.set("MUTATION_TEST", "maybe");
    refused(
        &flag.run("ci", "mutants"),
        "MUTATION_TEST must be true or false",
    );
    let mut survivors = Fixture::new();
    survivors.set("MUTATION_TEST", "true");
    survivors.stub("cargo", &outcomes(1));
    refused(
        &survivors.run("ci", "mutants"),
        "Surviving or timed-out mutants; strengthen the tests that should fail",
    );
    let mut silent = Fixture::new();
    silent.set("MUTATION_TEST", "true");
    silent.stub("cargo", "exit 0");
    refused(
        &silent.run("ci", "mutants"),
        "cargo-mutants produced no outcomes",
    );
    let mut shallow = Fixture::new();
    shallow.set("MUTATION_TEST", "true");
    shallow.set("GITHUB_BASE_REF", "main");
    shallow.stub("git", "exit 1");
    shallow.stub("cargo", &outcomes(0));
    refused(
        &shallow.run("ci", "mutants"),
        "pull request checkout must include the base parent",
    );
    assert!(
        !shallow.calls().contains("mutants"),
        "nothing runs on a shallow checkout"
    );
}
