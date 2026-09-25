//! HYG-007: no text the repository tracks uses a word a glossary marks
//! `_Never_`, the organization's glossary this release carries or the
//! repository's own `CONTEXT.md`. A `_Never_` word is refused whatever it
//! means in the sentence; an `_Avoid_` word depends on its meaning, so review
//! judges it and the gate does not. Records keep the words of their day, and
//! a glossary names the words it refuses, so neither is read.

use crate::checks::findings::Finding;
use std::fs;
use std::mem;
use std::path::Path;

/// The organization's glossary, copied from `.github` beside the golden
/// rules and embedded when the gate is built.
const ORGANIZATION: &str = include_str!("../../../golden-rules/glossary.md");

/// The extensions of the text files read.
const TEXT: &[&str] = &["md", "rs", "toml", "yml", "yaml", "sh", "py", "json", "txt"];

/// The records, which keep the words they were written with.
const RECORDS: &[&str] = &["docs/adr/", "docs/superpowers/", "specs/"];

/// One word no repository uses: its words, the term to use, and the glossary
/// that says so.
struct Never {
    /// The word, split into lowercase words.
    words: Vec<String>,
    /// The term the glossary defines instead.
    term: String,
    /// Where the glossary is, as a finding names it.
    source: &'static str,
}

/// HYG-007 over `files`.
pub(super) fn findings(workspace: &Path, files: &[String]) -> Vec<Finding> {
    let mut never = refused(ORGANIZATION, "the organization's glossary");
    if let Ok(context) = fs::read_to_string(workspace.join("CONTEXT.md")) {
        never.extend(refused(&context, "CONTEXT.md"));
    }
    files
        .iter()
        .filter(|file| is_read(file))
        .filter_map(|file| {
            let text = fs::read_to_string(workspace.join(file)).ok()?;
            Some(text_findings(file, &text, &never))
        })
        .flatten()
        .collect()
}

/// HYG-007 over the text of one file.
fn text_findings(file: &str, text: &str, never: &[Never]) -> Vec<Finding> {
    let words = words_of(text);
    let mut found = Vec::new();
    for (index, (_, line)) in words.iter().enumerate() {
        let rest = words.get(index..).unwrap_or_default();
        for word in never.iter().filter(|word| matches(rest, &word.words)) {
            let said = word.words.join(" ");
            let message = format!(
                "`{said}` is a word {} never uses; say {}",
                word.source,
                lowered(&word.term)
            );
            found.push(Finding::new("HYG-007", file.to_owned(), *line, message).about(&said));
        }
    }
    found
}

/// `term` with its first letter in lowercase, unless it opens an acronym.
fn lowered(term: &str) -> String {
    let mut letters = term.chars();
    match (letters.next(), letters.next()) {
        (Some(first), Some(second)) if !second.is_uppercase() => {
            first.to_lowercase().chain(term.chars().skip(1)).collect()
        }
        _ => term.to_owned(),
    }
}

/// Whether HYG-007 reads `file`: a text file that is neither a record nor a
/// glossary, nor `maestro-quality.toml`, whose exceptions name the words
/// they excuse.
fn is_read(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    let text = name == "justfile"
        || name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| TEXT.contains(&extension));
    let record = name == "CHANGELOG.md" || RECORDS.iter().any(|record| file.starts_with(record));
    let glossary = name == "CONTEXT.md" || name == "glossary.md";
    text && !record && !glossary && file != "maestro-quality.toml"
}

/// Whether `words` open with `never`, its last word also taken with a plural
/// or past `s`, `es` or `ed`.
fn matches(words: &[(String, usize)], never: &[String]) -> bool {
    let Some((last, first)) = never.split_last() else {
        return false;
    };
    let Some(said) = words.get(..never.len()) else {
        return false;
    };
    let head = said
        .iter()
        .zip(first)
        .all(|((word, _), never)| word == never);
    head && said.last().is_some_and(|(word, _)| {
        word.strip_prefix(last.as_str())
            .is_some_and(|rest| ["", "s", "es", "ed"].contains(&rest))
    })
}

/// Every `_Never_` word of a glossary, with the term whose entry names it: an
/// entry opens with `**Term**:` at the start of a line.
fn refused(glossary: &str, source: &'static str) -> Vec<Never> {
    let mut term = String::new();
    let mut found = Vec::new();
    for line in glossary.lines() {
        if let Some((name, _)) = line
            .strip_prefix("**")
            .and_then(|rest| rest.split_once("**:"))
        {
            name.clone_into(&mut term);
        } else if let Some(list) = line.strip_prefix("_Never_:")
            && !term.is_empty()
        {
            found.extend(listed(list).into_iter().map(|words| Never {
                words,
                term: term.clone(),
                source,
            }));
        }
    }
    found
}

/// The words of each item of a `_Never_:` list, a parenthesised note left
/// out.
fn listed(list: &str) -> Vec<Vec<String>> {
    list.trim()
        .trim_end_matches('.')
        .split(',')
        .map(|item| {
            let item = item.split_once('(').map_or(item, |(item, _)| item);
            words_of(item)
                .into_iter()
                .map(|(word, _)| word)
                .collect::<Vec<_>>()
        })
        .filter(|words| !words.is_empty())
        .collect()
}

/// The words of a text in lowercase, each with its line, URLs left out:
/// `snake_case`, `kebab-case`, `camelCase`, `PascalCase` and `SCREAMING_CASE`
/// all split into their words.
fn words_of(text: &str) -> Vec<(String, usize)> {
    let mut words = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for token in line.split_whitespace() {
            if token.contains("://") {
                continue;
            }
            for run in token.split(|letter: char| !letter.is_ascii_alphanumeric()) {
                words.extend(split_case(run).into_iter().map(|word| (word, index + 1)));
            }
        }
    }
    words
}

/// A run of letters and digits split where its case says a word begins:
/// `HTTPServer` is `http` and `server`, `camelCase` is `camel` and `case`.
fn split_case(run: &str) -> Vec<String> {
    let letters: Vec<char> = run.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    for (index, &letter) in letters.iter().enumerate() {
        let previous = index.checked_sub(1).and_then(|at| letters.get(at));
        let next = letters.get(index + 1);
        let begins = letter.is_ascii_uppercase()
            && previous.is_some_and(|previous| {
                !previous.is_ascii_uppercase() || next.is_some_and(char::is_ascii_lowercase)
            });
        if begins && !word.is_empty() {
            words.push(mem::take(&mut word));
        }
        word.push(letter.to_ascii_lowercase());
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::{is_read, lowered, matches, refused, split_case, words_of};

    /// The words of `text`, without their lines.
    fn bare(text: &str) -> Vec<String> {
        words_of(text).into_iter().map(|(word, _)| word).collect()
    }

    #[test]
    fn every_case_splits_into_lowercase_words() {
        assert_eq!(split_case("HTTPServer"), ["http", "server"]);
        assert_eq!(split_case("camelCase"), ["camel", "case"]);
        assert_eq!(
            bare("snake_case kebab-case SCREAMING_CASE https://x.y/a_b"),
            ["snake", "case", "kebab", "case", "screaming", "case"]
        );
    }

    #[test]
    fn a_term_matches_its_consecutive_words_and_their_endings() {
        let never = bare("coherence check");
        assert!(matches(&words_of("coherenceChecks pass"), &never));
        assert!(matches(&words_of("coherence\ncheck"), &never));
        assert!(!matches(&words_of("coherence of the check"), &never));
        assert!(!matches(&words_of("coherence checker"), &never));
        assert!(!matches(&words_of("coherence"), &never));
    }

    #[test]
    fn a_glossary_refuses_its_never_words_under_their_term() {
        let glossary = concat!(
            "**Allowlist**:\nA list.\n_Avoid_: pass list\n_Never_: green list, ",
            "gold list (in any sense).\n\n**Gate**: A check.\n_Never_: guard\n",
        );
        let found = refused(glossary, "CONTEXT.md");
        let listed: Vec<(String, &str)> = found
            .iter()
            .map(|never| (never.words.join(" "), never.term.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("green list".to_owned(), "Allowlist"),
                ("gold list".to_owned(), "Allowlist"),
                ("guard".to_owned(), "Gate"),
            ]
        );
    }

    #[test]
    fn a_term_is_said_in_lowercase_unless_an_acronym() {
        assert_eq!(lowered("Coherence check"), "coherence check");
        assert_eq!(lowered("CI run"), "CI run");
        assert_eq!(lowered("A"), "A");
    }

    #[test]
    fn records_and_glossaries_are_not_read() {
        assert!(!is_read("maestro-quality.toml") && is_read("sub/maestro-quality.toml"));
        assert!(is_read("docs/ci.md") && is_read("justfile") && is_read("src/a.rs"));
        assert!(!is_read("CHANGELOG.md") && !is_read("docs/adr/0001-a.md"));
        assert!(!is_read("specs/001/plan.md") && !is_read("CONTEXT.md"));
        assert!(!is_read("gate/golden-rules/glossary.md") && !is_read("logo.png"));
    }
}
