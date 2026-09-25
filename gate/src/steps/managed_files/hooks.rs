//! The commit hooks every repository runs, rendered into its
//! `.pre-commit-config.yaml`: prek's own checks, each tool through prek's
//! `mise` language at the version this repository's `mise.toml` pins, the
//! commit message rules, and for Rust the formatter, the gate's source rules at
//! the pinned release and Clippy before a push. Each tool takes the
//! organization's configuration on its command line, from the hook, rather
//! than from a file in the repository; Clippy takes it through
//! `rust-gate clippy --local`, since no option carries it. At the same release, the gate
//! keeps the rule map and the Copilot guide current: a hook rewrites either when
//! it is stale, so the next commit carries it. A developer needs prek and
//! rustup, nothing else.

use crate::checks::organization_config::RUSTFMT_OPTIONS;
use std::fmt::Write as _;

/// Every tool a hook runs and the `mise` spec that installs it.
const TOOLS: &[(&str, &str)] = &[
    ("actionlint", "aqua:rhysd/actionlint@1.7.12"),
    (
        "editorconfig-checker",
        "github:editorconfig-checker/editorconfig-checker@4.0.2",
    ),
    ("gitleaks", "aqua:gitleaks/gitleaks@8.30.1"),
    ("lychee", "aqua:lycheeverse/lychee@0.24.2"),
    ("rumdl", "aqua:rvben/rumdl@0.2.77"),
    ("shellcheck", "aqua:koalaman/shellcheck@0.11.0"),
    ("shfmt", "aqua:mvdan/sh@3.14.1"),
    ("taplo", "aqua:tamasfe/taplo@0.10.0"),
    ("typos", "aqua:crate-ci/typos@1.50.2"),
    ("yamlfmt", "aqua:google/yamlfmt@0.21.0"),
    ("zizmor", "aqua:zizmorcore/zizmor@1.30.1"),
];

/// The jaq the gate reads TOML and JSON through, built by Cargo.
const JAQ: &str = "cli:jaq:3.1.1";

/// prek's own checks, with their arguments.
const BUILTIN: &str = concat!(
    "  - repo: builtin\n",
    "    hooks:\n",
    "      - id: check-merge-conflict\n",
    "      - id: check-yaml\n",
    "      - id: check-toml\n",
    "      - id: check-json\n",
    "      - id: end-of-file-fixer\n",
    "      - id: trailing-whitespace\n",
    "        args: [--markdown-linebreak-ext=md]\n",
    "      - id: mixed-line-ending\n",
    "        args: [--fix=no]\n",
    "      - id: check-added-large-files\n",
    "        args: [--maxkb=500]\n",
    "      - id: check-case-conflict\n",
    "      - id: check-executables-have-shebangs\n",
    "      - id: check-shebang-scripts-are-executable\n",
);

/// A hook whose tools prek installs through mise: its id, its name, the
/// tools it needs, its command with the organization's configuration, and the
/// files it reads, as a `types:` or `files:` line, or nothing for every file.
struct ToolHook {
    /// The hook's id, what `SKIP` names.
    id: &'static str,
    /// What prek prints for it.
    name: &'static str,
    /// The tools of [`TOOLS`] it needs.
    tools: &'static [&'static str],
    /// The command, the files it reads appended.
    entry: &'static str,
    /// The `types:` or `files:` line, or empty.
    files: &'static str,
    /// The `exclude:` pattern of files it leaves alone, or empty.
    exclude: &'static str,
    /// Whether it reads the repository itself rather than the files passed.
    whole: bool,
}

/// Every hook whose tools prek installs through mise.
const TOOL_HOOKS: &[ToolHook] = &[
    ToolHook {
        id: "typos",
        name: "Spelling in code and prose",
        tools: &["typos"],
        entry: "typos --force-exclude",
        files: "",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "gitleaks",
        name: "Secrets in the staged change",
        tools: &["gitleaks"],
        entry: "gitleaks git --pre-commit --staged --redact --no-banner",
        files: "",
        exclude: "",
        whole: true,
    },
    ToolHook {
        id: "yamlfmt",
        name: "YAML formatting",
        tools: &["yamlfmt"],
        entry: "yamlfmt -no_global_conf -lint -formatter indent=2,include_document_start=false,\
                retain_line_breaks_single=true,pad_line_comments=2,line_ending=lf",
        files: "types: [yaml]",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "taplo",
        name: "TOML formatting",
        tools: &["taplo"],
        entry: "taplo fmt --check --diff --no-auto-config --option array_auto_collapse=false",
        files: "types: [toml]",
        exclude: "(^|/)supply-chain/",
        whole: false,
    },
    ToolHook {
        id: "actionlint",
        name: "GitHub workflow lint",
        tools: &["actionlint", "shellcheck"],
        entry: "actionlint",
        files: "files: '^\\.github/workflows/.*\\.ya?ml$'",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "zizmor",
        name: "GitHub workflow security audit",
        tools: &["zizmor"],
        entry: "zizmor --offline --persona=pedantic --no-progress",
        files: "files: '^\\.github/(workflows/.*|dependabot|actions/.*/action)\\.ya?ml$'",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "shellcheck",
        name: "Shell lint",
        tools: &["shellcheck"],
        entry: "shellcheck",
        files: "types: [shell]",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "shfmt",
        name: "Shell formatting",
        tools: &["shfmt"],
        entry: "shfmt -d",
        files: "types: [shell]",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "rumdl",
        name: "Markdown structure",
        tools: &["rumdl"],
        entry: "rumdl check --no-cache --no-config --disable MD013,MD041 --config \
                'MD033.allowed-elements = [\"a\", \"br\", \"details\", \"h1\", \"img\", \"p\", \
                \"picture\", \"source\", \"strong\", \"summary\"]'",
        files: "types: [markdown]",
        exclude: "^CHANGELOG\\.md$",
        whole: false,
    },
    ToolHook {
        id: "lychee",
        name: "Markdown links, offline",
        tools: &["lychee"],
        entry: "lychee --offline --no-progress",
        files: "types: [markdown]",
        exclude: "",
        whole: false,
    },
    ToolHook {
        id: "editorconfig-checker",
        name: "Every file against .editorconfig",
        tools: &["editorconfig-checker"],
        entry: "editorconfig-checker -disable-indent-size",
        files: "",
        exclude: "",
        whole: false,
    },
];

/// The commit message rules: a conventional header, lowercase after the
/// type, a subject within 71 characters, and every line within 80 columns.
const MESSAGE: &str = concat!(
    "      - id: conventional-commit-header\n",
    "        name: Commit message opens with a conventional header\n",
    "        language: pygrep\n",
    "        entry: '\\A(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)",
    "(\\([a-z0-9-]+\\))?!?: [a-z][^\\n]{0,70}\\n'\n",
    "        args: [--negate, --multiline]\n",
    "        stages: [commit-msg]\n",
    "      - id: eighty-columns\n",
    "        name: Commit message lines stay within 80 columns\n",
    "        language: pygrep\n",
    "        entry: '^.{81,}$'\n",
    "        stages: [commit-msg]\n",
);

/// The formatter, run by the toolchain rustup provides with the
/// organization's options.
fn rustfmt_hook() -> String {
    format!(
        concat!(
            "      - id: rustfmt\n",
            "        name: Rust formatting\n",
            "        language: system\n",
            "        entry: cargo fmt --all --check -- --config {options}\n",
            "        types: [rust]\n",
            "        pass_filenames: false\n",
        ),
        options = RUSTFMT_OPTIONS,
    )
}

/// The `.pre-commit-config.yaml` of a repository whose caller pins release
/// `version`, with the Rust hooks when it builds Rust.
pub(super) fn commit_hooks(header: &str, rust: bool, version: &str) -> String {
    let mut text = format!(
        "{header}minimum_prek_version: '0.5.3'\ndefault_stages: [pre-commit]\n\
         default_install_hook_types: [pre-commit, commit-msg, pre-push]\nrepos:\n{BUILTIN}  \
         - repo: local\n    hooks:\n"
    );
    for hook in TOOL_HOOKS {
        let specs: Vec<String> = hook
            .tools
            .iter()
            .filter_map(|tool| TOOLS.iter().find(|(name, _)| name == tool))
            .map(|(_, spec)| format!("\"{spec}\""))
            .collect();
        let _ = write!(
            text,
            "      - id: {}\n        name: {}\n        language: mise\n        \
             additional_dependencies: [{}]\n        entry: {}\n",
            hook.id,
            hook.name,
            specs.join(", "),
            hook.entry
        );
        if !hook.files.is_empty() {
            let _ = writeln!(text, "        {}", hook.files);
        }
        if !hook.exclude.is_empty() {
            let _ = writeln!(text, "        exclude: '{}'", hook.exclude);
        }
        if hook.whole {
            text.push_str("        pass_filenames: false\n");
        }
    }
    text.push_str(MESSAGE);
    let gate = format!(
        concat!(
            "        language: rust\n",
            "        additional_dependencies:\n",
            "          - \"cli:https://github.com/Orchestration-Maestro/",
            "rust-workflows:v{version}:rust-gate\"\n",
            "          - \"{jaq}\"\n",
            "        pass_filenames: false\n",
        ),
        version = version,
        jaq = JAQ,
    );
    if rust {
        text.push_str(&rustfmt_hook());
        let _ = write!(
            text,
            "      - id: clippy\n        name: Clippy with the organization's lints\n        \
             entry: rust-gate clippy --local\n        types: [rust]\n        stages: \
             [pre-push]\n{gate}"
        );
        let _ = write!(
            text,
            "      - id: rust-gate-architecture\n        name: Source rules\n        entry: \
             rust-gate architecture --local\n        files: \
             '(\\.rs|Cargo\\.toml|Cargo\\.lock|maestro-quality\\.toml|clippy\\.toml)$'\n{gate}"
        );
    }
    let _ = write!(
        text,
        "      - id: rust-gate-hygiene\n        name: Repository hygiene\n        entry: rust-gate \
         hygiene --local\n        always_run: true\n{gate}"
    );
    let _ = write!(
        text,
        "      - id: rust-gate-rules\n        name: Rule map from the golden rules\n        \
         entry: rust-gate rules\n        always_run: true\n{gate}      - id: \
         rust-gate-guide\n        name: Copilot guide from the tracked files\n        entry: \
         rust-gate guide\n        always_run: true\n{gate}"
    );
    text
}

#[cfg(test)]
mod tests {
    use super::{TOOL_HOOKS, TOOLS, commit_hooks};

    #[test]
    fn every_hook_names_a_pinned_tool_and_the_gate_at_the_release() {
        for hook in TOOL_HOOKS {
            for tool in hook.tools {
                assert!(
                    TOOLS.iter().any(|(name, _)| name == tool),
                    "{} needs {tool}, which no spec pins",
                    hook.id
                );
            }
        }
        let rust = commit_hooks("# h\n", true, "2.0.0");
        assert!(rust.starts_with("# h\nminimum_prek_version: '0.5.3'\n"));
        assert!(rust.contains(
            "additional_dependencies: [\"aqua:rhysd/actionlint@1.7.12\", \
             \"aqua:koalaman/shellcheck@0.11.0\"]\n"
        ));
        assert!(rust.contains("rust-workflows:v2.0.0:rust-gate\"\n"));
        let other = commit_hooks("# h\n", false, "2.0.0");
        assert_eq!(rust.matches("language: rust\n").count(), 5);
        assert_eq!(other.matches("language: rust\n").count(), 3);
        for (id, entry) in [
            ("rust-gate-rules", "rust-gate rules"),
            ("rust-gate-guide", "rust-gate guide"),
        ] {
            let hook = format!("      - id: {id}\n        name: ");
            assert!(rust.contains(&hook) && other.contains(&hook), "{id}");
            assert!(other.contains(&format!(
                "        entry: {entry}\n        always_run: true\n"
            )));
        }
        assert!(rust.contains("        stages: [pre-push]\n"));
        assert!(!other.contains("id: rustfmt") && !other.contains("id: rust-gate-architecture"));
        assert!(other.contains("id: rust-gate-hygiene"));
    }
}
