//! The release a repository declares: a commit of rust-workflows and the
//! version it was released as, read from `RUST_WORKFLOWS_PIN` or from the
//! `uses:` lines of its workflows, or the version alone from its commit hooks;
//! and every call to rust-workflows moved to it, under the name the repository
//! answers to.

use crate::checks::workflow_home::{HOME, NAMES, ORGANIZATION};

/// What every call to the repository named `name`, a workflow or an action,
/// starts with.
fn calls(name: &str) -> String {
    format!("{ORGANIZATION}/{name}/.github/")
}

/// The earliest call to the home repository in `text`: where it starts and
/// how long its `calls` prefix is.
fn next_call(text: &str) -> Option<(usize, usize)> {
    NAMES
        .iter()
        .filter_map(|name| {
            let prefix = calls(name);
            text.find(&prefix).map(|start| (start, prefix.len()))
        })
        .min()
}

/// A commit of rust-workflows and the version it was released as.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Pin {
    /// The full commit hash.
    pub(super) commit: String,
    /// The version without its `v`: `2.0.0`.
    pub(super) version: String,
}

/// `text` with every call to rust-workflows, `<path>@<commit>  # v<version>`,
/// moved to `pin` and to the name [`HOME`]: `rust-gate sync` owns every such
/// pin, and Dependabot none. A call without a pin keeps its name.
pub(super) fn repinned(text: &str, pin: &Pin) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((start, prefix)) = next_call(rest) {
        let (before, call) = rest.split_at(start);
        out.push_str(before);
        let path_end = call
            .find('@')
            .filter(|at| !call[..*at].contains(char::is_whitespace));
        let old = path_end.and_then(|at| {
            let commit = call.get(at + 1..at + 41)?;
            let tail = call.get(at + 41..)?;
            let comment = tail.trim_start_matches(' ').strip_prefix('#')?;
            let version = comment.trim_start_matches(' ').strip_prefix('v')?;
            let digits = version
                .find(|character: char| !character.is_ascii_digit() && character != '.')
                .unwrap_or(version.len());
            let consumed = call.len() - version.len() + digits;
            commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
                .then_some((at, consumed))
        });
        let Some((at, consumed)) = old else {
            out.push_str(call.get(..prefix).unwrap_or_default());
            rest = call.get(prefix..).unwrap_or_default();
            continue;
        };
        out.push_str(&calls(HOME));
        out.push_str(call.get(prefix..=at).unwrap_or_default());
        out.push_str(&pin.commit);
        out.push_str("  # v");
        out.push_str(&pin.version);
        rest = call.get(consumed..).unwrap_or_default();
    }
    out.push_str(rest);
    out
}

/// Whether `version` is a release version without its `v`: `2.0.0`.
fn is_version(version: &str) -> bool {
    version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

/// The pin `text` names: a full commit hash, a space, and a version with or
/// without its `v`.
pub(super) fn parse_pin(text: &str) -> Option<Pin> {
    let (commit, version) = text.trim().split_once(' ')?;
    let version = version.trim().trim_start_matches('v');
    let hex = commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit());
    (hex && is_version(version)).then(|| Pin {
        commit: commit.to_owned(),
        version: version.to_owned(),
    })
}

/// Every pin the `uses:` lines of `workflow` name, in line order.
pub(super) fn workflow_pins(workflow: &str) -> Vec<Pin> {
    workflow
        .lines()
        .filter_map(|line| {
            let (start, prefix) = next_call(line)?;
            let rest = line.get(start + prefix..)?.strip_prefix("workflows/")?;
            let (_, pinned) = rest.split_once('@')?;
            let (commit, version) = pinned.split_once("# ")?;
            parse_pin(&format!("{} {}", commit.trim(), version.trim()))
        })
        .collect()
}

/// The pin the `uses:` lines of `caller` name, when every one names the same.
pub(super) fn caller_pin(caller: &str) -> Option<Pin> {
    let mut pins = workflow_pins(caller).into_iter();
    let first = pins.next()?;
    pins.all(|pin| pin == first).then_some(first)
}

/// The version of the gate the commit hooks in `hooks` install, from
/// `cli:https://github.com/<organization>/<name>:v<version>:rust-gate`.
pub(super) fn hook_version(hooks: &str) -> Option<String> {
    NAMES.iter().find_map(|name| {
        let prefix = format!("cli:https://github.com/{ORGANIZATION}/{name}:v");
        let start = hooks.find(&prefix)? + prefix.len();
        let rest = hooks.get(start..)?;
        let version = rest.get(..rest.find(":rust-gate")?)?;
        is_version(version).then(|| version.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::{Pin, caller_pin, hook_version, parse_pin, repinned, workflow_pins};

    /// The `uses:` value that calls `workflow` at `pin`, as sync writes it.
    fn reference(pin: &Pin, workflow: &str) -> String {
        format!(
            "Orchestration-Maestro/rust-workflows/.github/workflows/{workflow}@{}  # v{}",
            pin.commit, pin.version
        )
    }

    #[test]
    fn every_call_to_rust_workflows_moves_to_the_pin_and_nothing_else() {
        let pin = Pin {
            commit: "b".repeat(40),
            version: "2.1.0".to_owned(),
        };
        let old = "a".repeat(40);
        let text = format!(
            "jobs:\n  binaries:\n    uses: Orchestration-Maestro/rust-workflows/.github/\
             workflows/publish-binaries.yml@{old}  # v1.2.1\n  check:\n    uses: \
             actions/checkout@{old} # v7.0.1\n  gate:\n    uses: Orchestration-Maestro/\
             rust-workflows/.github/actions/gate@{old} # v1.2.1 and more\n"
        );
        let moved = repinned(&text, &pin);
        let commit = "b".repeat(40);
        assert!(moved.contains(&format!("publish-binaries.yml@{commit}  # v2.1.0\n")));
        assert!(moved.contains(&format!("actions/gate@{commit}  # v2.1.0 and more\n")));
        assert!(moved.contains(&format!("actions/checkout@{old} # v7.0.1")));
        assert_eq!(repinned(&moved, &pin), moved);
        assert_eq!(repinned("no call here\n", &pin), "no call here\n");
    }

    #[test]
    fn a_call_under_the_new_name_moves_to_the_rendered_name() {
        let pin = Pin {
            commit: "b".repeat(40),
            version: "2.5.0".to_owned(),
        };
        let old = "a".repeat(40);
        let text = format!(
            "    uses: Orchestration-Maestro/maestro-rust-workflows/.github/workflows/ci.yml@\
             {old}  # v2.4.0\n    uses: Orchestration-Maestro/rust-workflows/.github/actions/\
             gate@{old} # v2.4.0\n    uses: Orchestration-Maestro/maestro-rust-workflows/\
             .github/workflows/ci.yml@main\n"
        );
        let commit = "b".repeat(40);
        assert_eq!(
            repinned(&text, &pin),
            format!(
                "    uses: {}\n    uses: Orchestration-Maestro/rust-workflows/.github/actions/\
                 gate@{commit}  # v2.5.0\n    uses: Orchestration-Maestro/\
                 maestro-rust-workflows/.github/workflows/ci.yml@main\n",
                reference(&pin, "ci.yml")
            )
        );
    }

    #[test]
    fn a_pin_is_a_full_commit_and_a_version() {
        let commit = "a".repeat(40);
        let pin = parse_pin(&format!("{commit} v2.0.0")).unwrap();
        assert_eq!(pin.version, "2.0.0");
        assert_eq!(parse_pin(&format!("{commit} 2.0.0")), Some(pin));
        assert_eq!(parse_pin("abc v2.0.0"), None);
        assert_eq!(parse_pin(&format!("{commit} v2.0")), None);
    }

    #[test]
    fn the_caller_pin_is_the_one_every_uses_line_names() {
        let pin = Pin {
            commit: "b".repeat(40),
            version: "2.1.0".to_owned(),
        };
        let caller = format!(
            "jobs:\n  rust:\n    uses: {}\n  sarif:\n    uses: {}\n",
            reference(&pin, "ci.yml"),
            reference(&pin, "upload-sarif.yml")
        );
        assert_eq!(caller_pin(&caller), Some(pin));
        let mixed = caller.replacen(&"b".repeat(40), &"c".repeat(40), 1);
        assert_eq!(caller_pin(&mixed), None);
        assert_eq!(caller_pin("name: CI\n"), None);
        let renamed = caller.replace("/rust-workflows/", "/maestro-rust-workflows/");
        assert_eq!(caller_pin(&renamed), caller_pin(&caller));
        let both = caller.replacen("/rust-workflows/", "/maestro-rust-workflows/", 1);
        assert_eq!(caller_pin(&both), caller_pin(&caller));
        assert!(caller_pin(&caller).is_some());
    }

    #[test]
    fn every_pinned_workflow_call_is_read_in_line_order() {
        let old = Pin {
            commit: "a".repeat(40),
            version: "2.0.0".to_owned(),
        };
        let new = Pin {
            commit: "b".repeat(40),
            version: "2.1.0".to_owned(),
        };
        let release = format!(
            "jobs:\n  binaries:\n    uses: {}\n  attest:\n    uses: {}\n  gate:\n    uses: \
             Orchestration-Maestro/rust-workflows/.github/workflows/ci.yml@main\n",
            reference(&new, "publish-binaries.yml"),
            reference(&old, "attest-binaries.yml")
        );
        assert_eq!(workflow_pins(&release), [new, old]);
        assert!(workflow_pins("on: push\n").is_empty());
    }

    #[test]
    fn the_commit_hooks_name_the_version_of_the_gate() {
        let hooks = "          - \"cli:https://github.com/Orchestration-Maestro/rust-workflows:\
                     v3.0.1:rust-gate\"\n";
        assert_eq!(hook_version(hooks).as_deref(), Some("3.0.1"));
        let renamed = hooks.replace("/rust-workflows:", "/maestro-rust-workflows:");
        assert_eq!(hook_version(&renamed).as_deref(), Some("3.0.1"));
        assert_eq!(hook_version(&hooks.replace("v3.0.1", "vmain")), None);
        assert_eq!(hook_version("repos: []\n"), None);
    }
}
