//! Diagnostic reports survive failed tools without changing their verdicts.

use crate::harness::{Fixture, refused, succeeds};
use serde_json::Value;
use std::fs;

#[test]
fn successful_nextest_cannot_omit_or_empty_its_junit_report() {
    for mode in ["missing", "empty"] {
        let mut f = Fixture::new();
        f.set("REPORT_MODE", mode);
        succeeds(&f.run_body(
            "cd \"$PROJECT\"; cargo metadata --format-version 1 --offline \
             > \"$RUNNER_TEMP/source-metadata.json\"",
        ));
        f.stub(
            "cargo",
            r#"case "$1" in
metadata) cat "$RUNNER_TEMP/source-metadata.json" ;;
nextest) [[ "$REPORT_MODE" != empty ]] || : > "$REPORTS/tests.xml" ;;
esac
exit 0"#,
        );
        refused(
            &f.run("ci", "quality"),
            "cargo-nextest produced no JUnit report",
        );
    }
}

#[test]
fn real_nextest_preserves_junit_for_passed_and_failed_tests() {
    let mut f = Fixture::new();
    fs::write(
        f.root.join("project/src/lib.rs"),
        "//! Fixture.\n#[test]\nfn an_expected_variable_must_exist() {\n\
         \x20   assert!(std::env::var_os(\"EXPECTED_RUN\").is_some());\n}\n",
    )
    .unwrap();
    succeeds(&f.run_body("cd \"$PROJECT\"; cargo generate-lockfile --offline"));
    f.set("EXPECTED_RUN", "yes");
    succeeds(&f.run("ci", "quality"));
    let report = f.root.join("reports/tests.xml");
    let passed = fs::read_to_string(&report).unwrap();
    assert!(
        passed.contains("<testsuites") && passed.contains("<testcase"),
        "{passed}"
    );
    assert!(!passed.contains("<failure"));
    fs::remove_file(&report).unwrap();
    f.env.remove("EXPECTED_RUN");
    assert!(!f.run("ci", "quality").status.success());
    let failed = fs::read_to_string(report).unwrap();
    assert!(failed.contains("<failure"), "{failed}");
}

#[test]
fn a_real_clippy_failure_keeps_json_sarif_and_its_status() {
    for broken_converter in [false, true] {
        let mut f = Fixture::new();
        f.set("SARIF_REPORTS", "true");
        fs::write(
            f.root.join("project/src/lib.rs"),
            "//! Fixture.\n/// A deliberate Clippy finding.\n\
             pub fn answer() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();
        succeeds(&f.run_body("cd \"$PROJECT\"; cargo generate-lockfile --offline"));
        if broken_converter {
            f.stub("clippy-sarif", "exit 9");
        }
        let result = f.run("ci", "quality");
        assert_eq!(result.status.code(), Some(101));
        let report = f.root.join("reports/clippy.json");
        assert!(
            report.is_file(),
            "Clippy diagnostics must survive its failure"
        );
        let messages: Vec<Value> = fs::read_to_string(report)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(
            messages
                .iter()
                .any(|message| message["message"]["code"]["code"] == "clippy::eq_op")
        );
        if !broken_converter {
            let sarif: Value = serde_json::from_str(
                &fs::read_to_string(f.root.join("reports/clippy.sarif")).unwrap(),
            )
            .unwrap();
            assert!(!sarif["runs"][0]["results"].as_array().unwrap().is_empty());
        }
        let trace = f.trace();
        assert_eq!(
            trace
                .lines()
                .filter(|line| line.starts_with("cargo clippy "))
                .count(),
            1
        );
        assert!(
            !trace.contains("cargo nextest"),
            "failed lint must still stop the tests"
        );
    }
}
