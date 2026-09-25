//! What CI's cache step keeps from one run to the next, kept the same way
//! here: the directories its `path:` names, moved into the job after the
//! `registry` step made its fresh Cargo home, and moved back out when the run
//! ends, whatever it came to. A move, not a copy, so a build directory of
//! gigabytes costs nothing.

use crate::runner::Failure;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// What the cache step keeps: a directory of the job, named by the variable
/// that holds it, and the path under it.
const KEPT: &[(&str, &str)] = &[
    ("CARGO_HOME", "registry/index"),
    ("CARGO_HOME", "registry/cache"),
    ("CARGO_HOME", "git/db"),
    ("RUNNER_TEMP", "rust-target"),
];

/// Each kept directory: where the cache holds it, and where the job does,
/// when `job` names the variable's directory.
fn places(cache: &Path, job: &dyn Fn(&str) -> String) -> Vec<(PathBuf, PathBuf)> {
    KEPT.iter()
        .filter_map(|(variable, path)| {
            let directory = job(variable);
            (!directory.is_empty()).then(|| {
                (
                    cache.join(variable).join(path),
                    Path::new(&directory).join(path),
                )
            })
        })
        .collect()
}

/// Move what the cache holds into the job.
pub(super) fn restore(cache: &Path, job: &dyn Fn(&str) -> String) -> Result<usize, Failure> {
    let mut moved = 0;
    for (kept, used) in places(cache, job) {
        moved += usize::from(shift(&kept, &used)?);
    }
    Ok(moved)
}

/// Move what the job built back into the cache.
pub(super) fn save(cache: &Path, job: &dyn Fn(&str) -> String) -> Result<(), Failure> {
    for (kept, used) in places(cache, job) {
        shift(&used, &kept)?;
    }
    Ok(())
}

/// Move `from` to `to` when `from` exists and `to` does not: whether it
/// moved.
fn shift(from: &Path, to: &Path) -> Result<bool, Failure> {
    if !from.is_dir() || to.exists() {
        return Ok(false);
    }
    let failed = |error: io::Error| Failure::from(format!("{}: {error}", to.display()));
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(failed)?;
    }
    fs::rename(from, to).map_err(failed)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{restore, save};
    use std::env;
    use std::fs;
    use std::process;

    #[test]
    fn a_build_directory_moves_in_and_back_out() {
        let root = env::temp_dir().join(format!("local-ci-cache-{}", process::id()));
        fs::remove_dir_all(&root).ok();
        let (cache, temp) = (root.join("cache"), root.join("temp"));
        fs::create_dir_all(cache.join("RUNNER_TEMP/rust-target/debug")).unwrap();
        let home = temp.join("cargo-home");
        let job = |variable: &str| match variable {
            "RUNNER_TEMP" => temp.display().to_string(),
            "CARGO_HOME" => home.display().to_string(),
            _ => String::new(),
        };
        assert_eq!(restore(&cache, &job).unwrap(), 1);
        assert!(temp.join("rust-target/debug").is_dir());
        fs::create_dir_all(home.join("registry/index")).unwrap();
        save(&cache, &job).unwrap();
        assert!(cache.join("RUNNER_TEMP/rust-target/debug").is_dir());
        assert!(cache.join("CARGO_HOME/registry/index").is_dir());
        assert!(!temp.join("rust-target").exists());
        let nothing = |_: &str| String::new();
        assert_eq!(restore(&cache, &nothing).unwrap(), 0);
        fs::remove_dir_all(&root).unwrap();
    }
}
