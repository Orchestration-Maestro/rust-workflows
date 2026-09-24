//! `ci.yml`: functions whose syntax trees look alike. A pair is reported to
//! the consumer; three functions of one shape fail the run, DUP-001, unless
//! `maestro-quality.toml` records why the shape is shared on purpose.

use crate::harness::{Fixture, refused, succeeds};
use std::fmt::Write as _;
use std::fs;

/// A similarity-rs stand-in that prints `pairs`, each two functions.
fn listing(fixture: &Fixture, pairs: &[(&str, &str)]) {
    let mut body = String::from("printf '%s\\n' \"$*\" >> \"$CALLS\"\n");
    for (left, right) in pairs {
        let _ = writeln!(
            body,
            "printf '  {left} <-> {right}\\n  Similarity: 92.00%%\\n'"
        );
    }
    let _ = write!(
        body,
        "printf 'Total duplicate pairs found: {}\\n'",
        pairs.len()
    );
    fixture.stub("similarity-rs", &body);
}

#[test]
fn similar_pairs_are_reported_and_three_alike_are_refused() {
    // Two pairs of four different functions: reported, counted, never refused.
    let fixture = Fixture::new();
    listing(
        &fixture,
        &[
            ("src/lib.rs:10-30 function a", "src/lib.rs:40-60 function b"),
            (
                "src/lib.rs:70-90 function c",
                "src/lib.rs:95-115 function d",
            ),
        ],
    );
    succeeds(&fixture.run("ci", "duplication"));
    let report = fs::read_to_string(fixture.root.join("reports/duplication.txt")).unwrap();
    assert!(report.contains("# pairs: 2"), "{report}");
    assert!(
        report.contains("function a <-> src/lib.rs:40-60 function b"),
        "{report}"
    );
    let calls = fixture.calls();
    assert!(
        calls.contains("--threshold 0.9 --min-lines 8 --exclude target"),
        "{calls}"
    );
    assert!(
        calls.contains("project"),
        "the project directory is what is compared: {calls}"
    );

    // Two pairs joining three functions: one shape, the rule of three.
    listing(
        &fixture,
        &[
            ("src/lib.rs:10-30 function a", "src/lib.rs:40-60 function b"),
            (
                "src/lib.rs:40-60 function b",
                "src/other.rs:5-25 function c",
            ),
        ],
    );
    refused(
        &fixture.run("ci", "duplication"),
        "duplication: 1 finding; each names its rule, its file and what to do",
    );
    let report = fs::read_to_string(fixture.root.join("reports/duplication.txt")).unwrap();
    assert!(
        report.starts_with(
            "DUP-001 src/lib.rs:10: 3 functions share one shape (a, b, c); extract what they \
             share: the third copy is the signal\n"
        ),
        "{report}"
    );

    // A shape shared on purpose, recorded with its reason.
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[[exception]]\nrule = \"DUP-001\"\npath = \"src/lib.rs\"\nitem = \"a\"\n\
         reason = \"three parsers of three formats\"\n",
    )
    .unwrap();
    succeeds(&fixture.run("ci", "duplication"));
    let report = fs::read_to_string(fixture.root.join("reports/duplication.txt")).unwrap();
    assert!(
        report.starts_with("EXCUSED DUP-001 src/lib.rs:10:"),
        "{report}"
    );

    // The tool failing is recorded, not guessed: the report says so and the
    // step exits zero.
    fixture.stub("similarity-rs", "exit 1");
    succeeds(&fixture.run("ci", "duplication"));
    let report = fs::read_to_string(fixture.root.join("reports/duplication.txt")).unwrap();
    assert!(
        report.starts_with("NOT MEASURED: similarity-rs failed"),
        "{report}"
    );
}
