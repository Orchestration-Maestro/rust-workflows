<p align="center">
  <strong>Automate the guardrails to deliver faster, with higher quality, and more securely.</strong>
</p>
<p align="center">
  Speed, quality and security are not a trade-off triangle. Automation is what lets one repository have all three.
</p>

# 🧭 North Star for Rust Workflows

This file is how `rust-workflows` steers by that motto: four axes, one bar
each, and for every bar the gate that holds it and the test that fails when
it slips. These are acceptance targets, not claims of a measured green run.

## 🎯 The point

One Rust pipeline for every Orchestration-Maestro team instead of one per application: the same
gates, reproducible artifacts and a publication path that cannot skip CI.
Application code stays in consumer repositories; deployment stays in the
platform workflows.

## 📐 Four axes

| Axis | Bar | Held by | Proof |
| --- | --- | --- | --- |
| Quality | All 15 consumer and compiler cases pass; at least 90% line coverage on every owned fixture; zero surviving mutants (mutation testing) in the owned fixtures; zero formatting, Clippy or rustdoc warnings, doctests included, Clippy's pedantic group among them. | `just check` replays the steps of `ci.yml` against every fixture, coverage floor at 90 and every offline gate on; the hosted matrix runs the 15 cases. | `example_gate_replays_ci_step_bodies_against_every_fixture`, `north_star_promises_are_enforced_by_the_local_gate` |
| Speed | `just check` at or under 40 seconds with tools and dependencies cached. | `just check` prints its own duration against the target on every run. | `the_speed_target_is_the_one_the_gate_prints` |
| Security | Zero detected secrets, known vulnerabilities, disallowed licences or unexpected package sources; every action pinned to a commit and every download to a checksum; no silenced lint in the gate or the fixtures. | Gitleaks, the mandatory hosted RustSec lookup, the cargo-deny licence and source policy, the CycloneDX SBOM, and the pin and download tests. | `scanners_propagate_findings_execution_errors_and_missing_tools`, `permissions_timeouts_and_shell_policy_hold_in_every_workflow`, `tool_installation_verifies_every_download_and_fetches_nothing_unasked`, `no_lint_is_silenced_in_the_gate_or_the_fixtures` |
| Maintainability | Every file explained in the Copilot inventory, and no explanation naming a code artefact its file does not hold; every item of the gate documented, private ones included; zero stale links; every test a standard or a gate cites exists; every gate names the test that proves its failure case; no function above 15 in cognitive complexity, 100 lines or 5 parameters; no name a layer door offers used by fewer than two modules without its reason written down, and no name the harness door offers that no test outside it uses; no Rust file above 500 lines of code, with the files over 300 reported; no line of Rust or shell above 100 columns. | The inventory, link, documentation and citation tests; Clippy's private-item documentation lint on the gate crate; strict rustdoc; the thresholds in each crate's `clippy.toml` and the limits tests. | `metadata_forms_and_copilot_inventory_are_complete`, `every_code_token_a_tree_comment_names_lives_in_its_file`, `every_relative_link_in_the_repository_resolves`, `every_input_output_and_secret_is_documented`, `every_test_the_standards_cite_exists`, `every_gate_names_the_test_that_proves_it`, `a_seam_serving_one_outside_caller_is_refused_unless_excused`, `no_test_module_imports_the_whole_harness`, `every_crate_holds_the_complexity_limits`, `no_source_file_exceeds_five_hundred_lines`, `files_over_three_hundred_lines_are_reported`, `no_code_line_exceeds_one_hundred_columns` |

## 📊 KPIs

One number per axis: what it is today, where it must be, and what produces it.
A value a gate verifies on every run is written plain; it cannot drift without
turning something red. A value read by hand carries the date it was read,
because nothing keeps it current afterwards. Unmeasured is written as not
measured, never estimated.

| Axis | KPI | Current | Target | Measured by |
| --- | --- | --- | --- | --- |
| Quality | Line coverage floor on the owned fixtures | 90%, verified on every run | 90% | The example gate, `COVERAGE=90` |
| Speed | `just check` wall time, tools cached | 54 s, read 2026-09-23 | 40 s or less | The SPEED line of `just check` |
| Security | Silenced lints in the gate and the fixtures | 0, verified on every run | 0 | `no_lint_is_silenced_in_the_gate_or_the_fixtures` |
| Maintainability | Undocumented items in the gate crate | 0, verified on every run | 0 | Clippy `clippy::missing_docs_in_private_items`, denied |

Speed is an improvement target, not a merge gate: `just check` prints its
duration against the target and never fails a run for time.
Measure from this repository root with tool installation, the live advisory
fetch and the hosted matrix excluded. A missed target requires a recorded
cause and a focused improvement, not fewer checks. Hosted cold start and queue
time are a different measurement.

## 🛡️ How we hold ourselves to it

- Shift left, automated. A rule that matters is a gate in the prek hooks and
  in `just check`, the same gate CI runs. A rule that cannot be automated is
  written down as a check someone runs, and the standards say which is which.
- Guardrails over gatekeepers. A test, a lint or a pinned tool beats a review
  comment that will be forgotten.
- A gate that cannot fail is not a gate. Every gate in the README names the
  test that proves its failure case, and a test that no longer exists turns
  the gate red.
- A decision is recorded where it is enforced. The test that holds a rule
  carries the reason in its own comment, and the standards name that test.
- A change names the axis it moves. The pull request template asks which axis
  a change moves and what it costs the others; a trade-off nobody stated is
  not merged quietly.

## ⚖️ Defaults a consumer inherits

The owned-fixture coverage floor is 90%; the public `ci.yml` default stays 80
and callers keep the documented input. Five gates are on by default as
deliberate exceptions to adoption safety, because a golden workflow enforces
the standard: mutation testing, the unused-dependency check, the `unsafe` ban,
SARIF reports and public API compatibility, each with one input to switch it off. The
scaffolding lints have no off switch. Dependency policy defaults to
`license-policy: auto`: a consumer `deny.toml` when present, otherwise the
source/version policy and the organization allowlist when provided. It says
when no licence list applied; `off` skips the whole dependency-policy step.
Every other default may not change to something that fails a consumer on upgrade. The README lists every gate against these
axes. Coverage measures Rust fixture lines, not how completely the gate's own
refusals are tested; preserve executable negative cases.

## 🧾 Evidence before green claims

Local checks establish local behavior only. `CHECK_NETWORK=1 just check` adds a
fresh advisory lookup; it still does not establish GitHub runner, authentication or
live-publication readiness. This candidate has no published hosted-run baseline yet.

1. Select completed successful **Repository quality** and **Consumer workflow tests**
   runs for the intended commit. Preserve the run URL and attempt, not a screenshot alone.
2. Download their `repository-evidence-*` and `consumer-evidence-*` artifacts plus
   the relevant `*-reports` artifacts. Receipts identify revision, run, scope and outcomes.
3. Download logs through GitHub's **Download log archive** operation. GitHub applies
   its masking there; raw process output and environment dumps are not copied into receipts.
4. Attach the receipts, reports and checked log archive to the approved internal
   review record, with both run links. Check for confidential information before sharing.

Artifacts request seven-day retention, subject to organization limits. Archive
needed evidence before expiry. A consumer receipt is generated only after all
required matrix and dry-run outcomes succeed; failed, cancelled, skipped or missing
outcomes cannot produce a success receipt. The completed GitHub run remains the
source of truth, including an upload failure or rerun. Per-job reports alone are
not proof that the whole workflow passed. Live publishing needs separate evidence.

## ✂️ Keep the bar small

Use fast prek hooks for commit feedback and full checks in CI. Put engineering and
rustdoc rules in [engineering.md](engineering.md), terminology in
[CONTEXT.md](../../CONTEXT.md), and the short scorecard in [README.md](../../README.md).
Update the README when a target changes. Add a KPI only when it changes a
review or delivery decision. A hand-read KPI carries its date in the table
above; every other piece of evidence stays out of committed source files.
