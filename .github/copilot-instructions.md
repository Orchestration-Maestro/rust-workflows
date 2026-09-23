# Copilot instructions for Rust Workflows

## Start here

This repository provides reusable GitHub Actions CI and protected publishing for
Rust packages and Cargo workspaces. It is not a Rust application. Paths below are
relative to this repository, not the temporary parent workspace.

Before editing, read [AGENTS.md](../AGENTS.md) for the authoritative constraints
and [CONTRIBUTING.md](../CONTRIBUTING.md) for tools and verification. For CI changes,
read [docs/ci.md](../docs/ci.md); for release changes, read
[docs/publishing.md](../docs/publishing.md). For runner, registry, identity or
installation changes, read [platform requirements](../docs/platform-requirements.md).
For quality or engineering changes, read
[northstar.md](../docs/standards/northstar.md) and
[engineering.md](../docs/standards/engineering.md).

Describe this project's Rust functionality directly. Keep changes scoped to the
request, preserve existing artwork, and use current executable contracts rather
than treating historical planning notes as instructions to start additional work.

## Repository tree

This is the complete maintained-file tree. Build/cache directories, Git metadata,
generated SBOM output and local download markers are intentionally excluded.

```text
.                                               # Repository root
├── .github/                                    # GitHub metadata, templates and workflows
│   ├── ISSUE_TEMPLATE/                         # Structured issue forms and the chooser
│   │   ├── bug_report.yml                      # Bug form: runner, arch, regression, release impact
│   │   ├── config.yml                          # Chooser links for security, support and contracts
│   │   └── feature_request.yml                 # Feature form, including breaking-change intent
│   ├── actions/                                # The one action every workflow calls by commit SHA
│   │   └── gate/                               # Builds rust-gate from this commit and puts it on the PATH
│   │       └── action.yml                      # Cache per commit, then cargo build with the crate's own compiler
│   ├── assets/                                 # Repository artwork referenced by the README
│   │   ├── how-it-works.svg                    # Diagram in the README: what calls what, and what publication must pass
│   │   └── rust-workflows.webp                 # README banner
│   ├── release-please/                         # Release-please configuration and version manifest
│   │   ├── config.json                         # Release strategy and changelog sections
│   │   └── manifest.json                       # Current released version per package
│   ├── workflows/                              # Callable workflows and this repository own CI
│   │   ├── attest-binaries.yml                 # Isolated signing job; re-verifies before it signs
│   │   ├── ci-internal.yml                     # Repository quality, the consumer matrix and both dry-run publishers on every pull request
│   │   ├── ci.yml                              # The reusable Rust CI a consumer calls
│   │   ├── fuzz.yml                            # Bounded fuzz regression on a nightly toolchain
│   │   ├── publish-binaries.yml                # Protected binary release, dry-run by default
│   │   ├── publish-crate.yml                   # Protected crate publication explicitly to public crates.io
│   │   ├── publish-evidence.yml                # Verifies release reports, dry-run first, then uploads GitHub Release assets
│   │   ├── release-please.yml                  # Release pull request and tag on a GitHub App token, skipped until the app is set up
│   │   ├── unsafe-audit.yml                    # Undefined-behaviour audit under Miri
│   │   └── upload-sarif.yml                    # Clippy and secret-scan SARIF into code scanning, the one job with security-events: write
│   ├── CODEOWNERS                              # Required reviewers for every change
│   ├── actionlint.yml                          # Uses the built-in GitHub-hosted runner labels
│   ├── copilot-instructions.md                 # This file: the maintained-file map
│   ├── dependabot.yml                          # Action and Cargo update schedule
│   ├── pull_request_template.md                # Review checklist and release-impact prompt
│   └── zizmor.yml                              # Workflow audit exceptions, each with its reason
├── docs/                                       # Contracts, standards and platform boundaries
│   ├── standards/                              # The bars this repository holds itself to
│   │   ├── engineering.md                      # Engineering rules, each with its enforcement status
│   │   ├── northstar.md                        # The motto, four axes, the KPI table and the test behind each bar
│   │   └── security.md                         # Security requirements and how they are enforced
│   ├── README.md                               # Complete workflow contracts and usage examples
│   ├── ci.md                                   # Every CI input, output, gate and report
│   ├── platform-requirements.md                # Runners, registries, identity and their unknowns
│   ├── publishing.md                           # Dry-run and protected publication procedures
│   ├── rust-gate.md                            # The gate: why one binary holds the step bodies, its invariants and layout
│   └── steps.md                                # Generated by rust-gate describe: every step, what it reads, runs and writes
├── examples/                                   # Real consumer shapes the gate runs against
│   ├── binary/                                 # Single binary package fixture
│   │   ├── src/                                # Workspace library sources
│   │   │   ├── lib.rs                          # Workspace library surface with doc comments
│   │   │   └── main.rs                         # Binary entry point
│   │   ├── tests/                              # Workflow contract validation
│   │   │   └── cli.rs                          # CLI integration test
│   │   ├── Cargo.lock                          # Locked resolution for the test crate
│   │   ├── Cargo.toml                          # Isolated workflow-contract test target
│   │   ├── LICENSE                             # MIT notice included in the Cargo package
│   │   └── rust-toolchain.toml                 # Exact stable compiler pin for tests
│   ├── library/                                # Library-only package fixture
│   │   ├── src/                                # Workspace library sources
│   │   │   └── lib.rs                          # Workspace library surface with doc comments
│   │   ├── Cargo.lock                          # Locked resolution for the test crate
│   │   ├── Cargo.toml                          # Isolated workflow-contract test target
│   │   ├── LICENSE                             # MIT notice included in the Cargo package
│   │   └── rust-toolchain.toml                 # Exact stable compiler pin for tests
│   └── workspace/                              # Multi-member Cargo workspace fixture
│       ├── app/                                # Workspace binary package
│       │   ├── src/                            # Workspace library sources
│       │   │   └── main.rs                     # Binary entry point
│       │   ├── tests/                          # Workflow contract validation
│       │   │   └── cli.rs                      # CLI integration test
│       │   ├── Cargo.toml                      # Isolated workflow-contract test target
│       │   └── LICENSE                         # MIT notice included in the Cargo package
│       ├── core/                               # Workspace library package
│       │   ├── src/                            # Workspace library sources
│       │   │   └── lib.rs                      # Workspace library surface with doc comments
│       │   ├── Cargo.toml                      # Isolated workflow-contract test target
│       │   └── LICENSE                         # MIT notice included in the Cargo package
│       ├── Cargo.lock                          # Locked resolution for the test crate
│       ├── Cargo.toml                          # Isolated workflow-contract test target
│       ├── deny.toml                           # Licence allowlist, dependency bans and source policy
│       └── rust-toolchain.toml                 # Exact stable compiler pin for tests
├── gate/                                       # The gate: one binary the workflows build at the pinned commit
│   ├── src/                                    # The three layers: runner, checks, steps
│   │   ├── checks/                             # What the steps share, built on the runner and never on a step
│   │   │   ├── cargo_metadata.rs               # The jaq programs several steps read over Cargo's records
│   │   │   ├── checkout_paths.rs               # Canonical forms, containment in the checkout, symlinks, Rust sources
│   │   │   ├── inputs.rs                       # The ci.yml inputs with a shape of their own: policies, threshold and key, typed
│   │   │   ├── mod.rs                          # The registry of every step, run and describe, the two doors main.rs calls
│   │   │   ├── private_directories.rs          # Private temporary directories under the runner's own
│   │   │   ├── release_boundary.rs             # What both publishers ask of a release before anything is published
│   │   │   ├── rust_versions.rs                # Rust version strings compared the way sort -V compared them
│   │   │   └── simple_names.rs                 # One validator for every simple-name rule, and hex strings
│   │   ├── runner/                             # The runner as the gate sees it: inputs, GITHUB_* files, tools
│   │   │   ├── commands.rs                     # Running a pinned tool: streamed, captured into a report, or both, and the trace
│   │   │   ├── github_actions.rs               # Inputs from env, the four GITHUB_* writers, masking, the job's directories
│   │   │   ├── mod.rs                          # The registry of every step, run and describe, the two doors main.rs calls
│   │   │   ├── outcome.rs                      # How a step ends: complete, or failed with the tool's own status or with one message
│   │   │   └── step_declaration.rs             # A step as data: what it declares, and the refusal of anything undeclared
│   │   ├── steps/                              # One module per step, private to the directory; mod.rs is the one door
│   │   │   ├── quality_scorecard/              # rust-gate scorecard: the step and the value it renders
│   │   │   │   ├── mod.rs                      # rust-gate scorecard: what ran, as JSON, Markdown and a self-contained badge
│   │   │   │   └── scorecard.rs                # A run's scorecard as a value: its controls, and the JSON, Markdown and badge of them
│   │   │   ├── attest_binaries.rs              # rust-gate attest-binaries: validate, extract the SBOM, verify, record the outcome
│   │   │   ├── binary_hardening.rs             # rust-gate hardening: reproducible, PIE, RELRO, no executable stack, auditable
│   │   │   ├── configure_cargo_registry.rs       # rust-gate registry: private job-local Cargo home for direct crates.io
│   │   │   ├── declared_msrv.rs                # rust-gate msrv: every member declares a rust-version the compiler under test reaches
│   │   │   ├── dependency_policy.rs            # rust-gate licenses: the consumer's deny.toml, or the generated default policy
│   │   │   ├── feature_combinations.rs         # rust-gate features: cargo hack builds each declared feature, not only the default set
│   │   │   ├── format_lint_test.rs             # rust-gate quality: fmt, Clippy, tests, doc tests, strict rustdoc
│   │   │   ├── fuzz_regression.rs              # rust-gate fuzz: inputs, nightly toolchain with cargo-fuzz, corpus replay and exploration
│   │   │   ├── install_toolchain.rs            # rust-gate install-tools: what it refuses, honours, and ci.yml installs
│   │   │   ├── install_tools.rs                # rust-gate install-tools: official release assets, digests verified before extraction
│   │   │   ├── line_coverage.rs                # rust-gate coverage: LCOV line coverage, failing below the threshold
│   │   │   ├── mod.rs                          # The registry of every step, run and describe, the two doors main.rs calls
│   │   │   ├── mutation_testing.rs             # rust-gate mutants: cargo-mutants scoped to the change, a diff or the last commit
│   │   │   ├── publish_binaries.rs             # rust-gate publish-binaries: the publication boundary of the binary publisher
│   │   │   ├── publish_crate.rs                # rust-gate publish-crate: boundary, toolchain, package, semver, publish
│   │   │   ├── publish_evidence.rs             # rust-gate publish-evidence: validate reports and upload release assets
│   │   │   ├── recorded_audits.rs              # rust-gate vet: cargo-vet against the committed ledger
│   │   │   ├── release_build.rs                # rust-gate build: release tests, auditable build, packages, per-member SBOMs
│   │   │   ├── report_duplicates.rs            # rust-gate duplication: functions whose syntax trees look alike, reported and never enforced
│   │   │   ├── report_sizes.rs                 # rust-gate complexity: function and file sizes, reported and never enforced
│   │   │   ├── require_every_check.rs          # rust-gate required: the one status a branch protection can require
│   │   │   ├── secret_scan.rs                  # rust-gate secrets: Gitleaks over the current revision, findings redacted
│   │   │   ├── stage_payload.rs                # rust-gate stage: the immutable payload, its provenance and checksums
│   │   │   ├── unsafe_audit.rs                 # rust-gate unsafe-audit: inputs, nightly toolchain with Miri, the run and its reach
│   │   │   ├── unused_dependencies.rs          # rust-gate unused: cargo-machete on declared-but-unused dependencies
│   │   │   ├── validate_inputs.rs              # rust-gate validate: every ci.yml input checked before any side effect
│   │   │   ├── verify_payload.rs               # rust-gate verify-payload: manifest, checksums, symlinks, revision and provenance
│   │   │   └── vulnerability_audit.rs          # rust-gate audit: the lockfile against RustSec, yanked and unsound denied
│   │   └── main.rs                             # Argument parsing only; every step runs through steps::run, describe prints the steps
│   ├── Cargo.lock                              # Locked resolution for the test crate
│   ├── Cargo.toml                              # Isolated workflow-contract test target
│   └── LICENSE                                 # MIT notice included in the Cargo package
├── scripts/                                    # Provisioning that has to run before the toolbelt exists
│   └── bootstrap.sh                            # Verified pinned Linux x64 toolbelt and hooks
├── tests/                                      # Workflow contract validation
│   ├── ci/                                     # ci.yml, one module per gate it runs: what each step accepts, refuses, builds and reports
│   │   ├── complexity_report.rs                # ci.yml: function and file sizes, reported and never held against the run
│   │   ├── duplication_report.rs               # ci.yml: duplicated functions, reported and never held against the run
│   │   ├── feature_combinations.rs             # ci.yml: real per-feature and combined compilation, plus replay coverage
│   │   ├── input_validation.rs                 # unsafe-audit.yml and fuzz.yml: every malformed input refused before a toolchain is touched
│   │   ├── install_tools.rs                    # rust-gate install-tools: what it refuses, honours, and ci.yml installs
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── quality_gates.rs                    # ci.yml: lint, documentation, coverage and analysis gates, each proven to fail
│   │   ├── quality_reports.rs                  # ci.yml: diagnostics survive failing tools without changing their verdict
│   │   ├── release_payload.rs                  # ci.yml: release build, payload, bills of materials, and the example gate
│   │   ├── release_payload_refusals.rs         # The release payload's refusals: lockfile drift, unhardened or irreproducible binaries, malformed staging
│   │   ├── scorecard_and_required_status.rs    # ci.yml: the scorecard, the required status and mutation testing
│   │   ├── scorecard_states.rs                 # ci.yml: selection, applicability and execution reported separately
│   │   ├── supply_chain.rs                     # ci.yml: dependency policy, direct crates.io reads and the scanners
│   │   └── workspace_boundary.rs               # ci.yml: a workspace whose manifests or sources reach outside the checkout is refused before any lint
│   ├── gate/                                   # The gate and the tests as structures: layers, no import cycle, the step registry, what holds every step and refusal
│   │   ├── acyclic_imports.rs                  # No crate holds an import cycle: no file names another that names it back, directly or through others
│   │   ├── layer_boundaries.rs                 # The gate's three layers and the harness door, enforced on every import
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── step_and_refusal_coverage.rs        # Every declared step is run by a contract test; every refusal the binary composes is asserted by a test
│   │   └── step_registry.rs                    # The step registry: declarations, the generated document, every body registered
│   ├── harness/                                # The one door of the tests: the repository, YAML readers, gate declarations and the fixture
│   │   ├── fixture.rs                          # One temporary checkout, one environment table, a step run against stand-ins, every command traced
│   │   ├── gate_declarations.rs                # The gate built once per test process, and what rust-gate describe declares about its steps
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── native_runtime.rs                   # Real native Windows processes, registry boundary and temporary trees
│   │   ├── repository.rs                       # The repository root, the toolbelt, commands run to completion, temporary directories, every test file
│   │   └── workflow_yaml.rs                    # Readers of workflow and action YAML: whole documents, one step's body, tool rows, jaq queries
│   ├── nightly/                                # The nightly workflows, unsafe-audit.yml and fuzz.yml, outside the stable policy
│   │   ├── fuzz_regression.rs                  # fuzz.yml: the nightly with rust-src and cargo-fuzz, every committed target replayed with the corpus first
│   │   ├── input_validation.rs                 # unsafe-audit.yml and fuzz.yml: every malformed input refused before a toolchain is touched
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   └── unsafe_audit.rs                     # unsafe-audit.yml: Miri on the selected nightly, and no pass without a test executed under it
│   ├── publishers/                             # The publishing workflows: dry run first, a live path only a protected release takes, what each verifies before handing a release on
│   │   ├── binary_attestation.rs               # attest-binaries.yml: signing only what the job verified
│   │   ├── crate_and_binaries.rs               # Publishers: dry-run first, and a live path only a protected release takes
│   │   ├── crate_toolchains.rs                 # publish-crate.yml: the dry run installs the pinned toolchain, the publication the validated one
│   │   ├── evidence_publication.rs             # publish-evidence.yml: source-bound release reports and explicit dry-runs
│   │   ├── github_releases.rs                  # Live approval, release identity and no-overwrite upload contracts
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── payload_verification.rs             # The shared payload verification: revision, checksum manifest and provenance, every flaw refused by name
│   │   └── recorded_attestation.rs             # attest-binaries.yml: the recorded attestation verified through gh, refused unless it covers the digest
│   ├── repository/                             # The repository itself: files, documents, pins, sizes, policies, hooks, scans and the names of its tests
│   │   ├── commit_message_hooks.rs             # The commit-msg hooks: a conventional header first, 80 columns, refused by prek in a fresh repository
│   │   ├── documentation_coverage.rs           # Every report, input and secret documented; links resolve; cited tests exist
│   │   ├── evidence_receipt.rs                 # The evidence receipt: produced only when every upstream result succeeded
│   │   ├── gate_action.rs                      # The gate action: one pin at every call site, and a commit that ships it
│   │   ├── metadata_and_inventory.rs           # Repository files, hook, editor and release policies, the Copilot inventory
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── naming_rules.rs                     # Every test module names what it proves in two words at least, every test function in four, no test_ prefix, no _works, _ok or _test suffix
│   │   ├── north_star.rs                       # Promised controls run, every gate names its proof, no lint silenced
│   │   ├── pinned_tool_usage.rs                # Every job installs every pinned tool it invokes before a step reads it
│   │   ├── secret_and_advisory_scans.rs        # Gitleaks over the tree; RustSec audits under CHECK_NETWORK=1
│   │   ├── size_limits.rs                      # The size limits: Clippy thresholds, 300-line files, 100-column lines
│   │   ├── toolbelt_and_shellcheck.rs          # Toolbelt links to the locked builds; ShellCheck over every Bash line left
│   │   ├── version_pins.rs                     # Tool versions, the toolchain pin and the speed target, one copy each
│   │   └── workflow_policy.rs                  # Permissions, timeouts, runners, trust boundaries, shell policy and the local calls
│   ├── Cargo.lock                              # Locked resolution for the test crate
│   ├── Cargo.toml                              # Isolated workflow-contract test target
│   ├── LICENSE                                 # MIT notice included in the Cargo package
│   ├── native_windows.rs                       # Native gate units, ACLs and real Cargo examples, not Linux ELF replay
│   └── workflows.rs                            # Test crate root: one directory per what the tests prove, and the harness they share
├── .editorconfig                               # UTF-8, LF, final newlines, space indentation
├── .gitattributes                              # Text normalization, Rust-aware diff, binary images
├── .gitignore                                  # Local tools/caches, Cargo build output, Windows markers
├── .pre-commit-config.yaml                     # Fast prek hooks: format, lint, basic checks; commit-msg header and column checks
├── .taplo.toml                                 # TOML formatting: arrays keep the shape they were written in
├── .yamlfmt.yml                                # YAML formatting for workflows and metadata
├── AGENTS.md                                   # Authoritative workflow objectives and constraints
├── CHANGELOG.md                                # Written by release-please from conventional commit titles
├── CONTEXT.md                                  # Domain glossary for workflows, runners and publication
├── CONTRIBUTING.md                             # Pinned tools, setup, checks and review procedure
├── LICENSE                                     # MIT licence for the repository and its Cargo packages
├── README.md                                   # Complete workflow contracts and usage examples
├── SECURITY.md                                 # Runner trust, token handling, publication boundaries
├── SUPPORT.md                                  # Troubleshooting and safe diagnostic steps
├── clippy.toml                                 # The size limits Clippy holds every crate to; the binary's unit tests may unwrap
├── deny.toml                                   # Licence allowlist, dependency bans and source policy
├── justfile                                    # Development commands: setup and check
├── mise.lock                                   # Resolved URL and checksum of every toolbelt download
├── mise.toml                                   # The toolbelt: each tool at the version CI pins
├── rust-toolchain.toml                         # The one compiler pin: the gate, the tests and the action build with it
├── typos.toml                                  # The words this repository means, so the spell checker reports only mistakes
└── version.txt                                 # Simple-release version, not a compiler pin
```

## Root files: repository policy and developer entry points

These files define how contributors work on the repository. The examples and test
harness have their own Cargo manifests; there is deliberately no root Cargo package.

## `.github/`: automation, ownership and contribution intake

This section contains the executable workflows and GitHub-specific maintenance
configuration. Workflow files are active only when this repository is at the
GitHub repository root. Metadata files do not create runner groups, grant access
or enable organization apps by themselves.

## `docs/`: detailed contracts, quality baseline and security boundaries

The direct documents describe the current public and platform contracts:
`docs/ci.md`, `docs/publishing.md` and `docs/platform-requirements.md`, indexed by
`docs/README.md`. `docs/standards/` holds what guides every change: the quality
targets in `docs/standards/northstar.md`, the engineering rules and rustdoc style
in `docs/standards/engineering.md`, and the security boundaries and enforcement
standards in `docs/standards/security.md`.

## `examples/`: real consumers of the workflows

These packages are small executable compatibility fixtures, not production
libraries supplied by this repository. They use Rust 2024, declare MSRV, forbid
unsafe code and rely on the standard library plus local workspace dependencies.
Keep their behavior assertions meaningful when changing them.

### `examples/binary/`: package with a CLI

The library holds checked arithmetic; the CLI and integration test exercise real
process execution and release-binary behavior.

### `examples/library/`: library-only package

This fixture ensures CI and crate packaging work when there is no release binary.

### `examples/workspace/`: related library and application members

The workspace exercises multi-member Cargo operations, local dependencies and
selected-package publishing. Its `examples/workspace/core/` member owns
arithmetic; `examples/workspace/app/` consumes
that library and exposes a real CLI.

## `tests/`: isolated development test crate

This crate validates repository and workflow contracts; it is not linked into
consumer applications. Tests parse actual YAML and execute extracted workflow
commands against controlled stand-ins, complemented by real Cargo fixture gates.
The modules sit in one directory per what they prove, `tests/ci/`,
`tests/publishers/`, `tests/nightly/`, `tests/gate/` and `tests/repository/`,
behind the one door of `tests/harness/`; a module is named in two words at
least and a test function in four, and `tests/repository/naming_rules.rs`
refuses anything shorter.

## Change and verification procedure

1. Trace the affected workflow from checkout through its required result. Consumer
   checkout is not this workflow repository: never assume sibling helper files or
   an unpublished setup action are available there.
2. Preserve `ubuntu-24.04`, the stable-only MSRV-floored version policy, exact
   action/tool pins, least
   privilege, timeouts and fail-closed validation. Keep publication dry-run-first
   and tied to the validated source revision.
3. Add an executable regression check for behavior changes. Update public contract
   docs and the changelog when inputs, outputs, artifact identity or support change.
4. If files are added, moved or removed, update the tree and its inline comments here. Do not weaken the inventory test to hide documentation drift.
5. Run `just check`. Report commands actually run; local evidence does not establish GitHub
   authentication, runner readiness or live publication. Remote changes and
   releases require explicit authorization.
