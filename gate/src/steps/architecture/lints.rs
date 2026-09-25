//! LNT-001 read from the files that carry it: every lint of the
//! organization's list denied in the root manifest, and a `clippy.toml` the
//! repository commits no looser than the organization's, which the gate
//! passes to Clippy at run time when the repository commits none.

use crate::checks::findings::{Finding, relative};
use crate::checks::lint_policy::{
    clippy_settings_of, looser_settings, manifest_lint_levels, undenied,
};
use crate::checks::manifests::Workspace;
use crate::checks::organization_config::nearest_clippy_config;
use crate::runner::Failure;
use std::path::Path;

/// Every LNT-001 finding of the Cargo workspace `cargo_workspace`.
pub(super) fn findings(
    cargo_workspace: &Workspace,
    workspace: &Path,
) -> Result<Vec<Finding>, Failure> {
    let manifest = cargo_workspace.root.join("Cargo.toml");
    let missing = undenied(&manifest_lint_levels(&manifest)?);
    let mut found: Vec<Finding> = undenied_finding(&missing, relative(workspace, &manifest))
        .into_iter()
        .collect();
    if let Some(config) = nearest_clippy_config(&cargo_workspace.root, workspace) {
        let file = relative(workspace, &config);
        for message in looser_settings(&clippy_settings_of(&config)?) {
            found.push(Finding::new("LNT-001", file.clone(), 0, message));
        }
    }
    Ok(found)
}

/// The finding of the manifest `file` that leaves the lints `missing` not
/// denied, or none when every one is.
fn undenied_finding(missing: &[&str], file: String) -> Option<Finding> {
    if missing.is_empty() {
        return None;
    }
    let verb = if missing.len() == 1 { "is" } else { "are" };
    let message = format!(
        "{} of the organization's lints {verb} not denied ({}); write them with rust-gate \
         lints --write",
        missing.len(),
        missing.join(", ")
    );
    Some(Finding::new("LNT-001", file, 0, message))
}

#[cfg(test)]
mod tests {
    use super::undenied_finding;

    #[test]
    fn lints_left_undenied_are_one_finding_on_the_manifest() {
        assert!(undenied_finding(&[], "Cargo.toml".to_owned()).is_none());
        let found = undenied_finding(&["clippy::a", "clippy::b"], "Cargo.toml".to_owned());
        let found = found.unwrap();
        assert_eq!(found.file, "Cargo.toml");
        assert_eq!(
            found.message,
            "2 of the organization's lints are not denied (clippy::a, clippy::b); write them \
             with rust-gate lints --write"
        );
    }
}
