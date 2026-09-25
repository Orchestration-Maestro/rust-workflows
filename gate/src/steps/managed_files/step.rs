//! The steps: `sync` writes every managed file, and moves every other call to
//! rust-workflows to the caller's release; `sync --check` and the
//! `managed-files` step of `ci.yml` refuse any difference, and `init` writes
//! them for a repository whose caller pins no release yet, with its rule map
//! and, in a git repository, its Copilot guide.

use super::pin::{Pin, caller_pin, parse_pin, repinned};
use super::render::{EXECUTABLE, Repository, managed_files};
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
        summary: "The managed files of a repository whose caller pins no release yet",
        inputs: &["RUST_WORKFLOWS_PIN"],
        tools: &["jaq", "rust-gate"],
        reports: &[],
        run: init,
    },
];

/// The caller every repository but the home of the workflows holds.
const CALLER: &str = ".github/workflows/ci.yml";

/// Whether a manifest is a workspace root: it holds a `[workspace]` table.
const IS_WORKSPACE: &str = "has(\"workspace\")";

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
    refuse("managed files", &differing)
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
    refuse("sync --check", &differences(&root, None)?)
}

/// Run `init`: the managed files of a repository with no caller yet, at the
/// release `RUST_WORKFLOWS_PIN` names.
fn init() -> Outcome {
    let root = canonical(Path::new("."))?;
    if root.join(CALLER).is_file() {
        return Err("init: the repository already has a caller; run rust-gate sync".into());
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

/// Write every managed file of the repository at `root`, the release `pin`
/// names or, without one, the release its caller pins.
fn write_all(root: &Path, pin: Option<&str>) -> Outcome {
    for (path, text) in rendered(root, pin)? {
        let file = root.join(&path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{path}: {error}"))?;
        }
        write(&file, text.as_bytes(), false)?;
        if path == EXECUTABLE {
            executable(&file)?;
        }
    }
    Ok(())
}

/// Let anybody run `file`, as a script's shebang expects.
#[cfg(unix)]
fn executable(file: &Path) -> Outcome {
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = fs::metadata(file)
        .map_err(|error| format!("{}: {error}", file.display()))?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(file, permissions)
        .map_err(|error| Failure::from(format!("{}: {error}", file.display())))
}

/// Nothing to do where modes are not Unix modes: Git records the bit.
#[cfg(not(unix))]
fn executable(_file: &Path) -> Outcome {
    Ok(())
}

/// The managed files of the repository at `root` whose bytes differ from the
/// rendering, missing ones included.
fn differences(root: &Path, pin: Option<&str>) -> Result<Vec<String>, Failure> {
    Ok(rendered(root, pin)?
        .into_iter()
        .filter(|(path, text)| fs::read_to_string(root.join(path)).ok().as_ref() != Some(text))
        .map(|(path, _)| path)
        .collect())
}

/// Every managed file of the repository at `root`, rendered.
fn rendered(root: &Path, pin: Option<&str>) -> Result<Vec<(String, String)>, Failure> {
    let pin =
        match pin {
            Some(text) => Some(parse_pin(text).ok_or_else(|| {
                format!("RUST_WORKFLOWS_PIN `{text}` is not `<commit> v<version>`")
            })?),
            None => fs::read_to_string(root.join(CALLER))
                .ok()
                .and_then(|caller| caller_pin(&caller)),
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
    let home = is_workflow_home(root);
    let others = match (&pin, home) {
        (Some(pin), false) => repinned_workflows(root, pin),
        _ => Vec::new(),
    };
    let repository = Repository {
        rust: manifest.is_some() || root.join("rust-toolchain.toml").is_file(),
        home,
        manifest,
        config: read_config(root)?,
        pin,
    };
    let mut files = managed_files(&repository)?;
    files.extend(others);
    files.sort();
    Ok(files)
}

/// Where a repository keeps workflows that may call rust-workflows: its own,
/// and the organization's workflow templates in `.github`.
const WORKFLOW_DIRECTORIES: [&str; 2] = [".github/workflows", "workflow-templates"];

/// Every other workflow of the repository at `root` that calls
/// rust-workflows, each call moved to `pin`: a release pinned in `ci.yml` and
/// an older one in `release.yml` would test one gate and release with another.
fn repinned_workflows(root: &Path, pin: &Pin) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for directory in WORKFLOW_DIRECTORIES {
        let Ok(entries) = fs::read_dir(root.join(directory)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = format!("{directory}/{}", entry.file_name().to_string_lossy());
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            let moved = repinned(&text, pin);
            if path != CALLER && moved != text {
                found.push((path, moved));
            }
        }
    }
    found
}

/// `path` resolved, or a refusal naming it.
fn canonical(path: &Path) -> Result<PathBuf, Failure> {
    fs::canonicalize(path).map_err(|error| Failure::from(format!("{}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::{CALLER, Pin, refuse, repinned_workflows};
    use std::{env, fs, process};

    #[test]
    fn every_workflow_but_the_caller_moves_to_the_caller_release() {
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
