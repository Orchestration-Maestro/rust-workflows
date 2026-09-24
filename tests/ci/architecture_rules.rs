//! `ci.yml`: the organization's module structure rules, ARC-001 to ARC-007,
//! each refused by its identifier with its file and line, and the exceptions
//! `maestro-quality.toml` takes, a stale one refused too.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

/// The report the step wrote.
fn report(fixture: &Fixture) -> String {
    fs::read_to_string(fixture.root.join("reports/architecture.txt")).unwrap()
}

#[test]
fn a_project_without_findings_passes_with_an_empty_report() {
    let fixture = Fixture::with_sources(&[
        (
            "src/lib.rs",
            "//! Crate.\nmod parse;\npub use parse::parse;\n",
        ),
        (
            "src/parse.rs",
            concat!(
                "//! Parse.\n/// Parse.\npub fn parse() -> u8 { crate::parse::helper() }\n",
                "fn helper() -> u8 { 1 }\n",
            ),
        ),
    ]);
    succeeds(&fixture.run("ci", "architecture"));
    assert_eq!(report(&fixture), "");
    let summary = fs::read_to_string(fixture.root.join("summary")).unwrap();
    assert!(summary.contains("No finding; 0 excused."), "{summary}");
}

#[test]
fn an_import_cycle_between_two_files_is_refused_by_name() {
    let fixture = Fixture::with_sources(&[
        ("src/lib.rs", "//! Crate.\nmod a;\nmod b;\n"),
        ("src/a.rs", "//! A.\npub(crate) fn f() { crate::b::g() }\n"),
        ("src/b.rs", "//! B.\npub(crate) fn g() { crate::a::f() }\n"),
    ]);
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding; each names its rule, its file and what to do",
    );
    assert_eq!(
        report(&fixture),
        "ARC-001 project/src/a.rs: import cycle project/src/a.rs -> project/src/b.rs -> \
         project/src/a.rs; one of these files must stop naming the next\n"
    );
}

#[test]
fn the_quality_file_refuses_unknown_tables_and_unreasoned_exceptions() {
    let fixture = Fixture::with_sources(&[]);
    let file = fixture.root.join("maestro-quality.toml");
    fs::write(&file, "[typo]\nx = 1\n").unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "maestro-quality.toml: unknown table `typo`; it takes crate, exception, limits, \
         performance, typos",
    );
    fs::write(
        &file,
        "[[exception]]\nrule = \"ARC-001\"\npath = \"project/src/lib.rs\"\nreason = \"x\"\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "ARC-001 takes no exception",
    );
    fs::write(
        &file,
        "[[exception]]\nrule = \"ARC-005\"\npath = \"project/src/lib.rs\"\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "the ARC-005 exception names no path or gives no reason",
    );
    fs::write(
        &file,
        "[[crate]]\nroot = \"project/src/lib.rs\"\nlayers = [\"a\"]\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "a [[crate]] names its root and two layers at least",
    );
}

#[test]
fn a_door_holding_a_function_is_refused_and_a_listing_door_passes() {
    let fixture = Fixture::with_sources(&[
        ("src/lib.rs", "//! Crate.\nmod shapes;\n"),
        (
            "src/shapes/mod.rs",
            concat!(
                "//! Shapes.\nmod circle;\npub(crate) use circle::area;\n\n/// Pi.\n",
                "fn pi() -> f64 { 3.14 }\n",
            ),
        ),
        (
            "src/shapes/circle.rs",
            "//! Circle.\npub(crate) fn area() {}\n",
        ),
    ]);
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding",
    );
    assert_eq!(
        report(&fixture),
        "ARC-002 project/src/shapes/mod.rs:6: a door holds only mod and use declarations; \
         move this fn into a module of its own\n"
    );
}

#[test]
fn a_path_past_a_door_re_export_is_refused() {
    let fixture = Fixture::with_sources(&[
        ("src/lib.rs", "//! Crate.\nmod report;\nmod shapes;\n"),
        (
            "src/report.rs",
            concat!(
                "//! Report.\n",
                "pub(crate) fn print() { crate::shapes::circle::area(); crate::shapes::area(); }\n",
            ),
        ),
        (
            "src/shapes/mod.rs",
            "//! Shapes.\npub(super) mod circle;\npub(super) use circle::area;\n",
        ),
        (
            "src/shapes/circle.rs",
            "//! Circle.\npub(crate) fn area() {}\n",
        ),
    ]);
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding",
    );
    assert_eq!(
        report(&fixture),
        "ARC-003 project/src/report.rs:2: `crate::shapes::circle::area` walks past the door of \
         crate::shapes; name `area` through it\n"
    );
}

#[test]
fn declared_layers_refuse_an_import_within_or_against_the_order() {
    let fixture = Fixture::with_sources(&[
        (
            "src/main.rs",
            "//! Binary.\nmod checks;\nmod runner;\nmod steps;\nmod stray;\nfn main() {}\n",
        ),
        (
            "src/steps.rs",
            "//! Steps.\npub(crate) fn go() { crate::runner::run(); }\n",
        ),
        (
            "src/checks.rs",
            "//! Checks.\npub(crate) fn check() { crate::runner::run(); crate::steps::go(); }\n",
        ),
        ("src/runner.rs", "//! Runner.\npub(crate) fn run() {}\n"),
        ("src/stray.rs", "//! Stray.\n"),
    ]);
    fs::remove_file(fixture.root.join("project/src/lib.rs")).unwrap();
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        concat!(
            "[[crate]]\nroot = \"project/src/main.rs\"\n",
            "layers = [\"steps\", \"checks\", \"runner\", \"missing\"]\n",
        ),
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 3 findings",
    );
    assert_eq!(
        report(&fixture),
        concat!(
            "ARC-004 maestro-quality.toml: the layers of project/src/main.rs name `missing`, ",
            "which that root does not declare\n",
            "ARC-004 project/src/checks.rs:2: `crate::steps::go` imports `steps` from `checks`; ",
            "a layer imports only from layers to its right\n",
            "ARC-004 project/src/main.rs:5: module `stray` sits in no layer maestro-quality.toml ",
            "declares for this root; add it to one\n",
        )
    );
}

#[test]
fn layers_declared_for_a_root_no_target_has_are_refused() {
    let fixture = Fixture::with_sources(&[]);
    fs::write(
        fixture.root.join("maestro-quality.toml"),
        "[[crate]]\nroot = \"project/src/gone.rs\"\nlayers = [\"a\", \"b\"]\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding",
    );
    assert_eq!(
        report(&fixture),
        "ARC-004 maestro-quality.toml: declares layers for project/src/gone.rs, which is no \
         target's root\n"
    );
}

#[test]
fn a_seam_serving_one_outside_caller_is_refused_unless_excused() {
    let fixture = Fixture::with_sources(&[
        (
            "src/lib.rs",
            "//! Crate.\nmod first;\nmod runner;\nmod second;\n",
        ),
        (
            "src/runner/mod.rs",
            "//! Runner.\nmod commands;\npub(crate) use commands::{Cmd, only};\n",
        ),
        (
            "src/runner/commands.rs",
            "//! Commands.\npub(crate) struct Cmd;\npub(crate) fn only() {}\n",
        ),
        (
            "src/first.rs",
            "//! First.\nuse crate::runner::{Cmd, only};\n",
        ),
        ("src/second.rs", "//! Second.\nuse crate::runner::Cmd;\n"),
    ]);
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding",
    );
    assert_eq!(
        report(&fixture),
        "ARC-005 project/src/runner/mod.rs:3: `only` serves only project/src/first.rs; move it \
         next to its caller, or record in maestro-quality.toml why this seam stays\n"
    );

    let excused = "[[exception]]\nrule = \"ARC-005\"\npath = \"project/src/runner/mod.rs\"\n\
                   item = \"only\"\nreason = \"the first module is its one caller by design\"\n";
    fs::write(fixture.root.join("maestro-quality.toml"), excused).unwrap();
    succeeds(&fixture.run("ci", "architecture"));
    assert_eq!(
        report(&fixture),
        "EXCUSED ARC-005 project/src/runner/mod.rs:3: `only` serves only project/src/first.rs; \
         move it next to its caller, or record in maestro-quality.toml why this seam stays \
         (because the first module is its one caller by design)\n"
    );

    fs::write(
        fixture.root.join("project/src/second.rs"),
        "//! Second.\nuse crate::runner::{Cmd, only};\n",
    )
    .unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 1 finding",
    );
    assert_eq!(
        report(&fixture),
        "ARC-005 project/src/runner/mod.rs: the exception for `only` excuses nothing any more; \
         remove it from maestro-quality.toml\n"
    );
}

#[test]
fn a_binary_root_beyond_declarations_and_a_short_main_is_refused() {
    let long_main = format!("fn main() {{\n{}}}\n", "    run();\n".repeat(25));
    let main = format!("//! Binary.\nmod app;\nuse app::run;\n\nstruct Config;\n\n{long_main}");
    let fixture = Fixture::with_sources(&[
        ("src/main.rs", main.as_str()),
        ("src/app.rs", "//! App.\npub(crate) fn run() {}\n"),
    ]);
    fs::remove_file(fixture.root.join("project/src/lib.rs")).unwrap();
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 2 findings",
    );
    assert_eq!(
        report(&fixture),
        concat!(
            "ARC-006 project/src/main.rs:5: a binary root holds only mod and use declarations ",
            "and fn main; move this struct into a module\n",
            "ARC-006 project/src/main.rs:7: fn main spans 27 lines; keep it within 25 and call ",
            "into modules\n",
        )
    );
}

#[test]
fn path_attributes_and_rust_includes_are_refused_while_include_str_passes() {
    let fixture = Fixture::with_sources(&[
        ("src/lib.rs", "//! Crate.\nmod parts;\n"),
        (
            "src/parts.rs",
            concat!(
                "//! Parts.\n#[path = \"elsewhere/inner.rs\"]\nmod inner;\n",
                "include!(\"generated.rs\");\n",
                "pub(crate) const TEXT: &str = include_str!(\"text.txt\");\n",
            ),
        ),
    ]);
    refused(
        &fixture.run("ci", "architecture"),
        "source rules: 2 findings",
    );
    assert_eq!(
        report(&fixture),
        concat!(
            "ARC-007 project/src/parts.rs:2: `#[path]` makes the module tree differ from the ",
            "file tree; move the file where its module is declared\n",
            "ARC-007 project/src/parts.rs:4: `include!` of Rust source hides code from the ",
            "module tree; declare it with `mod`\n",
        )
    );
}
