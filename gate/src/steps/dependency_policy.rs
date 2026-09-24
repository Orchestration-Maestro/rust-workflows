//! `rust-gate licenses`: the consumer's own `deny.toml` when committed, and
//! otherwise the line every project must hold, generated here: dependencies
//! from crates.io directly, no unapproved git dependency, no wildcard version
//! requirement, and the organization licence
//! allowlist when administrators set one. No list is assumed.

use crate::checks::inputs::{LicensePolicy, license_policy};
use crate::runner::{Cmd, Job, Outcome, Step, input, tee_line};
use std::fmt::Write as _;
use std::fs;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "licenses",
    summary: "Licence, dependency-ban and source policy",
    inputs: &["DENY_CONFIG", "LICENSE_ALLOWLIST", "LICENSE_POLICY"],
    tools: &["cargo deny"],
    reports: &["licenses.txt"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let project = &job.project;
    let report = job.report("licenses.txt")?;
    if license_policy()? == LicensePolicy::Off {
        return tee_line("SKIPPED: license-policy=off", &report, false);
    }
    let committed = input("DENY_CONFIG")?;
    if !committed.is_empty() {
        // cargo-audit reports published vulnerabilities; `advisories` is here
        // for what it does not cover, a crate yanked from the registry.
        return Cmd::new("cargo deny --config")
            .arg(&committed)
            .args(["check", "licenses", "bans", "sources", "advisories"])
            .cwd(project)
            .tee(&report, false);
    }
    let generated = job.temp.join("default-deny.toml");
    let mut policy = String::from(
        "[bans]\nwildcards = \"deny\"\n\n[sources]\nunknown-registry = \"deny\"\n\
         unknown-git = \"deny\"\n\
         allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]\n",
    );
    let mut checks = vec!["bans", "sources"];
    let allowlist = input("LICENSE_ALLOWLIST")?;
    if allowlist.is_empty() {
        println!(
            "::notice title=Licence allowlist not applied::Commit a deny.toml, or have \
             administrators set the LICENSE_ALLOWLIST organization variable, to enforce \
             a licence policy."
        );
        tee_line(
            "licences NOT APPLIED: no deny.toml in the working directory \
             and no LICENSE_ALLOWLIST variable",
            &report,
            false,
        )?;
    } else {
        if !is_spdx_list(&allowlist) {
            return Err("LICENSE_ALLOWLIST must be comma-separated SPDX identifiers".into());
        }
        let quoted: Vec<String> = allowlist.split(',').map(|id| format!("\"{id}\"")).collect();
        let _ = write!(policy, "\n[licenses]\nallow = [{}]\n", quoted.join(", "));
        checks.push("licenses");
        tee_line(
            &format!("organization licence allowlist: {allowlist}"),
            &report,
            false,
        )?;
    }
    fs::write(&generated, policy)
        .map_err(|error| format!("cannot write {}: {error}", generated.display()))?;
    tee_line(
        "default policy: approved registries only, no git dependencies, no wildcard versions",
        &report,
        true,
    )?;
    Cmd::new("cargo deny --config")
        .arg(&generated)
        .arg("check")
        .args(&checks)
        .cwd(project)
        .tee(&report, true)
}

/// Comma-separated SPDX identifiers: letters, digits, dot, plus, space and
/// hyphen, none of them empty.
fn is_spdx_list(value: &str) -> bool {
    value.split(',').all(|id| {
        !id.is_empty()
            && id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ".+ -".contains(character))
    })
}

#[cfg(test)]
mod tests {
    use super::is_spdx_list;

    #[test]
    fn allowlists_are_comma_separated_identifiers() {
        assert!(is_spdx_list(
            "MIT,Apache-2.0,Apache-2.0 WITH LLVM-exception"
        ));
        for bad in ["", "MIT;GPL-3.0", "MIT,", ",MIT", "MIT\"", "MIT\n"] {
            assert!(!is_spdx_list(bad), "{bad:?}");
        }
    }
}
