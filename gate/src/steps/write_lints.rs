//! `rust-gate lints --write`: the organization's lints, LNT-001, written into
//! the manifest in the current directory between their markers,
//! `[workspace.lints]` for a workspace root and `[lints]` for a single
//! package. Run where the root `Cargo.toml` is, and again whenever the gate's
//! list changes; the lints a repository adds below the block stay.

use crate::checks::lint_policy::with_lint_block;
use crate::runner::{Cmd, Outcome, Step, write};
use std::fs;
use std::path::Path;

/// What this command declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "local",
    id: "lints --write",
    summary: "The organization's lints written into the root manifest",
    inputs: &[],
    tools: &["jaq"],
    reports: &[],
    run,
}];

/// Whether a manifest is a workspace root: it holds a `[workspace]` table.
const IS_WORKSPACE: &str = "has(\"workspace\")";

/// Run the command.
fn run() -> Outcome {
    let manifest = Path::new("Cargo.toml");
    let text = fs::read_to_string(manifest).map_err(|error| {
        format!("lints --write: Cargo.toml: {error}; run it where the root manifest is")
    })?;
    let workspace = Cmd::new("jaq --from toml")
        .arg(IS_WORKSPACE)
        .arg(manifest)
        .capture()?;
    let written = with_lint_block(&text, workspace.trim() == "true")
        .map_err(|error| format!("lints --write: Cargo.toml: {error}"))?;
    write(manifest, written.as_bytes(), false)
}
