//! A rule's finding and how a step reports it: the rule, the file relative
//! to the repository and its line, and what to do, one line each; and the
//! exceptions `maestro-quality.toml` takes, each excusing one finding, a stale
//! one reported as a finding itself.

use super::quality_config::Exception;
use crate::runner::{Failure, Outcome, summary, write};
use std::fmt::{self, Write as _};
use std::path::Path;

/// One rule broken at one place.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Finding {
    /// The file, relative to the repository root.
    pub(crate) file: String,
    /// The line, or 0 when the finding is about the whole file.
    pub(crate) line: usize,
    /// The rule broken: `ARC-001`.
    pub(crate) rule: String,
    /// The item an exception names, empty when the rule names none.
    pub(crate) item: String,
    /// What is wrong, and what to do.
    pub(crate) message: String,
}

impl Finding {
    /// A finding of `rule` at `file` and `line`.
    pub(crate) fn new(rule: &str, file: String, line: usize, message: String) -> Self {
        Self {
            file,
            line,
            rule: rule.to_owned(),
            item: String::new(),
            message,
        }
    }

    /// The same finding, naming the item an exception would name.
    #[must_use]
    pub(crate) fn about(mut self, item: &str) -> Self {
        item.clone_into(&mut self.item);
        self
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{} {}: {}", self.rule, self.file, self.message)
        } else {
            write!(
                formatter,
                "{} {}:{}: {}",
                self.rule, self.file, self.line, self.message
            )
        }
    }
}

/// Write every finding, every excused one with its reason, then every report
/// line, to `report` and to the summary under `title`, and fail when a
/// finding is left.
pub(crate) fn publish_findings(
    report: &Path,
    title: &str,
    kept: &[Finding],
    excused: &[(Finding, &str)],
    notes: &[String],
) -> Outcome {
    let mut text = String::new();
    for finding in kept {
        let _ = writeln!(text, "{finding}");
    }
    for (finding, reason) in excused {
        let _ = writeln!(text, "EXCUSED {finding} (because {reason})");
    }
    for note in notes {
        let _ = writeln!(text, "{note}");
    }
    write(report, text.as_bytes(), false)?;
    if kept.is_empty() {
        println!("{title}: no finding, {} excused", excused.len());
        for note in notes {
            println!("{note}");
        }
        return summary(&format!(
            "### {title}\n\nNo finding; {} excused.\n",
            excused.len()
        ));
    }
    eprint!("{text}");
    summary(&format!("### {title}\n\n```text\n{text}```\n"))?;
    let plural = if kept.len() == 1 { "" } else { "s" };
    Err(Failure::from(format!(
        "{}: {} finding{plural}; each names its rule, its file and what to do",
        title.to_lowercase(),
        kept.len()
    )))
}

/// `file` relative to `workspace`, the way a finding names it.
pub(crate) fn relative(workspace: &Path, file: &Path) -> String {
    file.strip_prefix(workspace)
        .unwrap_or(file)
        .display()
        .to_string()
}

/// What the exceptions leave: the findings none excuses; each excused one
/// with its reason; and a finding for every exception of `rules` under
/// `scope` that excuses nothing, since an exception outliving its violation
/// would hide the next one.
pub(crate) fn excuse<'a>(
    findings: Vec<Finding>,
    exceptions: &'a [Exception],
    rules: &[&str],
    scope: &str,
) -> (Vec<Finding>, Vec<(Finding, &'a str)>) {
    let mut used = vec![false; exceptions.len()];
    let mut kept = Vec::new();
    let mut excused = Vec::new();
    for finding in findings {
        let excusing = exceptions.iter().enumerate().find(|(_, exception)| {
            exception.rule == finding.rule
                && exception.path == finding.file
                && exception.item == finding.item
        });
        if let Some((index, exception)) = excusing {
            if let Some(flag) = used.get_mut(index) {
                *flag = true;
            }
            excused.push((finding, exception.reason.as_str()));
        } else {
            kept.push(finding);
        }
    }
    for (exception, used) in exceptions.iter().zip(used) {
        let judged_here =
            rules.contains(&exception.rule.as_str()) && exception.path.starts_with(scope);
        if judged_here && !used {
            let message = format!(
                "the exception for `{}` excuses nothing any more; remove it from \
                 maestro-quality.toml",
                exception.item
            );
            kept.push(
                Finding::new(&exception.rule, exception.path.clone(), 0, message)
                    .about(&exception.item),
            );
        }
    }
    kept.sort();
    (kept, excused)
}

#[cfg(test)]
mod tests {
    use super::{Exception, Finding, excuse};

    /// An exception of `rule` for `item` at `path`.
    fn exception(rule: &str, path: &str, item: &str) -> Exception {
        Exception {
            rule: rule.to_owned(),
            path: path.to_owned(),
            item: item.to_owned(),
            reason: "because".to_owned(),
        }
    }

    #[test]
    fn a_finding_reads_rule_file_line_and_message() {
        let finding = Finding::new(
            "ARC-002",
            "src/a/mod.rs".to_owned(),
            4,
            "move it".to_owned(),
        );
        assert_eq!(finding.to_string(), "ARC-002 src/a/mod.rs:4: move it");
        let whole = Finding::new("ARC-001", "src/a.rs".to_owned(), 0, "cycle".to_owned());
        assert_eq!(whole.to_string(), "ARC-001 src/a.rs: cycle");
    }

    #[test]
    fn exceptions_excuse_their_finding_and_a_stale_one_in_scope_is_reported() {
        let one = |item: &str| {
            Finding::new(
                "ARC-005",
                "p/src/r/mod.rs".to_owned(),
                3,
                "serves one".to_owned(),
            )
            .about(item)
        };
        let exceptions = [
            exception("ARC-005", "p/src/r/mod.rs", "only"),
            exception("ARC-005", "p/src/r/mod.rs", "gone"),
            exception("ARC-005", "q/src/lib.rs", "elsewhere"),
            exception("DUP-001", "p/src/lib.rs", ""),
        ];
        let (kept, excused) = excuse(
            vec![one("only"), one("other")],
            &exceptions,
            &["ARC-005"],
            "p/",
        );
        let kept: Vec<String> = kept.iter().map(ToString::to_string).collect();
        assert_eq!(
            kept,
            [
                "ARC-005 p/src/r/mod.rs: the exception for `gone` excuses nothing any more; \
                 remove it from maestro-quality.toml",
                "ARC-005 p/src/r/mod.rs:3: serves one",
            ]
        );
        assert_eq!(excused.len(), 1);
        assert_eq!(excused[0].1, "because");
    }
}
