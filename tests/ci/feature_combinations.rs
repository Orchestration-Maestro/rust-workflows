//! Real feature checks supplement the no-feature consumer examples.

use crate::harness::{Fixture, GATE_STEPS, succeeds};
use std::fs;

#[test]
fn the_example_replay_includes_the_feature_gate() {
    assert!(GATE_STEPS.contains(&"features"));
}

#[test]
fn real_features_reject_broken_isolated_and_combined_builds() {
    let mut fixture = Fixture::new();
    fixture.set("CARGO_NET_OFFLINE", "true");
    fs::write(
        fixture.root.join("project/Cargo.toml"),
        "[package]\nname = 'features-fixture'\nversion = '0.1.0'\nedition = '2024'\n\
         rust-version = '1.85'\n[features]\ndefault = ['alpha']\nalpha = []\nbeta = []\n",
    )
    .unwrap();
    succeeds(&fixture.run_body(
        "cd \"$PROJECT\"; cargo metadata --format-version 1 --offline \
         > \"$RUNNER_TEMP/metadata.json\"",
    ));
    let result = fixture.run("ci", "features");
    succeeds(&result);
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for selection in [
        "--all-features",
        "--no-default-features",
        "--features alpha",
        "--features beta",
        "--features default",
    ] {
        assert!(log.contains(selection), "{selection}: {log}");
    }
    assert_eq!(
        fs::read_to_string(fixture.root.join("reports/features.txt")).unwrap(),
        "alpha\nbeta\ndefault"
    );
    assert!(
        fs::read_to_string(fixture.root.join("output"))
            .unwrap()
            .contains("applied=true\n")
    );
    fs::remove_file(fixture.root.join("output")).unwrap();
    for condition in [
        "all(feature = \"alpha\", feature = \"beta\")",
        "all(feature = \"beta\", not(feature = \"alpha\"))",
    ] {
        fs::write(
            fixture.root.join("project/src/lib.rs"),
            format!("#[cfg({condition})]\ncompile_error!(\"invalid_feature_selection\");\n"),
        )
        .unwrap();
        let result = fixture.run("ci", "features");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("invalid_feature_selection"));
        assert!(!fixture.root.join("output").exists());
    }
}
