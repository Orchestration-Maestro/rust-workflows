//! `rust-gate secrets`: Gitleaks over an archive of the current revision, not
//! its history, with built-in rules only, every finding redacted, and no
//! consumer allowlist honoured.

use crate::runner::{Cmd, Job, Outcome, Step, flag, non_empty, path};
use std::fs;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "secrets",
    summary: "Redacted source secret scan",
    inputs: &["GITHUB_WORKSPACE", "SARIF_REPORTS"],
    tools: &["git", "gitleaks", "tar"],
    reports: &["secrets.json", "secrets.sarif"],
    run,
}];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let temp = &job.temp;
    let source = temp.join("secret-source");
    fs::create_dir_all(&source)
        .map_err(|error| format!("cannot create {}: {error}", source.display()))?;
    let archive = Cmd::new("git -C")
        .arg(path("GITHUB_WORKSPACE")?)
        .args(["archive", "HEAD"])
        .capture_bytes()?;
    Cmd::new("tar -x -C")
        .arg(&source)
        .stdin_bytes(&archive)
        .run()?;
    let config = temp.join("gitleaks.toml");
    fs::write(&config, "[extend]\nuseDefault = true\n")
        .map_err(|error| format!("cannot write {}: {error}", config.display()))?;
    let scan = |format: &str, report: &Path| -> Outcome {
        Cmd::new("gitleaks dir")
            .arg(&source)
            .arg("--config")
            .arg(&config)
            .arg("--gitleaks-ignore-path")
            .arg(temp.join("no-ignore-file"))
            .args([
                "--ignore-gitleaks-allow",
                "--redact=100",
                "--no-banner",
                "--report-format",
            ])
            .arg(format)
            .arg("--report-path")
            .arg(report)
            .run()?;
        non_empty(report)
    };
    scan("json", &job.report("secrets.json")?)?;
    if flag("SARIF_REPORTS")? {
        // Findings stay redacted in SARIF too; the report records locations,
        // never the matched secret value.
        scan("sarif", &job.report("secrets.sarif")?)?;
    }
    Ok(())
}
