//! The toolbelt on every platform `rust-gate setup` installs it on: each pin
//! locked to a release asset of that platform with its checksum, or a gap
//! `mise.toml` declares with its reason, mise told to leave it out there, and
//! nothing else; and CI's checks run on it before a push on each platform.

use crate::harness::{capture, query, root, tool, workflow};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// Every platform the toolbelt is pinned for, as mise names it.
const PLATFORMS: [&str; 5] = [
    "linux-x64",
    "linux-arm64",
    "macos-x64",
    "macos-arm64",
    "windows-x64",
];

/// Words an asset name for `platform` never holds: another system's or
/// another architecture's.
fn foreign_words(platform: &str) -> Vec<&'static str> {
    let mut words = match platform.split('-').next() {
        Some("linux") => vec!["darwin", "apple", "macos", "osx", "windows", ".exe"],
        Some("macos") => vec!["linux", "windows", ".exe"],
        _ => vec!["linux", "darwin", "apple", "macos", "osx"],
    };
    words.extend(["freebsd", "android"]);
    if platform.ends_with("x64") {
        words.extend(["aarch64", "arm64"]);
    } else {
        words.extend(["x86_64", "amd64", "x64"]);
    }
    words
}

/// The platforms one entry of a tool's `os` field allows.
fn allowed(entry: &str) -> Vec<&'static str> {
    match entry {
        "linux" => vec!["linux-x64", "linux-arm64"],
        "linux/x64" => vec!["linux-x64"],
        "linux/arm64" => vec!["linux-arm64"],
        "macos" => vec!["macos-x64", "macos-arm64"],
        "macos/x64" => vec!["macos-x64"],
        "macos/arm64" => vec!["macos-arm64"],
        "windows" => vec!["windows-x64"],
        other => panic!("mise.toml restricts a tool to {other}, which no platform is"),
    }
}

/// Every gap the `# source:` and `# skip:` lines of `toml` declare: each tool
/// and platform, and which kind.
fn declared_gaps(toml: &str) -> BTreeMap<(String, String), String> {
    let mut gaps = BTreeMap::new();
    for (kind, line) in toml.lines().filter_map(|line| {
        ["source", "skip"]
            .into_iter()
            .find_map(|kind| Some((kind, line.strip_prefix(&format!("# {kind}: "))?)))
    }) {
        let fields: Vec<&str> = line.splitn(3, '|').map(str::trim).collect();
        let [tool, platforms, reason] = fields.as_slice() else {
            panic!("a gap line needs a tool, its platforms and a reason: {line}");
        };
        assert!(reason.len() > 20, "{tool}: the gap names no reason");
        // The one accepted skip: a Valgrind tool, where Valgrind does not run.
        assert!(
            kind == "source" || *tool == "gungraun-runner",
            "{tool}: only gungraun-runner may be skipped; build the rest from source"
        );
        for platform in platforms.split_whitespace() {
            assert!(PLATFORMS.contains(&platform), "{tool}: {platform}");
            gaps.insert(((*tool).to_owned(), platform.to_owned()), kind.to_owned());
        }
    }
    gaps
}

/// Every download `mise.lock` records: each tool and platform, and the URL
/// and checksum, `-` for one it lacks.
fn locked_downloads() -> BTreeMap<(String, String), (String, String)> {
    let locked = capture(tool("jaq").current_dir(root()).args([
        "-r",
        "--from",
        "toml",
        concat!(
            r#".tools | to_entries[] | .key as $tool | .value[] | to_entries[]"#,
            r#" | select(.key | startswith("platforms."))"#,
            r#" | "\($tool) \(.key | ltrimstr("platforms."))"#,
            r#" \(.value.url // "-") \(.value.checksum // "-")""#,
        ),
        "mise.lock",
    ]));
    locked
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let [name, platform, url, checksum] = words.as_slice() else {
                return None;
            };
            Some((
                ((*name).to_owned(), (*platform).to_owned()),
                ((*url).to_owned(), (*checksum).to_owned()),
            ))
        })
        .collect()
}

/// Refuse a download of `name` for `platform` that is no release asset, has
/// no checksum, or is built for another platform.
fn assert_platform_download(name: &str, platform: &str, url: &str, checksum: &str) {
    assert!(
        url.starts_with("https://github.com/") && url.contains("/releases/download/"),
        "{name} {platform}: {url} is not a release asset"
    );
    assert!(
        checksum.starts_with("sha256:") || checksum.starts_with("blake3:"),
        "{name} {platform}: no checksum recorded for {url}"
    );
    let asset = url.rsplit('/').next().unwrap_or_default().to_lowercase();
    for word in foreign_words(platform) {
        assert!(
            !asset.contains(word),
            "{name} {platform}: {asset} is built for another platform"
        );
    }
}

#[test]
fn every_pin_is_locked_with_a_checksum_on_every_platform_or_declares_its_gap() {
    // `rust-gate setup` installs the same tools at the same versions on every
    // platform. Where a tool publishes no build, mise.toml says so above its
    // pin: `# source:` has setup build it from crates.io, `# skip:` leaves out
    // the one tool that cannot run there at all. Any other hole in the lock
    // would install a tool unverified, or not at all, on some machine.
    let gaps = declared_gaps(&fs::read_to_string(root().join("mise.toml")).unwrap());
    let entries = locked_downloads();
    let restrictions = query(
        &root().join("mise.toml"),
        concat!(
            r#".tools | to_entries[] | "\(.key) \(if (.value | type) == "object""#,
            r#" then (.value.os // [] | join(" ")) else "" end)""#,
        ),
    );
    let mut checked = 0;
    for line in restrictions.lines() {
        let (name, os) = line.split_once(' ').unwrap_or((line, ""));
        let runs: BTreeSet<&str> = if os.is_empty() {
            PLATFORMS.into_iter().collect()
        } else {
            os.split_whitespace().flat_map(allowed).collect()
        };
        for platform in PLATFORMS {
            let key = (name.to_owned(), platform.to_owned());
            let gap = gaps.contains_key(&key);
            // A gap and only a gap is what the os field leaves out, and
            // mise.lock records no download there.
            assert_eq!(runs.contains(platform), !gap, "{name} {platform}");
            assert_eq!(entries.contains_key(&key), !gap, "{name} {platform}");
            if let Some((url, checksum)) = entries.get(&key) {
                assert_platform_download(name, platform, url, checksum);
                checked += 1;
            }
        }
    }
    assert!(checked > 120, "only {checked} downloads checked");
}

#[test]
fn every_platform_runs_ci_s_checks_before_a_push_and_stops_a_broken_change() {
    // `rust-gate ci --local` runs the same on every platform the toolbelt is
    // pinned for: the workflow that proves the toolbelt runs it on a runner of
    // each, over a consumer whose default branch passes and whose failing test
    // stops at the step ci.yml names for it.
    let platforms = workflow("toolbelt-platforms");
    let job = &platforms["jobs"]["toolbelt"];
    let runners: Vec<&str> = job["strategy"]["matrix"]["include"]
        .as_array()
        .unwrap()
        .iter()
        .map(|leg| leg["runner"].as_str().unwrap())
        .collect();
    assert_eq!(
        runners,
        [
            "ubuntu-24.04",
            "ubuntu-24.04-arm",
            "macos-15-intel",
            "macos-15",
            "windows-2025"
        ]
    );
    let body = job["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|step| step["run"].as_str())
        .find(|run| run.contains("rust-gate ci --local"))
        .unwrap();
    assert_eq!(body.matches("rust-gate ci --local").count(), 2, "{body}");
    let quality = workflow("ci")["jobs"]["checks"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["id"] == "quality")
        .map(|step| step["name"].as_str().unwrap().to_owned())
        .unwrap();
    assert!(
        body.contains(&format!("'^ci --local: one step failed: {quality}'")),
        "{body}"
    );
}
