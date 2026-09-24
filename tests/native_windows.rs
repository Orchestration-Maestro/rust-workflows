//! Native Windows execution, distinct from the Linux workflow/ELF replay.

#![forbid(unsafe_code)]

mod native_runtime;

use crate::native_runtime::{checked, registry, root, temporary};
use std::fs;
use std::process::Command;

#[test]
fn native_gate_units_lints_and_documentation_execute() {
    assert_eq!(
        std::env::consts::OS,
        "windows",
        "this suite requires native Windows"
    );
    assert_eq!(std::env::consts::ARCH, "x86_64");
    let root = root();
    for manifest in ["gate/Cargo.toml", "tests/Cargo.toml"] {
        checked(
            "cargo",
            &["fmt", "--manifest-path", manifest, "--all", "--check"],
            &root,
        );
    }
    checked(
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            "gate/Cargo.toml",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        &root,
    );
    checked(
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            "tests/Cargo.toml",
            "--test",
            "native_windows",
            "--features",
            "native-windows",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        &root,
    );
    checked(
        "cargo",
        &[
            "test",
            "--manifest-path",
            "gate/Cargo.toml",
            "--locked",
            "--offline",
        ],
        &root,
    );
    let output = Command::new("cargo")
        .args([
            "doc",
            "--manifest-path",
            "gate/Cargo.toml",
            "--no-deps",
            "--locked",
            "--offline",
            "--document-private-items",
        ])
        .env("RUSTDOCFLAGS", "-D warnings -D missing_docs")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    checked(
        "cargo",
        &[
            "+1.85.0",
            "check",
            "--manifest-path",
            "gate/Cargo.toml",
            "--locked",
            "--offline",
        ],
        &root,
    );
}

#[test]
fn native_registry_creates_real_private_configuration() {
    assert_eq!(
        std::env::consts::OS,
        "windows",
        "this suite requires native Windows"
    );
    let directory = temporary();
    let output = registry(&directory);
    assert!(output.status.success(), "{output:?}");
    let exported = fs::read_to_string(directory.join("environment")).unwrap();
    let home = exported
        .lines()
        .find_map(|line| line.strip_prefix("CARGO_HOME="))
        .unwrap();
    let config = fs::read_to_string(std::path::Path::new(home).join("config.toml")).unwrap();
    assert_eq!(config, "[registries.crates-io]\nprotocol = \"sparse\"\n");
    assert!(!config.contains("Basic "));
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r"
$ErrorActionPreference = 'Stop'
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
foreach ($path in @($env:PRIVATE_HOME, (Join-Path $env:PRIVATE_HOME 'config.toml'))) {
    $acl = Get-Acl -LiteralPath $path
    $rules = @($acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference -ne $sid -or
        $rules[0].AccessControlType -ne 'Allow' -or
        $rules[0].FileSystemRights -ne 'FullControl') { exit 1 }
}
if (!(Get-Acl -LiteralPath $env:PRIVATE_HOME).AreAccessRulesProtected) { exit 2 }
",
        ])
        .env("PRIVATE_HOME", home)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_registry_uses_fresh_private_state_for_every_call() {
    assert_eq!(
        std::env::consts::OS,
        "windows",
        "this suite requires native Windows"
    );
    let directory = temporary();
    let first = registry(&directory);
    assert!(first.status.success(), "{first:?}");
    let before = fs::read_to_string(directory.join("environment")).unwrap();
    let second = registry(&directory);
    assert!(second.status.success(), "{second:?}");
    let after = fs::read_to_string(directory.join("environment")).unwrap();
    let homes: Vec<_> = after
        .lines()
        .filter(|line| line.starts_with("CARGO_HOME="))
        .collect();
    assert_eq!(homes.len(), 2);
    assert_ne!(homes[0], homes[1]);
    assert!(after.starts_with(&before));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_examples_run_real_debug_and_release_tests() {
    assert_eq!(
        std::env::consts::OS,
        "windows",
        "this suite requires native Windows"
    );
    for example in ["binary", "library", "workspace"] {
        let directory = root().join("examples").join(example);
        checked(
            "cargo",
            &["test", "--workspace", "--locked", "--offline"],
            &directory,
        );
        checked(
            "cargo",
            &["test", "--workspace", "--release", "--locked", "--offline"],
            &directory,
        );
        checked(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--offline",
                "--",
                "-D",
                "warnings",
                "-D",
                "unsafe_code",
            ],
            &directory,
        );
    }
}
