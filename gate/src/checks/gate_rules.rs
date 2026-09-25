//! The rules the gate refuses, as one list kept in `gate_rules.tsv`: each
//! rule's ID, its short name, whether `maestro-quality.toml` may excuse a
//! finding of it, and what it holds in one line. `maestro-quality.toml` takes
//! an exception only where the list allows one; `rust-gate gate-rules` prints
//! the list, `just docs` indexes it in `docs/ci.md`, and the organization's
//! `.github` renders it on its page at every release.

/// The list: one rule a line, four fields separated by tabs; `#` opens a
/// comment line.
const LIST: &str = include_str!("gate_rules.tsv");

/// Every rule's line of the list, the comments left out.
pub(crate) fn rules() -> impl Iterator<Item = &'static str> {
    LIST.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The IDs of the rules a repository may take an exception to, in list order.
pub(crate) fn excepted() -> Vec<&'static str> {
    rules()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let id = fields.next()?;
            (fields.nth(1)? == "exception").then_some(id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{excepted, rules};
    use std::collections::BTreeSet;

    /// Whether `id` reads as a rule's ID: three or four capitals, a dash and
    /// three digits.
    fn is_rule_id(id: &str) -> bool {
        id.split_once('-').is_some_and(|(family, number)| {
            (3..=4).contains(&family.len())
                && family.bytes().all(|byte| byte.is_ascii_uppercase())
                && number.len() == 3
                && number.bytes().all(|byte| byte.is_ascii_digit())
        })
    }

    #[test]
    fn every_rule_has_one_id_a_name_a_kind_and_a_line() {
        let mut seen = BTreeSet::new();
        for line in rules() {
            let fields: Vec<&str> = line.split('\t').collect();
            let [id, name, kind, holds] = fields.as_slice() else {
                panic!("not four fields: {line}");
            };
            assert!(is_rule_id(id), "not a rule ID: {id}");
            assert!(seen.insert(*id), "listed twice: {id}");
            assert!(
                !name.trim().is_empty() && !holds.trim().is_empty(),
                "{line}"
            );
            assert!(matches!(*kind, "exception" | "none"), "{line}");
        }
        assert!(seen.len() > 30, "only {} rules", seen.len());
    }

    #[test]
    fn nine_rules_take_an_exception_and_no_other() {
        assert_eq!(
            excepted(),
            [
                "ARC-005", "NAME-004", "TST-001", "DUP-001", "HYG-003", "HYG-006", "HYG-007",
                "DEP-001", "PRF-001"
            ]
        );
    }
}
