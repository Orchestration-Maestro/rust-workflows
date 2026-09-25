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
        assert_eq!(
            wrapped.lines().next().unwrap(),
            "word ".repeat(16).trim_end()
        );
        let long = format!("short {} end", "x".repeat(90));
        assert_eq!(prose(&long), format!("short\n{}\nend", "x".repeat(90)));
        assert_eq!(prose("  two   spaces\ncollapse  "), "two spaces collapse");
    }
}
