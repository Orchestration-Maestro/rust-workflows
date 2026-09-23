//! `rust-gate publish-binaries <step>`: authorize and upload verified assets
//! to an existing GitHub Release, without rebuilding or replacing files.

use crate::checks::release_boundary::{
    is_approved_environment, is_protected_release, upload_release,
};
use crate::runner::{Outcome, Step, flag};

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "publish-binaries",
        id: "authorize",
        summary: "Validate publication boundary",
        inputs: &["DRY_RUN", "EVENT", "GITHUB_REPOSITORY", "PROTECTED", "REF"],
        tools: &["gh", "jaq"],
        reports: &[],
        run: authorize,
    },
    Step {
        workflow: "publish-binaries",
        id: "publish",
        summary: "Upload verified binaries to the existing release",
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
        run: publish,
    },
];

/// Dry-run never queries GitHub authorization or requests publishing credentials.
fn authorize() -> Outcome {
    if flag("DRY_RUN")? {
        return Ok(());
    }
    is_protected_release()?;
    is_approved_environment()
}

/// The preceding verify-payload step checked these exact immutable artifacts.
fn publish() -> Outcome {
    upload_release(&[
        "rust-release/payload.tar.gz",
        "rust-release/provenance.json",
        "rust-release/SHA256SUMS",
    ])
}
