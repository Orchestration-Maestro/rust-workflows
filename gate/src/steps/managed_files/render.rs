//! Every managed file of a repository, rendered: only what a tool or GitHub
//! reads from the repository itself. The gate passes every other tool its
//! configuration at run time. The files every repository holds as they are
//! here are this repository's own, read in when the gate is built, so the
//! home of the gate is the one place to change them; the rest are written
//! from the gate's data and the repository's `maestro-quality.toml`.
//!
//! The home is not compared with its own sources: `.editorconfig`,
//! `.gitattributes` and `rust-toolchain.toml`, which the gate reads in from it:
//! compared there, a source would only be compared with itself. What the gate
//! writes from its
//! data, `typos.toml` and the manifest's lint block, is compared in the home
//! as everywhere else.

use super::hooks::commit_hooks;
use crate::checks::lint_policy::with_lint_block;
use crate::checks::organization_config::HEADER;
use crate::checks::quality_config::QualityConfig;
use crate::checks::workflow_home::{NAMES, ORGANIZATION};
use std::fmt::Write as _;

/// The files `rust-gate sync` wrote before and writes no more: the caller of
/// `ci.yml` or `hygiene.yml`, which the organization's rulesets run in its
/// place, uploads included, and the settings the gate now passes their tools at
/// run time. Sync deletes each one that still opens with the header, and the
/// check refuses it until then.
pub(super) const RETIRED: [&str; 8] = [
    ".config/nextest.toml",
    ".github/workflows/ci.yml",
    ".rumdl.toml",
    ".taplo.toml",
    ".yamlfmt.yml",
    "clippy.toml",
    "deny.toml",
    "rustfmt.toml",
];

/// Editor settings that survive the editor.
const EDITORCONFIG: &str = include_str!("../../../../.editorconfig");

/// How git treats each kind of file.
const GITATTRIBUTES: &str = include_str!("../../../../.gitattributes");

/// The organization's compiler.
const TOOLCHAIN: &str = include_str!("../../../../rust-toolchain.toml");

/// The files every repository holds as they are in the home, read in when
/// the gate is built; the home holds their source and is not compared.
const SOURCES: &[(&str, &str)] = &[
    (".editorconfig", EDITORCONFIG),
    (".gitattributes", GITATTRIBUTES),
];

/// Dependabot's settings for one ecosystem, after its `package-ecosystem`.
const WEEKLY: &str = concat!(
    "    directory: /\n",
    "    schedule:\n",
    "      interval: weekly\n",
    "    cooldown:\n",
    "      default-days: 7\n",
);

/// What the gate knows of a repository to render its files.
pub(super) struct Repository {
    /// Whether it builds Rust: a root `Cargo.toml` or `rust-toolchain.toml`.
    pub(super) rust: bool,
    /// Whether it is the home of the reusable workflows, whose Dependabot
    /// settings and hooks are its own.
    pub(super) home: bool,
    /// The root manifest and whether it is a workspace root, when there is one.
    pub(super) manifest: Option<(String, bool)>,
    /// What its `maestro-quality.toml` says.
    pub(super) config: QualityConfig,
    /// The version of the release it declares, when one is known.
    pub(super) version: Option<String>,
}

/// Every managed file of `repository` and its content, in path order.
pub(super) fn managed_files(repository: &Repository) -> Result<Vec<(String, String)>, String> {
    let mut files: Vec<(&str, String)> = vec![("typos.toml", typos(&repository.config.typos))];
    if !repository.home {
        files.extend(
            SOURCES
                .iter()
                .map(|(path, text)| (*path, (*text).to_owned())),
        );
        let version = repository.version.as_deref().ok_or(
            "no release to pin: no workflow or commit hook names one; set RUST_WORKFLOWS_PIN to \
             `<commit> v<version>`",
        )?;
        files.push((".github/dependabot.yml", dependabot(repository.rust)));
        let hooks = commit_hooks(HEADER, repository.rust, version);
        files.push((".pre-commit-config.yaml", hooks));
    }
    if repository.rust && !repository.home {
        files.push(("rust-toolchain.toml", TOOLCHAIN.to_owned()));
    }
    if let Some((text, workspace)) = &repository.manifest {
        files.push(("Cargo.toml", with_lint_block(text, *workspace)?));
    }
    let mut files: Vec<(String, String)> = files
        .into_iter()
        .map(|(path, text)| (path.to_owned(), text))
        .collect();
    files.sort();
    Ok(files)
}

/// The words every repository means: `FND`, the prefix of the foundations its
/// rule map cites.
const ORGANIZATION_WORDS: [&str; 1] = ["FND"];

/// `typos.toml`: the words the repository means, each allowed as written: the
/// organization's first, then its own, each once. The header stays as it was,
/// so a repository that already listed `FND` renders the same bytes under the
/// gate its CI pins and under this one.
fn typos(words: &[String]) -> String {
    let mut text = format!(
        "{HEADER}# The words this repository means, from [typos] words in maestro-quality.toml.\n\n\
         [default.extend-words]\n"
    );
    let own = words
        .iter()
        .map(String::as_str)
        .filter(|word| !ORGANIZATION_WORDS.contains(word));
    for word in ORGANIZATION_WORDS.into_iter().chain(own) {
        let _ = writeln!(text, "{word} = \"{word}\"");
    }
    text
}

/// Dependabot: the actions, and Cargo for a Rust repository, weekly with a
/// week's cooldown and conventional titles; the reusable workflows are left
/// to `rust-gate sync`, under every name their repository answers to.
fn dependabot(rust: bool) -> String {
    let mut text = format!(
        "{HEADER}version: 2\nupdates:\n  - package-ecosystem: github-actions\n{WEEKLY}    \
         # The organization merges only conventional titles: \"ci(deps): bump ...\".\n    \
         commit-message:\n      prefix: ci\n      include: scope\n    \
         # rust-gate sync moves the organization's reusable workflows.\n    ignore:\n"
    );
    for name in NAMES {
        let _ = writeln!(text, "      - dependency-name: {ORGANIZATION}/{name}*");
    }
    text.push_str(
        "    groups:\n      actions:\n        patterns: [\"*\"]\n        update-types: \
         [minor, patch]\n",
    );
    if rust {
        let _ = write!(
            text,
            "  - package-ecosystem: cargo\n{WEEKLY}    # \"build(deps): bump ...\", a \
             conventional title the organization accepts.\n    commit-message:\n      prefix: \
             build\n      include: scope\n    groups:\n      cargo:\n        patterns: \
             [\"*\"]\n        update-types: [minor, patch]\n"
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{Repository, managed_files};
    use crate::checks::organization_config::HEADER;
    use crate::checks::quality_config::QualityConfig;

    /// A Rust repository pinned at `2.0.0`, what `config` says.
    fn repository(config: QualityConfig) -> Repository {
        Repository {
            rust: true,
            home: false,
            manifest: Some(("[package]\nname = \"a\"\n".to_owned(), false)),
            config,
            version: Some("2.0.0".to_owned()),
        }
    }

    #[test]
    fn a_rust_repository_holds_every_managed_file_under_the_header() {
        let files = managed_files(&repository(QualityConfig::default())).unwrap();
        let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                ".editorconfig",
                ".gitattributes",
                ".github/dependabot.yml",
                ".pre-commit-config.yaml",
                "Cargo.toml",
                "rust-toolchain.toml",
                "typos.toml",
            ]
        );
        for (path, text) in files.iter().filter(|(path, _)| path != "Cargo.toml") {
            assert!(text.starts_with(HEADER), "{path} opens without the header");
        }
        let dependabot = &files[2].1;
        assert!(dependabot.contains(
            "    ignore:\n      - dependency-name: Orchestration-Maestro/rust-workflows*\n      \
             - dependency-name: Orchestration-Maestro/maestro-rust-workflows*\n    groups:\n"
        ));
    }

    #[test]
    fn the_home_is_not_compared_with_the_files_it_is_the_source_of() {
        let mut home = repository(QualityConfig::default());
        home.home = true;
        home.version = None;
        let files = managed_files(&home).unwrap();
        let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["Cargo.toml", "typos.toml"]);
    }

    #[test]
    fn every_repository_means_the_foundations_prefix_once() {
        let config = QualityConfig {
            typos: vec!["FND".to_owned(), "jaq".to_owned()],
            ..QualityConfig::default()
        };
        let files = managed_files(&repository(config)).unwrap();
        let typos = files
            .iter()
            .find(|(path, _)| path == "typos.toml")
            .map(|(_, text)| text.as_str())
            .unwrap();
        assert_eq!(typos.matches("FND = \"FND\"").count(), 1);
        assert!(typos.ends_with("[default.extend-words]\nFND = \"FND\"\njaq = \"jaq\"\n"));
    }

    #[test]
    fn the_hooks_install_the_declared_release_and_no_caller_is_rendered() {
        let files = managed_files(&repository(QualityConfig::default())).unwrap();
        assert!(
            files
                .iter()
                .all(|(path, _)| !path.starts_with(".github/workflows/"))
        );
        let hooks = files
            .iter()
            .find(|(path, _)| path == ".pre-commit-config.yaml")
            .map(|(_, text)| text.as_str())
            .unwrap();
        assert!(hooks.contains("/rust-workflows:v2.0.0:rust-gate\"\n"));
        let mut unpinned = repository(QualityConfig::default());
        unpinned.version = None;
        assert!(
            managed_files(&unpinned)
                .unwrap_err()
                .starts_with("no release to pin")
        );
    }
}
