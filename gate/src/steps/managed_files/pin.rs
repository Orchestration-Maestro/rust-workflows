//! The release a caller pins: a commit of rust-workflows and the version it
//! was released as, read from the caller's `uses:` lines or from
//! `RUST_WORKFLOWS_PIN`, and every other call to rust-workflows moved to it.

/// Where every reusable workflow of the organization lives.
const WORKFLOWS: &str = "Orchestration-Maestro/rust-workflows/.github/workflows/";

/// What every call to rust-workflows, a workflow or an action, starts with.
const CALLS: &str = "Orchestration-Maestro/rust-workflows/.github/";

/// A commit of rust-workflows and the version it was released as.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Pin {
    /// The full commit hash.
    pub(super) commit: String,
    /// The version without its `v`: `2.0.0`.
    pub(super) version: String,
}

impl Pin {
    /// The `uses:` value that calls `workflow` at this pin, with the version
    /// as the comment Dependabot and a reader both read.
    pub(super) fn reference(&self, workflow: &str) -> String {
        format!("{WORKFLOWS}{workflow}@{}  # v{}", self.commit, self.version)
    }
}

/// `text` with every call to rust-workflows, `<path>@<commit>  # v<version>`,
/// moved to `pin`: `rust-gate sync` owns every such pin, and Dependabot none.
pub(super) fn repinned(text: &str, pin: &Pin) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(CALLS) {
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
            out.push_str(CALLS);
            rest = call.get(CALLS.len()..).unwrap_or_default();
            continue;
        };
        out.push_str(call.get(..=at).unwrap_or_default());
        out.push_str(&pin.commit);
        out.push_str("  # v");
        out.push_str(&pin.version);
        rest = call.get(consumed..).unwrap_or_default();
    }
    out.push_str(rest);
    out
}

/// The pin `text` names: a full commit hash, a space, and a version with or
/// without its `v`.
pub(super) fn parse_pin(text: &str) -> Option<Pin> {
    let (commit, version) = text.trim().split_once(' ')?;
    let version = version.trim().trim_start_matches('v');
    let hex = commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit());
    let numbered = version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    (hex && numbered).then(|| Pin {
        commit: commit.to_owned(),
        version: version.to_owned(),
    })
}

/// The pin the `uses:` lines of `caller` name, when every one names the same.
pub(super) fn caller_pin(caller: &str) -> Option<Pin> {
    let mut pins = caller.lines().filter_map(|line| {
        let rest = line.split_once(WORKFLOWS)?.1;
        let (_, pinned) = rest.split_once('@')?;
        let (commit, version) = pinned.split_once("# ")?;
        parse_pin(&format!("{} {}", commit.trim(), version.trim()))
    });
    let first = pins.next()?;
    pins.all(|pin| pin == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::{Pin, caller_pin, parse_pin, repinned};

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
            pin.reference("ci.yml"),
            pin.reference("upload-sarif.yml")
        );
        assert_eq!(caller_pin(&caller), Some(pin));
        let mixed = caller.replacen(&"b".repeat(40), &"c".repeat(40), 1);
        assert_eq!(caller_pin(&mixed), None);
        assert_eq!(caller_pin("name: CI\n"), None);
    }
}
