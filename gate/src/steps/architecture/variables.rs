//! NAME-004: in the code a `maestro-` package ships, every environment
//! variable read by name starts with `MAESTRO_`, or is one the platform, Cargo
//! or the CI sets. A variable another tool owns takes an exception naming it.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::{is_identifier_byte, line_at, without_tests};
use std::collections::BTreeSet;
use std::path::Path;

/// The calls that read a variable by a name written in them.
const READERS: &[&str] = &["env::var(", "env::var_os(", "env!(", "option_env!("];

/// The prefixes of the variables the platform, Cargo and the CI set.
const PLATFORM_PREFIXES: &[&str] = &["CARGO_", "RUST", "GITHUB_", "RUNNER_", "XDG_", "LC_"];

/// The variables the platform, the shell, Cargo's build scripts and the CI
/// set under a name of their own.
const PLATFORM_NAMES: &[&str] = &[
    "HOME",
    "PATH",
    "TMPDIR",
    "TEMP",
    "TMP",
    "USER",
    "USERNAME",
    "SHELL",
    "TERM",
    "LANG",
    "NO_COLOR",
    "CI",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "SystemRoot",
    "windir",
    "ProgramFiles",
    "ProgramData",
    "ComSpec",
    "PATHEXT",
    "OUT_DIR",
    "TARGET",
    "HOST",
    "PROFILE",
    "DEBUG",
    "OPT_LEVEL",
    "NUM_JOBS",
];

/// NAME-004 over the targets of `maestro-` packages, their test and bench
/// targets and test items left out, each file once.
pub(super) fn findings(trees: &[Tree], workspace: &Path) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    let shipped = trees.iter().filter(|tree| {
        tree.package.starts_with("maestro-")
            && !tree
                .kinds
                .iter()
                .any(|kind| kind == "test" || kind == "bench")
    });
    for module in shipped.flat_map(|tree| &tree.modules) {
        if !seen.insert(&module.file) {
            continue;
        }
        let code = without_tests(&module.code);
        for (offset, name) in variables(&code, &module.source) {
            if is_allowed(name) {
                continue;
            }
            let message = format!(
                "`{name}` is read from the environment; name it `MAESTRO_{name}`, or take an \
                 exception naming it when another tool owns it"
            );
            let file = relative(workspace, &module.file);
            found.push(Finding::new("NAME-004", file, line_at(&code, offset), message).about(name));
        }
    }
    found
}

/// Whether a variable is the organization's own or the platform's.
fn is_allowed(name: &str) -> bool {
    name.starts_with("MAESTRO_")
        || PLATFORM_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
        || PLATFORM_NAMES.contains(&name)
}

/// Every variable `code`, blanked, reads by a string literal of `source`:
/// the offset of the call and the name, in order.
fn variables<'source>(code: &str, source: &'source str) -> Vec<(usize, &'source str)> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    for reader in READERS {
        for (start, _) in code.match_indices(reader) {
            let joined = start
                .checked_sub(1)
                .and_then(|before| bytes.get(before))
                .is_some_and(|&byte| is_identifier_byte(byte));
            if let Some(name) = (!joined)
                .then(|| literal(source, start + reader.len()))
                .flatten()
            {
                found.push((start, name));
            }
        }
    }
    found.sort_unstable();
    found
}

/// The name a string literal starting at or after `from` holds, when the
/// next token is one and the name is a plain variable name.
fn literal(source: &str, from: usize) -> Option<&str> {
    let rest = source.get(from..)?.trim_start();
    let (name, _) = rest.strip_prefix('"')?.split_once('"')?;
    let plain = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    plain.then_some(name)
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_maestro_package_reads_only_its_own_or_platform_variables() {
        let source = concat!(
            "fn read() {\n",
            "    let _ = std::env::var(\"MAESTRO_HOME\");\n",
            "    let _ = env::var_os(\"MODEL_ROUTER_COMMIT\");\n",
            "    let _ = env!(\"CARGO_PKG_VERSION\");\n",
            "    let _ = option_env!(\n        \"API_TOKEN\");\n",
            "    let _ = env::var(\"SystemRoot\");\n",
            "    let _ = env::var(name);\n",
            "    let _ = myenv::var(\"OTHER\");\n",
            "    let _ = \"env::var(\\\"QUOTED\\\")\";\n",
            "}\n",
            "#[cfg(test)]\nmod tests {\n    fn t() { std::env::var(\"FIXTURE\"); }\n}\n",
        );
        let mut library = sample(&["lib"], &[("lib.rs", source)]);
        library.package = "maestro-router".to_owned();
        let found = findings(&[library], Path::new("/w"));
        let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "NAME-004 src/lib.rs:3: `MODEL_ROUTER_COMMIT` is read from the environment; \
                 name it `MAESTRO_MODEL_ROUTER_COMMIT`, or take an exception naming it when \
                 another tool owns it",
                "NAME-004 src/lib.rs:5: `API_TOKEN` is read from the environment; name it \
                 `MAESTRO_API_TOKEN`, or take an exception naming it when another tool owns it",
            ]
        );
        assert_eq!(found[0].item, "MODEL_ROUTER_COMMIT");
    }

    #[test]
    fn other_packages_and_test_targets_read_any_variable() {
        let source = "fn read() { let _ = std::env::var(\"API_TOKEN\"); }\n";
        let other = sample(&["lib"], &[("lib.rs", source)]);
        let mut test_target = sample(&["test"], &[("main.rs", source)]);
        test_target.package = "maestro-router".to_owned();
        assert!(findings(&[other, test_target], Path::new("/w")).is_empty());
    }
}
