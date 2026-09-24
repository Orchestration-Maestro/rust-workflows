//! `maestro-quality.toml`, the one file a repository writes to shape the
//! organization's rules: the layers a target declares and the exceptions the
//! repository takes, each with its reason. Read through the pinned jaq, which
//! reads TOML, so the gate stays standard-library only.

use crate::runner::{Cmd, Failure};
use std::collections::BTreeSet;
use std::path::Path;

/// The file, at the root of the repository.
pub(crate) const FILE: &str = "maestro-quality.toml";

/// The rules that take an exception; every other rule has none.
const EXCEPTED: &[&str] = &[
    "ARC-005", "DUP-001", "HYG-003", "TST-001", "DEP-001", "PRF-001",
];

/// The tables the file may hold.
const TABLES: &[&str] = &["crate", "exception", "limits", "performance", "typos"];

/// The top-level tables of the file, one per line.
const KEYS: &str = "keys[]";

/// Each `[[crate]]`: its root, a tab, then its layers joined by `|`, the
/// modules of one layer by spaces.
const CRATES: &str = ".crate // [] | .[] | [.root, (.layers | map(if type == \"array\" \
    then join(\" \") else . end) | join(\"|\"))] | @tsv";

/// Each `[[exception]]`: rule, path, item and reason, tab-separated.
const EXCEPTIONS: &str =
    ".exception // [] | .[] | [.rule, .path, (.item // \"\"), (.reason // \"\")] | @tsv";

/// Each `[limits]` key and its value, tab-separated.
const LIMITS: &str = ".limits // {} | to_entries[] | [.key, (.value | tostring)] | @tsv";

/// The organization's floors a repository may tighten: the longest file, in
/// lines of code, and the widest line, in columns.
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    /// The most lines of code a file may hold, doc comments not counted.
    pub(crate) file_lines: usize,
    /// The most columns a line may take.
    pub(crate) line_columns: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_lines: 500,
            line_columns: 100,
        }
    }
}

/// The layers one target declares, left to right.
pub(crate) struct Layers {
    /// The target's root file, relative to the repository root.
    pub(crate) root: String,
    /// Each layer's top-level modules.
    pub(crate) layers: Vec<Vec<String>>,
}

/// One exception: the rule, where, the item when the rule names one, and why.
pub(crate) struct Exception {
    /// The rule it excuses: `ARC-005`.
    pub(crate) rule: String,
    /// The file, relative to the repository root.
    pub(crate) path: String,
    /// The item, empty when the rule names none.
    pub(crate) item: String,
    /// Why the finding stays.
    pub(crate) reason: String,
}

/// What the file says, empty when the repository has none.
#[derive(Default)]
pub(crate) struct QualityConfig {
    /// The layers each declared target keeps.
    pub(crate) layers: Vec<Layers>,
    /// The exceptions the repository takes.
    pub(crate) exceptions: Vec<Exception>,
    /// The limits, the organization's floors unless the file tightens them.
    pub(crate) limits: Limits,
}

/// Read the file at the root of `workspace`, refusing a table it does not
/// know, a declaration of layers that orders nothing, and an exception its
/// rule does not take or that gives no reason.
pub(crate) fn read_config(workspace: &Path) -> Result<QualityConfig, Failure> {
    let file = workspace.join(FILE);
    if !file.is_file() {
        return Ok(QualityConfig::default());
    }
    let query = |program: &str| {
        Cmd::new("jaq --from toml -r")
            .arg(program)
            .arg(&file)
            .capture()
    };
    for table in query(KEYS)?.lines() {
        if !TABLES.contains(&table) {
            return Err(format!(
                "{FILE}: unknown table `{table}`; it takes {}",
                TABLES.join(", ")
            )
            .into());
        }
    }
    let mut limits = Limits::default();
    for line in query(LIMITS)?.lines() {
        parse_limit(&mut limits, line)?;
    }
    Ok(QualityConfig {
        limits,
        layers: query(CRATES)?
            .lines()
            .map(parse_layers)
            .collect::<Result<_, _>>()?,
        exceptions: query(EXCEPTIONS)?
            .lines()
            .map(parse_exception)
            .collect::<Result<_, _>>()?,
    })
}

/// One `[limits]` line applied to `limits`: a known key, a whole number,
/// and never looser than the organization's floor.
fn parse_limit(limits: &mut Limits, line: &str) -> Result<(), Failure> {
    let (key, value) = line.split_once('\t').unwrap_or((line, ""));
    let floors = Limits::default();
    let (slot, floor) = match key {
        "file-lines" => (&mut limits.file_lines, floors.file_lines),
        "line-columns" => (&mut limits.line_columns, floors.line_columns),
        _ => {
            return Err(format!(
                "{FILE}: unknown limit `{key}`; it takes file-lines, line-columns"
            )
            .into());
        }
    };
    let Ok(number) = value.parse::<usize>() else {
        return Err(format!("{FILE}: {key} must be a whole number").into());
    };
    if number > floor {
        return Err(format!(
            "{FILE}: {key} = {number} loosens the organization's {floor}; \
             a repository may only tighten it"
        )
        .into());
    }
    *slot = number;
    Ok(())
}

/// One line of the layers listing.
fn parse_layers(line: &str) -> Result<Layers, Failure> {
    let (root, joined) = line.split_once('\t').unwrap_or((line, ""));
    let layers: Vec<Vec<String>> = joined
        .split('|')
        .map(|layer| {
            layer
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|layer| !layer.is_empty())
        .collect();
    if root.is_empty() || layers.len() < 2 {
        return Err(format!("{FILE}: a [[crate]] names its root and two layers at least").into());
    }
    let mut seen = BTreeSet::new();
    for module in layers.iter().flatten() {
        if !seen.insert(module) {
            return Err(format!("{FILE}: module `{module}` sits in two layers of {root}").into());
        }
    }
    Ok(Layers {
        root: root.to_owned(),
        layers,
    })
}

/// One line of the exceptions listing.
fn parse_exception(line: &str) -> Result<Exception, Failure> {
    let mut fields = line.split('\t').map(str::to_owned);
    let mut next = || fields.next().unwrap_or_default();
    let (rule, path, item, reason) = (next(), next(), next(), next());
    if !EXCEPTED.contains(&rule.as_str()) {
        return Err(format!(
            "{FILE}: {rule} takes no exception; only {} do",
            EXCEPTED.join(", ")
        )
        .into());
    }
    if path.is_empty() || reason.trim().is_empty() {
        return Err(
            format!("{FILE}: the {rule} exception names no path or gives no reason").into(),
        );
    }
    Ok(Exception {
        rule,
        path,
        item,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::{Limits, parse_exception, parse_layers, parse_limit};

    #[test]
    fn layers_split_into_modules_and_refuse_a_single_layer_or_a_repeated_module() {
        let layers = parse_layers("gate/src/main.rs\tsteps|checks|runner").unwrap();
        assert_eq!(layers.root, "gate/src/main.rs");
        assert_eq!(layers.layers, [["steps"], ["checks"], ["runner"]]);
        let grouped = parse_layers("tests/workflows.rs\tci gate|harness").unwrap();
        assert_eq!(grouped.layers, [vec!["ci", "gate"], vec!["harness"]]);
        assert_eq!(
            parse_layers("gate/src/main.rs\tsteps")
                .err()
                .unwrap()
                .message
                .as_deref(),
            Some("maestro-quality.toml: a [[crate]] names its root and two layers at least")
        );
        assert_eq!(
            parse_layers("r.rs\ta b|b")
                .err()
                .unwrap()
                .message
                .as_deref(),
            Some("maestro-quality.toml: module `b` sits in two layers of r.rs")
        );
    }

    #[test]
    fn limits_tighten_the_organization_floors_and_never_loosen_them() {
        let mut limits = Limits::default();
        assert_eq!((limits.file_lines, limits.line_columns), (500, 100));
        parse_limit(&mut limits, "file-lines\t400").unwrap();
        parse_limit(&mut limits, "line-columns\t90").unwrap();
        assert_eq!((limits.file_lines, limits.line_columns), (400, 90));
        let refusal = |line: &str| {
            parse_limit(&mut Limits::default(), line)
                .err()
                .unwrap()
                .message
                .unwrap_or_default()
        };
        assert_eq!(
            refusal("file-lines\t600"),
            "maestro-quality.toml: file-lines = 600 loosens the organization's 500; \
             a repository may only tighten it"
        );
        assert_eq!(
            refusal("line-columns\twide"),
            "maestro-quality.toml: line-columns must be a whole number"
        );
        assert_eq!(
            refusal("depth\t3"),
            "maestro-quality.toml: unknown limit `depth`; it takes file-lines, line-columns"
        );
    }

    #[test]
    fn exceptions_need_a_rule_that_takes_one_a_path_and_a_reason() {
        let exception =
            parse_exception("ARC-005\tgate/src/runner/mod.rs\tenter\tthe registry runs steps")
                .unwrap();
        assert_eq!(
            (exception.rule.as_str(), exception.item.as_str()),
            ("ARC-005", "enter")
        );
        assert_eq!(
            parse_exception("ARC-001\tx\t\twhy")
                .err()
                .unwrap()
                .message
                .as_deref(),
            Some(
                "maestro-quality.toml: ARC-001 takes no exception; \
                 only ARC-005, DUP-001, HYG-003, TST-001, DEP-001, PRF-001 do"
            )
        );
        assert_eq!(
            parse_exception("ARC-005\tx\t\t")
                .err()
                .unwrap()
                .message
                .as_deref(),
            Some("maestro-quality.toml: the ARC-005 exception names no path or gives no reason")
        );
    }
}
