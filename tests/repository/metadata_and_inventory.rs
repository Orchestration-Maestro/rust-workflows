//! The repository's metadata: the files every repository carries, the hook,
//! editor, release and issue-form policies, and the Copilot inventory.

use crate::harness::{capture, query, root, tool};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn inventory(directory: &Path, prefix: &str, tree: &mut String, paths: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            !matches!(name, ".git" | ".tools" | "target")
                && !name.ends_with(":Zone.Identifier")
                && !name.ends_with(".cdx.json")
        })
        .collect();
    entries.sort_by_key(|entry| (!entry.file_type().unwrap().is_dir(), entry.file_name()));
    for (index, entry) in entries.iter().enumerate() {
        let last = index + 1 == entries.len();
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        assert!(
            !kind.is_symlink(),
            "Document source symlinks explicitly: {}",
            path.display()
        );
        let branch = if last { "└── " } else { "├── " };
        let suffix = if kind.is_dir() { "/" } else { "" };
        let _ = writeln!(
            tree,
            "{prefix}{branch}{}{suffix}",
            entry.file_name().to_str().unwrap()
        );
        paths.push(path.clone());
        if kind.is_dir() {
            let continuation = if last { "    " } else { "│   " };
            inventory(&path, &format!("{prefix}{continuation}"), tree, paths);
        }
    }
}

#[test]
fn metadata_forms_and_copilot_inventory_are_complete() {
    let root = root();
    required_files_exist(&root);
    hooks_are_local(&root);
    editor_and_git_policies_hold(&root);
    release_manifest_matches_the_version(&root);
    release_workflow_uses_an_app_token(&root);
    dependabot_titles_and_directories_are_wired(&root);
    copilot_tree_is_current(&root);
    guides_point_at_the_standards(&root);
}

/// Every file a repository of this kind carries.
fn required_files_exist(root: &Path) {
    for required in [
        "docs/standards/northstar.md",
        "docs/standards/engineering.md",
        "docs/standards/engineering.md",
        "docs/standards/security.md",
        "docs/standards/controls.md",
        "CONTEXT.md",
        ".pre-commit-config.yaml",
        ".gitattributes",
        ".editorconfig",
        ".github/CODEOWNERS",
        ".github/dependabot.yml",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/feature_request.yml",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
        ".github/copilot-instructions.md",
        ".github/workflows/release-please.yml",
        ".github/release-please/config.json",
        ".github/release-please/manifest.json",
        "version.txt",
        // Without it, a consumer cannot know the terms they are bound by.
        "LICENSE",
        // The single entry point a new contributor runs; without it the setup
        // instructions are a block of commands to paste by hand.
        "scripts/bootstrap.sh",
        // The toolbelt pin and the checksum of every download it resolves to.
        "mise.toml",
        "mise.lock",
    ] {
        assert!(
            root.join(required).is_file(),
            "Missing repository file: {required}"
        );
    }
}

/// Commit hooks come from this repository, not from a remote index.
fn hooks_are_local(root: &Path) {
    let hooks = root.join(".pre-commit-config.yaml");
    let validation = tool("prek")
        .arg("validate-config")
        .arg(&hooks)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        query(&hooks, ".repos[].repo")
            .lines()
            .all(|repo| matches!(repo, "builtin" | "local"))
    );
    assert!(
        query(
            &hooks,
            ".repos[] | select(.repo == \"local\") | .hooks[].language"
        )
        .lines()
        .all(|language| matches!(language, "system" | "pygrep"))
    );
}

/// Line endings and editor settings that syntax depends on.
fn editor_and_git_policies_hold(root: &Path) {
    let attributes = fs::read_to_string(root.join(".gitattributes")).unwrap();
    assert!(attributes.lines().any(|line| line == "* text=auto eol=lf"));
    let editor = fs::read_to_string(root.join(".editorconfig")).unwrap();
    for setting in [
        "root = true",
        "charset = utf-8",
        "end_of_line = lf",
        // Just accepts tabs or spaces but not both in one recipe, so the
        // indentation must be fixed.
        "[justfile]\nindent_size = 4",
        // Markdown turns two trailing spaces into a line break; trimming them
        // silently changes rendered output.
        "[*.md]\ntrim_trailing_whitespace = false",
    ] {
        assert!(editor.contains(setting), "Missing editor policy: {setting}");
    }
    assert!(
        fs::read_to_string(root.join(".github/CODEOWNERS"))
            .unwrap()
            .lines()
            .any(|line| line.starts_with("* @"))
    );
}

/// release-please's configuration names the version file, and the manifest
/// records the version that file holds.
fn release_manifest_matches_the_version(root: &Path) {
    let release = root.join(".github/release-please/config.json");
    assert_eq!(query(&release, ".[\"release-type\"]"), "simple");
    assert_eq!(
        query(&release, ".packages[\".\"][\"version-file\"]"),
        "version.txt"
    );
    let version = fs::read_to_string(root.join("version.txt")).unwrap();
    assert_eq!(
        version.trim(),
        query(
            &root.join(".github/release-please/manifest.json"),
            ".[\".\"]"
        )
    );
    assert!(!version.trim().is_empty());
}

/// release-please runs as an Action on a GitHub App token: the organization
/// forbids `GITHUB_TOKEN` from opening pull requests, and a pull request it
/// opened would trigger none of the checks a merge requires. A release it
/// creates starts the organization's sync at once, through an event to the
/// `.github` repository only.
fn release_workflow_uses_an_app_token(root: &Path) {
    assert!(!root.join(".github/release-please.yml").exists());
    let workflow = root.join(".github/workflows/release-please.yml");
    assert!(
        query(&workflow, ".jobs.release.steps[0].uses")
            .starts_with("actions/create-github-app-token@")
    );
    let step = |field: &str| query(&workflow, &format!(".jobs.release.steps[1]{field}"));
    assert!(step(".uses").starts_with("googleapis/release-please-action@"));
    assert_eq!(step(".with.token"), "${{ steps.token.outputs.token }}");
    assert_eq!(
        step(".with[\"config-file\"]"),
        ".github/release-please/config.json"
    );
    assert_eq!(
        step(".with[\"manifest-file\"]"),
        ".github/release-please/manifest.json"
    );
    let created = "${{ steps.release.outputs.releases_created == 'true' }}";
    let later = |index: usize, field: &str| {
        query(&workflow, &format!(".jobs.release.steps[{index}]{field}"))
    };
    assert_eq!(step(".id"), "release");
    assert!(later(2, ".uses").starts_with("actions/create-github-app-token@"));
    assert_eq!(later(2, ".with.repositories"), ".github");
    assert_eq!(later(2, ".with[\"permission-contents\"]"), "write");
    assert_eq!(
        later(3, ".env.GH_TOKEN"),
        "${{ steps.sync-token.outputs.token }}"
    );
    assert!(later(3, ".run").contains("-f event_type=rust-workflows-release"));
    for index in [2, 3] {
        assert_eq!(later(index, ".if"), created);
    }
}

/// Dependabot covers what is actually here, with titles the organization's
/// conventional-commit ruleset accepts once squashed. Each ecosystem's patch
/// and minor updates arrive as one pull request, which the auto-merge workflow
/// queues; a major update keeps a pull request of its own for review.
fn dependabot_titles_and_directories_are_wired(root: &Path) {
    let dependabot = root.join(".github/dependabot.yml");
    for line in query(
        &dependabot,
        ".updates[] | .groups | to_entries[] \
         | [.value.patterns[], .value[\"update-types\"][]] | join(\" \")",
    )
    .lines()
    {
        assert_eq!(line, "* minor patch");
    }
    let groups = query(&dependabot, ".updates[] | .groups | length");
    assert!(groups.lines().all(|count| count == "1"), "{groups}");
    for line in query(
        &dependabot,
        ".updates[] | [.[\"commit-message\"].prefix, .[\"commit-message\"].include] | join(\" \")",
    )
    .lines()
    {
        assert!(matches!(line, "build scope" | "ci scope"), "{line}");
    }
    assert_eq!(query(&dependabot, ".version"), "2");
    let ecosystems = query(&dependabot, ".updates[][\"package-ecosystem\"]");
    assert!(ecosystems.lines().any(|kind| kind == "cargo"));
    assert!(ecosystems.lines().any(|kind| kind == "github-actions"));
    let directories = query(
        &dependabot,
        ".updates[] | select(.[\"package-ecosystem\"] == \"cargo\") | .directories[]",
    );
    assert!(!directories.is_empty(), "Cargo update coverage is empty");
    for directory in directories.lines() {
        assert!(
            root.join(directory.trim_start_matches('/'))
                .join("Cargo.toml")
                .is_file(),
            "Unknown Cargo update directory: {directory}"
        );
    }
}

/// The Copilot tree names every file with an explanation, and nothing else.
fn copilot_tree_is_current(root: &Path) {
    let guide = fs::read_to_string(root.join(".github/copilot-instructions.md")).unwrap();
    let mut tree = String::from(".\n");
    inventory(root, "", &mut tree, &mut Vec::new());
    let annotated = guide
        .split_once("```text\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    let mut plain = String::new();
    for line in annotated.lines() {
        let (entry, description) = line
            .split_once("  # ")
            .unwrap_or_else(|| panic!("Inline tree explanation missing: {line}"));
        assert!(
            !description.trim().is_empty(),
            "Empty tree explanation: {entry}"
        );
        let _ = writeln!(plain, "{}", entry.trim_end());
    }
    assert_eq!(plain, tree, "Copilot repository tree is stale");
    assert!(
        guide.contains("../AGENTS.md"),
        "Link the authoritative repository rules"
    );
    assert!(guide.contains("../docs/standards/engineering.md"));
    assert!(guide.contains("../docs/standards/northstar.md"));
}

/// The README, AGENTS.md and the Copilot guide point at the standards.
fn guides_point_at_the_standards(root: &Path) {
    for path in ["README.md", "AGENTS.md"] {
        let text = fs::read_to_string(root.join(path)).unwrap();
        assert!(
            text.contains("docs/standards/northstar.md"),
            "Missing North Star pointer: {path}"
        );
        assert!(
            text.contains("CONTEXT.md"),
            "Missing glossary pointer: {path}"
        );
    }
    let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(agents.contains(".github/copilot-instructions.md"));
    assert!(agents.contains("docs/standards/engineering.md"));
    let just = fs::read_to_string(root.join("justfile")).unwrap();
    assert!(just.contains("-D missing_docs") && just.contains("cargo doc"));
}

#[test]
fn every_just_recipe_carries_its_own_description() {
    // `just` alone prints this list, so a recipe without a description is a
    // command a contributor can see and not understand. The list comes from
    // just itself rather than from the file, because that is what they read.
    let listed = capture(
        tool("just")
            .args(["--list", "--unsorted"])
            .current_dir(root()),
    );
    let recipes: Vec<&str> = listed
        .lines()
        .skip_while(|line| !line.starts_with("Available recipes:"))
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        recipes
            .iter()
            .map(|recipe| recipe.split_whitespace().next().unwrap())
            .collect::<Vec<_>>(),
        ["help", "setup", "check", "update-tools", "docs"],
        "the local entry points must list only native repository tooling:\n{listed}"
    );
    for recipe in recipes {
        assert!(
            recipe.contains('#'),
            "just lists `{}` with no description",
            recipe.trim()
        );
    }
    // The first recipe is the one a bare `just` runs, so it must be the list
    // rather than a gate that takes minutes or a command that writes to disk.
    assert!(
        listed
            .lines()
            .nth(1)
            .is_some_and(|first| first.trim_start().starts_with("help ")),
        "`help` must be the first recipe, so a bare `just` prints the list:\n{listed}"
    );
}

#[test]
fn every_code_token_a_tree_comment_names_lives_in_its_file() {
    // The inventory gate compares the entries, so a comment can keep naming an
    // artefact its file no longer holds and nothing notices; that is how two of
    // these rotted. This settles the part a machine can settle: a token with an
    // underscore is a name out of the code and has to be in the file the
    // comment describes. A hyphenated word is prose here and is left alone.
    let root = root();
    let mut tree = String::from(".\n");
    let mut paths = vec![root.clone()];
    inventory(&root, "", &mut tree, &mut paths);
    let guide = fs::read_to_string(root.join(".github/copilot-instructions.md")).unwrap();
    let annotated: Vec<&str> = guide
        .split_once("```text\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0
        .lines()
        .collect();
    assert_eq!(
        annotated.len(),
        paths.len(),
        "the tree and the walk disagree; the inventory gate names why"
    );
    let mut checked = 0;
    for (line, path) in annotated.iter().zip(&paths) {
        let Some((_, comment)) = line.split_once("  # ") else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let tokens: Vec<&str> = comment
            .split(|character: char| !character.is_alphanumeric() && character != '_')
            .filter(|token| token.contains('_') && token.len() >= 4)
            .collect();
        if tokens.is_empty() {
            continue;
        }
        // A banner is bytes, not text: nothing in it can be a name to find.
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for token in tokens {
            assert!(
                text.contains(token),
                "{}: the tree says {token}, which the file does not hold",
                path.strip_prefix(&root).unwrap().display()
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no code token was checked; the walk drifted");
}
