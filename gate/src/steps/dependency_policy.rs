//! `rust-gate licenses`: DEP-001, the organization's dependency policy, for
//! every repository: one version of each crate, no wildcard requirement,
//! crates.io alone, no yanked or unmaintained crate, and the licences the
//! organization reviewed, with the `LICENSE_ALLOWLIST` organization variable's
//! licences added when administrators set it. The gate renders it at run time
//! with the duplicate versions `maestro-quality.toml` excuses; a repository
//! does not commit a `deny.toml` of its own, and one that does is refused.
//! No repository opts out: `validate` refuses `license-policy: off`.

use crate::checks::organization_config::is_generated;
use crate::checks::quality_config::{QualityConfig, read_config};
use crate::runner::{Cmd, Job, Outcome, Step, input, path, tee_line};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "licenses",
    summary: "Licence, dependency-ban and source policy",
    inputs: &["DENY_CONFIG", "GITHUB_WORKSPACE", "LICENSE_ALLOWLIST"],
    tools: &["cargo deny", "jaq"],
    reports: &["licenses.txt"],
    run,
}];

/// The licences the organization reviewed.
const LICENCES: [&str; 5] = ["Apache-2.0", "MIT", "MIT-0", "Unicode-3.0", "Unlicense"];

/// DEP-001's cargo-deny policy after its licence list, up to its exceptions.
const POLICY: &str = concat!(
    "confidence-threshold = 0.93\n",
    "unused-allowed-license = \"allow\"\n",
    "private = { ignore = true }\n",
    "\n",
    "[advisories]\n",
    "yanked = \"deny\"\n",
    "unmaintained = \"all\"\n",
    "\n",
    "[sources]\n",
    "unknown-registry = \"deny\"\n",
    "unknown-git = \"deny\"\n",
    "allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]\n",
    "\n",
    "[bans]\n",
    "multiple-versions = \"deny\"\n",
    "wildcards = \"deny\"\n",
);

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("licenses.txt")?;
    let committed = input("DENY_CONFIG")?;
    if !committed.is_empty() && !is_generated(Path::new(&committed)) {
        return Err(
            "deny.toml: the organization's DEP-001 policy applies to every repository; \
                    delete deny.toml and take exceptions in maestro-quality.toml"
                .into(),
        );
    }
    let allowlist = input("LICENSE_ALLOWLIST")?;
    if !allowlist.is_empty() && !is_spdx_list(&allowlist) {
        return Err("LICENSE_ALLOWLIST must be comma-separated SPDX identifiers".into());
    }
    let added: Vec<&str> = allowlist.split(',').filter(|id| !id.is_empty()).collect();
    let config = read_config(&path("GITHUB_WORKSPACE")?)?;
    let policy = job.temp.join("organization-deny.toml");
    fs::write(&policy, deny_policy(&config, &added))
        .map_err(|error| format!("cannot write {}: {error}", policy.display()))?;
    tee_line(
        "policy: the organization's DEP-001, rendered by the gate: one version of each crate, \
         no wildcard, crates.io alone, no yanked or unmaintained crate, reviewed licences",
        &report,
        false,
    )?;
    if !added.is_empty() {
        tee_line(
            &format!("organization licence allowlist: {allowlist}"),
            &report,
            true,
        )?;
    }
    // cargo-audit reports published vulnerabilities; `advisories` is here
    // for what it does not cover, a crate yanked from the registry.
    Cmd::new("cargo deny --config")
        .arg(&policy)
        .args(["check", "licenses", "bans", "sources", "advisories"])
        .cwd(&job.project)
        .tee(&report, true)
}

/// DEP-001's policy: the reviewed licences and the `added` ones, then, as
/// cargo-deny's skips, the duplicate versions `maestro-quality.toml`
/// excuses, `path` naming the crate and its version, `windows-sys@0.52`.
fn deny_policy(config: &QualityConfig, added: &[&str]) -> String {
    let mut text = String::from("[graph]\nall-features = true\n\n[licenses]\nallow = [\n");
    let extra = added.iter().filter(|id| !LICENCES.contains(id));
    for licence in LICENCES.iter().chain(extra) {
        let _ = writeln!(text, "  \"{licence}\",");
    }
    text.push_str("]\n");
    text.push_str(POLICY);
    let skips: Vec<_> = config
        .exceptions
        .iter()
        .filter(|exception| exception.rule == "DEP-001")
        .collect();
    if !skips.is_empty() {
        text.push_str("skip = [\n");
        for exception in skips {
            let reason = exception.reason.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = writeln!(
                text,
                "  {{ crate = \"{}\", reason = \"{reason}\" }},",
                exception.path
            );
        }
        text.push_str("]\n");
    }
    text
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
    use super::{deny_policy, is_spdx_list};
    use crate::checks::quality_config::{Exception, QualityConfig};

    #[test]
    fn allowlists_are_comma_separated_identifiers() {
        assert!(is_spdx_list(
            "MIT,Apache-2.0,Apache-2.0 WITH LLVM-exception"
        ));
        for bad in ["", "MIT;GPL-3.0", "MIT,", ",MIT", "MIT\"", "MIT\n"] {
            assert!(!is_spdx_list(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_policy_adds_listed_licences_and_excused_duplicates() {
        let plain = deny_policy(&QualityConfig::default(), &[]);
        assert!(plain.starts_with(
            "[graph]\nall-features = true\n\n[licenses]\nallow = [\n  \"Apache-2.0\",\n"
        ));
        assert!(plain.ends_with("multiple-versions = \"deny\"\nwildcards = \"deny\"\n"));
        let config = QualityConfig {
            exceptions: vec![Exception {
                rule: "DEP-001".to_owned(),
                path: "windows-sys@0.52".to_owned(),
                item: String::new(),
                reason: "two \"platform\" crates".to_owned(),
            }],
            ..QualityConfig::default()
        };
        let text = deny_policy(&config, &["MIT", "ISC"]);
        assert_eq!(text.matches("\"MIT\"").count(), 1);
        assert!(text.contains("  \"Unlicense\",\n  \"ISC\",\n]\n"), "{text}");
        assert!(text.ends_with(
            "skip = [\n  { crate = \"windows-sys@0.52\", reason = \"two \\\"platform\\\" \
             crates\" },\n]\n"
        ));
    }
}
