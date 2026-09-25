//! The steps: `sync` writes every managed file, deletes each retired one it
//! wrote before, the caller of `ci.yml` included, and moves every call to
//! rust-workflows to the release the repository declares; `sync --check` and
//! the `managed-files` step of `ci.yml` refuse any difference, a retired file
//! still present included, Markdown settings at the root, which rumdl would
//! read over the organization's, and tool pins of the repository's own, where
//! `rust-gate setup` installs the organization's toolbelt; `init` writes them
//! for a repository that declares no release yet, with its rule map and, in a
//! git repository, its Copilot guide.

use super::pin::{Pin, caller_pin, hook_version, parse_pin, repinned, workflow_pins};
use super::render::{RETIRED, Repository, managed_files};
use crate::checks::organization_config::is_generated;
use crate::checks::quality_config::read_config;
use crate::checks::workflow_home::is_workflow_home;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, input, optional, summary, write};
use std::fs;
use std::path::{Path, PathBuf};

/// What these steps declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "ci",
        id: "managed-files",
        summary: "Managed files as the organization renders them",
        inputs: &["GITHUB_WORKSPACE"],
        tools: &["jaq"],
        reports: &["managed-files.txt"],
        run: in_ci,
    },
    Step {
        workflow: "local",
        id: "sync",
        summary: "Every managed file written as the organization renders it",
        inputs: &["RUST_WORKFLOWS_PIN"],
        tools: &["jaq"],
        reports: &[],
        run: sync,
    },
    Step {
        workflow: "local",
        id: "sync --check",
        summary: "Every managed file compared with the organization's rendering",
        inputs: &[],
        tools: &["jaq"],
        reports: &[],
        run: check,
    },
    Step {
        workflow: "local",
        id: "init",
        summary: "The managed files of a repository that declares no release yet",
        inputs: &["RUST_WORKFLOWS_PIN"],
        tools: &["jaq", "rust-gate"],
        reports: &[],
        run: init,
    },
];

/// The caller of `ci.yml` or `hygiene.yml` every repository but the home of
/// the workflows held, until the organization's rulesets ran them instead.
const CALLER: &str = ".github/workflows/ci.yml";

/// The commit hooks, whose gate hook names the release's version.
const HOOKS: &str = ".pre-commit-config.yaml";

/// Whether a manifest is a workspace root: it holds a `[workspace]` table.
const IS_WORKSPACE: &str = "has(\"workspace\")";

/// The files rumdl reads its settings from at the root of a repository. The
/// commit hook passes the organization's settings over them, so one of them
/// at the root would loosen every Markdown file; in a subdirectory, rumdl
/// applies it to that subdirectory alone.
const MARKDOWN_SETTINGS: [&str; 10] = [
    ".config/rumdl.toml",
    ".markdownlint-cli2.jsonc",
    ".markdownlint-cli2.yaml",
    ".markdownlint.json",
    ".markdownlint.jsonc",
    ".markdownlint.yaml",
    ".markdownlint.yml",
    ".rumdl.toml",
    "pyproject.toml",
    "rumdl.toml",
];

/// The files a repository would pin tools of its own in, beside the
/// organization's toolbelt that `rust-gate setup` installs: two pins of one
/// tool drift apart, and the one CI runs is the gate's. A
/// `scripts/bootstrap.sh` counts when it fetches mise, the toolbelt's
/// bootstrap; the home keeps all of them, as the source of the pins.
const TOOL_PINS: [&str; 6] = [
    ".github/workflows/tool-updates.yml",
    ".mise.toml",
    ".tool-versions",
    "mise.lock",
    "mise.toml",
    "scripts/bootstrap.sh",
];

/// Run `managed-files`: the checkout's managed files against the rendering.
fn in_ci() -> Outcome {
    let job = Job::current()?;
    let root = canonical(&PathBuf::from(input("GITHUB_WORKSPACE")?))?;
    let differing = differences(&root, None)?;
    let report = job.report("managed-files.txt")?;
    let mut listing = differing.join("\n");
    if !listing.is_empty() {
        listing.push('\n');
    }
    write(&report, listing.as_bytes(), false)?;
    summary(&format!(
        "## Managed files\n\n{} of the organization's managed files differ; the list is \
         `managed-files.txt` in the reports artifact.\n",
        differing.len()
    ))?;
    refuse("managed files", &differing)?;
    refuse_root_markdown("managed files", &root)?;
    refuse_tool_pins("managed files", &root)
}

/// Run `sync`: write every managed file of the repository here.
fn sync() -> Outcome {
    let root = canonical(Path::new("."))?;
    let pin = optional("RUST_WORKFLOWS_PIN")?;
    write_all(&root, Some(pin.as_str()).filter(|pin| !pin.is_empty()))
}

/// Run `sync --check`: refuse any managed file here that differs.
fn check() -> Outcome {
    let root = canonical(Path::new("."))?;
    refuse("sync --check", &differences(&root, None)?)?;
    refuse_root_markdown("sync --check", &root)?;
    refuse_tool_pins("sync --check", &root)
}

/// Run `init`: the managed files of a repository that declares no release
/// yet, at the release `RUST_WORKFLOWS_PIN` names.
fn init() -> Outcome {
    let root = canonical(Path::new("."))?;
    if declared_version(&root)?.is_some() {
        return Err("init: the repository already declares a release; run rust-gate sync".into());
    }
    let pin = optional("RUST_WORKFLOWS_PIN")?;
    if pin.is_empty() {
        return Err(
            "init: set RUST_WORKFLOWS_PIN to `<commit> v<version>`, the release to pin".into(),
        );
    }
    write_all(&root, Some(&pin))?;
    adapted(&root)
}

/// Write the repository's rule map and, in a git repository, its Copilot
/// guide, which the commit hooks keep current from then on; the guide lists
/// tracked files, so outside git the first commit's hook writes it.
fn adapted(root: &Path) -> Outcome {
    Cmd::new("rust-gate rules").cwd(root).run()?;
    if root.join(".git").exists() {
        return Cmd::new("rust-gate guide").cwd(root).run();
    }
    println!("guide: not a git repository yet; the first commit's hook writes it");
    Ok(())
}

/// Refuse the `differing` files, by name, with the fix.
fn refuse(context: &str, differing: &[String]) -> Outcome {
    if differing.is_empty() {
        return Ok(());
    }
    let count = if differing.len() == 1 {
        "1 managed file differs".to_owned()
    } else {
        format!("{} managed files differ", differing.len())
    };
    Err(Failure::from(format!(
        "{context}: {count} from the organization's rendering ({}); run rust-gate sync",
        differing.join(", ")
    )))
}

/// Refuse the Markdown settings rumdl would read at `root`: a `pyproject.toml`
/// only when it holds rumdl's table. The home's are its own.
fn refuse_root_markdown(context: &str, root: &Path) -> Outcome {
    if is_workflow_home(root) {
        return Ok(());
    }
    let found: Vec<&str> = MARKDOWN_SETTINGS
        .into_iter()
        .filter(|path| match fs::read_to_string(root.join(path)) {
            Ok(text) => *path != "pyproject.toml" || text.contains("[tool.rumdl"),
            Err(_) => false,
        })
        .collect();
    if found.is_empty() {
        return Ok(());
    }
    Err(Failure::from(format!(
        "{context}: {} would loosen the organization's Markdown settings at the root; a \
         .rumdl.toml in a subdirectory applies to it alone",
        found.join(", ")
    )))
}

/// Refuse the tool pins of the repository at `root`, by name, with what
/// replaces them. The home's are the organization's.
fn refuse_tool_pins(context: &str, root: &Path) -> Outcome {
    if is_workflow_home(root) {
        return Ok(());
    }
    let found: Vec<&str> = TOOL_PINS
        .into_iter()
        .filter(|path| match fs::read_to_string(root.join(path)) {
            Ok(text) => *path != "scripts/bootstrap.sh" || text.contains("mise"),
            Err(_) => false,
        })
        .collect();
    if found.is_empty() {
        return Ok(());
    }
    Err(Failure::from(format!(
        "{context}: the repository pins tools of its own in {}; delete them, and run \
         rust-gate setup, which installs every tool CI runs at the version it runs",
        found.join(", ")
    )))
}

/// Write every managed file of the repository at `root`, the release `pin`
/// names or, without one, the release it declares, and delete each retired
/// one sync wrote.
fn write_all(root: &Path, pin: Option<&str>) -> Outcome {
    for (path, text) in rendered(root, pin)? {
        let file = root.join(&path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{path}: {error}"))?;
        }
        write(&file, text.as_bytes(), false)?;
    }
    for path in leftovers(root) {
        fs::remove_file(root.join(&path)).map_err(|error| format!("{path}: {error}"))?;
    }
    Ok(())
}

/// The managed files of the repository at `root` whose bytes differ from the
/// rendering, missing ones included, and the retired ones sync wrote.
fn differences(root: &Path, pin: Option<&str>) -> Result<Vec<String>, Failure> {
    let mut differing: Vec<String> = rendered(root, pin)?
        .into_iter()
        .filter(|(path, text)| fs::read_to_string(root.join(path)).ok().as_ref() != Some(text))
        .map(|(path, _)| path)
        .chain(leftovers(root))
        .collect();
    differing.sort();
    Ok(differing)
}

/// The retired files of the repository at `root` that still open with the
/// header sync wrote them with. A file without it is the repository's own,
/// as the home's are, which its own tools read.
fn leftovers(root: &Path) -> Vec<String> {
    RETIRED
        .iter()
        .filter(|path| is_generated(&root.join(path)))
        .map(|path| (*path).to_owned())
        .collect()
}

/// The release the repository at `root` pins, the first found: `pin`, from
/// `RUST_WORKFLOWS_PIN`; the retired caller while it is there; then every
/// other workflow that calls rust-workflows at a commit, which must all name
/// the same one.
fn declared_pin(root: &Path, pin: Option<&str>) -> Result<Option<Pin>, Failure> {
    if let Some(text) = pin {
        let parsed = parse_pin(text)
            .ok_or_else(|| format!("RUST_WORKFLOWS_PIN `{text}` is not `<commit> v<version>`"))?;
        return Ok(Some(parsed));
    }
    let caller = fs::read_to_string(root.join(CALLER)).ok();
    if let Some(pin) = caller.as_deref().and_then(caller_pin) {
        return Ok(Some(pin));
    }
    let mut named: Vec<(String, Pin)> = Vec::new();
    for (path, text) in workflows(root) {
        for pin in workflow_pins(&text) {
            if !named
                .iter()
                .any(|(seen, known)| *seen == path && *known == pin)
            {
                named.push((path.clone(), pin));
            }
        }
    }
    let Some((_, first)) = named.first() else {
        return Ok(None);
    };
    if named.iter().all(|(_, pin)| pin == first) {
        return Ok(named.into_iter().next().map(|(_, pin)| pin));
    }
    named.sort_by(|left, right| left.0.cmp(&right.0));
    let listed: Vec<String> = named
        .iter()
        .map(|(path, pin)| format!("{path} pins v{} at {}", pin.version, pin.commit))
        .collect();
    Err(Failure::from(format!(
        "the workflows pin different releases of rust-workflows ({}); set RUST_WORKFLOWS_PIN to \
         `<commit> v<version>`, the release to keep, and run rust-gate sync",
        listed.join(", ")
    )))
}

/// The version of the release the repository at `root` declares: its pin's,
/// or, when no workflow pins one, the one its commit hooks install.
fn declared_version(root: &Path) -> Result<Option<String>, Failure> {
    Ok(match declared_pin(root, None)? {
        Some(pin) => Some(pin.version),
        None => hooks_version(root),
    })
}

/// The version of the gate the commit hooks of the repository at `root`
/// install.
fn hooks_version(root: &Path) -> Option<String> {
    fs::read_to_string(root.join(HOOKS))
        .ok()
        .as_deref()
        .and_then(hook_version)
}

/// Every managed file of the repository at `root`, rendered.
fn rendered(root: &Path, pin: Option<&str>) -> Result<Vec<(String, String)>, Failure> {
    let home = is_workflow_home(root);
    let pin = if home { None } else { declared_pin(root, pin)? };
    let version = match &pin {
        Some(pin) => Some(pin.version.clone()),
        None if home => None,
        None => hooks_version(root),
    };
    let manifest_path = root.join("Cargo.toml");
    let manifest = match fs::read_to_string(&manifest_path) {
        Ok(text) => {
            let workspace = Cmd::new("jaq --from toml")
                .arg(IS_WORKSPACE)
                .arg(&manifest_path)
                .capture()?;
            Some((text, workspace.trim() == "true"))
        }
        Err(_) => None,
    };
    let others = pin
        .as_ref()
        .map(|pin| repinned_workflows(root, pin))
        .unwrap_or_default();
    let repository = Repository {
        rust: manifest.is_some() || root.join("rust-toolchain.toml").is_file(),
        home,
        manifest,
        config: read_config(root)?,
        version,
    };
    let mut files = managed_files(&repository)?;
    files.extend(others);
    files.sort();
    Ok(files)
}

/// Where a repository keeps workflows that may call rust-workflows: its own,
/// and the organization's workflow templates in `.github`.
const WORKFLOW_DIRECTORIES: [&str; 2] = [".github/workflows", "workflow-templates"];

/// Every workflow of the repository at `root` but the retired caller, which
/// sync deletes, and its text.
fn workflows(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for directory in WORKFLOW_DIRECTORIES {
        let Ok(entries) = fs::read_dir(root.join(directory)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = format!("{directory}/{}", entry.file_name().to_string_lossy());
            if path != CALLER {
                found.push((path, fs::read_to_string(entry.path()).unwrap_or_default()));
            }
        }
    }
    found
}

/// Every workflow of the repository at `root` that calls rust-workflows,
/// each call moved to `pin`: a release pinned in one workflow and an older one
/// in `release.yml` would release with a gate the check never ran.
fn repinned_workflows(root: &Path, pin: &Pin) -> Vec<(String, String)> {
    workflows(root)
        .into_iter()
        .filter_map(|(path, text)| {
            let moved = repinned(&text, pin);
            (moved != text).then_some((path, moved))
        })
        .collect()
}

/// `path` resolved, or a refusal naming it.
fn canonical(path: &Path) -> Result<PathBuf, Failure> {
    fs::canonicalize(path).map_err(|error| Failure::from(format!("{}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::{CALLER, HOOKS, Pin, declared_pin, declared_version, refuse, repinned_workflows};
    use std::{env, fs, process};

    #[test]
    fn a_release_is_found_in_the_declared_order_and_pins_must_agree() {
        let root = env::temp_dir().join(format!("declared-release-{}", process::id()));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        assert_eq!(declared_version(&root).unwrap(), None);
        let hooks = "- \"cli:https://github.com/Orchestration-Maestro/rust-workflows:v2.0.0:\
                     rust-gate\"\n";
        fs::write(root.join(HOOKS), hooks).unwrap();
        assert_eq!(declared_pin(&root, None).unwrap(), None);
        assert_eq!(declared_version(&root).unwrap().as_deref(), Some("2.0.0"));
        let call = |letter: &str, version: &str| {
            format!(
                "uses: Orchestration-Maestro/rust-workflows/.github/workflows/ci.yml@{}  \
                 # v{version}\n",
                letter.repeat(40)
            )
        };
        let release = root.join(".github/workflows/release.yml");
        fs::write(&release, call("b", "2.1.0")).unwrap();
        let pin = |letter: &str, version: &str| Pin {
            commit: letter.repeat(40),
            version: version.to_owned(),
        };
        assert_eq!(declared_pin(&root, None).unwrap(), Some(pin("b", "2.1.0")));
        assert_eq!(declared_version(&root).unwrap().as_deref(), Some("2.1.0"));
        fs::write(root.join(CALLER), call("c", "2.2.0")).unwrap();
        assert_eq!(declared_pin(&root, None).unwrap(), Some(pin("c", "2.2.0")));
        let given = format!("{} v2.3.0", "d".repeat(40));
        assert_eq!(
            declared_pin(&root, Some(&given)).unwrap(),
            Some(pin("d", "2.3.0"))
        );
        fs::remove_file(root.join(CALLER)).unwrap();
        fs::write(
            root.join(".github/workflows/nightly.yml"),
            call("a", "2.0.0"),
        )
        .unwrap();
        let refusal = declared_pin(&root, None).unwrap_err();
        assert_eq!(
            refusal.message.unwrap_or_default(),
            format!(
                "the workflows pin different releases of rust-workflows \
                 (.github/workflows/nightly.yml pins v2.0.0 at {}, .github/workflows/release.yml \
                 pins v2.1.0 at {}); set RUST_WORKFLOWS_PIN to `<commit> v<version>`, the \
                 release to keep, and run rust-gate sync",
                "a".repeat(40),
                "b".repeat(40)
            )
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_workflow_but_the_retired_caller_moves_to_the_pin() {
        let root = env::temp_dir().join(format!("repinned-workflows-{}", process::id()));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        fs::create_dir_all(root.join("workflow-templates")).unwrap();
        let old = format!(
            "uses: Orchestration-Maestro/rust-workflows/.github/workflows/ci.yml@{}  # v1.2.1\n",
            "a".repeat(40)
        );
        for path in [
            CALLER,
            ".github/workflows/release.yml",
            "workflow-templates/rust-ci.yml",
        ] {
            fs::write(root.join(path), &old).unwrap();
        }
        fs::write(root.join(".github/workflows/scorecard.yml"), "on: push\n").unwrap();
        let pin = Pin {
            commit: "b".repeat(40),
            version: "2.1.0".to_owned(),
        };
        let mut found = repinned_workflows(&root, &pin);
        found.sort();
        let new = old
            .replace(&"a".repeat(40), &"b".repeat(40))
            .replace("1.2.1", "2.1.0");
        assert_eq!(
            found,
            [
                (".github/workflows/release.yml".to_owned(), new.clone()),
                ("workflow-templates/rust-ci.yml".to_owned(), new)
            ]
        );
        fs::remove_dir_all(&root).unwrap();
        assert!(repinned_workflows(&root, &pin).is_empty());
    }

    #[test]
    fn differing_files_are_refused_by_name_with_the_fix() {
        assert!(refuse("sync --check", &[]).is_ok());
        let one = refuse("sync --check", &["clippy.toml".to_owned()]).unwrap_err();
        assert_eq!(
            one.message.unwrap_or_default(),
            "sync --check: 1 managed file differs from the organization's rendering \
             (clippy.toml); run rust-gate sync"
        );
        let two = refuse("managed files", &["a".to_owned(), "b".to_owned()]).unwrap_err();
        assert_eq!(
            two.message.unwrap_or_default(),
            "managed files: 2 managed files differ from the organization's rendering (a, b); \
             run rust-gate sync"
        );
    }
}
