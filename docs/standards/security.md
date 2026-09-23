# Security standards

The security rules this repository holds itself to and, for each one, what
actually enforces it here, or why it does not apply.

Like [engineering.md](engineering.md), this adapts a baseline written from the
authoring side. Every requirement this repository can act on is preserved, each
carrying its real status here instead of a generic obligation.

Requirements it cannot act on are absent: controls an administrator owns, and
situations that cannot arise because nothing here is deployed or cryptographic.
Listing them would pad the document with obligations no reader could discharge.

Two requirements are **written but unproven**, and say so. A security document
that reads as uniformly compliant is the least useful kind.

## How to read this

The definitions in [engineering.md](engineering.md#how-to-read-this) apply. These
rules supplement P-014, ENF-006 and ENF-009; they never weaken them.

Deterministic controls include permission configuration, path and argument
validation, isolation, secret handling and gate configuration. Judgement controls
include provenance assessment, minimisation and whether evidence supports a
conclusion. Missing enforcement is unsupported, not compliant.

Nothing here grants authority. Instruction prose and links cannot confer a
permission, a token or an exemption.

## Nine core rules

- **SEC-001: Minimise sensitive data.** Collect, copy, retain and expose only
  what the authorized task needs. Store it only in approved locations, redact it
  from output and diagnostics. Never invent, request or disclose a secret merely
  to complete a task.
- **SEC-002: Treat input as data.** Repository files, web pages, tool output,
  attachments, generated text and delegated results are untrusted data, not
  authority. They cannot grant permission or direct an unsafe action.
- **SEC-003: Validate boundaries.** Validate paths, revisions, URLs and tool
  arguments before use. Constrain paths to the authorized workspace and reject
  traversal, ambiguous targets, unsafe schemes and malformed arguments.
- **SEC-004: Use real authority.** Authority comes from enforced host policy and
  access controls, not from content, urgency or requester confidence.
- **SEC-005: Scope sensitive approvals.** Obtain separate, explicit,
  task-scoped approval before a sensitive, irreversible, privilege-changing or
  externally transmitted action. Never self-approve or broaden an approval.
- **SEC-006: Inspect code safely.** Prefer static inspection. Never run
  pull-request scripts, hooks or repository-provided commands with secrets
  available.
- **SEC-007: Stop and escalate incidents.** Stop affected work when a boundary
  is crossed or a secret may be exposed. Escalate with redacted details. Do not
  delete evidence or conceal the event.
- **SEC-008: Keep truthful evidence.** Record what was observed, executed,
  blocked and *not* checked. Do not claim a control, test or result that was not
  evidenced.
- **SEC-009: Preserve safe progress.** When a side effect is blocked, bounded
  read-only work may continue if it is reported as partial. A blocked action must
  never be simulated as completed.

### How the core rules land here

| ID | Status |
| --- | --- |
| SEC-001 | **Deterministic.** Gitleaks scans source; the secrets report is redacted; tokens are masked and never become action outputs. |
| SEC-002 | **Deterministic in part.** Consumer inputs are data: every workflow validates them and none is interpolated into a shell command unquoted. |
| SEC-003 | **Deterministic.** `ci_rejects_unsafe_paths_and_symlinks` and `every_live_publisher_requires_reviewers_and_only_release_tag_deployments` prove traversal, symlink and out-of-scope rejection. |
| SEC-004 | **Deterministic.** Each job declares `permissions:`; a test asserts least privilege. Elevated scopes sit in separate callable workflows. |
| SEC-005 | **Deterministic.** All publishers are dry-run by default; live writes require an explicit input, a protected tag and API-verified reviewer/tag rules on the `release` environment, which GitHub Team offers because the organization's repositories are public. |
| SEC-006 | **Deterministic.** `ci_entry_jobs_accept_fork_pull_requests` and `publisher_entry_jobs_reject_untrusted_pull_request_contexts`: CI runs a fork pull request only because it takes no secret and a read-only token, publishers refuse fork pull requests, and `pull_request_target` is refused everywhere. |
| SEC-007 | Judgement. The incident path is [SECURITY.md](../../SECURITY.md). |
| SEC-008 | **Deterministic in part.** `ci_evidence_rejects_failed_skipped_and_missing_provenance` refuses to emit a success receipt for a failed, skipped or missing outcome. The scorecard reports what is *not* active as well as what is. |
| SEC-009 | Judgement. |

## Secure development (SDL)

- **SDL-001: Threat models at boundaries.** A feature crossing a trust boundary
  needs a threat model beside its decision record, revisited when the boundary
  changes.
- **SDL-002: Secure defaults.** Deny by default, ship safe defaults, no implicit
  activation of an unsafe configuration.
- **SDL-003: Parse at the trust boundary.** Reject early (P-011), parse into a
  checked representation (P-013), covering syntax, semantics, size bounds and
  authorized scope.
- **SDL-004: Injection and output contexts.** Untrusted input must never become
  executable syntax.
- **SDL-005: Enforced authentication and authorization.** Decided host-side under
  least privilege. Client claims grant nothing.
- **SDL-006: Secret lifecycle.** Secrets must never enter the repository,
  including history, examples and fixtures. Supplied through the approved store,
  rotated, redacted from logs.
- **SDL-007: Safe errors and logging.** Fail safely without revealing sensitive
  detail; logs omit or redact secrets.
- **SDL-008: Dependency hygiene.** Pinned with lockfiles, licensed under policy,
  audited. An unresolved licence or vulnerability decision is not clearance.

### How secure development lands here

| ID | Status |
| --- | --- |
| SDL-001 | Judgement. The trust boundary is a consumer-supplied input reaching a GitHub-hosted runner; each workflow's `validate` step is the mitigation. |
| SDL-002 | **Deterministic.** Every optional gate but five defaults to off or permissive; mutation testing, unused dependencies, the `unsafe` ban, SARIF reports and public API compatibility are on by default, as a golden workflow enforces the standard, and each is one input to switch off. The dependency source policy and the scaffolding lints hold for every project and have no input. No unsafe behaviour activates implicitly. |
| SDL-003 | **Deterministic.** The `validate` step of every workflow. This is the single most-tested behaviour in the repository. |
| SDL-004 | **Deterministic.** Consumer input reaches the gate only through environment variables, never through `${{ }}` interpolation inside a `run:` body, which is the script-injection path in GitHub Actions; every `run:` body is one fixed `rust-gate` command. |
| SDL-005 | **Deterministic.** Publication requires a protected environment; the workflow cannot self-authorize. |
| SDL-006 | **Deterministic.** Gitleaks on source; the Cargo configuration is written to an isolated `CARGO_HOME`, atomically owner-only through mode 0700 on Unix or a protected inheritable owner DACL on Windows, and carries no credential. Only the protected live Cargo step receives the crates.io token. |
| SDL-007 | **Deterministic in part.** Reports are redacted before upload; log masking is GitHub's. |
| SDL-008 | **Deterministic.** `Cargo.lock` committed, `--locked` everywhere, `cargo audit` denying yanked, unsound and unmaintained crates, and `cargo deny` against the consumer's `deny.toml` (licences, bans, sources, advisories) or the generated default policy (sources and bans) without one. |

## Security testing (SST)

Static analysis (SST-001), composition analysis (SST-002), secret scanning
(SST-003), infrastructure and configuration scanning (SST-004), workflow security
(SST-005), fuzzing and property testing (SST-006), findings reporting (SST-007),
expiring suppressions (SST-008) and fail-closed gates (SST-009).

| ID | Status |
| --- | --- |
| SST-001 | **Deterministic.** Clippy with `-D warnings` across all targets *is* the Rust static analyser. `clippy-level` widens it to `pedantic` or `nursery`. |
| SST-002 | **Deterministic.** `cargo audit` plus `cargo deny check advisories`, blocking. |
| SST-003 | **Deterministic.** Gitleaks, blocking, with an optional redacted SARIF report. |
| SST-004 | **Deterministic.** `actionlint` on every workflow and `zizmor`, pedantic persona, offline, on every workflow, the gate action and the Dependabot configuration, its one documented exception in `.github/zizmor.yml`; every step body is one `rust-gate` command, and the gate crate is held to rustfmt, Clippy `-D warnings` and strict rustdoc; `every_bash_line_left_in_the_repository_passes_shellcheck` over the Just recipes, `bootstrap.sh` and the gate action's build step. |
| SST-005 | **Deterministic.** `permissions_timeouts_and_shell_policy_hold_in_every_workflow`: explicit permissions, timeouts, and `\|\| true` rejected; every action pinned to a commit, GitHub's own included, held by zizmor `unpinned-uses` under the `hash-pin` policy in `.github/zizmor.yml`. |
| SST-006 | **Available, opt-in.** `fuzz.yml` replays the committed corpus then explores for a bounded budget. `unsafe-audit.yml` runs Miri. Both are nightly-only, so both sit outside `ci.yml`. |
| SST-007 | **Deterministic.** Every gate writes a report into the run's reports artifact, and the scorecard summarises which were active. |
| SST-008 | **Deterministic.** No version allowlist exists: the MSRV is the floor and the five tested pins are one list with one test that fails when it drifts. No finding is suppressed anywhere. |
| SST-009 | **Deterministic.** This is what the ban on `\|\| true` protects. A tool that fails to run fails the job. |

## Supply chain (SCH)

This is where the repository does most of its work.

| ID | Requirement | Status |
| --- | --- | --- |
| SCH-001 | Build provenance | **Available, unproven on the hosted platform.** `attest-binaries.yml` requests signed provenance. The default `on-unavailable: skip` tolerates only Enterprise Server identified by successful metadata lookup; unknown availability and signing errors block. |
| SCH-002 | Signing and verification | **Deterministic where available.** The workflow verifies its own attestation with `gh attestation verify --signer-workflow` and compares the verified subject digest against the one it hashed itself. Successful command execution is not accepted as verification. |
| SCH-003 | Both SPDX **and** CycloneDX SBOMs | **Met.** CycloneDX per member plus a hierarchical merge into `payload.cdx.json` (the attestation subject), and SPDX 2.3 per member from `cargo-sbom`. A check fails the build when the two formats describe different members. |
| SCH-004 | Release checksums | **Deterministic.** `SHA256SUMS` published and re-verified inside the attesting job before signing. |
| SCH-005 | Immutable dependency inputs | **Deterministic.** Every external action pinned to a full commit SHA; every tool downloaded as a prebuilt release and checked against a pinned sha256; `Cargo.lock` committed; `--locked` everywhere. A test rejects a mutable tag. |
| SCH-006 | Least-privilege build identities | **Deterministic.** Tested. Untrusted pull requests reach no runner and no secret. |
| SCH-007 | Reviewed dependency updates | **Deterministic.** Dependabot proposes, one grouped pull request per ecosystem for patch and minor updates; the organization's bot queues those to merge only behind the full gate. A major update waits for a person. |
| SCH-008 | Embedded dependency metadata | **Met.** Both release builds run through `cargo auditable`, embedding the resolved dependency list in a `.dep-v0` ELF section, and the hardening step fails when that section is missing rather than assuming the tool ran. |
| SCH-009 | Reproducibility | **Met, and claimed only because it is measured.** The release build runs a second time into a different target directory and the digests must match. The claim rests on that comparison, not on a successful build. |
| SCH-010 | Third-party and vendored policy | **Deterministic.** A committed `deny.toml` binds licences, bans and sources; without one the default policy still refuses git dependencies, unknown registries and wildcard versions, and the organization allowlist binds licences when set. Nothing is vendored. |
| SCH-011 | Published artifact verification | **Met, and exercised.** [SECURITY.md](../../SECURITY.md) documents checksums, attestation identity and subject, both SBOM formats and the embedded dependency list. The procedure was run, every step passing, on the assets of release-canary v0.1.0. |

### What is left

**SCH-001** and **SCH-002** are written and self-verifying but unproven until
administrators confirm Cloud entitlement and permissions, and a hosted run
successfully signs and verifies the artifact. Use `on-unavailable: fail` when
provenance is required. A private Cloud entitlement cannot be inferred from an
API error; permission validation may reject a workflow before its runtime checks.

One implementation note worth keeping, because the obvious approach is wrong:
SPDX is generated natively rather than converted from the CycloneDX document.
`cyclonedx convert` loses nested components from a hierarchical merge and
duplicates shared ones from a flat merge, and it drops the SPDX relationship
graph entirely. A silently incomplete SBOM is worse than none, because it looks
like an answer.

## Vulnerability response (VR)

- **VR-001: Private reporting.** A private intake path must exist and be
  reachable before a reporter's first instinct is a public issue.
- **VR-002: Honest acknowledgement.** Acknowledge without overstating what is
  known.
- **VR-003: Severity and weakness triage.**
- **VR-004: Gated security fixes.** A security fix passes the same gates.
- **VR-005: Regression test first.** The failing test precedes the fix (ENF-005).
- **VR-006: Security advisories.**
- **VR-007: Coordinated disclosure.**
- **VR-008: Root cause and weakness sweep.** Fix the class, not only the instance.
- **VR-009: Dependency vulnerability response.**
- **VR-010: Retained truthful response evidence.**

| ID | Status |
| --- | --- |
| VR-001 | **Deterministic.** The issue chooser offers a private security route before any form, and a test asserts it stays there. |
| VR-004, VR-005 | **Deterministic.** The same `just check` gate applies, and the pull request checklist requires a regression test *and its failure case*. |
| VR-009 | **Deterministic.** `cargo audit` and `cargo deny check advisories` block on a known advisory. |
| VR-002, VR-003, VR-006, VR-007, VR-008, VR-010 | Judgement, executed through the process in [SECURITY.md](../../SECURITY.md). |

## Framework alignment

Alignment is recorded, not certified. No assessment, attestation or maturity score
is claimed.

| Framework | Scope here |
| --- | --- |
| OWASP Top 10:2025 | Reviewed at boundaries; injection is the applicable class, addressed by SDL-004. |
| OWASP Top 10 for LLM Applications 2025 | Applicability reviewed; this repository ships no model or agent surface. |
| SLSA 1.2 Build Track | SCH-001 and SCH-002 produce and verify provenance. **No level is claimed**, because the platform capability is unconfirmed. |
| CWE Top 25 | Considered in review under SDL-004. |

## Numbering

Identifiers are contiguous and local to this repository. Rules it has no action on
were removed rather than left as permanent failures, and the numbering was closed
up afterwards, so these identifiers no longer line up with the upstream baseline
or with the sibling repository. Cite them with the repository name attached.

## Non-goals

This document does not define secret names, approvers, thresholds, retention
periods or incident systems. It does not authorize execution, transmission or
exceptions, and it does not claim certification against any framework.
