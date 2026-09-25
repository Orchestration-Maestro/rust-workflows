//! `ci.yml`'s pull request steps: COV-002, the coverage of the lines a pull
//! request adds, and PRL-001 and PRL-002, a feature without a test and a
//! change past four hundred lines, each read from a real change against a
//! base commit; and PRL-003 and PRL-004, a title and a head branch outside
//! the conventions, refused by `ci.yml` and by `hygiene.yml` alike.

use crate::harness::{Fixture, refused, succeeds, tool};
use std::fs;
use std::path::Path;

/// Run git in `directory`.
fn git(directory: &Path, args: &[&str]) {
    let output = tool("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    succeeds(&output);
}

/// A checkout whose last commit writes `files` over a base commit holding
/// `src/lib.rs`, the pull request `title` names, as a merge commit's first
/// parent is its base.
fn pull_request(title: &str, files: &[(&str, &str)]) -> Fixture {
    let mut fixture = Fixture::new();
    let project = fixture.root.join("project");
    git(&project, &["init", "-q"]);
    fs::write(project.join("src/lib.rs"), "//! Base.\npub fn a() {}\n").unwrap();
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "base"]);
    for (path, text) in files {
        let file = project.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "change"]);
    fixture.set("GITHUB_WORKSPACE", &project.display().to_string());
    fixture.set("GITHUB_BASE_REF", "main");
    fixture.set("PULL_REQUEST_TITLE", title);
    fixture.set("GITHUB_HEAD_REF", "feat/a-change");
    fixture
}

/// What PRL-003 says of the title `Add b`.
const TITLE_FINDING: &str = "PRL-003 the title `Add b` is not a conventional header: write \
     `<type>(<scope>)!: <subject>`, the type one of feat, fix, docs, style, refactor, perf, \
     test, build, ci, chore, revert, the lowercase scope and the `!` optional, the subject \
     opening in lowercase within 71 bytes; fix the title, then push a commit or close and \
     reopen the pull request: a rerun reads the title the run started with";

/// What PRL-004 says of the branch `wip`.
const BRANCH_FINDING: &str = "PRL-004 the branch `wip` is not named `<type>/<name>`: the \
     type one of feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert, then \
     segments in lowercase kebab-case separated by `/`, such as feat/refuse-a-title; open \
     the pull request again from a branch so named: a branch cannot be renamed under an \
     open pull request";

#[test]
fn a_title_or_branch_outside_the_conventions_is_refused() {
    let mut named = pull_request("Add b", &[("guide.md", "A guide.\n")]);
    named.set("GITHUB_HEAD_REF", "wip");
    refused(
        &named.run("ci", "pull-request"),
        &format!("pull-request: {TITLE_FINDING}; {BRANCH_FINDING}"),
    );
    let report = fs::read_to_string(named.root.join("reports/pull-request.txt")).unwrap();
    assert!(
        report.contains(&format!("{TITLE_FINDING}\n{BRANCH_FINDING}\n")),
        "{report}"
    );
    // A repository without Rust is held to the same two by hygiene.yml.
    refused(
        &named.run("hygiene", "pull-request-names"),
        &format!("pull-request-names: {TITLE_FINDING}; {BRANCH_FINDING}"),
    );
    // Each bot's title and branch pass both, as does GitHub's Revert button;
    // Dependabot's long titles are not held to the length. A push has no pull
    // request to name.
    let long = format!(
        "build(deps): bump the cargo group across 5 directories with {}",
        "a ".repeat(20)
    );
    for (title, branch) in [
        (
            "chore(main): release 2.4.0",
            "release-please--branches--main",
        ),
        (
            "ci(deps): bump actions/checkout from 6.0.0 to 7.0.1",
            "dependabot/github_actions/actions/checkout-7.0.1",
        ),
        (long.as_str(), "dependabot/cargo/cargo-a1b2c3"),
        (
            "chore: sync the organization's files to rust-workflows v2.4.0",
            "maestro/sync",
        ),
        (
            "Revert \"feat: refuse a title\"",
            "revert-54-feat/refuse-a-title",
        ),
    ] {
        let mut bot = pull_request(title, &[("guide.md", "A.\n")]);
        bot.set("GITHUB_HEAD_REF", branch);
        succeeds(&bot.run("ci", "pull-request"));
        succeeds(&bot.run("hygiene", "pull-request-names"));
    }
    let mut bot = pull_request("chore(main): release 2.4.0", &[("guide.md", "A.\n")]);
    bot.set("GITHUB_HEAD_REF", "release-please--branches--main");
    succeeds(&bot.run("hygiene", "pull-request-names"));
    let summary = fs::read_to_string(bot.root.join("summary")).unwrap();
    assert!(
        summary.contains("PRL-003 and PRL-004: no finding."),
        "{summary}"
    );
    named.set("GITHUB_BASE_REF", "");
    succeeds(&named.run("hygiene", "pull-request-names"));
    succeeds(&named.run("ci", "pull-request"));
}

#[test]
fn new_lines_that_never_run_are_refused_past_the_allowance() {
    let source = "//! Base.\npub fn a() {}\npub fn b() {}\npub fn c() {}\npub fn d() {}\n";
    let fixture = pull_request("feat: add three functions", &[("src/lib.rs", source)]);
    let lcov = |hits: [u32; 3]| {
        let file = fixture.root.join("project/src/lib.rs");
        format!(
            "SF:{}\nDA:2,1\nDA:3,{}\nDA:4,{}\nDA:5,{}\nend_of_record\n",
            file.display(),
            hits[0],
            hits[1],
            hits[2]
        )
    };
    let report = || fs::read_to_string(fixture.root.join("reports/changed-coverage.txt")).unwrap();
    fs::write(fixture.root.join("reports/coverage.lcov"), lcov([1, 0, 1])).unwrap();
    succeeds(&fixture.run("ci", "changed-coverage"));
    assert_eq!(
        report(),
        "3 coverable new lines, 1 uncovered, 1 allowed at 95 %\nUNCOVERED src/lib.rs:4\n"
    );
    fs::write(fixture.root.join("reports/coverage.lcov"), lcov([0, 0, 1])).unwrap();
    refused(
        &fixture.run("ci", "changed-coverage"),
        "changed-coverage: 2 of 3 new lines never run; this pull request covers 95 % of them, \
         leaving at most 1; changed-coverage.txt names each",
    );
    let mut push = fixture;
    push.set("GITHUB_BASE_REF", "");
    succeeds(&push.run("ci", "changed-coverage"));
    assert!(
        fs::read_to_string(push.root.join("reports/changed-coverage.txt"))
            .unwrap()
            .starts_with("NOT APPLICABLE")
    );
}

#[test]
fn a_feature_without_a_test_is_refused_and_a_large_change_reported() {
    let changed = "//! Base.\npub fn a() {}\npub fn b() {}\n";
    let untested = pull_request("feat: add b", &[("src/lib.rs", changed)]);
    refused(
        &untested.run("ci", "pull-request"),
        "pull-request: a feat pull request changes product code and touches no test \
         (PRL-001); pull-request.txt names the files",
    );
    let report = fs::read_to_string(untested.root.join("reports/pull-request.txt")).unwrap();
    assert!(
        report.contains(
            "PRL-001 a feat pull request changes product code (src/lib.rs) and touches no test; \
             add the test that fails without the change\n"
        ),
        "{report}"
    );
    // A test beside the change, in the file or under tests/, lets it pass; a
    // docs pull request is not held to it.
    let module =
        format!("{changed}\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn b() {{}}\n}}\n");
    succeeds(&pull_request("fix: b", &[("src/lib.rs", &module)]).run("ci", "pull-request"));
    let beside = [("src/lib.rs", changed), ("tests/it/main.rs", "//! It.\n")];
    succeeds(&pull_request("feat: add b", &beside).run("ci", "pull-request"));
    succeeds(&pull_request("docs: b", &[("src/lib.rs", changed)]).run("ci", "pull-request"));
    // Past four hundred lines, reported and never refused; the lockfile
    // does not count.
    let long = "line\n".repeat(401);
    let large = pull_request(
        "docs: a long guide",
        &[("guide.md", &long), ("Cargo.lock", &long)],
    );
    succeeds(&large.run("ci", "pull-request"));
    let report = fs::read_to_string(large.root.join("reports/pull-request.txt")).unwrap();
    assert!(
        report.contains(
            "PRL-002 401 changed lines, over 400; reported, not refused: split the change where \
             it splits\n"
        ),
        "{report}"
    );
}
