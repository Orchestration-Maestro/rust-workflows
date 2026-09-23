//! `ci.yml`: the function and file sizes, reported to the consumer and never
//! held against the run.

use crate::harness::{Fixture, succeeds};
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
    let mut f = Fixture::new();
    f.set(
        "MESSAGES",
        &format!(
            "{}\n{}\n{{\"reason\":\"build-finished\"}}\n",
            message("clippy::too_many_lines", 140, 100, 3),
            message("clippy::cognitive_complexity", 20, 15, 40)
        ),
    );
    f.stub(
        "cargo",
        r#"printf 'CLIPPY_CONF_DIR=%s\n' "${CLIPPY_CONF_DIR:-}" >> "$CALLS"
printf '%s' "$MESSAGES""#,
    );
    let long = "fn a() {}\n".repeat(301);
    fs::write(f.root.join("project/src/long.rs"), &long).unwrap();
    succeeds(&f.run("ci", "complexity"));
    let report = fs::read_to_string(f.root.join("reports/complexity.txt")).unwrap();
    assert!(
        report.contains("clippy::too_many_lines\t140\t100\tsrc/lib.rs:3"),
        "{report}"
    );
    assert!(report.contains("file\t301\t300\tsrc/long.rs"), "{report}");
    assert!(
        report.contains("thresholds from this workflow's defaults"),
        "{report}"
    );
    let data = fs::read_to_string(f.root.join("reports/complexity.json")).unwrap();
    assert!(
        data.contains("\"functions_over\":2") && data.contains("\"files_over\":1"),
        "{data}"
    );
    assert!(
        f.calls().contains("CLIPPY_CONF_DIR=/"),
        "without a consumer clippy.toml the defaults must be handed to Clippy: {}",
        f.calls()
    );

    // A committed clippy.toml is the consumer's own bar, and is left alone.
    fs::write(
        f.root.join("project/clippy.toml"),
        "too-many-lines-threshold = 50\n",
    )
    .unwrap();
    fs::write(f.root.join("calls"), "").unwrap();
    succeeds(&f.run("ci", "complexity"));
    assert!(f.calls().contains("CLIPPY_CONF_DIR=\n"), "{}", f.calls());
    assert!(
        fs::read_to_string(f.root.join("reports/complexity.txt"))
            .unwrap()
            .contains("thresholds from the consumer's clippy.toml")
    );

    // The scorecard carries the numbers as a line that changes no count.
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
    let scorecard = fs::read_to_string(f.root.join("reports/scorecard.md")).unwrap();
    assert!(
        scorecard
            .contains("Complexity, informational: 2 functions over the size thresholds, 1 files"),
        "{scorecard}"
    );
    assert!(
        fs::read_to_string(f.root.join("reports/scorecard.json"))
            .unwrap()
            .contains("\"complexity\":{\"measured\":true"),
    );

    // Clippy failing is reported, not propagated.
    f.stub("cargo", "exit 1");
    succeeds(&f.run("ci", "complexity"));
    assert!(
        fs::read_to_string(f.root.join("reports/complexity.txt"))
            .unwrap()
            .starts_with("NOT MEASURED")
    );
}
