//! `rust-gate rules` and `rules --check`: a repository's rule map written
//! from the golden rules the release carries, kept where the repository wrote
//! it, and refused when stale or not mapped yet.

use crate::harness::{Fixture, succeeds};
use std::fs;
use std::process::Output;

/// The three pages of the rule map.
const PAGES: [&str; 3] = [
    "docs/standards/engineering.md",
    "docs/standards/northstar.md",
    "docs/standards/security.md",
];

/// Run `command` in the fixture's project.
fn in_project(fixture: &Fixture, command: &str) -> Output {
    fixture.run_body(&format!("cd project && {command}"))
}

/// Every page of the fixture's rule map, in `PAGES` order.
fn pages(fixture: &Fixture) -> Vec<String> {
    PAGES
        .iter()
        .map(|path| fs::read_to_string(fixture.root.join("project").join(path)).unwrap())
        .collect()
}

#[test]
fn rules_writes_three_pages_and_a_second_run_changes_nothing() {
    let fixture = Fixture::new();
    succeeds(&in_project(
        &fixture,
        "git init -q && git remote add origin \
         https://github.com/Orchestration-Maestro/example.git && rust-gate rules",
    ));
    let first = pages(&fixture);
    assert!(first[0].starts_with("# Engineering rules in `example`\n"));
    assert!(first[1].starts_with("# Northstar for `example`\n\n> Automate the guardrails"));
    assert!(first[2].contains("| SEC-011 Sign every release | Not mapped yet |\n"));
    succeeds(&in_project(&fixture, "rust-gate rules"));
    assert_eq!(first, pages(&fixture));
}
