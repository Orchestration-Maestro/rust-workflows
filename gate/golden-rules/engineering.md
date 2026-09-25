# Engineering rules

The engineering rules every repository in the organization follows: four
foundations, eighteen named principles, fourteen hard mandates and four rules for
adopting them. Security has its own [security rules](security.md); the
[Northstar](northstar.md) says what all of them steer toward.

## These rules come first

- **Nothing overrides them.** A repository, a team, a request or an agent
  instruction may add a stricter rule; none may weaken one of these.
- **MUST** and **MUST NOT** are mandatory. **SHOULD** is the expected default;
  departing from it needs an [exception](#exceptions).
- **Missing evidence is not compliance.** A rule whose control or evidence is
  missing is reported unsupported and stops the affected work; it is never
  presumed to pass.
- **Each repository adapts them to its reality.** It records, for every rule,
  what enforces it there or why it does not apply ([C-001](#c-001--map-every-rule)).
  The rule itself does not change.
- **Judgement is not proof.** A review is a judgement; a gate is a proof. The
  evidence says which one it is.

## Four foundations

### FND-001 — Think before coding

- **Requirement:** State assumptions before implementing, and push back when a
  simpler approach exists. Where more than one reading exists, surface the
  competing interpretations. When something is unclear, stop and name what is
  confusing.
- **Applies to:** every change; for an unambiguous one-line change, use
  judgement and record the basis briefly.
- **Evidence:** the change record or review shows the assumptions, the
  alternatives or pushback, any competing interpretations, and the uncertainty
  named or resolved.

### FND-002 — Simplicity first

- **Requirement:** Choose the least complex solution that satisfies the
  requirement. Add no speculative abstraction, configuration, dependency or
  scaffolding.
- **Applies to:** every implementation and design decision.
- **Evidence:** review names the requirement served and why the chosen
  solution is enough; nothing speculative is present.

### FND-003 — Surgical changes

- **Requirement:** A change touches only the files and behaviour its goal
  requires: no drive-by refactoring, no unrelated formatting.
- **Applies to:** every change, documentation and configuration included.
- **Evidence:** diff review traces every changed line to the stated goal and
  records any necessary adjacent fix.

### FND-004 — Goal-driven execution

- **Requirement:** A change defines an observable result that means done, runs
  that previously defined check, and reports its actual result.
- **Applies to:** every change; a one-line change needs no gratuitous process.
- **Evidence:** the change record holds the completion criteria and the
  check's output, or states plainly that the check was unavailable.

## Eighteen named principles

The principles behind the four foundations. A shared name makes a review one
word long. Principles marked **non-negotiable** accept no
[exception](#exceptions).

| ID | Principle | What it means |
| --- | --- | --- |
| P-001 | YAGNI | Build for the requirement in front of you, not a possible future. |
| P-002 | KISS | Prefer the boring construct the next reader can understand. |
| P-003 | DRY | Share what is genuinely one idea, not merely similar text. |
| P-004 | WET | Write everything twice before guessing an abstraction. |
| P-005 | Rule of three | Consider extraction on the third occurrence. |
| P-006 | Chesterton's fence | Understand why something exists before removing it. |
| P-007 | Boy Scout rule | Improve within the diff you already have reason to touch. |
| P-008 | Least astonishment | Make the reader's first guess correct. |
| P-009 | Single responsibility | Give each unit one reason to change. |
| P-010 | Composition over inheritance | Assemble behaviour rather than inheriting it. |
| P-011 | Fail fast | Refuse bad input at the boundary. **Non-negotiable.** |
| P-012 | Make illegal states unrepresentable | Encode constraints so invalid states cannot be constructed. |
| P-013 | Parse, don't validate | Turn unstructured input into a value that carries the checked guarantee. **Non-negotiable.** |
| P-014 | Principle of least privilege | Give each token, workflow and actor only the access it needs. **Non-negotiable.** |
| P-015 | Separation of concerns | Keep distinct jobs and boundaries distinct. |
| P-016 | Zero, one or many | If it can happen twice, design for any count. |
| P-017 | Premature optimisation | Measure before optimising. |
| P-018 | Broken windows | Fix small neglect before it becomes permission for more. **Non-negotiable.** |

## Fourteen hard mandates

Every mandate except ENF-008 and ENF-009 is **non-negotiable**.

### ENF-001 — No machine-named paths

- **Requirement:** Code, configuration, task runners and workflows never write
  an absolute path that names a machine: no home directory, drive letter or
  user profile. Paths are derived at runtime; platform roots (`/usr`, `/opt`,
  `/etc`, `/var`, `/tmp`) are allowed, and tests use synthetic roots such as
  `/somewhere`.
- **Applies to:** every repository surface and test fixture that handles paths.
- **Evidence:** a path check and review show derived paths and rejected
  machine-named forms.

### ENF-002 — Every claimed platform is tested

- **Requirement:** Each platform a repository claims to support is covered on
  every pull request by merge-blocking checks. A platform without that coverage
  is not claimed.
- **Applies to:** every repository that claims platform support.
- **Evidence:** CI and ruleset configuration show the coverage for each claimed
  platform.

### ENF-003 — English only

- **Requirement:** Prose and identifiers are English. A diacritic scan may help,
  but it does not prove English; review does.
- **Applies to:** source, configuration, documentation, identifiers and content
  written for agents.
- **Evidence:** scan output plus review.

### ENF-004 — Conventional commits

- **Requirement:** Commit titles follow Conventional Commits. The organization
  refuses any other form, and changelogs are generated from the commit types.
- **Applies to:** every commit and every release workflow.
- **Evidence:** the enforcing ruleset, the changelog configuration, and a
  refused non-conforming title.

### ENF-005 — Failing test first

- **Requirement:** For a behaviour change, the test is written first and seen
  failing before the implementation.
- **Applies to:** every behaviour change; not documentation-only changes.
- **Evidence:** history or recorded output showing the failure, then the pass.

### ENF-006 — Never weaken a gate

- **Requirement:** A gate is never weakened, bypassed or removed to make a
  change pass. A gate that blocks something correct is reported in the pull
  request; correcting the gate is its own reviewed change, never a local bypass.
- **Applies to:** every quality, security and governance gate.
- **Evidence:** protected configuration, gate history and the pull request
  report; no silent local change.

### ENF-007 — Pull requests only

- **Requirement:** Every change to the default branch arrives through a pull
  request, maintainers included, as a signed commit. The platform refuses a
  direct push, a force-push and the deletion of the default branch.
- **Applies to:** every repository.
- **Evidence:** the branch rulesets, and a refused direct push, force-push and
  unsigned commit.

### ENF-008 — Tiered checks

- **Requirement:** Cheap commit checks SHOULD be distinct from pre-push checks.
  The local aggregate check MUST run the same commands as the CI quality gate,
  not a separately maintained equivalent. Fast required CI MUST block merges;
  heavy checks SHOULD report weekly.
- **Applies to:** every repository with local and CI quality gates.
- **Evidence:** hook, CI and ruleset configuration showing the tiers.

### ENF-009 — Allowlists that cannot rot

- **Requirement:** Every allowlist entry, suppressed finding and excused lint
  records its reason and its scope, and a check fails when the entry is no
  longer true. A suppression never hides a tool error or a missing report.
- **Applies to:** accepted duplication, retired vocabulary, and every security
  or quality finding a repository suppresses.
- **Evidence:** the entries, their reasons, and the check's output.

### ENF-010 — Configuration is the authority

- **Requirement:** Enforced configuration is authoritative over settings applied
  by hand. Renaming a required check updates every rule that requires it in the
  same change.
- **Applies to:** security configuration, CI and rulesets.
- **Evidence:** protected configuration and the coordinated rename.

### ENF-011 — Instructions grant nothing

- **Requirement:** Instruction prose and links never grant tools, permissions,
  execution authority or security exemptions. Authority comes only from
  enforced configuration and access control.
- **Applies to:** every instruction a person or an agent consumes.
- **Evidence:** permission configuration that holds whatever the instructions
  say.

### ENF-012 — Pinned inputs

- **Requirement:** Every dependency resolves from a committed lockfile, every
  external CI action or reusable workflow is pinned to a full commit SHA, and
  every downloaded tool is checked against a pinned checksum. A mutable tag or
  an unverified download is refused.
- **Applies to:** build, test, release and CI inputs, the tools and models
  agents load included.
- **Evidence:** the lockfiles, the pinning policy, and a refused unpinned
  reference.

### ENF-013 — No secret in history

- **Requirement:** A secret never enters version control, history, examples and
  fixtures included. Every push is scanned and the platform blocks one that
  carries a secret. A leaked secret is revoked and rotated, not merely deleted.
- **Applies to:** every repository.
- **Evidence:** the push protection and scanning configuration, and the
  rotation record of any leak.

### ENF-014 — Multi-factor authentication

- **Requirement:** Every member and outside collaborator signs in with a second
  factor; the organization refuses access to any account without one.
- **Applies to:** every account with access to the organization.
- **Evidence:** the organization's enforced two-factor requirement.

## Adopting the rules

### C-001 — Map every rule

- **Requirement:** Each repository records, for every rule here and in the
  [security rules](security.md), what enforces it there, or why it does not
  apply, before affected work begins.
- **Applies to:** every repository.
- **Evidence:** the repository's rule map, each entry naming a gate, a test, a
  review step or a reason.

### C-004 — Detect drift

- **Requirement:** An automated check detects when a repository no longer holds
  the rules, and escalates what it finds.
- **Applies to:** every repository.
- **Evidence:** the drift check's result and the escalation it opened.

### C-005 — Keep the evidence

- **Requirement:** Evidence of compliance is retained, and so is the record of
  every control that was unavailable.
- **Applies to:** every repository.
- **Evidence:** an auditable record: runs, reports and stated gaps.

### C-006 — Controlled exceptions

- **Requirement:** An exception exists only as an authorized, scoped and
  expiring decision, as set out under [exceptions](#exceptions).
  **Non-negotiable.**
- **Applies to:** every departure from a rule.
- **Evidence:** the exception record.

## Exceptions

- An exception records the authorized decision, its scope, the rationale, an
  expiry date and where it is reported.
- The organization approves it; no one approves their own, and an agent never
  does.
- A rule marked **non-negotiable** accepts no exception.
- A lasting change is a pull request to these rules, never a local weakening.

## Aligned with

These rules follow the standards below, in the versions reviewed on
2026-09-24; a new version of any of them triggers a review of these rules. This
is alignment, not certification: no assessment, score or level is claimed. The
[security rules](security.md#aligned-with) map the rest, and list what no rule
covers yet.

| Standard | Version | Control → rule |
| --- | --- | --- |
| [OWASP Top 10](https://owasp.org/Top10/2025/) | 2025 | A01 → P-014; A02 → ENF-010; A03 → ENF-012; A05 → P-013; A07 → ENF-014; A08 → ENF-007; A10 → P-011 |
| [OWASP Top 10 for Agentic Applications](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) | 2026 | ASI01 → ENF-011; ASI03 → P-014; ASI04 → ENF-012 |
| [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/) | 2025 | LLM01 → ENF-011; LLM03 → ENF-012; LLM06 → P-014 |
| [OpenSSF OSPS Baseline](https://baseline.openssf.org/) | 2026-08-28 | OSPS-AC-01.01 → ENF-014; OSPS-AC-03.01 and AC-03.02 → ENF-007; OSPS-AC-04.01 and AC-04.02 → P-014; OSPS-BR-05.01 → ENF-012; OSPS-BR-07.01 → ENF-013; OSPS-QA-03.01 → ENF-006; OSPS-VM-05.03 and VM-06.02 → ENF-009 |
| [NIST SSDF, SP 800-218](https://csrc.nist.gov/pubs/sp/800/218/final) | 1.1 | PS.1 → ENF-007, ENF-013, ENF-014 |
| [SLSA](https://slsa.dev/spec/v1.2/) | 1.2 | Source Track → ENF-007 |
| [OpenSSF Scorecard](https://scorecard.dev/) | Current checks | Branch-Protection → ENF-007; Pinned-Dependencies → ENF-012; Token-Permissions → P-014 |
| [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) | 1.0.0 | The whole specification → ENF-004 |

ENF-001, ENF-002, ENF-003, ENF-005 and ENF-008 are the organization's own
engineering practice: no external standard owns them.
