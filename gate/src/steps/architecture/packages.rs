//! LIB-002, TST-003, WSP-001 and WSP-002, read from the manifests: a library
//! hands out typed errors, a package links one integration-test crate, and
//! a workspace keeps one edition, one resolver, one lockfile and one set of
//! shared settings.

use crate::checks::findings::{Finding, relative};
use crate::checks::manifests::{
    Package, Workspace, lockfile_tracked, member_inheritance, root_resolver,
};
use crate::runner::Failure;
use std::path::Path;

/// The error crates a binary may use and a library may not hand out.
const APPLICATION_ERRORS: &[&str] = &["anyhow", "eyre", "color-eyre"];

/// Every finding of the manifests of `packages` and their `cargo_workspace`.
pub(super) fn findings(
    packages: &[Package],
    cargo_workspace: &Workspace,
    workspace: &Path,
) -> Result<Vec<Finding>, Failure> {
    let mut found = Vec::new();
    for package in packages {
        found.extend(package_rules(package, workspace));
        if cargo_workspace.members >= 2 {
            found.extend(inheritance(package, workspace)?);
        }
    }
    found.extend(workspace_rules(cargo_workspace, workspace)?);
    Ok(found)
}

/// LIB-002, TST-003 and the edition of WSP-002 for one package.
fn package_rules(package: &Package, workspace: &Path) -> Vec<Finding> {
    let file = relative(workspace, &package.manifest);
    let mut found = Vec::new();
    let library_only = package.library && !package.binary && package.publishable;
    for dependency in package
        .dependencies
        .iter()
        .filter(|dependency| library_only && APPLICATION_ERRORS.contains(&dependency.as_str()))
    {
        let message = format!(
            "a library exposes typed errors; `{dependency}` belongs in a binary or in \
             [dev-dependencies]"
        );
        found.push(Finding::new("LIB-002", file.clone(), 0, message));
    }
    if package.plain_tests.len() > 1 {
        let message = format!(
            "{} integration-test crates build without required-features ({}); fold them into \
             one test crate, tests/it/main.rs with its modules beside it",
            package.plain_tests.len(),
            package.plain_tests.join(", ")
        );
        found.push(Finding::new("TST-003", file.clone(), 0, message));
    }
    if package.edition != "2024" {
        let message = format!(
            "package `{}` uses edition {}; the organization builds with 2024",
            package.name, package.edition
        );
        found.push(Finding::new("WSP-002", file, 0, message));
    }
    found
}

/// WSP-001 for one member of a workspace of two or more.
fn inheritance(package: &Package, workspace: &Path) -> Result<Vec<Finding>, Failure> {
    let file = relative(workspace, &package.manifest);
    let taken = member_inheritance(&package.manifest)?;
    let missing = &taken.missing;
    let mut found = Vec::new();
    if !missing.is_empty() {
        let message = format!(
            "member `{}` does not inherit {} from the workspace",
            package.name,
            missing.join(", ")
        );
        found.push(Finding::new("WSP-001", file.clone(), 0, message));
    }
    if !taken.local_dependencies.is_empty() {
        let message = format!(
            "member `{}` declares {} without `workspace = true`; declare them in \
             [workspace.dependencies]",
            package.name,
            taken.local_dependencies.join(", ")
        );
        found.push(Finding::new("WSP-001", file, 0, message));
    }
    Ok(found)
}

/// The resolver and lockfile halves of WSP-002, for the workspace root.
fn workspace_rules(cargo_workspace: &Workspace, workspace: &Path) -> Result<Vec<Finding>, Failure> {
    let root = &cargo_workspace.root;
    let mut found = Vec::new();
    let manifest = root.join("Cargo.toml");
    if let Some(resolver) = root_resolver(&manifest)?.filter(|resolver| resolver != "3") {
        let message = format!("the workspace sets resolver {resolver}; set resolver = \"3\"");
        found.push(Finding::new(
            "WSP-002",
            relative(workspace, &manifest),
            0,
            message,
        ));
    }
    let lockfile = root.join("Cargo.lock");
    let problem = if lockfile.is_file() {
        (lockfile_tracked(root) == Some(false)).then_some("Cargo.lock is not tracked; commit it")
    } else {
        Some("Cargo.lock is missing; commit it")
    };
    if let Some(message) = problem {
        found.push(Finding::new(
            "WSP-002",
            relative(workspace, &lockfile),
            0,
            message.to_owned(),
        ));
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::package_rules;
    use crate::checks::manifests::Package;
    use std::path::{Path, PathBuf};

    /// A package with the given shape.
    fn package(library: bool, binary: bool, dependencies: &[&str], tests: &[&str]) -> Package {
        Package {
            name: "maestro-x".to_owned(),
            publishable: true,
            manifest: PathBuf::from("/w/x/Cargo.toml"),
            edition: "2021".to_owned(),
            dependencies: dependencies.iter().map(|name| (*name).to_owned()).collect(),
            library,
            binary,
            plain_tests: tests.iter().map(|name| (*name).to_owned()).collect(),
            features: Vec::new(),
        }
    }

    #[test]
    fn a_library_only_package_hands_out_typed_errors_and_one_test_crate() {
        let found: Vec<String> = package_rules(
            &package(true, false, &["anyhow", "serde"], &["a", "b"]),
            Path::new("/w"),
        )
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            found,
            [
                "LIB-002 x/Cargo.toml: a library exposes typed errors; `anyhow` belongs in a \
                 binary or in [dev-dependencies]",
                "TST-003 x/Cargo.toml: 2 integration-test crates build without \
                 required-features (a, b); fold them into one test crate, tests/it/main.rs with \
                 its modules beside it",
                "WSP-002 x/Cargo.toml: package `maestro-x` uses edition 2021; the organization \
                 builds with 2024",
            ]
        );
        let binary = package_rules(
            &package(true, true, &["anyhow"], &["main"]),
            Path::new("/w"),
        );
        assert_eq!(binary.len(), 1, "only the edition: a binary may use anyhow");
    }
}
