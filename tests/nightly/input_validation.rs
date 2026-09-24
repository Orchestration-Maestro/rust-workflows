//! The nightly workflows, `unsafe-audit.yml` and `fuzz.yml`: every malformed
//! input refused before a toolchain is touched, and a fuzz project without a
//! corpus refused too.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

#[test]
fn nightly_workflows_reject_every_malformed_input_before_touching_a_toolchain() {
    // Miri and fuzzing run on nightly, outside the stable versions ci.yml
    // accepts, and both install a toolchain as their first side effect.
    // Whatever they refuse has to be refused before that happens, not after.
    let simple = "working-directory must be a simple relative path";
    let traverse = "working-directory must not traverse or contain option-like components";
    let nightly = "toolchain must be nightly or nightly-YYYY-MM-DD";
    let whole = "max-total-time must be a whole number of seconds";
    let range = "max-total-time must be between 10 and 1800 seconds";
    let target = "target must be a simple fuzz target name";
    for (workflow, ok, cases) in [
        (
            "unsafe-audit",
            vec![
                ("DIRECTORY", "project"),
                ("TOOLCHAIN", "nightly-2026-09-14"),
                ("TEST_FILTER", ""),
                ("STRICT_PROVENANCE", "true"),
            ],
            vec![
                ("DIRECTORY", "/absolute", simple),
                ("DIRECTORY", "../escape", simple),
                ("DIRECTORY", "--option-like", simple),
                ("DIRECTORY", "project/../..", traverse),
                ("TOOLCHAIN", "stable", nightly),
                ("TOOLCHAIN", "1.98.1", nightly),
                ("TOOLCHAIN", "nightly-2026-9-14", nightly),
                (
                    "TEST_FILTER",
                    "a; rm -rf /",
                    "test-filter must be a simple test path",
                ),
            ],
        ),
        (
            "fuzz",
            vec![
                ("DIRECTORY", "project"),
                ("TOOLCHAIN", "nightly"),
                ("TARGET", ""),
                ("MAX_TOTAL_TIME", "120"),
            ],
            vec![
                ("DIRECTORY", "/absolute", simple),
                ("DIRECTORY", "../escape", simple),
                ("TOOLCHAIN", "beta", nightly),
                ("TARGET", "a b", target),
                ("TARGET", "-flag", target),
                ("MAX_TOTAL_TIME", "9", range),
                ("MAX_TOTAL_TIME", "1801", range),
                ("MAX_TOTAL_TIME", "12.5", whole),
                ("MAX_TOTAL_TIME", "abc", whole),
            ],
        ),
    ] {
        // fuzz refuses a project with no corpus, so the accepted case needs one.
        let prepare = |fixture: &Fixture| {
            fs::create_dir_all(fixture.root.join("project/fuzz/fuzz_targets")).unwrap();
        };
        let mut good = Fixture::new();
        prepare(&good);
        for (key, value) in &ok {
            good.set(key, value);
        }
        succeeds(&good.run(workflow, "validate"));
        for (key, bad, message) in cases {
            let mut fixture = Fixture::new();
            prepare(&fixture);
            for (key, value) in &ok {
                fixture.set(key, value);
            }
            fixture.set(key, bad);
            refused(&fixture.run(workflow, "validate"), message);
        }
    }

    // A missing corpus is an error, not a skipped run: enabling the workflow
    // and fuzzing nothing would report success while proving nothing.
    let mut empty = Fixture::new();
    for (key, value) in [
        ("DIRECTORY", "project"),
        ("TOOLCHAIN", "nightly"),
        ("TARGET", ""),
        ("MAX_TOTAL_TIME", "120"),
    ] {
        empty.set(key, value);
    }
    refused(
        &empty.run("fuzz", "validate"),
        "No fuzz/fuzz_targets directory; run cargo fuzz init before enabling this workflow",
    );
}
