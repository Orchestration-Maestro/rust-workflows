//! The home of the reusable workflows: the repository whose
//! `.github/workflows/ci.yml` declares `workflow_call`, rust-workflows itself.
//! Its `ci.yml`, its Dependabot settings and its hooks are its own, and its
//! hooks run in `just check` on its pinned toolbelt.

use std::fs;
use std::path::Path;

/// Whether the repository at `root` is the home of the reusable workflows.
pub(crate) fn is_workflow_home(root: &Path) -> bool {
    fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .is_ok_and(|workflow| workflow.contains("\n  workflow_call:"))
}

#[cfg(test)]
mod tests {
    use super::is_workflow_home;
    use std::env;
    use std::fs;
    use std::process;

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
