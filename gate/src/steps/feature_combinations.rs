//! `rust-gate features`: every feature the workspace declares must compile.
//! A default build proves one combination out of many, so a feature nobody
//! selects in CI can stop compiling and ship that way: the consumer who turns
//! it on is the one who finds out. `cargo hack --each-feature` builds each
//! feature on its own, the default set, none of them and all of them. It does
//! not enumerate the powerset or every optional feature with defaults enabled.
//! A workspace declaring no feature has nothing to check, and the
//! step says so rather than spending a compile to prove it.

use crate::runner::{Cmd, Job, Outcome, Step, output};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "features",
    summary: "Every declared feature compiles",
    inputs: &[],
    tools: &["cargo hack", "jaq"],
    reports: &["features.txt"],
    run,
}];

/// Every feature name the workspace members declare, one per line. `default`
/// is one of them: a crate whose only feature is `default` still has the
/// combination where it is off, which is exactly the one nobody builds. A
/// member without the field is an empty map rather than an error, and the
/// query stays out of `jaq -e`, whose exit status turns no output into a
/// failure when having no feature at all is the ordinary case.
const DECLARED: &str = ".workspace_members as $m | .packages[] |
  select(.id as $i | $m | index($i)) | (.features // {}) | keys[]";

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let declared = Cmd::new("jaq -r")
        .arg(DECLARED)
        .arg(job.temp.join("metadata.json"))
        .capture()?;
    let mut names: Vec<&str> = declared
        .lines()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    let report = job.report("features.txt")?;
    std::fs::write(&report, names.join("\n"))
        .map_err(|error| format!("cannot write {}: {error}", report.display()))?;
    if names.is_empty() {
        println!("SKIPPED: the workspace declares no feature");
        return output("applied", "false");
    }
    // `check` rather than `build`: a combination that fails does so in the
    // front end, and a consumer pays for one type check per feature instead of
    // one link. `--each-feature` is linear in the number of features, where a
    // powerset is exponential and would price this gate out of every run.
    Cmd::new("cargo hack check --workspace --locked --each-feature --no-dev-deps")
        .cwd(&job.project)
        .run()?;
    output("applied", "true")
}
