//! `rust-gate setup`: the organization's toolbelt installed from the pins the
//! gate was built with, linked into one directory whose path it prints, the
//! commit hooks wired in, and a download of mise refused unless its digest is
//! the one `scripts/bootstrap.sh` pins.

use crate::harness::{Fixture, refused, root, succeeds, write_executable};
use std::fs;

/// What `scripts/bootstrap.sh` pins mise at: its version and the SHA-256 of
/// its Linux x64 archive.
fn bootstrap_pin() -> (String, String) {
    let bootstrap = fs::read_to_string(root().join("scripts/bootstrap.sh")).unwrap();
    let pinned = |key: &str| {
        bootstrap
            .lines()
            .find_map(|line| line.strip_prefix(&format!("readonly {key}=")))
            .unwrap()
            .to_owned()
    };
    (pinned("MISE_VERSION"), pinned("MISE_SHA256"))
}

#[test]
fn setup_installs_the_locked_toolbelt_links_it_and_prints_its_path() {
    let mut fixture = Fixture::new();
    let installed = fixture.root.join("installs/typos/1.50.2");
    fs::create_dir_all(&installed).unwrap();
    write_executable(&installed.join("typos"), "#!/bin/bash\necho typos\n");
    let tools = fixture.cached_mise(&installed.display().to_string());
    fixture.stub("prek", "true");
    fixture.stub("curl", "exit 1");
    fs::write(
        fixture.root.join("project/.pre-commit-config.yaml"),
        "repos: []\n",
    )
    .unwrap();
    let output = fixture.run_body("cd project && rust-gate setup");
    succeeds(&output);
    let calls = fixture.calls();
    assert!(
        calls.contains("mise\ninstall --locked\nmise\nbin-paths\nprek\ninstall\n"),
        "{calls}"
    );
    // The mise already fetched is reused, never downloaded again.
    assert!(!calls.contains("curl"), "{calls}");
    // mise reads the gate's pins alone, in a store of their own.
    let version = fs::read_to_string(root().join("version.txt")).unwrap();
    let store = tools.join(version.trim());
    for file in ["mise.toml", "mise.lock"] {
        assert_eq!(
            fs::read_to_string(store.join(file)).unwrap(),
            fs::read_to_string(root().join(file)).unwrap(),
            "{file}"
        );
    }
    let trace = fixture.trace();
    for setting in [
        "MISE_DATA_DIR",
        "MISE_CONFIG_DIR",
        "MISE_GLOBAL_CONFIG_FILE",
        "MISE_TRUSTED_CONFIG_PATHS",
        "MISE_CEILING_PATHS",
    ] {
        assert!(trace.contains(&format!("{setting}=")), "{setting}: {trace}");
    }
    let bin = tools.join("bin");
    assert_eq!(
        fs::read_link(bin.join("typos")).unwrap(),
        installed.join("typos")
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "setup: the toolbelt of rust-gate {} is in {}. Add it to your PATH, in your \
             shell's profile:\n\n  export PATH=\"{}:$PATH\"\n",
            version.trim(),
            bin.display(),
            bin.display()
        )),
        "{stdout}"
    );
    // In a GitHub Actions job, every later step finds the toolbelt.
    assert_eq!(
        fs::read_to_string(fixture.root.join("path")).unwrap(),
        format!("{}\n", bin.display())
    );
    // Again: the same links, the same line.
    succeeds(&fixture.run_body("cd project && rust-gate setup"));
    assert!(bin.join("typos").exists());
}

#[test]
fn setup_refuses_a_mise_download_whose_digest_is_not_the_pinned_one() {
    let mut fixture = Fixture::new();
    fixture.set(
        "XDG_CACHE_HOME",
        &fixture.root.join("cache").display().to_string(),
    );
    fixture.stub(
        "curl",
        r#"out=""; while [[ $# -gt 0 ]]; do [[ "$1" == --output ]] && out=$2; shift; done
printf 'not mise' > "$out""#,
    );
    fixture.stub("tar", "exit 1");
    let (version, sha256) = bootstrap_pin();
    let archive = format!("mise-{version}-linux-x64.tar.gz");
    refused(
        &fixture.run_body("rust-gate setup"),
        &format!(
            "toolbelt: {archive} has SHA-256 \
             4865026793c5017a685bbb2de39e53ee274fe320fb14ef24de3248bc340665e1, not the pinned \
             {sha256}; nothing was unpacked"
        ),
    );
    let calls = fixture.calls();
    assert!(
        calls.contains(&format!(
            "https://github.com/jdx/mise/releases/download/{version}/{archive}"
        )),
        "{calls}"
    );
    assert!(!calls.contains("tar\n"), "{calls}");
    // Nothing half-fetched is left for the next run to trust.
    let tools = fixture.root.join("cache/maestro/tools");
    assert!(!tools.join("mise").exists());
    assert!(fs::read_dir(&tools).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("mise-")
    }));
}
