//! `rust-gate publish-evidence <step>`: release reports archived locally,
//! then attached to an existing GitHub Release only with verified approval.

use crate::checks::release_boundary::{
    is_approved_environment, is_protected_release, upload_release,
};
use crate::checks::simple_names::is_hex;
use crate::runner::{Cmd, Outcome, Step, flag, input, non_empty};
use std::fs;
use std::path::Path;

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "publish-evidence",
        id: "authorize",
        summary: "Validate publication boundary",
        inputs: &["DRY_RUN", "EVENT", "GITHUB_REPOSITORY", "PROTECTED", "REF"],
        tools: &["gh", "jaq"],
        reports: &[],
        run: authorize,
    },
    Step {
        workflow: "publish-evidence",
        id: "validate",
        summary: "Validate evidence before publishing",
        inputs: &["GITHUB_SHA", "REVISION"],
        tools: &["jaq", "tar"],
        reports: &[],
        run: validate,
    },
    Step {
        workflow: "publish-evidence",
        id: "upload",
        summary: "Upload evidence to the existing release",
        inputs: &[
            "DRY_RUN",
            "EVENT",
            "GITHUB_REPOSITORY",
            "GITHUB_SHA",
            "PROTECTED",
            "REF",
            "REVISION",
        ],
        tools: &["gh", "jaq"],
        reports: &[],
        run: upload,
    },
];

/// No authorization API call or live write takes place during a dry run.
fn authorize() -> Outcome {
    if flag("DRY_RUN")? {
        return Ok(());
    }
    is_protected_release()?;
    is_approved_environment()
}

/// Check source identity and every entry before creating the archive. Reports
/// can describe failures; their presence is not a claim that CI succeeded.
fn validate() -> Outcome {
    let revision = input("REVISION")?;
    if !is_hex(&revision, 40) {
        return Err("revision must be an immutable commit SHA".into());
    }
    if revision != input("GITHUB_SHA")? {
        return Err("Evidence revision does not match this source".into());
    }
    if fs::symlink_metadata("evidence").is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("Evidence must not contain symlinks".into());
    }
    non_empty(Path::new("evidence/scorecard.json"))
        .map_err(|_| "Reports are missing the run scorecard")?;
    let root = fs::canonicalize("evidence").map_err(|error| format!("evidence: {error}"))?;
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("evidence: {error}"))?;
            let file = entry.path();
            let kind = entry
                .file_type()
                .map_err(|error| format!("evidence: {error}"))?;
            if kind.is_symlink() {
                return Err("Evidence must not contain symlinks".into());
            }
            if !kind.is_file() && !kind.is_dir() {
                return Err("Evidence entries must be regular files or directories".into());
            }
            let real =
                fs::canonicalize(&file).map_err(|error| format!("{}: {error}", file.display()))?;
            inside_evidence(&real, &root)?;
            if kind.is_dir() {
                pending.push(real);
            }
        }
    }
    let matching = Cmd::new("jaq -e --arg revision")
        .arg(&revision)
        .args([".revision == $revision", "evidence/scorecard.json"])
        .capture()
        .map_err(|_| "Scorecard revision does not match this source")?;
    if matching.trim() != "true" {
        return Err("Scorecard revision does not match this source".into());
    }
    Cmd::new("tar -czf evidence.tar.gz -C evidence .").run()
}

/// The workflow validates the reports again in the protected publishing job.
fn upload() -> Outcome {
    upload_release(&["evidence.tar.gz"])
}

/// Refuse an entry whose real path left the evidence directory.
fn inside_evidence(real: &Path, root: &Path) -> Outcome {
    if !real.starts_with(root) || real == root {
        return Err("Evidence escapes its directory".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::inside_evidence;
    use std::path::Path;

    #[test]
    fn an_entry_outside_the_evidence_directory_is_refused() {
        let root = Path::new("/tmp/evidence");
        assert!(inside_evidence(Path::new("/tmp/evidence/scorecard.json"), root).is_ok());
        for outside in ["/tmp/evidence", "/tmp/elsewhere/scorecard.json"] {
            let error = inside_evidence(Path::new(outside), root).unwrap_err();
            assert_eq!(
                error.message.as_deref(),
                Some("Evidence escapes its directory")
            );
        }
    }
}
