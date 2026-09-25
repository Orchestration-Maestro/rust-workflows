//! The golden rules this release carries: the pages of the `.github`
//! repository's golden-rules directory, copied beside the gate and embedded
//! when it is built, and the commit they were copied from. The three rule
//! pages are embedded here; the glossary beside them is embedded by the
//! hygiene step, whose HYG-007 refuses its `_Never_` words.

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
    letters
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
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
        assert!(
            motto_of(NORTHSTAR)
                .unwrap()
                .starts_with("> Automate the guardrails")
        );
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
