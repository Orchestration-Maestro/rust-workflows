//! ARC-001: no import cycle between the files of one crate. A file depends on
//! another when it names one of that file's items through `crate`, `super`,
//! `self` or a child it declares; each cycle is named file by file.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Which modules each module names, by index; no module names itself.
type Graph = BTreeMap<usize, BTreeSet<usize>>;

/// ARC-001 over one tree: a finding per cycle, named from its first file.
pub(super) fn findings(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let graph = graph(tree);
    let mut done = BTreeSet::new();
    let mut found = Vec::new();
    for &start in graph.keys() {
        let Some(cycle) = walk(start, &graph, &mut done, &mut Vec::new()) else {
            continue;
        };
        done.extend(cycle.iter().copied());
        let files: Vec<String> = cycle
            .iter()
            .filter_map(|&index| tree.modules.get(index))
            .map(|module| relative(workspace, &module.file))
            .collect();
        let message = format!(
            "import cycle {}; one of these files must stop naming the next",
            files.join(" -> ")
        );
        let first = files.first().cloned().unwrap_or_default();
        found.push(Finding::new("ARC-001", first, 0, message));
    }
    found
}

/// Every path each module names, resolved to the module it lands in.
fn graph(tree: &Tree) -> Graph {
    tree.modules
        .iter()
        .enumerate()
        .map(|(index, module)| {
            let targets = module
                .paths
                .iter()
                .filter_map(|path| tree.absolute(&module.path, &path.segments))
                .filter_map(|absolute| tree.landing(&absolute))
                .filter(|&target| target != index)
                .collect();
            (index, targets)
        })
        .collect()
}

/// A depth-first walk from `node` that returns the first cycle it closes,
/// the repeated module last.
fn walk(
    node: usize,
    graph: &Graph,
    done: &mut BTreeSet<usize>,
    path: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    if let Some(start) = path.iter().position(|&seen| seen == node) {
        let mut cycle = path.get(start..).unwrap_or_default().to_vec();
        cycle.push(node);
        return Some(cycle);
    }
    if done.contains(&node) {
        return None;
    }
    path.push(node);
    for &next in graph.get(&node).into_iter().flatten() {
        if let Some(cycle) = walk(next, graph, done, path) {
            return Some(cycle);
        }
    }
    path.pop();
    done.insert(node);
    None
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_cycle_is_named_file_by_file_from_its_first_module() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod a;\nmod b;\nmod c;\n"),
                ("a.rs", "fn f() { crate::b::g() }\n"),
                ("b.rs", "fn g() { crate::a::f() }\n"),
                ("c.rs", "fn h() { crate::a::f() }\n"),
            ],
        );
        let found: Vec<String> = findings(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "ARC-001 src/a.rs: import cycle src/a.rs -> src/b.rs -> src/a.rs; \
              one of these files must stop naming the next"
            ]
        );
    }

    #[test]
    fn a_parent_naming_a_child_that_names_nothing_back_is_no_cycle() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod a;\nfn f() { a::g() }\n"),
                ("a.rs", "pub(crate) fn g() {}\n"),
            ],
        );
        assert!(findings(&tree, Path::new("/w")).is_empty());
    }
}
