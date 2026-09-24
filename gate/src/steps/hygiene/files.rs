//! HYG-002 to HYG-005, read from the tracked files themselves: no pending
//! snapshot, no large file, executables and shebangs that agree, paths that
//! differ by more than case, symlinks that stay inside, and the files every
//! repository holds.

use crate::checks::findings::Finding;
use std::collections::BTreeMap;
use std::path::Path;

/// The largest file a repository tracks without an exception, in bytes.
const LARGEST: u64 = 500 * 1024;

/// The configurations that make a repository release with release-please.
const RELEASE_PLEASE: &[&str] = &[
    ".github/release-please/config.json",
    "release-please-config.json",
    ".release-please-manifest.json",
];

/// HYG-002 to HYG-005 over `files`.
pub(super) fn findings(workspace: &Path, files: &[String]) -> Vec<Finding> {
    let mut found = Vec::new();
    for file in files {
        let path = workspace.join(file);
        if file.ends_with(".snap.new") || file.ends_with(".pending-snap") {
            let message = "a pending snapshot; accept or reject it, never commit it";
            found.push(Finding::new("HYG-002", file.clone(), 0, message.to_owned()));
        }
        found.extend(content_findings(workspace, file, &path));
    }
    found.extend(case_conflicts(files));
    found.extend(required_files(files));
    found
}

/// HYG-003 and HYG-004 for one tracked file: its size, its mode and its
/// shebang, or where it points when it is a symlink.
fn content_findings(workspace: &Path, file: &str, path: &Path) -> Vec<Finding> {
    let finding = |rule: &str, message: String| Finding::new(rule, file.to_owned(), 0, message);
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink() {
        let message = match std::fs::canonicalize(path) {
            Err(_) => "a symlink whose target is missing",
            Ok(target) if !target.starts_with(workspace) => "a symlink that leaves the repository",
            Ok(_) => return Vec::new(),
        };
        return vec![finding("HYG-004", message.to_owned())];
    }
    let mut found = Vec::new();
    if metadata.len() > LARGEST {
        let message = format!(
            "{} KB, over 500 KB; keep large files out of the repository, or record the asset \
             in maestro-quality.toml",
            metadata.len() / 1024
        );
        found.push(finding("HYG-003", message));
    }
    let shebang = std::fs::read(path).is_ok_and(|bytes| {
        bytes.starts_with(b"#!") && bytes.get(2).is_some_and(|&byte| byte != b'[')
    });
    match (executable(&metadata), shebang) {
        (true, false) => found.push(finding(
            "HYG-004",
            "executable without a shebang; add one or drop the executable bit".to_owned(),
        )),
        (false, true) => found.push(finding(
            "HYG-004",
            "a shebang without the executable bit; make it executable".to_owned(),
        )),
        _ => {}
    }
    found
}

/// Whether the file's mode lets anybody execute it.
#[cfg(unix)]
fn executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

/// Whether the file's mode lets anybody execute it: never, where modes are
/// not Unix modes.
#[cfg(not(unix))]
fn executable(_metadata: &std::fs::Metadata) -> bool {
    false
}

/// HYG-004 for every path that differs from an earlier one only by case.
fn case_conflicts(files: &[String]) -> Vec<Finding> {
    let mut first: BTreeMap<String, &String> = BTreeMap::new();
    let mut found = Vec::new();
    for file in files {
        match first.get(&file.to_lowercase()) {
            Some(other) => found.push(Finding::new(
                "HYG-004",
                file.clone(),
                0,
                format!("differs from `{other}` only by case"),
            )),
            None => {
                first.insert(file.to_lowercase(), file);
            }
        }
    }
    found
}

/// HYG-005: the files every repository holds at its root, and the changelog
/// a release-please repository writes.
fn required_files(files: &[String]) -> Vec<Finding> {
    let has = |name: &str| files.iter().any(|file| file == name);
    let mut found = Vec::new();
    if !has("README.md") {
        let message = "README.md is missing; say what the repository is for";
        found.push(Finding::new(
            "HYG-005",
            "README.md".to_owned(),
            0,
            message.to_owned(),
        ));
    }
    if !files
        .iter()
        .any(|file| !file.contains('/') && file.starts_with("LICENSE"))
    {
        let message = "LICENSE is missing; say how others may use the code";
        found.push(Finding::new(
            "HYG-005",
            "LICENSE".to_owned(),
            0,
            message.to_owned(),
        ));
    }
    if RELEASE_PLEASE.iter().any(|config| has(config)) && !has("CHANGELOG.md") {
        let message = "CHANGELOG.md is missing beside a release-please configuration";
        found.push(Finding::new(
            "HYG-005",
            "CHANGELOG.md".to_owned(),
            0,
            message.to_owned(),
        ));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{case_conflicts, required_files};

    /// Owned paths.
    fn owned(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn paths_differing_only_by_case_are_refused() {
        let found = case_conflicts(&owned(&["README.md", "docs/a.md", "Readme.md"]));
        assert_eq!(
            found.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["HYG-004 Readme.md: differs from `README.md` only by case"]
        );
    }

    #[test]
    fn every_repository_holds_a_readme_a_licence_and_its_changelog() {
        assert!(required_files(&owned(&["README.md", "LICENSE-MIT"])).is_empty());
        let found = required_files(&owned(&["release-please-config.json"]));
        assert_eq!(
            found.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [
                "HYG-005 README.md: README.md is missing; say what the repository is for",
                "HYG-005 LICENSE: LICENSE is missing; say how others may use the code",
                "HYG-005 CHANGELOG.md: CHANGELOG.md is missing beside a release-please \
                 configuration",
            ]
        );
    }
}
