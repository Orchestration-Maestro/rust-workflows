//! What Cargo says about a project beyond its module trees: the metadata
//! file every reader shares, the packages it lists with their targets and
//! dependencies, the workspace they form, and, read from each manifest
//! through jaq, what a member inherits from its workspace.

use crate::runner::{Cmd, Failure};
use std::path::{Path, PathBuf};

/// Every package, one line each: name, whether it may be published, its
/// manifest, its edition, its normal dependencies, the kinds of its targets,
/// the test targets that build without required features, and the features it
/// declares, those Cargo makes of an optional dependency left out.
const PACKAGES: &str = ".packages[] | [.name, (.publish != [] | tostring), .manifest_path, \
    .edition, ([.dependencies[] | select(.kind == null) | .name] | join(\",\")), \
    ([.targets[].kind[]] | unique | join(\",\")), ([.targets[] | select(.kind == [\"test\"] \
    and ((.[\"required-features\"] // []) | length) == 0) | .name] | join(\",\")), \
    ([.features | to_entries[] | select(.value != [\"dep:\" + .key]) | .key] \
    | join(\",\"))] | @tsv";

/// The workspace: its root, then how many members it has.
const WORKSPACE: &str = "[.workspace_root, (.workspace_members | length | tostring)] | @tsv";

/// What a member manifest takes from its workspace: `[lints]`, `edition`,
/// `rust-version` and `license`, then every dependency declared without
/// `workspace = true`.
const INHERITANCE: &str = "def inherited: if type == \"object\" then .workspace == true \
    else false end; [(.lints | inherited), (.package.edition | inherited), \
    (.package[\"rust-version\"] | inherited), (.package.license | inherited), \
    ([(.dependencies // {}), (.[\"dev-dependencies\"] // {}), \
    (.[\"build-dependencies\"] // {}), ((.target // {}) | .[] | (.dependencies // {}), \
    (.[\"dev-dependencies\"] // {}), (.[\"build-dependencies\"] // {}))] \
    | map(to_entries[] | select(.value | inherited | not) | .key) | unique | join(\",\"))] \
    | map(tostring) | @tsv";

/// The resolver a virtual workspace root sets, `unset` when it sets none,
/// or nothing when the root is itself a package.
const RESOLVER: &str =
    "if has(\"package\") then \"\" else (.workspace.resolver // \"unset\" | tostring) end";

/// The kinds of target that make a package a library.
pub(crate) const LIBRARY_KINDS: &[&str] =
    &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

/// One package of the project.
pub(crate) struct Package {
    /// Its name.
    pub(crate) name: String,
    /// Whether it may be published: its `publish` is not `false`.
    pub(crate) publishable: bool,
    /// Its manifest.
    pub(crate) manifest: PathBuf,
    /// Its edition.
    pub(crate) edition: String,
    /// Its normal dependencies, build and dev ones left out.
    pub(crate) dependencies: Vec<String>,
    /// Whether it has a library target.
    pub(crate) library: bool,
    /// Whether it has a binary target.
    pub(crate) binary: bool,
    /// Its test targets that build without required features.
    pub(crate) plain_tests: Vec<String>,
    /// The features its manifest declares, in Cargo's order.
    pub(crate) features: Vec<String>,
}

/// The workspace the packages form.
pub(crate) struct Workspace {
    /// Its root directory.
    pub(crate) root: PathBuf,
    /// How many members it has.
    pub(crate) members: usize,
}

/// The settings a member takes from its workspace, in the order the
/// inheritance listing gives them.
const INHERITED: [&str; 4] = ["[lints]", "edition", "rust-version", "license"];

/// What one member does not take from its workspace.
pub(crate) struct Inheritance {
    /// The settings it declares itself: `[lints]`, `edition`, `rust-version`
    /// or `license`.
    pub(crate) missing: Vec<&'static str>,
    /// The dependencies declared without `workspace = true`.
    pub(crate) local_dependencies: Vec<String>,
}

/// Run `cargo metadata` once for the project and return the file every
/// reader of it shares.
pub(crate) fn read_cargo_metadata(project: &Path, temp: &Path) -> Result<PathBuf, Failure> {
    let metadata = temp.join("cargo-metadata.json");
    Cmd::new("cargo metadata --no-deps --format-version 1 --offline --manifest-path")
        .arg(project.join("Cargo.toml"))
        .stdout_to(&metadata)?;
    Ok(metadata)
}

/// Every package the metadata lists.
pub(crate) fn cargo_packages(metadata: &Path) -> Result<Vec<Package>, Failure> {
    let listing = Cmd::new("jaq -r").arg(PACKAGES).arg(metadata).capture()?;
    listing.lines().map(parse_package).collect()
}

/// The workspace the metadata describes.
pub(crate) fn workspace_of(metadata: &Path) -> Result<Workspace, Failure> {
    let line = Cmd::new("jaq -r").arg(WORKSPACE).arg(metadata).capture()?;
    parse_workspace(line.trim_end_matches('\n'))
}

/// What the member at `manifest` inherits from its workspace.
pub(crate) fn member_inheritance(manifest: &Path) -> Result<Inheritance, Failure> {
    let line = Cmd::new("jaq --from toml -r")
        .arg(INHERITANCE)
        .arg(manifest)
        .capture()?;
    parse_inheritance(line.trim_end_matches('\n'))
}

/// The resolver the virtual workspace at `manifest` sets, or `None` when
/// its root is a package.
pub(crate) fn root_resolver(manifest: &Path) -> Result<Option<String>, Failure> {
    let line = Cmd::new("jaq --from toml -r")
        .arg(RESOLVER)
        .arg(manifest)
        .capture()?;
    let resolver = line.trim();
    Ok((!resolver.is_empty()).then(|| resolver.to_owned()))
}

/// Whether git tracks the lockfile at `root`, or `None` when no git work
/// tree holds it and the question has no answer.
pub(crate) fn lockfile_tracked(root: &Path) -> Option<bool> {
    root.ancestors()
        .find(|directory| directory.join(".git").exists())?;
    let listed = Cmd::new("git -C")
        .arg(root)
        .args(["ls-files", "--", "Cargo.lock"])
        .capture()
        .ok()?;
    Some(!listed.trim().is_empty())
}

/// A comma-separated list, empty for an empty field.
fn list(field: &str) -> Vec<String> {
    field
        .split(',')
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One line of the packages listing.
fn parse_package(line: &str) -> Result<Package, Failure> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [
        name,
        publishable,
        manifest,
        edition,
        dependencies,
        kinds,
        plain_tests,
        features,
    ] = fields[..]
    else {
        return Err(format!("cargo metadata listed a package the gate cannot read: {line}").into());
    };
    let kinds = list(kinds);
    Ok(Package {
        name: name.to_owned(),
        publishable: publishable == "true",
        manifest: PathBuf::from(manifest),
        edition: edition.to_owned(),
        dependencies: list(dependencies),
        library: kinds
            .iter()
            .any(|kind| LIBRARY_KINDS.contains(&kind.as_str())),
        binary: kinds.iter().any(|kind| kind == "bin"),
        plain_tests: list(plain_tests),
        features: list(features),
    })
}

/// The workspace line: its root, then its member count.
fn parse_workspace(line: &str) -> Result<Workspace, Failure> {
    let parsed = line
        .split_once('\t')
        .and_then(|(root, members)| Some((root, members.parse().ok()?)));
    let Some((root, members)) = parsed else {
        return Err(format!("cargo metadata gave a workspace the gate cannot read: {line}").into());
    };
    Ok(Workspace {
        root: PathBuf::from(root),
        members,
    })
}

/// One member's inheritance line.
fn parse_inheritance(line: &str) -> Result<Inheritance, Failure> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [lints, edition, rust_version, license, local] = fields[..] else {
        return Err(format!("a member manifest the gate cannot read: {line}").into());
    };
    let missing = [lints, edition, rust_version, license]
        .into_iter()
        .zip(INHERITED)
        .filter(|(inherited, _)| *inherited != "true")
        .map(|(_, setting)| setting)
        .collect();
    Ok(Inheritance {
        missing,
        local_dependencies: list(local),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_inheritance, parse_package, parse_workspace};
    use std::path::Path;

    #[test]
    fn a_package_line_gives_its_targets_dependencies_and_plain_tests() {
        let package = parse_package(
            "maestro-core\ttrue\t/w/core/Cargo.toml\t2024\tanyhow,serde\tbin,lib,test\tmain,other\t\
             default,serve",
        )
        .unwrap();
        assert_eq!(package.name, "maestro-core");
        assert!(package.publishable && package.library && package.binary);
        assert_eq!(package.manifest, Path::new("/w/core/Cargo.toml"));
        assert_eq!(package.edition, "2024");
        assert_eq!(package.dependencies, ["anyhow", "serde"]);
        assert_eq!(package.plain_tests, ["main", "other"]);
        assert_eq!(package.features, ["default", "serve"]);
        let bare = parse_package("fixture\tfalse\t/w/Cargo.toml\t2021\t\tlib\t\t").unwrap();
        assert!(!bare.publishable && bare.library && !bare.binary);
        assert!(bare.dependencies.is_empty() && bare.plain_tests.is_empty());
        assert!(bare.features.is_empty());
        assert_eq!(
            parse_package("broken").err().unwrap().message.as_deref(),
            Some("cargo metadata listed a package the gate cannot read: broken")
        );
    }

    #[test]
    fn the_workspace_line_gives_its_root_and_member_count() {
        let workspace = parse_workspace("/w\t3").unwrap();
        assert_eq!(
            (workspace.root.as_path(), workspace.members),
            (Path::new("/w"), 3)
        );
        assert_eq!(
            parse_workspace("/w").err().unwrap().message.as_deref(),
            Some("cargo metadata gave a workspace the gate cannot read: /w")
        );
    }

    #[test]
    fn an_inheritance_line_names_what_a_member_takes_from_its_workspace() {
        let inheritance = parse_inheritance("true\tfalse\ttrue\tfalse\tlocal,other").unwrap();
        assert_eq!(inheritance.missing, ["edition", "license"]);
        assert_eq!(inheritance.local_dependencies, ["local", "other"]);
        assert_eq!(
            parse_inheritance("true").err().unwrap().message.as_deref(),
            Some("a member manifest the gate cannot read: true")
        );
    }
}
