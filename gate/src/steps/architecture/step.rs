//! The architecture step: every target's module tree, the ARC rules run over
//! it, the exceptions `maestro-quality.toml` takes, and one report line per
//! finding.

use crate::checks::findings::{Finding, excuse, relative};
use crate::checks::module_tree::module_trees;
use crate::checks::quality_config::{self, QualityConfig};
use crate::runner::{Failure, Job, Outcome, Step, input, summary, write};
use std::fmt::Write as _;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "architecture",
    summary: "Module structure rules ARC-001 to ARC-007",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["cargo metadata", "jaq"],
    reports: &["architecture.txt"],
    run,
}];

/// The rules this step runs: the exceptions it judges are theirs.
const RULES: &[&str] = &[
    "ARC-001", "ARC-002", "ARC-003", "ARC-004", "ARC-005", "ARC-006", "ARC-007",
];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let workspace = std::fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read_config(&workspace)?;
    let scope = scope(&workspace, &job.project);
    let found = findings(&job, &workspace, &config, &scope)?;
    let (kept, excused) = excuse(found, &config.exceptions, RULES, &scope);
    report(&job, &kept, &excused)
}

/// Every finding of every rule over every target of the project.
fn findings(
    job: &Job,
    workspace: &Path,
    config: &QualityConfig,
    scope: &str,
) -> Result<Vec<Finding>, Failure> {
    let trees = module_trees(&job.project, &job.temp)?;
    let mut found = super::layers::unknown_roots(&trees, workspace, &config.layers, scope);
    for tree in &trees {
        found.extend(super::cycles::findings(tree, workspace));
        found.extend(super::doors::contents(tree, workspace));
        found.extend(super::doors::bypasses(tree, workspace));
        found.extend(super::layers::findings(tree, workspace, &config.layers));
        found.extend(super::seams::findings(tree, workspace));
        found.extend(super::roots::thin_roots(tree, workspace));
        found.extend(super::roots::path_attributes(tree, workspace));
    }
    found.sort();
    found.dedup();
    Ok(found)
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

/// Write every finding, then every excused one with its reason, to the
/// report and the summary, and fail when a finding is left.
fn report(job: &Job, kept: &[Finding], excused: &[(Finding, &str)]) -> Outcome {
    let mut text = String::new();
    for finding in kept {
        let _ = writeln!(text, "{finding}");
    }
    for (finding, reason) in excused {
        let _ = writeln!(text, "EXCUSED {finding} (because {reason})");
    }
    write(&job.report("architecture.txt")?, text.as_bytes(), false)?;
    if kept.is_empty() {
        println!("Module structure: no finding, {} excused", excused.len());
        return summary(&format!(
            "### Module structure\n\nNo finding; {} excused.\n",
            excused.len()
        ));
    }
    eprint!("{text}");
    summary(&format!("### Module structure\n\n```text\n{text}```\n"))?;
    let plural = if kept.len() == 1 { "" } else { "s" };
    Err(Failure::from(format!(
        "module structure: {} finding{plural}; each names its rule, its file and what to do",
        kept.len()
    )))
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
