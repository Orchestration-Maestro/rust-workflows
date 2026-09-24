# Organization quality gate, plan 2 of 2: everything after the architecture rules

> **For agentic workers:** executed inline in the owner's session (same model); steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** finish the organization quality gate the spec describes: every remaining rule family in `rust-gate`, the generated files and their sync, release v2.0.0, the organization automation, and every repository brought onto it.

**Architecture:** one plan in seven phases, executed back to back. Phases A to D extend `rust-gate` on branch `feat/org-quality-gate-2`, stacked on pull request #30; phase E repins and releases; phase F changes the `.github` repository; phase G brings the other repositories on. Each phase ends with `just check` green and a signed commit.

**Tech Stack:** as plan 1: Rust 1.98.1 edition 2024, standard library only in the gate, jaq 3.1.1 for TOML and JSON, `cargo metadata --no-deps --offline`, contract tests in the workflows test crate.

**Spec:** docs/superpowers/specs/2026-09-24-org-quality-gate-design.md. Plan 1, merged or in review as pull request #30, delivered ARC-001 to ARC-007 and the reader this plan builds on.

## How this plan is written

The owner asked for one plan instead of seven. This plan fixes every contract up front: the rules, where each is measured, its refusal text, its proof tests, the files touched and the order. The code is written test-first in the commits, by the same session that wrote the plan; each phase's commit carries its tests. A phase whose contract turns out wrong gets a ruling in this plan's execution record before the code moves.

## Global Constraints

Plan 1's constraints hold, in particular:

- The gate stays standard-library only; TOML and JSON go through the pinned jaq.
- Never commit unless `timeout 1800 just check` exits 0; signed commits; conventional titles, lowercase after the type, subject within 71 characters, lines within 80 columns.
- Every item of the gate documented; Clippy pedantic with `-D warnings`; functions within 100 lines, cognitive complexity 15, 5 parameters; files within 500 lines; lines within 100 columns.
- Every refusal the gate composes is asserted by a test in its own words; every step is run by a contract test; a workflow step's `name:` equals its step's summary.
- New `ci.yml` steps run behind `if: ${{ inputs.quality-preview }}` until phase E.
- Directory steps declare `pub(crate) const STEPS` in `step.rs`; step and checks code names siblings by path, never through a `use super::{...}` group; checks functions that run a tool carry names no step calls for another reason (the registry scanner matches names).
- Backticked paths in this repository's Markdown exist; write future paths without backticks.
- Outward actions (push, pull request, merge, tag, release, organization settings) wait for the owner's go.

## Phases

| Phase | Delivers | Repository | Estimate |
| --- | --- | --- | --- |
| A | SIZE-002, SIZE-003, NAME, DOC-001, LIB, TST-001, TST-003, WSP in `rust-gate architecture`; `[limits]` in `maestro-quality.toml` | rust-workflows | 1 day |
| B | `rust-gate hygiene`: HYG-001 to HYG-005, DOC-002, DOC-003, shell width and format; DUP-001 enforced; new pinned tools | rust-workflows | 1.5 days |
| C | LNT-001, the generated files, `sync`, `sync --check`, `init`, `lints --write`, `managed-files`, the hooks manifest, `hygiene.yml`, TST-004 | rust-workflows | 2 days |
| D | COV-001, COV-002, PRL-001, PRL-002, DEP-001, VET-001, PRF-001 | rust-workflows | 2 days |
| E | Repin the gate, remove `quality-preview`, scorecard controls, gates always on, release v2.0.0 | rust-workflows | 0.5 day |
| F | `stack` property, `hygiene-required` ruleset, `quality-sync.yml`, audit extension, template on v2.0.0 | `.github` | 1 day |
| G | Sync pull requests with their fixes | maestro-model-router, maestro-core, release-canary | 2 to 3 days |

Pull requests: phases A to D reach `main` as one pull request once #30 is merged; the repository's rule of one pull request per session makes that the next session's, unless the owner lifts it. Phase E is the second pull request of the gate procedure. Phases F and G are one pull request per repository.

---

## Phase A: source and manifest rules

All rules run in `rust-gate architecture`, over the trees plan 1 reads plus the packages `cargo metadata` lists. The step's summary, and the `ci.yml` step name, becomes `Source rules ARC, SIZE, NAME, DOC, LIB, TST and WSP`.

### A1. Packages and limits in the checks layer

**Files:** create `gate/src/checks/manifests.rs`; modify `gate/src/checks/module_tree.rs`, `gate/src/checks/quality_config.rs`, `gate/src/checks/mod.rs`.

**Interfaces:**
- `manifests::read_cargo_metadata(project: &Path, temp: &Path) -> Result<PathBuf, Failure>` runs `cargo metadata` once and returns the file; `module_tree::module_trees(metadata: &Path)` reads that file instead of running cargo itself.
- `manifests::cargo_packages(metadata: &Path) -> Result<Vec<Package>, Failure>`, `Package { name, publishable, manifest: PathBuf, edition, dependencies: Vec<String> (normal only), library: bool, binary: bool, plain_tests: Vec<String> (test targets without required-features) }`.
- `manifests::workspace_of(metadata: &Path) -> Result<Workspace, Failure>`, `Workspace { root: PathBuf, members: usize }`.
- `manifests::member_inheritance(manifest: &Path) -> Result<Inheritance, Failure>`, read through `jaq --from toml`: which of `[lints]`, `edition`, `rust-version`, `license` are inherited, and the dependency names not declared `{ workspace = true }` in `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and target tables.
- `manifests::root_resolver(manifest: &Path) -> Result<Option<String>, Failure>`: the `resolver` of a virtual workspace root, `None` for a root that is a package.
- `module_tree::Module` gains `source: String`, the file as written.
- `quality_config::QualityConfig` gains `limits: Limits { file_lines: usize, line_columns: usize }`, default 500 and 100, read from `[limits]` keys `file-lines` and `line-columns`.

**Refusals:** `maestro-quality.toml: unknown limit `{key}`; it takes file-lines, line-columns`; `maestro-quality.toml: {key} must be a whole number`; `maestro-quality.toml: {key} = {value} loosens the organization's {floor}; a repository may only tighten it`.

**Unit tests:** the jaq-free parsers of each listing line; limits parsing, tightening and loosening.

### A2. Test code in the reader

**Files:** create `gate/src/checks/rust_tests.rs`.

**Interfaces:** `test_functions(code: &str) -> Vec<(usize, String)>` (line and name of every function under `#[test]`, `#[tokio::test]` or another `::test` attribute, at any depth); `only_tests(code: &str) -> String` (blanked code with everything but `#[cfg(test)]` items blanked); `enclosing_function(code: &str, line: usize) -> Option<String>` (the innermost function whose body holds the line).

**Unit tests:** nested test modules, attributes between `#[test]` and `fn`, async tests, a sleep inside a helper inside a test module.

### A3. The rules

**Files:** create `gate/src/steps/architecture/sizes.rs`, `names.rs`, `sources.rs`, `packages.rs`; modify `step.rs` and `mod.rs`.

| ID | Measured | Refusal (after `RULE file:line:`) |
| --- | --- | --- |
| SIZE-002 | Every module file once: lines not starting, after indentation, with `///` or `//!` | `{n} lines of code, over {limit}; split it by what varies (doc comments are not counted)` |
| SIZE-002 note | Same, over 300 and within the limit | report line `NOTE SIZE-002 {file}: {n} lines of code, over 300; reported, not refused` |
| SIZE-003 | Every line of every module file, by characters | `{n} columns, over {limit}; wrap it, strings and comments included` |
| NAME-001 | Every package | `package `{name}` is not lowercase kebab-case without a -rs or -rust suffix`; `package `{name}` may be published, so its name starts with maestro-; or set publish = false` |
| NAME-002 | Module files of test targets holding tests, crate roots and `mod.rs` aside; every test function of every target | `test module `{stem}` names what it proves in fewer than two words`; `test `{name}` names what it proves in fewer than four words`; `test `{name}` starts with test_; the attribute already says so`; `test `{name}` ends with {suffix}; name what was proven` |
| DOC-001 | Every module file: first non-blank line | `the file does not open with a //! comment saying what the module is for` |
| LIB-001 | Modules of library targets, test items aside | `` `{macro}!` in a library; return the value or report through tracing, and let the binary print `` |
| LIB-002 | Publishable packages with a library target and no binary target | `a library exposes typed errors; `{dep}` belongs in a binary or in [dev-dependencies]` |
| TST-001 | Test code: every module of a test target, test items elsewhere; a path ending `thread::sleep`, `time::sleep` or `task::sleep` | `` `{path}` waits on time; wait on a fake clock or a synchronisation primitive ``; exception item: the enclosing function |
| TST-003 | Every package | `{n} integration-test crates build without required-features ({names}); fold them into one tests/main.rs` |
| WSP-001 | Members of a workspace of two or more | `member `{name}` does not inherit {what} from the workspace`; `member `{name}` declares {deps} without `workspace = true`; declare them in [workspace.dependencies]` |
| WSP-002 | Every package; the workspace root | `package `{name}` uses edition {edition}; the organization builds with 2024`; `the workspace sets resolver {r}; set resolver = "3"`; `Cargo.lock is missing; commit it`; `Cargo.lock is not tracked; commit it` (only where `git` finds a work tree) |

Ruling carried from the design review: LIB-002 applies to packages with a library and no binary. A binary package's library is its own implementation, and the spec's concern is the errors a library hands its callers.

**Contract tests** (tests/ci/source_rules.rs): `oversized_files_and_lines_are_refused_and_three_hundred_is_reported`, `package_names_follow_the_form_and_publishable_ones_the_prefix`, `badly_named_tests_and_one_word_test_modules_are_refused`, `a_file_without_a_module_comment_is_refused`, `printing_from_a_library_is_refused_and_from_a_binary_allowed`, `a_library_depending_on_anyhow_is_refused_but_a_binary_is_not`, `sleeping_in_a_test_is_refused_unless_excused`, `more_than_one_plain_integration_test_crate_is_refused`, `workspace_members_inherit_their_settings_and_dependencies`, `edition_resolver_and_lockfile_are_held`, `tightened_limits_apply_and_loosened_ones_are_refused`.

### A4. This repository holds them

- Rename the publishable example packages: `bounded-arithmetic`, `bounded-sum` and `workspace-arithmetic` gain the `maestro-` prefix; their crate paths, `CARGO_BIN_EXE_` names, lockfiles (`cargo update --workspace --offline`), `ci-internal.yml` dry-run package names and the release payload tests follow.
- The two example integration tests open with a `//!` comment.
- The workspace example inherits `edition`, `rust-version`, `license` and `[lints]` from its root and declares its one internal dependency in `[workspace.dependencies]`.
- Retire tests/repository/naming_rules.rs and, from tests/repository/size_limits.rs, the two file-size tests; the column test stays until phase B covers shell and the justfile. Update the citations in the standards and the Copilot inventory.
- `docs/ci.md`, `docs/gates.toml` and the generated tables: the gate "Module structure" becomes "Source rules" with every proof.

**Exit:** `just check` green, the step reporting no finding on the five projects here.

---

## Phase B: `rust-gate hygiene`

A new step, `hygiene`, in `ci.yml` behind `quality-preview`, over every file `git ls-files` lists in the checkout.

| ID | Measured | Tool |
| --- | --- | --- |
| HYG-001 | `TODO`, `FIXME`, `HACK`, `XXX` in Rust, shell, TOML, YAML and justfile comments without `#123` or an issue URL | gate |
| HYG-002 | Tracked `*.snap.new`, `*.pending-snap` | gate |
| HYG-003 | Tracked file over 500 KB; exception per path | gate |
| HYG-004 | Executable without shebang, shebang file not executable, paths differing only by case, broken symlink or one leaving the repository | gate |
| HYG-005 | `README.md` and `LICENSE` missing; `CHANGELOG.md` missing beside a release-please configuration | gate |
| SIZE-003 shell | Lines over the limit in shell scripts and justfiles | gate |
| Shell format | shellcheck and shfmt on every shell script | shellcheck, shfmt |
| DOC-002 | Relative links and anchors in tracked Markdown | lychee `--offline --include-fragments` |
| DOC-003 | Markdown structure | rumdl |
| DUP-001 | Components of three or more similar functions; the `duplication` step fails on them and keeps listing pairs; exception per path | similarity-rs |

New pinned tools, each with its `# tool:` line in `mise.toml`, its `mise.lock` entry and its `ci.yml` install row with digest: lychee, rumdl, shfmt, editorconfig-checker. This repository's own Markdown and shell must pass first; the column test of tests/repository/size_limits.rs is retired. The speed test of spec section 10 lands here: `architecture` and `hygiene` together under 10 seconds on the largest local repository, measured by a test on this repository with a budget.

---

## Phase C: lints and generated files

- LNT-001: the lint list of spec section 5.10 as data in the gate; `rust-gate lints --write` writes it between markers into `[workspace.lints]` or `[lints]`; the `quality` step refuses a manifest missing any of it and a `clippy.toml` looser than the generated one; a contract test runs Clippy with every lint and key on a fixture, failing on an unknown name.
- `rust-gate sync`, `sync --check` and `init` write and verify the generated files of spec section 7 from templates held in the gate; each file opens with `generated by rust-gate sync; do not edit`.
- A `managed-files` step in `ci.yml` and in the new reusable `hygiene.yml` refuses any difference and prints `run rust-gate sync`.
- `.pre-commit-hooks.yaml` published by this repository: the universal set of spec section 5.13 and the Rust commit and push hooks; a test runs the generated `.pre-commit-config.yaml` in a fresh clone with only prek and rustup on the PATH.
- TST-004: the generated `.config/nextest.toml` and the gate's runtime profile rendered from one source with `retries = 0`.
- This repository adopts its own generated files and loses the tests they replace: `every_crate_holds_the_complexity_limits`, `the_binary_refuses_every_way_to_panic`, and the hand-kept hook set.

---

## Phase D: coverage, pull requests, dependencies, performance

- COV-001: `coverage-threshold` defaults to 90 and refuses a lower value.
- COV-002: a `changed-coverage` step: LCOV intersected with `git diff -U0` against the merge base; 95 % for `feat` and `fix` titles, 90 % otherwise; `max(1, floor((100 - target) % of n))` lines may stay uncovered; not applicable to a push.
- PRL-001 and PRL-002: a `pull-request` step reading the diff and the title.
- DEP-001: the generated `deny.toml` with `multiple-versions = "deny"`, wildcards and unknown sources denied, exceptions rendered from `maestro-quality.toml`.
- VET-001: `dependency-audit` on by default; the `supply-chain/config.toml` import list checked; this repository publishes the organization's audits file.
- PRF-001: a `performance` step, opt-in through `[performance] benches`, gungraun under Valgrind, base against head, 5 % instruction budget.

---

## Phase E: repin and release v2.0.0

After phases A to D are merged: repin every gate call site to that squash commit, remove `quality-preview` and every guard it holds, move the gates in `docs/gates.toml` to `always`, add a scorecard control per rule family, and release v2.0.0 through release-please with a `feat!:` title. Tag and release wait for the owner's go.

---

## Phase F: the organization

In `.github`: `stack` required with values `rust` and `other`; ruleset `hygiene-required` for `stack=other`; `quality-sync.yml` opening one sync pull request per repository on every release, minors merging themselves; `org-drift.yml` extended per spec section 8; the workflow template on v2.0.0. Organization settings change only after the owner's go.

---

## Phase G: the repositories

One sync pull request per repository, each carrying the fixes it needs to pass: maestro-model-router (its `catalog.rs` integration test split, test module doors, lints, one integration test crate), maestro-core (its S0 state as found then), release-canary. Each pull request's fixes are listed from the gate's own report on that repository, never guessed ahead.

## Execution record

Rulings taken while executing, newest last.
