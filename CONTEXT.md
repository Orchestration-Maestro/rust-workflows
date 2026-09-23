# Rust Workflow Context

Shared language for reusable Rust CI and protected publication at Orchestration-Maestro.

Each entry says what the word means *here*. Where our meaning is narrower than the
usual one, the entry says what it excludes, because the gap is where the
misunderstandings happen.

## The work

**Consumer**: A team's Rust package or Cargo workspace validated by the reusable
workflows. Its source revision is the subject of the checks and release artifacts.

**Fixture**: A project under `examples/` that stands in for a consumer. The three
shapes a real caller can have (one binary, one library, a workspace), so the gate
exercises the workflows against something a compiler actually accepts.

**Harness**: The one door of the test crate, `tests/harness/`. Every test module
names what it takes from it, and nothing reaches the workflows, the gate or the
fixture around it.
_Avoid_: test harness for the consumer matrix.

**Consumer matrix**: The fifteen cases `ci-internal.yml` runs on every pull
request, three fixtures across the five tested versions, plus both dry-run
publishers. It proves the tested set; `ci.yml` accepts more.
_Avoid_: harness, CI harness, test harness.

**Step declaration**: The `STEPS` constant beside a step's code, naming the
workflow, the id, the summary, the inputs it reads, the tools it runs and the
reports it writes. The runner refuses anything undeclared while the step runs,
and `rust-gate describe` renders the declarations as `docs/steps.md`.

**Trace**: The file `RUST_GATE_TRACE` names, one line per command with the
environment overrides, redacted except for reviewed non-sensitive values,
written just before the command starts. A contract test reads the plan from it
instead of from a function body.

**Refusal**: The message a step prints before it exits non-zero. Never a panic:
the crate denies every way to panic, and a test asserts every refusal verbatim.

**Contract test**: A test in `tests/` that parses real workflow YAML and runs the
step body it finds there against controlled stand-ins. It checks behaviour, not
wording: a test that only greps for a string proves nothing about what the step does.

**Step body**: What a step runs: one `rust-gate` command, its inputs arriving as
environment variables. A reusable workflow runs in the caller's checkout and
cannot read this repository, so the gate action builds the binary from this
repository's pinned commit first; the contract tests run that exact command, and
the local gate runs the same commands against the example fixtures.

**rust-gate**: The standard-library-only binary in `gate/` that holds every step
body as a subcommand: `rust-gate <id>` for a `ci.yml` step, `rust-gate <workflow>
<step>` for the other workflows, and `install-tools` and `verify-payload` for the
two steps several workflows share.

**Gate action**: The composite action under `.github/actions/gate` that builds
`rust-gate` from the pinned commit of this repository and puts it on the PATH,
called by commit SHA since no other sharing mechanism exists. One pin covers
every call site.

**Example gate**: The ignored test in `tests/` that replays `ci.yml`'s own step
bodies, read by id, against every fixture with the pinned toolbelt and the owned
floors (coverage 90, unsafe denied, every offline gate on). `just check` is its
one caller; plain `cargo test` skips it. One environment table serves it and the
contract tests, so the two cannot disagree about what a body reads.

## Checks

**North Star**: The motto every change is held to, automate the guardrails to
deliver faster, with higher quality, and more securely, written out in
`docs/standards/northstar.md` as four axes with one bar each, the gate that
holds it and the test that proves it.

**Proof**: The test a gate names in the README's Gates tables, the one that
fails when the gate slips. A test that no longer exists turns the gate red.

**Gate**: A check that can refuse. A step that reports without being able to fail
the run is a report, not a gate, and the distinction is load-bearing throughout
this repository: `|| true` turns one into the other silently, which is why a test
rejects it.

**Opt-in gate**: A gate a caller switches on. All but four default to off or
permissive, so upgrading never fails a consumer who did not ask for it; mutation
testing, the unused-dependency check, the `unsafe` ban and SARIF reports default
to on because a golden workflow enforces the standard, and each is one input to switch off. The dependency source policy and the scaffolding lints
are not gates a caller selects: they hold for every project. The
scorecard makes the resulting set visible.

**Informational report**: A step that writes what it found and always exits
zero: `complexity` and `duplication`. It is a report, never a gate, and the
scorecard carries its line without changing a count.

**Toolbelt**: The pinned maintainer tools in `mise.toml`, installed against the
checksums in `mise.lock` and linked into ignored `.tools/`. Distinct from what a
workflow installs on a runner, which is the `install-tools` table.

**Scorecard**: The per-run record of which gates were active out of those
available, written as JSON, prose and a self-contained SVG. It answers "what was
actually checked on this run", which a green tick does not.

**Reports artifact**: The `<artifact-name>-reports` bundle every run uploads. A
gate that did not run still writes its report saying so, so a reader can tell "not
checked" from "checked and passed" without opening the workflow.

## Versions and dependencies

**Pin**: An exact version plus a verified digest. A tag is not a pin: its bytes can
change under it. External actions are pinned to a 40-character commit SHA, tools
to a release asset and its recorded SHA-256.

**MSRV**: The minimum Rust version a package declares through `rust-version`. Every
workspace member must declare one, and it must not exceed the compiler under test;
an undeclared MSRV compiles today and breaks silently for a consumer on an older
toolchain.

**Tested version**: One of the five pins the consumer matrix runs on every pull
request: MSRV `1.85.0` plus `1.95.0`, `1.96.1`, `1.97.1` and the `1.98.1`
default. What CI accepts is wider: any exact stable version from the MSRV up,
never a channel like `stable`, and never nightly inside `ci.yml`.

**deny.toml**: The consumer's committed licence, dependency-ban and source policy.
It belongs to them, not to us: we run the check against their file. Without one,
the default policy applies: approved registries only, no git dependency, no
wildcard version; licences alone are checked against the `LICENSE_ALLOWLIST`
organization variable when administrators set it, and the report says so when no
licence list applied.

## Release

**Release payload**: The validated binaries or crate package associated with an
exact source revision, compiler selection and artifact identity.

**Artifact identity**: The hash distinguishing one run's artifacts from another's.
It covers the project path, the caller's key, the selected compiler and the run
identity, so two matrix entries cannot overwrite each other's output.

**SBOM**: The bill of materials for the payload, shipped in both CycloneDX and
SPDX. Two formats because consumer tooling is split between them, and one a
consumer cannot read is no bill of materials at all.

**Auditable binary**: A released binary carrying its resolved dependency list in a
`.dep-v0` ELF section. It answers "what is in this file" from the file itself,
which still works after the binary has been copied somewhere its SBOM did not
follow.

**Reproducible build**: A claim supported by building twice into different
directories and comparing digests. A successful build is not evidence of
reproducibility; the comparison is.

**Provenance**: The signed statement of what built an artifact, from which source
revision. It describes origin, not quality.

**Attestation**: Provenance recorded and verifiable through GitHub and Sigstore.
It uses `id-token: write`; GitHub Release uploads instead use the scoped job
token and do not request OIDC. Attestations are written but unproven
until administrators enable the capability.

**Dry-run**: Verification of the publication path without writing to a destination.
It is not evidence that live registry authorization works.

**Publication destination**: Public crates.io for crates, or an existing GitHub
Release for binaries and release evidence. Both require explicit live approval;
ordinary expiring CI artifacts are not a publication destination.

**CI evidence**: A scoped receipt and associated reports from a specific run and
attempt. The completed run is authoritative; evidence is not a universal warranty.
