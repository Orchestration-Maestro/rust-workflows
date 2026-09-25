//! The steps: `rules` writes each page of the rule map that differs from the
//! rendering, and `rules --check` refuses a stale page or a row not mapped
//! yet, which the daily drift check reports.

use super::render::{UNMAPPED, pages};
use crate::runner::{Cmd, Failure, Outcome, Step, write};
use std::env;
use std::fs;
use std::path::Path;

/// What these steps declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "local",
        id: "rules",
        summary: "The rule map written from the golden rules this release carries",
        inputs: &[],
        tools: &["git"],
        reports: &[],
        run,
    },
    Step {
        workflow: "local",
        id: "rules --check",
        summary: "The rule map compared with the golden rules this release carries",
        inputs: &[],
        tools: &["git"],
        reports: &[],
        run: check,
    },
];

/// The three pages rendered for the repository here.
type Rendered = [(&'static str, String); 3];

/// Run `rules`: write every page whose text differs from the rendering.
fn run() -> Outcome {
    let rendered = rendered()?;
    for (path, text) in &rendered {
        if fs::read_to_string(path).ok().as_ref() == Some(text) {
            continue;
        }
        let file = Path::new(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("rules: {path}: {error}"))?;
        }
        write(file, text.as_bytes(), false)?;
    }
    println!("rules: {} rows not mapped yet", unmapped(&rendered));
    Ok(())
}

/// Run `rules --check`: refuse a stale page or a row not mapped yet.
fn check() -> Outcome {
    let rendered = rendered()?;
    let stale: Vec<&str> = rendered
        .iter()
        .filter(|(path, text)| fs::read_to_string(path).ok().as_ref() != Some(text))
        .map(|(path, _)| *path)
        .collect();
    refusal(&stale, unmapped(&rendered)).map_or(Ok(()), |message| Err(Failure::from(message)))
}

/// The refusal for `stale` pages and `unmapped` rows, with the fix; None
/// when the rule map is current and complete.
fn refusal(stale: &[&str], unmapped: usize) -> Option<String> {
    if stale.is_empty() && unmapped == 0 {
        return None;
    }
    let stale = if stale.is_empty() {
        "none".to_owned()
    } else {
        stale.join(", ")
    };
    Some(format!(
        "rules --check: stale {stale}; {unmapped} rows not mapped yet; run rust-gate rules, \
         then map every row"
    ))
}

/// The three pages of the repository here, rendered.
fn rendered() -> Result<Rendered, Failure> {
    let name = repository_name()?;
    pages(&name, |path| fs::read_to_string(path).unwrap_or_default()).map_err(Failure::from)
}

/// The repository's name: the last part of its `origin` remote, else the
/// name of the directory it runs in.
fn repository_name() -> Result<String, Failure> {
    let remote = Cmd::new("git config --get remote.origin.url")
        .capture()
        .unwrap_or_default();
    let name = name_of(&remote);
    if !name.is_empty() {
        return Ok(name.to_owned());
    }
    let here = env::current_dir().map_err(|error| Failure::from(format!("rules: {error}")))?;
    Ok(here
        .file_name()
        .map(|directory| directory.to_string_lossy().into_owned())
        .unwrap_or_default())
}

/// The last part of a remote URL, without a trailing `/` or `.git`.
fn name_of(remote: &str) -> &str {
    let remote = remote.trim().trim_end_matches('/');
    let remote = remote.strip_suffix(".git").unwrap_or(remote);
    remote.rsplit('/').next().unwrap_or_default()
}

/// The rows and sections still not mapped, counted below each page's intro,
/// which names the marker itself.
fn unmapped(rendered: &Rendered) -> usize {
    rendered
        .iter()
        .map(|(_, text)| {
            text.split_once("\n## ")
                .map_or(0, |(_, body)| body.matches(UNMAPPED).count())
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{name_of, refusal, unmapped};

    #[test]
    fn the_repository_name_is_the_last_part_of_its_remote() {
        assert_eq!(
            name_of("https://github.com/Orchestration-Maestro/maestro-core.git\n"),
            "maestro-core"
        );
        assert_eq!(
            name_of("git@github.com:Orchestration-Maestro/release-canary.git"),
            "release-canary"
        );
        assert_eq!(
            name_of("https://github.com/Orchestration-Maestro/.github/"),
            ".github"
        );
        assert_eq!(name_of(""), "");
    }

    #[test]
    fn a_stale_page_or_an_unmapped_row_is_refused_with_the_fix() {
        assert_eq!(refusal(&[], 0), None);
        assert_eq!(
            refusal(&["docs/standards/security.md"], 0).unwrap(),
            "rules --check: stale docs/standards/security.md; 0 rows not mapped yet; run \
             rust-gate rules, then map every row"
        );
        assert_eq!(
            refusal(&[], 3).unwrap(),
            "rules --check: stale none; 3 rows not mapped yet; run rust-gate rules, then map \
             every row"
        );
    }

    #[test]
    fn only_the_rows_below_the_intro_count_as_unmapped() {
        let page = |text: &str| ("docs/standards/page.md", text.to_owned());
        let rendered = [
            page("Intro says \"Not mapped yet\".\n## Rule map\n| A | Not mapped yet |\n"),
            page("No heading, Not mapped yet.\n"),
            page("\n## KPIs\n| Speed | Not mapped yet | not measured |\n"),
        ];
        assert_eq!(unmapped(&rendered), 2);
    }
}
