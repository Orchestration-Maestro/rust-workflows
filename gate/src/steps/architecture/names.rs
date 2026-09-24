//! NAME-001 and NAME-002: a package is named in lowercase kebab-case, and a
//! publishable one carries the organization's `maestro-` prefix; a test
//! states its claim in four words at least, and a test module in two.

use crate::checks::findings::{Finding, relative};
use crate::checks::manifests::Package;
use crate::checks::module_tree::Tree;
use crate::checks::rust_tests::test_functions;
use std::collections::BTreeSet;
use std::path::Path;

/// The suffixes that say a test ran without saying what it proved.
const EMPTY_SUFFIXES: &[&str] = &["_works", "_ok", "_test"];

/// NAME-001 over every package.
pub(super) fn packages(packages: &[Package], workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for package in packages {
        let file = relative(workspace, &package.manifest);
        let name = &package.name;
        if !kebab(name) {
            let message = format!(
                "package `{name}` is not lowercase kebab-case without a -rs or -rust suffix"
            );
            found.push(Finding::new("NAME-001", file, 0, message));
        } else if package.publishable && !name.starts_with("maestro-") {
            let message = format!(
                "package `{name}` may be published, so its name starts with maestro-; \
                 or set publish = false"
            );
            found.push(Finding::new("NAME-001", file, 0, message));
        }
    }
    found
}

/// NAME-002 over the test modules and the test functions of `trees`, each
/// file once.
pub(super) fn tests(trees: &[Tree], workspace: &Path) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for tree in trees {
        for module in &tree.modules {
            if !seen.insert(&module.file) {
                continue;
            }
            let file = relative(workspace, &module.file);
            let functions = test_functions(&module.code);
            let named_module = tree.kinds.iter().any(|kind| kind == "test")
                && !module.path.is_empty()
                && !module.is_mod_rs()
                && !functions.is_empty();
            let stem = module.path.last().map(String::as_str).unwrap_or_default();
            if named_module && words(stem).is_none_or(|words| words < 2) {
                let message =
                    format!("test module `{stem}` names what it proves in fewer than two words");
                found.push(Finding::new("NAME-002", file.clone(), 0, message));
            }
            found.extend(functions.into_iter().filter_map(|(line, name)| {
                claim(&name).map(|message| Finding::new("NAME-002", file.clone(), line, message))
            }));
        }
    }
    found
}

/// What is wrong with a test's name, if anything.
fn claim(name: &str) -> Option<String> {
    if name.starts_with("test_") {
        return Some(format!(
            "test `{name}` starts with test_; the attribute already says so"
        ));
    }
    if let Some(suffix) = EMPTY_SUFFIXES.iter().find(|suffix| name.ends_with(*suffix)) {
        return Some(format!(
            "test `{name}` ends with {suffix}; name what was proven"
        ));
    }
    words(name)
        .is_none_or(|words| words < 4)
        .then(|| format!("test `{name}` names what it proves in fewer than four words"))
}

/// How many words a snake-case name holds, or nothing for a name that is
/// not snake case.
fn words(name: &str) -> Option<usize> {
    let words: Vec<&str> = name.split('_').collect();
    let snake = name.starts_with(|character: char| character.is_ascii_lowercase())
        && words.iter().all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    snake.then_some(words.len())
}

/// Whether a package name is lowercase kebab-case without a Rust suffix.
fn kebab(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    name.starts_with(|character: char| character.is_ascii_lowercase())
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        && !name.ends_with("-rs")
        && !name.ends_with("-rust")
}

#[cfg(test)]
mod tests {
    use super::{Package, packages, tests};
    use crate::checks::module_tree::sample;
    use std::path::{Path, PathBuf};

    /// A package named `name`, publishable or not.
    fn package(name: &str, publishable: bool) -> Package {
        Package {
            name: name.to_owned(),
            publishable,
            manifest: PathBuf::from("/w/Cargo.toml"),
            edition: "2024".to_owned(),
            dependencies: Vec::new(),
            library: true,
            binary: false,
            plain_tests: Vec::new(),
        }
    }

    #[test]
    fn package_names_hold_their_form_and_publishable_ones_the_prefix() {
        let found: Vec<String> = packages(
            &[
                package("maestro-core", true),
                package("fixture", false),
                package("Router_rs", false),
                package("router-rs", false),
                package("router", true),
            ],
            Path::new("/w"),
        )
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            found,
            [
                "NAME-001 Cargo.toml: package `Router_rs` is not lowercase kebab-case without \
                 a -rs or -rust suffix",
                "NAME-001 Cargo.toml: package `router-rs` is not lowercase kebab-case without \
                 a -rs or -rust suffix",
                "NAME-001 Cargo.toml: package `router` may be published, so its name starts \
                 with maestro-; or set publish = false",
            ]
        );
    }

    #[test]
    fn tests_state_a_claim_and_test_modules_name_it() {
        let module = concat!(
            "#[test]\nfn a_refusal_names_its_rule() {}\n",
            "#[test]\nfn short_name() {}\n",
            "#[test]\nfn test_the_parser_reads_files() {}\n",
            "#[test]\nfn the_parser_reads_files_ok() {}\n",
        );
        let tree = sample(
            &["test"],
            &[
                ("main.rs", "mod parser;\nmod support;\n"),
                ("parser.rs", module),
                ("support.rs", ""),
            ],
        );
        let found: Vec<String> = tests(&[tree], Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "NAME-002 src/parser.rs: test module `parser` names what it proves in fewer \
                 than two words",
                "NAME-002 src/parser.rs:4: test `short_name` names what it proves in fewer than \
                 four words",
                "NAME-002 src/parser.rs:6: test `test_the_parser_reads_files` starts with test_; \
                 the attribute already says so",
                "NAME-002 src/parser.rs:8: test `the_parser_reads_files_ok` ends with _ok; name \
                 what was proven",
            ]
        );
    }
}
