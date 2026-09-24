//! Real native Windows processes and temporary trees, without tool stand-ins.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// The current source snapshot, including uncommitted changes.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Run an actual native tool and keep both streams in a failure diagnostic.
pub(crate) fn checked(program: &str, args: &[&str], directory: &Path) -> Output {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program} {args:?}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Build the production binary once, with no dependency or executable cache import.
pub(crate) fn gate() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        checked(
            "cargo",
            &[
                "build",
                "--manifest-path",
                "gate/Cargo.toml",
                "--locked",
                "--offline",
            ],
            &root(),
        );
        root().join("gate/target/debug/rust-gate.exe")
    })
}

/// An exclusively created real Windows test directory, not an ACL stand-in.
pub(crate) fn temporary() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("native spaces-{}-{nanos}", std::process::id()));
    fs::create_dir(&path).unwrap();
    path
}

/// Run the registry boundary with no credentials, and no remote access.
pub(crate) fn registry(directory: &Path) -> Output {
    Command::new(gate())
        .arg("registry")
        .env("RUNNER_TEMP", directory)
        .env("GITHUB_ENV", directory.join("environment"))
        .output()
        .unwrap()
}
