//! The hygiene step: the files git tracks in the checkout, the HYG rules and
//! the width of shell scripts run over them, the exceptions
//! `maestro-quality.toml` takes, and one report line per finding.

use crate::checks::findings::{excuse, publish_findings};
use crate::checks::quality_config;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, input};
use std::collections::BTreeSet;
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
    "HYG-001", "HYG-002", "HYG-003", "HYG-004", "HYG-005", "HYG-006", "HYG-007", "SIZE-003",
];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let workspace = fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read_config(&workspace)?;
    let files = tracked_files(&workspace)?;
    let executables = executable_files(&workspace)?;
    let mut found = super::comments::findings(&workspace, &files);
    found.extend(super::files::findings(&workspace, &files));
    found.extend(super::names::findings(&files, &executables));
    found.extend(super::words::findings(&workspace, &files));
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

/// Every file git's index records with the executable mode, `100755`.
fn executable_files(workspace: &Path) -> Result<BTreeSet<String>, Failure> {
    let listing = Cmd::new("git -C")
        .arg(workspace)
        .args(["ls-files", "-z", "--stage"])
        .capture()?;
    Ok(executables_of(&listing))
}

/// The paths a NUL-separated `git ls-files --stage` listing records as
/// executable: each entry is the mode, the object, the stage, a tab and the
/// path.
fn executables_of(listing: &str) -> BTreeSet<String> {
    split_listing(listing)
        .into_iter()
        .filter_map(|entry| {
            let (fields, path) = entry.split_once('\t')?;
            fields.starts_with("100755 ").then(|| path.to_owned())
        })
        .collect()
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
    use super::{executables_of, split_listing};

    #[test]
    fn the_index_mode_says_which_files_are_executable() {
        let listing = concat!(
            "100755 0123 0\tscripts/run.sh\0",
            "100644 4567 0\tREADME.md\0",
            "120000 89ab 0\tlink\0",
        );
        assert_eq!(
            executables_of(listing).into_iter().collect::<Vec<_>>(),
            ["scripts/run.sh"]
        );
    }

    #[test]
    fn a_nul_separated_listing_splits_into_its_paths() {
        assert_eq!(split_listing("a b.md\0src/c.rs\0"), ["a b.md", "src/c.rs"]);
        assert!(split_listing("").is_empty());
    }
}
