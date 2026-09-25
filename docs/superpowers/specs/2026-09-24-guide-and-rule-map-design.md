# Copilot guide and rule map in rust-gate: written where they go stale

- Date: 2026-09-24
- Status: approved by the owner on 2026-09-24
- Owner: rust-workflows (`rust-gate`), `.github` (golden rules, drift check)
- Release: a minor release of rust-workflows per phase that changes `rust-gate`

## 1. Problem

Two files in every repository are generated today by Python scripts that live
in the `.github` repository:

| File | Script | Goes stale when |
| --- | --- | --- |
| `.github/copilot-instructions.md`, the Copilot guide | `copilot-instructions.py`, 476 lines | A file is added, moved or removed |
| The rule map, `docs/standards/{engineering,security,northstar}.md` | `golden-rules.py`, 251 lines | A golden rule is added, renamed or removed |

Nothing runs either script where the file goes stale. Someone has to remember
to run it by hand in each repository, the daily drift check then opens an
issue, and fixing it takes a pull request of its own. A guide that changes with
every new file cannot be kept current by pull requests, and a documentation
file must never block a merge.

## 2. Goals

1. The commit that makes a guide or a rule map stale also refreshes it: a
   commit hook rewrites the file, the way a formatter does.
2. A repository gets both at bootstrap, from `rust-gate init`, with nothing
   else to remember.
3. The golden rules reach every repository through the `maestro/sync` pull
   request that already exists, and only when they change.
4. No repository's CI fails because its guide or its rule map is stale. The
   daily drift check reports it instead.
5. One implementation, in `rust-gate`, versioned with its releases; the two
   Python scripts are removed.

## 3. Non-goals

- Generating the organization page's tables from the golden rules.
- Settling `CONSTITUTION.md` against the golden rules.
- Changing what the guide or the rule map says: the port renders what the
  scripts render.

## 4. Design

### 4.1 Two commands

| Command | Writes | `--check` exits 1 when |
| --- | --- | --- |
| `rust-gate guide` | `.github/copilot-instructions.md` | The committed guide differs from what it renders |
| `rust-gate rules` | The three rule-map pages | A page differs from what it renders, or still says "Not mapped yet" |

Each is a step of the `local` workflow, registered like `sync` and `init`, so
`rust-gate describe` lists it and the step registry tests cover it.

Both keep what a person wrote:

- **Guide:** an explanation the current guide already gives for a path is kept
  as written. A new path takes its README table row, else what the file says of
  itself: a module's first doc line, a Markdown title or first sentence, a
  workflow's name, a configuration file's first comment.
- **Rule map:** every row a repository filled in is kept, and so are its "The
  point", "What this repository protects" and "Stricter here" sections and its
  KPI rows. A rule the organization holds everywhere gets its default; any
  other new rule arrives as "Not mapped yet".

Below each page's intro, the rendering is byte-identical to the scripts' on
every organization repository; that parity is the port's acceptance test
(section 6). The intro differs by design: it names `rust-gate rules` rather than
the script, the version of the rules it follows, and links to that version.
The default for C-001 names the command the same way.

### 4.2 The golden rules travel inside the binary

The golden rules are written in the `.github` repository and nowhere else by
hand. rust-workflows keeps a copy of `golden-rules/*.md` beside the gate's
source, and `rust-gate` embeds it at build time with `include_str!`. A release
of `rust-gate` therefore carries one fixed version of the rules:

- the hooks work offline;
- each rule map's opening paragraph names the version it follows, as
  `.github@<commit>`;
- a repository moves to new rules only through its `rust-gate` pin.

The copy is not edited in rust-workflows. When the golden rules change on the
default branch of `.github`, a workflow there sends rust-workflows the event
`golden-rules-changed`. rust-workflows then opens one pull request as the
organization bot that replaces the copy, titled
`fix: follow the golden rules of .github@<commit>`. Merged, it becomes a patch
release, and the existing `quality-sync` carries it to every repository in the
`maestro/sync` pull request, which merges itself once green.

A contract test compares the embedded copy with the `.github` default branch
when `CHECK_NETWORK=1`, like the live hooks test, so a missed event cannot go
unnoticed for long.

### 4.3 Where each command runs

| When | What runs | Can it fail a merge |
| --- | --- | --- |
| Bootstrap | `rust-gate init` writes the managed files, then the guide and the rule map | No |
| Every commit | Hooks `rust-gate-guide` and `rust-gate-rules` rewrite a stale file; the commit stops once and the next one carries the file | No |
| `just docs` | Both commands, beside `rust-gate sync` and the lint tables | No |
| A repository's CI | Neither | No |
| Daily drift check in `.github` | `rust-gate guide --check` and `rust-gate rules --check` | No: it opens or updates the `Drift: <repository>` issue |

The hooks are rendered into every `.pre-commit-config.yaml` by the
`managed_files` hook renderer, next to `rust-gate-hygiene`, through the same
pinned `cli:` install. The CI `hooks` step adds both to the hooks it skips,
beside `rust-gate-hygiene`, so a stale file never fails CI. A repository whose
guide or map a person edits on the web is repaired by its next local commit.

"Not mapped yet" is a task, not a failure: the drift issue lists every such
row until someone maps it.

### 4.4 Porting constraints

`rust-gate` uses the standard library only, so the scripts' regular
expressions become small hand-written parsers, each with unit tests: table
rows, level-two sections, the Northstar's pillars and quotation, a module's first
doc line, a YAML `name:`, a first comment. TOML and JSON are read through the
pinned jaq, as elsewhere in the gate. The file list comes from `git ls-files`.
Every limit of the gate holds: functions of 100 lines or fewer, files of 500
lines of code or fewer, every item documented.

## 5. Phases

Each phase is one pull request; a phase that changes `rust-gate` is followed by
its release.

| Phase | Content |
| --- | --- |
| 1 | `rust-gate rules`: the embedded golden rules, the parsers, the step, and tests including parity with `golden-rules.py` |
| 2 | `rust-gate guide`: the parsers, the step, and tests including parity with `copilot-instructions.py` |
| 3 | The two hooks, skipped by the CI `hooks` step; `init` writing both files; `just docs` running both; release |
| 4 | `.github`: the `golden-rules-changed` event, the drift check calling `rust-gate`, the two scripts removed; rust-workflows: the copy-refresh workflow and the network contract test |
| 5 | rust-workflows maps its own standards onto the golden rule IDs, and leaves the drift check's exemption |

## 6. Testing

- **Unit:** every parser, with the refusal each one gives on malformed input.
- **Contract, per command:** on a fixture repository, the first run writes the
  file; a second run changes nothing; a hand-written cell, section or
  explanation survives a run; a rule added to the embedded copy appears as
  "Not mapped yet"; `--check` exits 1 on a stale file and 0 on a current one.
- **Parity:** in phases 1 and 2, the output for each organization repository
  equals the Python script's output below each page's intro, byte for byte,
  but for the C-001 default. The plan runs the comparison on fresh clones
  before the pull request; the contract tests' fixtures cover each behaviour.
- **Hooks:** the rendered `.pre-commit-config.yaml` holds both hooks, and the
  live hooks test runs them.
- **Network:** with `CHECK_NETWORK=1`, the embedded copy equals the `.github`
  default branch.

## 7. Risks

| Risk | Answer |
| --- | --- |
| A hook that rewrites files on every commit annoys | It writes only when the rendering differs; parity keeps the first run a no-op in current repositories |
| The copy of the rules lags `.github` | The event, the daily `quality-sync` run and the network contract test |
| The port drifts from the scripts during phases 1 and 2 | The scripts stay the reference and are frozen until phase 4 removes them |
| Hand-written parsers miss a case the regular expressions caught | Parity on every organization repository before the scripts go |
