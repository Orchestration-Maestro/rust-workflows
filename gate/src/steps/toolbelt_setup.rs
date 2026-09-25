//! `rust-gate setup`: the organization's toolbelt on a developer's machine,
//! exactly the tools and versions CI runs, from the pins the gate was built
//! with, and in a repository with commit hooks, the hooks wired into git. It
//! prints the one PATH entry to add, and in a GitHub Actions job adds it to
//! the PATH of every later step; running it again is fast and changes
//! nothing that is already in place.

use crate::checks::toolbelt::{install_toolbelt, toolbelt_path, toolbelt_version};
use crate::runner::{Cmd, Outcome, Step, add_to_path, optional};
use std::env::consts;
use std::path::Path;

/// What this command declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "local",
    id: "setup",
    summary: "The organization's pinned toolbelt for this user, and the repository's commit hooks",
    inputs: &[
        "HOME",
        "LOCALAPPDATA",
        "PATH",
        "SystemRoot",
        "XDG_CACHE_HOME",
    ],
    // On Windows, mise's download directory is made private through
    // PowerShell, as every private directory there is.
    tools: if cfg!(windows) {
        &[
            "cargo install",
            "curl",
            "mise",
            "powershell.exe",
            "prek",
            "tar",
        ]
    } else {
        &["cargo install", "curl", "mise", "prek", "tar"]
    },
    reports: &[],
    run,
}];

/// Run the command.
fn run() -> Outcome {
    let toolbelt = install_toolbelt()?;
    // The hooks run from the toolbelt's prek; git calls it on every commit.
    if Path::new(".pre-commit-config.yaml").is_file() {
        Cmd::new("prek install")
            .env("PATH", &toolbelt_path(&toolbelt.bin)?)
            .run()?;
    }
    if !optional("GITHUB_PATH")?.is_empty() {
        add_to_path(&toolbelt.bin)?;
    }
    for skipped in &toolbelt.skipped {
        println!("setup: not installed here: {skipped}");
    }
    let bin = toolbelt.bin.display();
    let line = if consts::OS == "windows" {
        format!("$env:Path = \"{bin};$env:Path\"")
    } else {
        format!("export PATH=\"{bin}:$PATH\"")
    };
    println!(
        "setup: the toolbelt of rust-gate {} is in {bin}. Add it to your PATH, in your \
         shell's profile:\n\n  {line}",
        toolbelt_version()
    );
    Ok(())
}
