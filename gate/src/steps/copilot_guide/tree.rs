//! The annotated tree of the guide: every tracked file under its
//! directories, each with the explanation the current guide gives it, else
//! the one a README table or an image's alternative text gives, else what
//! the file or the directory says of itself.

use super::describe::describe_file;
use super::text::{module_doc, paragraph, sentence};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The column explanations start at, as in rust-workflows' own guide.
const COLUMN: usize = 48;

/// What a directory is for when nothing else says, by path, then by name.
const BY_DIRECTORY: [(&str, &str); 27] = [
    (".github", "GitHub metadata, templates and workflows"),
    (".github/workflows", "GitHub Actions workflows"),
    (".github/ISSUE_TEMPLATE", "Issue forms and the chooser"),
    (".github/skills", "Agent skills"),
    (".cargo", "Cargo settings for this workspace"),
    (".config", "Tool settings that live in a directory"),
    (".specify", "Spec Kit's templates, scripts and workflows"),
    ("assets", "Images and other assets"),
    ("bin", "Binaries, one file per executable"),
    ("expected", "What the tool must write for the example input"),
    ("plans", "Implementation plans"),
    (
        "profile/foundations",
        "The foundation cards on the organization page",
    ),
    (
        "profile/pillars",
        "The four pillar icons on the organization page",
    ),
    ("crates", "The workspace's crates"),
    ("docs", "Documentation"),
    ("docs/adr", "Architecture decision records"),
    ("examples", "Worked examples"),
    ("fixtures", "Test fixtures"),
    (
        "org",
        "The organization's live settings, as scripts/export-org.py exports them",
    ),
    (
        "org/rulesets",
        "Organization rulesets, in the shape the API accepts",
    ),
    ("profile", "The organization page on GitHub"),
    ("scripts", "Maintenance scripts"),
    ("specs", "Specifications, one directory per slice"),
    ("src", "The crate's sources"),
    (
        "supply-chain",
        "cargo-vet audits, configuration and imports",
    ),
    ("tests", "Integration tests"),
    (
        "workflow-templates",
        "Workflow templates offered under Actions, New workflow",
    ),
];

/// A directory of the tree: its subdirectories and its files, by name.
#[derive(Debug, Default)]
struct Directory {
    /// The directories it holds, by name.
    directories: BTreeMap<String, Directory>,
    /// The files it holds, by name.
    files: Vec<String>,
}

impl Directory {
    /// Place the file at `path`, creating the directories it lies under.
    fn insert(&mut self, path: &str) {
        match path.split_once('/') {
            Some((directory, rest)) => self
                .directories
                .entry(directory.to_owned())
                .or_default()
                .insert(rest),
            None => self.files.push(path.to_owned()),
        }
    }
}

/// What explaining an entry reads: the repository, the explanations the
/// current guide gives, and those README tables and images give.
#[derive(Debug)]
struct Explained<'guide> {
    /// The repository's root.
    root: &'guide Path,
    /// The current guide's explanations, by path.
    previous: &'guide BTreeMap<String, String>,
    /// The READMEs' and images' explanations, by path.
    tables: &'guide BTreeMap<String, String>,
}

impl Explained<'_> {
    /// Append the entries of `directory`, whose path is `parent`, drawn
    /// under `prefix`: its directories first, then its files, each by name.
    fn walk(
        &self,
        directory: &Directory,
        parent: &str,
        prefix: &str,
        entries: &mut Vec<(String, String)>,
    ) {
        let mut files = directory.files.clone();
        files.sort();
        let count = directory.directories.len() + files.len();
        let names = directory
            .directories
            .iter()
            .map(|(name, holds)| (name, Some(holds)))
            .chain(files.iter().map(|name| (name, None)));
        for (index, (name, holds)) in names.enumerate() {
            let last = index + 1 == count;
            let relative = if parent.is_empty() {
                name.clone()
            } else {
                format!("{parent}/{name}")
            };
            let branch = if last { "└── " } else { "├── " };
            let Some(holds) = holds else {
                entries.push((format!("{prefix}{branch}{name}"), self.file(&relative)));
                continue;
            };
            entries.push((
                format!("{prefix}{branch}{name}/"),
                self.directory(&relative),
            ));
            let deeper = format!("{prefix}{}", if last { "    " } else { "│   " });
            self.walk(holds, &relative, &deeper, entries);
        }
    }

    /// The explanation of the directory at `relative`.
    fn directory(&self, relative: &str) -> String {
        self.previous
            .get(relative)
            .cloned()
            .unwrap_or_else(|| describe_directory(self.root, relative, self.tables))
    }

    /// The explanation of the file at `relative`.
    fn file(&self, relative: &str) -> String {
        self.previous
            .get(relative)
            .or_else(|| self.tables.get(relative))
            .filter(|text| !text.is_empty())
            .cloned()
            .unwrap_or_else(|| describe_file(self.root, relative))
    }
}

/// The tree's lines: the root, then every tracked path, each padded so the
/// explanations line up from `COLUMN` or further.
pub(super) fn tree_lines(
    root: &Path,
    paths: &[String],
    previous: &BTreeMap<String, String>,
    tables: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut top = Directory::default();
    for path in paths {
        top.insert(path);
    }
    let root_text = previous
        .get(".")
        .cloned()
        .unwrap_or_else(|| "Repository root".to_owned());
    let mut entries = vec![(".".to_owned(), root_text)];
    let explained = Explained {
        root,
        previous,
        tables,
    };
    explained.walk(&top, "", "", &mut entries);
    let width = entries
        .iter()
        .map(|(entry, _)| entry.chars().count() + 2)
        .max()
        .unwrap_or(COLUMN)
        .max(COLUMN);
    entries
        .into_iter()
        .map(|(entry, text)| {
            let padding = " ".repeat(width - entry.chars().count());
            format!("{entry}{padding}# {text}")
        })
        .collect()
}

/// What a directory is for: its README table row, its README's first
/// sentence, its Rust module's doc, else its usual role or its name.
fn describe_directory(root: &Path, relative: &str, tables: &BTreeMap<String, String>) -> String {
    if let Some(row) = tables.get(&format!("{relative}/")) {
        return row.clone();
    }
    let directory = root.join(relative);
    if let Some(found) = paragraph(&read(&directory.join("README.md"))) {
        return found;
    }
    for module in [directory.join("mod.rs"), directory.with_extension("rs")] {
        if let Some(doc) = module_doc(&read(&module)) {
            return sentence(doc);
        }
    }
    let name = relative.rsplit('/').next().unwrap_or(relative);
    BY_DIRECTORY
        .iter()
        .find(|(path, _)| *path == relative)
        .or_else(|| BY_DIRECTORY.iter().find(|(path, _)| *path == name))
        .map_or_else(|| capitalized(name), |(_, role)| (*role).to_owned())
}

/// `name` as words, the first capitalized and the others in lowercase.
fn capitalized(name: &str) -> String {
    let words = name.replace(['-', '_'], " ");
    let mut characters = words.chars();
    characters.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(characters.flat_map(char::to_lowercase))
            .collect()
    })
}

/// The whole file at `path`, empty when it cannot be read.
fn read(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// The explanations the current guide gives, by path, `.` for the root.
pub(super) fn kept(guide: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let Some(block) = fenced_text(guide) else {
        return found;
    };
    let mut stack: Vec<&str> = Vec::new();
    for line in block.lines() {
        let (entry, text) = line.split_once("  # ").unwrap_or((line, ""));
        let (entry, text) = (entry.trim_end(), text.trim());
        if entry == "." {
            found.insert(".".to_owned(), text.to_owned());
            continue;
        }
        let Some((depth, name)) = tree_entry(entry).filter(|_| !text.is_empty()) else {
            continue;
        };
        stack.truncate(depth);
        let bare = name.trim_end_matches('/');
        let path = stack
            .iter()
            .copied()
            .chain([bare])
            .collect::<Vec<_>>()
            .join("/");
        found.insert(path, text.to_owned());
        if name.ends_with('/') {
            stack.push(bare);
        }
    }
    found
}

/// The body of the first fenced block of `guide` whose language is `text`.
fn fenced_text(guide: &str) -> Option<&str> {
    let (_, after) = guide.split_once("```text\n")?;
    after.split_once("```").map(|(block, _)| block)
}

/// The depth and name of one tree line: whole units of `│   ` or four
/// spaces, then `├── ` or `└── `, then the name.
fn tree_entry(entry: &str) -> Option<(usize, &str)> {
    let mut depth = 0;
    let mut rest = entry;
    loop {
        if let Some(name) = rest
            .strip_prefix("├── ")
            .or_else(|| rest.strip_prefix("└── "))
        {
            return (!name.is_empty()).then_some((depth, name));
        }
        rest = rest
            .strip_prefix("│   ")
            .or_else(|| rest.strip_prefix("    "))?;
        depth += 1;
    }
}

/// The explanations READMEs give paths in their tables, a first cell naming
/// a backticked path and a second saying what it is, relative to each
/// README, the shallowest README's first; then each image's alternative
/// text where no table explains it.
pub(super) fn readme_rows(root: &Path, paths: &[String]) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    let mut readmes: Vec<&String> = paths
        .iter()
        .filter(|path| path.rsplit('/').next() == Some("README.md"))
        .collect();
    readmes.sort_by_key(|path| path.matches('/').count());
    for readme in readmes {
        let base = parent_of(readme);
        for (named, text) in read(&root.join(readme)).lines().filter_map(table_row) {
            let slash = if named.ends_with('/') { "/" } else { "" };
            rows.entry(format!("{}{slash}", joined(base, named)))
                .or_insert_with(|| sentence(text));
        }
    }
    for (path, alt) in image_alts(root, paths) {
        rows.entry(path).or_insert(alt);
    }
    rows
}

/// The directory part of the relative path `path`, empty at the root.
fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// The path and explanation one README table row gives: a first cell that
/// is a backticked path, a second that says what it is.
fn table_row(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('|')?.trim_start().strip_prefix('`')?;
    let (named, rest) = rest.split_once('`')?;
    let rest = rest.trim_start().strip_prefix('|')?;
    let (text, _) = rest.split_once('|')?;
    let text = text.trim();
    let named = named.trim();
    (!named.is_empty() && !named.contains('|') && !text.is_empty()).then_some((named, text))
}

/// `named`, read from the directory `base`, written the way Python's
/// `pathlib` writes it: no empty or `.` part and no trailing slash.
fn joined(base: &str, named: &str) -> String {
    let absolute = named.starts_with('/');
    let parts: Vec<&str> = if absolute {
        named.split('/').collect()
    } else {
        base.split('/').chain(named.split('/')).collect()
    };
    let path = parts
        .into_iter()
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    if absolute { format!("/{path}") } else { path }
}

/// Each image's alternative text, from a Markdown page that shows it, by the
/// path of the image; a source that is a URL counts through its `/main/`
/// part only.
fn image_alts(root: &Path, paths: &[String]) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let pages = paths.iter().filter(|path| {
        Path::new(path.as_str())
            .extension()
            .is_some_and(|kind| kind == "md")
    });
    for page in pages {
        let text = read(&root.join(page));
        let base = parent_of(page);
        let shown = markdown_images(&text).into_iter().chain(img_tags(&text));
        for (source, alt) in shown {
            let Some(path) = image_path(base, source) else {
                continue;
            };
            if paths.binary_search(&path).is_ok() {
                found.entry(path).or_insert_with(|| sentence(alt));
            }
        }
    }
    found
}

/// The source and alternative text of each Markdown image of `text`: an
/// exclamation mark, the text in brackets, then the source in parentheses.
fn markdown_images(text: &str) -> Vec<(&str, &str)> {
    text.match_indices("![")
        .filter_map(|(at, _)| {
            let rest = text.get(at + 2..)?;
            let close = rest.find(']')?;
            let alt = rest.get(..close).filter(|alt| !alt.is_empty())?;
            let target = rest.get(close + 1..)?.strip_prefix('(')?;
            let end = target
                .find(|character: char| character == ')' || character.is_whitespace())
                .unwrap_or(target.len());
            let source = target.get(..end).filter(|source| !source.is_empty())?;
            Some((source, alt))
        })
        .collect()
}

/// The source and alternative text of each `<img>` tag of `text` that has
/// both.
fn img_tags(text: &str) -> Vec<(&str, &str)> {
    text.match_indices("<img")
        .filter_map(|(at, _)| {
            let rest = text.get(at + 4..)?;
            if rest.starts_with(|character: char| character.is_alphanumeric() || character == '_') {
                return None;
            }
            let (tag, _) = rest.split_once('>')?;
            Some((attribute(tag, "src")?, attribute(tag, "alt")?))
        })
        .collect()
}

/// The value of the double-quoted attribute `name` of `tag`.
fn attribute<'tag>(tag: &'tag str, name: &str) -> Option<&'tag str> {
    let marker = format!("{name}=\"");
    tag.match_indices(&marker)
        .find(|(at, _)| {
            tag.get(..*at).is_some_and(|before| {
                !before.ends_with(|character: char| character.is_alphanumeric() || character == '_')
            })
        })
        .and_then(|(at, _)| tag.get(at + marker.len()..)?.split_once('"'))
        .map(|(value, _)| value)
        .filter(|value| !value.is_empty())
}

/// The repository path an image source names, read from the directory
/// `base`; a URL names one through its `/main/` part only.
fn image_path(base: &str, source: &str) -> Option<String> {
    if source.contains("://") {
        return source.split_once("/main/").map(|(_, path)| path.to_owned());
    }
    Some(joined(base, source))
}

#[cfg(test)]
mod tests {
    use super::{capitalized, img_tags, joined, kept, markdown_images, table_row, tree_lines};
    use std::collections::BTreeMap;
    use std::path::Path;

    #[test]
    fn the_tree_puts_directories_first_and_aligns_every_explanation() {
        let paths = ["README.md", "src/a.rs", "src/deep/b.rs"].map(str::to_owned);
        let previous = BTreeMap::from([
            ("README.md".to_owned(), "The front page".to_owned()),
            ("src".to_owned(), "Sources".to_owned()),
            ("src/deep".to_owned(), "Deeper".to_owned()),
            ("src/a.rs".to_owned(), "A".to_owned()),
            ("src/deep/b.rs".to_owned(), "B".to_owned()),
        ]);
        let lines = tree_lines(Path::new("/nowhere"), &paths, &previous, &BTreeMap::new());
        let expected = [
            (".", "Repository root"),
            ("├── src/", "Sources"),
            ("│   ├── deep/", "Deeper"),
            ("│   │   └── b.rs", "B"),
            ("│   └── a.rs", "A"),
            ("└── README.md", "The front page"),
        ];
        for (line, (entry, text)) in lines.iter().zip(expected) {
            assert_eq!(
                *line,
                format!("{entry}{}# {text}", " ".repeat(48 - entry.chars().count()))
            );
        }
        assert_eq!(lines.len(), expected.len());
    }

    #[test]
    fn kept_explanations_are_read_back_by_their_path() {
        let lines = tree_lines(
            Path::new("/nowhere"),
            &["a/b/c.md".to_owned()],
            &BTreeMap::from([
                ("a".to_owned(), "Top".to_owned()),
                ("a/b".to_owned(), "Middle".to_owned()),
                ("a/b/c.md".to_owned(), "Leaf".to_owned()),
            ]),
            &BTreeMap::new(),
        );
        let guide = format!("# Guide\n\n```text\n{}\n```\n", lines.join("\n"));
        let read = kept(&guide);
        assert_eq!(read.get("a/b/c.md").map(String::as_str), Some("Leaf"));
        assert_eq!(read.get("a/b").map(String::as_str), Some("Middle"));
        assert_eq!(read.get(".").map(String::as_str), Some("Repository root"));
        assert_eq!(read.len(), 4);
    }

    #[test]
    fn readme_rows_and_images_name_repository_paths() {
        assert_eq!(
            table_row("| `docs/` | Where notes live |"),
            Some(("docs/", "Where notes live"))
        );
        assert_eq!(table_row("| docs | plain |"), None);
        assert_eq!(joined("guide", "./a//b/"), "guide/a/b");
        assert_eq!(joined("", "/abs"), "/abs");
        let text = concat!(
            "![Logo]",
            "(assets/logo.png \"t\") <img alt=\"Mark\" src=\"https://x/main/m.png\">"
        );
        assert_eq!(markdown_images(text), [("assets/logo.png", "Logo")]);
        assert_eq!(img_tags(text), [("https://x/main/m.png", "Mark")]);
        assert_eq!(capitalized("ISSUE_TEMPLATE"), "Issue template");
    }
}
