//! The gate's three layers, and the one door of each: imports flow from the
//! steps to the checks to the runner, never sideways and never upward, and
//! the contract tests reach the gate's source only through the harness.

use crate::harness::{root, test_sources};
use std::fs;
use std::path::{Path, PathBuf};

/// The text following every `marker` that begins a path rather than ending an
/// identifier. The two rules below share this scan and cut what they need from
/// each tail: one reads a whole `use` statement, the other every mention
/// wherever it sits, so the scan itself must consume neither.
fn tails_after<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    text.match_indices(marker)
        .filter(|(index, _)| {
            text[..*index]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
        })
        .map(|(index, _)| &text[index + marker.len()..])
        .collect()
}

/// Every `crate::<module>` a source text names, outside its test module. Two
/// mentions in one statement are two targets, so this keeps the first segment
/// of each tail and never reads to the end of a statement.
fn crate_targets(text: &str) -> Vec<String> {
    let body = text.split("#[cfg(test)]").next().unwrap_or_default();
    tails_after(body, "crate::")
        .into_iter()
        .map(|tail| {
            tail.split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .filter(|target| !target.is_empty())
        .collect()
}

/// Every Rust file of a layer, keyed by its path below that layer, so a
/// module nested inside a step is held to the same rules as one beside it.
fn layer_files(layer: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let mut queue = vec![layer.to_path_buf()];
    while let Some(current) = queue.pop() {
        for entry in fs::read_dir(&current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                queue.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push((
                    path.strip_prefix(layer).unwrap().display().to_string(),
                    path,
                ));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn imports_flow_one_way_through_the_layer_gates() {
    // A step reaches the runner and the checks, never another step; a check
    // reaches the runner; the runner reaches nothing. Each layer is entered
    // through its own mod.rs, and a step's only door is its declaration,
    // `STEPS`. The walk recurses, so a module nested inside a step is held to
    // the same rules and cannot slip past them by sitting one directory down.
    let src = root().join("gate/src");
    let mut checked = 0;
    for (layer, upstream) in [
        ("runner", &[][..]),
        ("checks", &["runner"][..]),
        ("steps", &["runner", "checks"][..]),
    ] {
        for (name, path) in layer_files(&src.join(layer)) {
            let text = fs::read_to_string(&path).unwrap();
            for target in crate_targets(&text) {
                assert!(
                    upstream.contains(&target.as_str()),
                    "{layer}/{name} imports crate::{target}; a {layer} module may reach \
                     only {upstream:?}"
                );
                checked += 1;
            }
            if layer != "steps" || name == "mod.rs" {
                continue;
            }
            let body = text.split("#[cfg(test)]").next().unwrap_or_default();
            // A file inside a step's directory is that step's internal seam: it
            // splits an implementation without offering the crate a name, which
            // is what a file split for its size used to do from a shared layer.
            if name.contains('/') && !name.ends_with("/mod.rs") {
                assert!(
                    !body.contains("pub(crate) "),
                    "{name} is a step's internal seam; pub(super) is as far as it reaches"
                );
                continue;
            }
            assert!(
                !body.contains("super::"),
                "{name} reaches a sibling step through super::"
            );
            for line in body.lines().filter(|line| line.starts_with("pub(crate) ")) {
                assert!(
                    line.starts_with("pub(crate) const STEPS:"),
                    "{name} exposes {line:?}; a step's only door is STEPS"
                );
            }
        }
    }
    let main = fs::read_to_string(src.join("main.rs")).unwrap();
    assert!(!main.contains("crate::"), "main.rs reaches into a layer");
    assert!(
        checked > 40,
        "only {checked} imports checked; the scan drifted"
    );
}

#[test]
fn modules_of_the_tests_reach_the_gate_only_through_the_harness() {
    // The harness is the one door into workflow YAML, the gate and the
    // fixture; a test module that imported anything else would be a second
    // reader to keep in step with the first, one that imported the whole
    // harness would hide which readers it depends on, and a harness part that
    // named a test module would open the door from the inside.
    let mut checked = 0;
    for path in test_sources() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap();
        if path
            .parent()
            .is_some_and(|parent| parent.ends_with("harness"))
        {
            let outside = crate_targets(&text);
            assert!(
                outside.is_empty(),
                "harness/{name} reaches crate::{}",
                outside[0]
            );
            continue;
        }
        if name == "workflows.rs" {
            continue;
        }
        assert!(
            !text.contains(concat!("use crate::harness::", "*")),
            "{name} imports the whole harness; name what it uses"
        );
        for target in crate_targets(&text) {
            assert_eq!(target, "harness", "{name} imports crate::{target}");
            checked += 1;
        }
    }
    assert!(
        checked > 15,
        "only {checked} imports checked; the scan drifted"
    );
}

#[test]
fn every_check_and_step_seam_with_a_function_has_unit_tests() {
    // A check is a rule several steps trust; a rule without a unit test is
    // held only by whichever contract test happens to cross it.
    let mut checked = 0;
    let mut modules = layer_files(&root().join("gate/src/checks"));
    modules.extend(
        layer_files(&root().join("gate/src/steps"))
            .into_iter()
            .filter(|(name, _)| name.contains('/') && !name.ends_with("/mod.rs")),
    );
    for (name, path) in modules {
        if name == "mod.rs" {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let (body, tests) = text.split_once("#[cfg(test)]").unwrap_or((&text, ""));
        if !body.contains("fn ") {
            continue;
        }
        assert!(
            tests.contains("#[test]"),
            "{name} defines functions and has no unit test"
        );
        checked += 1;
    }
    assert!(
        checked >= 4,
        "only {checked} checks modules scanned; the walk drifted"
    );
}

#[test]
fn the_checks_door_offers_modules_and_not_their_names() {
    // The runner's door names a vocabulary every step speaks, each name used
    // by many of them. The checks door held twenty-four names drawn from seven
    // unrelated concerns, so a call site named a rule without saying what kind
    // of rule it was. It offers the modules instead, and the import says it.
    let door = fs::read_to_string(root().join("gate/src/checks/mod.rs")).unwrap();
    let offered: Vec<&str> = door
        .lines()
        .filter(|line| line.starts_with("pub(crate) "))
        .collect();
    assert!(
        offered.len() >= 5,
        "only {} names offered; the door drifted",
        offered.len()
    );
    for line in offered {
        assert!(
            line.starts_with("pub(crate) mod "),
            "the checks door offers {line:?}; it offers modules, not their names"
        );
    }
}

#[test]
fn every_harness_door_item_serves_a_module_outside_it() {
    // Two callers make a seam real at the gate's own doors, where a name with
    // one caller can move into it. Most of the harness cannot move: fourteen of
    // its items serve the harness itself as well, so descending into a caller
    // is not an action this rule could ask for. What still holds is the tighter
    // half. An item no test outside the harness names is a door open on
    // nothing, and the last caller of an item can disappear without the door
    // noticing.
    let door = fs::read_to_string(root().join("tests/harness/mod.rs")).unwrap();
    let items = offered(&door);
    assert!(
        items.len() >= 10,
        "only {} items at the harness door; the scan drifted",
        items.len()
    );
    let outside: Vec<String> = test_sources()
        .into_iter()
        .filter(|path| {
            !path
                .parent()
                .is_some_and(|parent| parent.ends_with("harness"))
        })
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    for item in items {
        let users = outside
            .iter()
            .filter(|text| imported(text, "harness").contains(&item))
            .count();
        assert!(
            users >= 1,
            "harness::{item} is named by no test outside the harness; take it off \
             the door, or delete it if nothing needs it at all"
        );
    }
}

/// A door item only one module outside its layer uses. Two callers make a seam
/// real; one makes it hypothetical, so an entry here is a decision with the
/// reason it stays, never a backlog item.
const HYPOTHETICAL_SEAMS: &[(&str, &str)] = &[
    (
        "enter",
        "the step registry is the only thing that can run a step",
    ),
    (
        "add_to_path",
        "the fourth GITHUB_* writer, beside export, output and summary",
    ),
];

/// What a layer's door offers: the names it re-exports one by one, and the
/// modules it offers whole. Both shapes are read the same way here.
fn offered(door: &str) -> Vec<String> {
    let mut items: Vec<String> = door
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub(crate) mod "))
        .map(|rest| rest.trim_end_matches(';').to_owned())
        .collect();
    let mut rest = door;
    while let Some(at) = rest.find("pub(crate) use ") {
        let tail = &rest[at + "pub(crate) use ".len()..];
        let end = tail.find(';').unwrap_or(tail.len());
        let names = tail[..end].split_once("::").map_or("", |(_, names)| names);
        items.extend(
            names
                .replace(['{', '}', '\n'], " ")
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
        );
        rest = &tail[end..];
    }
    items.sort();
    items.dedup();
    items
}

/// The door items one file imports: the first segment of a module path, or
/// every name of a braced list. Read from the imports, so a mention in a
/// comment or a string is not a caller.
fn imported(text: &str, layer: &str) -> Vec<String> {
    let marker = format!("use crate::{layer}::");
    let mut names = Vec::new();
    for tail in tails_after(text, &marker) {
        let statement = &tail[..tail.find(';').unwrap_or(tail.len())];
        if let Some((module, _)) = statement.split_once("::") {
            names.push(module.trim().to_owned());
        } else {
            names.extend(
                statement
                    .replace(['{', '}', '\n'], " ")
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    names
}

#[test]
fn every_door_item_serves_two_modules_or_names_its_reason() {
    // Depth is leverage at the interface: a name the door offers for a single
    // caller is a seam nothing varies across, and the split that produced it
    // was chosen by something other than what changes. The rule is mechanical,
    // so the ledger above is where judgement has to be written down.
    let root = root();
    let sources: Vec<(String, String)> = layer_files(&root.join("gate/src"))
        .into_iter()
        .map(|(name, path)| (name, fs::read_to_string(path).unwrap()))
        .collect();
    for layer in ["runner", "checks"] {
        let door = fs::read_to_string(root.join(format!("gate/src/{layer}/mod.rs"))).unwrap();
        let items = offered(&door);
        assert!(!items.is_empty(), "the {layer} door offers nothing");
        for item in items {
            let users = sources
                .iter()
                .filter(|(name, _)| !name.starts_with(&format!("{layer}/")))
                .filter(|(_, text)| imported(text, layer).contains(&item))
                .count();
            let listed = HYPOTHETICAL_SEAMS.iter().any(|(name, _)| *name == item);
            if users >= 2 {
                assert!(
                    !listed,
                    "{layer}::{item} has {users} callers and is still listed as a \
                     hypothetical seam; take its entry out"
                );
            } else {
                assert!(
                    listed,
                    "{layer}::{item} is used by {users} module(s); two callers make a \
                     seam real, so move it into its caller or name the reason it stays"
                );
            }
        }
    }
}
