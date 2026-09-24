//! ARC-002 and ARC-003: a door holds only `mod` and `use` declarations, and a
//! path from outside a door goes through it: when the door re-exports a
//! name, walking past it to the module defining the name is refused.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::Item;
use std::path::Path;

/// The kinds of target whose root is the crate's public door.
const LIBRARY_KINDS: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

/// ARC-002: every item of a door that does more than declare.
pub(super) fn contents(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let library = tree
        .kinds
        .iter()
        .any(|kind| LIBRARY_KINDS.contains(&kind.as_str()));
    let mut found = Vec::new();
    for module in &tree.modules {
        let root_door = module.path.is_empty() && library && module.children().next().is_some();
        if !(module.is_mod_rs() || root_door) {
            continue;
        }
        for item in module
            .items
            .iter()
            .filter(|item| !item.is_test() && !declares(item))
        {
            found.push(Finding::new(
                "ARC-002",
                relative(workspace, &module.file),
                item.line,
                format!(
                    "a door holds only mod and use declarations; move this {} into a module of \
                     its own",
                    item.kind
                ),
            ));
        }
    }
    found
}

/// Whether an item only declares: a module kept in its own file, an import
/// or an external crate.
fn declares(item: &Item) -> bool {
    item.is_module_file() || matches!(item.kind.as_str(), "use" | "extern crate")
}

/// ARC-003: every path from outside a `mod.rs` door that reaches, past the
/// door, a name the door re-exports from one of its modules.
pub(super) fn bypasses(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for door in tree.modules.iter().filter(|module| module.is_mod_rs()) {
        for export in &door.exports {
            let Some(target) = tree.absolute(&door.path, &export.segments) else {
                continue;
            };
            let Some(name) = target.last().filter(|_| target.len() > door.path.len() + 1) else {
                continue;
            };
            let outside = tree
                .modules
                .iter()
                .filter(|module| !module.path.starts_with(&door.path));
            for module in outside {
                for path in &module.paths {
                    let past = tree
                        .absolute(&module.path, &path.segments)
                        .is_some_and(|absolute| absolute.starts_with(&target));
                    if past {
                        found.push(Finding::new(
                            "ARC-003",
                            relative(workspace, &module.file),
                            path.line,
                            format!(
                                "`{}` walks past the door of {}; name `{name}` through it",
                                path.segments.join("::"),
                                display(&door.path)
                            ),
                        ));
                    }
                }
            }
        }
    }
    found
}

/// A module path as a reader writes it: `crate::runner`.
fn display(path: &[String]) -> String {
    std::iter::once("crate")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::{bypasses, contents};
    use crate::checks::module_tree::sample;
    use std::path::Path;

    /// Every finding of `rule` as its report line.
    fn lines(found: &[crate::checks::findings::Finding]) -> Vec<String> {
        found.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_door_holding_more_than_declarations_is_refused() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod shapes;\npub use shapes::area;\n"),
                (
                    "shapes/mod.rs",
                    concat!(
                        "mod circle;\npub(crate) use circle::area;\nconst PI: f64 = 3.14;\n",
                        "mod inline {}\n#[cfg(test)]\nmod tests {}\n",
                    ),
                ),
                (
                    "shapes/circle.rs",
                    "pub(crate) fn area() {}\nfn helper() {}\n",
                ),
            ],
        );
        assert_eq!(
            lines(&contents(&tree, Path::new("/w"))),
            [
                "ARC-002 src/shapes/mod.rs:3: a door holds only mod and use declarations; \
                 move this const into a module of its own",
                "ARC-002 src/shapes/mod.rs:4: a door holds only mod and use declarations; \
                 move this mod into a module of its own",
            ]
        );
    }

    #[test]
    fn a_single_file_library_and_a_binary_root_are_not_doors() {
        let library = sample(&["lib"], &[("lib.rs", "pub fn f() {}\n")]);
        let binary = sample(
            &["bin"],
            &[("main.rs", "mod a;\nfn main() {}\n"), ("a.rs", "")],
        );
        assert!(contents(&library, Path::new("/w")).is_empty());
        assert!(contents(&binary, Path::new("/w")).is_empty());
    }

    #[test]
    fn a_path_walking_past_a_re_export_is_refused() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod report;\nmod shapes;\n"),
                (
                    "report.rs",
                    concat!(
                        "fn print() {\n    crate::shapes::circle::area();\n",
                        "    crate::shapes::area();\n}\n",
                    ),
                ),
                (
                    "shapes/mod.rs",
                    "pub(super) mod circle;\npub(super) use circle::area;\n",
                ),
                (
                    "shapes/circle.rs",
                    "pub(crate) fn area() { super::circle::helper() }\n",
                ),
            ],
        );
        assert_eq!(
            lines(&bypasses(&tree, Path::new("/w"))),
            [
                "ARC-003 src/report.rs:2: `crate::shapes::circle::area` walks past the door of \
              crate::shapes; name `area` through it"
            ]
        );
    }
}
