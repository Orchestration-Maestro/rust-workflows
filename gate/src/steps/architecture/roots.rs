//! ARC-006 and ARC-007: a binary's root declares its modules and calls into
//! them from a short `main`, and the module tree is the file tree: no
//! `#[path]` attribute and no `include!` of Rust source.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::{Item, is_identifier_byte, line_at};
use std::path::Path;

/// The lines `fn main` may span, from its signature to its closing brace.
const MAIN_LINES: usize = 25;

/// ARC-006: every item of a binary's root beyond declarations and a short
/// `fn main`.
pub(super) fn thin_roots(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    if !tree.kinds.iter().any(|kind| kind == "bin") {
        return Vec::new();
    }
    let Some(root) = tree.modules.first() else {
        return Vec::new();
    };
    let file = relative(workspace, &root.file);
    root.items
        .iter()
        .filter(|item| !item.is_test())
        .filter_map(|item| {
            problem(item).map(|message| Finding::new("ARC-006", file.clone(), item.line, message))
        })
        .collect()
}

/// What is wrong with one item of a binary's root, if anything.
fn problem(item: &Item) -> Option<String> {
    let lines = item.text.lines().count();
    match (item.kind.as_str(), item.name.as_str()) {
        ("use" | "extern crate", _) => None,
        ("mod", _) if item.is_module_file() => None,
        ("fn", "main") if lines <= MAIN_LINES => None,
        ("fn", "main") => Some(format!(
            "fn main spans {lines} lines; keep it within {MAIN_LINES} and call into modules"
        )),
        (kind, _) => Some(format!(
            "a binary root holds only mod and use declarations and fn main; move this {kind} \
             into a module"
        )),
    }
}

/// ARC-007: every `#[path]` attribute and every `include!` of the tree.
pub(super) fn path_attributes(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for module in &tree.modules {
        let file = relative(workspace, &module.file);
        let code = module.code.as_str();
        for (offset, _) in code.match_indices("#[") {
            let compact: String = code
                .get(offset..)
                .unwrap_or_default()
                .chars()
                .take_while(|&character| character != ']')
                .filter(|character| !character.is_whitespace())
                .collect();
            if compact.starts_with("#[path=")
                || (compact.starts_with("#[cfg_attr(") && compact.contains(",path="))
            {
                found.push(Finding::new(
                    "ARC-007",
                    file.clone(),
                    line_at(code, offset),
                    "`#[path]` makes the module tree differ from the file tree; move the file \
                     where its module is declared"
                        .to_owned(),
                ));
            }
        }
        for (offset, _) in code.match_indices("include!") {
            let standalone = offset
                .checked_sub(1)
                .and_then(|previous| code.as_bytes().get(previous))
                .is_none_or(|&byte| !is_identifier_byte(byte));
            if standalone {
                found.push(Finding::new(
                    "ARC-007",
                    file.clone(),
                    line_at(code, offset),
                    "`include!` of Rust source hides code from the module tree; declare it with \
                     `mod`"
                        .to_owned(),
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{path_attributes, thin_roots};
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_binary_root_holds_declarations_and_a_short_main_only() {
        let long_main = format!("fn main() {{\n{}}}\n", "    run();\n".repeat(25));
        let main = format!("mod app;\nuse app::run;\nstruct Config;\n{long_main}");
        let tree = sample(&["bin"], &[("main.rs", &main), ("app.rs", "")]);
        let found: Vec<String> = thin_roots(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "ARC-006 src/main.rs:3: a binary root holds only mod and use declarations and \
                 fn main; move this struct into a module",
                "ARC-006 src/main.rs:4: fn main spans 27 lines; keep it within 25 and call into \
                 modules",
            ]
        );
        let library = sample(&["lib"], &[("lib.rs", "struct Config;\n")]);
        assert!(thin_roots(&library, Path::new("/w")).is_empty());
    }

    #[test]
    fn path_attributes_and_rust_includes_are_refused_but_include_str_is_not() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod parts;\n"),
                (
                    "parts.rs",
                    concat!(
                        "#[path = \"elsewhere.rs\"]\nmod inner;\n",
                        "#[cfg_attr(test, path = \"t.rs\")]\nmod other;\n",
                        "include!(\"generated.rs\");\n",
                        "const TEXT: &str = include_str!(\"text.txt\");\n",
                    ),
                ),
            ],
        );
        let found: Vec<String> = path_attributes(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        let path = "`#[path]` makes the module tree differ from the file tree; move the file \
                    where its module is declared";
        assert_eq!(
            found,
            [
                format!("ARC-007 src/parts.rs:1: {path}"),
                format!("ARC-007 src/parts.rs:3: {path}"),
                "ARC-007 src/parts.rs:5: `include!` of Rust source hides code from the module \
                 tree; declare it with `mod`"
                    .to_owned(),
            ]
        );
    }
}
