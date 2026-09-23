//! Readers of the workflow and action YAML: whole documents, one step's body,
//! the tool rows a step installs, and `jaq` queries for anything else.

use super::repository::{root, tool};
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn yaml(path: PathBuf) -> Value {
    assert!(path.is_file(), "Missing file: {}", path.display());
    let output = tool("jaq")
        .args(["--from", "yaml", "."])
        .arg(path)
        .output()
        .expect("pinned jaq must be installed: just setup");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("jaq must emit valid JSON")
}

pub(crate) fn workflow(name: &str) -> Value {
    yaml(root().join(format!(".github/workflows/{name}.yml")))
}

/// An action shipped in this repository, under `.github/actions/`.
pub(crate) fn action(name: &str) -> Value {
    yaml(root().join(format!(".github/actions/{name}/action.yml")))
}

/// Where the workflows call this repository's own actions from. A reusable
/// workflow runs in the consumer's checkout, so this is the one way every
/// workflow gets the same gate binary.
const HELPER_ACTION_PREFIX: &str = "Orchestration-Maestro/rust-workflows/.github/actions/";

/// The `(name, pin)` of a `uses:` reference to one of this repository's own
/// actions, if it is one.
pub(crate) fn helper_action(reference: &str) -> Option<(String, String)> {
    let (name, pin) = reference
        .strip_prefix(HELPER_ACTION_PREFIX)?
        .split_once('@')?;
    Some((name.to_owned(), pin.to_owned()))
}

/// One line of a `rust-gate install-tools` table: the executable's name, its
/// GitHub release asset path, the digest, and the member inside an archive.
pub(crate) struct ToolRow {
    pub(crate) name: String,
    pub(crate) asset: String,
    pub(crate) digest: String,
    pub(crate) member: Option<String>,
}

/// The table a `rust-gate install-tools` step installs, parsed once for every
/// reader; empty for any other step. A malformed line is a defect in the
/// workflow.
pub(crate) fn tool_rows(step: &Value) -> Vec<ToolRow> {
    let is_install = step["run"]
        .as_str()
        .is_some_and(|run| run.trim() == "rust-gate install-tools");
    if !is_install {
        return Vec::new();
    }
    step["env"]["TOOLS"]
        .as_str()
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            assert!(
                words.len() == 3 || words.len() == 4,
                "malformed tool line: {line}"
            );
            ToolRow {
                name: words[0].to_owned(),
                asset: words[1].to_owned(),
                digest: words[2].to_owned(),
                member: words.get(3).map(|member| (*member).to_owned()),
            }
        })
        .collect()
}

/// Read one step's shell body straight out of the workflow it lives in.
pub(crate) fn step(name: &str, id: &str) -> String {
    let workflow = workflow(name);
    let found = workflow["jobs"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|job| job["steps"].as_array())
        .flatten()
        .find(|step| step["id"] == id)
        .unwrap_or_else(|| panic!("Missing executable boundary: {name}/{id}"))
        .clone();
    found["run"].as_str().unwrap().to_owned()
}

/// The `ci.yml` steps the example gate replays, in workflow order. The gate
/// and the North Star test read this one list.
pub(crate) const GATE_STEPS: [&str; 11] = [
    "quality",
    "complexity",
    "duplication",
    "coverage",
    "mutants",
    "msrv",
    "features",
    "unused",
    "build",
    "hardening",
    "stage",
];

/// One jaq query over a YAML or TOML file, as the repository tests read
/// metadata. jaq needs the format spelled out, and the file extension gives it.
pub(crate) fn query(path: &Path, expression: &str) -> String {
    let format = if path.extension().is_some_and(|kind| kind == "toml") {
        "toml"
    } else {
        "yaml"
    };
    let output = tool("jaq")
        .args(["--from", format, "-r", expression])
        .arg(path)
        .output()
        .expect("install the pinned jaq with the development toolbelt");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
