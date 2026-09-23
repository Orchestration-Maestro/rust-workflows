//! `rust-gate duplication`: functions whose syntax trees look alike, reported
//! and never enforced. similarity-rs compares every pair of functions of at
//! least `MIN_LINES` lines and lists the pairs at or above `THRESHOLD`; the
//! step exits zero whatever it finds, and says so when the tool itself fails.
//! The listing is information for the consumer: a pair worth merging, or a
//! shape two functions share on purpose.

use crate::runner::{Cmd, Job, Outcome, Step, summary, write};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "duplication",
    summary: "Duplicated functions, never blocking",
    inputs: &[],
    tools: &["similarity-rs"],
    reports: &["duplication.txt"],
    run,
}];

/// Pairs at or above this similarity are listed; below it, two functions
/// merely share a shape.
const THRESHOLD: &str = "0.9";

/// Functions shorter than this are not compared: a four-line accessor looks
/// like every other four-line accessor.
const MIN_LINES: &str = "8";

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
    let (text, line) = match listing {
        Some(listing) => {
            let pairs = pairs(&listing);
            let header = format!(
                "# rust-gate duplication, informational: pairs at or above \
                 {THRESHOLD} similarity, functions of {MIN_LINES} lines or more\n"
            );
            (
                format!("{header}# pairs: {pairs}\n{listing}"),
                format!(
                    "{pairs} pairs of functions at or above {THRESHOLD} similarity, \
                     {MIN_LINES} lines or more each"
                ),
            )
        }
        None => (
            "NOT MEASURED: similarity-rs failed; see the log\n".to_owned(),
            "not measured: similarity-rs failed, see the log".to_owned(),
        ),
    };
    write(&report, text.as_bytes(), false)?;
    summary(&format!(
        "## Duplicated functions, informational\n\n{line}. Nothing here fails the run; \
         the detail is `duplication.txt` in the reports artifact.\n"
    ))
}

/// How many pairs a listing reports: similarity-rs prints one `Similarity:`
/// line per pair.
fn pairs(listing: &str) -> usize {
    listing
        .lines()
        .filter(|line| line.trim_start().starts_with("Similarity:"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::pairs;

    #[test]
    fn pairs_are_counted_one_per_similarity_line() {
        let listing = "Duplicates in src/lib.rs:\n  a <-> b\n  Similarity: 92.00%\n  \
                       c <-> d\n  Similarity: 90.50%\nTotal duplicate pairs found: 2\n";
        assert_eq!(pairs(listing), 2);
        assert_eq!(pairs("No duplicate functions found!\n"), 0);
    }
}
