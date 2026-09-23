//! `ci.yml`: functions whose syntax trees look alike, reported to the consumer
//! and never held against the run.

use crate::harness::{Fixture, succeeds};
use std::fs;

#[test]
fn duplicated_functions_are_reported_but_never_fail_the_run() {
    // A listing with two pairs lands in the report as written, counted once
    // per pair, and the run goes on whatever the count.
    let f = Fixture::new();
    f.stub(
        "similarity-rs",
        r#"printf '%s\n' "$*" >> "$CALLS"
printf 'Duplicates in src/lib.rs:\n'
printf '  src/lib.rs:10-30 function a <-> src/lib.rs:40-60 function b\n'
printf '  Similarity: 92.00%%\n'
printf '  src/lib.rs:70-90 function c <-> src/lib.rs:95-115 function d\n'
printf '  Similarity: 90.50%%\n'
printf 'Total duplicate pairs found: 2\n'"#,
    );
    succeeds(&f.run("ci", "duplication"));
    let report = fs::read_to_string(f.root.join("reports/duplication.txt")).unwrap();
    assert!(report.contains("# pairs: 2"), "{report}");
    assert!(
        report.contains("function a <-> src/lib.rs:40-60 function b"),
        "{report}"
    );
    let calls = f.calls();
    assert!(
        calls.contains("--threshold 0.9 --min-lines 8 --exclude target"),
        "{calls}"
    );
    assert!(
        calls.contains("project"),
        "the project directory is what is compared: {calls}"
    );

    // The tool failing is recorded, not fatal: the report says so and the
    // step still exits zero.
    f.stub("similarity-rs", "exit 1");
    succeeds(&f.run("ci", "duplication"));
    let report = fs::read_to_string(f.root.join("reports/duplication.txt")).unwrap();
    assert!(
        report.starts_with("NOT MEASURED: similarity-rs failed"),
        "{report}"
    );
}
