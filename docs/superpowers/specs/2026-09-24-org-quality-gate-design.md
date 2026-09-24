# Organization quality gate: one standard every repository inherits

- Date: 2026-09-24
- Status: approved by the owner on 2026-09-24
- Owner: rust-workflows (engine), `.github` (organization automation)
- Release: rust-workflows v2.0.0, a breaking change for every consumer

## 1. Problem

The quality rules this organization wants already exist, but only as
rust-workflows' own contract tests. A consumer repository receives none of
them as a blocking check, and each repository has started to carry its own
copy of the configuration.

What exists today, by reach:

| Reach | Rules |
| --- | --- |
| Blocking for every consumer through `ci.yml` | rustfmt, Clippy `-D warnings` with `todo` and `dbg_macro` denied, `unsafe_code` denied, rustdoc `-D missing_docs`, line coverage at 80 %, mutation testing on the change, unused dependencies, public API compatibility, dependency and licence policy, secrets, vulnerabilities |
| Blocking for rust-workflows only | No import cycle (`every_crate_has_an_acyclic_import_graph`), one-way layers and one door (`imports_flow_one_way_through_the_layer_gates`), two callers per seam (`every_door_item_serves_two_modules_or_names_its_reason`), function and file sizes and line width (`size_limits.rs`), test names (`naming_rules.rs`) |
| Reported to consumers, never blocking | Function and file sizes (`rust-gate complexity`), duplicated functions (`rust-gate duplication`) |
| Nowhere | Crate names, visibility, the rule of three, module layout, shared hooks, shared configuration |

Measured drift on 2026-09-24, across the four Rust repositories:

| Repository | `clippy.toml` | prek hooks | Files over 500 lines | Lines over 100 columns |
| --- | --- | --- | --- | --- |
| rust-workflows | variant A | 15 | 0 | 0 |
| maestro-core | variant A | 16 | 5 (`chunk_split.rs` holds 1,477) | 19 |
| maestro-model-router | variant B | 12 | 1 (its `catalog.rs` integration test holds 713) | 6 |
| release-canary | none | 0 | 0 | 0 |

maestro-core's `ci.yml` also calls rust-workflows at a placeholder commit,
`0000000000000000000000000000000000000000`.

## 2. Goals

1. Every rule below blocks in CI and, where it is fast enough, at commit time,
   from one source: the `rust-gate` binary of this repository.
2. A repository holds its product code, its product tests and at most one
   configuration file, `maestro-quality.toml`. It holds no quality test and
   no hand-copied configuration.
3. Every shared configuration file in a repository is generated, marked as
   generated, and refused by CI when it differs by one byte from what
   `rust-gate` would write.
4. A release of rust-workflows reaches every repository as a bot pull request,
   without a human copying anything.
5. rust-workflows holds itself to the same rules through the same command.

## 3. Non-goals

- Language packs other than Rust. A repository without Rust receives the
  universal set (section 4.12) only; a Python or TypeScript pack is a later
  design.
- GitHub's ruleset rule "Require workflows to pass before merging". It exists
  only on GitHub Enterprise Cloud; this organization is on GitHub Team, so each
  repository keeps a generated caller workflow instead (section 6).
- Rewriting maestro-core beyond what its S0 fresh start already plans.

## 4. Decisions recorded in the design conversation

| Decision | Choice |
| --- | --- |
| Existing violations when a rule starts blocking | Blocking immediately, no frozen baseline |
| Crate names | `maestro-<domain>` for every publishable crate; form only for `publish = false` |
| Architecture rules | Generic rules everywhere, plus optional declared layers per crate |
| Where the rules live | Everything in `rust-gate` (approach A) |
| Scope of phase 1 | Every rule family in this document, including cargo-vet and the performance gate |
| Coverage | 90 % of all lines; 95 % of changed lines for `feat` and `fix`, 90 % for other types |
| Exceptions | Only for the six rules where a deliberate choice is legitimate (ARC-005, DUP-001, HYG-003, TST-001, DEP-001, PRF-001), each with a reason; a stale exception fails. The conversation started from two rules; the owner confirmed the other four in the review of this spec |
| Distribution | Generated, byte-checked files; automatic sync pull requests on every release |

## 5. Rule catalogue

Every rule has an identifier, fails the run with that identifier, the file
and line, and the fix, and is proven by a failing fixture and a passing one.
Thresholds are organization floors: `maestro-quality.toml` may tighten any of
them and a looser value is refused.

Exceptions exist only where a deliberate choice is legitimate. They are marked
**Exception** below; everything else has none. An exception is an
`[[exception]]` entry with `rule`, `path` and `reason`, and an exception that no
longer matches a violation fails the run, so the list cannot rot (ENF-008).
ARC-005, TST-001 and PRF-001 exceptions also name the `item`: the offered
name, the test function or the benchmark.

### 5.1 Architecture: `ARC`

`rust-gate architecture` reads every crate's module tree from its root file.
It tokenises Rust source itself, standard library only: strings, raw strings,
character literals, line and nested block comments are recognised, so a path
inside a string or a comment is never read as an import. Inline
`#[cfg(test)]` modules are excluded from the import graph.

| ID | Refuses | Measured by |
| --- | --- | --- |
| ARC-001 | An import cycle between files of one crate, directly or through others | Graph of `crate::`, `super::` and declared-child paths; the message names every file of the cycle |
| ARC-002 | A door holding more than doors hold: any `mod.rs`, and `lib.rs` when its crate has other modules, may contain only `mod`, `use`, `pub use` and `pub(...) use` declarations, attributes and doc comments | Top-level items of the door file |
| ARC-003 | A path that walks past a door: when a `mod.rs` re-exports a name, a path from outside that directory to the module defining the name is refused, `crate::m::child::Name` where the door offers `crate::m::Name`. A door that offers a child module itself, `pub(crate) mod child;`, lets the path continue at that child | Resolved path against each door's re-exports |
| ARC-004 | An import against the declared layer order, when `maestro-quality.toml` declares layers for the crate. A module may import only from layers to its right; every top-level module of that crate must belong to exactly one layer | `[[crate]] layers` |
| ARC-005 | A seam serving one caller: an item a `mod.rs` door offers past its parent, that exactly one module outside the directory uses and no module inside shares besides the one defining it, must move next to its caller. Three cases cannot move and are not counted: an offer no wider than `pub(super)`, a use by the crate root, which composes the crate, and an item shared inside the directory. A library's root, its public API, is not a `mod.rs`. **Exception** | Door offers against importers; rustc `unreachable_pub` enforces the visibility half (section 5.10) |
| ARC-006 | A thick binary root: the root of every binary target may hold only `mod` and `use` declarations, attributes, doc comments and a `fn main` spanning at most 25 lines, signature and closing brace included | Top-level items and `main`'s span |
| ARC-007 | A module tree that differs from the file tree: `#[path = ...]` attributes and `include!` of Rust source. `include_str!` and `include_bytes!` stay allowed | Attribute and macro scan |

ARC-007 is what makes the other six exact: with it, the directory layout is the
module tree, and the scanner never has to evaluate a path attribute.

### 5.2 Size: `SIZE`

| ID | Refuses | Measured by |
| --- | --- | --- |
| SIZE-001 | A function over 100 lines, over cognitive complexity 15, over 5 parameters or nested over 4 levels, in every target including tests | Clippy `too_many_lines`, `cognitive_complexity`, `too_many_arguments`, `excessive_nesting`, thresholds from the generated `clippy.toml` |
| SIZE-002 | A file over 500 lines of code; doc comments (`///`, `//!`) are not counted, so explaining an item is never the reason to split its module. Files over 300 lines are reported | `rust-gate architecture` |
| SIZE-003 | A line over 100 columns in Rust and shell sources, comments and strings included, since rustfmt does not wrap either | `rust-gate architecture` |

### 5.3 Duplication, the rule of three: `DUP`

| ID | Refuses | Measured by |
| --- | --- | --- |
| DUP-001 | Three or more functions of at least 8 lines that are similar at 0.9 or above: the third copy is the signal to extract. A pair is reported, not refused. **Exception**, for shapes shared on purpose | similarity-rs pairs, grouped into connected components; a component of three or more fails |

ARC-005 is the other half of the same principle: an abstraction needs two
callers to exist, and a third copy needs an abstraction.

### 5.4 Names: `NAME`

| ID | Refuses | Measured by |
| --- | --- | --- |
| NAME-001 | A package name that is not lowercase ASCII kebab-case (`[a-z][a-z0-9]*(-[a-z0-9]+)*`), or that ends in `-rs` or `-rust`. A publishable package, one whose `publish` is not `false`, must also start with `maestro-` | `cargo metadata` |
| NAME-002 | A test module under `tests/` named in fewer than two words, a test function named in fewer than four, a `test_` prefix, or a `_works`, `_ok` or `_test` suffix. Unit tests are held to the same rule | `rust-gate architecture`, plus Clippy `redundant_test_prefix` |

### 5.5 Documentation: `DOC`

| ID | Refuses | Measured by |
| --- | --- | --- |
| DOC-001 | A Rust file that does not open with a `//!` comment saying what the module is for | `rust-gate architecture` |
| DOC-002 | A relative link or an anchor in a tracked Markdown file that does not resolve | lychee `--offline --include-fragments`; external links are checked by a weekly job that opens an issue and never blocks a pull request, because third-party sites go down |
| DOC-003 | Markdown that breaks the generated rumdl configuration: heading structure, list style, fenced code without a language. Line length is not limited, matching `.editorconfig` | rumdl |

The existing rules stay: rustdoc with `-D warnings -D missing_docs`, doc tests
run by `cargo test --doc`.

### 5.6 Hygiene: `HYG`

| ID | Refuses | Measured by |
| --- | --- | --- |
| HYG-001 | A `TODO`, `FIXME`, `HACK` or `XXX` comment that does not reference an issue, `#123` or a full issue URL. Comments only: Rust, shell, TOML, YAML and justfile comments are read, Markdown prose is not, so a document may name the words | `rust-gate hygiene` |
| HYG-002 | A pending snapshot committed: `*.snap.new`, `*.pending-snap` | `rust-gate hygiene` |
| HYG-003 | A tracked file over 500 KB. **Exception**, for a declared asset | `rust-gate hygiene` in CI; the prek built-in large-file hook at commit |
| HYG-004 | An executable without a shebang, a shebang script that is not executable, two paths that differ only by case, a broken symlink or one that leaves the repository | `rust-gate hygiene`; prek built-ins at commit where prek provides them |
| HYG-005 | A missing `README.md` or `LICENSE`; a missing `CHANGELOG.md` in a repository with a release-please configuration | `rust-gate hygiene` |

### 5.7 Workspace and toolchain: `WSP`

| ID | Refuses | Measured by |
| --- | --- | --- |
| WSP-001 | In a workspace of two members or more, a member that does not inherit `[lints]`, `edition`, `rust-version` and `license` from the workspace, or that declares a dependency other than `{ workspace = true }` | Manifests |
| WSP-002 | An edition other than 2024, a resolver other than 3, an untracked `Cargo.lock` | Manifests and the index |

The compiler pin itself is a generated file (section 7), so it cannot drift.

### 5.8 Libraries: `LIB`

| ID | Refuses | Measured by |
| --- | --- | --- |
| LIB-001 | `print!`, `println!`, `eprint!`, `eprintln!` in any file reachable from a library root: a library reports through return values or `tracing`, and the binary decides what to print | `rust-gate architecture`. Cargo's lint table applies to every target of a package, so Clippy's `print_stdout` cannot be limited to the library target and is not used |
| LIB-002 | A publishable package with a library target that depends on `anyhow`, `eyre` or `color-eyre` outside `[dev-dependencies]`: a library exposes typed errors | `cargo metadata` |

### 5.9 Tests: `TST`

| ID | Refuses | Measured by |
| --- | --- | --- |
| TST-001 | `std::thread::sleep`, `tokio::time::sleep` or `async_std::task::sleep` in test code: a test waits on a fake clock or a synchronisation primitive, never on time. **Exception**, for a test that genuinely exercises wall-clock behaviour | `rust-gate architecture`, test code only |
| TST-002 | `#[ignore]` without a reason, `#[should_panic]` without `expected` | Clippy `ignore_without_reason`, `should_panic_without_expect` |
| TST-003 | More than one integration-test crate root per package without `required-features`: one `main.rs` in the package's tests directory and its modules, one binary to link | Manifests and the tests directory |
| TST-004 | A test that is retried: the generated nextest profile sets `retries = 0`, so a flaky test fails the run | Generated `.config/nextest.toml`, rendered from the same source as the profile the gate writes for its own run |

Mutation testing stays as it is: every mutant of the changed lines must be
killed, and a timed-out mutant counts as surviving.

### 5.10 Lints: `LNT`

LNT-001 is one list, owned by `rust-gate`, applied at `deny`. It is written into
every repository's `[workspace.lints]`, or `[lints]` for a single package,
between generated markers, so rust-analyzer shows the same errors the gate
raises. A repository may add lints below the markers and may not remove one.
Every name was checked against Clippy 0.1.98 on Rust 1.98.1, and a contract
test keeps checking them: an unknown lint name fails the gate's own build.

| Family | Lints |
| --- | --- |
| Clippy groups | `all`, `pedantic`; `cargo` for publishable packages, without `multiple_crate_versions`, which DEP-001 owns |
| No silent opt-out | `allow_attributes`, `allow_attributes_without_reason`: every opt-out is `#[expect(lint, reason = "...")]`, which fails once it stops being needed |
| No panic outside tests | `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `unreachable`, `exit`, `todo`, `unimplemented`, `dbg_macro` |
| Structure | `self_named_module_files` (one layout: `directory/mod.rs`), `absolute_paths`, `min_ident_chars` |
| Tests | `redundant_test_prefix`, `ignore_without_reason`, `should_panic_without_expect` |
| Unsafe, where `unsafe-policy: allow` | `undocumented_unsafe_blocks` |
| rustc | `unsafe_code` (per `unsafe-policy`), `missing_docs`, `unreachable_pub`, `missing_debug_implementations`, `unused_qualifications`, `rust_2018_idioms`, `let_underscore_drop`, `redundant_lifetimes`, `unit_bindings`, `unexpected_cfgs` |

Deliberately left out, with the reason recorded next to the list:
`module_name_repetitions` and `similar_names` (pedantic, too many false
positives), `redundant_pub_crate` (contradicts `unreachable_pub`),
`arithmetic_side_effects`, `exhaustive_enums`, `exhaustive_structs`,
`missing_docs_in_private_items`, `missing_assert_message`,
`tests_outside_test_module`, `print_stdout` (see LIB-001) and the `shadow_*`
lints.

The generated `clippy.toml`:

```toml
cognitive-complexity-threshold = 15
too-many-lines-threshold = 100
too-many-arguments-threshold = 5
excessive-nesting-threshold = 4
absolute-paths-max-segments = 2
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-print-in-tests = true
allow-dbg-in-tests = false
```

Every key was checked against Clippy 0.1.98, which rejects an unknown key.

### 5.11 Coverage, dependencies, supply chain and performance

| ID | Refuses | Measured by |
| --- | --- | --- |
| COV-001 | Line coverage under 90 % across the workspace. `coverage-threshold` defaults to 90 and a lower value is refused | cargo-llvm-cov |
| COV-002 | Changed-line coverage under 95 % on a `feat` or `fix` pull request, under 90 % on any other type. At most `max(1, floor((100 - target) % of n))` of the `n` coverable changed lines may stay uncovered, so a three-line change is not failed by one line. Not applicable to a push | LCOV intersected with `git diff -U0` against the merge base, over Rust files of measured packages; the type is read from the pull request title |
| DEP-001 | Two versions of one crate, a wildcard version, a git or registry source not on the allowlist, a yanked or unmaintained crate. **Exception**, for a duplicate version the ecosystem forces | The generated `deny.toml`, with exceptions rendered from `maestro-quality.toml`; cargo-audit for advisories |
| DEP-002 | A declared dependency the code does not use | cargo-machete, unchanged |
| VET-001 | A dependency that is neither audited nor exempted. `dependency-audit` becomes on by default; `supply-chain/config.toml` must import the organization's audits, published by rust-workflows, and those of Mozilla, Google, the Bytecode Alliance, ISRG and the Zcash Foundation | cargo-vet `--locked`; the import list is checked, the exemptions stay the repository's own reviewed state |
| PRF-001 | A benchmark whose instruction count rises more than 5 % against the base branch, when `[performance] benches` names benchmarks. **Exception**, for an accepted regression | gungraun (formerly iai-callgrind) under Valgrind, base and head in the same job. Instruction counts do not depend on runner load |

### 5.12 Pull request level: `PRL`

| ID | Refuses | Measured by |
| --- | --- | --- |
| PRL-001 | A `feat` or `fix` pull request that changes product Rust code, a Rust file outside the `tests`, `benches` and `examples` directories, and touches no test: no file under `tests/`, no line inside a `#[cfg(test)]` module or a `#[test]` function. This makes ENF-005, failing test first, deterministic | The diff against the merge base |
| PRL-002 | Nothing: a pull request over 400 changed lines, lockfiles, snapshots and generated files excluded, is reported in the summary to support FND-003, surgical changes | The diff |

### 5.13 Universal set: `UNI`

Every repository, whatever its stack, runs these at commit time and in CI.

| Stage | Checks |
| --- | --- |
| Commit | Merge conflict markers, YAML, TOML and JSON syntax, final newline, trailing whitespace (Markdown keeps its two-space breaks), mixed line endings, large files, case conflicts, shebangs, typos, gitleaks, yamlfmt, taplo, actionlint, zizmor, shellcheck, shfmt, rumdl, lychee offline, editorconfig-checker |
| Commit message | Conventional header, lowercase after the type, subject within 71 characters; every line within 80 columns |
| Commit, Rust repositories | rustfmt, `rust-gate architecture`, `rust-gate hygiene` |
| Push, Rust repositories | Clippy with LNT-001 |
| CI | All of the above, then the full Rust gate |

## 6. Where the rules run

One command, three places, and the same bytes in each.

1. **CI.** `ci.yml` runs a new `architecture` step first, because it only reads
   text and finishes in seconds, then `hygiene`, then the existing steps.
   Neither step has an input that turns it off. A repository without Rust calls
   a new reusable workflow, `hygiene.yml`, which runs the universal set and the
   `hygiene` and `DOC` rules.
2. **Commit and push.** rust-workflows publishes a `.pre-commit-hooks.yaml`.
   Every hook installs its tool through prek's managed languages at the version
   rust-workflows' `mise.lock` pins, so a developer needs prek and rustup and
   nothing else. `rust-gate` itself is built by prek with
   `cargo install --locked` at the pinned tag.
3. **The full gate locally.** A manual-stage hook, `prek run --hook-stage manual
   gate`, runs `rust-gate check`: every CI step in CI's order, with the tools
   `rust-gate install-tools` downloads and verifies against the same digests CI
   uses. Linux x64 only, the platform the gate's tool rows cover.

rust-workflows runs the same command on itself in `just check` and in
`ci-internal.yml`. Its internal tests for import cycles, the generic parts of
its layer boundaries, its size limits and its test names are deleted, and
their rules become the fixtures of the command. The rust-workflows-specific tests about its own
workflows and step registry stay. Its layers, `steps`, `checks` and `runner`,
are declared in its own `maestro-quality.toml`.

## 7. Generated files

`rust-gate sync` writes these files; `rust-gate init` writes them for a new
repository; the `managed-files` step of `ci.yml` and `hygiene.yml` refuses any
difference and prints the fix, `rust-gate sync`. Each generated file opens with
the line `generated by rust-gate sync; do not edit`, in that format's comment
syntax.

| Stack | Files |
| --- | --- |
| Every repository | `.github/workflows/ci.yml` (the caller: rust-workflows' `ci.yml` for Rust, `hygiene.yml` otherwise), `.github/dependabot.yml` (ignores rust-workflows, which sync owns), `.pre-commit-config.yaml`, `.editorconfig`, `.gitattributes`, `typos.toml`, `.yamlfmt.yml`, `.taplo.toml`, `.rumdl.toml` |
| Rust | `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `.config/nextest.toml`, and the marked lint block of the root `Cargo.toml` |

A file the repository must shape itself is never generated. What a repository
may say lives in one optional file:

```toml
# maestro-quality.toml

[limits]              # tighten only; a looser value is refused
file-lines = 400

[[crate]]             # ARC-004: the target whose root is this file
root = "gate/src/main.rs"
layers = ["steps", "checks", "runner"]   # a layer may name several modules

[typos]               # words this repository means, merged into typos.toml
words = ["jaq", "zizmor"]

[performance]         # PRF-001
benches = ["tokenizer"]

[[exception]]         # ARC-005, DUP-001, HYG-003, TST-001, DEP-001, PRF-001
rule = "ARC-005"
path = "gate/src/runner/mod.rs"
item = "enter"
reason = "the step registry is the only thing that can run a step"
```

`rust-gate` reads TOML through the pinned jaq, as it reads JSON today, so the
binary stays standard-library only (rust-gate.md, invariant 1).

## 8. Keeping every repository on the standard

1. **Which repositories.** The organization property `stack` becomes required,
   with the values `rust` and `other`, editable by organization owners only.
   Every repository has one value.
2. **Merge lock.** The existing ruleset `rust-ci-required` already requires
   `rust / Required Rust CI` on every `stack=rust` default branch. A second
   ruleset, `hygiene-required`, requires `hygiene / Required hygiene` on every
   `stack=other` default branch. Neither can be removed from inside a
   repository.
3. **Sync.** A workflow in `.github`, `quality-sync.yml`, runs on every
   rust-workflows release, weekly and on demand. For each repository it runs
   `rust-gate sync` at the new tag and, when anything changed, opens or updates
   one pull request, `maestro/sync`, with a commit signed through the
   organization bot, the mechanism `tool-updates.yml` already uses. Minor and
   patch sync pull requests merge themselves once green; a major one waits for
   a human. Because repositories pin a released commit, nothing on a default
   branch turns red by itself: the sync pull request carries the upgrade and
   cannot merge until the repository passes it.
4. **Audit.** `org-drift.yml` fails, and opens or updates one issue per
   repository, when a repository has no `stack`, when its sync pull request is
   more than 14 days old, or when `rust-gate sync --check` at the latest tag
   reports a difference on its default branch.

The bot token already holds Contents, Pull requests and Workflows write on every
repository, which sync needs to write `.github/workflows/ci.yml`. The future
private `ctm-collection` repository needs no additional grant for sync; the
audit App needs Contents read on it.

## 9. Changes to `rust-gate`

- New steps: `architecture`, `hygiene`, `managed-files`, `changed-coverage`
  (COV-002), `pull-request` (PRL-001, PRL-002), `performance` (PRF-001).
  `duplication` starts enforcing DUP-001 and keeps its report of pairs.
  `complexity` stays a report: Clippy enforces SIZE-001 and `architecture`
  enforces SIZE-002 and SIZE-003.
- New commands outside the workflows: `sync`, `sync --check`, `init`, `check`,
  `lints --write`.
- A Rust tokeniser and module-tree reader in the checks layer, shared by every
  `ARC`, `SIZE`, `DOC`, `LIB`, `NAME` and `TST` rule, with fixtures for raw
  strings, nested comments, `r#` identifiers, `cfg_attr` and macros.
- New pinned tools, each with its `# tool:` line, its install row and its digest:
  lychee, rumdl, shfmt, editorconfig-checker, gungraun's runner. Valgrind comes
  from the runner image's package archive at a pinned version, installed only
  when PRF-001 applies.
- The scorecard gains one control per rule family, `enforced` whenever it ran.

## 10. Proof

1. **Test-first per rule.** Each identifier gets a fixture that violates it and
   one that does not, and a contract test asserting the refusal names the
   identifier, the file and the line.
2. **Gates and documents.** `docs/gates.toml` gains one entry per family with
   its proofs; the existing test refuses a proof that does not exist and
   `just docs` renders the tables.
3. **Lint names and keys.** A test runs Clippy with every LNT-001 lint and the
   generated `clippy.toml` against a fixture and fails on any unknown lint or
   key.
4. **Parity.** Before a rust-workflows internal test is deleted, the command
   must refuse the same deliberately broken copy of the repository that the
   test refuses.
5. **False positives.** Outside the rules marked **Exception**, a false positive
   is fixed in the reader, never excepted.
6. **Speed.** `architecture` and `hygiene` together finish in under 10 seconds
   on maestro-model-router, the largest repository, measured by a test.
7. **Generated files.** A test runs the generated `.pre-commit-config.yaml` in a
   fresh clone with only prek and rustup on the PATH, and `sync --check` on a
   freshly `init`ed repository reports nothing.

## 11. Rollout

| Order | Repository | Change | Estimate |
| --- | --- | --- | --- |
| 1 | rust-workflows | The rule families, `sync`, `init`, the hooks manifest and `hygiene.yml`, in self-contained pull requests over several sessions; rust-workflows brought into compliance (two `mod.rs` files, three publishable example crates renamed, and what the new lints find, such as absolute `std::` paths in `gate/src/main.rs`); gate repin; release v2.0.0 | 8 to 9 days |
| 2 | `.github` | `stack` required with `other`, `hygiene-required` ruleset, `quality-sync.yml`, audit extension, template on v2.0.0. Organization settings change only after explicit confirmation | 1 day |
| 3 | maestro-model-router | Sync pull request with its fixes: the `catalog.rs` integration test split, three test `mod.rs` files, lints | 1 to 2 days |
| 4 | maestro-core | Born compliant in its S0 fresh start: `maestro-canonicalization`, five files split | 1 day |
| 5 | release-canary | Sync pull request | 1 hour |

Total: 11 to 13 days. Every release tag and every organization setting change
waits for explicit confirmation.

## 12. Risks

| Risk | Mitigation |
| --- | --- |
| The standard-library reader misreads unusual source | ARC-007 removes path attributes; the tokeniser handles every literal and comment form; fixtures cover the known hard cases; a false positive is a reader bug |
| Immediate blocking stalls repositories | The pin means a default branch never turns red; the sync pull request waits until its repository is fixed |
| Sync pull requests on every release are noise | One pull request per repository per release, updated in place; minor and patch merge themselves |
| cargo-vet's first adoption is large | Exemptions are generated once per repository at adoption and only shrink by review afterwards; the organization audit ledger grows as audits are done |
| Valgrind or gungraun changes | Only repositories that declare benchmarks run PRF-001; both versions are pinned |
| The plan cannot require workflows centrally | The generated caller plus the property-targeted ruleset give the same guarantee: the check must exist and pass |

## 13. Verified during planning

These are facts to confirm while writing the plan, not decisions; each has a
pass condition.

1. For each universal tool, the prek language that installs it at the pinned
   version: pass when the fresh-clone test in section 10 succeeds.
2. Which of HYG-003 and HYG-004's commit-time checks prek ships as built-ins:
   pass when each is either a built-in hook or a `rust-gate hygiene` rule.
3. gungraun's current crate names and version: pass when the PRF-001 fixture
   fails on a deliberate 10 % regression.
4. The Dependabot `ignore` entry that excludes rust-workflows from
   `github-actions` updates: pass when a Dependabot run on release-canary opens
   no pull request for it.
