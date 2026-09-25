# Rule map in rust-gate, phase 2: `rust-gate guide` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rust-gate guide` writes a repository's Copilot guide,
`.github/copilot-instructions.md`, the way `copilot-instructions.py` writes it,
and `rust-gate guide --check` refuses a stale guide.

**Architecture:** One step module, `gate/src/steps/copilot_guide`, in the
gate's `steps` layer, reaching only the runner. `text.rs` holds the prose
helpers, `describe.rs` explains a file, `tree.rs` draws the annotated tree,
`render.rs` writes the guide and `step.rs` declares and runs the two local
steps. JSON is read through the pinned jaq; the file list comes from
`git ls-files`.

**Tech Stack:** Rust 1.98.1, edition 2024, standard library only; `git` and
`jaq` as declared tools; the contract-test harness of `tests/`.

**Spec:** `docs/superpowers/specs/2026-09-24-guide-and-rule-map-design.md`

**Reference:** `.github`'s `copilot-instructions.py` at commit
`ac41142`, frozen while this phase ran.

## Global Constraints

Every constraint of the phase 1 plan,
`docs/superpowers/plans/2026-09-24-guide-and-rule-map-1-rules.md`, holds.
Two more surfaced here:

- Clippy's `indexing_slicing` refuses string slicing as well: product code
  reads strings with `get`, `split_once`, `split_at_checked` and
  `strip_prefix`.
- The repository's link check reads every Markdown file, code blocks
  included: code that builds a Markdown link or a test that holds one writes
  the closing bracket and the opening parenthesis apart, with `concat!` or
  pushes.

---

### Task 1: The contract tests, seen failing

**Files:** `tests/ci/copilot_guide.rs`, and `mod copilot_guide;` in
`tests/ci/mod.rs` after `mod complexity_report;`.

- [x] **Step 1:** Write three tests over a fixture repository holding a README
  with a title, a purpose, a table and an image, a Rust module, a workflow, a
  ruleset and a note: the guide explains each file and a second run changes
  nothing; a hand-written explanation survives and the superseded default of
  the guide's own line gives way; `guide --check` refuses a guide a new file
  left stale, with its fix.
- [x] **Step 2:** Run
  `cargo test --manifest-path tests/Cargo.toml copilot_guide`. Seen failing:
  `unknown gate command: guide` and `unknown gate command: guide --check`.

### Task 2: The module

**Files:** `gate/src/steps/copilot_guide/{mod,text,describe,tree,render,step}.rs`;
`mod copilot_guide;` in `gate/src/steps/mod.rs`; `super::copilot_guide::STEPS`
in `gate/src/steps/registry.rs` after `super::rule_map::STEPS`.

**Interfaces:**

- `text.rs`: `sentence(&str) -> String`, `wrap(&str, indent: &str) -> String`
  (Python's `textwrap.fill` at 80 columns: tabs expanded, whitespace runs kept
  inside a line), `blocks`, `is_prose`, `paragraph`, `first_comment`,
  `value_of(text, key)`, `module_doc`, `title_of`, `MANAGED`.
- `describe.rs`: `describe_file(root: &Path, relative: &str) -> String`.
- `tree.rs`: `kept(guide: &str) -> BTreeMap<String, String>`,
  `readme_rows(root, paths) -> BTreeMap<String, String>`,
  `tree_lines(root, paths, previous, tables) -> Vec<String>`.
- `render.rs`: `GUIDE`, `render(root: &Path, paths: &[String], remote: &str)
  -> String`, paths sorted.
- `step.rs`: `STEPS` with the local steps `guide` and `guide --check`.
- [x] **Step 1:** Port each function of the reference, in the module the
  architecture names, with a unit test per behaviour: 17 unit tests.
- [x] **Step 2:** Word three places differently, by design: the tree's intro
  and the third step name `rust-gate guide` and the commit hook, and the
  guide's own line reads "This guide, written by rust-gate guide at every
  commit". `SUPERSEDED` in `render.rs` holds the reference's old wording of
  that line, so a guide that still holds it gets the new one.
- [x] **Step 3:** Run the unit and contract tests: all pass.

### Task 3: The repository's own records

- [x] **Step 1:** Add the module's six files and the contract test to
  `.github/copilot-instructions.md`, from column 49.
- [x] **Step 2:** Run `just docs`, which adds both steps to `docs/steps.md`.
- [x] **Step 3:** Add a paragraph on `rust-gate guide` to `docs/ci.md`, after
  the one on `rust-gate rules`.
- [x] **Step 4:** Run `timeout 1800 just check`, gate the commit on its own
  exit status, and commit.

### Task 4: Parity with the reference

- [x] **Step 1:** Clone `.github`, maestro-core, maestro-model-router and
  release-canary twice each; run the reference in one clone and
  `rust-gate guide` in the other.
- [x] **Step 2:** Compare the two guides line by line, leaving out the three
  places Task 2 words differently. Result: the four repositories identical.
- [x] **Step 3:** Repeat with each guide deleted first, so every explanation
  is computed rather than kept. Result: the four repositories identical,
  459 explanations among them.

## Deviations during execution

- The code was written, compiled and tested before this plan, after the
  contract tests were seen failing; the plan records the tasks as they ran
  and points to the code rather than copying its 1,400 lines, which would
  have been a second copy to keep in step.
- Two places read a regular expression's quirk differently, where the Rust
  reading is the intended one and no organization repository reaches the
  difference: a module doc whose first `//!` line is empty takes its next
  line's text, not that line with its marks; and a README table cell holding
  only spaces explains nothing instead of an empty string.
