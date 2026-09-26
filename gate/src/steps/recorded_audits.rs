//! `rust-gate vet`: every dependency must have a recorded cargo-vet audit.
//! cargo-vet needs a committed ledger; without one it would offer to create
//! it, so the ledger is required explicitly rather than auditing nothing.
//! VET-001: the ledger imports the audits the organization publishes and
//! those of Mozilla, Google, the Bytecode Alliance, ISRG and the Zcash
//! Foundation, so a crate one of them reviewed needs no exemption here; the
//! exemptions stay the repository's own reviewed state.

use crate::checks::workflow_home::{NAMES, ORGANIZATION};
use crate::runner::{Cmd, Failure, Job, Outcome, Step, flag, tee_line};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "vet",
    summary: "Recorded dependency audits",
    inputs: &["DEPENDENCY_AUDIT"],
    tools: &["cargo vet", "jaq"],
    reports: &["dependency-audit.txt"],
    run,
}];

/// The name of the organization's own import, whose audits file lives in the
/// home repository.
const OWN_IMPORT: &str = "orchestration-maestro";

/// Every other import VET-001 requires: its name and the audits file it reads.
const IMPORTS: &[(&str, &str)] = &[
    (
        "mozilla",
        "https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml",
    ),
    (
        "google",
        "https://raw.githubusercontent.com/google/supply-chain/main/audits.toml",
    ),
    (
        "bytecode-alliance",
        "https://raw.githubusercontent.com/bytecodealliance/wasmtime/main/supply-chain/audits.toml",
    ),
    (
        "isrg",
        "https://raw.githubusercontent.com/divviup/libprio-rs/main/supply-chain/audits.toml",
    ),
    (
        "zcash",
        "https://raw.githubusercontent.com/zcash/rust-ecosystem/main/supply-chain/audits.toml",
    ),
];

/// Each import of a cargo-vet config and its URLs, tab-separated.
const IMPORTED: &str = ".imports // {} | to_entries[] | [.key, (.value.url | if type == \
    \"array\" then join(\" \") else . end)] | @tsv";

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("dependency-audit.txt")?;
    if !flag("DEPENDENCY_AUDIT")? {
        return tee_line("SKIPPED: dependency-audit=false", &report, false);
    }
    let project = &job.project;
    let config = project.join("supply-chain/config.toml");
    if !config.is_file() {
        return Err(
            "dependency-audit=true requires a committed supply-chain/config.toml; \
             run cargo vet init"
                .into(),
        );
    }
    let imported = Cmd::new("jaq --from toml -r")
        .arg(IMPORTED)
        .arg(&config)
        .capture()?;
    let missing = missing_imports(&imported);
    if !missing.is_empty() {
        return Err(Failure::from(format!(
            "vet: supply-chain/config.toml does not import {} (VET-001); import the \
             organization's audits and those of Mozilla, Google, the Bytecode Alliance, ISRG \
             and the Zcash Foundation, then run cargo vet regenerate imports",
            missing.join(", ")
        )));
    }
    Cmd::new("cargo vet --locked")
        .cwd(project)
        .tee(&report, false)
}

/// Every import VET-001 requires, the organization's first: its name and the
/// audits files it may read, the organization's under every name its home
/// repository answers to.
fn required_imports() -> Vec<(&'static str, Vec<String>)> {
    let own = NAMES
        .iter()
        .map(|name| {
            format!(
                "https://raw.githubusercontent.com/{ORGANIZATION}/{name}/main/supply-chain/\
                 audits.toml"
            )
        })
        .collect();
    let others = IMPORTS
        .iter()
        .map(|(name, url)| (*name, vec![(*url).to_owned()]));
    [(OWN_IMPORT, own)].into_iter().chain(others).collect()
}

/// The required imports `imported` lacks, or names with another URL.
fn missing_imports(imported: &str) -> Vec<&'static str> {
    required_imports()
        .into_iter()
        .filter(|(name, accepted)| {
            !imported.lines().any(|line| {
                line.split_once('\t').is_some_and(|(key, urls)| {
                    key == *name
                        && urls
                            .split(' ')
                            .any(|found| accepted.iter().any(|url| url == found))
                })
            })
        })
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{missing_imports, required_imports};

    #[test]
    fn every_required_import_is_named_with_its_url() {
        let imports = required_imports();
        let own = "https://raw.githubusercontent.com/Orchestration-Maestro/rust-workflows/main/\
                   supply-chain/audits.toml";
        assert_eq!(
            imports.first().map(|(name, urls)| (*name, urls.first())),
            Some(("orchestration-maestro", Some(&own.to_owned())))
        );
        let all = imports
            .iter()
            .map(|(name, urls)| format!("{name}\t{}", urls.first().unwrap()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(missing_imports(&all).is_empty());
        let moved = all.replace("mozilla/supply-chain", "mozilla/elsewhere");
        assert_eq!(missing_imports(&moved), ["mozilla"]);
        assert_eq!(missing_imports("").len(), imports.len());
        let renamed = all.replace("/rust-workflows/", "/maestro-rust-workflows/");
        assert!(missing_imports(&renamed).is_empty());
        let elsewhere = all.replace("/rust-workflows/", "/other-workflows/");
        assert_eq!(missing_imports(&elsewhere), ["orchestration-maestro"]);
    }
}
