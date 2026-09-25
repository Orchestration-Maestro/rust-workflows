//! `ci.yml`: the function and file sizes, reported to the consumer and never
//! held against the run.

use crate::harness::{Fixture, SCORECARD_OUTCOMES, succeeds};
use serde_json::json;
use std::fs;

#[test]
fn complexity_is_reported_but_never_fails_the_run() {
    // Sizes are information for the consumer, never a gate: the step counts
    // what Clippy reports over the thresholds and what files exceed 300 lines
    // of code, writes both, and exits zero even when Clippy itself fails.
    let message = |lint: &str, value: u64, limit: u64, line: u64| {
        json!({"reason": "compiler-message", "message": {
            "code": {"code": lint}, "message": format!("too much ({value}/{limit})"),
            "spans": [{"file_name": "src/lib.rs", "line_start": line, "is_primary": true}]}})
        .to_string()
    };
    let mut fixture = Fixture::new();
    fixture.set(
        "MESSAGES",
        &format!(
            "{}\n{}\n{{\"reason\":\"build-finished\"}}\n",
            message("clippy::too_many_lines", 140, 100, 3),
            message("clippy::cognitive_complexity", 20, 15, 40)
        ),
    );
    fixture.stub(
        "cargo",
        r#"printf 'CLIPPY_CONF_DIR=%s\n' "${CLIPPY_CONF_DIR:-}" >> "$CALLS"
printf '%s' "$MESSAGES""#,
    );
    let long = "fn a() {}\n".repeat(301);
    fs::write(fixture.root.join("project/src/long.rs"), &long).unwrap();
    succeeds(&fixture.run("ci", "complexity"));
    let report = fs::read_to_string(fixture.root.join("reports/complexity.txt")).unwrap();
    assert!(
        report.contains("clippy::too_many_lines\t140\t100\tsrc/lib.rs:3"),
        "{report}"
    );
    assert!(report.contains("file\t301\t300\tsrc/long.rs"), "{report}");
    assert!(
        report.contains("thresholds from the organization's clippy.toml"),
        "{report}"
    );
    let data = fs::read_to_string(fixture.root.join("reports/complexity.json")).unwrap();
    assert!(
        data.contains("\"functions_over\":2") && data.contains("\"files_over\":1"),
        "{data}"
    );
    assert!(
        fixture.calls().contains("CLIPPY_CONF_DIR=/"),
        "without a consumer clippy.toml the organization's must be handed to Clippy: {}",
        fixture.calls()
    );

    // A committed clippy.toml is the consumer's own bar, and is left alone.
    fs::write(
        fixture.root.join("project/clippy.toml"),
        "too-many-lines-threshold = 50\n",
    )
    .unwrap();
    fs::write(fixture.root.join("calls"), "").unwrap();
    succeeds(&fixture.run("ci", "complexity"));
    assert!(
        fixture.calls().contains("CLIPPY_CONF_DIR=\n"),
        "{}",
        fixture.calls()
    );
    assert!(
        fs::read_to_string(fixture.root.join("reports/complexity.txt"))
            .unwrap()
            .contains("thresholds from the consumer's clippy.toml")
    );

    // The scorecard carries the numbers as a line that changes no count.
    for key in SCORECARD_OUTCOMES {
        fixture.set(key, "success");
    }
    succeeds(&fixture.run("ci", "scorecard"));
    let scorecard = fs::read_to_string(fixture.root.join("reports/scorecard.md")).unwrap();
    assert!(
        scorecard
            .contains("Complexity, informational: 2 functions over the size thresholds, 1 files"),
        "{scorecard}"
    );
    assert!(
        fs::read_to_string(fixture.root.join("reports/scorecard.json"))
            .unwrap()
            .contains("\"complexity\":{\"measured\":true"),
    );

    // Clippy failing is reported, not propagated.
    fixture.stub("cargo", "exit 1");
    succeeds(&fixture.run("ci", "complexity"));
    assert!(
        fs::read_to_string(fixture.root.join("reports/complexity.txt"))
            .unwrap()
            .starts_with("NOT MEASURED")
    );
}
