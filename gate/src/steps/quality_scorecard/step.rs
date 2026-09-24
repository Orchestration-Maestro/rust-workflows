//! `rust-gate scorecard`: what this run actually enforced, as data, prose and
//! a self-contained badge. A consumer can see which of the available controls
//! they switched on instead of assuming the defaults cover everything, and
//! it never claims a control that did not run.

use super::scorecard::{Control, Scorecard, State};
use crate::checks::inputs::{LicensePolicy, UnsafePolicy, license_policy, unsafe_policy};
use crate::runner::{Failure, Job, Outcome, Step, flag, input, optional, summary, write};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "scorecard",
    summary: "Build quality scorecard",
    inputs: &[
        "API_APPLIED",
        "API_COMPATIBILITY",
        "FEATURES_APPLIED",
        "GITHUB_SHA",
        "LICENSE_POLICY",
        "MUTANTS_APPLIED",
        "MUTATION_TEST",
        "OUT_API",
        "OUT_AUDIT",
        "OUT_COVERAGE",
        "OUT_FEATURES",
        "OUT_LICENCES",
        "OUT_MSRV",
        "OUT_MUTANTS",
        "OUT_QUALITY",
        "OUT_SECRETS",
        "OUT_STAGE",
        "OUT_UNUSED",
        "RUSTUP_TOOLCHAIN",
        "SARIF_REPORTS",
        "UNSAFE_POLICY",
        "UNUSED_DEPENDENCIES",
    ],
    tools: &[],
    reports: &["scorecard.json", "scorecard.md", "scorecard.svg"],
    run,
}];

/// Run the step: the controls gathered once, then written as JSON, as
/// Markdown for the log and the summary, and as the badge.
fn run() -> Outcome {
    let job = Job::current()?;
    // A failed coverage gate leaves no report; that is a missing value, not
    // an error to swallow, so the absence is handled rather than silenced.
    let coverage = std::fs::read_to_string(job.earlier("coverage.lcov"))
        .ok()
        .and_then(|lcov| line_coverage(&lcov));
    let scorecard = Scorecard {
        controls: controls()?,
        revision: input("GITHUB_SHA")?,
        toolchain: input("RUSTUP_TOOLCHAIN")?,
        coverage,
    };
    // Informational, so its absence is not an error: the line appears only
    // when the complexity step ran.
    let complexity = std::fs::read_to_string(job.earlier("complexity.json")).ok();
    let mut json = scorecard.json();
    if let Some(data) = &complexity {
        json = json.replacen("}\n", &format!(",\"complexity\":{}}}\n", data.trim()), 1);
    }
    write(&job.report("scorecard.json")?, json.as_bytes(), false)?;
    let mut markdown = scorecard.markdown();
    if let Some(data) = &complexity {
        markdown.push_str(&complexity_line(data));
    }
    print!("{markdown}");
    write(&job.report("scorecard.md")?, markdown.as_bytes(), false)?;
    summary(&markdown)?;
    write(
        &job.report("scorecard.svg")?,
        scorecard.badge().as_bytes(),
        false,
    )
}

/// Selection alone never proves execution. Steps that can find no applicable
/// work also supply an explicit application result; an absent result stays unrun.
fn controls() -> Result<Vec<Control>, Failure> {
    let sarif = match (
        input("OUT_QUALITY")?.as_str(),
        input("OUT_SECRETS")?.as_str(),
    ) {
        ("failure", _) | (_, "failure") => "failure",
        ("success", "success") => "success",
        _ => "skipped",
    };
    let state = |name: &str, enabled: bool, applied: &str| -> Result<State, Failure> {
        Ok(State::from_outcome(&input(name)?, enabled, applied))
    };
    Ok(vec![
        (
            "formatting, clippy, tests, rustdoc",
            "enforced",
            state("OUT_QUALITY", true, "true")?,
        ),
        (
            "line coverage",
            "enforced",
            state("OUT_COVERAGE", true, "true")?,
        ),
        ("advisories", "enforced", state("OUT_AUDIT", true, "true")?),
        (
            "secret scan",
            "enforced",
            state("OUT_SECRETS", true, "true")?,
        ),
        (
            "declared MSRV",
            "enforced",
            state("OUT_MSRV", true, "true")?,
        ),
        (
            "feature combinations",
            "enforced",
            state("OUT_FEATURES", true, &optional("FEATURES_APPLIED")?)?,
        ),
        (
            "packaging and SBOM",
            "enforced",
            state("OUT_STAGE", true, "true")?,
        ),
        (
            "sources, versions and licences",
            "enforced",
            state(
                "OUT_LICENCES",
                license_policy()? != LicensePolicy::Off,
                "true",
            )?,
        ),
        (
            "mutation testing",
            "optional",
            state(
                "OUT_MUTANTS",
                flag("MUTATION_TEST")?,
                &optional("MUTANTS_APPLIED")?,
            )?,
        ),
        (
            "API compatibility",
            "optional",
            state(
                "OUT_API",
                flag("API_COMPATIBILITY")?,
                &optional("API_APPLIED")?,
            )?,
        ),
        (
            "unused dependencies",
            "optional",
            state("OUT_UNUSED", flag("UNUSED_DEPENDENCIES")?, "true")?,
        ),
        (
            "unsafe denied",
            "optional",
            state(
                "OUT_QUALITY",
                unsafe_policy()? == UnsafePolicy::Deny,
                "true",
            )?,
        ),
        (
            "SARIF reports",
            "optional",
            State::from_outcome(sarif, flag("SARIF_REPORTS")?, "true"),
        ),
    ])
}

/// The informational line under the table: what the complexity step counted,
/// or that it could not measure.
fn complexity_line(data: &str) -> String {
    let number = |field: &str| -> String {
        data.split_once(&format!("\"{field}\":"))
            .map(|(_, rest)| rest.chars().take_while(char::is_ascii_digit).collect())
            .unwrap_or_default()
    };
    if data.contains("\"measured\":true") {
        format!(
            "\nComplexity, informational: {} functions over the size thresholds, {} files over \
             300 lines of code; see complexity.txt.\n",
            number("functions_over"),
            number("files_over")
        )
    } else {
        "\nComplexity, informational: not measured.\n".to_owned()
    }
}

/// Covered lines over instrumented lines from an LCOV report, one decimal,
/// or nothing when the report instruments no line.
fn line_coverage(lcov: &str) -> Option<String> {
    let (mut found, mut hit) = (0u32, 0u32);
    for line in lcov.lines() {
        if let Some(n) = line.strip_prefix("LF:") {
            found = found.saturating_add(n.trim().parse::<u32>().unwrap_or(0));
        } else if let Some(n) = line.strip_prefix("LH:") {
            hit = hit.saturating_add(n.trim().parse::<u32>().unwrap_or(0));
        }
    }
    (found > 0).then(|| format!("{:.1}", 100.0 * f64::from(hit) / f64::from(found)))
}

#[cfg(test)]
mod tests {
    use super::line_coverage;

    #[test]
    fn coverage_is_hit_over_found_lines() {
        assert_eq!(
            line_coverage("LF:10\nLH:9\nLF:10\nLH:8\n"),
            Some("85.0".into())
        );
        assert_eq!(line_coverage("LF:0\n"), None);
        assert_eq!(line_coverage(""), None);
    }
}
