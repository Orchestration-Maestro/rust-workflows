//! Proof that no crate holds an import cycle: a file that names another
//! file's items, through `crate`, `super` or a child module it declares,
//! is never named back by that file, directly or through others.

use crate::harness::root;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// A module inside one crate: `[]` for the root file, `["runner", "commands"]`
/// for `runner/commands.rs`, `["runner"]` for `runner/mod.rs`.
type ModulePath = Vec<String>;

/// Every crate of the repository as the directory holding its modules and
/// the file at its root.
fn crates(root: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut queue = vec![root.to_path_buf()];
    while let Some(directory) = queue.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() && !name.starts_with('.') && name != "target" {
                queue.push(path);
            } else if name == "Cargo.toml" {
                found.extend(crate_root(&directory));
            }
        }
    }
    found.sort();
    found
}

/// Where a crate's module tree starts: the target path the manifest declares,
/// else `src/main.rs` or `src/lib.rs`. A workspace manifest declares neither.
fn crate_root(directory: &Path) -> Option<(PathBuf, String)> {
    let manifest = fs::read_to_string(directory.join("Cargo.toml")).unwrap();
    let declared = manifest.lines().find_map(|line| {
        line.trim()
            .strip_prefix("path = \"")
            .and_then(|rest| rest.strip_suffix(".rs\""))
    });
    if let Some(stem) = declared {
        return Some((directory.to_path_buf(), format!("{stem}.rs")));
    }
    ["main.rs", "lib.rs"]
        .into_iter()
        .find(|file| directory.join("src").join(file).is_file())
        .map(|file| (directory.join("src"), file.to_owned()))
}

/// Every Rust file of a crate keyed by its module, the root file at `[]`.
fn modules(directory: &Path, root_file: &str) -> BTreeMap<ModulePath, PathBuf> {
    let mut found = BTreeMap::new();
    let mut queue = vec![directory.to_path_buf()];
    while let Some(current) = queue.pop() {
        for entry in fs::read_dir(&current).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() && name != "target" {
                queue.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.insert(module_of(directory, &path, root_file), path);
            }
        }
    }
    found
}

/// The module a file holds: its path under the crate directory without the
/// extension, `mod.rs` standing for its directory and the root file for `[]`.
fn module_of(directory: &Path, file: &Path, root_file: &str) -> ModulePath {
    let relative = file.strip_prefix(directory).unwrap();
    if relative == Path::new(root_file) {
        return Vec::new();
    }
    let mut module: ModulePath = relative
        .with_extension("")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    if module.last().is_some_and(|last| last == "mod") {
        module.pop();
    }
    module
}

/// The files this file names: through `crate`, through `super` and
/// through the children it declares. Inline test modules and comment lines
/// are not read; a test module names its own file through `super`.
fn named_files(
    file: &Path,
    module: &[String],
    modules: &BTreeMap<ModulePath, PathBuf>,
) -> BTreeSet<PathBuf> {
    let text = fs::read_to_string(file).unwrap();
    let body: Vec<&str> = text
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    let body = body.join("\n");
    let parent = module[..module.len().saturating_sub(1)].to_vec();
    let mut named = BTreeSet::new();
    for (marker, base) in [
        (concat!("crate", "::"), Vec::new()),
        (concat!("super", "::"), parent),
    ] {
        for chain in chains_after(&body, marker) {
            named.extend(resolve(&base, &chain, modules));
        }
    }
    for child in body.lines().filter_map(declared_child) {
        let mut path = module.to_vec();
        path.push(child.to_owned());
        if !chains_after(&body, &format!("{child}::")).is_empty() {
            named.extend(modules.get(&path).cloned());
        }
    }
    named.remove(file);
    named
}

/// The child module a `mod name;` line declares.
fn declared_child(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
    line.strip_prefix("mod ")
        .and_then(|rest| rest.strip_suffix(';'))
        .filter(|name| name.chars().all(|c| c.is_alphanumeric() || c == '_'))
}

/// The identifier chains that follow each occurrence of `marker` standing
/// at the start of a path: `crate::harness::{root}` yields `["harness"]`.
fn chains_after(body: &str, marker: &str) -> Vec<Vec<String>> {
    body.match_indices(marker)
        .filter(|(index, _)| {
            body[..*index]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != ':')
        })
        .map(|(index, _)| {
            let rest = &body[index + marker.len()..];
            let end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
                .unwrap_or(rest.len());
            rest[..end]
                .split("::")
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

/// The file holding the longest module prefix of `base` followed by `chain`,
/// each leading `super` climbing one level from the base.
fn resolve(
    base: &[String],
    chain: &[String],
    modules: &BTreeMap<ModulePath, PathBuf>,
) -> Option<PathBuf> {
    let mut base = base.to_vec();
    let mut chain = chain;
    while chain.first().is_some_and(|first| first == "super") {
        base.pop();
        chain = &chain[1..];
    }
    (0..=chain.len()).rev().find_map(|length| {
        let mut candidate = base.clone();
        candidate.extend_from_slice(&chain[..length]);
        modules.get(&candidate).cloned()
    })
}

/// The first cycle of the graph as the files along it, the first repeated.
fn first_cycle(graph: &BTreeMap<PathBuf, BTreeSet<PathBuf>>) -> Option<Vec<PathBuf>> {
    let mut done = BTreeSet::new();
    graph
        .keys()
        .find_map(|start| walk(start, graph, &mut done, &mut Vec::new()))
}

/// A depth-first walk from `node` that returns the first cycle it closes.
fn walk(
    node: &Path,
    graph: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    done: &mut BTreeSet<PathBuf>,
    path: &mut Vec<PathBuf>,
) -> Option<Vec<PathBuf>> {
    if let Some(start) = path.iter().position(|seen| seen == node) {
        let mut cycle = path[start..].to_vec();
        cycle.push(node.to_path_buf());
        return Some(cycle);
    }
    if done.contains(node) {
        return None;
    }
    path.push(node.to_path_buf());
    for next in graph.get(node).into_iter().flatten() {
        if let Some(cycle) = walk(next, graph, done, path) {
            return Some(cycle);
        }
    }
    path.pop();
    done.insert(node.to_path_buf());
    None
}

#[test]
fn every_crate_has_an_acyclic_import_graph() {
    // A cycle means two files that each need the other to be understood;
    // the layers of the gate and the door of the tests both rest on the
    // absence of one, so every crate is walked, examples included.
    let root = root();
    let mut resolved = 0;
    for (directory, root_file) in crates(&root) {
        let modules = modules(&directory, &root_file);
        let graph: BTreeMap<PathBuf, BTreeSet<PathBuf>> = modules
            .iter()
            .map(|(module, file)| (file.clone(), named_files(file, module, &modules)))
            .collect();
        resolved += graph.values().map(BTreeSet::len).sum::<usize>();
        if let Some(cycle) = first_cycle(&graph) {
            let shown: Vec<String> = cycle
                .iter()
                .map(|file| file.strip_prefix(&root).unwrap().display().to_string())
                .collect();
            panic!("import cycle: {}", shown.join(" -> "));
        }
    }
    assert!(
        resolved >= 40,
        "only {resolved} imports resolved; the walk drifted"
    );
}
