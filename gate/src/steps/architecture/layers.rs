//! ARC-004: the layers a target declares in `maestro-quality.toml`. Each
//! top-level module sits in one layer, and a module imports only from layers
//! to its right: never from its own layer, never from one to its left.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::{Module, Tree};
use crate::checks::quality_config::{FILE, Layers};
use std::path::Path;

/// ARC-004 over one tree, when `maestro-quality.toml` declares its root.
pub(super) fn findings(tree: &Tree, workspace: &Path, declared: &[Layers]) -> Vec<Finding> {
    let Some(root) = tree.modules.first() else {
        return Vec::new();
    };
    let root_file = relative(workspace, &root.file);
    let Some(layers) = declared.iter().find(|layers| layers.root == root_file) else {
        return Vec::new();
    };
    let mut found = placement(root, layers, &root_file);
    for module in &tree.modules {
        let file = relative(workspace, &module.file);
        for (line, path, own, other) in crossings(tree, module, layers) {
            found.push(Finding::new(
                "ARC-004",
                file.clone(),
                line,
                format!(
                    "`{path}` imports `{other}` from `{own}`; a layer imports only from layers to \
                     its right"
                ),
            ));
        }
    }
    found
}

/// A finding for every root under `scope` that declares layers and that no
/// target has.
pub(super) fn unknown_roots(
    trees: &[Tree],
    workspace: &Path,
    declared: &[Layers],
    scope: &str,
) -> Vec<Finding> {
    let roots: Vec<String> = trees
        .iter()
        .filter_map(|tree| tree.modules.first())
        .map(|root| relative(workspace, &root.file))
        .collect();
    declared
        .iter()
        .filter(|layers| layers.root.starts_with(scope) && !roots.contains(&layers.root))
        .map(|layers| {
            Finding::new(
                "ARC-004",
                FILE.to_owned(),
                0,
                format!(
                    "declares layers for {}, which is no target's root",
                    layers.root
                ),
            )
        })
        .collect()
}

/// The layer holding top-level module `name`, counted from the left.
fn layer_of(layers: &Layers, name: &str) -> Option<usize> {
    layers
        .layers
        .iter()
        .position(|layer| layer.iter().any(|module| module == name))
}

/// Every top-level module the root declares outside the layers, and every
/// layer module the root does not declare.
fn placement(root: &Module, layers: &Layers, root_file: &str) -> Vec<Finding> {
    let mut found = Vec::new();
    for child in root.children() {
        if layer_of(layers, &child.name).is_none() {
            found.push(Finding::new(
                "ARC-004",
                root_file.to_owned(),
                child.line,
                format!(
                    "module `{}` sits in no layer {FILE} declares for this root; add it to one",
                    child.name
                ),
            ));
        }
    }
    for name in layers.layers.iter().flatten() {
        if !root.children().any(|child| &child.name == name) {
            found.push(Finding::new(
                "ARC-004",
                FILE.to_owned(),
                0,
                format!(
                    "the layers of {root_file} name `{name}`, which that root does not declare"
                ),
            ));
        }
    }
    found
}

/// Every path of `module` into a top-level module of its own layer or of
/// one to its left: the line, the path, and the two modules.
fn crossings<'a>(
    tree: &Tree,
    module: &'a Module,
    layers: &Layers,
) -> Vec<(usize, String, &'a str, String)> {
    let Some(own) = module.path.first() else {
        return Vec::new();
    };
    let Some(from) = layer_of(layers, own) else {
        return Vec::new();
    };
    module
        .paths
        .iter()
        .filter_map(|path| {
            let target = tree.absolute(&module.path, &path.segments)?;
            let other = target.first().filter(|other| *other != own)?;
            let to = layer_of(layers, other)?;
            (to <= from).then(|| {
                (
                    path.line,
                    path.segments.join("::"),
                    own.as_str(),
                    other.clone(),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{findings, unknown_roots};
    use crate::checks::module_tree::sample;
    use crate::checks::quality_config::Layers;
    use std::path::Path;

    /// The layers declared for `src/main.rs`, one string per layer.
    fn declared(layers: &[&str]) -> Vec<Layers> {
        vec![Layers {
            root: "src/main.rs".to_owned(),
            layers: layers
                .iter()
                .map(|layer| layer.split_whitespace().map(str::to_owned).collect())
                .collect(),
        }]
    }

    #[test]
    fn imports_run_only_to_layers_on_the_right() {
        let tree = sample(
            &["bin"],
            &[
                (
                    "main.rs",
                    "mod checks;\nmod runner;\nmod steps;\nmod stray;\nfn main() {}\n",
                ),
                (
                    "steps.rs",
                    "fn go() { crate::checks::check(); crate::runner::run(); }\n",
                ),
                ("checks.rs", "fn check() {\n    crate::steps::go();\n}\n"),
                ("runner.rs", "fn run() {}\n"),
                ("stray.rs", ""),
            ],
        );
        let found: Vec<String> = findings(
            &tree,
            Path::new("/w"),
            &declared(&["steps", "checks", "runner", "gone"]),
        )
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            found,
            [
                "ARC-004 src/main.rs:4: module `stray` sits in no layer maestro-quality.toml \
                 declares for this root; add it to one",
                "ARC-004 maestro-quality.toml: the layers of src/main.rs name `gone`, which \
                 that root does not declare",
                "ARC-004 src/checks.rs:2: `crate::steps::go` imports `steps` from `checks`; a \
                 layer imports only from layers to its right",
            ]
        );
    }

    #[test]
    fn modules_of_one_layer_never_import_each_other() {
        let tree = sample(
            &["test"],
            &[
                ("main.rs", "mod ci;\nmod gate;\nmod harness;\n"),
                (
                    "ci.rs",
                    "use crate::gate::helper;\nuse crate::harness::Fixture;\n",
                ),
                ("gate.rs", ""),
                ("harness.rs", ""),
            ],
        );
        let found = findings(&tree, Path::new("/w"), &declared(&["ci gate", "harness"]));
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].to_string(),
            "ARC-004 src/ci.rs:1: `crate::gate::helper` imports `gate` from `ci`; a layer \
             imports only from layers to its right"
        );
    }

    #[test]
    fn layers_for_a_root_no_target_has_are_refused_in_scope_only() {
        let trees = [sample(&["bin"], &[("main.rs", "fn main() {}\n")])];
        let mut layers = declared(&["a", "b"]);
        layers[0].root = "src/gone.rs".to_owned();
        layers.extend(declared(&["a", "b"]));
        layers[1].root = "other/src/main.rs".to_owned();
        let found: Vec<String> = unknown_roots(&trees, Path::new("/w"), &layers, "src/")
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "ARC-004 maestro-quality.toml: declares layers for src/gone.rs, which is no \
              target's root"
            ]
        );
    }
}
