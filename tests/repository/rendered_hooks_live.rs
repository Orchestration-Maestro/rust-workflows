//! The rendered commit hooks against the network: in a fresh clone with only
//! prek, rustup and the operating system on the PATH, every hook installs its
//! own tool and passes over what the gate rendered.

use crate::harness::{Fixture, succeeds, tool, toolbelt_path};
use std::env;
use std::path::Path;

#[test]
fn the_rendered_hooks_run_in_a_fresh_clone_with_only_prek_and_rustup() {
    // Every hook downloads its tool, so this runs under CHECK_NETWORK=1. The
    // gate's own hooks install the release a caller pins, which does not
    // exist before the release; they are skipped here and run in `just check`.
    // `rust-gate init` writes the rule map and the guide the other hooks read;
    // the guide links to AGENTS.md, which every repository keeps.
    if !env::var("CHECK_NETWORK").is_ok_and(|value| value == "1") {
        return;
    }
    // A repository without Rust: the fixture's crate goes first.
    let fixture = Fixture::new();
    let repository = fixture.root.join("project");
    succeeds(&fixture.run_body(&format!(
        concat!(
            "cd project && rm -r Cargo.toml Cargo.lock src rust-toolchain.toml && git init -q ",
            "&& printf '# Probe\\n' > README.md && printf 'MIT\\n' > LICENSE && ",
            "printf '# Agents\\n' > AGENTS.md && ",
            "RUST_WORKFLOWS_PIN='{} v2.0.0' rust-gate init && git add -A",
        ),
        "a".repeat(40)
    )));
    let prek = toolbelt_path()
        .split(':')
        .map(Path::new)
        .find(|directory| directory.join("prek").is_file())
        .map(Path::to_path_buf)
        .unwrap();
    let cargo = env::var("CARGO_HOME").map_or_else(
        |_| Path::new(&env::var("HOME").unwrap()).join(".cargo/bin"),
        |home| Path::new(&home).join("bin"),
    );
    let output = tool("prek")
        .args(["run", "--all-files"])
        .env(
            "PATH",
            format!("{}:{}:/usr/bin:/bin", prek.display(), cargo.display()),
        )
        .env(
            "SKIP",
            "rust-gate-architecture,rust-gate-hygiene,rust-gate-rules,rust-gate-guide",
        )
        .current_dir(&repository)
        .output()
        .unwrap();
    succeeds(&output);
}
