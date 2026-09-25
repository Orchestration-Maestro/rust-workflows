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
| TST-003 | Every package | `{n} integration-test crates build without required-features ({names}); fold them into one test crate, tests/it/main.rs with its modules beside it` |
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

1. TST-003 advises tests/it/main.rs with its modules beside it, not a tests/main.rs: Cargo builds every file directly in the tests directory as a crate of its own, a module that such a root declares included, while a subdirectory's `main.rs` and its siblings form one crate.
2. `lockfile_tracked` asks git only when a `.git` entry sits above the root, so
   a project outside any repository says nothing on stderr and skips the
   question instead of printing `fatal: not a git repository`.
3. `Inheritance` lists the settings a member does not inherit rather than
   holding four booleans, which Clippy's `struct_excessive_bools` refuses.
4. This repository's example tests take names of four words or more, which the
   retired naming test never asked of the examples, and the repository test
   that reads the example manifests resolves the values a member inherits from
   the workspace root.
5. Phase B keeps the rules the gate reads itself, HYG-001 to HYG-005, SIZE-003
   for shell scripts and justfiles, and DUP-001. DOC-002 (lychee), DOC-003
   (rumdl), shellcheck, shfmt and editorconfig-checker move to phase C's hook
   set, which CI runs through prek exactly as a commit does: pinning them a
   second time in the `hygiene` step would give each tool two installers and
   two versions to keep in step, the drift this gate exists to remove.
6. The HYG refusals read as `docs/ci.md` lists them; a DUP-001 exception names
   the group's first function, by file and name, since a group has no single
   file.
7. When similarity-rs itself fails, `duplication` still reports `NOT MEASURED`
   and passes, as before this plan: failing the run on a tool outage would make
   every consumer's CI depend on one tool's uptime. Cost if wrong: a broken
   similarity-rs lets a third copy through until the report is read.
8. LNT-001's manifest and `clippy.toml` checks run in `architecture`, not in
   `quality`: they read manifests like the other source rules, stay behind
   `quality-preview` until phase E, and give the commit hook the same refusal.
   The list adds `cognitive_complexity` and `excessive_nesting`, which SIZE-001
   names and whose `clippy.toml` thresholds do nothing while the lints are off;
   `unsafe_code` stays with the `quality` step, per `unsafe-policy`; the `cargo`
   group applies to every package, since Clippy's `cargo_common_metadata`
   skips `publish = false`.
9. The commands a developer or a hook runs are steps of a `local` workflow:
   `rust-gate lints --write` names the step `lints --write`. The registry test
   takes the justfile as what runs a local step; `just docs` rewrites every
   crate's block.
10. The module reader gives an inner attribute to its module, never to the
    next item, so an integration-test crate may open with `#![cfg(test)]`,
    which Clippy needs to read the whole crate as test code.
11. `hygiene` reads the new files git does not ignore as well as the tracked
    ones, so a local run sees what the next commit adds; HYG-001 leaves a
    marker in backticks alone, since it names the word and leaves no work.
12. The files every repository holds as they are, `.editorconfig`,
    `.gitattributes`, `.taplo.toml`, `.yamlfmt.yml` and `rust-toolchain.toml`,
    are this repository's own, read into the gate when it is built: one copy,
    changed here and nowhere else. The rest are rendered from the gate's data
    and `maestro-quality.toml`, which gains `[ci]`, the inputs a caller
    passes.
13. The home of the reusable workflows, the repository whose
    `.github/workflows/ci.yml` declares `workflow_call`, keeps its own
    `ci.yml`, Dependabot settings and hooks. A caller's pin is read from its
    own `uses:` lines by `sync`, `sync --check` and `managed-files`;
    `RUST_WORKFLOWS_PIN` moves it, and `init` requires it. `deny.toml` joins the
    managed files with DEP-001 in phase D, and `.pre-commit-config.yaml` and
    `.rumdl.toml` with the hook set; a non-Rust caller comes with `hygiene.yml`.
14. `describe` gathers each workflow's steps, so one module declares a
    `ci.yml` step and the local commands that share its code in one `STEPS`.
15. The hook set is rendered into every repository's `.pre-commit-config.yaml`
    as local hooks rather than published in a `.pre-commit-hooks.yaml`: prek's
    `rust` language installs a hook repository only from a root `Cargo.toml`,
    which rust-workflows has none of, while a local hook takes
    `cli:<url>:<tag>:rust-gate` from the release tag; one rendering keeps every
    version in the gate. The tools come through prek's `mise` language at the
    versions `mise.toml` pins, versions and not digests, since a managed hook
    environment takes no lockfile; what CI installs itself keeps its digest.
    `.rumdl.toml` joins the managed files, since Markdown lines wrap where
    their writer wraps them; editorconfig-checker runs with
    `-disable-indent-size`, since rustfmt owns the indentation of a continued
    Rust line.
16. `rust-gate architecture --local` and `rust-gate hygiene --local` run the
    step outside Actions, the way a commit hook does: the repository root as
    workspace and project, a scratch directory for the reports, the report
    printed when the step refuses. `rust-gate check`, every CI step locally,
    waits for after v2.0.0: CI runs the whole gate on every pull request, and
    no rule depends on it.
17. COV-001 needs no preview: `validate` refuses a threshold under 90 and every
    workflow's default is 90, and a caller meets both only by moving its pin.
    COV-002 and PRL read the change as the mutation step does, the merge
    commit against its first parent, and need no network; PRL-001 counts a
    test as touched when a file under a `tests` directory changes or a new line
    falls inside a top-level `cfg(test)` item of the changed file.
18. VET-001 checks the six imports by name and URL before `cargo vet --locked`
    runs; the organization's audits file lives at rust-workflows'
    `supply-chain/audits.toml`, read from `main`. Until it is merged there, a
    ledger locks that import as empty, which is what the file holds, and
    `cargo vet regenerate imports` refreshes it afterwards. taplo leaves
    `supply-chain` to `cargo vet fmt`.
19. PRF-001 pins gungraun 0.19.4: the runner must match the library the
    benchmarks link, so the step refuses a lockfile on another version rather
    than installing a runner per repository. Valgrind comes from the runner
    image's archive at `1:3.22.0-0ubuntu3`, installed by the step only when a
    bench is declared; the runner is one pinned row of `install-tools`. The
    base runs in a `git worktree` of the merge commit's first parent, both runs
    sharing one gungraun home, so the second compares with the first.
20. The pull request of phases A to D carries `feat!:`: it renames the example
    packages, which the API step, at the pinned gate, reads as a break, and it
    changes two defaults a caller meets on moving its pin, the coverage floor
    and the dependency audit. Release-please proposes 2.0.0 from it, and phase
    E's repin joins that release rather than opening it.
21. The first run of phase E's pin showed two faults, fixed in a pull request of
    their own before the repin moves to it: the action is fetched as the
    repository's archive, which left out `.editorconfig` and `.gitattributes`,
    two files the gate reads when built, so neither is export-ignore any more
    and a test holds every file the gate reads to the archive; and mise, even
    through the aqua backend, asks GitHub's API for each release, so the
    `hooks` step and the live hooks test pass it the job's read-only token as
    `MISE_GITHUB_TOKEN`, which no hook reads.
22. The second run of phase E's pin showed the `hooks` step running prek over
    the home of the workflows, whose hooks use its local toolbelt: the pinned
    gate did not know the home yet. The gate's part of phase E ships first,
    behind `quality-preview` still: the home check, shared by `sync` and
    `hooks`, an `applied` output on each new step, and one scorecard control
    per rule family. The repin and the removal of `quality-preview` follow,
    pinned to that commit.
23. The repin names `12f8b53`, the gate of rulings 21 and 22 on top of phases A
    to D, and removes `quality-preview` with every guard it held: from here on
    every call runs every rule, and a caller that still passes the input fails
    to start.
24. prek fetches the latest mise when none is on PATH, and a release it found
    had no assets. The `hooks` step of `ci.yml` and `hygiene.yml` gets the mise
    `scripts/bootstrap.sh` pins, installed with its digest like every other
    tool; the version test reads mise's pin from `bootstrap.sh`.
25. Phase G's first sync left release-canary's `release.yml` at v1.2.1: `sync`
    moved the caller alone, and Dependabot ignores rust-workflows. `sync` now
    moves every call to rust-workflows in `.github/workflows/` and the
    organization's workflow templates to the caller's release, and
    `sync --check` refuses one left behind. release-please, on creating a
    Release, sends `rust-workflows-release` to `.github`, whose quality-sync
    workflow starts on it; the daily run stays as the net.
26. The gate's rules are one list, `gate/src/checks/gate_rules.tsv`: the ID,
    a short name, whether an exception is allowed, and one line. The gate
    embeds it, takes its allowed exceptions from it and prints it with
    `rust-gate gate-rules`; `just docs` indexes it in `docs/ci.md`; the
    organization's page renders it at every release through the sync. A test
    refuses a rule ID the gate's code names that the list lacks, and a row of
    `docs/ci.md` that names one; COV-001, which no table named, now has its row.
27. Naming, Rust: LNT-001 denies rustc's `nonstandard_style`, so the case of
    every name is the gate's, and Clippy's `same_name_method`; the Rust API
    Guidelines' naming conventions already came with `all` and `pedantic`.
    `renamed_function_params` stays out: it demands the `f` of `fmt`, which
    `min_ident_chars` refuses. NAME-003 refuses a feature that is not
    lowercase kebab-case or starts with `use-`, `with-`, `enable-`, `has-` or
    `feature-`; NAME-004 refuses, in the code a `maestro-` package ships, an
    environment variable read by name that neither starts with `MAESTRO_` nor
    is the platform's, and takes an exception for one another tool owns. The
    repository becomes `maestro-rust-workflows` at a later cutover: `sync`
    reads and moves a pinned call under either name to the one constant
    `HOME` renders, `rust-workflows` until the rename flips it; the Dependabot
    ignore, the attestation's signer workflow and the audits URL derive from
    it too, so flipping `HOME` is the whole rename for the gate's code. The
    prose that names today's repository moves at the cutover: `SECURITY.md`,
    `docs/publishing.md`, the README, `docs/ci.md` and the tests' fixtures.
