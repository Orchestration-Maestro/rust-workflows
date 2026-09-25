//! The steps: `guide` writes the Copilot guide where it differs from the
//! rendering, and `guide --check` refuses a stale one, which the daily drift
//! check reports.

use super::render::{GUIDE, render};
use crate::runner::{Cmd, Failure, Outcome, Step, write};
use std::fs;
use std::path::Path;

/// What these steps declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "local",
        id: "guide",
        summary: "The Copilot guide written from the repository's tracked files",
        inputs: &[],
        tools: &["git", "jaq"],
        reports: &[],
        run,
    },
    Step {
        workflow: "local",
        id: "guide --check",
        summary: "The Copilot guide compared with the repository's tracked files",
        inputs: &[],
        tools: &["git", "jaq"],
        reports: &[],
        run: check,
    },
];

/// What the guide leaves out: Windows' download marks and generated SBOMs.
const SKIPPED: [&str; 2] = [":Zone.Identifier", ".cdx.json"];

/// Run `guide`: write the guide when it differs from the rendering.
fn run() -> Outcome {
    let text = rendered()?;
    let guide = Path::new(GUIDE);
    if fs::read_to_string(guide).ok().as_ref() == Some(&text) {
        return Ok(());
    }
    if let Some(parent) = guide.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("guide: {GUIDE}: {error}"))?;
    }
    write(guide, text.as_bytes(), false)?;
    println!("guide: wrote {GUIDE}");
    Ok(())
}

/// Run `guide --check`: refuse a guide that differs from the rendering.
fn check() -> Outcome {
    let text = rendered()?;
    if fs::read_to_string(GUIDE).ok().as_ref() == Some(&text) {
        return Ok(());
    }
    Err(Failure::from(format!(
        "guide --check: {GUIDE} is stale; run rust-gate guide and commit it"
    )))
}

/// The guide of the repository here, from its tracked files and remote.
fn rendered() -> Result<String, Failure> {
    let listed = Cmd::new("git ls-files -z").capture()?;
    let paths = tracked(&listed);
    let remote = Cmd::new("git config --get remote.origin.url")
        .capture()
        .unwrap_or_default();
    Ok(render(Path::new("."), &paths, &remote))
}

/// The tracked files `git ls-files -z` listed, and the guide, sorted, each
/// once, without the ones the guide leaves out.
fn tracked(listed: &str) -> Vec<String> {
    let mut paths: Vec<String> = listed
        .split('\0')
        .filter(|path| !path.is_empty() && !SKIPPED.iter().any(|end| path.ends_with(end)))
        .map(str::to_owned)
        .chain([GUIDE.to_owned()])
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use super::tracked;

    #[test]
    fn tracked_files_are_sorted_with_the_guide_and_without_skipped_ones() {
        let listed = "src/b.rs\0a.cdx.json\0README.md\0logo.png:Zone.Identifier\0";
        assert_eq!(
            tracked(listed),
            [".github/copilot-instructions.md", "README.md", "src/b.rs"]
        );
        assert_eq!(
            tracked(".github/copilot-instructions.md\0"),
            [".github/copilot-instructions.md"]
        );
    }
}
