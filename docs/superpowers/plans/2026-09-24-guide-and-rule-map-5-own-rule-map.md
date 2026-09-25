# Rule map in rust-gate, phase 5: this repository's own rule map

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `docs/standards/engineering.md`, `security.md` and `northstar.md` become
this repository's rule map, written by `rust-gate rules` from the golden rules
it carries, so an ID here means what it means in every other repository.

**Architecture:** The three pages are rendered by the gate this repository
builds; what each row says is this repository's own evidence, renumbered onto
the golden rule IDs. What the rule map has no room for moves, unchanged, to a
page written by hand, `docs/standards/controls.md`. `just docs` writes the rule
map and `just check` refuses it stale or unmapped.

**Spec:** `docs/superpowers/specs/2026-09-24-guide-and-rule-map-design.md`

## Why

The pages numbered their mandates locally after dropping rules, so ENF-007
meant "Tiered checks" here and "Pull requests only" in the golden rules, and
ENF-008 and ENF-009 were shifted the same way; ENF-007, ENF-010, ENF-012 to
ENF-014, SEC-010, SEC-011 and the adoption rules had no row at all.

## Where every section went

| Before | After |
| --- | --- |
| Each rule's "Enforced here by" and "Status" cells | The rule map's "Held here by" cells: ENF-007 to ENF-008, ENF-008 to ENF-009, ENF-009 to ENF-011, SCH-005 also as ENF-012 |
| The Rust rules, the tests' layout (NAME-002), the workflow-shape notes on P-011, P-013, P-014 and P-015 | `engineering.md`, "Stricter here", which `rust-gate rules` keeps |
| What an attacker would want here | `security.md`, "What this repository protects", which it keeps |
| The point, and the four KPIs | `northstar.md`, which it keeps |
| The four axes with their bars, gates and proofs, the defaults a consumer inherits, evidence before green claims, how we hold ourselves to it, keep the bar small, the notes behind ENF-002, ENF-004, ENF-006, ENF-008 and ENF-009, SDL, SST, SCH, VR and framework alignment | `controls.md`, word for word but for the renumbered mandates |
| The rule statements copied from the golden rules, the adoption status and the numbering note | Removed: the golden rules state them, and the IDs now match |

## Tasks

- [x] **1. Seed and render.** A script read each row's cell from the pages on
  `main`, renumbered the mandates, added the missing rows with what really holds
  them, and `rust-gate rules` rendered the three pages: 0 rows not mapped yet.
- [x] **2. `controls.md`.** Every section in the table above, moved as written.
- [x] **3. Tests.** `north_star_promises_are_enforced_by_the_local_gate` reads the
  four axes from `controls.md`; `the_speed_target_is_the_one_the_gate_prints`
  reads the Speed bar there and the Speed KPI in `northstar.md`;
  `every_test_the_standards_cite_exists` and the required-files list include
  `controls.md`.
- [x] **4. Gate.** `just docs` runs `rust-gate rules`; `just check` runs
  `rust-gate rules --check`.
- [x] **5. Pointers.** README, AGENTS.md, CONTEXT.md, SECURITY.md,
  `docs/README.md`, `docs/ci.md` and the Copilot guide name `controls.md` where
  they named a moved section.

## Deviation from the spec

The spec had this repository leave the drift check's exemption. It stays
exempt there: the drift check renders with the latest release's copy of the
golden rules, and this repository renders with the copy on its default branch,
which runs ahead of the release after every carry. Its own `just check`, a
required CI status, refuses its rule map stale or unmapped instead. `.github`'s
carry pull request therefore rewrites this rule map with the gate it builds
from the carried copy, so a carry that only changes wording merges itself, and
one that adds a rule waits until a person maps it.
