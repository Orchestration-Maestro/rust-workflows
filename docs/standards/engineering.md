# Engineering rules in `rust-workflows`

`rust-workflows` follows the organization's [engineering
rules](https://github.com/Orchestration-Maestro/.github/blob/864d85597a833864cd8506c3925830503b3c2163/golden-rules/engineering.md).
This page is its rule map (C-001): for every rule, what holds it here, or why it
does not apply. A row may name a stricter local rule; none weakens one.

`rust-gate rules` writes the rows from the golden rules of `.github@864d855` at
every commit and keeps what each row says here. A rule added there arrives as
"Not mapped yet", and the daily drift check reports it until it is mapped.

## Stricter here

This repository holds the organization's golden rules and, beyond them,
the controls in [controls.md](controls.md): the bar behind each North Star
pillar, the notes behind the mandates, and the extended security controls.

### Tests

The named tests live under `tests/`, one directory per what they prove,
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

### Rust rules

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

### Workflow shape

**P-011 and P-013 together describe every workflow's first step.** Each one
validates its inputs (path shape, toolchain pin, threshold range, boolean) and
writes only checked values into `GITHUB_ENV`. Nothing downstream re-reads the raw
input. A malformed `working-directory` fails before a checkout happens, not
halfway through a publish.

**P-014 and P-015 explain why there are nine workflow files rather than three.**
GitHub validates a job's requested scopes at startup even when an `if:` will skip
that job, so a single file declaring `attestations: write` would impose that grant
on every caller. Isolating elevated scopes keeps the common path least-privileged.

## Rule map

| Rule | Held here by |
| --- | --- |
| FND-001 Think before coding | Judgement. Pull request **Change** section. |
| FND-002 Simplicity first | Judgement. Pull request review. |
| FND-003 Surgical changes | Judgement. Diff-to-goal trace at review. |
| FND-004 Goal-driven execution | `just check` is the acceptance check; the pull request **Verification** checklist records its result. |
| P-001 YAGNI | Judgement. |
| P-002 KISS | Judgement. |
| P-003 DRY | Judgement. |
| P-004 WET | Judgement. |
| P-005 Rule of three | Judgement. |
| P-006 Chesterton's fence | Judgement. |
| P-007 Boy Scout rule | Judgement. |
| P-008 Least astonishment | Partly deterministic: `RUSTDOCFLAGS='-D warnings -D missing_docs' cargo doc` forces every public item to be described. |
| P-009 Single responsibility | Judgement, with one deterministic case: `provenance_attestation_is_isolated_and_reverifies_the_payload`. |
| P-010 Composition over inheritance | Partly deterministic: the gate composes three layers, runner, checks and steps, declared in `maestro-quality.toml` and held by `rust-gate architecture` in `just check`: ARC-004 keeps their imports flowing one way (`declared_layers_refuse_an_import_within_or_against_the_order`) and ARC-001 refuses an import cycle in any crate, the tests included (`an_import_cycle_between_two_files_is_refused_by_name`). |
| P-011 Fail fast | **Deterministic.** Every workflow validates its inputs before any side effect. `ci_rejects_unsafe_paths_and_symlinks`, `every_live_publisher_requires_reviewers_and_only_release_tag_deployments`. |
| P-012 Make illegal states unrepresentable | Judgement in Rust; in workflows, approximated by rejecting the state at the boundary rather than representing it. |
| P-013 Parse, don't validate | **Deterministic.** A `validate` step writes to `GITHUB_ENV` only values it has already checked; later steps read the checked value, never the raw input. |
| P-014 Principle of least privilege | **Deterministic.** `permissions_timeouts_and_shell_policy_hold_in_every_workflow`. Elevated scopes live in separate callable workflows so no caller inherits them. |
| P-015 Separation of concerns | **Deterministic.** `attest-binaries.yml`, `unsafe-audit.yml` and `publish-evidence.yml` are isolated, and a test asserts the isolation. |
| P-016 Zero, one or many | Partly deterministic: the compatibility matrix covers five versions and the workspace fixture covers multiple members. |
| P-017 Premature optimisation | Judgement, measured when it matters: `just setup` went from about four minutes to five seconds only after the cost was measured. |
| P-018 Broken windows | **Deterministic.** `-D warnings` on Clippy and rustdoc; `permissions_timeouts_and_shell_policy_hold_in_every_workflow` rejects `\|\| true` anywhere in a workflow. |
| ENF-001 No machine-named paths | **Deterministic.** Paths derive from `$GITHUB_WORKSPACE` and `$RUNNER_TEMP`; `ci_rejects_unsafe_paths_and_symlinks` proves the rejection. |
| ENF-002 Every claimed platform is tested | **Not met for native Windows.** Hosted workflows remain Linux x64. The distinct native Windows suite remains available but its execution is unverified; see [platform-requirements.md](../platform-requirements.md). |
| ENF-003 English only | Judgement. All prose and identifiers are English. |
| ENF-004 Conventional commits | Pull request template documents the prefixes and their version effect; `release-please` consumes them. A test requires the template to keep explaining it. The organization's `commits-are-conventional` ruleset refuses any other title. |
| ENF-005 Failing test first | Judgement. The pull request checklist requires a regression test **and its failure case**. |
| ENF-006 Never weaken a gate | **Deterministic.** `\|\| true` is rejected by test, and `north_star_promises_are_enforced_by_the_local_gate` fails when a control named in the four axes of [controls.md](controls.md) is absent from the justfile; `every_test_the_standards_cite_exists` refuses a standard that names a test which no longer exists. |
| ENF-007 Pull requests only | Organization: the `default-branch-discipline` ruleset (pull request, signed commits, code scanning) and `floor-no-destruction`, with no bypass actor; `rust-workflows-ci-required` requires `Required repository quality` and `Required consumer tests`. |
| ENF-008 Tiered checks | **Deterministic.** Fast prek hooks at commit; `just check` locally; `ci-internal.yml` runs the same `just check` in CI, not a separately maintained equivalent. |
| ENF-009 Allowlists that cannot rot | **Deterministic.** The five tested pins are one list with one test: `ci_accepts_any_exact_stable_from_the_msrv_and_keeps_matrix_artifacts_distinct`. |
| ENF-010 Configuration is the authority | Organization: its settings as code in the `.github` repository, checked weekly by `org-drift.yml`; here, the `rust-workflows-ci-required` ruleset binds `Required repository quality` and `Required consumer tests`, renamed only together with the jobs |
| ENF-011 Instructions grant nothing | Satisfied by construction: these documents grant nothing. Permissions come from each workflow's `permissions:` block. |
| ENF-012 Pinned inputs | **Deterministic.** Every external action pinned to a full commit SHA; every tool downloaded as a prebuilt release and checked against a pinned sha256; `Cargo.lock` committed; `--locked` everywhere. A test rejects a mutable tag. |
| ENF-013 No secret in history | Organization: secret scanning with push protection and validity checks (`maestrolabs-baseline`); CI: gitleaks |
| ENF-014 Multi-factor authentication | Organization: two-factor authentication is required of every member and outside collaborator |
| C-001 Map every rule | These pages, written by `rust-gate rules` in `just docs` and checked by `rust-gate rules --check` in `just check`: a rule the carried golden rules add fails `just check` until it is mapped. [controls.md](controls.md) holds what a row has no room for. |
| C-004 Detect drift | Organization: the daily drift check opens a `Drift:` issue for this repository |
| C-005 Keep the evidence | GitHub: pull requests, CI runs with their reports, and drift issues |
| C-006 Controlled exceptions | No exception has been taken. An exception is recorded in the pull request that makes it, with its scope, rationale and expiry. |
