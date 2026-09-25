//! `rust-gate rules` and `rules --check`: a repository's rule map written
//! from the golden rules the release carries, kept where the repository wrote
//! it, and refused when stale or not mapped yet.

use crate::harness::{Fixture, refused, succeeds};
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

#[test]
fn the_check_refuses_every_stale_page_and_counts_unmapped_rows() {
    let fixture = Fixture::new();
    refused(
        &in_project(&fixture, "rust-gate rules --check"),
        "rules --check: stale docs/standards/northstar.md, docs/standards/engineering.md, \
         docs/standards/security.md; 21 rows not mapped yet; run rust-gate rules, then map \
         every row",
    );
    succeeds(&in_project(&fixture, "rust-gate rules"));
    refused(
        &in_project(&fixture, "rust-gate rules --check"),
        "rules --check: stale none; 21 rows not mapped yet",
    );
}

#[test]
fn a_mapped_row_survives_and_the_check_passes_once_all_are_mapped() {
    let fixture = Fixture::new();
    succeeds(&in_project(&fixture, "rust-gate rules"));
    for path in PAGES {
        let file = fixture.root.join("project").join(path);
        let mapped = fs::read_to_string(&file)
            .unwrap()
            .split("\n\n")
            .map(|paragraph| {
                if paragraph.starts_with("Not mapped yet:") {
                    "Filled in by a test."
                } else {
                    paragraph
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
            .replace("| Not mapped yet |", "| Held by a test |");
        fs::write(&file, mapped).unwrap();
    }
    succeeds(&in_project(&fixture, "rust-gate rules"));
    let written = pages(&fixture);
    assert!(written[0].contains("| ENF-001 No machine-named paths | Held by a test |\n"));
    assert!(written[1].contains("## The point\n\nFilled in by a test.\n"));
    succeeds(&in_project(&fixture, "rust-gate rules --check"));
}
