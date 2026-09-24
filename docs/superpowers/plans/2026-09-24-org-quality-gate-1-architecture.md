# Organization quality gate, plan 1 of 8: the architecture rules

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rust-gate architecture` refuses ARC-001 to ARC-007 in any Cargo project, runs in `ci.yml` behind the temporary `quality-preview` input, and holds rust-workflows itself through `just check`.

**Architecture:** A standard-library reader in the checks layer blanks comments and literals, lists top-level items, expands paths and builds every Cargo target's module tree from `cargo metadata`. A step directory, `gate/src/steps/architecture`, runs one module per group of rules over those trees, applies the exceptions `maestro-quality.toml` takes, and writes one report line per finding. rust-workflows declares its own layers and exceptions and runs the step on every project it holds.

**Tech Stack:** Rust 1.98.1, edition 2024, standard library only in the gate; jaq 3.1.1 reads TOML and JSON; `cargo metadata --no-deps --offline`; contract tests in the workflows test crate.

**Spec:** docs/superpowers/specs/2026-09-24-org-quality-gate-design.md, sections 5.1, 6, 7 and 10.

## Global Constraints

- The gate stays standard-library only (docs/rust-gate.md, invariant 1): TOML and JSON go through the pinned jaq.
- Never commit unless `timeout 1800 just check` exits 0; commits are signed; titles are conventional, lowercase after the type, subject within 71 characters, every line within 80 columns.
- One pull request for this plan, on branch `feat/org-quality-gate`, which already holds the spec.
- Every item of the gate carries a `///` doc comment, private ones included (`missing_docs_in_private_items` is denied in `gate/src/main.rs`).
- Clippy pedantic with `-D warnings`; functions within 100 lines, cognitive complexity 15, 5 parameters; files within 500 lines, over 300 reported; lines within 100 columns.
- Every refusal a gate function composes (a string literal of 12 characters or more inside `Err(`, `Failure::from(`, `.map_err(`, `.ok_or(`) is asserted, in its own words, by a test.
- Every module under `gate/src/checks/`, and every file inside a step directory other than its `mod.rs`, that defines a function carries unit tests.
- A registered step is run by a contract test, `f.run("ci", "<id>")`, and the workflow step's `name:` equals the step's `summary`.
- New gate steps land behind `if: ${{ inputs.quality-preview }}` (default `false`): this repository's CI still runs the previously pinned gate, which does not know them. Plan 6 removes the input and repins.
- In Markdown under this repository, a backticked path beginning with a top-level directory of this repository, or ending with `/`, must exist; write paths that do not exist yet without backticks.
- Before each commit: `export PATH=$PWD/.tools/bin:$PATH MISE_TRUSTED_CONFIG_PATHS=$PWD`, then `just docs` when a workflow, the step list or `docs/gates.toml` changed.

## The eight plans

| Plan | Delivers | Estimate |
| --- | --- | --- |
| 1, this one | ARC-001 to ARC-007, the reader they share, rust-workflows held to them | 1.5 days |
| 2 | SIZE, NAME, DOC-001, LIB, TST-001, TST-003 and WSP on the same reader; the remaining internal quality tests removed | 1.5 days |
| 3 | `rust-gate hygiene`: HYG-001 to HYG-005, DOC-002, DOC-003, DUP-001 enforced; lychee, rumdl, shfmt and editorconfig-checker pinned; the speed test of spec section 10 | 1.5 days |
| 4 | LNT-001, the generated files, `sync`, `init`, `managed-files`, the hooks manifest, `hygiene.yml`, TST-004 | 2 days |
| 5 | COV-001, COV-002, PRL-001, PRL-002, DEP-001, VET-001, PRF-001 | 2 days |
| 6 | Repin the gate, remove `quality-preview`, scorecard controls, release v2.0.0 after confirmation | 0.5 day |
| 7 | `.github`: `stack` property, `hygiene-required` ruleset after confirmation, `quality-sync.yml`, audit | 1 day |
| 8 | maestro-model-router, maestro-core and release-canary sync pull requests | 2 to 3 days |

Each later plan is written when the previous one is merged, from the code as it then is.

## File structure

| File | Responsibility |
| --- | --- |
| `gate/src/steps/registry.rs` | Every step's declaration in workflow order; `run` and `describe`, moved out of the steps door |
| `gate/src/steps/quality_scorecard/step.rs` | The scorecard step, moved out of its directory's door |
| `gate/src/checks/rust_code.rs` | Blank comments and literals; list top-level items; line numbers |
| `gate/src/checks/rust_paths.rs` | Expand `use` trees; find `a::b` chains |
| `gate/src/checks/module_tree.rs` | Cargo targets, their module trees, path resolution |
| `gate/src/checks/quality_config.rs` | Read `maestro-quality.toml` through jaq |
| `gate/src/checks/findings.rs` | The finding type, its report line, exceptions and stale exceptions |
| `gate/src/steps/architecture/step.rs` | The step: trees, rules, exceptions, report |
| `gate/src/steps/architecture/cycles.rs` | ARC-001 |
| `gate/src/steps/architecture/doors.rs` | ARC-002, ARC-003 |
| `gate/src/steps/architecture/layers.rs` | ARC-004 |
| `gate/src/steps/architecture/seams.rs` | ARC-005 |
| `gate/src/steps/architecture/roots.rs` | ARC-006, ARC-007 |
| tests/ci/architecture_rules.rs | Contract tests: each rule refused through the real step |
| maestro-quality.toml | rust-workflows' own layers and exceptions |

---

### Task 1: Doors that only declare, in the gate itself

The steps door holds the registry and the scorecard's door holds its step, both of which ARC-002 will refuse. Moving them first is a refactor with a green commit of its own.

**Files:**
- Create: `gate/src/steps/registry.rs`
- Modify: `gate/src/steps/mod.rs` (whole file)
- Create: `gate/src/steps/quality_scorecard/step.rs` (moved from its `mod.rs`)
- Modify: `gate/src/steps/quality_scorecard/mod.rs` (whole file)
- Modify: tests/gate/step_registry.rs, tests/gate/layer_boundaries.rs, justfile, docs/rust-gate.md, .github/copilot-instructions.md

**Interfaces:**
- Produces: `crate::steps::run(command: &str, step: &str) -> Outcome` and `crate::steps::describe() -> String`, unchanged for `main.rs`; directory steps declare `pub(in crate::steps) const STEPS: &[Step]` and their door re-exports it with `pub(super) use step::STEPS;`.

- [ ] **Step 1: Create the registry module**

`gate/src/steps/registry.rs`:

```rust
//! The registry of every step: the declarations in the order the workflows
//! run them, and the two doors `main.rs` calls, `run` and `describe`.

use super::{
    api_compatibility, attest_binaries, binary_hardening, configure_cargo_registry,
    declared_msrv, dependency_policy, feature_combinations, format_lint_test, fuzz_regression,
    install_toolchain, install_tools, line_coverage, mutation_testing, publish_binaries,
    publish_crate, publish_evidence, quality_scorecard, recorded_audits, release_build,
    report_duplicates, report_sizes, require_every_check, secret_scan, stage_payload,
    unsafe_audit, unused_dependencies, validate_inputs, verify_payload, vulnerability_audit,
};
use crate::runner::{Failure, Outcome, Step, enter};
use std::fmt::Write as _;

/// Every step of every workflow, in the order the workflows run them:
/// `ci.yml` first, then the commands several workflows share, then the other
/// workflows.
const REGISTRY: &[&[Step]] = &[
    validate_inputs::STEPS,
    configure_cargo_registry::STEPS,
    install_toolchain::STEPS,
    format_lint_test::STEPS,
    report_sizes::STEPS,
    report_duplicates::STEPS,
    line_coverage::STEPS,
    vulnerability_audit::STEPS,
    dependency_policy::STEPS,
    mutation_testing::STEPS,
    api_compatibility::STEPS,
    secret_scan::STEPS,
    declared_msrv::STEPS,
    feature_combinations::STEPS,
    unused_dependencies::STEPS,
    recorded_audits::STEPS,
    release_build::STEPS,
    binary_hardening::STEPS,
    stage_payload::STEPS,
    quality_scorecard::STEPS,
    require_every_check::STEPS,
    install_tools::STEPS,
    verify_payload::STEPS,
    publish_binaries::STEPS,
    publish_crate::STEPS,
    publish_evidence::STEPS,
    attest_binaries::STEPS,
    fuzz_regression::STEPS,
    unsafe_audit::STEPS,
];
```

Then append, unchanged, the four items that follow `REGISTRY` in the current `gate/src/steps/mod.rs`: `fn steps()`, `pub(crate) fn run`, `pub(crate) fn describe` and `fn cell`, with their doc comments.

- [ ] **Step 2: Reduce the steps door to declarations**

`gate/src/steps/mod.rs`, the whole file:

```rust
//! One module per step, private to this directory: a step reaches the runner
//! and the checks, never another step. Each module declares its steps as
//! data, `STEPS`; the registry lists them and holds `run` and `describe`, the
//! two doors in.

mod api_compatibility;
mod attest_binaries;
mod binary_hardening;
mod configure_cargo_registry;
mod declared_msrv;
mod dependency_policy;
mod feature_combinations;
mod format_lint_test;
mod fuzz_regression;
mod install_toolchain;
mod install_tools;
mod line_coverage;
mod mutation_testing;
mod publish_binaries;
mod publish_crate;
mod publish_evidence;
mod quality_scorecard;
mod recorded_audits;
mod registry;
mod release_build;
mod report_duplicates;
mod report_sizes;
mod require_every_check;
mod secret_scan;
mod stage_payload;
mod unsafe_audit;
mod unused_dependencies;
mod validate_inputs;
mod verify_payload;
mod vulnerability_audit;

pub(crate) use registry::{describe, run};
```

- [ ] **Step 3: Move the scorecard step out of its door**

1. `git mv gate/src/steps/quality_scorecard/mod.rs gate/src/steps/quality_scorecard/step.rs`
2. In `step.rs`: delete the line `mod scorecard;` and the blank line after it; replace `use self::scorecard::{Control, Scorecard, State};` with `use super::scorecard::{Control, Scorecard, State};`; replace `pub(crate) const STEPS: &[Step]` with `pub(in crate::steps) const STEPS: &[Step]`.
3. Create `gate/src/steps/quality_scorecard/mod.rs`:

```rust
//! `rust-gate scorecard`: what this run actually enforced, as data, prose and
//! a self-contained badge. The step lives in `step.rs`, the value it renders
//! in `scorecard.rs`.

mod scorecard;
mod step;

pub(super) use step::STEPS;
```

- [ ] **Step 4: Teach the repository tests the new shape**

1. tests/gate/step_registry.rs: replace each of the three occurrences of `"pub(crate) const STEPS"` with `"const STEPS: &[Step]"`, so a directory step's `pub(in crate::steps)` declaration is found; and replace `if module == "mod.rs" {` with `if module == "mod.rs" || module == "registry.rs" {`.
2. tests/gate/layer_boundaries.rs, in `imports_flow_one_way_through_the_layer_gates`: replace `if layer != "steps" || name == "mod.rs" {` with:

```rust
            // The registry is the steps door's implementation: the one module
            // that names every step, and the only one allowed to.
            if layer != "steps" || name == "mod.rs" || name == "registry.rs" {
```

3. justfile, in `_tables`: replace `gate/src/steps/quality_scorecard/mod.rs` with `gate/src/steps/quality_scorecard/step.rs`.
4. docs/rust-gate.md: replace `` `quality_scorecard/mod.rs` is the step and `` with `` `quality_scorecard/step.rs` is the step and ``, and replace `` `steps/mod.rs` holds the registry and exposes `run` and `describe`, the two `` with `` `steps/registry.rs` holds the registry and `run` and `describe`, the two ``.
5. .github/copilot-instructions.md: under the steps directory, change the `mod.rs` line's description to `One module per step, the registry among them; run and describe are its doors`, add after `recorded_audits.rs`:

```text
│   │   │   ├── registry.rs                     # Every step's declaration in workflow order, and the two doors main.rs calls
```

and under the `quality_scorecard` directory make the three lines:

```text
│   │   │   │   ├── mod.rs                      # The step's door: its two modules and its declaration
│   │   │   │   ├── scorecard.rs                # A run's scorecard as a value: its controls, and the JSON, Markdown and badge of them
│   │   │   │   └── step.rs                     # rust-gate scorecard: what ran, as JSON, Markdown and a self-contained badge
```

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline` then `timeout 1800 just check > /tmp/just-check.log; echo $?`
Expected: `0`; the gate's unit tests and all contract tests pass, docs/steps.md unchanged.

```bash
git add -A gate/src/steps tests/gate justfile docs/rust-gate.md .github/copilot-instructions.md
git commit -S -m "refactor: keep the gate's doors to declarations"
```

---

### Task 2: Rust source with comments and literals blanked

**Files:**
- Create: `gate/src/checks/rust_code.rs`
- Modify: `gate/src/checks/mod.rs` (add `pub(crate) mod rust_code;` after `pub(crate) mod release_boundary;`)

**Interfaces:**
- Produces: `pub(crate) struct Item { line: usize, attributes: String, visibility: String, kind: String, name: String, text: String, span: (usize, usize) }` with `is_test()`, `is_offered()`, `is_module_file()`; `blanked(&str) -> String`, `blank(&mut [u8])`, `items(&str) -> Vec<Item>`, `without_tests(&str) -> String`, `line_at(&str, usize) -> usize`, `is_identifier_byte(u8) -> bool`, all `pub(crate)`.

- [ ] **Step 1: Write the failing tests**

Create `gate/src/checks/rust_code.rs` holding only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::{blanked, items, without_tests};

    #[test]
    fn comments_and_literals_blank_to_spaces_on_their_own_lines() {
        let source = concat!(
            "a // crate::x\n",
            "b /* crate::y /* nested */ crate::z */ c\n",
            "d \"crate::s \\\" crate::t\" r#\"crate::u\"# b\"crate::v\"\n",
            "e 'x' '\\'' '\\u{e9}' 'é' '\"' f\n",
        );
        let code = blanked(source);
        assert_eq!(code.len(), source.len());
        assert_eq!(code.lines().count(), 4);
        assert!(!code.contains("crate"), "{code}");
        assert_eq!(
            code.split_whitespace().collect::<Vec<_>>(),
            ["a", "b", "c", "d", "e", "f"]
        );
    }

    #[test]
    fn lifetimes_labels_and_raw_identifiers_stay_code() {
        let source = "fn f<'a>(x: &'a str) -> &'static str { 'outer: loop { break 'outer r#type(x); } }";
        assert_eq!(blanked(source), source);
    }

    #[test]
    fn top_level_items_carry_attributes_visibility_kind_and_name() {
        let code = blanked(concat!(
            "//! Door.\n",
            "#![forbid(unsafe_code)]\n",
            "mod a;\n",
            "pub(crate) mod b;\n",
            "pub(in crate::x) use a::{C, D};\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    fn t() {}\n",
            "}\n",
            "const X: [u8; 2] = [1, 2];\n",
            "pub(crate) const fn f() -> S { S {} }\n",
            "extern crate alloc;\n",
            "thread_local! { static Y: u8 = 0; }\n",
            "impl S { fn g(&self) {} }\n",
        ));
        let found = items(&code);
        let heads: Vec<(usize, &str, &str, &str)> = found
            .iter()
            .map(|item| {
                (item.line, item.visibility.as_str(), item.kind.as_str(), item.name.as_str())
            })
            .collect();
        assert_eq!(
            heads,
            [
                (3, "", "mod", "a"),
                (4, "pub(crate)", "mod", "b"),
                (5, "pub(in crate::x)", "use", ""),
                (7, "", "mod", "tests"),
                (10, "", "const", "X"),
                (11, "pub(crate)", "fn", "f"),
                (12, "", "extern crate", "alloc"),
                (13, "", "macro", ""),
                (14, "", "impl", ""),
            ]
        );
        assert!(found[3].is_test() && !found[0].is_test());
        assert!(found[0].is_module_file() && !found[3].is_module_file());
        assert!(found[1].is_offered() && found[2].is_offered() && !found[0].is_offered());
        assert!(found[0].attributes.contains("forbid"));
    }

    #[test]
    fn test_items_leave_the_code_the_import_graph_reads() {
        let code = blanked("use crate::a;\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n");
        let kept = without_tests(&code);
        assert!(kept.contains("crate::a") && !kept.contains("super"), "{kept}");
        assert_eq!(kept.lines().count(), code.lines().count());
    }
}
```

Add `pub(crate) mod rust_code;` to `gate/src/checks/mod.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline rust_code`
Expected: compile error, `unresolved imports super::blanked, super::items, super::without_tests`.

- [ ] **Step 3: Write the implementation above the test module**

```rust
//! Rust source as the architecture rules read it: every comment and every
//! literal blanked to spaces, so a path inside a string or a comment is never
//! read as an import, each line where it was; and the top-level items of a
//! file, each with its attributes, visibility, kind and name.

/// One top-level item of a file.
#[derive(Debug)]
pub(crate) struct Item {
    /// The line its first word is on, counting from 1.
    pub(crate) line: usize,
    /// The attributes written on it, `#[cfg(test)]` among them.
    pub(crate) attributes: String,
    /// Its visibility, whitespace collapsed: empty, `pub`, `pub(crate)`,
    /// `pub(super)` or `pub(in crate::path)`.
    pub(crate) visibility: String,
    /// What it is: `mod`, `use`, `fn`, `struct`, `extern crate`,
    /// `macro_rules!`, or `macro` for any other macro called at the top level.
    pub(crate) kind: String,
    /// The name it declares; empty for `use`, `impl` and macro calls.
    pub(crate) name: String,
    /// Its text, from its first word to its end.
    pub(crate) text: String,
    /// Where it starts, attributes included, and where it ends, in bytes.
    pub(crate) span: (usize, usize),
}

impl Item {
    /// Whether the item exists only under test: `#[cfg(test)]` on it.
    pub(crate) fn is_test(&self) -> bool {
        self.attributes
            .replace(char::is_whitespace, "")
            .contains("cfg(test)")
    }

    /// Whether its visibility reaches past the parent module: `pub`,
    /// `pub(crate)` or `pub(in path)`, but neither `pub(super)` nor
    /// `pub(self)`.
    pub(crate) fn is_offered(&self) -> bool {
        self.visibility.starts_with("pub")
            && !matches!(
                self.visibility.as_str(),
                "pub(super)" | "pub(self)" | "pub(in super)" | "pub(in self)"
            )
    }

    /// Whether it declares a module kept in a file of its own: `mod name;`.
    pub(crate) fn is_module_file(&self) -> bool {
        self.kind == "mod" && self.text.trim_end().ends_with(';')
    }
}

/// The source with every comment and every string, byte-string, C-string,
/// raw-string and character literal replaced by spaces. Line breaks stay, so
/// every byte offset and every line number is the original's.
pub(crate) fn blanked(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut index = 0;
    while index < bytes.len() {
        match literal_end(bytes, index) {
            Some(end) => {
                blank(out.get_mut(index..end).unwrap_or_default());
                index = end;
            }
            None => index += 1,
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Turn every byte but a line break into a space.
pub(crate) fn blank(bytes: &mut [u8]) {
    for byte in bytes.iter_mut().filter(|byte| **byte != b'\n') {
        *byte = b' ';
    }
}

/// Where the comment or literal starting at `start` ends, or `None` when
/// none starts there.
fn literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let rest = bytes.get(start..)?;
    if rest.starts_with(b"//") {
        let length = rest.iter().position(|&byte| byte == b'\n');
        return Some(start + length.unwrap_or(rest.len()));
    }
    if rest.starts_with(b"/*") {
        return Some(block_comment_end(bytes, start));
    }
    if start > 0 && bytes.get(start - 1).is_some_and(|&byte| is_identifier_byte(byte)) {
        return None;
    }
    let (prefix, raw) = match rest {
        [b'b' | b'c', b'r', ..] => (2, true),
        [b'r', ..] => (1, true),
        [b'b' | b'c', ..] => (1, false),
        _ => (0, false),
    };
    if raw {
        return raw_string_end(bytes, start + prefix);
    }
    match bytes.get(start + prefix) {
        Some(b'"') => Some(string_end(bytes, start + prefix)),
        Some(b'\'') => char_end(bytes, start + prefix),
        _ => None,
    }
}

/// The end of the block comment opening at `start`, nested ones included.
fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 0usize;
    let mut index = start;
    while let (Some(&first), Some(&second)) = (bytes.get(index), bytes.get(index + 1)) {
        match (first, second) {
            (b'/', b'*') => {
                depth += 1;
                index += 2;
            }
            (b'*', b'/') => {
                depth = depth.saturating_sub(1);
                index += 2;
                if depth == 0 {
                    return index;
                }
            }
            _ => index += 1,
        }
    }
    bytes.len()
}

/// The end of the raw string whose hashes or quote start at `at`, or `None`
/// when `at` starts a raw identifier such as `r#type`.
fn raw_string_end(bytes: &[u8], at: usize) -> Option<usize> {
    let tail = bytes.get(at..)?;
    let hashes = tail.iter().take_while(|&&byte| byte == b'#').count();
    if tail.get(hashes) != Some(&b'"') {
        return None;
    }
    let mut closing = vec![b'"'];
    closing.extend(std::iter::repeat_n(b'#', hashes));
    let body = at + hashes + 1;
    let found = bytes
        .get(body..)?
        .windows(closing.len())
        .position(|window| window == closing.as_slice());
    Some(found.map_or(bytes.len(), |offset| body + offset + closing.len()))
}

/// The end of the string whose opening quote is at `at`: past the first
/// quote no backslash escapes.
fn string_end(bytes: &[u8], at: usize) -> usize {
    let mut index = at + 1;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// The end of the character literal opening at `at`, or `None` when the
/// quote starts a lifetime or a label such as `'a` or `'static`.
fn char_end(bytes: &[u8], at: usize) -> Option<usize> {
    let first = *bytes.get(at + 1)?;
    if first == b'\\' {
        let close = bytes.get(at + 3..)?.iter().position(|&byte| byte == b'\'')?;
        return Some(at + 3 + close + 1);
    }
    let width = match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    };
    (bytes.get(at + 1 + width) == Some(&b'\'')).then_some(at + 2 + width)
}

/// Whether a byte can continue an identifier: ASCII letters, digits, `_`,
/// and any byte of a non-ASCII character.
pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// The line, counting from 1, that byte `offset` of `text` is on.
pub(crate) fn line_at(text: &str, offset: usize) -> usize {
    text.as_bytes()
        .get(..offset)
        .unwrap_or_default()
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

/// The top-level items of blanked code, in order.
pub(crate) fn items(code: &str) -> Vec<Item> {
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(start) = next_word(code, index) {
        let Some(first) = next_word(code, attributes_end(code, start)) else {
            break;
        };
        let (visibility, kind, name) = head(code.get(first..).unwrap_or_default());
        let end = item_end(code.as_bytes(), first, ends_at_semicolon(&kind));
        found.push(Item {
            line: line_at(code, first),
            attributes: code.get(start..first).unwrap_or_default().trim().to_owned(),
            visibility,
            kind,
            name,
            text: code.get(first..end).unwrap_or_default().to_owned(),
            span: (start, end),
        });
        index = end;
    }
    found
}

/// Blanked code with its `#[cfg(test)]` items blanked too: what the import
/// graph reads, since a test names its own module's items without that being
/// a dependency.
pub(crate) fn without_tests(code: &str) -> String {
    let mut out = code.as_bytes().to_vec();
    for item in items(code).iter().filter(|item| item.is_test()) {
        blank(out.get_mut(item.span.0..item.span.1).unwrap_or_default());
    }
    String::from_utf8(out).unwrap_or_default()
}

/// The offset of the first non-whitespace byte at or after `from`.
fn next_word(code: &str, from: usize) -> Option<usize> {
    code.get(from..)?
        .find(|c: char| !c.is_whitespace())
        .map(|offset| from + offset)
}

/// Where the attributes starting at `start` end: past every `#[...]` and
/// `#![...]`, and the whitespace after each.
fn attributes_end(code: &str, start: usize) -> usize {
    let bytes = code.as_bytes();
    let mut index = start;
    loop {
        let rest = bytes.get(index..).unwrap_or_default();
        let open = if rest.starts_with(b"#[") {
            1
        } else if rest.starts_with(b"#![") {
            2
        } else {
            return index;
        };
        let mut depth = 0usize;
        let mut cursor = index + open;
        while let Some(&byte) = bytes.get(cursor) {
            cursor += 1;
            match byte {
                b'[' => depth += 1,
                b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        index = next_word(code, cursor).unwrap_or(bytes.len());
    }
}

/// The visibility, kind and name an item's text opens with.
fn head(text: &str) -> (String, String, String) {
    let (visibility, rest) = visibility_of(text);
    let mut words = rest
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '!'))
        .filter(|word| !word.is_empty())
        .take(8)
        .peekable();
    while let Some(word) = words.next() {
        let next = words.peek().copied().unwrap_or_default();
        if word == "extern" && next == "crate" {
            words.next();
            let name = words.next().unwrap_or_default();
            return (visibility, "extern crate".to_owned(), name.to_owned());
        }
        let qualifier = matches!(word, "unsafe" | "async" | "default" | "safe" | "extern")
            || (word == "const" && matches!(next, "fn" | "unsafe" | "async" | "extern"));
        if qualifier {
            continue;
        }
        let call = word.ends_with('!') && word != "macro_rules!";
        let kind = if call { "macro" } else { word };
        let name = if call || matches!(word, "use" | "impl") { "" } else { next };
        return (visibility, kind.to_owned(), name.to_owned());
    }
    (visibility, String::new(), String::new())
}

/// The visibility an item's text opens with, whitespace collapsed, and
/// the text after it.
fn visibility_of(text: &str) -> (String, &str) {
    let Some(after) = text.strip_prefix("pub") else {
        return (String::new(), text);
    };
    if after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
        return (String::new(), text);
    }
    let after = after.trim_start();
    let Some(group) = after.strip_prefix('(') else {
        return ("pub".to_owned(), after);
    };
    let close = group.find(')').unwrap_or(group.len());
    let inner = group
        .get(..close)
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (format!("pub({inner})"), group.get(close + 1..).unwrap_or_default())
}

/// Whether an item of `kind` ends at its `;` even after a braced value, as
/// `const X: S = S { a: 1 };` does.
fn ends_at_semicolon(kind: &str) -> bool {
    matches!(kind, "const" | "static" | "type" | "use" | "extern crate")
}

/// Where the item whose first word is at `from` ends: past its `;` at depth
/// zero, or past the `}` that brings a braced item back to depth zero.
fn item_end(bytes: &[u8], from: usize, at_semicolon: bool) -> usize {
    let mut depth = 0usize;
    for (offset, &byte) in bytes.get(from..).unwrap_or_default().iter().enumerate() {
        match byte {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && byte == b'}' && !at_semicolon {
                    return from + offset + 1;
                }
            }
            b';' if depth == 0 => return from + offset + 1,
            _ => {}
        }
    }
    bytes.len()
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline rust_code`
Expected: 4 passed. Dead-code warnings are expected until Task 6 gives these functions a caller; do not commit yet.

---

### Task 3: The paths a file names

**Files:**
- Create: `gate/src/checks/rust_paths.rs`
- Modify: `gate/src/checks/mod.rs` (add `pub(crate) mod rust_paths;` after `pub(crate) mod rust_code;`: its `NamedPath` appears in `Module`'s fields, so a private module would trip `private_interfaces`)

**Interfaces:**
- Consumes: `rust_code::{blank, is_identifier_byte, line_at}`.
- Produces: `pub(crate) struct NamedPath { line: usize, segments: Vec<String> }` (derives `Debug, PartialEq, Eq`); `paths(code: &str) -> Vec<NamedPath>`; `use_leaves(declaration: &str) -> Vec<Vec<String>>`.

- [ ] **Step 1: Write the failing tests**

Create `gate/src/checks/rust_paths.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::{paths, use_leaves};

    /// Owned segments, for comparing with what the reader returns.
    fn owned(paths: &[&[&str]]) -> Vec<Vec<String>> {
        paths
            .iter()
            .map(|path| path.iter().map(|segment| (*segment).to_owned()).collect())
            .collect()
    }

    #[test]
    fn use_trees_expand_to_every_leaf_with_self_and_renames_resolved() {
        let leaves = use_leaves(
            "pub(crate) use crate::runner::{self, Cmd as Command, jobs::{Job, summary}};",
        );
        assert_eq!(
            leaves,
            owned(&[
                &["crate", "runner"],
                &["crate", "runner", "Cmd"],
                &["crate", "runner", "jobs", "Job"],
                &["crate", "runner", "jobs", "summary"],
            ])
        );
        assert_eq!(use_leaves("use std :: fs ;"), owned(&[&["std", "fs"]]));
    }

    #[test]
    fn chains_outside_use_declarations_come_with_their_line() {
        let code =
            "use std::fs;\nfn f() {\n    crate::a::b(super::c::D::new());\n    x.y::<u8>();\n}\n";
        let found: Vec<(usize, String)> = paths(code)
            .iter()
            .map(|path| (path.line, path.segments.join("::")))
            .collect();
        assert_eq!(
            found,
            [
                (1, "std::fs".to_owned()),
                (3, "crate::a::b".to_owned()),
                (3, "super::c::D::new".to_owned()),
            ]
        );
    }
}
```

Add `pub(crate) mod rust_paths;` to `gate/src/checks/mod.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline rust_paths`
Expected: compile error, unresolved imports `super::paths`, `super::use_leaves`.

- [ ] **Step 3: Write the implementation above the test module**

```rust
//! The paths a Rust file names: every `use` tree expanded to its leaves, and
//! every other `a::b` chain, each with its line. Read from blanked code, so a
//! path never comes out of a string or a comment.

use super::rust_code::{blank, is_identifier_byte, line_at};

/// One path a file names, and its line.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NamedPath {
    /// The line, counting from 1.
    pub(crate) line: usize,
    /// Its segments: `["crate", "runner", "Cmd"]`.
    pub(crate) segments: Vec<String>,
}

/// Every path of two segments or more that blanked `code` names, the `use`
/// declarations first.
pub(crate) fn paths(code: &str) -> Vec<NamedPath> {
    let mut rest = code.as_bytes().to_vec();
    let mut found = Vec::new();
    for (start, end) in use_declarations(code) {
        let line = line_at(code, start);
        let leaves = use_leaves(code.get(start..end).unwrap_or_default());
        found.extend(
            leaves
                .into_iter()
                .filter(|segments| segments.len() >= 2)
                .map(|segments| NamedPath { line, segments }),
        );
        blank(rest.get_mut(start..end).unwrap_or_default());
    }
    found.extend(chains(&String::from_utf8(rest).unwrap_or_default()));
    found
}

/// The leaves of one `use` declaration: `use a::{b, c::d};` gives `a::b` and
/// `a::c::d`. `self` names the module its braces hang from, and a renamed
/// import keeps its original name.
pub(crate) fn use_leaves(declaration: &str) -> Vec<Vec<String>> {
    let Some(start) = word_offsets(declaration, "use").first().copied() else {
        return Vec::new();
    };
    let tree = declaration.get(start + 3..).unwrap_or_default();
    expand(&[], &normalize(tree.trim_end().trim_end_matches(';')))
}

/// Where each `use` declaration of `code` starts and ends, its `;` included.
fn use_declarations(code: &str) -> Vec<(usize, usize)> {
    word_offsets(code, "use")
        .into_iter()
        .map(|start| {
            let tail = code.get(start..).unwrap_or_default();
            (start, start + tail.find(';').map_or(tail.len(), |offset| offset + 1))
        })
        .collect()
}

/// Every offset where `word` stands as a whole word in `text`.
fn word_offsets(text: &str, word: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    text.match_indices(word)
        .map(|(offset, _)| offset)
        .filter(|&offset| {
            let before = offset.checked_sub(1).and_then(|index| bytes.get(index));
            let after = bytes.get(offset + word.len());
            !before.is_some_and(|&byte| is_identifier_byte(byte))
                && !after.is_some_and(|&byte| is_identifier_byte(byte))
        })
        .collect()
}

/// A `use` tree with its whitespace gone, but for the spaces around `as`.
fn normalize(tree: &str) -> String {
    let words: Vec<&str> = tree.split_whitespace().collect();
    let mut out = String::new();
    for (index, word) in words.iter().enumerate() {
        let previous = index.checked_sub(1).and_then(|previous| words.get(previous));
        if *word == "as" || previous.is_some_and(|previous| *previous == "as") {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// The leaves of a normalized `use` tree, each after `prefix`.
fn expand(prefix: &[String], tree: &str) -> Vec<Vec<String>> {
    if let Some(open) = tree.find('{') {
        let close = tree.rfind('}').unwrap_or(tree.len());
        let mut base = prefix.to_vec();
        base.extend(segments(tree.get(..open).unwrap_or_default()));
        return split_top_level(tree.get(open + 1..close).unwrap_or_default())
            .into_iter()
            .flat_map(|part| expand(&base, part))
            .collect();
    }
    let original = tree.split(" as ").next().unwrap_or_default();
    let mut path = prefix.to_vec();
    path.extend(segments(original));
    if path.last().is_some_and(|last| last == "self") {
        path.pop();
    }
    vec![path]
}

/// The segments of a path written with `::`.
fn segments(path: &str) -> Vec<String> {
    path.split("::")
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The parts of a brace group, split at its top-level commas.
fn split_top_level(group: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in group.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(group.get(start..index).unwrap_or_default());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(group.get(start..).unwrap_or_default());
    parts.into_iter().filter(|part| !part.is_empty()).collect()
}

/// Whether a byte can start an identifier.
fn starts_identifier(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

/// Every `a::b` chain of `code`: one starts where an identifier follows
/// neither another identifier nor `:`.
fn chains(code: &str) -> Vec<NamedPath> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let boundary = index
            .checked_sub(1)
            .and_then(|previous| bytes.get(previous))
            .is_none_or(|&previous| !is_identifier_byte(previous) && previous != b':');
        if !(boundary && starts_identifier(byte)) {
            index += 1;
            continue;
        }
        let (segments, end) = chain_at(code, index);
        if segments.len() >= 2 {
            found.push(NamedPath {
                line: line_at(code, index),
                segments,
            });
        }
        index = end.max(index + 1);
    }
    found
}

/// The segments of the chain starting at `start`, and where it ends: words
/// joined by `::`, whitespace allowed around it; `::<` ends it.
fn chain_at(code: &str, start: usize) -> (Vec<String>, usize) {
    let bytes = code.as_bytes();
    let mut segments = Vec::new();
    let mut index = start;
    loop {
        let length = bytes
            .get(index..)
            .unwrap_or_default()
            .iter()
            .take_while(|&&byte| is_identifier_byte(byte))
            .count();
        segments.push(code.get(index..index + length).unwrap_or_default().to_owned());
        let word_end = index + length;
        let colons = skip_spaces(bytes, word_end);
        if !bytes.get(colons..).unwrap_or_default().starts_with(b"::") {
            return (segments, word_end);
        }
        let next = skip_spaces(bytes, colons + 2);
        if !bytes.get(next).is_some_and(|&byte| starts_identifier(byte)) {
            return (segments, word_end);
        }
        index = next;
    }
}

/// The first offset at or after `from` that is not whitespace.
fn skip_spaces(bytes: &[u8], from: usize) -> usize {
    from + bytes
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|byte| byte.is_ascii_whitespace())
        .count()
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline rust_paths`
Expected: 2 passed. Do not commit yet.

---

### Task 4: Module trees

**Files:**
- Create: `gate/src/checks/module_tree.rs`
- Modify: `gate/src/checks/mod.rs` (add `pub(crate) mod module_tree;` after `pub(crate) mod inputs;`)

**Interfaces:**
- Consumes: `rust_code::{Item, blanked, items, without_tests}`, `rust_paths::{NamedPath, paths, use_leaves}`, `crate::runner::{Cmd, Failure}`.
- Produces: `pub(crate) struct Tree { kinds: Vec<String>, modules: Vec<Module> }` with `find(&[String]) -> Option<usize>`, `absolute(from: &[String], segments: &[String]) -> Option<Vec<String>>`, `landing(&[String]) -> Option<usize>`; `pub(crate) struct Module { path, file, code, items, paths, exports }` with `read(Vec<String>, PathBuf, &str) -> Module`, `is_mod_rs()`, `children()`; `pub(crate) struct Export { line, segments, offered }`; `trees(project: &Path, temp: &Path) -> Result<Vec<Tree>, Failure>`; under `#[cfg(test)]`, `sample(kinds: &[&str], files: &[(&str, &str)]) -> Tree` for the rules' unit tests.

- [ ] **Step 1: Write the failing tests**

Create `gate/src/checks/module_tree.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::{Module, sample, tree, walk};
    use std::path::PathBuf;

    /// Owned segments.
    fn owned(path: &[&str]) -> Vec<String> {
        path.iter().map(|segment| (*segment).to_owned()).collect()
    }

    #[test]
    fn named_paths_become_absolute_from_the_root_a_parent_or_a_child() {
        let tree = sample(&["lib"], &[("lib.rs", "mod a;"), ("a.rs", "mod b;"), ("a/b.rs", "")]);
        let from = owned(&["a"]);
        let absolute = |from: &[String], path: &[&str]| tree.absolute(from, &owned(path));
        assert_eq!(absolute(&from, &["crate", "x", "Y"]), Some(owned(&["x", "Y"])));
        assert_eq!(absolute(&from, &["b", "f"]), Some(owned(&["a", "b", "f"])));
        assert_eq!(absolute(&owned(&["a", "b"]), &["super", "super", "g"]), Some(owned(&["g"])));
        assert_eq!(absolute(&from, &["super", "super", "g"]), None);
        assert_eq!(absolute(&from, &["std", "fs"]), None);
        assert_eq!(tree.landing(&owned(&["a", "b", "f"])), Some(2));
        assert_eq!(tree.landing(&owned(&["a", "f"])), Some(1));
    }

    #[test]
    fn a_door_records_what_it_re_exports_and_how_far() {
        let door = Module::read(
            owned(&["runner"]),
            PathBuf::from("/w/runner/mod.rs"),
            "mod commands;\npub(crate) use commands::{Cmd, write};\npub(super) use commands::Job;\n#[cfg(test)]\nmod tests;\n",
        );
        let exports: Vec<(String, bool)> = door
            .exports
            .iter()
            .map(|export| (export.segments.join("::"), export.offered))
            .collect();
        assert_eq!(
            exports,
            [
                ("commands::Cmd".to_owned(), true),
                ("commands::write".to_owned(), true),
                ("commands::Job".to_owned(), false),
            ]
        );
        assert!(door.is_mod_rs());
        let children: Vec<&str> = door.children().map(|child| child.name.as_str()).collect();
        assert_eq!(children, ["commands"]);
    }

    #[test]
    fn a_walk_follows_declarations_into_both_file_layouts() {
        let root = std::env::temp_dir().join(format!("module-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for directory in ["a", "b"] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
        }
        for (file, source) in [
            ("lib.rs", "mod a;\nmod b;\nmod missing;\n#[cfg(test)]\nmod tests;\n"),
            ("a.rs", "mod c;\n"),
            ("a/c.rs", ""),
            ("b/mod.rs", ""),
            ("tests.rs", ""),
        ] {
            std::fs::write(root.join(file), source).unwrap();
        }
        let modules = walk(&root.join("lib.rs")).unwrap();
        let paths: Vec<String> = modules.iter().map(|module| module.path.join("::")).collect();
        assert_eq!(paths, ["", "a", "a::c", "b"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_listing_line_without_a_root_is_refused_by_name() {
        let error = tree("lib").err().unwrap();
        assert_eq!(
            error.message.as_deref(),
            Some("cargo metadata listed a target the gate cannot read: lib")
        );
    }
}
```

Add `pub(crate) mod module_tree;` to `gate/src/checks/mod.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline module_tree`
Expected: compile error, unresolved imports `Module`, `sample`, `tree`, `walk`.

- [ ] **Step 3: Write the implementation above the test module**

```rust
//! The module trees of a Cargo project, the way the compiler builds them:
//! every target `cargo metadata` reports, and for each the files its `mod`
//! declarations reach from its root, with the items, paths and re-exports of
//! each.

use super::rust_code::{Item, blanked, items, without_tests};
use super::rust_paths::{NamedPath, paths, use_leaves};
use crate::runner::{Cmd, Failure};
use std::path::{Path, PathBuf};

/// Cargo's targets, one line each: their kinds and their root file.
const TARGETS: &str = ".packages[] | .targets[] | [(.kind | join(\",\")), .src_path] | @tsv";

/// One target and the modules its root reaches.
pub(crate) struct Tree {
    /// The target's kinds, as Cargo names them: `lib`, `bin`, `test`...
    pub(crate) kinds: Vec<String>,
    /// Every module, the root first.
    pub(crate) modules: Vec<Module>,
}

/// One module of a tree.
pub(crate) struct Module {
    /// Its path from the crate root: `[]` for the root, `["runner"]`.
    pub(crate) path: Vec<String>,
    /// The file holding it.
    pub(crate) file: PathBuf,
    /// Its code with comments and literals blanked, test items included.
    pub(crate) code: String,
    /// Its top-level items.
    pub(crate) items: Vec<Item>,
    /// The paths it names outside its test items.
    pub(crate) paths: Vec<NamedPath>,
    /// Every name it re-exports with a `pub` visibility.
    pub(crate) exports: Vec<Export>,
}

/// One name a module re-exports.
pub(crate) struct Export {
    /// The line of the `use` declaring it.
    pub(crate) line: usize,
    /// The path it re-exports, as written from the module.
    pub(crate) segments: Vec<String>,
    /// Whether it is offered past the module's parent.
    pub(crate) offered: bool,
}

impl Module {
    /// The module at `path`, held in `file`, read from its source.
    pub(crate) fn read(path: Vec<String>, file: PathBuf, source: &str) -> Self {
        let code = blanked(source);
        let items = items(&code);
        let paths = paths(&without_tests(&code));
        let exports = items
            .iter()
            .filter(|item| item.kind == "use" && item.visibility.starts_with("pub"))
            .filter(|item| !item.is_test())
            .flat_map(|item| {
                use_leaves(&item.text)
                    .into_iter()
                    .map(|segments| Export {
                        line: item.line,
                        segments,
                        offered: item.is_offered(),
                    })
            })
            .collect();
        Self {
            path,
            file,
            code,
            items,
            paths,
            exports,
        }
    }

    /// Whether the module is a directory's door, a `mod.rs`.
    pub(crate) fn is_mod_rs(&self) -> bool {
        self.file.file_name().is_some_and(|name| name == "mod.rs")
    }

    /// The modules it declares in files of their own, test modules left out.
    pub(crate) fn children(&self) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .filter(|item| item.is_module_file() && !item.is_test())
    }
}

impl Tree {
    /// The index of the module at `path`.
    pub(crate) fn find(&self, path: &[String]) -> Option<usize> {
        self.modules.iter().position(|module| module.path == path)
    }

    /// The absolute path a named path stands for, seen from the module at
    /// `from`: `crate` starts at the root, `self` and a child's name where
    /// `from` is, and each `super` climbs one level. `None` for a path into
    /// another crate or above the root.
    pub(crate) fn absolute(&self, from: &[String], segments: &[String]) -> Option<Vec<String>> {
        let (first, tail) = segments.split_first()?;
        let (mut base, mut rest) = match first.as_str() {
            "crate" => (Vec::new(), tail),
            "self" => (from.to_vec(), tail),
            "super" => (from.to_vec(), segments),
            _ => {
                let mut child = from.to_vec();
                child.push(first.clone());
                self.find(&child)?;
                (from.to_vec(), segments)
            }
        };
        while rest.first().is_some_and(|segment| segment == "super") {
            base.pop()?;
            rest = rest.get(1..).unwrap_or_default();
        }
        base.extend(rest.iter().cloned());
        Some(base)
    }

    /// The module an absolute path lands in: its longest prefix that is a
    /// module.
    pub(crate) fn landing(&self, absolute: &[String]) -> Option<usize> {
        (0..=absolute.len())
            .rev()
            .find_map(|length| self.find(absolute.get(..length)?))
    }
}

/// Every target of the Cargo project at `project` and its module tree.
pub(crate) fn trees(project: &Path, temp: &Path) -> Result<Vec<Tree>, Failure> {
    let metadata = temp.join("module-trees.json");
    Cmd::new("cargo metadata --no-deps --format-version 1 --offline --manifest-path")
        .arg(project.join("Cargo.toml"))
        .stdout_to(&metadata)?;
    let listing = Cmd::new("jaq -r").arg(TARGETS).arg(&metadata).capture()?;
    listing.lines().map(tree).collect()
}

/// The tree of the target one line of the listing names.
fn tree(line: &str) -> Result<Tree, Failure> {
    let Some((kinds, root)) = line.split_once('\t') else {
        return Err(format!("cargo metadata listed a target the gate cannot read: {line}").into());
    };
    Ok(Tree {
        kinds: kinds.split(',').map(str::to_owned).collect(),
        modules: walk(Path::new(root))?,
    })
}

/// The modules a root file reaches through its `mod` declarations, the root
/// first. A declaration whose file is missing is left to the compiler.
fn walk(root: &Path) -> Result<Vec<Module>, Failure> {
    let mut modules = Vec::new();
    let mut pending = vec![(Vec::new(), root.to_path_buf())];
    while let Some((path, file)) = pending.pop() {
        let source = std::fs::read_to_string(&file)
            .map_err(|error| format!("{}: {error}", file.display()))?;
        let module = Module::read(path, file, &source);
        let directory = children_directory(&module);
        for child in module.children() {
            if let Some(found) = child_file(&directory, &child.name) {
                let mut path = module.path.clone();
                path.push(child.name.clone());
                pending.push((path, found));
            }
        }
        modules.push(module);
    }
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(modules)
}

/// Where a module's children live: beside a root or a `mod.rs`, and in the
/// directory named after any other file.
fn children_directory(module: &Module) -> PathBuf {
    let parent = module.file.parent().map(Path::to_path_buf).unwrap_or_default();
    if module.path.is_empty() || module.is_mod_rs() {
        return parent;
    }
    parent.join(module.file.file_stem().unwrap_or_default())
}

/// The file of child `name` in `directory`: `name.rs`, else `name/mod.rs`.
fn child_file(directory: &Path, name: &str) -> Option<PathBuf> {
    [
        directory.join(format!("{name}.rs")),
        directory.join(name).join("mod.rs"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// A tree of in-memory modules for unit tests: each a file under `/w/src`
/// and its source, the module path read from the file name, the root first.
#[cfg(test)]
pub(crate) fn sample(kinds: &[&str], files: &[(&str, &str)]) -> Tree {
    let mut modules: Vec<Module> = files
        .iter()
        .map(|(file, source)| {
            let stem = file.trim_end_matches(".rs");
            let stem = stem.strip_suffix("/mod").unwrap_or(stem);
            let path = if matches!(stem, "lib" | "main") {
                Vec::new()
            } else {
                stem.split('/').map(str::to_owned).collect()
            };
            Module::read(path, PathBuf::from("/w/src").join(file), source)
        })
        .collect();
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    Tree {
        kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect(),
        modules,
    }
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline module_tree`
Expected: 4 passed. Do not commit yet.

---

### Task 5: The quality file and findings

**Files:**
- Create: `gate/src/checks/quality_config.rs`, `gate/src/checks/findings.rs`
- Modify: `gate/src/checks/mod.rs` (whole file, below)

**Interfaces:**
- Consumes: `crate::runner::{Cmd, Failure}`.
- Produces: `quality_config::{FILE, Layers { root: String, layers: Vec<Vec<String>> }, Exception { rule, path, item, reason: String }, QualityConfig { layers, exceptions }, read(workspace: &Path) -> Result<QualityConfig, Failure>}`; `findings::{Finding { file: String, line: usize, rule: String, item: String, message: String }, Finding::new(rule: &str, file: String, line: usize, message: String), Finding::about(self, item: &str) -> Finding, relative(workspace: &Path, file: &Path) -> String, excuse(findings: Vec<Finding>, exceptions: &[Exception], rules: &[&str], scope: &str) -> (Vec<Finding>, Vec<(Finding, &str)>)}`. `Finding` derives `Debug, PartialEq, Eq, PartialOrd, Ord` and implements `Display` as `RULE file:line: message`, the `:line` left out when the line is 0.

- [ ] **Step 1: Write the failing tests**

`gate/src/checks/quality_config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{parse_exception, parse_layers};

    #[test]
    fn layers_split_into_modules_and_refuse_a_single_layer_or_a_repeated_module() {
        let layers = parse_layers("gate/src/main.rs\tsteps|checks|runner").unwrap();
        assert_eq!(layers.root, "gate/src/main.rs");
        assert_eq!(layers.layers, [["steps"], ["checks"], ["runner"]]);
        let grouped = parse_layers("tests/workflows.rs\tci gate|harness").unwrap();
        assert_eq!(grouped.layers, [vec!["ci", "gate"], vec!["harness"]]);
        assert_eq!(
            parse_layers("gate/src/main.rs\tsteps").err().unwrap().message.as_deref(),
            Some("maestro-quality.toml: a [[crate]] names its root and two layers at least")
        );
        assert_eq!(
            parse_layers("r.rs\ta b|b").err().unwrap().message.as_deref(),
            Some("maestro-quality.toml: module `b` sits in two layers of r.rs")
        );
    }

    #[test]
    fn exceptions_need_a_rule_that_takes_one_a_path_and_a_reason() {
        let exception =
            parse_exception("ARC-005\tgate/src/runner/mod.rs\tenter\tthe registry runs steps")
                .unwrap();
        assert_eq!(
            (exception.rule.as_str(), exception.item.as_str()),
            ("ARC-005", "enter")
        );
        assert_eq!(
            parse_exception("ARC-001\tx\t\twhy").err().unwrap().message.as_deref(),
            Some(
                "maestro-quality.toml: ARC-001 takes no exception; \
                 only ARC-005, DUP-001, HYG-003, TST-001, DEP-001, PRF-001 do"
            )
        );
        assert_eq!(
            parse_exception("ARC-005\tx\t\t").err().unwrap().message.as_deref(),
            Some("maestro-quality.toml: the ARC-005 exception names no path or gives no reason")
        );
    }
}
```

`gate/src/checks/findings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{Exception, Finding, excuse};

    /// An exception of `rule` for `item` at `path`.
    fn exception(rule: &str, path: &str, item: &str) -> Exception {
        Exception {
            rule: rule.to_owned(),
            path: path.to_owned(),
            item: item.to_owned(),
            reason: "because".to_owned(),
        }
    }

    #[test]
    fn a_finding_reads_rule_file_line_and_message() {
        let finding = Finding::new("ARC-002", "src/a/mod.rs".to_owned(), 4, "move it".to_owned());
        assert_eq!(finding.to_string(), "ARC-002 src/a/mod.rs:4: move it");
        let whole = Finding::new("ARC-001", "src/a.rs".to_owned(), 0, "cycle".to_owned());
        assert_eq!(whole.to_string(), "ARC-001 src/a.rs: cycle");
    }

    #[test]
    fn exceptions_excuse_their_finding_and_a_stale_one_in_scope_is_reported() {
        let one = |item: &str| {
            Finding::new("ARC-005", "p/src/r/mod.rs".to_owned(), 3, "serves one".to_owned())
                .about(item)
        };
        let exceptions = [
            exception("ARC-005", "p/src/r/mod.rs", "only"),
            exception("ARC-005", "p/src/r/mod.rs", "gone"),
            exception("ARC-005", "q/src/lib.rs", "elsewhere"),
            exception("DUP-001", "p/src/lib.rs", ""),
        ];
        let (kept, excused) = excuse(vec![one("only"), one("other")], &exceptions, &["ARC-005"], "p/");
        let kept: Vec<String> = kept.iter().map(ToString::to_string).collect();
        assert_eq!(
            kept,
            [
                "ARC-005 p/src/r/mod.rs: the exception for `gone` excuses nothing any more; \
                 remove it from maestro-quality.toml",
                "ARC-005 p/src/r/mod.rs:3: serves one",
            ]
        );
        assert_eq!(excused.len(), 1);
        assert_eq!(excused[0].1, "because");
    }
}
```

`gate/src/checks/mod.rs`, the whole file:

```rust
//! What the steps check and share, one module per concern so a call site
//! names the kind of rule it reaches for: paths in the checkout, simple
//! names, Rust versions, the release vocabulary, private directories,
//! Cargo's records, the typed `ci.yml` inputs, Rust source read into module
//! trees, `maestro-quality.toml` and the findings rules report. Built on the
//! runner, never on a step.

pub(crate) mod cargo_metadata;
pub(crate) mod checkout_paths;
pub(crate) mod findings;
pub(crate) mod inputs;
pub(crate) mod module_tree;
pub(crate) mod private_directories;
pub(crate) mod quality_config;
pub(crate) mod release_boundary;
pub(crate) mod rust_code;
pub(crate) mod rust_paths;
pub(crate) mod rust_versions;
pub(crate) mod simple_names;
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline quality_config findings`
Expected: compile errors, unresolved imports in both test modules.

- [ ] **Step 3: Write `quality_config.rs` above its test module**

```rust
//! `maestro-quality.toml`, the one file a repository writes to shape the
//! organization's rules: the layers a target declares and the exceptions the
//! repository takes, each with its reason. Read through the pinned jaq, which
//! reads TOML, so the gate stays standard-library only.

use crate::runner::{Cmd, Failure};
use std::collections::BTreeSet;
use std::path::Path;

/// The file, at the root of the repository.
pub(crate) const FILE: &str = "maestro-quality.toml";

/// The rules that take an exception; every other rule has none.
const EXCEPTED: &[&str] = &["ARC-005", "DUP-001", "HYG-003", "TST-001", "DEP-001", "PRF-001"];

/// The tables the file may hold.
const TABLES: &[&str] = &["crate", "exception", "limits", "performance", "typos"];

/// The top-level tables of the file, one per line.
const KEYS: &str = "keys[]";

/// Each `[[crate]]`: its root, a tab, then its layers joined by `|`, the
/// modules of one layer by spaces.
const CRATES: &str = ".crate // [] | .[] | [.root, (.layers | map(if type == \"array\" \
    then join(\" \") else . end) | join(\"|\"))] | @tsv";

/// Each `[[exception]]`: rule, path, item and reason, tab-separated.
const EXCEPTIONS: &str =
    ".exception // [] | .[] | [.rule, .path, (.item // \"\"), (.reason // \"\")] | @tsv";

/// The layers one target declares, left to right.
pub(crate) struct Layers {
    /// The target's root file, relative to the repository root.
    pub(crate) root: String,
    /// Each layer's top-level modules.
    pub(crate) layers: Vec<Vec<String>>,
}

/// One exception: the rule, where, the item when the rule names one, and why.
pub(crate) struct Exception {
    /// The rule it excuses: `ARC-005`.
    pub(crate) rule: String,
    /// The file, relative to the repository root.
    pub(crate) path: String,
    /// The item, empty when the rule names none.
    pub(crate) item: String,
    /// Why the finding stays.
    pub(crate) reason: String,
}

/// What the file says, empty when the repository has none.
#[derive(Default)]
pub(crate) struct QualityConfig {
    /// The layers each declared target keeps.
    pub(crate) layers: Vec<Layers>,
    /// The exceptions the repository takes.
    pub(crate) exceptions: Vec<Exception>,
}

/// Read the file at the root of `workspace`, refusing a table it does not
/// know, a declaration of layers that orders nothing, and an exception its
/// rule does not take or that gives no reason.
pub(crate) fn read(workspace: &Path) -> Result<QualityConfig, Failure> {
    let file = workspace.join(FILE);
    if !file.is_file() {
        return Ok(QualityConfig::default());
    }
    let query = |program: &str| {
        Cmd::new("jaq --from toml -r")
            .arg(program)
            .arg(&file)
            .capture()
    };
    for table in query(KEYS)?.lines() {
        if !TABLES.contains(&table) {
            return Err(format!("{FILE}: unknown table `{table}`; it takes {}", TABLES.join(", ")).into());
        }
    }
    Ok(QualityConfig {
        layers: query(CRATES)?.lines().map(parse_layers).collect::<Result<_, _>>()?,
        exceptions: query(EXCEPTIONS)?
            .lines()
            .map(parse_exception)
            .collect::<Result<_, _>>()?,
    })
}

/// One line of the layers listing.
fn parse_layers(line: &str) -> Result<Layers, Failure> {
    let (root, joined) = line.split_once('\t').unwrap_or((line, ""));
    let layers: Vec<Vec<String>> = joined
        .split('|')
        .map(|layer| layer.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
        .filter(|layer| !layer.is_empty())
        .collect();
    if root.is_empty() || layers.len() < 2 {
        return Err(format!("{FILE}: a [[crate]] names its root and two layers at least").into());
    }
    let mut seen = BTreeSet::new();
    for module in layers.iter().flatten() {
        if !seen.insert(module) {
            return Err(format!("{FILE}: module `{module}` sits in two layers of {root}").into());
        }
    }
    Ok(Layers {
        root: root.to_owned(),
        layers,
    })
}

/// One line of the exceptions listing.
fn parse_exception(line: &str) -> Result<Exception, Failure> {
    let mut fields = line.split('\t').map(str::to_owned);
    let mut next = || fields.next().unwrap_or_default();
    let (rule, path, item, reason) = (next(), next(), next(), next());
    if !EXCEPTED.contains(&rule.as_str()) {
        return Err(format!(
            "{FILE}: {rule} takes no exception; only {} do",
            EXCEPTED.join(", ")
        )
        .into());
    }
    if path.is_empty() || reason.trim().is_empty() {
        return Err(format!("{FILE}: the {rule} exception names no path or gives no reason").into());
    }
    Ok(Exception {
        rule,
        path,
        item,
        reason,
    })
}
```

- [ ] **Step 4: Write `findings.rs` above its test module**

```rust
//! A rule's finding and how a step reports it: the rule, the file relative
//! to the repository and its line, and what to do, one line each; and the
//! exceptions `maestro-quality.toml` takes, each excusing one finding, a stale
//! one reported as a finding itself.

use super::quality_config::Exception;
use std::fmt;
use std::path::Path;

/// One rule broken at one place.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Finding {
    /// The file, relative to the repository root.
    pub(crate) file: String,
    /// The line, or 0 when the finding is about the whole file.
    pub(crate) line: usize,
    /// The rule broken: `ARC-001`.
    pub(crate) rule: String,
    /// The item an exception names, empty when the rule names none.
    pub(crate) item: String,
    /// What is wrong, and what to do.
    pub(crate) message: String,
}

impl Finding {
    /// A finding of `rule` at `file` and `line`.
    pub(crate) fn new(rule: &str, file: String, line: usize, message: String) -> Self {
        Self {
            file,
            line,
            rule: rule.to_owned(),
            item: String::new(),
            message,
        }
    }

    /// The same finding, naming the item an exception would name.
    #[must_use]
    pub(crate) fn about(mut self, item: &str) -> Self {
        item.clone_into(&mut self.item);
        self
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{} {}: {}", self.rule, self.file, self.message)
        } else {
            write!(
                formatter,
                "{} {}:{}: {}",
                self.rule, self.file, self.line, self.message
            )
        }
    }
}

/// `file` relative to `workspace`, the way a finding names it.
pub(crate) fn relative(workspace: &Path, file: &Path) -> String {
    file.strip_prefix(workspace)
        .unwrap_or(file)
        .display()
        .to_string()
}

/// What the exceptions leave: the findings none excuses; each excused one
/// with its reason; and a finding for every exception of `rules` under
/// `scope` that excuses nothing, since an exception outliving its violation
/// would hide the next one.
pub(crate) fn excuse<'a>(
    findings: Vec<Finding>,
    exceptions: &'a [Exception],
    rules: &[&str],
    scope: &str,
) -> (Vec<Finding>, Vec<(Finding, &'a str)>) {
    let mut used = vec![false; exceptions.len()];
    let mut kept = Vec::new();
    let mut excused = Vec::new();
    for finding in findings {
        let excusing = exceptions.iter().enumerate().find(|(_, exception)| {
            exception.rule == finding.rule
                && exception.path == finding.file
                && exception.item == finding.item
        });
        if let Some((index, exception)) = excusing {
            if let Some(flag) = used.get_mut(index) {
                *flag = true;
            }
            excused.push((finding, exception.reason.as_str()));
        } else {
            kept.push(finding);
        }
    }
    for (exception, used) in exceptions.iter().zip(used) {
        let judged_here = rules.contains(&exception.rule.as_str()) && exception.path.starts_with(scope);
        if judged_here && !used {
            let message = format!(
                "the exception for `{}` excuses nothing any more; remove it from maestro-quality.toml",
                exception.item
            );
            kept.push(Finding::new(&exception.rule, exception.path.clone(), 0, message).about(&exception.item));
        }
    }
    kept.sort();
    (kept, excused)
}
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline`
Expected: every unit test passes, including the 4 new ones. Do not commit yet.

---

### Task 6: The architecture step and ARC-001

**Files:**
- Create: `gate/src/steps/architecture/mod.rs`, `gate/src/steps/architecture/step.rs`, `gate/src/steps/architecture/cycles.rs`
- Create: tests/ci/architecture_rules.rs
- Modify: `gate/src/steps/mod.rs` (add `mod architecture;` after `mod api_compatibility;`), `gate/src/steps/registry.rs` (import `architecture`, add `architecture::STEPS,` after `install_toolchain::STEPS,`)
- Modify: .github/workflows/ci.yml, .github/workflows/publish-crate.yml, .github/workflows/publish-binaries.yml, tests/ci/mod.rs, docs/ci.md, docs/gates.toml, .github/copilot-instructions.md

**Interfaces:**
- Consumes: everything Tasks 2 to 5 produce.
- Produces: `rust-gate architecture`, reading `PROJECT`, `REPORTS`, `RUNNER_TEMP`, `GITHUB_STEP_SUMMARY` and `GITHUB_WORKSPACE`, writing `architecture.txt`; rule modules expose `pub(super) fn ...(tree: &Tree, workspace: &Path) -> Vec<Finding>`.

- [ ] **Step 1: Write the failing contract tests**

tests/ci/architecture_rules.rs:

```rust
//! `ci.yml`: the organization's module structure rules, ARC-001 to ARC-007,
//! each refused by its identifier with its file and line, and the exceptions
//! `maestro-quality.toml` takes, a stale one refused too.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

/// A fixture whose project holds `files`, each a path under the project and
/// its source.
fn project(files: &[(&str, &str)]) -> Fixture {
    let f = Fixture::new();
    for (path, source) in files {
        let file = f.root.join("project").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, source).unwrap();
    }
    f
}

/// The report the step wrote.
fn report(f: &Fixture) -> String {
    fs::read_to_string(f.root.join("reports/architecture.txt")).unwrap()
}

#[test]
fn a_project_without_findings_passes_with_an_empty_report() {
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod parse;\npub use parse::parse;\n"),
        ("src/parse.rs", "//! Parse.\n/// Parse.\npub fn parse() -> u8 { crate::parse::helper() }\nfn helper() -> u8 { 1 }\n"),
    ]);
    succeeds(&f.run("ci", "architecture"));
    assert_eq!(report(&f), "");
    let summary = fs::read_to_string(f.root.join("summary")).unwrap();
    assert!(summary.contains("No finding; 0 excused."), "{summary}");
}

#[test]
fn an_import_cycle_between_two_files_is_refused_by_name() {
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod a;\nmod b;\n"),
        ("src/a.rs", "//! A.\npub(crate) fn f() { crate::b::g() }\n"),
        ("src/b.rs", "//! B.\npub(crate) fn g() { crate::a::f() }\n"),
    ]);
    refused(
        &f.run("ci", "architecture"),
        "module structure: 1 finding; each names its rule, its file and what to do",
    );
    assert_eq!(
        report(&f),
        "ARC-001 project/src/a.rs: import cycle project/src/a.rs -> project/src/b.rs -> \
         project/src/a.rs; one of these files must stop naming the next\n"
    );
}

#[test]
fn the_quality_file_refuses_unknown_tables_and_unreasoned_exceptions() {
    let f = project(&[]);
    let file = f.root.join("maestro-quality.toml");
    fs::write(&file, "[typo]\nx = 1\n").unwrap();
    refused(
        &f.run("ci", "architecture"),
        "maestro-quality.toml: unknown table `typo`; it takes crate, exception, limits, performance, typos",
    );
    fs::write(&file, "[[exception]]\nrule = \"ARC-001\"\npath = \"project/src/lib.rs\"\nreason = \"x\"\n").unwrap();
    refused(&f.run("ci", "architecture"), "ARC-001 takes no exception");
    fs::write(&file, "[[exception]]\nrule = \"ARC-005\"\npath = \"project/src/lib.rs\"\n").unwrap();
    refused(&f.run("ci", "architecture"), "the ARC-005 exception names no path or gives no reason");
    fs::write(&file, "[[crate]]\nroot = \"project/src/lib.rs\"\nlayers = [\"a\"]\n").unwrap();
    refused(&f.run("ci", "architecture"), "a [[crate]] names its root and two layers at least");
}
```

In tests/ci/mod.rs add `mod architecture_rules;` in alphabetical position (first).

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd tests && cargo test --locked --test workflows architecture_rules`
Expected: FAIL, the harness finds no step `architecture` in `ci.yml`.

- [ ] **Step 3: Write the step**

`gate/src/steps/architecture/mod.rs`:

```rust
//! `rust-gate architecture`: the module structure every organization
//! repository holds to, ARC-001 to ARC-007, read from its source alone. The
//! step is `step.rs`; each other module holds one group of rules.

mod cycles;
mod step;

pub(super) use step::STEPS;
```

`gate/src/steps/architecture/step.rs`:

```rust
//! The architecture step: every target's module tree, the ARC rules run over
//! it, the exceptions `maestro-quality.toml` takes, and one report line per
//! finding.

use super::cycles;
use crate::checks::findings::{Finding, excuse, relative};
use crate::checks::module_tree::trees;
use crate::checks::quality_config;
use crate::runner::{Failure, Job, Outcome, Step, input, summary, write};
use std::fmt::Write as _;
use std::path::Path;

/// What this step declares: its inputs, its tools and its reports.
pub(in crate::steps) const STEPS: &[Step] = &[Step {
    workflow: "ci",
    id: "architecture",
    summary: "Module structure rules ARC-001 to ARC-007",
    inputs: &["GITHUB_WORKSPACE"],
    tools: &["cargo metadata", "jaq"],
    reports: &["architecture.txt"],
    run,
}];

/// The rules this step runs: the exceptions it judges are theirs.
const RULES: &[&str] = &[
    "ARC-001", "ARC-002", "ARC-003", "ARC-004", "ARC-005", "ARC-006", "ARC-007",
];

/// Run the step.
fn run() -> Outcome {
    let job = Job::current()?;
    let workspace = std::fs::canonicalize(input("GITHUB_WORKSPACE")?)
        .map_err(|error| format!("GITHUB_WORKSPACE: {error}"))?;
    let config = quality_config::read(&workspace)?;
    let scope = scope(&workspace, &job.project);
    let found = findings(&job, &workspace)?;
    let (kept, excused) = excuse(found, &config.exceptions, RULES, &scope);
    report(&job, &kept, &excused)
}

/// Every finding of every rule over every target of the project.
fn findings(job: &Job, workspace: &Path) -> Result<Vec<Finding>, Failure> {
    let trees = trees(&job.project, &job.temp)?;
    let mut found = Vec::new();
    for tree in &trees {
        found.extend(cycles::findings(tree, workspace));
    }
    found.sort();
    found.dedup();
    Ok(found)
}

/// The project's directory relative to the repository with a trailing
/// slash, or nothing when the project is the repository: this run judges the
/// exceptions under it.
fn scope(workspace: &Path, project: &Path) -> String {
    let project = std::fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf());
    let directory = relative(workspace, &project);
    if directory.is_empty() {
        directory
    } else {
        format!("{directory}/")
    }
}

/// Write every finding, then every excused one with its reason, to the
/// report and the summary, and fail when a finding is left.
fn report(job: &Job, kept: &[Finding], excused: &[(Finding, &str)]) -> Outcome {
    let mut text = String::new();
    for finding in kept {
        let _ = writeln!(text, "{finding}");
    }
    for (finding, reason) in excused {
        let _ = writeln!(text, "EXCUSED {finding} (because {reason})");
    }
    write(&job.report("architecture.txt")?, text.as_bytes(), false)?;
    if kept.is_empty() {
        println!("Module structure: no finding, {} excused", excused.len());
        return summary(&format!(
            "### Module structure\n\nNo finding; {} excused.\n",
            excused.len()
        ));
    }
    eprint!("{text}");
    summary(&format!("### Module structure\n\n```text\n{text}```\n"))?;
    let plural = if kept.len() == 1 { "" } else { "s" };
    Err(Failure::from(format!(
        "module structure: {} finding{plural}; each names its rule, its file and what to do",
        kept.len()
    )))
}
```

`gate/src/steps/architecture/cycles.rs`:

```rust
//! ARC-001: no import cycle between the files of one crate. A file depends on
//! another when it names one of that file's items through `crate`, `super`,
//! `self` or a child it declares; each cycle is named file by file.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Which modules each module names, by index; no module names itself.
type Graph = BTreeMap<usize, BTreeSet<usize>>;

/// ARC-001 over one tree: a finding per cycle, named from its first file.
pub(super) fn findings(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let graph = graph(tree);
    let mut done = BTreeSet::new();
    let mut found = Vec::new();
    for &start in graph.keys() {
        let Some(cycle) = walk(start, &graph, &mut done, &mut Vec::new()) else {
            continue;
        };
        done.extend(cycle.iter().copied());
        let files: Vec<String> = cycle
            .iter()
            .filter_map(|&index| tree.modules.get(index))
            .map(|module| relative(workspace, &module.file))
            .collect();
        let message = format!(
            "import cycle {}; one of these files must stop naming the next",
            files.join(" -> ")
        );
        let first = files.first().cloned().unwrap_or_default();
        found.push(Finding::new("ARC-001", first, 0, message));
    }
    found
}

/// Every path each module names, resolved to the module it lands in.
fn graph(tree: &Tree) -> Graph {
    tree.modules
        .iter()
        .enumerate()
        .map(|(index, module)| {
            let targets = module
                .paths
                .iter()
                .filter_map(|path| tree.absolute(&module.path, &path.segments))
                .filter_map(|absolute| tree.landing(&absolute))
                .filter(|&target| target != index)
                .collect();
            (index, targets)
        })
        .collect()
}

/// A depth-first walk from `node` that returns the first cycle it closes,
/// the repeated module last.
fn walk(
    node: usize,
    graph: &Graph,
    done: &mut BTreeSet<usize>,
    path: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    if let Some(start) = path.iter().position(|&seen| seen == node) {
        let mut cycle = path.get(start..).unwrap_or_default().to_vec();
        cycle.push(node);
        return Some(cycle);
    }
    if done.contains(&node) {
        return None;
    }
    path.push(node);
    for &next in graph.get(&node).into_iter().flatten() {
        if let Some(cycle) = walk(next, graph, done, path) {
            return Some(cycle);
        }
    }
    path.pop();
    done.insert(node);
    None
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_cycle_is_named_file_by_file_from_its_first_module() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod a;\nmod b;\nmod c;\n"),
                ("a.rs", "fn f() { crate::b::g() }\n"),
                ("b.rs", "fn g() { crate::a::f() }\n"),
                ("c.rs", "fn h() { crate::a::f() }\n"),
            ],
        );
        let found: Vec<String> = findings(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            ["ARC-001 src/a.rs: import cycle src/a.rs -> src/b.rs -> src/a.rs; \
              one of these files must stop naming the next"]
        );
    }

    #[test]
    fn a_parent_naming_a_child_that_names_nothing_back_is_no_cycle() {
        let tree = sample(
            &["lib"],
            &[("lib.rs", "mod a;\nfn f() { a::g() }\n"), ("a.rs", "pub(crate) fn g() {}\n")],
        );
        assert!(findings(&tree, Path::new("/w")).is_empty());
    }
}
```

Register it: `mod architecture;` in `gate/src/steps/mod.rs`; in `registry.rs` add `architecture,` to the `use super::{...}` list after `api_compatibility,` and `architecture::STEPS,` after `install_toolchain::STEPS,`.

- [ ] **Step 4: Run it from `ci.yml` behind the preview input**

In .github/workflows/ci.yml, after the `api-compatibility` input block, add:

```yaml
      quality-preview:
        description: >-
          Run the organization quality steps that ship with v2.0.0 before that
          release, the module structure rules first; v2.0.0 removes this input
          and runs them always
        type: boolean
        default: false
```

After the step `id: tools` (`run: rust-gate tools`), add:

```yaml
      - name: Module structure rules ARC-001 to ARC-007
        id: architecture
        # Always on from v2.0.0. Until then only on request: this repository's
        # own CI still runs the previously pinned gate, which lacks the step.
        if: ${{ inputs.quality-preview }}
        run: rust-gate architecture
```

In .github/workflows/publish-crate.yml and .github/workflows/publish-binaries.yml, after the `api-compatibility` input block, add:

```yaml
      quality-preview:
        description: >-
          Forwarded unchanged to the CI run, so a release passes the same gates
          as the project's own CI. `rust-version` is not: a release builds with
          the committed pin
        type: boolean
        default: false
```

and in the job that calls `ci.yml`, after `api-compatibility: ${{ inputs.api-compatibility }}`, add `quality-preview: ${{ inputs.quality-preview }}`.

- [ ] **Step 5: Run the contract tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline && cd tests && cargo test --locked --test workflows architecture_rules`
Expected: the gate's unit tests and the 3 contract tests pass.

- [ ] **Step 6: Document the step and its gate**

In docs/ci.md, after the section documenting `duplication.txt`, insert:

````markdown
### Module structure

Preview until v2.0.0: the step runs only with `quality-preview: true`, and
always from v2.0.0 on. `rust-gate architecture` reads every target Cargo
reports and the files its `mod` declarations reach, and refuses each finding
by its identifier, file and line:

| Rule | Refuses |
| --- | --- |
| ARC-001 | An import cycle between files of one crate |
| ARC-002 | A `mod.rs`, or a library root with modules, holding more than `mod` and `use` declarations |
| ARC-003 | A path from outside a door that walks past a name the door re-exports |
| ARC-004 | An import against the layers `maestro-quality.toml` declares for a target |
| ARC-005 | An item a door offers past its parent that one outside module uses and nothing inside shares |
| ARC-006 | A binary root holding more than declarations and a `fn main` of at most 25 lines |
| ARC-007 | A `#[path]` attribute or an `include!` of Rust source |

`architecture.txt` lists every finding, then every finding an exception
excuses, with its reason. `maestro-quality.toml`, at the root of the
repository, declares layers and takes exceptions; among these rules only
ARC-005 takes one, and an exception that excuses nothing is itself refused:

```toml
[[crate]]
root = "gate/src/main.rs"
layers = ["steps", "checks", "runner"]

[[exception]]
rule = "ARC-005"
path = "gate/src/runner/mod.rs"
item = "enter"
reason = "the step registry is the only thing that can run a step"
```
````

then, below that block, the sentence:

```text
The rules still to come are in the organization quality gate design,
`docs/superpowers/specs/2026-09-24-org-quality-gate-design.md`.
```

In docs/gates.toml, after the last `kind = "opt-in"` entry, add:

```toml
[[gate]]
name = "Module structure"
kind = "opt-in"
switch = "`quality-preview: true`, until v2.0.0 runs it always"
standard = "ARC-001 to ARC-007"
proofs = [
  "an_import_cycle_between_two_files_is_refused_by_name",
  "the_quality_file_refuses_unknown_tables_and_unreasoned_exceptions",
]
```

In .github/copilot-instructions.md add, in sorted position, the lines for the five checks modules, the `architecture` directory with `cycles.rs`, `mod.rs` and `step.rs`, and the contract test:

```text
│   │   │   ├── findings.rs                     # A rule's finding as one report line, and the exceptions that excuse some
│   │   │   ├── module_tree.rs                  # Every Cargo target's module tree: files, items, named paths and re-exports
│   │   │   ├── quality_config.rs               # maestro-quality.toml read through jaq: declared layers and reasoned exceptions
│   │   │   ├── rust_code.rs                    # Rust source with comments and literals blanked, and its top-level items
│   │   │   ├── rust_paths.rs                   # Every path a Rust file names: use trees expanded, a::b chains
│   │   │   ├── architecture/                   # rust-gate architecture: the step and one module per group of rules
│   │   │   │   ├── cycles.rs                   # ARC-001: no import cycle between the files of a crate
│   │   │   │   ├── mod.rs                      # The step's door: its modules and its declaration
│   │   │   │   └── step.rs                     # The step: module trees, the rules, the exceptions and the report
│   │   ├── architecture_rules.rs               # ci.yml: ARC-001 to ARC-007, each refused by name, and the exceptions maestro-quality.toml takes
```

Then run `just docs` to regenerate docs/steps.md, the input tables and the README gate tables.

- [ ] **Step 7: Verify and commit**

Run: `timeout 1800 just check > /tmp/just-check.log; echo $?`
Expected: `0`. If `every_step_module_declares_what_its_source_uses_and_nothing_else` reports that the architecture step uses or declares a name the other does not, align `STEPS` with what the step's own files and the checks it calls run: `cargo metadata` and `jaq`, the report `architecture.txt`, the input `GITHUB_WORKSPACE`.

```bash
git add -A gate/src tests/ci .github docs README.md
git commit -S -m "feat: refuse import cycles with rust-gate architecture"
```

---

### Task 7: ARC-002 and ARC-003, doors

**Files:**
- Create: `gate/src/steps/architecture/doors.rs`
- Modify: `gate/src/steps/architecture/mod.rs` (add `mod doors;`), `gate/src/steps/architecture/step.rs` (`use super::{cycles, doors};` and two lines in `findings`), tests/ci/architecture_rules.rs, docs/gates.toml, .github/copilot-instructions.md

**Interfaces:**
- Produces: `doors::contents(tree: &Tree, workspace: &Path) -> Vec<Finding>` (ARC-002) and `doors::bypasses(tree: &Tree, workspace: &Path) -> Vec<Finding>` (ARC-003).

- [ ] **Step 1: Write the failing contract tests**

Append to tests/ci/architecture_rules.rs:

```rust
#[test]
fn a_door_holding_a_function_is_refused_and_a_listing_door_passes() {
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod shapes;\n"),
        (
            "src/shapes/mod.rs",
            "//! Shapes.\nmod circle;\npub(crate) use circle::area;\n\n/// Pi.\nfn pi() -> f64 { 3.14 }\n",
        ),
        ("src/shapes/circle.rs", "//! Circle.\npub(crate) fn area() {}\n"),
    ]);
    refused(&f.run("ci", "architecture"), "module structure: 1 finding");
    assert_eq!(
        report(&f),
        "ARC-002 project/src/shapes/mod.rs:6: a door holds only mod and use declarations; \
         move this fn into a module of its own\n"
    );
}

#[test]
fn a_path_past_a_door_re_export_is_refused() {
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod report;\nmod shapes;\n"),
        (
            "src/report.rs",
            "//! Report.\npub(crate) fn print() { crate::shapes::circle::area(); crate::shapes::area(); }\n",
        ),
        ("src/shapes/mod.rs", "//! Shapes.\npub(super) mod circle;\npub(super) use circle::area;\n"),
        ("src/shapes/circle.rs", "//! Circle.\npub(crate) fn area() {}\n"),
    ]);
    refused(&f.run("ci", "architecture"), "module structure: 1 finding");
    assert_eq!(
        report(&f),
        "ARC-003 project/src/report.rs:2: `crate::shapes::circle::area` walks past the door of \
         crate::shapes; name `area` through it\n"
    );
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd tests && cargo test --locked --test workflows architecture_rules`
Expected: the two new tests FAIL, the step reports no finding.

- [ ] **Step 3: Write `doors.rs`**

```rust
//! ARC-002 and ARC-003: a door holds only `mod` and `use` declarations, and a
//! path from outside a door goes through it: when the door re-exports a
//! name, walking past it to the module defining the name is refused.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::Item;
use std::path::Path;

/// The kinds of target whose root is the crate's public door.
const LIBRARY_KINDS: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

/// ARC-002: every item of a door that does more than declare.
pub(super) fn contents(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let library = tree
        .kinds
        .iter()
        .any(|kind| LIBRARY_KINDS.contains(&kind.as_str()));
    let mut found = Vec::new();
    for module in &tree.modules {
        let root_door = module.path.is_empty() && library && module.children().next().is_some();
        if !(module.is_mod_rs() || root_door) {
            continue;
        }
        for item in module.items.iter().filter(|item| !item.is_test() && !declares(item)) {
            found.push(Finding::new(
                "ARC-002",
                relative(workspace, &module.file),
                item.line,
                format!(
                    "a door holds only mod and use declarations; move this {} into a module of its own",
                    item.kind
                ),
            ));
        }
    }
    found
}

/// Whether an item only declares: a module kept in its own file, an import
/// or an external crate.
fn declares(item: &Item) -> bool {
    item.is_module_file() || matches!(item.kind.as_str(), "use" | "extern crate")
}

/// ARC-003: every path from outside a `mod.rs` door that reaches, past the
/// door, a name the door re-exports from one of its modules.
pub(super) fn bypasses(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for door in tree.modules.iter().filter(|module| module.is_mod_rs()) {
        for export in &door.exports {
            let Some(target) = tree.absolute(&door.path, &export.segments) else {
                continue;
            };
            let Some(name) = target.last().filter(|_| target.len() > door.path.len() + 1) else {
                continue;
            };
            let outside = tree
                .modules
                .iter()
                .filter(|module| !module.path.starts_with(&door.path));
            for module in outside {
                for path in &module.paths {
                    let past = tree
                        .absolute(&module.path, &path.segments)
                        .is_some_and(|absolute| absolute.starts_with(&target));
                    if past {
                        found.push(Finding::new(
                            "ARC-003",
                            relative(workspace, &module.file),
                            path.line,
                            format!(
                                "`{}` walks past the door of {}; name `{name}` through it",
                                path.segments.join("::"),
                                display(&door.path)
                            ),
                        ));
                    }
                }
            }
        }
    }
    found
}

/// A module path as a reader writes it: `crate::runner`.
fn display(path: &[String]) -> String {
    std::iter::once("crate")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::{bypasses, contents};
    use crate::checks::module_tree::sample;
    use std::path::Path;

    /// Every finding of `rule` as its report line.
    fn lines(found: &[crate::checks::findings::Finding]) -> Vec<String> {
        found.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_door_holding_more_than_declarations_is_refused() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod shapes;\npub use shapes::area;\n"),
                (
                    "shapes/mod.rs",
                    "mod circle;\npub(crate) use circle::area;\nconst PI: f64 = 3.14;\nmod inline {}\n#[cfg(test)]\nmod tests {}\n",
                ),
                ("shapes/circle.rs", "pub(crate) fn area() {}\nfn helper() {}\n"),
            ],
        );
        assert_eq!(
            lines(&contents(&tree, Path::new("/w"))),
            [
                "ARC-002 src/shapes/mod.rs:3: a door holds only mod and use declarations; \
                 move this const into a module of its own",
                "ARC-002 src/shapes/mod.rs:4: a door holds only mod and use declarations; \
                 move this mod into a module of its own",
            ]
        );
    }

    #[test]
    fn a_single_file_library_and_a_binary_root_are_not_doors() {
        let library = sample(&["lib"], &[("lib.rs", "pub fn f() {}\n")]);
        let binary = sample(&["bin"], &[("main.rs", "mod a;\nfn main() {}\n"), ("a.rs", "")]);
        assert!(contents(&library, Path::new("/w")).is_empty());
        assert!(contents(&binary, Path::new("/w")).is_empty());
    }

    #[test]
    fn a_path_walking_past_a_re_export_is_refused() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod report;\nmod shapes;\n"),
                (
                    "report.rs",
                    "fn print() {\n    crate::shapes::circle::area();\n    crate::shapes::area();\n}\n",
                ),
                ("shapes/mod.rs", "pub(super) mod circle;\npub(super) use circle::area;\n"),
                ("shapes/circle.rs", "pub(crate) fn area() { super::circle::helper() }\n"),
            ],
        );
        assert_eq!(
            lines(&bypasses(&tree, Path::new("/w"))),
            ["ARC-003 src/report.rs:2: `crate::shapes::circle::area` walks past the door of \
              crate::shapes; name `area` through it"]
        );
    }
}
```

In `gate/src/steps/architecture/mod.rs` add `mod doors;` after `mod cycles;`. In `step.rs` replace `use super::cycles;` with `use super::{cycles, doors};` and, in `findings`, after `found.extend(cycles::findings(tree, workspace));` add:

```rust
        found.extend(doors::contents(tree, workspace));
        found.extend(doors::bypasses(tree, workspace));
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline doors && cd tests && cargo test --locked --test workflows architecture_rules`
Expected: 3 unit tests and 5 contract tests pass.

- [ ] **Step 5: Record the proofs, verify and commit**

Add `"a_door_holding_a_function_is_refused_and_a_listing_door_passes"` and `"a_path_past_a_door_re_export_is_refused"` to the Module structure proofs in docs/gates.toml; add under the `architecture` directory in .github/copilot-instructions.md:

```text
│   │   │   │   ├── doors.rs                    # ARC-002 and ARC-003: doors only declare, and paths go through them
```

Run `just docs`, then `timeout 1800 just check > /tmp/just-check.log; echo $?`, expected `0`.

```bash
git add -A gate/src tests/ci docs .github README.md
git commit -S -m "feat: keep doors to declarations and paths through them"
```

---

### Task 8: ARC-004, declared layers

**Files:**
- Create: `gate/src/steps/architecture/layers.rs`
- Modify: `gate/src/steps/architecture/mod.rs` (add `mod layers;`), `gate/src/steps/architecture/step.rs`, tests/ci/architecture_rules.rs, docs/gates.toml, .github/copilot-instructions.md

**Interfaces:**
- Consumes: `quality_config::{FILE, Layers, QualityConfig}`.
- Produces: `layers::findings(tree: &Tree, workspace: &Path, declared: &[Layers]) -> Vec<Finding>` and `layers::unknown_roots(trees: &[Tree], workspace: &Path, declared: &[Layers], scope: &str) -> Vec<Finding>`.

- [ ] **Step 1: Write the failing contract tests**

Append to tests/ci/architecture_rules.rs:

```rust
#[test]
fn declared_layers_refuse_an_import_within_or_against_the_order() {
    let f = project(&[
        ("src/main.rs", "//! Binary.\nmod checks;\nmod runner;\nmod steps;\nmod stray;\nfn main() {}\n"),
        ("src/steps.rs", "//! Steps.\npub(crate) fn go() { crate::runner::run(); }\n"),
        ("src/checks.rs", "//! Checks.\npub(crate) fn check() { crate::runner::run(); crate::steps::go(); }\n"),
        ("src/runner.rs", "//! Runner.\npub(crate) fn run() {}\n"),
        ("src/stray.rs", "//! Stray.\n"),
    ]);
    fs::remove_file(f.root.join("project/src/lib.rs")).unwrap();
    fs::write(
        f.root.join("maestro-quality.toml"),
        "[[crate]]\nroot = \"project/src/main.rs\"\nlayers = [\"steps\", \"checks\", \"runner\", \"missing\"]\n",
    )
    .unwrap();
    refused(&f.run("ci", "architecture"), "module structure: 3 findings");
    assert_eq!(
        report(&f),
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
    let f = project(&[]);
    fs::write(
        f.root.join("maestro-quality.toml"),
        "[[crate]]\nroot = \"project/src/gone.rs\"\nlayers = [\"a\", \"b\"]\n",
    )
    .unwrap();
    refused(&f.run("ci", "architecture"), "module structure: 1 finding");
    assert_eq!(
        report(&f),
        "ARC-004 maestro-quality.toml: declares layers for project/src/gone.rs, which is no \
         target's root\n"
    );
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd tests && cargo test --locked --test workflows architecture_rules`
Expected: the two new tests FAIL.

- [ ] **Step 3: Write `layers.rs`**

```rust
//! ARC-004: the layers a target declares in `maestro-quality.toml`. Each
//! top-level module sits in one layer, and a module imports only from layers
//! to its right: never from its own layer, never from one to its left.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::{Module, Tree};
use crate::checks::quality_config::{FILE, Layers};
use std::path::Path;

/// ARC-004 over one tree, when `maestro-quality.toml` declares its root.
pub(super) fn findings(tree: &Tree, workspace: &Path, declared: &[Layers]) -> Vec<Finding> {
    let Some(root) = tree.modules.first() else {
        return Vec::new();
    };
    let root_file = relative(workspace, &root.file);
    let Some(layers) = declared.iter().find(|layers| layers.root == root_file) else {
        return Vec::new();
    };
    let mut found = placement(root, layers, &root_file);
    for module in &tree.modules {
        let file = relative(workspace, &module.file);
        for (line, path, own, other) in crossings(tree, module, layers) {
            found.push(Finding::new(
                "ARC-004",
                file.clone(),
                line,
                format!(
                    "`{path}` imports `{other}` from `{own}`; a layer imports only from layers to its right"
                ),
            ));
        }
    }
    found
}

/// A finding for every root under `scope` that declares layers and that no
/// target has.
pub(super) fn unknown_roots(
    trees: &[Tree],
    workspace: &Path,
    declared: &[Layers],
    scope: &str,
) -> Vec<Finding> {
    let roots: Vec<String> = trees
        .iter()
        .filter_map(|tree| tree.modules.first())
        .map(|root| relative(workspace, &root.file))
        .collect();
    declared
        .iter()
        .filter(|layers| layers.root.starts_with(scope) && !roots.contains(&layers.root))
        .map(|layers| {
            Finding::new(
                "ARC-004",
                FILE.to_owned(),
                0,
                format!("declares layers for {}, which is no target's root", layers.root),
            )
        })
        .collect()
}

/// The layer holding top-level module `name`, counted from the left.
fn layer_of(layers: &Layers, name: &str) -> Option<usize> {
    layers
        .layers
        .iter()
        .position(|layer| layer.iter().any(|module| module == name))
}

/// Every top-level module the root declares outside the layers, and every
/// layer module the root does not declare.
fn placement(root: &Module, layers: &Layers, root_file: &str) -> Vec<Finding> {
    let mut found = Vec::new();
    for child in root.children() {
        if layer_of(layers, &child.name).is_none() {
            found.push(Finding::new(
                "ARC-004",
                root_file.to_owned(),
                child.line,
                format!(
                    "module `{}` sits in no layer {FILE} declares for this root; add it to one",
                    child.name
                ),
            ));
        }
    }
    for name in layers.layers.iter().flatten() {
        if !root.children().any(|child| &child.name == name) {
            found.push(Finding::new(
                "ARC-004",
                FILE.to_owned(),
                0,
                format!("the layers of {root_file} name `{name}`, which that root does not declare"),
            ));
        }
    }
    found
}

/// Every path of `module` into a top-level module of its own layer or of
/// one to its left: the line, the path, and the two modules.
fn crossings<'a>(
    tree: &Tree,
    module: &'a Module,
    layers: &Layers,
) -> Vec<(usize, String, &'a str, String)> {
    let Some(own) = module.path.first() else {
        return Vec::new();
    };
    let Some(from) = layer_of(layers, own) else {
        return Vec::new();
    };
    module
        .paths
        .iter()
        .filter_map(|path| {
            let target = tree.absolute(&module.path, &path.segments)?;
            let other = target.first().filter(|other| *other != own)?;
            let to = layer_of(layers, other)?;
            (to <= from).then(|| (path.line, path.segments.join("::"), own.as_str(), other.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{findings, unknown_roots};
    use crate::checks::module_tree::sample;
    use crate::checks::quality_config::Layers;
    use std::path::Path;

    /// The layers declared for `src/main.rs`, one string per layer.
    fn declared(layers: &[&str]) -> Vec<Layers> {
        vec![Layers {
            root: "src/main.rs".to_owned(),
            layers: layers
                .iter()
                .map(|layer| layer.split_whitespace().map(str::to_owned).collect())
                .collect(),
        }]
    }

    #[test]
    fn imports_run_only_to_layers_on_the_right() {
        let tree = sample(
            &["bin"],
            &[
                ("main.rs", "mod checks;\nmod runner;\nmod steps;\nmod stray;\nfn main() {}\n"),
                ("steps.rs", "fn go() { crate::checks::check(); crate::runner::run(); }\n"),
                ("checks.rs", "fn check() {\n    crate::steps::go();\n}\n"),
                ("runner.rs", "fn run() {}\n"),
                ("stray.rs", ""),
            ],
        );
        let found: Vec<String> =
            findings(&tree, Path::new("/w"), &declared(&["steps", "checks", "runner", "gone"]))
                .iter()
                .map(ToString::to_string)
                .collect();
        assert_eq!(
            found,
            [
                "ARC-004 src/main.rs:4: module `stray` sits in no layer maestro-quality.toml \
                 declares for this root; add it to one",
                "ARC-004 maestro-quality.toml: the layers of src/main.rs name `gone`, which \
                 that root does not declare",
                "ARC-004 src/checks.rs:2: `crate::steps::go` imports `steps` from `checks`; a \
                 layer imports only from layers to its right",
            ]
        );
    }

    #[test]
    fn modules_of_one_layer_never_import_each_other() {
        let tree = sample(
            &["test"],
            &[
                ("main.rs", "mod ci;\nmod gate;\nmod harness;\n"),
                ("ci.rs", "use crate::gate::helper;\nuse crate::harness::Fixture;\n"),
                ("gate.rs", ""),
                ("harness.rs", ""),
            ],
        );
        let found = findings(&tree, Path::new("/w"), &declared(&["ci gate", "harness"]));
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].to_string(),
            "ARC-004 src/ci.rs:1: `crate::gate::helper` imports `gate` from `ci`; a layer \
             imports only from layers to its right"
        );
    }

    #[test]
    fn layers_for_a_root_no_target_has_are_refused_in_scope_only() {
        let trees = [sample(&["bin"], &[("main.rs", "fn main() {}\n")])];
        let mut layers = declared(&["a", "b"]);
        layers[0].root = "src/gone.rs".to_owned();
        layers.extend(declared(&["a", "b"]));
        layers[1].root = "other/src/main.rs".to_owned();
        let found: Vec<String> = unknown_roots(&trees, Path::new("/w"), &layers, "src/")
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            ["ARC-004 maestro-quality.toml: declares layers for src/gone.rs, which is no \
              target's root"]
        );
    }
}
```

- [ ] **Step 4: Wire it into the step**

In `mod.rs` add `mod layers;` after `mod doors;`. In `step.rs`: replace `use super::{cycles, doors};` with `use super::{cycles, doors, layers};`, replace `use crate::checks::quality_config;` with `use crate::checks::quality_config::{self, QualityConfig};`, in `run` replace `let found = findings(&job, &workspace)?;` with `let found = findings(&job, &workspace, &config, &scope)?;`, and replace the whole `findings` function with:

```rust
/// Every finding of every rule over every target of the project.
fn findings(
    job: &Job,
    workspace: &Path,
    config: &QualityConfig,
    scope: &str,
) -> Result<Vec<Finding>, Failure> {
    let trees = trees(&job.project, &job.temp)?;
    let mut found = layers::unknown_roots(&trees, workspace, &config.layers, scope);
    for tree in &trees {
        found.extend(cycles::findings(tree, workspace));
        found.extend(doors::contents(tree, workspace));
        found.extend(doors::bypasses(tree, workspace));
        found.extend(layers::findings(tree, workspace, &config.layers));
    }
    found.sort();
    found.dedup();
    Ok(found)
}
```

- [ ] **Step 5: Run the tests, record the proofs, verify and commit**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline layers && cd tests && cargo test --locked --test workflows architecture_rules`
Expected: 3 unit tests and 7 contract tests pass.

Add both new contract test names to the Module structure proofs in docs/gates.toml and, under the `architecture` directory, the tree line:

```text
│   │   │   │   ├── layers.rs                   # ARC-004: imports run only to the layers on the right
```

Run `just docs` and `timeout 1800 just check > /tmp/just-check.log; echo $?`, expected `0`.

```bash
git add -A gate/src tests/ci docs .github README.md
git commit -S -m "feat: hold every target to the layers it declares"
```

---

### Task 9: ARC-005, seams that serve two

**Files:**
- Create: `gate/src/steps/architecture/seams.rs`
- Modify: `gate/src/steps/architecture/mod.rs` (add `mod seams;`), `gate/src/steps/architecture/step.rs`, tests/ci/architecture_rules.rs, docs/gates.toml, .github/copilot-instructions.md

**Interfaces:**
- Produces: `seams::findings(tree: &Tree, workspace: &Path) -> Vec<Finding>`, each finding naming its `item` for exceptions.

- [ ] **Step 1: Write the failing contract test**

Append to tests/ci/architecture_rules.rs:

```rust
#[test]
fn a_seam_serving_one_outside_caller_is_refused_unless_excused() {
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod first;\nmod runner;\nmod second;\n"),
        ("src/runner/mod.rs", "//! Runner.\nmod commands;\npub(crate) use commands::{Cmd, only};\n"),
        ("src/runner/commands.rs", "//! Commands.\npub(crate) struct Cmd;\npub(crate) fn only() {}\n"),
        ("src/first.rs", "//! First.\nuse crate::runner::{Cmd, only};\n"),
        ("src/second.rs", "//! Second.\nuse crate::runner::Cmd;\n"),
    ]);
    refused(&f.run("ci", "architecture"), "module structure: 1 finding");
    assert_eq!(
        report(&f),
        "ARC-005 project/src/runner/mod.rs:3: `only` serves only project/src/first.rs; move it \
         next to its caller, or record in maestro-quality.toml why this seam stays\n"
    );

    let excused = "[[exception]]\nrule = \"ARC-005\"\npath = \"project/src/runner/mod.rs\"\n\
                   item = \"only\"\nreason = \"the first module is its one caller by design\"\n";
    fs::write(f.root.join("maestro-quality.toml"), excused).unwrap();
    succeeds(&f.run("ci", "architecture"));
    assert_eq!(
        report(&f),
        "EXCUSED ARC-005 project/src/runner/mod.rs:3: `only` serves only project/src/first.rs; \
         move it next to its caller, or record in maestro-quality.toml why this seam stays \
         (because the first module is its one caller by design)\n"
    );

    fs::write(
        f.root.join("project/src/second.rs"),
        "//! Second.\nuse crate::runner::{Cmd, only};\n",
    )
    .unwrap();
    refused(&f.run("ci", "architecture"), "module structure: 1 finding");
    assert_eq!(
        report(&f),
        "ARC-005 project/src/runner/mod.rs: the exception for `only` excuses nothing any more; \
         remove it from maestro-quality.toml\n"
    );
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd tests && cargo test --locked --test workflows a_seam_serving_one_outside_caller_is_refused_unless_excused`
Expected: FAIL, the step passes the first run.

- [ ] **Step 3: Write `seams.rs`**

```rust
//! ARC-005: a seam serves two. An item a `mod.rs` door offers past its parent,
//! that exactly one module outside the door uses and that no module inside
//! shares besides the one defining it, could live next to its caller: it is
//! refused unless `maestro-quality.toml` records why it stays. The crate root
//! composes the crate and is not counted as a caller.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::{Module, Tree};
use std::path::Path;

/// One thing a door offers past its parent.
struct Offer {
    /// The name a caller writes after the door.
    name: String,
    /// The line of the door offering it.
    line: usize,
    /// Its absolute path through the door.
    through: Vec<String>,
    /// Its absolute path where it is defined.
    defined: Vec<String>,
}

/// ARC-005 over one tree.
pub(super) fn findings(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for door in tree.modules.iter().filter(|module| module.is_mod_rs()) {
        for offer in offers(tree, door) {
            let (outside, inside) = users(tree, door, &offer);
            let [only] = outside.as_slice() else {
                continue;
            };
            if !inside.is_empty() {
                continue;
            }
            let message = format!(
                "`{}` serves only {}; move it next to its caller, or record in \
                 maestro-quality.toml why this seam stays",
                offer.name,
                relative(workspace, &only.file)
            );
            found.push(
                Finding::new("ARC-005", relative(workspace, &door.file), offer.line, message)
                    .about(&offer.name),
            );
        }
    }
    found
}

/// What a door offers past its parent: the modules it declares with a wide
/// visibility, and the names it re-exports with one.
fn offers(tree: &Tree, door: &Module) -> Vec<Offer> {
    let mut offers: Vec<Offer> = door
        .children()
        .filter(|child| child.is_offered())
        .map(|child| {
            let mut through = door.path.clone();
            through.push(child.name.clone());
            Offer {
                name: child.name.clone(),
                line: child.line,
                defined: through.clone(),
                through,
            }
        })
        .collect();
    for export in door.exports.iter().filter(|export| export.offered) {
        let Some(defined) = tree.absolute(&door.path, &export.segments) else {
            continue;
        };
        let Some(name) = defined.last().cloned() else {
            continue;
        };
        let mut through = door.path.clone();
        through.push(name.clone());
        offers.push(Offer {
            name,
            line: export.line,
            through,
            defined,
        });
    }
    offers
}

/// The modules outside the door that use an offer, the crate root left out,
/// and those inside that use it besides the door and the defining module.
fn users<'a>(tree: &'a Tree, door: &Module, offer: &Offer) -> (Vec<&'a Module>, Vec<&'a Module>) {
    let defining = tree
        .landing(&offer.defined)
        .and_then(|index| tree.modules.get(index))
        .map(|module| module.path.clone())
        .unwrap_or_default();
    let mut outside = Vec::new();
    let mut inside = Vec::new();
    for module in &tree.modules {
        let skipped = module.path.is_empty()
            || module.path == door.path
            || module.path.starts_with(&defining);
        if skipped || !uses(tree, module, offer) {
            continue;
        }
        if module.path.starts_with(&door.path) {
            inside.push(module);
        } else {
            outside.push(module);
        }
    }
    (outside, inside)
}

/// Whether a module names an offer, through the door or where it is defined.
fn uses(tree: &Tree, module: &Module, offer: &Offer) -> bool {
    module
        .paths
        .iter()
        .filter_map(|path| tree.absolute(&module.path, &path.segments))
        .any(|absolute| absolute.starts_with(&offer.through) || absolute.starts_with(&offer.defined))
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_seam_with_one_outside_caller_and_no_inside_sharer_is_refused() {
        let tree = sample(
            &["bin"],
            &[
                ("main.rs", "mod first;\nmod runner;\nmod second;\nfn main() { runner::root_only(); }\n"),
                ("first.rs", "use crate::runner::{Cmd, only, shared};\n"),
                ("second.rs", "use crate::runner::Cmd;\n"),
                (
                    "runner/mod.rs",
                    "mod commands;\nmod jobs;\npub(crate) use commands::{Cmd, only, root_only, shared};\npub(super) use commands::narrow;\n",
                ),
                ("runner/commands.rs", "pub(crate) struct Cmd;\n"),
                ("runner/jobs.rs", "use super::commands::shared;\n"),
            ],
        );
        let found = findings(&tree, Path::new("/w"));
        let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["ARC-005 src/runner/mod.rs:3: `only` serves only src/first.rs; move it next to \
              its caller, or record in maestro-quality.toml why this seam stays"]
        );
        assert_eq!(found[0].item, "only");
    }
}
```

- [ ] **Step 4: Wire it into the step**

In `mod.rs` add `mod seams;` after `mod layers;`. In `step.rs` replace `use super::{cycles, doors, layers};` with `use super::{cycles, doors, layers, seams};` and, in `findings`, after the `layers::findings` line, add `found.extend(seams::findings(tree, workspace));`.

- [ ] **Step 5: Run the tests, record the proof, verify and commit**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline seams && cd tests && cargo test --locked --test workflows architecture_rules`
Expected: 1 unit test and 8 contract tests pass.

Add `"a_seam_serving_one_outside_caller_is_refused_unless_excused"` to the proofs in docs/gates.toml and, under the `architecture` directory, the tree line:

```text
│   │   │   │   ├── seams.rs                    # ARC-005: a seam a door offers serves two callers
```

Run `just docs` and `timeout 1800 just check > /tmp/just-check.log; echo $?`, expected `0`.

```bash
git add -A gate/src tests/ci docs .github README.md
git commit -S -m "feat: refuse seams that serve a single caller"
```

---

### Task 10: ARC-006 and ARC-007, roots and paths

**Files:**
- Create: `gate/src/steps/architecture/roots.rs`
- Modify: `gate/src/steps/architecture/mod.rs` (add `mod roots;`), `gate/src/steps/architecture/step.rs`, tests/ci/architecture_rules.rs, docs/gates.toml, .github/copilot-instructions.md

**Interfaces:**
- Produces: `roots::thin_roots(tree: &Tree, workspace: &Path) -> Vec<Finding>` (ARC-006) and `roots::path_attributes(tree: &Tree, workspace: &Path) -> Vec<Finding>` (ARC-007).

- [ ] **Step 1: Write the failing contract tests**

Append to tests/ci/architecture_rules.rs:

```rust
#[test]
fn a_binary_root_beyond_declarations_and_a_short_main_is_refused() {
    let long_main = format!("fn main() {{\n{}}}\n", "    run();\n".repeat(25));
    let main = format!("//! Binary.\nmod app;\nuse app::run;\n\nstruct Config;\n\n{long_main}");
    let f = project(&[
        ("src/main.rs", main.as_str()),
        ("src/app.rs", "//! App.\npub(crate) fn run() {}\n"),
    ]);
    fs::remove_file(f.root.join("project/src/lib.rs")).unwrap();
    refused(&f.run("ci", "architecture"), "module structure: 2 findings");
    assert_eq!(
        report(&f),
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
    let f = project(&[
        ("src/lib.rs", "//! Crate.\nmod parts;\n"),
        (
            "src/parts.rs",
            "//! Parts.\n#[path = \"elsewhere/inner.rs\"]\nmod inner;\ninclude!(\"generated.rs\");\npub(crate) const TEXT: &str = include_str!(\"text.txt\");\n",
        ),
    ]);
    refused(&f.run("ci", "architecture"), "module structure: 2 findings");
    assert_eq!(
        report(&f),
        concat!(
            "ARC-007 project/src/parts.rs:2: `#[path]` makes the module tree differ from the ",
            "file tree; move the file where its module is declared\n",
            "ARC-007 project/src/parts.rs:4: `include!` of Rust source hides code from the ",
            "module tree; declare it with `mod`\n",
        )
    );
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd tests && cargo test --locked --test workflows architecture_rules`
Expected: the two new tests FAIL.

- [ ] **Step 3: Write `roots.rs`**

```rust
//! ARC-006 and ARC-007: a binary's root declares its modules and calls into
//! them from a short `main`, and the module tree is the file tree: no
//! `#[path]` attribute and no `include!` of Rust source.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::Tree;
use crate::checks::rust_code::{Item, is_identifier_byte, line_at};
use std::path::Path;

/// The lines `fn main` may span, from its signature to its closing brace.
const MAIN_LINES: usize = 25;

/// ARC-006: every item of a binary's root beyond declarations and a short
/// `fn main`.
pub(super) fn thin_roots(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    if !tree.kinds.iter().any(|kind| kind == "bin") {
        return Vec::new();
    }
    let Some(root) = tree.modules.first() else {
        return Vec::new();
    };
    let file = relative(workspace, &root.file);
    root.items
        .iter()
        .filter(|item| !item.is_test())
        .filter_map(|item| {
            problem(item).map(|message| Finding::new("ARC-006", file.clone(), item.line, message))
        })
        .collect()
}

/// What is wrong with one item of a binary's root, if anything.
fn problem(item: &Item) -> Option<String> {
    let lines = item.text.lines().count();
    match (item.kind.as_str(), item.name.as_str()) {
        ("use" | "extern crate", _) => None,
        ("mod", _) if item.is_module_file() => None,
        ("fn", "main") if lines <= MAIN_LINES => None,
        ("fn", "main") => Some(format!(
            "fn main spans {lines} lines; keep it within {MAIN_LINES} and call into modules"
        )),
        (kind, _) => Some(format!(
            "a binary root holds only mod and use declarations and fn main; move this {kind} \
             into a module"
        )),
    }
}

/// ARC-007: every `#[path]` attribute and every `include!` of the tree.
pub(super) fn path_attributes(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for module in &tree.modules {
        let file = relative(workspace, &module.file);
        let code = module.code.as_str();
        for (offset, _) in code.match_indices("#[") {
            let compact: String = code
                .get(offset..)
                .unwrap_or_default()
                .chars()
                .take_while(|&c| c != ']')
                .filter(|c| !c.is_whitespace())
                .collect();
            if compact.starts_with("#[path=")
                || (compact.starts_with("#[cfg_attr(") && compact.contains(",path="))
            {
                found.push(Finding::new(
                    "ARC-007",
                    file.clone(),
                    line_at(code, offset),
                    "`#[path]` makes the module tree differ from the file tree; move the file \
                     where its module is declared"
                        .to_owned(),
                ));
            }
        }
        for (offset, _) in code.match_indices("include!") {
            let standalone = offset
                .checked_sub(1)
                .and_then(|previous| code.as_bytes().get(previous))
                .is_none_or(|&byte| !is_identifier_byte(byte));
            if standalone {
                found.push(Finding::new(
                    "ARC-007",
                    file.clone(),
                    line_at(code, offset),
                    "`include!` of Rust source hides code from the module tree; declare it with \
                     `mod`"
                        .to_owned(),
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{path_attributes, thin_roots};
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_binary_root_holds_declarations_and_a_short_main_only() {
        let long_main = format!("fn main() {{\n{}}}\n", "    run();\n".repeat(25));
        let main = format!("mod app;\nuse app::run;\nstruct Config;\n{long_main}");
        let tree = sample(&["bin"], &[("main.rs", &main), ("app.rs", "")]);
        let found: Vec<String> = thin_roots(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "ARC-006 src/main.rs:3: a binary root holds only mod and use declarations and \
                 fn main; move this struct into a module",
                "ARC-006 src/main.rs:4: fn main spans 27 lines; keep it within 25 and call into \
                 modules",
            ]
        );
        let library = sample(&["lib"], &[("lib.rs", "struct Config;\n")]);
        assert!(thin_roots(&library, Path::new("/w")).is_empty());
    }

    #[test]
    fn path_attributes_and_rust_includes_are_refused_but_include_str_is_not() {
        let tree = sample(
            &["lib"],
            &[
                ("lib.rs", "mod parts;\n"),
                (
                    "parts.rs",
                    "#[path = \"elsewhere.rs\"]\nmod inner;\n#[cfg_attr(test, path = \"t.rs\")]\nmod other;\ninclude!(\"generated.rs\");\nconst TEXT: &str = include_str!(\"text.txt\");\n",
                ),
            ],
        );
        let found: Vec<String> = path_attributes(&tree, Path::new("/w"))
            .iter()
            .map(ToString::to_string)
            .collect();
        let path = "`#[path]` makes the module tree differ from the file tree; move the file \
                    where its module is declared";
        assert_eq!(
            found,
            [
                format!("ARC-007 src/parts.rs:1: {path}"),
                format!("ARC-007 src/parts.rs:3: {path}"),
                "ARC-007 src/parts.rs:5: `include!` of Rust source hides code from the module \
                 tree; declare it with `mod`"
                    .to_owned(),
            ]
        );
    }
}
```

- [ ] **Step 4: Wire it into the step**

In `mod.rs` add `mod roots;` after `mod layers;`. In `step.rs` replace `use super::{cycles, doors, layers, seams};` with `use super::{cycles, doors, layers, roots, seams};` and, in `findings`, after the `seams::findings` line, add:

```rust
        found.extend(roots::thin_roots(tree, workspace));
        found.extend(roots::path_attributes(tree, workspace));
```

- [ ] **Step 5: Run the tests, record the proofs, verify and commit**

Run: `cargo test --manifest-path gate/Cargo.toml --locked --offline roots && cd tests && cargo test --locked --test workflows architecture_rules`
Expected: 2 unit tests and 10 contract tests pass.

Add both new contract test names to the proofs in docs/gates.toml and, under the `architecture` directory, the tree line:

```text
│   │   │   │   ├── roots.rs                    # ARC-006 and ARC-007: thin binary roots, and the module tree is the file tree
```

Run `just docs` and `timeout 1800 just check > /tmp/just-check.log; echo $?`, expected `0`.

```bash
git add -A gate/src tests/ci docs .github README.md
git commit -S -m "feat: keep binary roots thin and the module tree the file tree"
```

---

### Task 11: rust-workflows holds the rules itself

**Files:**
- Move: tests/harness/native_runtime.rs to tests/native_runtime.rs
- Modify: tests/native_windows.rs, tests/gate/layer_boundaries.rs, tests/gate/mod.rs, justfile, docs/standards/engineering.md, CONTRIBUTING.md, docs/rust-gate.md, .github/copilot-instructions.md
- Delete: tests/gate/acyclic_imports.rs
- Create: maestro-quality.toml

- [ ] **Step 1: Run the rules on this repository and see what they find**

Run:

```bash
cargo build --manifest-path gate/Cargo.toml --locked --offline --quiet
for project in gate tests examples/binary examples/library examples/workspace; do
  scratch=$(mktemp -d)
  PROJECT="$PWD/$project" REPORTS="$scratch" RUNNER_TEMP="$scratch" GITHUB_WORKSPACE="$PWD" \
    GITHUB_STEP_SUMMARY="$scratch/summary.md" \
    cargo run --manifest-path gate/Cargo.toml --locked --offline --quiet -- architecture || true
done
```

Expected findings, and nothing else: ARC-007 on tests/native_windows.rs line 5; ARC-005 on `gate/src/runner/mod.rs` for `enter` and `add_to_path`; ARC-005 on tests/harness/mod.rs for `Described`, `describe_text` and `helper_action`. Any other finding is a reader bug to fix in the checks modules first, with a unit test reproducing it.

- [ ] **Step 2: Give the native suite a module tree without `#[path]`**

`git mv tests/harness/native_runtime.rs tests/native_runtime.rs`, then in tests/native_windows.rs replace

```rust
#[path = "harness/native_runtime.rs"]
mod harness;

use crate::harness::{checked, registry, root, temporary};
```

with

```rust
mod native_runtime;

use crate::native_runtime::{checked, registry, root, temporary};
```

and every remaining `harness::` in that file with `native_runtime::`. In tests/gate/layer_boundaries.rs, in `modules_of_the_tests_reach_the_gate_only_through_the_harness`, treat the native suite as a crate of its own whose one door is `native_runtime.rs`: replace

```rust
        if path
            .parent()
            .is_some_and(|parent| parent.ends_with("harness"))
        {
```

with

```rust
        // The native suite is a crate of its own; native_runtime.rs is its
        // one door, as the harness is the workflows crate's.
        if path
            .parent()
            .is_some_and(|parent| parent.ends_with("harness"))
            || name == "native_runtime.rs"
        {
```

and `if name == "workflows.rs" {` with `if name == "workflows.rs" || name == "native_windows.rs" {`. Check it still compiles: `cargo check --manifest-path tests/Cargo.toml --locked --features native-windows --tests`.

- [ ] **Step 3: Declare this repository's layers and exceptions**

maestro-quality.toml:

```toml
# The organization's quality rules as this repository shapes them: the
# layers its two layered crates declare, and the seams that keep one caller,
# each with its reason. Read by rust-gate architecture.

[[crate]]
root = "gate/src/main.rs"
layers = ["steps", "checks", "runner"]

[[crate]]
root = "tests/workflows.rs"
layers = ["ci gate nightly publishers repository", "harness"]

[[exception]]
rule = "ARC-005"
path = "gate/src/runner/mod.rs"
item = "enter"
reason = "the step registry is the only thing that can run a step"

[[exception]]
rule = "ARC-005"
path = "gate/src/runner/mod.rs"
item = "add_to_path"
reason = "the fourth GITHUB_* writer, beside export, output and summary"

[[exception]]
rule = "ARC-005"
path = "tests/harness/mod.rs"
item = "Described"
reason = "the gate's description of itself is read through the harness alone"

[[exception]]
rule = "ARC-005"
path = "tests/harness/mod.rs"
item = "describe_text"
reason = "the gate's description of itself is read through the harness alone"

[[exception]]
rule = "ARC-005"
path = "tests/harness/mod.rs"
item = "helper_action"
reason = "workflow and action YAML are read through the harness alone"
```

Rerun the loop of Step 1. Expected: every project reports `Module structure: no finding`, with 2 excused for gate and 3 for tests.

- [ ] **Step 4: Run the rules in `just check`**

In the justfile `check` recipe, after the line `cargo test --manifest-path gate/Cargo.toml --locked --offline`, add:

```bash
    # The organization's module structure rules hold this repository too:
    # every project here goes through the step consumers run.
    for project in gate tests examples/binary examples/library examples/workspace; do
      scratch=$(mktemp -d)
      PROJECT="$PWD/$project" REPORTS="$scratch" RUNNER_TEMP="$scratch" \
        GITHUB_WORKSPACE="$PWD" GITHUB_STEP_SUMMARY="$scratch/summary.md" \
        cargo run --manifest-path gate/Cargo.toml --locked --offline --quiet -- architecture
      rm -rf "$scratch"
    done
```

- [ ] **Step 5: Retire the tests the step replaces**

1. `git rm tests/gate/acyclic_imports.rs` and remove `mod acyclic_imports;` from tests/gate/mod.rs.
2. In tests/gate/layer_boundaries.rs delete `every_harness_door_item_serves_a_module_outside_it`, `every_door_item_serves_two_modules_or_names_its_reason`, the constant `HYPOTHETICAL_SEAMS` and the helpers only they call (`offered`, `imported`); replace `imports_flow_one_way_through_the_layer_gates` with:

```rust
#[test]
fn a_step_offers_only_its_declaration_and_reaches_no_sibling() {
    // The layers are ARC-004, declared in maestro-quality.toml and held by
    // `rust-gate architecture` in `just check`. What stays here is this
    // crate's own shape: a step's only door is its declaration, `STEPS`; a
    // file inside a step's directory reaches no further than the step; and
    // main.rs reaches into no layer. The registry is the steps door's
    // implementation, the one module that names every step.
    let src = root().join("gate/src");
    let mut checked = 0;
    for (name, path) in layer_files(&src.join("steps")) {
        if name == "mod.rs" || name == "registry.rs" || name.ends_with("/mod.rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let body = text.split("#[cfg(test)]").next().unwrap_or_default();
        checked += 1;
        if name.contains('/') {
            assert!(
                !body.contains("pub(crate) "),
                "{name} is a step's internal seam; it reaches no further than the step"
            );
            continue;
        }
        assert!(
            !body.contains("super::"),
            "{name} reaches a sibling step through super::"
        );
        for line in body.lines().filter(|line| line.starts_with("pub(crate) ")) {
            assert!(
                line.starts_with("pub(crate) const STEPS:"),
                "{name} exposes {line:?}; a step's only door is STEPS"
            );
        }
    }
    let main = fs::read_to_string(src.join("main.rs")).unwrap();
    assert!(!main.contains("crate::"), "main.rs reaches into a layer");
    assert!(checked > 25, "only {checked} step modules checked; the walk drifted");
}
```

Remove any `use` the deleted tests alone needed; `cargo clippy --manifest-path tests/Cargo.toml --all-targets --locked -- -D warnings` names them.

- [ ] **Step 6: Point the documents at the new proofs**

1. docs/standards/engineering.md: make the P-010 row read

```markdown
| P-010 | Composition over inheritance | Partly deterministic: the gate composes three layers, runner, checks and steps, declared in `maestro-quality.toml` and held by `rust-gate architecture` in `just check`: ARC-004 keeps their imports flowing one way (`declared_layers_refuse_an_import_within_or_against_the_order`) and ARC-001 refuses an import cycle in any crate, the tests included (`an_import_cycle_between_two_files_is_refused_by_name`). |
```

   and add under "Rust rules" the bullet

```markdown
- Hold the module structure rules ARC-001 to ARC-007 of the organization
  quality gate, `docs/superpowers/specs/2026-09-24-org-quality-gate-design.md`;
  `just check` runs `rust-gate architecture` on every project here.
```

2. CONTRIBUTING.md: replace

```text
cycle, the tests included: `every_crate_has_an_acyclic_import_graph` names the
cycle it finds.
```

   with

```text
cycle, the tests included: `rust-gate architecture`, run by `just check`,
names the cycle it finds (`an_import_cycle_between_two_files_is_refused_by_name`).
```

   then replace the one remaining `every_crate_has_an_acyclic_import_graph` in the file with `an_import_cycle_between_two_files_is_refused_by_name`.
3. docs/rust-gate.md: replace

```text
The compiler keeps the layers apart: a step is private to `gate/src/steps/`, so
neither the checks nor the runner can reach it. The test
`imports_flow_one_way_through_the_layer_gates` keeps the rest: no step imports
another step, no check imports a step, and a step exposes nothing but `STEPS`.
```

   with

```text
The compiler keeps the layers apart: a step is private to `gate/src/steps/`, so
neither the checks nor the runner can reach it. ARC-004, declared in
`maestro-quality.toml`, keeps the imports one way, and
`a_step_offers_only_its_declaration_and_reaches_no_sibling` the rest: no step
imports another step, and a step exposes nothing but `STEPS`.
```

4. .github/copilot-instructions.md: remove the `acyclic_imports.rs` line; move the `native_runtime.rs` line from under the `harness` directory to the tests directory's files, before `native_windows.rs`, described `Real native Windows processes, registry boundary and temporary trees, the native suite's one door`; change the `layer_boundaries.rs` description to `This crate's own step shape, and the harness as the tests' one door`; add at the root, after `justfile`:

```text
├── maestro-quality.toml                        # The layers this repository's crates declare, and its reasoned exceptions
```

- [ ] **Step 7: Verify and commit**

Run: `timeout 1800 just check > /tmp/just-check.log; echo $?`
Expected: `0`, the loop printing `Module structure: no finding` five times.

```bash
git add -A tests justfile maestro-quality.toml docs CONTRIBUTING.md .github
git commit -S -m "test: hold this repository to its own architecture rules"
```

---

### Task 12: Parity, then the pull request

- [ ] **Step 1: Show the step refuses what the deleted tests refused**

```bash
scratch=$(mktemp -d)
cp -r gate "$scratch/gate"
printf '\n/// A step reached from the runner.\npub(crate) fn upward() { crate::steps::run("", ""); }\n' \
  >> "$scratch/gate/src/runner/outcome.rs"
printf '[[crate]]\nroot = "gate/src/main.rs"\nlayers = ["steps", "checks", "runner"]\n' \
  > "$scratch/maestro-quality.toml"
PROJECT="$scratch/gate" REPORTS="$scratch" RUNNER_TEMP="$scratch" GITHUB_WORKSPACE="$scratch" \
  GITHUB_STEP_SUMMARY="$scratch/summary.md" \
  cargo run --manifest-path gate/Cargo.toml --locked --offline --quiet -- architecture
cat "$scratch/architecture.txt"
rm -rf "$scratch"
```

Expected: the step fails; the report holds an ARC-001 cycle through `gate/src/runner/outcome.rs` and an ARC-004 line `crate::steps::run imports steps from runner`, the two rules the deleted tests held. The copy's exceptions are absent, so the ARC-005 findings for `enter` and `add_to_path` appear too.

- [ ] **Step 2: Final gate**

Run: `timeout 1800 just check > /tmp/just-check.log; echo $?` and `git log --oneline origin/main..HEAD`
Expected: `0`; the spec, this plan and the seven task commits.

- [ ] **Step 3: Ask before anything leaves the machine**

Pushing the branch and opening the pull request are outward actions: ask the owner, then `git push -u origin feat/org-quality-gate` and `gh pr create` titled `feat: refuse module structure faults with rust-gate architecture`, the body listing ARC-001 to ARC-007, the preview input, the exceptions taken and the tests retired.
