//! `rust-gate gate-rules`: the gate's list of rules as it is kept, one rule a
//! line, its comments left out: the ID, the short name, `exception` or `none`,
//! and what the rule holds, separated by tabs. The organization's `.github`
//! renders its page's gate rules from it at every release.

use crate::checks::gate_rules::rules;
use crate::runner::{Failure, Outcome, Step};
use std::io::{self, Write as _};

/// What this command declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "local",
    id: "gate-rules",
    summary: "The gate's rules, one a line: ID, name, whether it takes an exception, what it holds",
    inputs: &[],
    tools: &[],
    reports: &[],
    run,
}];

/// Run the command; a reader that closes the pipe early is a refusal.
fn run() -> Outcome {
    io::stdout()
        .write_all(listing().as_bytes())
        .map_err(|error| Failure::from(format!("gate-rules: {error}")))
}

/// The list, one rule a line.
fn listing() -> String {
    rules().flat_map(|line| [line, "\n"]).collect()
}

#[cfg(test)]
mod tests {
    use super::listing;

    #[test]
    fn the_listing_holds_every_rule_and_no_comment() {
        let listing = listing();
        assert!(listing.starts_with("ARC-001\tNo import cycle\tnone\t"));
        assert!(listing.lines().all(|line| !line.starts_with('#')));
        assert!(listing.contains("\nPRF-001\tPerformance budget\texception\t"));
    }
}
