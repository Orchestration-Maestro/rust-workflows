//! The home of the reusable workflows: the repository whose
//! `.github/workflows/ci.yml` declares `workflow_call`, this repository itself.
//! Its `ci.yml`, its Dependabot settings and its hooks are its own, and its
//! hooks run in `just check` on its pinned toolbelt. Its name on GitHub is
//! one constant, [`HOME`], which the rename from `rust-workflows` flipped for
//! every call, pin and hook URL the gate writes at once. What the gate reads
//! accepts either name in [`NAMES`], and what a run can tell it, such as the
//! signer of an attestation, it takes from the run.

use std::fs;
use std::path::Path;

/// The organization on GitHub.
pub(crate) const ORGANIZATION: &str = "Orchestration-Maestro";

/// The repository that holds every reusable workflow and action of the
/// organization, as the gate names it: `maestro-rust-workflows` since its
/// rename. GitHub Actions follows no rename, so every call must carry the new
/// name.
pub(crate) const HOME: &str = "maestro-rust-workflows";

/// Every name the home repository answers to: its name before the rename and
/// after. A pinned call under either moves to [`HOME`], and Dependabot leaves
/// both alone.
pub(crate) const NAMES: [&str; 2] = ["rust-workflows", "maestro-rust-workflows"];

/// Whether the repository at `root` is the home of the reusable workflows.
pub(crate) fn is_workflow_home(root: &Path) -> bool {
    fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .is_ok_and(|workflow| workflow.contains("\n  workflow_call:"))
}

#[cfg(test)]
mod tests {
    use super::{HOME, NAMES, is_workflow_home};
    use std::env;
    use std::fs;
    use std::process;

    #[test]
    fn the_home_is_one_of_the_names_it_answers_to() {
        assert!(NAMES.contains(&HOME));
        assert_eq!(HOME, "maestro-rust-workflows");
    }

    #[test]
    fn only_a_callable_ci_workflow_marks_the_home() {
        let root = env::temp_dir().join(format!("workflow-home-{}", process::id()));
        let workflows = root.join(".github/workflows");
        fs::create_dir_all(&workflows).unwrap();
        assert!(!is_workflow_home(&root));
        fs::write(workflows.join("ci.yml"), "name: CI\n\"on\":\n  push:\n").unwrap();
        assert!(!is_workflow_home(&root));
        fs::write(
            workflows.join("ci.yml"),
            "name: CI\n\"on\":\n  workflow_call:\n",
        )
        .unwrap();
        assert!(is_workflow_home(&root));
        fs::remove_dir_all(&root).unwrap();
    }
}
