//! `ci.yml`: what every tracked file of a repository holds to, HYG-001 to
//! HYG-007 and the width of shell scripts, each refused by its identifier
//! with its file and line.

use crate::harness::{Fixture, refused, succeeds, tool, write_executable};
use std::fs;
use std::os::unix::fs::symlink;

/// Run git in the fixture's checkout.
fn git(fixture: &Fixture, args: &[&str]) {
    let output = tool("git")
        .args(args)
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    succeeds(&output);
}

/// A fixture whose checkout tracks `files`, each a path and its content, on
/// top of a README and a licence; a path ending in `*` is made executable.
fn checkout(files: &[(&str, &str)]) -> Fixture {
    let fixture = Fixture::new();
    let mut tracked = vec![("README.md", "# Fixture\n"), ("LICENSE", "MIT\n")];
    tracked.extend_from_slice(files);
    git(&fixture, &["init", "-q"]);
    for (path, text) in tracked {
        let executable = path.ends_with('*');
        let path = path.trim_end_matches('*');
        let file = fixture.root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        if executable {
            write_executable(&file, text);
        } else {
            fs::write(&file, text).unwrap();
        }
        git(&fixture, &["add", "--", path]);
    }
    fixture
}

/// The report the step wrote.
fn report(fixture: &Fixture) -> String {
    fs::read_to_string(fixture.root.join("reports/hygiene.txt")).unwrap()
}

#[test]
fn unlinked_markers_in_comments_are_refused_and_linked_ones_pass() {
    let fixture = checkout(&[
        (
            "src/lib.rs",
            "//! Crate.\n// TODO: split this module\n// FIXME(#12): its issue names it\n",
        ),
        (
            "ci.sh*",
            "#!/bin/sh\necho \"# TODO in a string\" # XXX tidy\n",
        ),
    ]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 2 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "HYG-001 ci.sh:2: `XXX` without an issue; link one, `#123` or its URL, or do it now\n",
            "HYG-001 src/lib.rs:2: `TODO` without an issue; link one, `#123` or its URL, or do ",
            "it now\n",
        )
    );
}

#[test]
fn snapshots_large_files_modes_and_links_are_refused() {
    let large = "x".repeat(600 * 1024);
    let fixture = checkout(&[
        ("tests/snapshots/a.snap.new", "pending\n"),
        ("big.bin", &large),
        ("tool*", "echo\n"),
        ("run.sh", "#!/bin/sh\n"),
    ]);
    symlink("/tmp", fixture.root.join("outside")).unwrap();
    symlink("nowhere", fixture.root.join("dangling")).unwrap();
    git(&fixture, &["add", "--", "outside", "dangling"]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 6 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "HYG-003 big.bin: 600 KB, over 500 KB; keep large files out of the repository, or ",
            "record the asset in maestro-quality.toml\n",
            "HYG-004 dangling: a symlink whose target is missing\n",
            "HYG-004 outside: a symlink that leaves the repository\n",
            "HYG-004 run.sh: a shebang without the executable bit; make it executable\n",
            "HYG-002 tests/snapshots/a.snap.new: a pending snapshot; accept or reject it, never ",
            "commit it\n",
            "HYG-004 tool: executable without a shebang; add one or drop the executable bit\n",
        )
    );
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[[exception]]\nrule = \"HYG-003\"\npath = \"big.bin\"\nreason = \"a recorded fixture\"\n",
    )
    .unwrap();
    refused(&fixture.run("ci", "hygiene"), "hygiene: 5 findings");
    assert!(
        report(&fixture).contains("EXCUSED HYG-003 big.bin:"),
        "{}",
        report(&fixture)
    );
}

#[test]
fn a_readme_a_licence_and_a_changelog_are_required() {
    let fixture = checkout(&[(".github/release-please/config.json", "{}\n")]);
    git(&fixture, &["rm", "-qf", "README.md", "LICENSE"]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 3 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "HYG-005 CHANGELOG.md: CHANGELOG.md is missing beside a release-please ",
            "configuration\n",
            "HYG-005 LICENSE: LICENSE is missing; say how others may use the code\n",
            "HYG-005 README.md: README.md is missing; say what the repository is for\n",
        )
    );
}

#[test]
fn wide_shell_lines_and_justfiles_are_refused() {
    let wide = format!("# {}\n", "x".repeat(99));
    let script = format!("#!/bin/sh\necho {}\n", "y".repeat(115));
    let fixture = checkout(&[("justfile", &wide), ("scripts/run.sh*", &script)]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 2 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "SIZE-003 justfile:1: 101 columns, over 100; wrap it, strings and comments included\n",
            "SIZE-003 scripts/run.sh:2: 120 columns, over 100; wrap it, strings and comments ",
            "included\n",
        )
    );
}

#[test]
fn file_names_follow_the_form_of_their_kind() {
    let fixture = checkout(&[
        ("docs/Setup_Guide.md", "Guide\n"),
        ("docs/adr/10-choice.md", "Choice\n"),
        ("docs/adr/0010-choice.md", "Choice\n"),
        ("docs/adr/README.md", "Records\n"),
        ("CODE_OF_CONDUCT.md", "Conduct\n"),
        ("src/fooBar.rs", "//! Foo.\n"),
        (".github/workflows/Build.yaml", "on: push\n"),
        ("scripts/run_all.sh*", "#!/bin/sh\n"),
        ("tools/Lint*", "#!/bin/sh\n"),
        ("scripts/Helper.py", "VALUE = 1\n"),
        ("scripts/__init__.py", ""),
        ("tests/fixtures/Odd_Name.md", "Stand-in\n"),
    ]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 7 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "HYG-006 .github/workflows/Build.yaml: a workflow named `Build.yaml`; name it in ",
            "kebab-case with the extension `.yml`\n",
            "HYG-006 docs/Setup_Guide.md: a Markdown page named `Setup_Guide.md`; name it in ",
            "lowercase kebab-case, or UPPER_SNAKE for a community file such as README\n",
            "HYG-006 docs/adr/10-choice.md: a decision record named `10-choice.md`; name it ",
            "`NNNN-title.md`, the title in kebab-case, or README.md\n",
            "HYG-006 scripts/Helper.py: a Python module named `Helper.py`; name it in snake_case, ",
            "the form Python imports\n",
            "HYG-006 scripts/run_all.sh: a script named `run_all.sh`; name it in lowercase ",
            "kebab-case\n",
            "HYG-006 src/fooBar.rs: a Rust file named `fooBar.rs`; name it in snake_case\n",
            "HYG-006 tools/Lint: a script named `Lint`; name it in lowercase kebab-case\n",
        )
    );
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        concat!(
            "[[exception]]\nrule = \"HYG-006\"\npath = \"scripts/Helper.py\"\n",
            "reason = \"the name a plugin loader imports\"\n",
        ),
    )
    .unwrap();
    refused(&fixture.run("ci", "hygiene"), "hygiene: 6 findings");
    assert!(
        report(&fixture).contains("EXCUSED HYG-006 scripts/Helper.py:"),
        "{}",
        report(&fixture)
    );
}

#[test]
fn words_a_glossary_never_uses_are_refused_everywhere_but_records() {
    let fixture = checkout(&[
        (
            "CONTEXT.md",
            "**Supervisor**: The process that restarts servers.\n_Never_: babysitter\n",
        ),
        (
            "docs/guide.md",
            concat!(
                "Whitel",
                "isted names pass a Sani",
                "ty-Check.\n",
                "See https://x.y/white",
                "list for more.\n",
            ),
        ),
        (
            "src/lib.rs",
            concat!(
                "//! Babysitters restart servers.\nconst BLACK",
                "LISTED: u8 = 0;\n"
            ),
        ),
        ("CHANGELOG.md", concat!("Removed the white", "list.\n")),
        ("docs/adr/0001-names.md", concat!("A black", "list.\n")),
    ]);
    refused(&fixture.run("ci", "hygiene"), "hygiene: 4 findings");
    assert_eq!(
        report(&fixture),
        concat!(
            "HYG-007 docs/guide.md:1: `sani",
            "ty check` is a word the organization's glossary ",
            "never uses; say coherence check\n",
            "HYG-007 docs/guide.md:1: `white",
            "list` is a word the organization's glossary never ",
            "uses; say allowlist\n",
            "HYG-007 src/lib.rs:1: `babysitter` is a word CONTEXT.md never uses; say supervisor\n",
            "HYG-007 src/lib.rs:2: `black",
            "list` is a word the organization's glossary never uses; ",
            "say denylist\n",
        )
    );
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        concat!(
            "[[exception]]\nrule = \"HYG-007\"\npath = \"src/lib.rs\"\nitem = \"black",
            "list\"\n",
            "reason = \"the name a generated binding keeps\"\n",
        ),
    )
    .unwrap();
    // The fixture keeps its reports in its checkout, and the last one quotes
    // the words; a run on GitHub writes them outside it.
    fs::remove_file(fixture.root.join("reports/hygiene.txt")).unwrap();
    refused(&fixture.run("ci", "hygiene"), "hygiene: 3 findings");
    assert!(
        report(&fixture).contains("EXCUSED HYG-007 src/lib.rs:2:"),
        "{}",
        report(&fixture)
    );
}
