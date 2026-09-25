# Rust Golden Workflows

Reusable GitHub Actions CI and protected publication for Orchestration-Maestro Rust packages and
Cargo workspaces. Teams call these workflows instead of copying build, security
and release logic into every application repository.

This repository holds workflows, one binary that runs every step of them,
consumer fixtures and a contract-test harness. It is not a Rust application.

## Verify

```bash
./scripts/bootstrap.sh       # once, on a new machine: verified mise, the toolbelt, the hooks
just check                   # the whole gate, about 20 s with tools cached
CHECK_NETWORK=1 just check   # adds the live advisory lookup
just docs                    # regenerates docs/steps.md and every generated table
```

`just check` is the acceptance criterion. Report what it printed, not what you
expect it to print; its last lines are the SPEED reading and the verdict.

## Work here

1. Find the row of the table below that matches the change and read what it
   points at before touching anything.
2. Write the failing test first, in the directory named after what it proves:
   `tests/ci/`, `tests/publishers/` and `tests/nightly/` for a workflow step,
   `tests/gate/` for the binary's own rules, `tests/repository/` for files,
   documents and pins. Run the step against a fixture and assert what it
   wrote, traced and refused.
3. Make the smallest change. A step is data before it is code: its `STEPS`
   declaration in `gate/src/steps/` names the workflow, the id, the inputs it
   reads, the tools it runs and the reports it writes, and the runner refuses
   anything undeclared while the step runs.
4. Run `just docs`, then `just check`. When a gate
   blocks something correct, say so in the pull request and leave the gate
   intact; never weaken or remove a gate to pass.
5. List every added or removed file in `.github/copilot-instructions.md`, with
   its explanation aligned with its neighbours; a test compares that tree to
   the disk. Update every document that cites what you changed.
6. Title the commit as the changelog line: a conventional header, its subject
   at most 71 characters, and every line within 80 columns. The commit-msg
   hooks refuse the rest, and release-please writes `CHANGELOG.md` from the
   titles.
7. Bundle a session's work into one pull request, titled for the change a
   consumer sees; documentation, test and CI changes ride along. A `feat` and
   a `fix` need one each, because a squash merge keeps a single title and so a
   single changelog line. A gate change needs its second pull request anyway;
   see CONTRIBUTING.

## Before you change something

| Changing | Read first |
| --- | --- |
| A `ci.yml` input, report or quality gate | [docs/ci.md](docs/ci.md) |
| A step of the binary, or its layout | [docs/rust-gate.md](docs/rust-gate.md), then [docs/steps.md](docs/steps.md) |
| A publisher, or anything that writes to a registry | [docs/publishing.md](docs/publishing.md) |
| Runners, registries, authentication or tool sources | [docs/platform-requirements.md](docs/platform-requirements.md) |
| A quality target, or what counts as evidence | [docs/standards/northstar.md](docs/standards/northstar.md), [controls.md](docs/standards/controls.md) |
| Anything, when a rule's enforcement is unclear | [docs/standards/engineering.md](docs/standards/engineering.md), [security.md](docs/standards/security.md), [controls.md](docs/standards/controls.md) |
| A test, its name or its directory | [CONTRIBUTING.md](CONTRIBUTING.md), the gate and the coding standard |
| A word whose meaning here is narrower than usual | [CONTEXT.md](CONTEXT.md) |
| Any file, added or removed | [.github/copilot-instructions.md](.github/copilot-instructions.md) |

## How a run works

1. A consumer calls a reusable workflow pinned to a reviewed commit. Checkout
   retrieves the consumer's revision, never this repository's.
2. The gate action builds `rust-gate` from this repository's pinned commit,
   in a fresh directory without an executable cache, and puts it on the PATH.
   Every step body is one
   `rust-gate` command.
3. `ci.yml` validates its inputs, runs quality, coverage and security gates, then
   builds and packages a release payload with both SBOM formats and checksums.
   Its final required gate controls the exposed artifact outputs.
4. Both publishers rerun CI for that same revision and default to dry-run.
   `publish-binaries.yml` verifies the artifact and uploads live assets only to
   an existing GitHub Release. `publish-crate.yml` verifies the selected
   workspace member and publishes explicitly to public crates.io. All live writes
   require actual reviewers and tag restrictions on the `release` environment,
   which GitHub Team offers because the organization's repositories are public.
   Evidence live uploads
   are release-tag-only, not all-runs archival.
5. `attest-binaries.yml`, `unsafe-audit.yml`, `fuzz.yml` and
   `publish-evidence.yml` are separate files because each needs a scope or a
   toolchain the common path must not carry. GitHub validates a job's scopes at
   startup even when an `if:` will skip it, so a single file declaring
   `attestations: write` would impose that grant on every caller.

## Rules

Each rule below has a test behind it. When one blocks something correct, say so in
the pull request and leave the gate intact.

The workflows:

- Every executable job runs on `ubuntu-24.04`, with an explicit timeout and
  least-privilege permissions. `pull_request_target` stays refused everywhere,
  publishers refuse fork pull requests, and CI takes no secret so a fork can run
  it.
- Keep the five tested versions identical across the consumer matrix in
  `ci-internal.yml`, the examples and the README. `ci.yml` accepts any exact
  stable version from the MSRV up and stays on stable; nightly work lives in
  its own workflow.
- Pin every action to a commit SHA, GitHub's own included, and every tool to a
  release asset plus its recorded SHA-256, verified before unpacking. zizmor
  holds the first, in its pedantic persona, with the exceptions and their
  reasons in `.github/zizmor.yml`.
- A step body is one `rust-gate` command; inputs reach it through `env`, never
  through `${{ }}` inside a body; nothing is silenced with `|| true`. The only
  Bash left is `scripts/bootstrap.sh`, the Just recipes and the gate action's
  build step, all under ShellCheck.
- Default every new input to a value that leaves an existing consumer unaffected
  on upgrade. A gate a caller did not ask for stays off.
- Treat public inputs, outputs, artifact shape and required statuses as an API.
  A breaking change needs a migration note and a `feat!:` title.
- Reusable workflows run in the consumer's checkout, so they can use only what
  GitHub provides and what they download themselves.

The gate crate under `gate/`:

- Standard library only, unsafe code forbidden, no `#[allow]`, every item
  documented, private ones included. A refusal is a message and an exit
  status, never a panic: the manifest denies every way to panic outside the
  crate's own unit tests.
- Three layers, steps over checks over the runner, each entered through its own
  `mod.rs`, imports flowing one way and never in a cycle. Every module of
  `gate/src/checks/` that holds a function has unit tests.
- A layer door offers nothing that a single module uses: two callers make a
  seam real, and a name kept for one names its reason in `HYPOTHETICAL_SEAMS`.
  A step that needs a seam of its own becomes a directory, its parts
  `pub(super)` beside it, never a module in a shared layer.
- Cognitive complexity 15, 100 lines and 5 parameters per function; 500 lines
  of code per file, doc comments not counted, with the files over 300 reported
  and not refused; 100 columns per line of Rust, shell and Just.

The tests:

- Name a module in two words at least, a test in four, never behind a `test_`
  prefix or a `_works`, `_ok` or `_test` suffix.
- A test module imports only what it names from `crate::harness`, explicitly:
  no glob, no `super`, no other module. The harness is the one door to the
  binary; tests read the trace, the declarations and what a step wrote, not
  the gate's source.
- A contract test runs every registered step, and a test asserts every refusal
  the gate can print, word for word, through `refused(&output, message)`.

The documents:

- Every relative link resolves, every backticked test name exists, and every
  input, output and secret has its row. `just docs` writes `docs/steps.md` and
  every table between `generated by just docs` markers; nobody edits those by
  hand. Change the source instead: a workflow input's `description`, the
  `# Contract:` line at the top of a reusable workflow, the `# tool:` line above
  a pin in `mise.toml`, or `docs/gates.toml`. A test refuses a stale table.

## Write

Plain sentences, no em or en dashes, no bold-label lists. A version is a floor
(the MSRV), an accepted version or one of the tested set; nothing is
"supported". Keep references scoped to this provider and its documented upstream tools.

## Boundaries

- Rust distributions, crates.io dependency reads and explicit public publication
  are separate integrations. Downloads use direct upstream origins; never infer
  a write destination or authorization from successful dependency reads.
- Never invent a commit SHA, an organisation variable, a secret or a licence
  policy. A gate change repins this repository's own gate action in a second
  pull request; the procedure is in CONTRIBUTING.
- Live publication, remote writes, credentials, Git operations and releases need
  explicit authorization in the request.
- Local checks establish local behaviour. Nothing here exercises a hosted
  runner, GitHub Release or crates.io publication; report them as untested. When a claim outruns
  its proof, add the assertion rather than the claim.
- `.tools/`, `target/` and generated SBOMs are build output, not deliverables.
