//! `rust-gate duplication`: functions whose syntax trees look alike.
//! similarity-rs compares every pair of functions of at least `MIN_LINES`
//! lines and lists the pairs at or above `THRESHOLD`; a pair is reported, and
//! three functions or more that share one shape are refused, DUP-001, the rule
//! of three: the third copy is the signal to extract what they share. A shape
//! shared on purpose takes an exception in `maestro-quality.toml`. When the
//! tool itself fails, the step says so and does not guess.

use crate::checks::findings::{Finding, excuse, relative};
use crate::checks::quality_config;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, input, summary, write};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "duplication",
    summary: "Duplicated functions, pairs reported and three alike refused",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["jaq", "similarity-rs"],
    reports: &["duplication.txt"],
    run,
}];

/// Pairs at or above this similarity are listed; below it, two functions
/// merely share a shape.
const THRESHOLD: &str = "0.9";

/// Functions shorter than this are not compared: a four-line accessor looks
/// like every other four-line accessor.
const MIN_LINES: &str = "8";

/// How many functions of one shape make the rule of three.
const THIRD: usize = 3;

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("duplication.txt")?;
    let listing = Cmd::new("similarity-rs --threshold")
        .arg(THRESHOLD)
        .args(["--min-lines", MIN_LINES, "--exclude", "target"])
        .arg(&job.project)
        .cwd(&job.project)
        .capture()
        .ok();
    let Some(listing) = listing else {
        write(
            &report,
            b"NOT MEASURED: similarity-rs failed; see the log\n",
            false,
        )?;
        return summary(
            "## Duplicated functions\n\nNot measured: similarity-rs failed, see the log.\n",
        );
    };
    let workspace = fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read_config(&workspace)?;
    let found = shapes(&listing, &workspace);
    let (kept, excused) = excuse(found, &config.exceptions, &["DUP-001"], "");
    let pairs = pairs(&listing);
    let mut text = String::new();
    for finding in &kept {
        let _ = writeln!(text, "{finding}");
    }
    for (finding, reason) in &excused {
        let _ = writeln!(text, "EXCUSED {finding} (because {reason})");
    }
    let _ = write!(
        text,
        "# rust-gate duplication: pairs at or above {THRESHOLD} similarity, functions of \
         {MIN_LINES} lines or more\n# pairs: {pairs}\n{listing}"
    );
    write(&report, text.as_bytes(), false)?;
    summary(&format!(
        "## Duplicated functions\n\n{pairs} pairs at or above {THRESHOLD} similarity; {} \
         shapes shared by {THIRD} functions or more. The detail is `duplication.txt` in the \
         reports artifact.\n",
        kept.len()
    ))?;
    if kept.is_empty() {
        return Ok(());
    }
    let plural = if kept.len() == 1 { "" } else { "s" };
    Err(Failure::from(format!(
        "duplication: {} finding{plural}; each names its rule, its file and what to do",
        kept.len()
    )))
}

/// How many pairs a listing reports: similarity-rs prints one `Similarity:`
/// line per pair.
fn pairs(listing: &str) -> usize {
    listing
        .lines()
        .filter(|line| line.trim_start().starts_with("Similarity:"))
        .count()
}

/// DUP-001: every group of three functions or more that the listed pairs
/// join, named from its first function.
fn shapes(listing: &str, workspace: &Path) -> Vec<Finding> {
    let mut group: BTreeMap<String, String> = BTreeMap::new();
    for line in listing.lines() {
        let Some((left, right)) = line.trim().split_once(" <-> ") else {
            continue;
        };
        let (left, right) = (root(&mut group, left), root(&mut group, right));
        if left != right {
            let (leader, joined) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            group.insert(joined, leader);
        }
    }
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for function in group.keys().cloned().collect::<Vec<_>>() {
        let leader = root(&mut group, &function);
        members.entry(leader).or_default().push(function);
    }
    members
        .into_values()
        .filter(|functions| functions.len() >= THIRD)
        .map(|functions| finding(&functions, workspace))
        .collect()
}

/// The representative of the group `function` belongs to, joining it first
/// when it is new.
fn root(group: &mut BTreeMap<String, String>, function: &str) -> String {
    let mut current = function.to_owned();
    loop {
        let parent = group
            .entry(current.clone())
            .or_insert_with(|| current.clone())
            .clone();
        if parent == current {
            return current;
        }
        current = parent;
    }
}

/// The finding for one group, sorted, from its first function: its file,
/// its line and its name.
fn finding(functions: &[String], workspace: &Path) -> Finding {
    let mut sorted = functions.to_vec();
    sorted.sort();
    let first = sorted.first().cloned().unwrap_or_default();
    let (location, name) = first.split_once(" function ").unwrap_or((&first, ""));
    let (file, lines) = location.rsplit_once(':').unwrap_or((location, ""));
    let line = lines
        .split('-')
        .next()
        .and_then(|start| start.parse().ok())
        .unwrap_or(0);
    let names: Vec<&str> = sorted
        .iter()
        .map(|function| {
            function
                .split_once(" function ")
                .map_or("", |(_, name)| name)
        })
        .collect();
    let message = format!(
        "{} functions share one shape ({}); extract what they share: the third copy is the \
         signal",
        sorted.len(),
        names.join(", ")
    );
    Finding::new(
        "DUP-001",
        relative(workspace, Path::new(file)),
        line,
        message,
    )
    .about(name)
}

#[cfg(test)]
mod tests {
    use super::{pairs, shapes};
    use std::path::Path;

    #[test]
    fn pairs_are_counted_one_per_similarity_line() {
        let listing = "Duplicates in src/lib.rs:\n  a <-> b\n  Similarity: 92.00%\n  \
                       c <-> d\n  Similarity: 90.50%\nTotal duplicate pairs found: 2\n";
        assert_eq!(pairs(listing), 2);
        assert_eq!(pairs("No duplicate functions found!\n"), 0);
    }

    #[test]
    fn three_functions_joined_by_pairs_make_one_finding_and_a_pair_none() {
        let listing = concat!(
            "  /w/src/b.rs:40-60 function b <-> /w/src/a.rs:10-30 function a\n",
            "  Similarity: 92.00%\n",
            "  /w/src/c.rs:5-20 function c <-> /w/src/b.rs:40-60 function b\n",
            "  Similarity: 91.00%\n",
            "  /w/src/x.rs:1-9 function x <-> /w/src/y.rs:1-9 function y\n",
            "  Similarity: 95.00%\n",
        );
        let found = shapes(listing, Path::new("/w"));
        assert_eq!(
            found.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [
                "DUP-001 src/a.rs:10: 3 functions share one shape (a, b, c); extract what they \
              share: the third copy is the signal"
            ]
        );
        assert_eq!(found[0].item, "a");
    }
}
