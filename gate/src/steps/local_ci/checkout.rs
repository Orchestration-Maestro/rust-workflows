//! The commit a local run checks, as CI would check it out. A branch runs as
//! its pull request: a merge whose tree is the branch's last commit and whose
//! first parent is where the branch left the default branch, so every step
//! that reads `HEAD^1`, the settings, the changed lines and the mutants'
//! diff, sees the branch's whole change. The default branch runs as a push.
//! What is committed runs, never what is not, as a push sends it; the checkout
//! is a clone of its own, so no step touches the repository it came from.

use super::environment::Environment;
use crate::runner::{Cmd, Failure};
use std::fs;
use std::path::Path;

/// The commit checked out and what GitHub would say about its run.
pub(super) struct Checkout {
    /// The commit, `GITHUB_SHA`.
    pub(super) revision: String,
    /// The default branch a pull request targets, `GITHUB_BASE_REF`; empty
    /// for a push.
    pub(super) base: String,
    /// The branch, `GITHUB_HEAD_REF`; empty for a push or a detached HEAD.
    pub(super) branch: String,
    /// The pull request's title: `PULL_REQUEST_TITLE` when set, else the
    /// subject of the branch's first commit, what GitHub proposes for it.
    pub(super) title: String,
}

impl Checkout {
    /// How the run reads in the summary.
    pub(super) fn describe(&self) -> String {
        let short: String = self.revision.chars().take(12).collect();
        if self.base.is_empty() {
            format!("{short}, a push")
        } else {
            format!(
                "{short}, a pull request from {} into {}",
                display_branch(&self.branch),
                self.base
            )
        }
    }
}

/// `git -C directory` in `environment`, which carries no `GIT_*` variable.
fn git(environment: &Environment, directory: &Path) -> Result<Cmd, Failure> {
    Ok(environment.command("git -C")?.arg(directory))
}

/// The repository the current directory is in: its top level.
pub(super) fn repository_root(environment: &Environment) -> Result<String, Failure> {
    let top = git(environment, Path::new("."))?
        .args(["rev-parse", "--show-toplevel"])
        .capture()?;
    Ok(top.trim().to_owned())
}

/// Check the committed state of `root` out into a fresh clone in
/// `directory`, as a runner's checkout is: every file written anew, so a
/// build the last run kept, a mutant's included, is never taken for this
/// run's sources.
pub(super) fn check_out(
    environment: &Environment,
    root: &Path,
    directory: &Path,
) -> Result<Checkout, Failure> {
    let base = default_branch(environment, root)?;
    let branch = git(environment, root)?
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .capture()
        .map(|name| name.trim().to_owned())
        .unwrap_or_default();
    if directory.exists() {
        fs::remove_dir_all(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
    }
    git(environment, Path::new("."))?
        .args(["init", "--quiet"])
        .arg(directory)
        .run()?;
    git(environment, directory)?
        .args(["fetch", "--quiet", "--no-tags", "--force"])
        .arg(root)
        .arg("+HEAD:refs/local-ci/head")
        .arg(format!("+refs/remotes/origin/{base}:refs/local-ci/base"))
        .run()?;
    let read = |arguments: &[&str]| -> Result<String, Failure> {
        let text = git(environment, directory)?.args(arguments).capture()?;
        Ok(text.trim().to_owned())
    };
    let head = read(&["rev-parse", "refs/local-ci/head"])?;
    let fork = read(&["merge-base", "refs/local-ci/head", "refs/local-ci/base"])?;
    let checkout = if fork == head {
        Checkout {
            revision: head,
            base: String::new(),
            branch: String::new(),
            title: String::new(),
        }
    } else {
        let subjects = read(&[
            "log",
            "--reverse",
            "--format=%s",
            &format!("{fork}..{head}"),
        ])?;
        let title = match environment.get("PULL_REQUEST_TITLE") {
            "" => subjects.lines().next().unwrap_or_default().to_owned(),
            set => set.to_owned(),
        };
        let message = format!("Merge {} into {base}", display_branch(&branch));
        let merge = git(environment, directory)?
            .args([
                "commit-tree",
                &format!("{head}^{{tree}}"),
                "-p",
                &fork,
                "-p",
            ])
            .arg(&head)
            .arg("-m")
            .arg(message)
            .env("GIT_AUTHOR_NAME", "rust-gate")
            .env("GIT_AUTHOR_EMAIL", "rust-gate@localhost")
            .env("GIT_AUTHOR_DATE", "@0 +0000")
            .env("GIT_COMMITTER_NAME", "rust-gate")
            .env("GIT_COMMITTER_EMAIL", "rust-gate@localhost")
            .env("GIT_COMMITTER_DATE", "@0 +0000")
            .capture()?;
        Checkout {
            revision: merge.trim().to_owned(),
            base,
            branch,
            title,
        }
    };
    git(environment, directory)?
        .args(["checkout", "--quiet", "--force", "--detach"])
        .arg(&checkout.revision)
        .run()?;
    git(environment, directory)?
        .args(["clean", "-ffdxq"])
        .run()?;
    Ok(checkout)
}

/// The default branch of `root`'s `origin`: what `origin/HEAD` names, else
/// `main` when `origin/main` exists.
fn default_branch(environment: &Environment, root: &Path) -> Result<String, Failure> {
    let named = git(environment, root)?
        .args(["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"])
        .capture()
        .unwrap_or_default();
    if let Some(branch) = branch_of(&named) {
        return Ok(branch);
    }
    let main = git(environment, root)?
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            "refs/remotes/origin/main",
        ])
        .capture();
    if main.is_ok() {
        return Ok("main".to_owned());
    }
    Err(Failure::from(
        "ci --local: origin names no default branch; name it with git remote set-head origin \
         --auto",
    ))
}

/// The branch a remote-tracking `origin` ref names.
fn branch_of(reference: &str) -> Option<String> {
    reference
        .trim()
        .strip_prefix("refs/remotes/origin/")
        .filter(|branch| !branch.is_empty())
        .map(str::to_owned)
}

/// A branch as the summary names it, a detached HEAD included.
fn display_branch(branch: &str) -> &str {
    if branch.is_empty() {
        "a detached HEAD"
    } else {
        branch
    }
}

#[cfg(test)]
mod tests {
    use super::{Checkout, branch_of};

    #[test]
    fn the_default_branch_is_what_origin_s_head_names() {
        assert_eq!(
            branch_of("refs/remotes/origin/main\n").as_deref(),
            Some("main")
        );
        assert_eq!(
            branch_of("refs/remotes/origin/release/2").as_deref(),
            Some("release/2")
        );
        assert!(branch_of("").is_none() && branch_of("refs/remotes/origin/").is_none());
    }

    #[test]
    fn a_run_says_which_commit_it_checks_and_how() {
        let mut checkout = Checkout {
            revision: "0123456789abcdef".to_owned(),
            base: String::new(),
            branch: String::new(),
            title: String::new(),
        };
        assert_eq!(checkout.describe(), "0123456789ab, a push");
        checkout.base = "main".to_owned();
        assert_eq!(
            checkout.describe(),
            "0123456789ab, a pull request from a detached HEAD into main"
        );
        checkout.branch = "feat/x".to_owned();
        assert_eq!(
            checkout.describe(),
            "0123456789ab, a pull request from feat/x into main"
        );
    }
}
