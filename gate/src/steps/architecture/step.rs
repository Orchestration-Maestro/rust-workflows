//! The architecture step: every target's module tree and every package's
//! manifest, the source rules run over them, the exceptions
//! `maestro-quality.toml` takes, and one report line per finding.

use crate::checks::findings::{Finding, excuse, publish_findings, relative};
use crate::checks::manifests::{cargo_packages, read_cargo_metadata, workspace_of};
use crate::checks::module_tree::module_trees;
use crate::checks::quality_config::{self, QualityConfig};
use crate::runner::{Failure, Job, Outcome, Step, input};
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "architecture",
    summary: "Source rules ARC, SIZE, NAME, DOC, LIB, TST and WSP",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["cargo metadata", "git", "jaq"],
    reports: &["architecture.txt"],
    run,
}];

/// The rules this step runs: the exceptions it judges are theirs.
const RULES: &[&str] = &[
    "ARC-001", "ARC-002", "ARC-003", "ARC-004", "ARC-005", "ARC-006", "ARC-007", "SIZE-002",
    "SIZE-003", "NAME-001", "NAME-002", "DOC-001", "LIB-001", "LIB-002", "TST-001", "TST-003",
    "WSP-001", "WSP-002",
];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let workspace = std::fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read_config(&workspace)?;
    let scope = scope(&workspace, &job.project);
    let (found, notes) = findings(&job, &workspace, &config, &scope)?;
    let (kept, excused) = excuse(found, &config.exceptions, RULES, &scope);
    let report = job.report("architecture.txt")?;
    publish_findings(&report, "Source rules", &kept, &excused, &notes)
}

/// Every finding of every rule over every target and package of the
/// project, and the report lines of the files past three hundred lines.
fn findings(
    job: &Job,
    workspace: &Path,
    config: &QualityConfig,
    scope: &str,
) -> Result<(Vec<Finding>, Vec<String>), Failure> {
    let metadata = read_cargo_metadata(&job.project, &job.temp)?;
    let trees = module_trees(&metadata)?;
    let packages = cargo_packages(&metadata)?;
    let cargo_workspace = workspace_of(&metadata)?;
    let mut found = super::layers::unknown_roots(&trees, workspace, &config.layers, scope);
    for tree in &trees {
        found.extend(super::cycles::findings(tree, workspace));
        found.extend(super::doors::contents(tree, workspace));
        found.extend(super::doors::bypasses(tree, workspace));
        found.extend(super::layers::findings(tree, workspace, &config.layers));
        found.extend(super::seams::findings(tree, workspace));
        found.extend(super::roots::thin_roots(tree, workspace));
        found.extend(super::roots::path_attributes(tree, workspace));
        found.extend(super::sources::test_sleeps(tree, workspace));
    }
    let (sizes, notes) = super::sizes::findings(&trees, workspace, config.limits);
    found.extend(sizes);
    found.extend(super::names::packages(&packages, workspace));
    found.extend(super::names::tests(&trees, workspace));
    found.extend(super::sources::module_comments(&trees, workspace));
    found.extend(super::sources::library_prints(&trees, workspace));
    found.extend(super::packages::findings(
        &packages,
        &cargo_workspace,
        workspace,
    )?);
    found.sort();
    found.dedup();
    Ok((found, notes))
}

/// The project's directory relative to the repository with a trailing
/// slash, or nothing when the project is the repository: this run judges the
/// exceptions under it.
fn scope(workspace: &Path, project: &Path) -> String {
    let project = std::fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf());
    let directory = relative(workspace, &project);
    if directory.is_empty() {
        directory
    } else {
        format!("{directory}/")
    }
}

#[cfg(test)]
mod tests {
    use super::scope;
    use std::path::Path;

    #[test]
    fn the_scope_is_the_project_directory_under_the_repository() {
        let workspace = Path::new("/w");
        assert_eq!(scope(workspace, Path::new("/w")), "");
        assert_eq!(scope(workspace, Path::new("/w/gate")), "gate/");
    }
}
