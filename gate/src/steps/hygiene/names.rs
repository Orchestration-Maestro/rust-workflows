//! HYG-006: every file is named the way its kind is named across the
//! organization, so a path reads the same in every repository. Files under a
//! `fixtures`, `testdata` or `snapshots` directory stand in for somebody
//! else's and are left alone. Whether a file is executable is what git
//! records, so a run on Windows agrees with one on Linux.

use crate::checks::findings::Finding;
use std::collections::BTreeSet;

/// The directories whose files are named by what they stand in for.
const STAND_INS: &[&str] = &["fixtures", "testdata", "snapshots"];

/// A kind of file, as a finding names it, and the form its name takes.
struct Kind {
    /// What the finding calls the file.
    name: &'static str,
    /// The form its name takes, as the finding asks for it after "name it".
    form: &'static str,
    /// Whether a name has that form.
    fits: fn(&str, &str) -> bool,
}

/// A Markdown page: kebab-case, or `UPPER_SNAKE` for a community file.
const MARKDOWN: Kind = Kind {
    name: "Markdown page",
    form: "in lowercase kebab-case, or UPPER_SNAKE for a community file such as README",
    fits: |stem, _| kebab(stem) || upper_snake(stem),
};

/// An architecture decision record, under a `docs/adr/` directory.
const DECISION: Kind = Kind {
    name: "decision record",
    form: "`NNNN-title.md`, the title in kebab-case, or README.md",
    fits: |stem, name| name == "README.md" || numbered(stem),
};

/// A Rust source file.
const RUST: Kind = Kind {
    name: "Rust file",
    form: "in snake_case",
    fits: |stem, _| snake(stem),
};

/// A GitHub workflow.
const WORKFLOW: Kind = Kind {
    name: "workflow",
    form: "in kebab-case with the extension `.yml`",
    fits: |stem, name| kebab(stem) && name.rsplit_once('.').is_some_and(|(_, end)| end == "yml"),
};

/// A script: a `.sh` file or any executable.
const SCRIPT: Kind = Kind {
    name: "script",
    form: "in lowercase kebab-case",
    fits: |stem, _| kebab(stem),
};

/// A Python module another file imports.
const MODULE: Kind = Kind {
    name: "Python module",
    form: "in snake_case, the form Python imports",
    fits: |stem, _| snake(stem) || dunder(stem),
};

/// HYG-006 over `files`, `executables` the ones git records as executable.
pub(super) fn findings(files: &[String], executables: &BTreeSet<String>) -> Vec<Finding> {
    let mut found = Vec::new();
    for file in files {
        let directories: Vec<&str> = file.split('/').collect();
        let Some((name, parents)) = directories.split_last() else {
            continue;
        };
        if parents.iter().any(|parent| STAND_INS.contains(parent)) {
            continue;
        }
        let Some(kind) = kind_of(file, name, executables.contains(file)) else {
            continue;
        };
        let stem = name.rsplit_once('.').map_or(*name, |(stem, _)| stem);
        if !(kind.fits)(stem, name) {
            let message = format!("a {} named `{name}`; name it {}", kind.name, kind.form);
            found.push(Finding::new("HYG-006", file.clone(), 0, message));
        }
    }
    found
}

/// The kind of the file at `file`, called `name`, when its name has a form.
fn kind_of(file: &str, name: &str, executable: bool) -> Option<Kind> {
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    if file.starts_with(".github/workflows/") {
        return Some(WORKFLOW);
    }
    match extension {
        Some("md") if lies_under(file, "docs/adr") => Some(DECISION),
        Some("md") => Some(MARKDOWN),
        Some("rs") => Some(RUST),
        Some("sh") => Some(SCRIPT),
        _ if executable => Some(SCRIPT),
        Some("py") => Some(MODULE),
        _ => None,
    }
}

/// Whether `file` lies somewhere under a directory whose path ends with
/// `directory`, matched segment by segment: `sub/docs/adr/a.md` lies under
/// `docs/adr`, `docs/adrs/a.md` does not.
pub(super) fn lies_under(file: &str, directory: &str) -> bool {
    format!("/{file}").contains(&format!("/{directory}/"))
}

/// Whether `text` is words joined by `separator`, each made of the bytes
/// `allowed` accepts.
fn joined(text: &str, separator: char, allowed: fn(u8) -> bool) -> bool {
    text.split(separator)
        .all(|word| !word.is_empty() && word.bytes().all(allowed))
}

/// Lowercase kebab-case: `org-page`, `2026-09-24-plan`.
fn kebab(text: &str) -> bool {
    joined(text, '-', |byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit()
    })
}

/// Lowercase `snake_case`: `org_quality`, `mod`.
fn snake(text: &str) -> bool {
    joined(text, '_', |byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit()
    })
}

/// `UPPER_SNAKE`: `README`, `CODE_OF_CONDUCT`.
fn upper_snake(text: &str) -> bool {
    joined(text, '_', |byte| {
        byte.is_ascii_uppercase() || byte.is_ascii_digit()
    })
}

/// A decision record's stem: four digits, then a kebab-case title.
fn numbered(text: &str) -> bool {
    text.split_once('-').is_some_and(|(number, title)| {
        number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit()) && kebab(title)
    })
}

/// A module Python itself names: `__init__`, `__main__`.
fn dunder(text: &str) -> bool {
    text.strip_prefix("__")
        .and_then(|rest| rest.strip_suffix("__"))
        .is_some_and(snake)
}

#[cfg(test)]
mod tests {
    use super::{dunder, kebab, kind_of, lies_under, numbered, snake, upper_snake};

    #[test]
    fn a_directory_is_matched_segment_by_segment() {
        assert!(
            lies_under("docs/adr/a.md", "docs/adr") && lies_under("x/docs/adr/a.md", "docs/adr")
        );
        assert!(
            !lies_under("docs/adrs/a.md", "docs/adr") && !lies_under("mydocs/adr/a.md", "docs/adr")
        );
    }

    #[test]
    fn each_case_accepts_its_own_form_only() {
        assert!(kebab("org-page") && kebab("2026-09-24-plan") && !kebab("Org-page"));
        assert!(!kebab("org_page") && !kebab("org--page") && !kebab(""));
        assert!(snake("org_quality") && !snake("org-quality") && !snake("OrgQuality"));
        assert!(upper_snake("CODE_OF_CONDUCT") && !upper_snake("Readme"));
        assert!(numbered("0010-spec-kit") && !numbered("10-spec-kit") && !numbered("0010"));
        assert!(dunder("__init__") && !dunder("_init"));
    }

    #[test]
    fn a_file_takes_the_kind_its_place_and_extension_give() {
        let kind = |file: &str, executable| {
            let name = file.rsplit('/').next().unwrap();
            kind_of(file, name, executable).map(|kind| kind.name)
        };
        assert_eq!(kind(".github/workflows/ci.yaml", false), Some("workflow"));
        assert_eq!(kind("docs/adr/0001-a.md", false), Some("decision record"));
        assert_eq!(
            kind("sub/docs/adr/0001-a.md", false),
            Some("decision record")
        );
        assert_eq!(kind("docs/a.md", false), Some("Markdown page"));
        assert_eq!(kind("src/a.rs", false), Some("Rust file"));
        assert_eq!(kind("scripts/org-page.py", true), Some("script"));
        assert_eq!(kind("scripts/org_quality.py", false), Some("Python module"));
        assert_eq!(kind("bootstrap", true), Some("script"));
        assert_eq!(kind("Cargo.toml", false), None);
    }
}
