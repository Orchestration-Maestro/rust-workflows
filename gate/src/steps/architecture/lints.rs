//! LNT-001 read from the files that carry it: every lint of the
//! organization's list denied in the root manifest, and a `clippy.toml` no
//! looser than the organization's.

use crate::checks::findings::{Finding, relative};
use crate::checks::lint_policy::{
    clippy_settings_of, looser_settings, manifest_lint_levels, undenied,
};
use crate::checks::manifests::Workspace;
use crate::runner::Failure;
use std::path::{Path, PathBuf};

/// Every LNT-001 finding of the Cargo workspace `cargo_workspace`.
pub(super) fn findings(
    cargo_workspace: &Workspace,
    workspace: &Path,
) -> Result<Vec<Finding>, Failure> {
    let manifest = cargo_workspace.root.join("Cargo.toml");
    let mut found = Vec::new();
    let missing = undenied(&manifest_lint_levels(&manifest)?);
    if !missing.is_empty() {
        let verb = if missing.len() == 1 { "is" } else { "are" };
        let message = format!(
            "{} of the organization's lints {verb} not denied ({}); write them with rust-gate \
             lints --write",
            missing.len(),
            missing.join(", ")
        );
        found.push(Finding::new(
            "LNT-001",
            relative(workspace, &manifest),
            0,
            message,
        ));
    }
    let Some(config) = nearest_clippy_config(&cargo_workspace.root, workspace) else {
        let message = "no clippy.toml holds the organization's thresholds; add one at the \
                       repository root"
            .to_owned();
        found.push(Finding::new(
            "LNT-001",
            "clippy.toml".to_owned(),
            0,
            message,
        ));
        return Ok(found);
    };
    let file = relative(workspace, &config);
    for message in looser_settings(&clippy_settings_of(&config)?) {
        found.push(Finding::new("LNT-001", file.clone(), 0, message));
    }
    Ok(found)
}

/// The `clippy.toml` Clippy reads for the workspace at `root`: the nearest
/// one walking up to the repository root.
fn nearest_clippy_config(root: &Path, workspace: &Path) -> Option<PathBuf> {
    root.ancestors()
        .take_while(|directory| directory.starts_with(workspace))
        .flat_map(|directory| ["clippy.toml", ".clippy.toml"].map(|name| directory.join(name)))
        .find(|file| file.is_file())
}

#[cfg(test)]
mod tests {
    use super::nearest_clippy_config;
    use std::env;
    use std::fs;
    use std::process;

    #[test]
    fn the_nearest_clippy_toml_inside_the_repository_is_read() {
        let root = env::temp_dir().join(format!("lnt-config-{}", process::id()));
        let project = root.join("crates/app");
        fs::create_dir_all(&project).unwrap();
        assert_eq!(nearest_clippy_config(&project, &root), None);
        fs::write(root.join("clippy.toml"), "").unwrap();
        assert_eq!(
            nearest_clippy_config(&project, &root),
            Some(root.join("clippy.toml"))
        );
        fs::write(project.join(".clippy.toml"), "").unwrap();
        assert_eq!(
            nearest_clippy_config(&project, &root),
            Some(project.join(".clippy.toml"))
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
