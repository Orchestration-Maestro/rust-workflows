//! `rust-gate complexity`: the size of the consumer's functions and files,
//! reported and never enforced. Clippy measures cognitive complexity, function
//! length and parameter count against the consumer's own thresholds when a
//! `clippy.toml` is committed and against this workflow's otherwise; files are
//! measured by their lines of code, doc comments not counted. The step exits
//! zero whatever it finds: the report is information for the consumer.

use crate::checks::checkout_paths::rust_sources;
use crate::runner::{Cmd, Failure, Job, Outcome, Step, summary, write};
use std::cmp::Reverse;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// What this step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "complexity",
    summary: "Function and file sizes, never blocking",
    inputs: &[],
    tools: &["cargo clippy", "jaq"],
    reports: &["complexity.json", "complexity.txt"],
    run,
}];

/// The thresholds applied when the consumer committed no `clippy.toml`.
const DEFAULT_THRESHOLDS: &str = "cognitive-complexity-threshold = 15\n\
    too-many-lines-threshold = 100\n\
    too-many-arguments-threshold = 5\n";

/// Files longer than this many lines of code are listed.
const FILE_LIMIT: usize = 300;

/// The three size lints, read out of Clippy's JSON messages as one line per
/// finding: lint, value, limit, location.
const FINDINGS: &str = "
  select(.reason == \"compiler-message\") | .message
  | select(.code != null and (.code.code == \"clippy::cognitive_complexity\"
      or .code.code == \"clippy::too_many_lines\"
      or .code.code == \"clippy::too_many_arguments\"))
  | (.message | capture(\"\\\\((?<value>[0-9]+)/(?<limit>[0-9]+)\\\\)\")) as $m
  | [.code.code, $m.value, $m.limit,
     ((.spans[] | select(.is_primary)) | \"\\(.file_name):\\(.line_start)\")]
  | @tsv";

/// One function over a threshold: the lint, the value, the limit, where.
type Finding = (String, u64, u64, String);

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let report = job.report("complexity.txt")?;
    let data = job.report("complexity.json")?;
    let (thresholds, config_dir) = thresholds_for(&job)?;
    let messages = job.temp.join("complexity-messages.jsonl");
    let mut clippy =
        Cmd::new("cargo clippy --workspace --all-targets --locked --message-format=json --")
            .args(["-W", "clippy::cognitive_complexity"])
            .args([
                "-W",
                "clippy::too_many_lines",
                "-W",
                "clippy::too_many_arguments",
            ])
            .cwd(&job.project);
    if let Some(directory) = &config_dir {
        clippy = clippy.env("CLIPPY_CONF_DIR", &directory.display().to_string());
    }
    if clippy.stdout_to(&messages).is_err() {
        write(
            &report,
            b"NOT MEASURED: cargo clippy failed; see the log\n",
            false,
        )?;
        return write(&data, b"{\"measured\":false}\n", false);
    }
    let mut functions = findings(&messages)?;
    functions.sort_by_key(|finding| Reverse(finding.1));
    let mut files: Vec<(String, usize)> = rust_sources(&job.project)?
        .into_iter()
        .filter_map(|path| {
            let lines = code_lines(&path);
            (lines > FILE_LIMIT).then(|| {
                let relative = path.strip_prefix(&job.project).unwrap_or(&path);
                (relative.display().to_string(), lines)
            })
        })
        .collect();
    files.sort_by_key(|file| Reverse(file.1));
    let mut text = format!(
        "# rust-gate complexity, informational: thresholds from {thresholds}\n\
         # functions over the thresholds: {}; files over {FILE_LIMIT} lines of code: {}\n",
        functions.len(),
        files.len()
    );
    for (lint, value, limit, location) in &functions {
        let _ = writeln!(text, "{lint}\t{value}\t{limit}\t{location}");
    }
    for (file, lines) in &files {
        let _ = writeln!(text, "file\t{lines}\t{FILE_LIMIT}\t{file}");
    }
    write(&report, text.as_bytes(), false)?;
    write(
        &data,
        format!(
            "{{\"measured\":true,\"thresholds\":\"{thresholds}\",\
             \"functions_over\":{},\"files_over\":{}}}\n",
            functions.len(),
            files.len()
        )
        .as_bytes(),
        false,
    )?;
    summary(&format!(
        "## Function and file sizes, informational\n\n{} functions over the size thresholds \
         ({thresholds}), {} files over {FILE_LIMIT} lines of code. Nothing here fails the run; \
         the detail is `complexity.txt` in the reports artifact.\n",
        functions.len(),
        files.len()
    ))
}

/// Where Clippy's thresholds come from: the consumer's own `clippy.toml`, or
/// a generated one carrying this workflow's defaults, handed over through
/// `CLIPPY_CONF_DIR`.
fn thresholds_for(job: &Job) -> Result<(&'static str, Option<PathBuf>), String> {
    if job.project.join("clippy.toml").is_file() || job.project.join(".clippy.toml").is_file() {
        return Ok(("the consumer's clippy.toml", None));
    }
    let directory = job.temp.join("complexity-config");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    fs::write(directory.join("clippy.toml"), DEFAULT_THRESHOLDS)
        .map_err(|error| format!("cannot write the default thresholds: {error}"))?;
    Ok(("this workflow's defaults", Some(directory)))
}

/// Every size finding in Clippy's messages.
fn findings(messages: &Path) -> Result<Vec<Finding>, Failure> {
    let rows = Cmd::new("jaq -r").arg(FINDINGS).arg(messages).capture()?;
    let mut findings = Vec::new();
    for row in rows.lines() {
        let fields: Vec<&str> = row.split('\t').collect();
        if let [lint, value, limit, location] = fields[..] {
            let value = value.parse().unwrap_or(0);
            let limit = limit.parse().unwrap_or(0);
            findings.push((lint.to_owned(), value, limit, location.to_owned()));
        }
    }
    Ok(findings)
}

/// Lines that are not doc comments.
fn code_lines(path: &Path) -> usize {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("///") && !line.starts_with("//!")
        })
        .count()
}
