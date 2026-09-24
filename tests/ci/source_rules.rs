//! `ci.yml`: the source and manifest rules of `rust-gate architecture`,
//! SIZE, NAME, DOC, LIB, TST and WSP, each refused by its identifier with its
//! file and line, and the limits `maestro-quality.toml` may only tighten.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

/// The report the step wrote.
fn report(f: &Fixture) -> String {
    fs::read_to_string(f.root.join("reports/architecture.txt")).unwrap()
}

/// Replace the project's manifest.
fn manifest(f: &Fixture, text: &str) {
    fs::write(f.root.join("project/Cargo.toml"), text).unwrap();
}

#[test]
fn oversized_files_and_lines_are_refused_and_three_hundred_is_reported() {
    let long = format!("//! Long.\n{}", "fn f() {}\n".repeat(501));
    let wide = format!("//! Wide.\nconst X: &str = \"{}\";\n", "x".repeat(100));
    let grown = format!("//! Grown.\n{}", "fn g() {}\n".repeat(301));
    let f = Fixture::with_sources(&[
        (
            "src/lib.rs",
            "//! Crate.\nmod grown;\nmod long;\nmod wide;\n",
        ),
        ("src/long.rs", &long),
        ("src/wide.rs", &wide),
        ("src/grown.rs", &grown),
    ]);
    refused(&f.run("ci", "architecture"), "source rules: 2 findings");
    assert_eq!(
        report(&f),
        concat!(
            "SIZE-002 project/src/long.rs: 501 lines of code, over 500; split it by what ",
            "varies (doc comments are not counted)\n",
            "SIZE-003 project/src/wide.rs:2: 119 columns, over 100; wrap it, strings and ",
            "comments included\n",
            "NOTE SIZE-002 project/src/grown.rs: 301 lines of code, over 300; reported, not ",
            "refused\n",
        )
    );
}

#[test]
fn package_names_follow_the_form_and_publishable_ones_the_prefix() {
    let f = Fixture::with_sources(&[]);
    let named = |name: &str, publish: &str| {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{publish}")
    };
    manifest(&f, &named("router-rs", "publish = false\n"));
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "NAME-001 project/Cargo.toml: package `router-rs` is not lowercase kebab-case without \
         a -rs or -rust suffix\n"
    );
    manifest(&f, &named("router", ""));
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "NAME-001 project/Cargo.toml: package `router` may be published, so its name starts \
         with maestro-; or set publish = false\n"
    );
    manifest(&f, &named("maestro-router", ""));
    succeeds(&f.run("ci", "architecture"));
}

#[test]
fn badly_named_tests_and_one_word_test_modules_are_refused() {
    let f = Fixture::with_sources(&[
        ("tests/it/main.rs", "//! Tests.\nmod parser;\n"),
        (
            "tests/it/parser.rs",
            "//! Parser.\n#[test]\nfn short_name() {}\n",
        ),
    ]);
    refused(&f.run("ci", "architecture"), "source rules: 2 findings");
    assert_eq!(
        report(&f),
        concat!(
            "NAME-002 project/tests/it/parser.rs: test module `parser` names what it proves in ",
            "fewer than two words\n",
            "NAME-002 project/tests/it/parser.rs:3: test `short_name` names what it proves in ",
            "fewer than four words\n",
        )
    );
}

#[test]
fn a_file_without_a_module_comment_is_refused() {
    let f = Fixture::with_sources(&[
        ("src/lib.rs", "//! Crate.\nmod bare;\n"),
        ("src/bare.rs", "pub(crate) fn f() {}\n"),
    ]);
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "DOC-001 project/src/bare.rs: the file does not open with a //! comment saying what the \
         module is for\n"
    );
}

#[test]
fn printing_from_a_library_is_refused_and_from_a_binary_allowed() {
    let f = Fixture::with_sources(&[
        (
            "src/lib.rs",
            "//! Crate.\npub fn f() {\n    println!(\"x\");\n}\n",
        ),
        (
            "src/main.rs",
            "//! Binary.\nfn main() {\n    println!(\"y\");\n}\n",
        ),
    ]);
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "LIB-001 project/src/lib.rs:3: `println!` in a library; return the value or report \
         through tracing, and let the binary print\n"
    );
}

#[test]
fn a_library_depending_on_anyhow_is_refused_but_a_binary_is_not() {
    let f = Fixture::with_sources(&[]);
    manifest(
        &f,
        "[package]\nname = \"maestro-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nanyhow = \"1\"\n",
    );
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "LIB-002 project/Cargo.toml: a library exposes typed errors; `anyhow` belongs in a \
         binary or in [dev-dependencies]\n"
    );
    fs::write(
        f.root.join("project/src/main.rs"),
        "//! Binary.\nfn main() {}\n",
    )
    .unwrap();
    succeeds(&f.run("ci", "architecture"));
}

#[test]
fn sleeping_in_a_test_is_refused_unless_excused() {
    let f = Fixture::with_sources(&[(
        "src/lib.rs",
        concat!(
            "//! Crate.\n#[cfg(test)]\nmod tests {\n    #[test]\n",
            "    fn the_reply_arrives_in_time() {\n",
            "        std::thread::sleep(std::time::Duration::from_millis(1));\n    }\n}\n",
        ),
    )]);
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "TST-001 project/src/lib.rs:6: `std::thread::sleep` waits on time; wait on a fake clock \
         or a synchronisation primitive\n"
    );
    fs::write(
        f.root.join("maestro-quality.toml"),
        "[[exception]]\nrule = \"TST-001\"\npath = \"project/src/lib.rs\"\n\
         item = \"the_reply_arrives_in_time\"\nreason = \"it measures a real timeout\"\n",
    )
    .unwrap();
    succeeds(&f.run("ci", "architecture"));
    assert!(
        report(&f).starts_with("EXCUSED TST-001 project/src/lib.rs:6:"),
        "{}",
        report(&f)
    );
}

#[test]
fn more_than_one_plain_integration_test_crate_is_refused() {
    let f = Fixture::with_sources(&[
        ("tests/first.rs", "//! First.\n"),
        ("tests/second.rs", "//! Second.\n"),
    ]);
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "TST-003 project/Cargo.toml: 2 integration-test crates build without required-features \
         (first, second); fold them into one test crate, tests/it/main.rs with its modules beside \
         it\n"
    );
}

#[test]
fn workspace_members_inherit_their_settings_and_dependencies() {
    let f = Fixture::with_sources(&[
        ("a/src/lib.rs", "//! A.\n"),
        ("b/src/lib.rs", "//! B.\n"),
        (
            "a/Cargo.toml",
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition.workspace = true\n\
             publish = false\n\n[dependencies]\nb = { path = \"../b\" }\n",
        ),
        (
            "b/Cargo.toml",
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition.workspace = true\n\
             rust-version.workspace = true\nlicense.workspace = true\npublish = false\n\n\
             [lints]\nworkspace = true\n",
        ),
    ]);
    manifest(
        &f,
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"3\"\n\n[workspace.package]\n\
         edition = \"2024\"\nrust-version = \"1.85\"\nlicense = \"MIT\"\n\n\
         [workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
    );
    refused(&f.run("ci", "architecture"), "source rules: 2 findings");
    assert_eq!(
        report(&f),
        concat!(
            "WSP-001 project/a/Cargo.toml: member `a` declares b without `workspace = true`; ",
            "declare them in [workspace.dependencies]\n",
            "WSP-001 project/a/Cargo.toml: member `a` does not inherit [lints], rust-version, ",
            "license from the workspace\n",
        )
    );
}

#[test]
fn edition_resolver_and_lockfile_are_held() {
    let f = Fixture::with_sources(&[]);
    manifest(
        &f,
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n",
    );
    fs::remove_file(f.root.join("project/Cargo.lock")).unwrap();
    refused(&f.run("ci", "architecture"), "source rules: 2 findings");
    assert_eq!(
        report(&f),
        concat!(
            "WSP-002 project/Cargo.lock: Cargo.lock is missing; commit it\n",
            "WSP-002 project/Cargo.toml: package `fixture` uses edition 2021; the organization ",
            "builds with 2024\n",
        )
    );
    fs::write(f.root.join("project/Cargo.lock"), "version = 4\n").unwrap();
    fs::create_dir_all(f.root.join("project/a/src")).unwrap();
    fs::write(f.root.join("project/a/src/lib.rs"), "//! A.\n").unwrap();
    fs::write(
        f.root.join("project/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n",
    )
    .unwrap();
    manifest(&f, "[workspace]\nmembers = [\"a\"]\nresolver = \"2\"\n");
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "WSP-002 project/Cargo.toml: the workspace sets resolver 2; set resolver = \"3\"\n"
    );
}

#[test]
fn tightened_limits_apply_and_loosened_ones_are_refused() {
    let wide = format!("//! Crate.\nconst X: &str = \"{}\";\n", "x".repeat(31));
    let f = Fixture::with_sources(&[("src/lib.rs", &wide)]);
    let limits = f.root.join("maestro-quality.toml");
    fs::write(&limits, "[limits]\nline-columns = 40\n").unwrap();
    refused(&f.run("ci", "architecture"), "source rules: 1 finding");
    assert_eq!(
        report(&f),
        "SIZE-003 project/src/lib.rs:2: 50 columns, over 40; wrap it, strings and comments \
         included\n"
    );
    fs::write(&limits, "[limits]\nline-columns = 120\n").unwrap();
    refused(
        &f.run("ci", "architecture"),
        "maestro-quality.toml: line-columns = 120 loosens the organization's 100; a repository \
         may only tighten it",
    );
}
