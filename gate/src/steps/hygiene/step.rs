//! The hygiene step: the files git tracks in the checkout, the HYG rules and
//! the width of shell scripts run over them, the exceptions
//! `maestro-quality.toml` takes, and one report line per finding.

use crate::checks::findings::{excuse, publish_findings};
use crate::checks::quality_config;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, input};
use std::fs;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "hygiene",
    summary: "Repository hygiene HYG and the width of shell scripts",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["git", "jaq"],
    reports: &["hygiene.txt"],
    run,
}];

/// The rules this step runs: the exceptions it judges are theirs.
const RULES: &[&str] = &[
    "HYG-001", "HYG-002", "HYG-003", "HYG-004", "HYG-005", "SIZE-003",
];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let workspace = fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read_config(&workspace)?;
    let files = tracked_files(&workspace)?;
    let mut found = super::comments::findings(&workspace, &files);
    found.extend(super::files::findings(&workspace, &files));
    found.extend(super::widths::findings(&workspace, &files, config.limits));
    found.sort();
    found.dedup();
    let (kept, excused) = excuse(found, &config.exceptions, RULES, "");
    let report = job.report("hygiene.txt")?;
    publish_findings(&report, "Hygiene", &kept, &excused, &[])
}

/// Every file git tracks in the checkout, relative to it, and every new
/// file it does not ignore, so a local run sees what the next commit adds.
fn tracked_files(workspace: &Path) -> Result<Vec<String>, Failure> {
    let listing = Cmd::new("git -C")
        .arg(workspace)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .capture()?;
    Ok(split_listing(&listing))
}

/// The paths of a NUL-separated listing.
fn split_listing(listing: &str) -> Vec<String> {
    listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::split_listing;

    #[test]
    fn a_nul_separated_listing_splits_into_its_paths() {
        assert_eq!(split_listing("a b.md\0src/c.rs\0"), ["a b.md", "src/c.rs"]);
        assert!(split_listing("").is_empty());
    }
}
