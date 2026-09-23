//! Proof coverage: every step the gate declares is run by a contract test,
//! and every refusal it can print is asserted by a test somewhere, so no step
//! and no refusal is held by nothing.

use crate::harness::{described, root, test_sources};
use std::fs;
use std::path::PathBuf;

/// Every Rust file of the gate crate.
fn product_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut queue = vec![root().join("gate/src")];
    while let Some(directory) = queue.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                queue.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Everything the tests assert against, as one string: the contract tests
/// and the unit tests inside the gate crate's own modules.
fn assertions() -> String {
    let mut text: String = test_sources()
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    for file in product_sources() {
        if let Some((_, tests)) = fs::read_to_string(file).unwrap().split_once("#[cfg(test)]") {
            text.push_str(tests);
        }
    }
    // A string continued over lines with `\` reads as one line, the way the
    // compiler reads it.
    text.split("\\\n")
        .enumerate()
        .map(|(index, part)| if index == 0 { part } else { part.trim_start() })
        .collect()
}

/// The span of balanced parentheses opening at `start`, string literals
/// skipped, or the rest of the text when it never closes.
fn span(text: &str, start: usize) -> &str {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, c) in text[start..].char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return &text[start..start + offset];
                }
            }
            _ => {}
        }
    }
    &text[start..]
}

/// The body of the string literal `rest` opens, up to its closing quote.
fn literal(rest: &str) -> &str {
    let mut escaped = false;
    for (offset, c) in rest.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return &rest[..offset],
            _ => {}
        }
    }
    rest
}

/// The longest fixed fragment of a string literal once its `{placeholders}`
/// and line continuations are removed.
fn fixed_fragment(text: &str) -> String {
    let joined: String = text
        .split("\\\n")
        .enumerate()
        .map(|(index, part)| if index == 0 { part } else { part.trim_start() })
        .collect();
    let unescaped = joined.replace("\\\"", "\"").replace("\\n", "\n");
    let mut fragments = Vec::new();
    let mut rest = unescaped.as_str();
    while let Some(open) = rest.find('{') {
        fragments.push(&rest[..open]);
        rest = rest[open..].split_once('}').map_or("", |(_, after)| after);
    }
    fragments.push(rest);
    fragments
        .iter()
        .map(|fragment| fragment.trim())
        .max_by_key(|fragment| fragment.len())
        .unwrap_or_default()
        .to_owned()
}

/// Every refusal a source file can print: the string literals inside its
/// `Err(`, `Failure::from(`, `map_err(`, `ok_or(` and `ok_or_else(` spans,
/// outside comments and its own tests, as their longest fixed fragment. A
/// message that relays an operating system error, `{error}` in its text, is
/// the system's wording and is left out.
fn refusals(source: &str) -> Vec<String> {
    let body: Vec<&str> = source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    let body = body.join("\n");
    let mut found = Vec::new();
    for marker in [
        "Err(",
        "Failure::from(",
        ".map_err(",
        ".ok_or(",
        ".ok_or_else(",
    ] {
        for (index, _) in body.match_indices(marker) {
            let preceded = body[..index].chars().next_back();
            if marker == "Err(" && preceded.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let mut rest = span(&body, index + marker.len() - 1);
            while let Some(open) = rest.find('"') {
                let text = literal(&rest[open + 1..]);
                let fragment = fixed_fragment(text);
                if fragment.len() >= 12 && !text.contains("{error}") {
                    found.push(fragment);
                }
                rest = &rest[(open + 1 + text.len() + 1).min(rest.len())..];
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn a_test_asserts_every_refusal_the_gate_can_print() {
    // A refusal is part of the interface: the consumer reads it. One that no
    // test asserts can change meaning, or vanish, without anything failing.
    let asserted = assertions();
    let root = root();
    let mut missing = Vec::new();
    let mut checked = 0;
    for file in product_sources() {
        let source = fs::read_to_string(&file).unwrap();
        for fragment in refusals(&source) {
            checked += 1;
            if !asserted.contains(&fragment) {
                let name = file.strip_prefix(&root).unwrap().display().to_string();
                missing.push(format!("{name}: {fragment:?}"));
            }
        }
    }
    assert!(
        checked >= 40,
        "only {checked} refusals found; the scan drifted"
    );
    assert!(
        missing.is_empty(),
        "refusals no test asserts:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn a_contract_test_runs_every_registered_step() {
    // A step the tests never run is held by nothing but its own reading.
    let tests: String = test_sources()
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    let mut missing = Vec::new();
    for step in described() {
        let body = if matches!(step.workflow.as_str(), "ci" | "shared") {
            format!("rust-gate {}", step.id)
        } else {
            format!("rust-gate {} {}", step.workflow, step.id)
        };
        // Every way a test names a step ends the same way: the id closes
        // the call, `run("ci", "validate")`, `run(name, "authorize")` or a
        // table row such as `("cargo", "audit")`; a body runs by its text.
        let closing = format!(", \"{}\")", step.id);
        if !(tests.contains(&closing) || tests.contains(&body)) {
            missing.push(format!("{} {}", step.workflow, step.id));
        }
    }
    assert!(
        missing.is_empty(),
        "steps no contract test runs: {}",
        missing.join(", ")
    );
}
