//! `rust-gate performance`: PRF-001, the instruction counts of the benchmarks
//! `maestro-quality.toml` names under `[performance] benches`, the base branch
//! against the pull request in one job. gungraun counts instructions under
//! Valgrind, which the runner's load cannot move, so a rise past 5 % is the
//! change's own. A regression a repository accepts is a PRF-001 exception
//! naming the bench; its counts are still reported. The runner must match
//! the gungraun the benchmarks link, so the organization holds one version.

use crate::checks::findings::relative;
use crate::checks::quality_config::read_config;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, input, optional, output, tee_line};
use std::path::{Path, PathBuf};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "performance",
    summary: "Instruction counts of the declared benchmarks against the base",
    inputs: &["GITHUB_BASE_REF", "GITHUB_WORKSPACE"],
    tools: &["cargo bench", "git", "jaq", "sudo"],
    reports: &["performance.txt"],
    run,
}];

/// The gungraun the organization's runner is built from.
const GUNGRAUN: &str = "0.19.4";

/// The Valgrind the runner image's archive holds, pinned.
const VALGRIND: &str = "valgrind=1:3.22.0-0ubuntu3";

/// The largest rise in instructions a pull request may bring.
const BUDGET: &str = "ir=5%";

/// The gungraun version the lockfile resolves.
const LOCKED_GUNGRAUN: &str = ".package[] | select(.name == \"gungraun\") | .version";

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("performance.txt")?;
    let workspace = PathBuf::from(input("GITHUB_WORKSPACE")?);
    let config = read_config(&workspace)?;
    let reason = if config.benches.is_empty() {
        "NOT APPLICABLE: maestro-quality.toml names no [performance] benches"
    } else if optional("GITHUB_BASE_REF")?.is_empty() {
        "NOT APPLICABLE: a push has no base to compare with"
    } else {
        ""
    };
    if !reason.is_empty() {
        tee_line(reason, &report, false)?;
        return output("applied", "false");
    }
    output("applied", "true")?;
    let locked = Cmd::new("jaq --from toml -r")
        .arg(LOCKED_GUNGRAUN)
        .arg(job.project.join("Cargo.lock"))
        .capture()?;
    if locked.trim() != GUNGRAUN {
        return Err(Failure::from(format!(
            "performance: Cargo.lock resolves gungraun {}; the organization's runner is \
             {GUNGRAUN} (PRF-001)",
            locked.trim()
        )));
    }
    Cmd::new("sudo apt-get install -y --no-install-recommends")
        .arg(VALGRIND)
        .run()?;
    let base = job.temp.join("performance-base");
    Cmd::new("git -C")
        .arg(&workspace)
        .args(["worktree", "add", "--detach"])
        .arg(&base)
        .arg("HEAD^1")
        .run()?;
    let home = job.temp.join("gungraun");
    let project = relative(&workspace, &job.project);
    let excused: Vec<&str> = config
        .exceptions
        .iter()
        .filter(|exception| exception.rule == "PRF-001")
        .map(|exception| exception.path.as_str())
        .collect();
    let mut regressed = Vec::new();
    for bench in &config.benches {
        tee_line(&format!("# {bench}: the base"), &report, true)?;
        bench_run(&base.join(&project), bench, &home, "--save-baseline=base").tee(&report, true)?;
        tee_line(&format!("# {bench}: this pull request"), &report, true)?;
        let limited = !excused.contains(&bench.as_str());
        let mut head = bench_run(&job.project, bench, &home, "--baseline=base");
        if limited {
            head = head.arg(format!("--callgrind-limits={BUDGET}"));
        } else {
            tee_line(&format!("EXCUSED PRF-001 {bench}"), &report, true)?;
        }
        if let Err(failure) = head.tee(&report, true) {
            if failure.code != 3 {
                return Err(failure);
            }
            regressed.push(bench.clone());
        }
    }
    if regressed.is_empty() {
        return Ok(());
    }
    Err(Failure::from(format!(
        "performance: {} runs more instructions than the base, over 5 % (PRF-001); \
         performance.txt holds the counts",
        regressed.join(", ")
    )))
}

/// The run of `bench` in `project`, its results under `home`, with `mode`.
fn bench_run(project: &Path, bench: &str, home: &Path, mode: &str) -> Cmd {
    Cmd::new("cargo bench --locked --bench")
        .arg(bench)
        .arg("--")
        .arg(mode)
        .arg("--home")
        .arg(home)
        .cwd(project)
}
