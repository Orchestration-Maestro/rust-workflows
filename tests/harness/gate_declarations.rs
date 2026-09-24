//! The gate built once per test process, and what it declares about its own
//! steps through `rust-gate describe`.

use super::repository::{root, tool};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// The gate binary the step bodies invoke, built once per test process from
/// `gate/` with the compiler that crate pins; its directory goes on the fixture
/// PATH ahead of the toolbelt, so `rust-gate <id>` in a body runs this build.
pub(crate) fn gate_bin() -> &'static Path {
    static GATE: OnceLock<PathBuf> = OnceLock::new();
    GATE.get_or_init(|| {
        let crate_dir = root().join("gate");
        let status = tool("cargo")
            .args(["build", "--release", "--locked", "--offline", "--quiet"])
            .current_dir(&crate_dir)
            .status()
            .expect("cargo must be installed: just setup");
        assert!(status.success(), "the gate crate must build");
        crate_dir.join("target/release")
    })
}

/// One step as `rust-gate describe` renders it: its workflow and id, what it
/// does, and the inputs, tools and reports it declares.
pub(crate) struct Described {
    pub(crate) workflow: String,
    pub(crate) id: String,
    pub(crate) summary: String,
    pub(crate) inputs: Vec<String>,
    pub(crate) tools: Vec<String>,
    pub(crate) reports: Vec<String>,
}

/// The text `rust-gate describe` prints: the steps document.
pub(crate) fn describe_text() -> String {
    let output = Command::new(gate_bin().join("rust-gate"))
        .arg("describe")
        .output()
        .expect("the gate binary must run");
    assert!(output.status.success(), "rust-gate describe failed");
    String::from_utf8(output.stdout).unwrap()
}

/// Every step the gate describes, in registry order.
pub(crate) fn described() -> Vec<Described> {
    let names = |cell: &str| -> Vec<String> {
        cell.split(", ")
            .map(|name| name.trim_matches('`').to_owned())
            .filter(|name| !name.is_empty())
            .collect()
    };
    let mut workflow = String::new();
    let mut steps = Vec::new();
    for line in describe_text().lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            heading.clone_into(&mut workflow);
        } else if let Some(row) = line.strip_prefix("| `") {
            let cells: Vec<&str> = row.trim_end_matches(" |").split(" | ").collect();
            assert_eq!(cells.len(), 5, "unexpected describe row: {line}");
            steps.push(Described {
                workflow: workflow.clone(),
                id: cells[0].trim_end_matches('`').to_owned(),
                summary: cells[1].to_owned(),
                inputs: names(cells[2]),
                tools: names(cells[3]),
                reports: names(cells[4]),
            });
        }
    }
    assert!(steps.len() > 30, "only {} steps described", steps.len());
    steps
}

/// The described step a `rust-gate` body runs: one word names a `ci.yml`
/// step, a shared command or a local command, a flag after it a local
/// command, and two words a workflow and its step.
pub(crate) fn described_step<'a>(steps: &'a [Described], body: &str) -> Option<&'a Described> {
    let mut words = body.trim().strip_prefix("rust-gate ")?.split_whitespace();
    let (first, second) = (words.next()?, words.next());
    steps.iter().find(|step| match second {
        Some(flag) if flag.starts_with("--") => {
            step.workflow == "local" && step.id == format!("{first} {flag}")
        }
        Some(id) => step.workflow == first && step.id == id,
        None => step.id == first && matches!(step.workflow.as_str(), "ci" | "shared" | "local"),
    })
}
