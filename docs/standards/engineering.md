# Engineering standards

The rules this repository holds itself to and, for each one, what actually
enforces it here.

These rules come from the Maestro engineering baseline, which is written from the
authoring side: it says what *a downstream adopter* must do. This repository is
that adopter. Restating the requirement would leave a reader no better informed,
so every rule below carries the command, test or workflow that enforces it, or an
explicit statement that it rests on human judgement and is not tooled.

That distinction is the point of this document. A rule nobody checks is an
intention, and calling it enforced is the kind of claim the four measures in
[northstar.md](northstar.md) exist to prevent.

## How to read this

**MUST** and **MUST NOT** are mandatory. **SHOULD** is the expected default; a
deviation needs a recorded, reviewed reason. Missing evidence means noncompliant,
not presumed compliant.

A deterministic control produces a repeatable machine result. A judgement control
needs a person, and review is not proof. Both appear below, always labelled.

Nothing in this document grants a permission, a token or an execution right. Those
come from enforced platform configuration; prose cannot widen them.

Teams may add stricter requirements. None may weaken what is here.

## Where each rule is enforced

The single table a reviewer needs. `just check` is this repository's local gate;
the named tests live under `tests/`, one directory per what they prove,
`tests/ci/`, `tests/publishers/`, `tests/nightly/`, `tests/gate/` and
`tests/repository/`, each directory's `mod.rs` listing its modules and nothing
else, with the harness they share under `tests/harness/`: `tests/harness/mod.rs`
is its one door, in front of `tests/harness/repository.rs`,
`tests/harness/workflow_yaml.rs`, `tests/harness/gate_declarations.rs` and
`tests/harness/fixture.rs`, each naming what it takes from a sibling and none
naming a test module. A module names what it proves in two words at least and
a test function in four, never behind a `test_` prefix or a `_works`, `_ok` or
`_test` suffix: NAME-002, held by `rust-gate architecture` in `just check`
(`badly_named_tests_and_one_word_test_modules_are_refused`).

| ID | Rule | Enforced here by |
| --- | --- | --- |
| FND-001 | Think before coding | Judgement. Pull request **Change** section. |
| FND-002 | Simplicity first | Judgement. Pull request review. |
| FND-003 | Surgical changes | Judgement. Diff-to-goal trace at review. |
| FND-004 | Goal-driven execution | `just check` is the acceptance check; the pull request **Verification** checklist records its result. |
| P-001 | YAGNI | Judgement. |
| P-002 | KISS | Judgement. |
| P-003 | DRY | Judgement. |
| P-004 | WET | Judgement. |
| P-005 | Rule of three | Judgement. |
| P-006 | Chesterton's fence | Judgement. |
| P-007 | Boy Scout rule | Judgement. |
| P-008 | Least astonishment | Partly deterministic: `RUSTDOCFLAGS='-D warnings -D missing_docs' cargo doc` forces every public item to be described. |
| P-009 | Single responsibility | Judgement, with one deterministic case: `provenance_attestation_is_isolated_and_reverifies_the_payload`. |
| P-010 | Composition over inheritance | Partly deterministic: the gate composes three layers, runner, checks and steps, declared in `maestro-quality.toml` and held by `rust-gate architecture` in `just check`: ARC-004 keeps their imports flowing one way (`declared_layers_refuse_an_import_within_or_against_the_order`) and ARC-001 refuses an import cycle in any crate, the tests included (`an_import_cycle_between_two_files_is_refused_by_name`). |
| P-011 | Fail fast | **Deterministic.** Every workflow validates its inputs before any side effect. `ci_rejects_unsafe_paths_and_symlinks`, `every_live_publisher_requires_reviewers_and_only_release_tag_deployments`. |
| P-012 | Illegal states unrepresentable | Judgement in Rust; in workflows, approximated by rejecting the state at the boundary rather than representing it. |
| P-013 | Parse don't validate | **Deterministic.** A `validate` step writes to `GITHUB_ENV` only values it has already checked; later steps read the checked value, never the raw input. |
| P-014 | Least privilege | **Deterministic.** `permissions_timeouts_and_shell_policy_hold_in_every_workflow`. Elevated scopes live in separate callable workflows so no caller inherits them. |
| P-015 | Separation of concerns | **Deterministic.** `attest-binaries.yml`, `unsafe-audit.yml` and `publish-evidence.yml` are isolated, and a test asserts the isolation. |
| P-016 | Zero one or many | Partly deterministic: the compatibility matrix covers five versions and the workspace fixture covers multiple members. |
| P-017 | Premature optimisation | Judgement, measured when it matters: `just setup` went from about four minutes to five seconds only after the cost was measured. |
| P-018 | Broken windows | **Deterministic.** `-D warnings` on Clippy and rustdoc; `permissions_timeouts_and_shell_policy_hold_in_every_workflow` rejects `\|\| true` anywhere in a workflow. |
| ENF-001 | No machine-named paths | **Deterministic.** Paths derive from `$GITHUB_WORKSPACE` and `$RUNNER_TEMP`; `ci_rejects_unsafe_paths_and_symlinks` proves the rejection. |
| ENF-002 | Every claimed platform | **Not met for native Windows.** Hosted workflows remain Linux x64. The distinct native Windows suite remains available but its execution is unverified; see [platform-requirements.md](../platform-requirements.md). |
| ENF-003 | English only | Judgement. All prose and identifiers are English. |
| ENF-004 | Conventional commits | Pull request template documents the prefixes and their version effect; `release-please` consumes them. A test requires the template to keep explaining it. |
| ENF-005 | Failing test first | Judgement. The pull request checklist requires a regression test **and its failure case**. |
| ENF-006 | Never weaken a gate | **Deterministic.** `\|\| true` is rejected by test, and `north_star_promises_are_enforced_by_the_local_gate` fails when a control named in [northstar.md](northstar.md) is absent from the justfile; `every_test_the_standards_cite_exists` refuses a standard that names a test which no longer exists. |
| ENF-007 | Tiered checks | **Deterministic.** Fast prek hooks at commit; `just check` locally; `ci-internal.yml` runs the same `just check` in CI, not a separately maintained equivalent. |
| ENF-008 | Non-rotting allowlists | **Deterministic.** The five tested pins are one list with one test: `ci_accepts_any_exact_stable_from_the_msrv_and_keeps_matrix_artifacts_distinct`. |
| ENF-009 | Instruction authority boundary | Satisfied by construction: these documents grant nothing. Permissions come from each workflow's `permissions:` block. |

Nine rules are deterministically enforced. One is explicitly not met. The rest are
judgement, and saying so is more useful than implying a gate exists.

Rules that only an administrator can apply, such as branch and tag rulesets and
required context bindings, are deliberately absent. They are real requirements, but this
repository has no action available for them, and listing obligations it cannot act
on would dilute the ones it can.

## The four foundations

### FND-001: Think before coding

An implementer MUST state assumptions before implementing, and MUST say so and
push back when a simpler approach exists. Where more than one reading exists, the
implementer MUST surface the competing interpretations. If something is unclear,
stop and name what is confusing.

Applies to every change; for an unambiguous one-line change, use judgement and
record the basis briefly.

### FND-002: Simplicity first

An implementer MUST choose the least complex solution that satisfies the
requirement, and MUST NOT add speculative abstractions, configuration,
dependencies or scaffolding. No features beyond what was asked. No abstraction for
single-use code. No error handling for impossible scenarios.

If 200 lines could be 50, rewrite it.

### FND-003: Surgical changes

A change MUST touch only the files and behaviour its goal requires, and MUST NOT
include drive-by refactors or unrelated formatting. Match the existing style even
where you would choose differently. Remove the imports and helpers *your* change
orphaned; leave pre-existing dead code alone and mention it instead.

Every changed line should trace to the stated goal.

### FND-004: Goal-driven execution

A change MUST define an observable completion result and MUST run that
previously-defined check, reporting its actual result.

Turn the task into something checkable: "add validation" becomes "write tests for
invalid inputs, then make them pass". Weak criteria need constant clarification;
strong ones let the work proceed unattended.

## The eighteen named principles

A shared name makes a review one word long.

| ID | Principle | Meaning here |
| --- | --- | --- |
| P-001 | YAGNI | Build for the requirement in front of you, not a possible future. |
| P-002 | KISS | Prefer the boring construct the next reader can understand. |
| P-003 | DRY | Share what is genuinely one idea. Text that only looks similar is not one idea. |
| P-004 | WET | Write everything twice before guessing an abstraction. |
| P-005 | Rule of three | Consider extraction on the third occurrence, not the second. |
| P-006 | Chesterton's fence | Understand why something exists before removing it. |
| P-007 | Boy Scout rule | Improve within the diff you already have reason to touch. |
| P-008 | Least astonishment | Make the reader's first guess correct. |
| P-009 | Single responsibility | Give each unit one reason to change. |
| P-010 | Composition over inheritance | Assemble behaviour rather than inheriting it. |
| P-011 | Fail fast | Refuse bad input at the boundary, with an actionable failure. |
| P-012 | Make illegal states unrepresentable | Encode constraints so invalid states cannot be constructed. |
| P-013 | Parse don't validate | Turn unstructured input into a value that carries the checked guarantee. |
| P-014 | Principle of least privilege | Give each token, workflow and actor only the access it needs. |
| P-015 | Separation of concerns | Keep distinct jobs and boundaries distinct. |
| P-016 | Zero one or many | If it can happen twice, design for an arbitrary count. |
| P-017 | Premature optimisation | Measure before optimising. |
| P-018 | Broken windows | Fix small neglect before it becomes permission for more. |

Four of these carry weight here beyond review vocabulary, because the workflow
shape depends on them:

**P-011 and P-013 together describe every workflow's first step.** Each one
validates its inputs (path shape, toolchain pin, threshold range, boolean) and
writes only checked values into `GITHUB_ENV`. Nothing downstream re-reads the raw
input. A malformed `working-directory` fails before a checkout happens, not
halfway through a publish.

**P-014 and P-015 explain why there are nine workflow files rather than three.**
GitHub validates a job's requested scopes at startup even when an `if:` will skip
that job, so a single file declaring `attestations: write` would impose that grant
on every caller. Isolating elevated scopes keeps the common path least-privileged.

## Hard mandates

### ENF-001: No machine-named paths

Code, configuration, task runners and workflows MUST NOT write absolute paths that
name a machine: no home directory, drive letter or user profile. Paths MUST be
derived at runtime. Platform roots (`/usr`, `/opt`, `/etc`, `/var`, `/tmp`) are
allowed; tests MUST use synthetic roots such as `/somewhere`.

### ENF-002: Every claimed platform

Each claimed platform MUST be covered on every pull request by equivalent fast,
merge-blocking checks.

**Hosted gates remain Linux x64.** The separate native Windows Cargo suite
requires its own execution evidence, distinct from Linux acceptance. The gate
claims no Windows or macOS run of its own; the `platforms` input runs a
consumer's `cargo test` on pinned macOS, Windows and Linux arm64 runners.

### ENF-003: English only

Prose and identifiers MUST be English. A diacritic scan can be automated, but it
is not proof of English; semantic compliance needs review.

### ENF-004: Conventional commits

Commits MUST use exactly one accepted type: `feat:`, `fix:`, `docs:`, `refactor:`,
`test:`, `ci:`, `build:`, `chore:`, `perf:` or `revert:`. Changelog generation
consumes these types.

`release-please` derives the next version from the merged commit message. On a
squash merge that message is the pull request title, which is why the pull request
template states the prefixes and their version effect, and why a test keeps that
section present.

### ENF-005: Failing test first

For a behaviour change, the test MUST be written first and MUST be observed
failing before the implementation. A test that passes immediately proves nothing
about the requested behaviour.

### ENF-006: Never weaken a gate

A gate MUST NOT be weakened, bypassed or removed to make a change pass. If a gate
blocks something correct, report it in the pull request.

`|| true` is the usual way this rule gets broken quietly, so a test rejects it
anywhere in a workflow. That check exists because a silenced SBOM step stays
green without producing an SBOM.

### ENF-007: Tiered checks

Cheap commit checks SHOULD be distinct from pre-push checks. The local aggregate
check MUST run the same commands as its CI quality gate, not an independently
maintained equivalent. Fast required CI MUST block merges; heavy checks SHOULD
report on a schedule.

`ci-internal.yml` runs `just check`, the same entry point a contributor runs
locally. There is no second copy of the gate to drift.

### ENF-008: Non-rotting allowlists

An allowlist MUST record a reason and MUST have a check that fails when an entry
is no longer true.

The tested-version list is exactly five pins. A test compares them against the
consumer matrix and the declared default, so a sixth or a drifted entry fails
immediately. Acceptance is not an allowlist: any exact stable version from the
MSRV up is installed.

### ENF-009: Instruction authority boundary

Instruction prose and links MUST NOT grant tools, permissions, execution authority
or security exemptions.

## Rust rules

- Hold the module structure rules ARC-001 to ARC-007 of the organization
  quality gate, `docs/superpowers/specs/2026-09-24-org-quality-gate-design.md`;
  `just check` runs `rust-gate architecture` on every project here.
- Forbid `unsafe` in every library and consumer fixture; `#![forbid(unsafe_code)]`
  at crate level. Consumers may opt into the same rule through the CI
  `unsafe-policy` input.
- Document every public API with `///`, and include an example for anything whose
  use is not obvious. Enforced by `-D missing_docs`.
- Prefer `Result` over `panic!`.
- Use `#[inline]` only on a measured hot path (P-017).

## Adoption status

The baseline defines what an adopter owes. Stated plainly, here is where this
repository stands. Four of eight are unmet, and none of them can be met by writing
more documentation.

| ID | Requirement | Status here |
| --- | --- | --- |
| C-001 | Resolve all applicable mandatory content and controls | Met. This document is that resolution. |
| C-002 | Retain compliance evidence | Met for what runs: `just check` output, CI receipts and the per-run reports artifact. |
| C-003 | Controlled, authorized, scoped, expiring exceptions | No exception has been taken. |
| C-004 | Coordinate rollouts that change content, controls or contexts | Met by the pull request template's contract section. |

The baseline was adopted as content, not as a tracked dependency: it has no
published release to pin, so the requirements about consuming, versioning and
drift-checking one describe a relationship that does not exist. They are left out
rather than recorded as permanent failures.

This document is now the authority for these repositories. A change to it is a
change like any other: reviewed in a pull request, against the gate.

## Numbering

Identifiers are contiguous and local to this repository. Rules it has no action on
were removed rather than left as permanent failures, and the numbering was closed
up afterwards, so these identifiers no longer line up with the upstream baseline
or with the sibling repository. Cite them with the repository name attached.

## Non-goals

This document does not implement schemas, runtime loading, permissions, CI or
release automation, and it does not claim every rule above is machine-enforced.
The enforcement column is the claim; nothing wider is intended.
