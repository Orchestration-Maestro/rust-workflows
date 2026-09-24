//! `ci.yml`'s `performance` step, PRF-001: the declared benchmarks' instruction
//! counts, the base against the pull request under the pinned gungraun, a rise
//! past 5 % refused unless excused, and nothing measured without a bench or a
//! base.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

/// A pull request fixture declaring the bench `tokenizer`, its lockfile on
/// `gungraun` at `version`, and stand-ins for git, sudo and cargo; cargo bench
/// exits `head` on the pull request's run.
fn declared(version: &str, head: u8) -> Fixture {
    let mut fixture = Fixture::new();
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[performance]\nbenches = [\"tokenizer\"]\n",
    )
    .unwrap();
    fs::write(
        fixture.root.join("project/Cargo.lock"),
        format!("version = 4\n\n[[package]]\nname = \"gungraun\"\nversion = \"{version}\"\n"),
    )
    .unwrap();
    fixture.stub("sudo", "printf 'sudo %s\\n' \"$*\" >> \"$CALLS\"");
    fixture.stub(
        "git",
        "printf 'git %s\\n' \"$*\" >> \"$CALLS\"\nmkdir -p \"$6/project\"",
    );
    fixture.stub(
        "cargo",
        &format!(
            "printf 'cargo %s\\n' \"$*\" >> \"$CALLS\"\ncase \"$*\" in *--baseline=base*) \
             echo 'Instructions: 105 (+5.0%)'; exit {head};; esac"
        ),
    );
    fixture.set("GITHUB_BASE_REF", "main");
    fixture
}

#[test]
fn a_benchmark_past_its_budget_is_refused_unless_excused() {
    let fixture = declared("0.19.4", 3);
    refused(
        &fixture.run("ci", "performance"),
        "performance: tokenizer runs more instructions than the base, over 5 % (PRF-001); \
         performance.txt holds the counts",
    );
    let calls = fixture.calls();
    assert!(
        calls
            .contains("sudo apt-get install -y --no-install-recommends valgrind=1:3.22.0-0ubuntu3"),
        "{calls}"
    );
    assert!(calls.contains("worktree add --detach"), "{calls}");
    assert!(
        calls.contains("bench --locked --bench tokenizer -- --save-baseline=base --home"),
        "{calls}"
    );
    assert!(calls.contains("--callgrind-limits=ir=5%"), "{calls}");
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[performance]\nbenches = [\"tokenizer\"]\n\n[[exception]]\nrule = \"PRF-001\"\npath \
         = \"tokenizer\"\nreason = \"the new table trades time for memory\"\n",
    )
    .unwrap();
    let excused = declared("0.19.4", 0);
    fs::copy(
        fixture.root.join("maestro-quality.toml"),
        excused.root.join("maestro-quality.toml"),
    )
    .unwrap();
    succeeds(&excused.run("ci", "performance"));
    let report = fs::read_to_string(excused.root.join("reports/performance.txt")).unwrap();
    assert!(report.contains("EXCUSED PRF-001 tokenizer\n"), "{report}");
    assert!(!excused.calls().contains("--callgrind-limits"));
}

#[test]
fn nothing_is_measured_without_a_bench_a_base_or_the_pinned_gungraun() {
    let mut fixture = declared("0.18.0", 0);
    refused(
        &fixture.run("ci", "performance"),
        "performance: Cargo.lock resolves gungraun 0.18.0; the organization's runner is 0.19.4 \
         (PRF-001)",
    );
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[performance]\nbenches = [\"../x\"]\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "performance"),
        "maestro-quality.toml: [performance] bench `../x` is not a bench name",
    );
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[performance]\nbenches = [\"tokenizer\"]\n",
    )
    .unwrap();
    fixture.set("GITHUB_BASE_REF", "");
    succeeds(&fixture.run("ci", "performance"));
    let report = || fs::read_to_string(fixture.root.join("reports/performance.txt")).unwrap();
    assert_eq!(
        report(),
        "NOT APPLICABLE: a push has no base to compare with\n"
    );
    fs::remove_file(fixture.root.join("maestro-quality.toml")).unwrap();
    succeeds(&fixture.run("ci", "performance"));
    assert_eq!(
        report(),
        "NOT APPLICABLE: maestro-quality.toml names no [performance] benches\n"
    );
}
