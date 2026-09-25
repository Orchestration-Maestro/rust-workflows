//! The gate's list of rules is their one source: `rust-gate gate-rules`
//! prints it, every rule the gate's code names is on it, and every rule on it
//! has its row in `docs/ci.md`, where no table names a rule it lacks.

use crate::harness::{gate_bin, root, rust_files, succeeds};
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

/// The golden rules' families: the gate names them in the rule map it writes,
/// and they are the organization's rules, not the gate's.
const GOLDEN: [&str; 3] = ["ENF", "FND", "SEC"];

/// Whether `text` is a rule's ID: three or four capitals, a dash and three
/// digits.
fn is_rule_id(text: &str) -> bool {
    text.split_once('-').is_some_and(|(family, number)| {
        (3..=4).contains(&family.len())
            && family.bytes().all(|byte| byte.is_ascii_uppercase())
            && number.len() == 3
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[test]
fn every_rule_the_gate_names_is_listed_and_has_its_documented_row() {
    let output = Command::new(gate_bin().join("rust-gate"))
        .arg("gate-rules")
        .output()
        .unwrap();
    succeeds(&output);
    let listed: BTreeSet<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| line.split('\t').next().unwrap_or_default().to_owned())
        .collect();
    assert!(listed.len() > 30, "only {} rules listed", listed.len());
    // A rule the gate's code refuses by ID, a literal, is on the list.
    for source in rust_files(&root().join("gate/src")) {
        let text = fs::read_to_string(&source).unwrap();
        for quoted in text.split('"').filter(|piece| is_rule_id(piece)) {
            let family = quoted.split('-').next().unwrap_or_default();
            assert!(
                GOLDEN.contains(&family) || listed.contains(quoted),
                "{} names {quoted}, which gate_rules.tsv lacks",
                source.display()
            );
        }
    }
    // Every rule has its row in docs/ci.md, and every row names a listed rule.
    let page = fs::read_to_string(root().join("docs/ci.md")).unwrap();
    let rows: BTreeSet<String> = page
        .lines()
        .filter_map(|line| line.strip_prefix("| ")?.split(' ').next())
        .filter(|cell| is_rule_id(cell))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        rows.difference(&listed).collect::<Vec<_>>(),
        Vec::<&String>::new()
    );
    assert_eq!(
        listed.difference(&rows).collect::<Vec<_>>(),
        Vec::<&String>::new()
    );
}
