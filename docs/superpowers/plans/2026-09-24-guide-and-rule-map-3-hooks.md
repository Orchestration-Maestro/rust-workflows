# Rule map in rust-gate, phase 3: the commit hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every repository but the home of the workflows runs `rust-gate
rules` and `rust-gate guide` as commit hooks, CI skips both, and `rust-gate
init` writes the rule map and, in a git repository, the guide.

**Architecture:** `hooks.rs` renders two more hooks through the pinned
`rust-gate` install the gate's hooks already use; `commit_hooks.rs` adds both
to the hooks CI skips; `init` in `managed_files/step.rs` runs the two commands
as separate processes, the way `local_runs.rs` runs `hygiene`, since a step
never reaches another step's code.

**Tech Stack:** Rust 1.98.1, standard library only; prek; the contract-test
harness of `tests/`.

**Spec:** `docs/superpowers/specs/2026-09-24-guide-and-rule-map-design.md`

## Global Constraints

Every constraint of the phase 1 and phase 2 plans holds.

## Deviations from the spec, decided before execution

- `just docs` in this repository does not run either command: the home of
  the workflows keeps its own guide and standards pages until phase 5, and
  the rendered `.pre-commit-config.yaml` never reaches it (`!repository.home`
  in `render.rs`). No consumer repository has a managed justfile, so the
  commit hook is the only place that keeps a consumer's files current.
- `init` writes the guide only in a git repository: the guide describes
  tracked files, and outside git there are none. There, the first commit's
  hook writes it.

---

### Task 1: The two hooks, skipped by CI

**Files:** `gate/src/steps/managed_files/hooks.rs`,
`gate/src/steps/commit_hooks.rs`, `tests/ci/commit_hooks.rs`.

- [ ] **Step 1: Write the failing assertions.** In `hooks.rs`'s unit test,
  replace `assert_eq!(rust.matches("language: rust\n").count(), 2);` with:

```rust
        assert_eq!(rust.matches("language: rust\n").count(), 4);
        assert_eq!(other.matches("language: rust\n").count(), 3);
        for (id, entry) in [
            ("rust-gate-rules", "rust-gate rules"),
            ("rust-gate-guide", "rust-gate guide"),
        ] {
            let hook = format!("      - id: {id}\n        name: ");
            assert!(rust.contains(&hook) && other.contains(&hook), "{id}");
            assert!(other.contains(&format!("        entry: {entry}\n        always_run: true\n")));
        }
```

  moving `let other = commit_hooks("# h\n", false, "2.0.0");` above it. In
  `tests/ci/commit_hooks.rs`, expect
  `SKIP=rustfmt,clippy,rust-gate-architecture,rust-gate-hygiene,rust-gate-rules,rust-gate-guide`.

- [ ] **Step 2:** Run
  `cargo test --manifest-path gate/Cargo.toml managed_files` and
  `cargo test --manifest-path tests/Cargo.toml commit_hooks`: both FAIL.
- [ ] **Step 3: Render the hooks.** In `commit_hooks`, after the
  `rust-gate-hygiene` hook, for every repository:

```rust
    let _ = write!(
        text,
        "      - id: rust-gate-rules\n        name: Rule map from the golden rules\n        \
         entry: rust-gate rules\n        always_run: true\n{gate}      - id: \
         rust-gate-guide\n        name: Copilot guide from the tracked files\n        entry: \
         rust-gate guide\n        always_run: true\n{gate}"
    );
```

  Extend the module doc: the hooks also keep the rule map and the Copilot
  guide current. In `commit_hooks.rs`, set `SKIPPED` to the string above and
  say in its doc that CI never fails on a stale rule map or guide; the daily
  drift check reports them.

- [ ] **Step 4:** Run both tests: PASS.

### Task 2: `init` writes the rule map and the guide

**Files:** `gate/src/steps/managed_files/step.rs`, `tests/ci/managed_files.rs`,
`tests/repository/rendered_hooks_live.rs`.

- [ ] **Step 1: Write the failing contract test** in
  `tests/ci/managed_files.rs`:

```rust
#[test]
fn init_writes_the_rule_map_and_in_a_git_repository_the_guide() {
    let outside = Fixture::with_sources(&[("src/lib.rs", "//! A crate.\n")]);
    let written = in_project(&outside, &format!("{} rust-gate init", pin('a', "2.0.0")));
    succeeds(&written);
    assert!(managed(&outside, "docs/standards/engineering.md").starts_with("# Engineering rules in `"));
    assert!(!outside.root.join("project/.github/copilot-instructions.md").exists());
    assert!(String::from_utf8_lossy(&written.stdout).contains("the first commit's hook writes it"));
    let inside = Fixture::with_sources(&[("src/lib.rs", "//! A crate.\n")]);
    succeeds(&in_project(
        &inside,
        &format!(
            "git init -q && git remote add origin https://github.com/Orchestration-Maestro/example.git \
             && git add -A && {} rust-gate init",
            pin('a', "2.0.0")
        ),
    ));
    assert!(managed(&inside, "docs/standards/security.md").starts_with("# Security rules in `example`"));
    assert!(managed(&inside, ".github/copilot-instructions.md").contains("── lib.rs"));
}
```

- [ ] **Step 2:** Run `cargo test --manifest-path tests/Cargo.toml managed_files`:
  the new test FAILS, the pages missing.
- [ ] **Step 3: Implement.** `init` declares `tools: &["jaq", "rust-gate"]`,
  ends with `write_all(&root, Some(&pin))?; adapted(&root)`, and gains:

```rust
/// Write the repository's rule map and, in a git repository, its Copilot
/// guide, which the commit hooks keep current from then on; the guide lists
/// tracked files, so outside git the first commit's hook writes it.
fn adapted(root: &Path) -> Outcome {
    Cmd::new("rust-gate rules").cwd(root).run()?;
    if root.join(".git").exists() {
        return Cmd::new("rust-gate guide").cwd(root).run();
    }
    println!("guide: not a git repository yet; the first commit's hook writes it");
    Ok(())
}
```

  Update the module doc's `init` sentence to say it also writes the rule map
  and the guide.

- [ ] **Step 4:** In `rendered_hooks_live.rs`, add `printf '# Agents\\n' >
  AGENTS.md &&` before `rust-gate init`, since the guide links to it, and
  extend `SKIP` with `rust-gate-rules,rust-gate-guide`: like the other gate
  hooks, they install a release that does not exist yet.
- [ ] **Step 5:** Run the managed-files tests, then
  `CHECK_NETWORK=1 cargo test --manifest-path tests/Cargo.toml rendered_hooks_live`:
  PASS.

### Task 3: Documentation, check and commit

- [ ] **Step 1:** In `docs/ci.md`, after the `rust-gate guide` paragraph, add
  that both run as commit hooks in every repository but this one, rewrite a
  stale page or guide so the next commit carries it, are skipped by CI, and
  are written first by `rust-gate init`.
- [ ] **Step 2:** List this plan in `.github/copilot-instructions.md`; run
  `just docs`.
- [ ] **Step 3:** Run `timeout 1800 just check`; commit only when its own
  exit status is 0: `feat: keep the rule map and copilot guide current at
  every commit`.

## Deviations during execution

- The live hooks test, run with `CHECK_NETWORK=1` as CI runs it, failed on
  `typos`: every rule map cites `FND-001` to `FND-004`, and typos reads `FND`
  as a misspelt "FIND". Each repository had added `FND` to its own
  `maestro-quality.toml` by hand. The managed `typos.toml` now allows `FND`
  in every repository, before the repository's own words and once, so a
  repository that also lists it keeps a valid file:
  `every_repository_means_the_foundations_prefix_once`.
