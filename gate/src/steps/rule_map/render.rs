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

/// Defaults the organization gave a rule before, which a page may still hold:
/// such a cell is the organization's word, not the repository's, and takes
/// the current default.
const SUPERSEDED: &[(&str, &str)] = &[(
    "C-001",
    "These pages, kept current by `scripts/golden-rules.py`; the drift check fails on a rule \
     not mapped yet",
)];

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
            .filter(|cell| !SUPERSEDED.contains(&(rule.as_str(), *cell)))
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
        assert!(
            engineering
                .1
                .starts_with("# Engineering rules in `example`\n\n`example` follows")
        );
        assert!(engineering.1.contains(
            "| P-001 YAGNI | Review: a reviewer names the principle a change breaks |\n"
        ));
        assert!(engineering.1.contains(&format!(
            "| ENF-001 No machine-named paths | {UNMAPPED} |\n"
        )));
        assert!(
            engineering.1.contains(
                "| C-001 Map every rule | These pages, kept current by `rust-gate rules`"
            )
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
        assert!(
            engineering
                .1
                .contains("## Stricter here\n\nEvery crate is `no_std`.\n\n## Rule")
        );
        assert!(
            engineering
                .1
                .contains("| ENF-001 No machine-named paths | `paths_are_derived` |\n")
        );
        assert!(
            engineering
                .1
                .contains("| P-001 YAGNI | A stricter review |\n")
        );
        assert!(
            security
                .1
                .contains("## What this repository protects\n\nThe release keys.\n\n")
        );
        assert!(
            security
                .1
                .contains("| SEC-001 Minimise sensitive data | No data kept |\n")
        );
        assert!(northstar.1.contains("## The point\n\nFaster checks.\n\n"));
        assert!(
            northstar
                .1
                .contains("| Speed | Check time | 50 s | 40 s | just check |\n")
        );
        assert!(
            northstar
                .1
                .contains(&format!("| Quality | {UNMAPPED} | not measured |"))
        );
    }

    /// The engineering page rendered over a previous one that holds `previous`.
    fn engineering_over(previous: &str) -> String {
        let [_, engineering, _] = pages("example", |path: &str| {
            if path.ends_with("engineering.md") {
                previous.to_owned()
            } else {
                String::new()
            }
        })
        .unwrap();
        engineering.1
    }

    #[test]
    fn a_cell_holding_a_superseded_default_takes_the_current_one() {
        let superseded = "| C-001 Map every rule | These pages, kept current by \
                          `scripts/golden-rules.py`; the drift check fails on a rule not mapped \
                          yet |\n";
        let own = "| C-001 Map every rule | Our own map, checked by `just check` |\n";
        for (previous, expected) in [
            (
                superseded,
                "| C-001 Map every rule | These pages, kept current by `rust-gate rules`",
            ),
            (own, own),
        ] {
            assert!(engineering_over(previous).contains(expected), "{expected}");
        }
    }

    #[test]
    fn the_intro_names_the_command_and_the_rules_version() {
        let [_, engineering, _] = fresh();
        let short = &commit()[..7];
        assert!(engineering.1.contains(&format!("`.github@{short}`")));
        assert!(engineering.1.contains("`rust-gate rules` writes the rows"));
        assert!(
            engineering
                .1
                .contains(&format!("/blob/{}/golden-rules/", commit()))
        );
    }
}
