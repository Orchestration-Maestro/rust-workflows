# Rule map in rust-gate, phase 1: `rust-gate rules` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rust-gate rules` writes a repository's rule map from the golden
rules the release carries, keeps what the repository wrote, and
`rust-gate rules --check` refuses a stale page or a row not mapped yet.

**Architecture:** One new step module, `gate/src/steps/rule_map`, in the gate's
`steps` layer, reaching only the runner: `golden.rs` embeds and reads the
copy of the golden rules, `page.rs` reads back a repository's pages and wraps
paragraphs, `render.rs` renders the three pages, `step.rs` declares and runs
the two local steps. The copy of the golden rules lives in `gate/golden-rules`
and is embedded with `include_str!`.

**Tech Stack:** Rust 1.98.1, edition 2024, standard library only; `git` as a
declared tool; the contract-test harness of `tests/`.

**Spec:** `docs/superpowers/specs/2026-09-24-guide-and-rule-map-design.md`

## Global Constraints

- Standard library only; no regular expressions: every parser is hand-written.
- Every item documented (`clippy::missing_docs_in_private_items` denied);
  Clippy pedantic and the organization lints (LNT-001) at `-D warnings`:
  no `unwrap`, `expect`, `panic!` or indexing in product code, no single-letter
  identifiers, at most two path segments (`use std::mem;`, then `mem::take`).
- Functions of 100 lines or fewer, files of 500 lines of code or fewer, Rust
  lines of 100 columns or fewer; rustfmt does not wrap string literals, so a
  long literal is split with a trailing `\` or `concat!`.
- Every refusal literal inside `Err(`, `Failure::from(`, `.map_err(`,
  `.ok_or(` or `.ok_or_else(` is asserted by a test.
- Every step module with functions has unit tests; test names have four words
  or more; a local step is run by a contract test.
- A module doc never holds the literal attribute that opens a test module;
  write `` `cfg(test)` `` if it must be named.
- `.github/copilot-instructions.md` lists every tracked file, directories
  first, each sorted by name, explanations from column 49.
- A backticked path in Markdown that ends with `/` or starts with `docs/`,
  `.github/`, `examples/`, `tests/` or `scripts/` must exist.
- Commit only after `just check` exits 0, with `PATH=$PWD/.tools/bin:$PATH`
  and `MISE_TRUSTED_CONFIG_PATHS=$PWD` in the same shell; commits signed
  (`-S`), titles conventional and lowercase, lines of 80 columns or fewer.
- Below each page's intro, the output equals `.github`'s `golden-rules.py`
  byte for byte, but for the C-001 default (spec section 4.1).
- The golden rules are copied from `.github` at commit
  `0b8cede58a0ff4db4f4cf0cf8a92c8ef0dfbb5cb`; the organization defaults come
  from its `golden-rules.py` at `df6abce512262b9be1cde2ec6cbbb0ff01c3c303`.

---

### Task 1: The renderer and `rust-gate rules`

**Files:**

- Create: `gate/golden-rules/{commit.txt,engineering.md,northstar.md,security.md}`
- Create: `gate/src/steps/rule_map/{mod.rs,golden.rs,page.rs,render.rs,step.rs}`
- Modify: `gate/src/steps/mod.rs` (declare `mod rule_map;`)
- Modify: `gate/src/steps/registry.rs` (register `super::rule_map::STEPS`)
- Create: `rule_map.rs` in `tests/ci/`; modify `tests/ci/mod.rs`
- Modify: `.github/copilot-instructions.md`, `docs/steps.md` (by `just docs`)

**Interfaces:**

- Produces, `golden.rs`: `ENGINEERING`, `SECURITY`, `NORTHSTAR: &str`;
  `fn commit() -> &'static str`; `fn is_rule_id(&str) -> bool`;
  `fn is_pillar(&str) -> bool`; `fn rules_of(&str) -> Vec<(String, String)>`;
  `fn pillars_of(&str) -> Result<Vec<String>, String>`;
  `fn motto_of(&str) -> Result<String, String>`.
- Produces, `page.rs`: `fn section(&str, &str) -> Option<String>`;
  `fn cells(&str) -> BTreeMap<String, String>`;
  `fn kpis(&str) -> BTreeMap<String, [String; 4]>`; `fn prose(&str) -> String`.
- Produces, `render.rs`: `const UNMAPPED: &str`; `fn pages(name: &str,
  previous: impl Fn(&str) -> String) -> Result<[(&'static str, String); 3],
  String>`, pages in the order northstar, engineering, security.
- Produces, `step.rs`: `STEPS` with the local step `rules`.
- [ ] **Step 1: Copy the golden rules beside the gate**

```bash
G=/home/franc/workspace/Orchestration-Maestro/.github
C=0b8cede58a0ff4db4f4cf0cf8a92c8ef0dfbb5cb
mkdir -p gate/golden-rules
for page in engineering security northstar; do
  git -C "$G" show "$C:golden-rules/$page.md" > "gate/golden-rules/$page.md"
done
echo "$C" > gate/golden-rules/commit.txt
```

- [ ] **Step 2: Write the module door**

`gate/src/steps/rule_map/mod.rs`:

```rust
//! `rust-gate rules` and `rules --check`: a repository's rule map, the
//! organization's golden rules adapted to it (C-001), in three pages under
//! `docs/standards`. `golden.rs` reads the golden rules this release carries,
//! `page.rs` reads back what a repository wrote, `render.rs` writes the pages
//! and `step.rs` holds the steps.

mod golden;
mod page;
mod render;
mod step;

pub(super) use step::STEPS;
```

- [ ] **Step 3: Write `golden.rs` with its tests**

```rust
//! The golden rules this release carries: the three pages of the `.github`
//! repository's golden-rules directory, copied beside the gate and embedded
//! when it is built, and the commit they were copied from.

/// The engineering rules page.
pub(super) const ENGINEERING: &str = include_str!("../../../golden-rules/engineering.md");

/// The security rules page.
pub(super) const SECURITY: &str = include_str!("../../../golden-rules/security.md");

/// The Northstar page.
pub(super) const NORTHSTAR: &str = include_str!("../../../golden-rules/northstar.md");

/// The `.github` commit the three pages were copied from, on one line.
const COMMIT: &str = include_str!("../../../golden-rules/commit.txt");

/// The `.github` commit the embedded pages were copied from.
pub(super) fn commit() -> &'static str {
    COMMIT.trim()
}

/// Whether `text` is a rule ID: capital letters, a hyphen, three digits.
pub(super) fn is_rule_id(text: &str) -> bool {
    text.split_once('-').is_some_and(|(prefix, number)| {
        !prefix.is_empty()
            && prefix.bytes().all(|byte| byte.is_ascii_uppercase())
            && number.len() == 3
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Whether `word` names a pillar: a capital, then lowercase letters.
pub(super) fn is_pillar(word: &str) -> bool {
    let mut letters = word.chars();
    letters.next().is_some_and(|first| first.is_ascii_uppercase())
        && word.len() > 1
        && letters.all(|letter| letter.is_ascii_lowercase())
}

/// The rule IDs and titles of a golden-rules page, in document order: every
/// `### ID — Title` heading, and every `| P-NNN | Name |` principle row.
pub(super) fn rules_of(page: &str) -> Vec<(String, String)> {
    page.lines()
        .filter_map(|line| heading_rule(line).or_else(|| principle_row(line)))
        .collect()
}

/// The rule a `### ID — Title` heading names.
fn heading_rule(line: &str) -> Option<(String, String)> {
    let (id, title) = line.strip_prefix("### ")?.split_once(" — ")?;
    let title = title.trim();
    (is_rule_id(id) && !title.is_empty()).then(|| (id.to_owned(), title.to_owned()))
}

/// The principle a `| P-NNN | Name |` table row names.
fn principle_row(line: &str) -> Option<(String, String)> {
    let (id, rest) = line.strip_prefix("| ")?.split_once(" | ")?;
    let (name, _) = rest.split_once('|')?;
    let name = name.trim();
    (id.starts_with("P-") && is_rule_id(id) && !name.is_empty())
        .then(|| (id.to_owned(), name.to_owned()))
}

/// The Northstar's pillars, from the table under its `## Four pillars`
/// heading, or the refusal naming what the page lacks.
pub(super) fn pillars_of(northstar: &str) -> Result<Vec<String>, String> {
    let mut lines = northstar
        .lines()
        .skip_while(|line| *line != "## Four pillars");
    if lines.next().is_none() {
        return Err(missing("Four pillars table"));
    }
    Ok(lines
        .take_while(|line| !line.starts_with("## "))
        .filter_map(|line| line.strip_prefix("| ")?.split_once(" |"))
        .map(|(word, _)| word)
        .filter(|word| is_pillar(word) && *word != "Pillar")
        .map(str::to_owned)
        .collect())
}

/// The Northstar's opening quotation: its first run of `> ` lines, or the
/// refusal naming what the page lacks.
pub(super) fn motto_of(northstar: &str) -> Result<String, String> {
    let quoted: Vec<&str> = northstar
        .lines()
        .skip_while(|line| !line.starts_with("> "))
        .take_while(|line| line.starts_with("> "))
        .collect();
    if quoted.is_empty() {
        return Err(missing("opening quotation"));
    }
    Ok(quoted.join("\n"))
}

/// The refusal for an embedded Northstar that lacks `what`: the copy of the
/// golden rules changed shape, and this module has to follow it.
fn missing(what: &str) -> String {
    format!("rules: the embedded Northstar has no {what}; follow its new shape in rule_map")
}

#[cfg(test)]
mod tests {
    use super::{
        ENGINEERING, NORTHSTAR, SECURITY, commit, is_pillar, is_rule_id, motto_of, pillars_of,
        rules_of,
    };

    #[test]
    fn every_golden_page_yields_its_rules_in_document_order() {
        let engineering = rules_of(ENGINEERING);
        assert_eq!(engineering.len(), 40);
        assert_eq!(
            engineering[0],
            ("FND-001".to_owned(), "Think before coding".to_owned())
        );
        assert!(engineering.contains(&("P-013".to_owned(), "Parse, don't validate".to_owned())));
        assert_eq!(engineering[39].0, "C-006");
        let security = rules_of(SECURITY);
        assert_eq!(security.len(), 11);
        assert_eq!(
            security[0],
            ("SEC-001".to_owned(), "Minimise sensitive data".to_owned())
        );
    }

    #[test]
    fn a_rule_id_is_capitals_a_hyphen_and_three_digits() {
        for id in ["FND-001", "P-018", "SEC-011", "C-006"] {
            assert!(is_rule_id(id), "{id}");
        }
        for text in ["fnd-001", "ENF-01", "ENF-0012", "-001", "ENF001", "ENF-00a"] {
            assert!(!is_rule_id(text), "{text}");
        }
    }

    #[test]
    fn the_northstar_yields_its_four_pillars_and_motto() {
        assert_eq!(
            pillars_of(NORTHSTAR).unwrap(),
            ["Speed", "Quality", "Maintainability", "Security"]
        );
        assert!(motto_of(NORTHSTAR).unwrap().starts_with("> Automate the guardrails"));
        assert!(is_pillar("Speed"));
        for word in ["speed", "S", "Pillar1"] {
            assert!(!is_pillar(word), "{word}");
        }
    }

    #[test]
    fn a_northstar_without_its_table_or_motto_is_refused() {
        assert_eq!(
            pillars_of("# Northstar\n").unwrap_err(),
            "rules: the embedded Northstar has no Four pillars table; follow its new shape \
             in rule_map"
        );
        assert_eq!(
            motto_of("# Northstar\n").unwrap_err(),
            "rules: the embedded Northstar has no opening quotation; follow its new shape \
             in rule_map"
        );
    }

    #[test]
    fn the_embedded_commit_is_forty_hexadecimal_digits() {
        assert_eq!(commit().len(), 40);
        assert!(commit().bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
```

- [ ] **Step 4: Write `page.rs` with its tests**

```rust
//! What a repository already wrote in its rule map, read back so that a new
//! rendering keeps it, and the one way the pages wrap a paragraph.

use super::golden::{is_pillar, is_rule_id};
use std::collections::BTreeMap;
use std::mem;

/// The column a paragraph wraps at.
const WIDTH: usize = 80;

/// The body of the section a `## title` heading opens, from after the blank
/// line under it to the next `## ` heading, trimmed; None when the page has
/// no such section, or it is empty.
pub(super) fn section(text: &str, title: &str) -> Option<String> {
    let heading = format!("## {title}");
    let mut lines = text.lines().skip_while(|line| *line != heading);
    lines.next()?;
    if !lines.next()?.is_empty() {
        return None;
    }
    let body = lines
        .take_while(|line| !line.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n");
    let body = body.trim();
    (!body.is_empty()).then(|| body.to_owned())
}

/// What each rule's row says, by rule ID, from `| ID title | held by |` rows.
pub(super) fn cells(text: &str) -> BTreeMap<String, String> {
    text.lines().filter_map(cell).collect()
}

/// The rule and what holds it, from one rule-map row.
fn cell(line: &str) -> Option<(String, String)> {
    let (id, rest) = line.strip_prefix("| ")?.split_once(' ')?;
    let (_, held) = rest.split_once("| ")?;
    let held = held.strip_suffix(" |")?.trim();
    (is_rule_id(id) && !held.is_empty()).then(|| (id.to_owned(), held.to_owned()))
}

/// Each pillar's KPI, current value, target and measurement, by pillar.
pub(super) fn kpis(text: &str) -> BTreeMap<String, [String; 4]> {
    text.lines().filter_map(kpi).collect()
}

/// A pillar and its four cells, from one row of the KPI table.
fn kpi(line: &str) -> Option<(String, [String; 4])> {
    let inner = line.strip_prefix("| ")?.strip_suffix(" |")?;
    let mut cells = inner.splitn(5, " | ").map(str::trim);
    let pillar = cells.next()?;
    let row = [cells.next()?, cells.next()?, cells.next()?, cells.next()?].map(str::to_owned);
    (is_pillar(pillar) && pillar != "Pillar").then(|| (pillar.to_owned(), row))
}

/// A paragraph wrapped at `WIDTH` columns: its words joined by one space, a
/// line broken before the word that would pass the width, and a longer word
/// alone on its line.
pub(super) fn prose(text: &str) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut width = 0;
    for word in text.split_whitespace() {
        let length = word.chars().count();
        if width > 0 && width + 1 + length > WIDTH {
            lines.push(mem::take(&mut line));
            width = 0;
        }
        if width > 0 {
            line.push(' ');
            width += 1;
        }
        line.push_str(word);
        width += length;
    }
    if width > 0 {
        lines.push(line);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{cells, kpis, prose, section};

    /// A rule map a repository has partly filled in.
    const PAGE: &str = concat!(
        "# Rules\n\nIntro.\n\n## Stricter here\n\nEvery crate is `no_std`.\n\n",
        "## Rule map\n\n| Rule | Held here by |\n| --- | --- |\n",
        "| ENF-001 No paths | A test |\n| ENF-002 Platforms |   |\n\n## KPIs\n\n",
        "| Pillar | KPI | Current | Target | Measured by |\n| --- | --- | --- | --- | --- |\n",
        "| Speed | Check time | 50 s | 40 s | just check |\n",
    );

    #[test]
    fn a_section_body_stops_at_the_next_heading() {
        assert_eq!(
            section(PAGE, "Stricter here").unwrap(),
            "Every crate is `no_std`."
        );
        assert_eq!(section(PAGE, "The point"), None);
        assert_eq!(section("## Empty\n\n\n## Next\n", "Empty"), None);
        assert_eq!(section("## Tight\nNo blank line.\n", "Tight"), None);
    }

    #[test]
    fn rows_and_kpis_are_read_back_by_rule_and_pillar() {
        let kept = cells(PAGE);
        assert_eq!(kept.get("ENF-001").map(String::as_str), Some("A test"));
        assert!(!kept.contains_key("ENF-002"), "an empty cell is not kept");
        assert_eq!(kept.len(), 1);
        let measured = kpis(PAGE);
        assert_eq!(measured.len(), 1);
        assert_eq!(
            measured["Speed"],
            ["Check time", "50 s", "40 s", "just check"]
        );
    }

    #[test]
    fn prose_wraps_at_eighty_columns_and_keeps_long_words_whole() {
        let wrapped = prose(&"word ".repeat(30));
        assert!(wrapped.lines().all(|line| line.chars().count() <= 80));
        assert_eq!(wrapped.lines().next().unwrap(), "word ".repeat(16).trim_end());
        let long = format!("short {} end", "x".repeat(90));
        assert_eq!(prose(&long), format!("short\n{}\nend", "x".repeat(90)));
        assert_eq!(prose("  two   spaces\ncollapse  "), "two spaces collapse");
    }
}
```

- [ ] **Step 5: Write `render.rs` with its tests**

```rust
//! The three pages of a repository's rule map, rendered from the golden rules
//! this release carries and from what the repository's pages say today: a row,
//! a section or a KPI the repository wrote is kept, a rule the organization
//! holds everywhere gets its default, and any other rule arrives not mapped.

use super::golden::{ENGINEERING, NORTHSTAR, SECURITY, commit, motto_of, pillars_of, rules_of};
use super::page::{cells, kpis, prose, section};

/// What a row, a section or a KPI says until the repository maps it.
pub(super) const UNMAPPED: &str = "Not mapped yet";

/// The Northstar page of every repository's rule map.
const NORTHSTAR_PAGE: &str = "docs/standards/northstar.md";

/// The engineering page of every repository's rule map.
const ENGINEERING_PAGE: &str = "docs/standards/engineering.md";

/// The security page of every repository's rule map.
const SECURITY_PAGE: &str = "docs/standards/security.md";

/// What holds a principle every repository holds by review.
const PRINCIPLE: &str = "Review: a reviewer names the principle a change breaks";

/// The principles every repository holds by review, whatever it is for.
const REVIEWED_PRINCIPLES: [&str; 14] = [
    "P-001", "P-002", "P-003", "P-004", "P-005", "P-006", "P-007", "P-008", "P-009", "P-010",
    "P-012", "P-015", "P-016", "P-017",
];

/// What holds a rule in every repository, because the organization holds it.
const HELD_BY_ORGANIZATION: &[(&str, &str)] = &[
    (
        "FND-001",
        "Review: the pull request states its assumptions, the alternatives weighed and what \
         stays unclear",
    ),
    (
        "FND-002",
        "Review: the pull request names the requirement each change serves; nothing \
         speculative lands",
    ),
    (
        "FND-003",
        "Review: every changed line traces to the pull request's goal",
    ),
    (
        "FND-004",
        "Review: the pull request's Verification section holds the check that means done, \
         and its output",
    ),
    ("ENF-003", "Review: prose and identifiers are English"),
    (
        "ENF-004",
        "Organization: the `commits-are-conventional` ruleset refuses any other title on the \
         default branch",
    ),
    (
        "ENF-006",
        "Organization: required checks and code scanning block every merge; a gate changes \
         only in its own reviewed pull request",
    ),
    (
        "ENF-007",
        "Organization: the `default-branch-discipline` ruleset (pull request, signed commits, \
         code scanning) and `floor-no-destruction`, with no bypass actor",
    ),
    (
        "ENF-010",
        "Organization: its settings as code in `.github/org/`, checked weekly by \
         `org-drift.yml`",
    ),
    (
        "ENF-011",
        "Organization: authority lives in rulesets, workflow `permissions:` and access \
         control; no instruction file grants any",
    ),
    (
        "ENF-013",
        "Organization: secret scanning with push protection and validity checks \
         (`maestrolabs-baseline`); CI: gitleaks",
    ),
    (
        "ENF-014",
        "Organization: two-factor authentication is required of every member and outside \
         collaborator",
    ),
    (
        "C-001",
        "These pages, kept current by `rust-gate rules` at every commit; the daily drift \
         check reports a row not mapped yet",
    ),
    (
        "C-004",
        "Organization: the daily drift check opens a `Drift:` issue for this repository",
    ),
    (
        "C-005",
        "GitHub: pull requests, CI runs with their reports, and drift issues",
    ),
    (
        "C-006",
        "Review: an exception is recorded in the pull request that makes it, with its scope, \
         rationale and expiry",
    ),
    (
        "SEC-004",
        "Organization: rulesets, workflow permissions and the organization bot's own App \
         identity; automation borrows no person's credentials",
    ),
    (
        "SEC-005",
        "Review: a publication, release or settings change is approved in its own pull \
         request",
    ),
    (
        "SEC-007",
        "Review: a suspected exposure stops the work and goes to SECURITY.md's private \
         channel; a leaked secret is revoked and rotated",
    ),
    (
        "SEC-008",
        "Review: results are reported as run, with what was not checked",
    ),
    (
        "SEC-009",
        "Review: blocked work is reported as partial, never as done",
    ),
    (
        "SEC-010",
        "Organization: private vulnerability reporting is on (`maestrolabs-baseline`), and \
         SECURITY.md routes reports to it",
    ),
];

/// "The point" until the repository writes its own.
const POINT: &str = "Not mapped yet: whose problem this repository solves, and what changes \
                     for them when it works.";

/// "What this repository protects" until the repository writes its own.
const PROTECTS: &str = "Not mapped yet: what this repository holds or runs that an attacker \
                        would want, and where untrusted input enters it.";

/// The KPI row of a pillar the repository has not measured yet.
const UNMEASURED: [&str; 4] = [
    UNMAPPED,
    "not measured",
    "set from the first baseline",
    "none yet",
];

/// The three pages of the repository `name`, by path, in the order they are
/// written; `previous` reads what a page says today, empty when it is new.
pub(super) fn pages(
    name: &str,
    previous: impl Fn(&str) -> String,
) -> Result<[(&'static str, String); 3], String> {
    Ok([
        (
            NORTHSTAR_PAGE,
            northstar(name, &previous(NORTHSTAR_PAGE))?.join("\n"),
        ),
        (
            ENGINEERING_PAGE,
            engineering(name, &previous(ENGINEERING_PAGE)).join("\n"),
        ),
        (
            SECURITY_PAGE,
            security(name, &previous(SECURITY_PAGE)).join("\n"),
        ),
    ])
}

/// The two paragraphs a rule-map page opens with, and the blank line
/// between them.
fn intro(name: &str, page: &str, title: &str) -> Vec<String> {
    let commit = commit();
    let short = commit.get(..7).unwrap_or_default();
    vec![
        prose(&format!(
            "`{name}` follows the organization's [{title}](https://github.com/\
             Orchestration-Maestro/.github/blob/{commit}/golden-rules/{page}). This page is its \
             rule map (C-001): for every rule, what holds it here, or why it does not apply. A \
             row may name a stricter local rule; none weakens one."
        )),
        String::new(),
        prose(&format!(
            "`rust-gate rules` writes the rows from the golden rules of `.github@{short}` at \
             every commit and keeps what each row says here. A rule added there arrives as \
             \"{UNMAPPED}\", and the daily drift check reports it until it is mapped."
        )),
    ]
}

/// What holds `rule` in every repository, when the organization holds it.
fn held_by_organization(rule: &str) -> Option<&'static str> {
    if REVIEWED_PRINCIPLES.contains(&rule) {
        return Some(PRINCIPLE);
    }
    HELD_BY_ORGANIZATION
        .iter()
        .find(|(id, _)| *id == rule)
        .map(|(_, held)| *held)
}

/// The table of the golden-rules page `page`: every rule, with what the
/// repository's `previous` page says holds it, else the default.
fn rule_map(page: &str, previous: &str) -> Vec<String> {
    let kept = cells(previous);
    let mut lines = vec![
        "| Rule | Held here by |".to_owned(),
        "| --- | --- |".to_owned(),
    ];
    for (rule, title) in rules_of(page) {
        let held = kept
            .get(&rule)
            .map(String::as_str)
            .or_else(|| held_by_organization(&rule))
            .unwrap_or(UNMAPPED);
        lines.push(format!("| {rule} {title} | {held} |"));
    }
    lines
}

/// The engineering page: its intro, the repository's stricter rules when it
/// wrote some, and the rule map.
fn engineering(name: &str, previous: &str) -> Vec<String> {
    let mut lines = vec![format!("# Engineering rules in `{name}`"), String::new()];
    lines.extend(intro(name, "engineering.md", "engineering rules"));
    lines.push(String::new());
    if let Some(stricter) = section(previous, "Stricter here") {
        lines.extend([
            "## Stricter here".to_owned(),
            String::new(),
            stricter,
            String::new(),
        ]);
    }
    lines.extend(["## Rule map".to_owned(), String::new()]);
    lines.extend(rule_map(ENGINEERING, previous));
    lines.push(String::new());
    lines
}

/// The security page: its intro, what the repository protects, and the rule
/// map.
fn security(name: &str, previous: &str) -> Vec<String> {
    let mut lines = vec![format!("# Security rules in `{name}`"), String::new()];
    lines.extend(intro(name, "security.md", "security rules"));
    lines.extend([
        String::new(),
        "## What this repository protects".to_owned(),
        String::new(),
        section(previous, "What this repository protects").unwrap_or_else(|| prose(PROTECTS)),
        String::new(),
        "## Rule map".to_owned(),
        String::new(),
    ]);
    lines.extend(rule_map(SECURITY, previous));
    lines.push(String::new());
    lines
}

/// The Northstar page: the motto, the point the repository exists for, and
/// one KPI per pillar.
fn northstar(name: &str, previous: &str) -> Result<Vec<String>, String> {
    let kept = kpis(previous);
    let mut lines = vec![
        format!("# Northstar for `{name}`"),
        String::new(),
        motto_of(NORTHSTAR)?,
        String::new(),
        prose(&format!(
            "`{name}` steers by the organization's [Northstar](https://github.com/\
             Orchestration-Maestro/.github/blob/{}/golden-rules/northstar.md): one KPI per \
             pillar, each with its measurement. Unmeasured is written `not measured`, never \
             estimated; a value read by hand carries the date it was read.",
            commit()
        )),
        String::new(),
        "## The point".to_owned(),
        String::new(),
        section(previous, "The point").unwrap_or_else(|| prose(POINT)),
        String::new(),
        "## KPIs".to_owned(),
        String::new(),
        "| Pillar | KPI | Current | Target | Measured by |".to_owned(),
        "| --- | --- | --- | --- | --- |".to_owned(),
    ];
    for pillar in pillars_of(NORTHSTAR)? {
        let [kpi, current, target, measured] = kept
            .get(&pillar)
            .cloned()
            .unwrap_or_else(|| UNMEASURED.map(str::to_owned));
        lines.push(format!(
            "| {pillar} | {kpi} | {current} | {target} | {measured} |"
        ));
    }
    lines.push(String::new());
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{UNMAPPED, commit, pages};

    /// The pages of a repository called `example` that has none yet.
    fn fresh() -> [(&'static str, String); 3] {
        pages("example", |_| String::new()).unwrap()
    }

    #[test]
    fn a_new_repository_gets_organization_defaults_and_unmapped_rows() {
        let [northstar, engineering, security] = fresh();
        assert_eq!(northstar.0, "docs/standards/northstar.md");
        assert!(engineering.1.starts_with("# Engineering rules in `example`\n\n`example` follows"));
        assert!(engineering.1.contains(
            "| P-001 YAGNI | Review: a reviewer names the principle a change breaks |\n"
        ));
        assert!(
            engineering
                .1
                .contains(&format!("| ENF-001 No machine-named paths | {UNMAPPED} |\n"))
        );
        assert!(
            engineering
                .1
                .contains("| C-001 Map every rule | These pages, kept current by `rust-gate rules`")
        );
        assert!(!engineering.1.contains("## Stricter here"));
        assert!(engineering.1.ends_with(" |\n"));
        assert!(
            security
                .1
                .contains("## What this repository protects\n\nNot mapped yet: what this")
        );
        assert!(northstar.1.contains(
            "| Speed | Not mapped yet | not measured | set from the first baseline | none yet |\n"
        ));
        assert!(
            northstar
                .1
                .contains("\n## The point\n\nNot mapped yet: whose problem")
        );
    }

    #[test]
    fn every_row_and_section_a_repository_wrote_survives_a_new_rendering() {
        let previous = |path: &str| {
            match path {
                "docs/standards/engineering.md" => concat!(
                    "## Stricter here\n\nEvery crate is `no_std`.\n\n## Rule map\n\n",
                    "| ENF-001 No machine-named paths | `paths_are_derived` |\n",
                    "| P-001 YAGNI | A stricter review |\n",
                ),
                "docs/standards/security.md" => concat!(
                    "## What this repository protects\n\nThe release keys.\n\n## Rule map\n\n",
                    "| SEC-001 Minimise sensitive data | No data kept |\n",
                ),
                _ => concat!(
                    "## The point\n\nFaster checks.\n\n## KPIs\n\n",
                    "| Speed | Check time | 50 s | 40 s | just check |\n",
                ),
            }
            .to_owned()
        };
        let [northstar, engineering, security] = pages("example", previous).unwrap();
        assert!(engineering.1.contains("## Stricter here\n\nEvery crate is `no_std`.\n\n## Rule"));
        assert!(
            engineering
                .1
                .contains("| ENF-001 No machine-named paths | `paths_are_derived` |\n")
        );
        assert!(engineering.1.contains("| P-001 YAGNI | A stricter review |\n"));
        assert!(security.1.contains("## What this repository protects\n\nThe release keys.\n\n"));
        assert!(security.1.contains("| SEC-001 Minimise sensitive data | No data kept |\n"));
        assert!(northstar.1.contains("## The point\n\nFaster checks.\n\n"));
        assert!(northstar.1.contains("| Speed | Check time | 50 s | 40 s | just check |\n"));
        assert!(northstar.1.contains(&format!("| Quality | {UNMAPPED} | not measured |")));
    }

    #[test]
    fn the_intro_names_the_command_and_the_rules_version() {
        let [_, engineering, _] = fresh();
        let short = &commit()[..7];
        assert!(engineering.1.contains(&format!("`.github@{short}`")));
        assert!(engineering.1.contains("`rust-gate rules` writes the rows"));
        assert!(engineering.1.contains(&format!("/blob/{}/golden-rules/", commit())));
    }
}
```

- [ ] **Step 6: Write `step.rs` with the `rules` step and its tests**

```rust
//! The step `rules`: it writes each page of the rule map that differs from
//! the rendering, and says how many rows are not mapped yet.

use super::render::{UNMAPPED, pages};
use crate::runner::{Cmd, Failure, Outcome, Step, write};
use std::env;
use std::fs;
use std::path::Path;

/// What these steps declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[Step {
    workflow: "local",
    id: "rules",
    summary: "The rule map written from the golden rules this release carries",
    inputs: &[],
    tools: &["git"],
    reports: &[],
    run,
}];

/// The three pages rendered for the repository here.
type Rendered = [(&'static str, String); 3];

/// Run `rules`: write every page whose text differs from the rendering.
fn run() -> Outcome {
    let rendered = rendered()?;
    for (path, text) in &rendered {
        if fs::read_to_string(path).ok().as_ref() == Some(text) {
            continue;
        }
        let file = Path::new(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("rules: {path}: {error}"))?;
        }
        write(file, text.as_bytes(), false)?;
    }
    println!("rules: {} rows not mapped yet", unmapped(&rendered));
    Ok(())
}

/// The three pages of the repository here, rendered.
fn rendered() -> Result<Rendered, Failure> {
    let name = repository_name()?;
    pages(&name, |path| fs::read_to_string(path).unwrap_or_default()).map_err(Failure::from)
}

/// The repository's name: the last part of its `origin` remote, else the
/// name of the directory it runs in.
fn repository_name() -> Result<String, Failure> {
    let remote = Cmd::new("git config --get remote.origin.url")
        .capture()
        .unwrap_or_default();
    let name = name_of(&remote);
    if !name.is_empty() {
        return Ok(name.to_owned());
    }
    let here = env::current_dir().map_err(|error| Failure::from(format!("rules: {error}")))?;
    Ok(here
        .file_name()
        .map(|directory| directory.to_string_lossy().into_owned())
        .unwrap_or_default())
}

/// The last part of a remote URL, without a trailing `/` or `.git`.
fn name_of(remote: &str) -> &str {
    let remote = remote.trim().trim_end_matches('/');
    let remote = remote.strip_suffix(".git").unwrap_or(remote);
    remote.rsplit('/').next().unwrap_or_default()
}

/// The rows and sections still not mapped, counted below each page's intro,
/// which names the marker itself.
fn unmapped(rendered: &Rendered) -> usize {
    rendered
        .iter()
        .map(|(_, text)| {
            text.split_once("\n## ")
                .map_or(0, |(_, body)| body.matches(UNMAPPED).count())
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{name_of, unmapped};

    #[test]
    fn the_repository_name_is_the_last_part_of_its_remote() {
        assert_eq!(
            name_of("https://github.com/Orchestration-Maestro/maestro-core.git\n"),
            "maestro-core"
        );
        assert_eq!(
            name_of("git@github.com:Orchestration-Maestro/release-canary.git"),
            "release-canary"
        );
        assert_eq!(
            name_of("https://github.com/Orchestration-Maestro/.github/"),
            ".github"
        );
        assert_eq!(name_of(""), "");
    }

    #[test]
    fn only_the_rows_below_the_intro_count_as_unmapped() {
        let page = |text: &str| ("docs/standards/page.md", text.to_owned());
        let rendered = [
            page("Intro says \"Not mapped yet\".\n## Rule map\n| A | Not mapped yet |\n"),
            page("No heading, Not mapped yet.\n"),
            page("\n## KPIs\n| Speed | Not mapped yet | not measured |\n"),
        ];
        assert_eq!(unmapped(&rendered), 2);
    }
}
```

- [ ] **Step 7: Register the module and its step**

In `gate/src/steps/mod.rs`, add `mod rule_map;` between
`mod require_every_check;` and `mod secret_scan;`. In
`gate/src/steps/registry.rs`, add `super::rule_map::STEPS,` right after
`super::write_lints::STEPS,`.

- [ ] **Step 8: Write the contract test**

`rule_map.rs` in `tests/ci/`, and `mod rule_map;` in `tests/ci/mod.rs` between
`mod repository_hygiene;` and `mod scorecard_and_required_status;`:

```rust
//! `rust-gate rules` and `rules --check`: a repository's rule map written
//! from the golden rules the release carries, kept where the repository wrote
//! it, and refused when stale or not mapped yet.

use crate::harness::{Fixture, succeeds};
use std::fs;
use std::process::Output;

/// The three pages of the rule map.
const PAGES: [&str; 3] = [
    "docs/standards/engineering.md",
    "docs/standards/northstar.md",
    "docs/standards/security.md",
];

/// Run `command` in the fixture's project.
fn in_project(fixture: &Fixture, command: &str) -> Output {
    fixture.run_body(&format!("cd project && {command}"))
}

/// Every page of the fixture's rule map, in `PAGES` order.
fn pages(fixture: &Fixture) -> Vec<String> {
    PAGES
        .iter()
        .map(|path| fs::read_to_string(fixture.root.join("project").join(path)).unwrap())
        .collect()
}

#[test]
fn rules_writes_three_pages_and_a_second_run_changes_nothing() {
    let fixture = Fixture::new();
    succeeds(&in_project(
        &fixture,
        "git init -q && git remote add origin \
         https://github.com/Orchestration-Maestro/example.git && rust-gate rules",
    ));
    let first = pages(&fixture);
    assert!(first[0].starts_with("# Engineering rules in `example`\n"));
    assert!(first[1].starts_with("# Northstar for `example`\n\n> Automate the guardrails"));
    assert!(first[2].contains("| SEC-011 Sign every release | Not mapped yet |\n"));
    succeeds(&in_project(&fixture, "rust-gate rules"));
    assert_eq!(first, pages(&fixture));
}
```

- [ ] **Step 9: Run the new tests and watch them pass**

```bash
export PATH=$PWD/.tools/bin:$PATH MISE_TRUSTED_CONFIG_PATHS=$PWD
cargo test --manifest-path gate/Cargo.toml rule_map
cargo test --manifest-path tests/Cargo.toml rule_map
```

Expected: every `rule_map` test passes. A failure here is a defect of the
code above, not of the test: fix the code.

- [ ] **Step 10: Update the Copilot guide and the generated docs**

In `.github/copilot-instructions.md`, add, explanations from column 49:

- under `gate/`, before its `src` directory: the directory
  `golden-rules` "The golden rules this release carries, copied from .github",
  then `commit.txt` "The .github commit the golden rules were copied from",
  `engineering.md` "Copy of the engineering rules; never edited here",
  `northstar.md` "Copy of the Northstar; never edited here",
  `security.md` "Copy of the security rules; never edited here";
- under `gate/src/steps/`, after the `quality_scorecard` directory: the
  directory `rule_map`
  "rust-gate rules: a repository's rule map, the golden rules adapted to it",
  then `golden.rs` "The embedded golden rules: their rules, pillars and motto",
  `mod.rs` "The step's door: its four modules and its declarations",
  `page.rs` "What a repository wrote, read back; paragraphs wrapped at 80",
  `render.rs` "The three pages: kept rows, organization defaults, not mapped",
  `step.rs` "rust-gate rules: the pages written where they differ";
- under `tests/ci/`, after `repository_hygiene.rs`: `rule_map.rs`
  "rules and rules --check: written, kept, refused when stale or unmapped".

Then run `just docs`, which rewrites `docs/steps.md` with the new step.

- [ ] **Step 11: Check and commit**

```bash
timeout 1800 just check && git add -A && git commit -S -q -m \
  "feat: write the rule map with rust-gate rules"
```

Expected: `just check` exits 0. Fix any finding it names before committing.

### Task 2: `rust-gate rules --check`

**Files:**

- Modify: `gate/src/steps/rule_map/step.rs`
- Modify: `rule_map.rs` in `tests/ci/`
- Modify: `docs/steps.md` (by `just docs`)

**Interfaces:**

- Consumes: `pages`, `UNMAPPED` from Task 1.
- Produces: the local step `rules --check`.
- [ ] **Step 1: Write the failing contract tests**

Append to `rule_map.rs` in `tests/ci/`, and add `refused` to its
`use crate::harness::{...}` line:

```rust
#[test]
fn the_check_refuses_every_stale_page_and_counts_unmapped_rows() {
    let fixture = Fixture::new();
    refused(
        &in_project(&fixture, "rust-gate rules --check"),
        "rules --check: stale docs/standards/northstar.md, docs/standards/engineering.md, \
         docs/standards/security.md; 21 rows not mapped yet; run rust-gate rules, then map \
         every row",
    );
    succeeds(&in_project(&fixture, "rust-gate rules"));
    refused(
        &in_project(&fixture, "rust-gate rules --check"),
        "rules --check: stale none; 21 rows not mapped yet",
    );
}

#[test]
fn a_mapped_row_survives_and_the_check_passes_once_all_are_mapped() {
    let fixture = Fixture::new();
    succeeds(&in_project(&fixture, "rust-gate rules"));
    for path in PAGES {
        let file = fixture.root.join("project").join(path);
        let mapped = fs::read_to_string(&file)
            .unwrap()
            .split("\n\n")
            .map(|paragraph| {
                if paragraph.starts_with("Not mapped yet:") {
                    "Filled in by a test."
                } else {
                    paragraph
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
            .replace("| Not mapped yet |", "| Held by a test |");
        fs::write(&file, mapped).unwrap();
    }
    succeeds(&in_project(&fixture, "rust-gate rules"));
    let written = pages(&fixture);
    assert!(written[0].contains("| ENF-001 No machine-named paths | Held by a test |\n"));
    assert!(written[1].contains("## The point\n\nFilled in by a test.\n"));
    succeeds(&in_project(&fixture, "rust-gate rules --check"));
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test --manifest-path tests/Cargo.toml rule_map`
Expected: both new tests FAIL with `unknown gate command: rules --check`.

- [ ] **Step 3: Add the step**

In `gate/src/steps/rule_map/step.rs`, change the module doc's first sentence
to "The steps: `rules` writes each page of the rule map that differs from the
rendering, and `rules --check` refuses a stale page or a row not mapped yet,
which the daily drift check reports." Replace `STEPS` with:

```rust
/// What these steps declare: their inputs, their tools and their reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "local",
        id: "rules",
        summary: "The rule map written from the golden rules this release carries",
        inputs: &[],
        tools: &["git"],
        reports: &[],
        run,
    },
    Step {
        workflow: "local",
        id: "rules --check",
        summary: "The rule map compared with the golden rules this release carries",
        inputs: &[],
        tools: &["git"],
        reports: &[],
        run: check,
    },
];
```

Add, after `run`:

```rust
/// Run `rules --check`: refuse a stale page or a row not mapped yet.
fn check() -> Outcome {
    let rendered = rendered()?;
    let stale: Vec<&str> = rendered
        .iter()
        .filter(|(path, text)| fs::read_to_string(path).ok().as_ref() != Some(text))
        .map(|(path, _)| *path)
        .collect();
    refusal(&stale, unmapped(&rendered)).map_or(Ok(()), |message| Err(Failure::from(message)))
}

/// The refusal for `stale` pages and `unmapped` rows, with the fix; None
/// when the rule map is current and complete.
fn refusal(stale: &[&str], unmapped: usize) -> Option<String> {
    if stale.is_empty() && unmapped == 0 {
        return None;
    }
    let stale = if stale.is_empty() {
        "none".to_owned()
    } else {
        stale.join(", ")
    };
    Some(format!(
        "rules --check: stale {stale}; {unmapped} rows not mapped yet; run rust-gate rules, \
         then map every row"
    ))
}
```

Add to its unit tests, importing `refusal`:

```rust
    #[test]
    fn a_stale_page_or_an_unmapped_row_is_refused_with_the_fix() {
        assert_eq!(refusal(&[], 0), None);
        assert_eq!(
            refusal(&["docs/standards/security.md"], 0).unwrap(),
            "rules --check: stale docs/standards/security.md; 0 rows not mapped yet; run \
             rust-gate rules, then map every row"
        );
        assert_eq!(
            refusal(&[], 3).unwrap(),
            "rules --check: stale none; 3 rows not mapped yet; run rust-gate rules, then map \
             every row"
        );
    }
```

- [ ] **Step 4: Run the tests and watch them pass**

```bash
cargo test --manifest-path gate/Cargo.toml rule_map
cargo test --manifest-path tests/Cargo.toml rule_map
```

Expected: PASS.

- [ ] **Step 5: Regenerate, check and commit**

```bash
just docs && timeout 1800 just check && git add -A && git commit -S -q -m \
  "feat: refuse a stale or unmapped rule map"
```

### Task 3: Documentation and parity

**Files:**

- Modify: `docs/ci.md` (one paragraph after the organization lints paragraph)

- [ ] **Step 1: Document the commands**

In `docs/ci.md`, after the paragraph that ends "and removes none. ... reads
all of it as test code.", add:

```markdown
`rust-gate rules`, run at the root of a repository, writes its rule map, the
organization's golden rules adapted to it (C-001), from the copy of the golden
rules the release carries: `docs/standards/engineering.md`, `security.md` and
`northstar.md`. A row, a section or a KPI the repository wrote is kept; a rule
the organization holds everywhere gets its default; any other rule arrives as
"Not mapped yet". `rust-gate rules --check` refuses a stale page or a row not
mapped yet.
```

- [ ] **Step 2: Compare with the Python script on every repository**

```bash
W=/home/franc/workspace/Orchestration-Maestro
RW=$PWD
P=$(mktemp -d)
mkdir -p "$P/org/scripts" "$P/org/golden-rules"
git -C "$W/.github" show df6abce512262b9be1cde2ec6cbbb0ff01c3c303:scripts/golden-rules.py \
  > "$P/org/scripts/golden-rules.py"
cp gate/golden-rules/*.md "$P/org/golden-rules/"
for repo in maestro-core maestro-model-router release-canary; do
  gh repo clone "Orchestration-Maestro/$repo" "$P/$repo-py" -- -q --depth 1
  cp -r "$P/$repo-py" "$P/$repo-rs"
  (cd "$P/$repo-py" && python3 "$P/org/scripts/golden-rules.py" > /dev/null)
  (cd "$P/$repo-rs" && cargo run -q --manifest-path "$RW/gate/Cargo.toml" --locked \
    --offline -- rules > /dev/null)
done
python3 - "$P" <<'EOF'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
c001_py = ("These pages, kept current by `scripts/golden-rules.py`; the drift check fails "
           "on a rule not mapped yet")
c001_rs = ("These pages, kept current by `rust-gate rules` at every commit; the daily drift "
           "check reports a row not mapped yet")
failed = False
for repo in ("maestro-core", "maestro-model-router", "release-canary"):
    for page in ("engineering.md", "security.md", "northstar.md"):
        py = (root / f"{repo}-py/docs/standards/{page}").read_text().replace(c001_py, c001_rs)
        rs = (root / f"{repo}-rs/docs/standards/{page}").read_text()
        same = py.split("\n## ", 1)[1:] == rs.split("\n## ", 1)[1:]
        failed |= not same
        print(("same " if same else "DIFF ") + f"{repo} {page}")
sys.exit(failed)
EOF
```

Expected: nine `same` lines and exit 0. A `DIFF` is a parity defect: show the
two files with `diff`, fix the Rust side, and rerun from Task 1's tests.

- [ ] **Step 3: Check and commit**

```bash
timeout 1800 just check && git add -A && git commit -S -q -m \
  "docs: document rust-gate rules and rules --check"
```

- [ ] **Step 4: Ask the owner before publishing**

Pushing and opening the pull request are outward actions: report the three
commits, the parity result and `just check`, and wait for the owner's go. Then
push the branch and open one pull request titled
`feat: write each repository's rule map with rust-gate rules`.
