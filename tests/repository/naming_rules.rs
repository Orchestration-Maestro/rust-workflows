//! The names of the tests: a module under `tests/` names what it proves in
//! two words at least, a test function in four, and neither hides behind a
//! `test_` prefix or a `_works`, `_ok` or `_test` suffix.

use crate::harness::{root, test_sources};
use std::fs;
use std::path::{Path, PathBuf};

/// The words of a snake-case name, or nothing for a name that is not one.
fn words(name: &str) -> Option<Vec<&str>> {
    let words: Vec<&str> = name.split('_').collect();
    let snake = name.starts_with(|c: char| c.is_ascii_lowercase())
        && words.iter().all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    snake.then_some(words)
}

/// Every `#[test]` function a source text declares, attributes between the
/// marker and the function skipped.
fn test_functions(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut marked = false;
    for line in text.lines().map(str::trim_start) {
        if line == "#[test]" {
            marked = true;
        } else if marked && !line.starts_with("#[") {
            marked = false;
            if let Some(rest) = line.strip_prefix("fn ") {
                found.extend(
                    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next(),
                );
            }
        }
    }
    found
}

/// Every Rust file of the gate's crate, so its unit tests are held too.
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

/// A test module: any file of the tests but the crate root, a directory's
/// `mod.rs` and the harness, which are things and not claims.
fn is_test_module(path: &Path) -> bool {
    path != root().join("tests/workflows.rs")
        && path.file_name().is_some_and(|name| name != "mod.rs")
        && !path
            .parent()
            .is_some_and(|parent| parent.ends_with("harness"))
}

#[test]
fn every_test_module_names_what_it_proves_in_two_words_at_least() {
    // A module is named after the claim it holds, `quality_gates`,
    // `payload_verification`; one word, `quality`, names a topic and leaves
    // the reader to open the file. The crate root, a directory's `mod.rs`
    // and the harness are things, not claims, and keep their one word.
    let root = root();
    let mut checked = 0;
    for path in test_sources().iter().filter(|path| is_test_module(path)) {
        let stem = path.file_stem().unwrap().to_string_lossy();
        assert!(
            words(&stem).is_some_and(|words| words.len() >= 2),
            "{}: a test module names what it proves in two words at least",
            path.strip_prefix(&root).unwrap().display()
        );
        checked += 1;
    }
    assert!(
        checked > 30,
        "only {checked} test modules checked; the walk drifted"
    );
}

#[test]
fn every_test_function_names_what_it_proves_in_four_words_at_least() {
    // The name is the claim the test makes, read whole in a failure report:
    // three words state a topic, four state a claim.
    let root = root();
    let mut checked = 0;
    for path in test_sources().into_iter().chain(product_sources()) {
        let text = fs::read_to_string(&path).unwrap();
        for name in test_functions(&text) {
            assert!(
                words(name).is_some_and(|words| words.len() >= 4),
                "{}: {name} names what it proves in fewer than four words",
                path.strip_prefix(&root).unwrap().display()
            );
            checked += 1;
        }
    }
    assert!(
        checked > 100,
        "only {checked} test functions checked; the walk drifted"
    );
}

#[test]
fn no_test_function_is_named_by_a_test_prefix_or_a_works_ok_or_test_suffix() {
    // `test_` in front repeats the attribute; `_works`, `_ok` and `_test`
    // behind say nothing about what was proven.
    let root = root();
    let mut checked = 0;
    for path in test_sources().into_iter().chain(product_sources()) {
        let text = fs::read_to_string(&path).unwrap();
        for name in test_functions(&text) {
            let shown = path.strip_prefix(&root).unwrap().display();
            assert!(
                !name.starts_with("test_"),
                "{shown}: {name} starts with test_; the attribute already says so"
            );
            for suffix in ["_works", "_ok", "_test"] {
                assert!(
                    !name.ends_with(suffix),
                    "{shown}: {name} ends with {suffix}; name what was proven"
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked > 100,
        "only {checked} test functions checked; the walk drifted"
    );
}
