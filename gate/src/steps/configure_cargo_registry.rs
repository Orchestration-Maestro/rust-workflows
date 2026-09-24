//! `rust-gate registry`: a private job-local Cargo home using crates.io directly.
//! No source replacement or registry credentials are configured.

use crate::checks::private_directories::private_directory;
use crate::runner::{Outcome, Step, export, input};
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "registry",
    summary: "Configure private Cargo state for direct crates.io access",
    inputs: &[],
    tools: if cfg!(windows) {
        &["powershell.exe"]
    } else {
        &[]
    },
    reports: &[],
    run,
}];

/// Run the step without reading or exporting registry credentials.
fn run() -> Outcome {
    let home = private_directory(&input("RUNNER_TEMP")?, "cargo-home")?;
    let config = "[registries.crates-io]\nprotocol = \"sparse\"\n";
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(home.join("config.toml"))
        .and_then(|mut file| file.write_all(config.as_bytes()))
        .map_err(|error| format!("cannot write config.toml: {error}"))?;
    export(&[
        ("CARGO_HOME", &home.display().to_string()),
        ("CARGO_REGISTRIES_CRATES_IO_PROTOCOL", "sparse"),
        (
            "CARGO_REGISTRIES_CRATES_IO_INDEX",
            "sparse+https://index.crates.io/",
        ),
    ])
}
