//! `rust-gate install-tools`: GitHub release assets downloaded directly,
//! every digest verified before anything is extracted, the executables
//! installed under the runner's temporary directory and put on the PATH of
//! every later step. This is how untrusted bytes become executables on a
//! runner, so it lives in one place.

use crate::checks::private_directories::private_directory;
use crate::checks::simple_names::{is_hex, simple};
use crate::runner::{Cmd, Outcome, Step, add_to_path, input, native_linux, path};
use std::fs;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "shared",
    id: "install-tools",
    summary: "Install pinned tools, each download verified by digest before extraction",
    inputs: &["TOOLS"],
    tools: &["curl", "install", "sha256sum", "tar"],
    reports: &[],
    run,
}];

/// Where every release asset is downloaded from.
const RELEASES: &str = "https://github.com";

/// Run the step. `TOOLS` holds one tool per line, `<name> <owner>/<repo>/
/// releases/download/<tag>/<asset> <sha256> [<member>]`, the member being the
/// executable's path inside an archive; `#` lines are comments.
fn run() -> Outcome {
    native_linux()?;
    let bin = path("RUNNER_TEMP")?.join("rust-tools/bin");
    fs::create_dir_all(&bin)
        .map_err(|error| format!("cannot create {}: {error}", bin.display()))?;
    let downloads = private_directory(&input("RUNNER_TEMP")?, "tool-downloads")?;
    let mut installed = 0;
    for line in input("TOOLS")?.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(name) = words.first() else {
            continue;
        };
        if name.starts_with('#') {
            continue;
        }
        if !simple(name, "", "._-") {
            return Err(format!("Invalid tool name: {name}").into());
        }
        let asset = words.get(1).copied().unwrap_or_default();
        // An immutable release asset, never a branch or tag archive whose
        // bytes can change under a digest that then no longer matches.
        if !is_release_asset(asset) {
            return Err(format!("Not a release asset path: {asset}").into());
        }
        let digest = words.get(2).copied().unwrap_or_default();
        if !is_hex(digest, 64) {
            return Err(format!("Invalid sha256 for {name}").into());
        }
        // Everything after the digest is the member, so a fifth word is
        // refused rather than silently dropped.
        let member = words
            .get(3..)
            .filter(|rest| !rest.is_empty())
            .map_or_else(|| name.to_string(), |rest| rest.join(" "));
        if !is_member(&member) {
            return Err(format!("Invalid archive member for {name}").into());
        }
        let file_name = asset.rsplit('/').next().unwrap_or(asset);
        let archive = downloads.join(file_name);
        // A reset connection or a run of server errors is transient: seven
        // retries, one second apart and doubling, wait two minutes in all.
        // Retrying every error also retries a missing asset, which only
        // delays the same refusal.
        Cmd::new(
            "curl --retry 7 --retry-all-errors --fail --silent --show-error --location --output",
        )
        .arg(&archive)
        .arg(format!("{RELEASES}/{asset}"))
        .run()?;
        // Verified before extraction: a tarball is parsed by tar, and bytes
        // nobody vouched for must not reach a parser.
        Cmd::new("sha256sum --check --strict")
            .stdin_bytes(format!("{digest}  {}\n", archive.display()).as_bytes())
            .run()?;
        let gzip = file_name
            .strip_suffix(".tar.gz")
            .or_else(|| file_name.strip_suffix(".tgz"));
        if gzip.is_some() {
            Cmd::new("tar -xzf")
                .arg(&archive)
                .arg("-C")
                .arg(&downloads)
                .arg(&member)
                .run()?;
        } else if file_name.strip_suffix(".tar.xz").is_some() {
            Cmd::new("tar -xJf")
                .arg(&archive)
                .arg("-C")
                .arg(&downloads)
                .arg(&member)
                .run()?;
        } else {
            fs::copy(&archive, downloads.join(&member))
                .map_err(|error| format!("cannot copy {name}: {error}"))?;
        }
        Cmd::new("install -m755")
            .arg(downloads.join(&member))
            .arg(bin.join(name))
            .run()?;
        installed += 1;
    }
    if installed == 0 {
        return Err("tools lists nothing to install".into());
    }
    fs::remove_dir_all(&downloads)
        .map_err(|error| format!("cannot remove {}: {error}", downloads.display()))?;
    add_to_path(&bin)
}

/// An immutable release asset path, `owner/repo/releases/download/tag/asset`,
/// with no traversal anywhere in it.
fn is_release_asset(value: &str) -> bool {
    let parts: Vec<&str> = value.split('/').collect();
    let label = |part: &str, extra: &str| {
        !part.is_empty()
            && part.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || "._-".contains(character)
                    || extra.contains(character)
            })
    };
    matches!(parts.as_slice(), [owner, repository, "releases", "download", tag, file]
        if label(owner, "") && label(repository, "") && label(tag, "%") && label(file, ""))
        && !value.contains("..")
}

/// A path inside an archive: relative, no traversal, no empty component.
fn is_member(value: &str) -> bool {
    !value.contains("..")
        && value.split('/').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        })
        && !value.starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::{is_member, is_release_asset, simple};

    #[test]
    fn only_release_assets_are_installed() {
        assert!(is_release_asset(
            "01mf02/jaq/releases/download/v3.1.1/jaq-x86_64-unknown-linux-gnu"
        ));
        assert!(is_release_asset(
            "rustsec/rustsec/releases/download/cargo-audit%2Fv0.22.2/cargo-audit.tgz"
        ));
        for bad in [
            "https://evil.example/jaq",
            "01mf02/jaq/archive/refs/tags/v3.1.1.tar.gz",
            "../jaq/releases/download/t/a",
        ] {
            assert!(!is_release_asset(bad), "{bad}");
        }
        assert!(
            is_member("cargo-vet-x86_64-unknown-linux-gnu/cargo-vet")
                && !is_member("../../bin/jaq")
        );
        assert!(simple("cargo-vet", "", "._-") && !simple("../jaq", "", "._-"));
    }
}
