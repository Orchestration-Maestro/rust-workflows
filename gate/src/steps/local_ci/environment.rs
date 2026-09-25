//! The one environment every step of a local run starts with, built the way
//! a runner builds it: nothing of the developer's shell but what finds the
//! user, the toolchains and the network; the variables GitHub sets; each
//! step's `env:`; then whatever the steps before it wrote to `GITHUB_ENV` and
//! `GITHUB_PATH`. A `RUSTFLAGS` or `CARGO_TARGET_DIR` of the shell never
//! reaches a step, nor a `GIT_DIR` a hook was given.

use crate::runner::{Cmd, Failure, optional};
use std::collections::BTreeMap;
use std::env;
use std::env::consts;
use std::path::PathBuf;

/// The variables a local run takes from the developer's environment: where
/// the user, the toolchains and the temporary directory are, the proxy and
/// certificates of the network, the job count, what finds the platform's
/// linker (Windows' own directories and a Visual Studio prompt's, macOS's
/// developer directory and SDK), and two that stand for what only GitHub
/// knows: `LICENSE_ALLOWLIST`, the organization's variable, and
/// `PULL_REQUEST_TITLE`, the title the pull request will have.
pub(super) const INHERITED: &[&str] = &[
    "APPDATA",
    "CARGO_BUILD_JOBS",
    "ComSpec",
    "CommonProgramFiles",
    "CommonProgramFiles(x86)",
    "DEVELOPER_DIR",
    "HOME",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "INCLUDE",
    "LANG",
    "LC_ALL",
    "LIB",
    "LIBPATH",
    "LICENSE_ALLOWLIST",
    "LOCALAPPDATA",
    "LOGNAME",
    "NO_PROXY",
    "NUMBER_OF_PROCESSORS",
    "PATH",
    "PATHEXT",
    "PROCESSOR_ARCHITECTURE",
    "ProgramData",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "PULL_REQUEST_TITLE",
    "RUSTUP_HOME",
    "SDKROOT",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
    "SystemDrive",
    "SystemRoot",
    "TEMP",
    "TERM",
    "TMP",
    "TMPDIR",
    "TZ",
    "USER",
    "USERNAME",
    "USERPROFILE",
    "VCINSTALLDIR",
    "VSINSTALLDIR",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "https_proxy",
    "http_proxy",
    "no_proxy",
    "windir",
];

/// The environment of the next step.
pub(super) struct Environment {
    /// Every variable by name, `PATH` the one the developer's shell gave.
    variables: BTreeMap<String, String>,
    /// The directories put ahead of that `PATH`, the latest first.
    paths: Vec<PathBuf>,
}

impl Environment {
    /// What the developer's environment passes on.
    pub(super) fn inherited() -> Result<Self, Failure> {
        let mut variables = BTreeMap::new();
        for name in INHERITED {
            let value = optional(name)?;
            if !value.is_empty() {
                variables.insert((*name).to_owned(), value);
            }
        }
        Ok(Self {
            variables,
            paths: Vec::new(),
        })
    }

    /// Set `name` for every later step.
    pub(super) fn set(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_owned(), value.to_owned());
    }

    /// The value of `name`, empty when unset.
    pub(super) fn get(&self, name: &str) -> &str {
        self.variables.get(name).map_or("", String::as_str)
    }

    /// Put `directory` ahead of the PATH of every later step.
    pub(super) fn add_path(&mut self, directory: PathBuf) {
        self.paths.insert(0, directory);
    }

    /// Take what a step wrote: `exports`, its `GITHUB_ENV`, and `paths`, its
    /// `GITHUB_PATH`. The build target CI exports is Linux's; on another
    /// machine the step builds for the machine's own, and the change is
    /// returned to be said.
    pub(super) fn take(&mut self, exports: &str, paths: &str) -> Option<String> {
        for (name, value) in exported(exports) {
            self.set(name, value);
        }
        for line in paths.lines().filter(|line| !line.trim().is_empty()) {
            self.add_path(PathBuf::from(line));
        }
        let host = host_target(consts::OS, consts::ARCH, cfg!(target_env = "gnu"));
        let exported = self.get("CARGO_BUILD_TARGET").to_owned();
        if exported.is_empty() || exported == host {
            return None;
        }
        self.set("CARGO_BUILD_TARGET", &host);
        Some(format!("CARGO_BUILD_TARGET: {exported} in CI, {host} here"))
    }

    /// `words` run in this environment and nothing else.
    pub(super) fn command(&self, words: &str) -> Result<Cmd, Failure> {
        let inherited = env::split_paths(self.get("PATH"));
        let path = env::join_paths(self.paths.iter().cloned().chain(inherited))
            .map_err(|error| Failure::from(format!("PATH: {error}")))?;
        let mut command = Cmd::new(words).env_clear();
        for (name, value) in &self.variables {
            if name != "PATH" {
                command = command.env(name, value);
            }
        }
        // The trace the contract tests ask for follows every step.
        if let Some(trace) = env::var_os("RUST_GATE_TRACE") {
            command = command.env("RUST_GATE_TRACE", &trace);
        }
        Ok(command.env("PATH", &path))
    }
}

/// The `NAME=value` lines of a `GITHUB_ENV` file, the only form the gate
/// writes.
fn exported(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(name, _)| !name.is_empty() && !name.contains("<<"))
}

/// The target a Cargo on `os` and `arch` builds for by default, the GNU
/// environment named when a Windows build uses it.
fn host_target(os: &str, arch: &str, gnu: bool) -> String {
    match os {
        "macos" => format!("{arch}-apple-darwin"),
        "windows" if gnu => format!("{arch}-pc-windows-gnu"),
        "windows" => format!("{arch}-pc-windows-msvc"),
        _ => format!("{arch}-unknown-{os}-gnu"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Environment, exported, host_target};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn a_step_s_exports_and_paths_reach_every_later_step() {
        let mut environment = Environment {
            variables: BTreeMap::new(),
            paths: Vec::new(),
        };
        environment.set("PATH", "/usr/bin");
        let said = environment.take("PROJECT=/work/p\nCOVERAGE=90\n", "/first\n\n/second\n");
        assert!(said.is_none());
        assert_eq!(environment.get("PROJECT"), "/work/p");
        assert_eq!(environment.get("MISSING"), "");
        assert_eq!(
            environment.paths,
            [PathBuf::from("/second"), PathBuf::from("/first")]
        );
        assert!(environment.command("rust-gate quality").is_ok());
    }

    #[test]
    fn only_name_value_lines_are_exports() {
        let lines: Vec<(&str, &str)> = exported("A=1\nB=x=y\n=z\nC<<EOF\nplain\n").collect();
        assert_eq!(lines, [("A", "1"), ("B", "x=y")]);
    }

    #[test]
    fn the_build_target_is_the_machine_s_own() {
        assert_eq!(
            host_target("linux", "x86_64", true),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            host_target("macos", "aarch64", false),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            host_target("windows", "x86_64", false),
            "x86_64-pc-windows-msvc"
        );
        assert_eq!(
            host_target("windows", "x86_64", true),
            "x86_64-pc-windows-gnu"
        );
    }
}
