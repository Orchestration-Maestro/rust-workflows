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
├── .config/                                    # Tool settings that live in a directory
│   └── nextest.toml                            # TST-004: the nextest profile, retries = 0; rendered by rust-gate sync
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
│   │   ├── dependabot-auto-merge.yml           # Queues Dependabot patch and minor updates to merge on the organization's bot token
│   │   ├── docs-sync.yml                       # On a pull request from this repository, the bot commits the tables just docs regenerated
│   │   ├── fuzz.yml                            # Bounded fuzz regression on a nightly toolchain
│   │   ├── hygiene.yml                         # The reusable CI of a repository without Rust: secrets, hygiene, managed files, hooks
│   │   ├── publish-binaries.yml                # Protected binary release, dry-run by default
│   │   ├── publish-crate.yml                   # Protected crate publication explicitly to public crates.io
│   │   ├── publish-evidence.yml                # Verifies release reports, dry-run first, then uploads GitHub Release assets
│   │   ├── release-please.yml                  # Release pull request and tag on a GitHub App token, skipped until the app is set up
│   │   ├── scorecard.yml                       # Weekly OpenSSF Scorecard of this repository, published for the badge and shown in code scanning
│   │   ├── tool-updates.yml                    # Weekly pull request moving every pinned tool to its latest release, on the bot token
│   │   ├── unsafe-audit.yml                    # Undefined-behaviour audit under Miri
│   │   ├── upload-coverage.yml                 # Line coverage and test results into Codecov, the one job with id-token: write
│   │   └── upload-sarif.yml                    # Clippy and secret-scan SARIF into code scanning, the one job with security-events: write
│   ├── CODEOWNERS                              # Required reviewers for every change
│   ├── actionlint.yml                          # Uses the built-in GitHub-hosted runner labels
│   ├── copilot-instructions.md                 # This file: the maintained-file map
│   ├── dependabot.yml                          # Weekly action and Cargo updates, patch and minor grouped per ecosystem
│   ├── pull_request_template.md                # Review checklist and release-impact prompt
│   └── zizmor.yml                              # Workflow audit exceptions, each with its reason
├── docs/                                       # Contracts, standards and platform boundaries
│   ├── generators/                             # The jaq filters just docs renders the generated tables with
│   │   ├── gates.jq                            # The README's three gate tables from gates.toml
│   │   ├── inputs.jq                           # A workflow's inputs as a Markdown table
│   │   ├── outputs.jq                          # A workflow's outputs as a Markdown table
│   │   ├── own-inputs.jq                       # The inputs one publisher has and the other lacks
│   │   └── shared-inputs.jq                    # The inputs both publishers share, the forwarded ones in one row
│   ├── standards/                              # The bars this repository holds itself to
│   │   ├── engineering.md                      # Engineering rules, each with its enforcement status
│   │   ├── northstar.md                        # The motto, four axes, the KPI table and the test behind each bar
│   │   └── security.md                         # Security requirements and how they are enforced
│   ├── superpowers/                            # Designs written and approved before a change is built
│   │   ├── plans/                              # One implementation plan per approved design, task by task
│   │   │   ├── 2026-09-24-org-quality-gate-1-architecture.md  # Plan 1 of 2: the module structure rules, and this repository held to them
│   │   │   └── 2026-09-24-org-quality-gate-2-complete.md  # Plan 2 of 2: every remaining rule, the generated files, the release and the repositories
│   │   └── specs/                              # One approved design per change, named by date and topic
│   │       └── 2026-09-24-org-quality-gate-design.md  # The quality gate every organization repository inherits, and how
│   ├── README.md                               # Complete workflow contracts and usage examples
│   ├── ci.md                                   # Every CI input, output, gate and report
│   ├── gates.toml                              # Every gate the README lists: what fails it, its switch, its standard and proof
│   ├── platform-requirements.md                # Runners, registries, identity and their unknowns
│   ├── publishing.md                           # Dry-run and protected publication procedures
│   ├── rust-gate.md                            # The gate: why one binary holds the step bodies, its invariants and layout
│   └── steps.md                                # Generated by rust-gate describe: every step, what it reads, runs and writes
├── examples/                                   # Real consumer shapes the gate runs against
│   ├── binary/                                 # Single binary package fixture
│   │   ├── src/                                # Workspace library sources
│   │   │   ├── lib.rs                          # Workspace library surface with doc comments
│   │   │   └── main.rs                         # Binary entry point
│   │   ├── supply-chain/                       # cargo-vet ledger: the six imports VET-001 requires, and the exemptions
│   │   │   ├── audits.toml                     # The audits recorded here
│   │   │   ├── config.toml                     # The imports and the reviewed exemptions
│   │   │   └── imports.lock                    # The imported audits, pinned for cargo vet --locked
│   │   ├── tests/                              # Workflow contract validation
│   │   │   └── cli.rs                          # CLI integration test
│   │   ├── Cargo.lock                          # Locked resolution for the test crate
│   │   ├── Cargo.toml                          # Isolated workflow-contract test target
│   │   ├── LICENSE                             # MIT notice included in the Cargo package
│   │   ├── README.md                           # What the fixture is, for crates.io
│   │   └── rust-toolchain.toml                 # Exact stable compiler pin for tests
│   ├── library/                                # Library-only package fixture
│   │   ├── src/                                # Workspace library sources
│   │   │   └── lib.rs                          # Workspace library surface with doc comments
│   │   ├── supply-chain/                       # cargo-vet ledger: the six imports VET-001 requires, and the exemptions
│   │   │   ├── audits.toml                     # The audits recorded here
│   │   │   ├── config.toml                     # The imports and the reviewed exemptions
│   │   │   └── imports.lock                    # The imported audits, pinned for cargo vet --locked
│   │   ├── Cargo.lock                          # Locked resolution for the test crate
│   │   ├── Cargo.toml                          # Isolated workflow-contract test target
│   │   ├── LICENSE                             # MIT notice included in the Cargo package
│   │   ├── README.md                           # What the fixture is, for crates.io
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
│       │   ├── LICENSE                         # MIT notice included in the Cargo package
│       │   └── README.md                       # What the member is, for crates.io
│       ├── supply-chain/                       # cargo-vet ledger: the six imports VET-001 requires, and the exemptions
│       │   ├── audits.toml                     # The audits recorded here
│       │   ├── config.toml                     # The imports and the reviewed exemptions
│       │   └── imports.lock                    # The imported audits, pinned for cargo vet --locked
│       ├── Cargo.lock                          # Locked resolution for the test crate
│       ├── Cargo.toml                          # Isolated workflow-contract test target
│       ├── deny.toml                           # Licence allowlist, dependency bans and source policy
│       └── rust-toolchain.toml                 # Exact stable compiler pin for tests
├── gate/                                       # The gate: one binary the workflows build at the pinned commit
│   ├── src/                                    # The three layers: runner, checks, steps
│   │   ├── checks/                             # What the steps share, built on the runner and never on a step
│   │   │   ├── cargo_metadata.rs               # The jaq programs several steps read over Cargo's records
│   │   │   ├── checkout_paths.rs               # Canonical forms, containment in the checkout, symlinks, Rust sources
│   │   │   ├── findings.rs                     # A rule's finding as one report line, and the exceptions that excuse some
│   │   │   ├── inputs.rs                       # The ci.yml inputs with a shape of their own: policies, threshold and key, typed
│   │   │   ├── lint_policy.rs                  # LNT-001: the organization's lints and clippy.toml, written and compared
│   │   │   ├── manifests.rs                    # What Cargo says beyond module trees: packages, the workspace, what members inherit
│   │   │   ├── mod.rs                          # The registry of every step, run and describe, the two doors main.rs calls
│   │   │   ├── module_tree.rs                  # Every Cargo target's module tree: files, items, named paths and re-exports
│   │   │   ├── nextest_profile.rs              # TST-004: the one nextest profile, retries = 0, the gate's and every repository's
│   │   │   ├── private_directories.rs          # Private temporary directories under the runner's own
│   │   │   ├── pull_request.rs                 # A pull request against its base: added and touched lines, the title's type
│   │   │   ├── quality_config.rs               # maestro-quality.toml read through jaq: declared layers and reasoned exceptions
│   │   │   ├── release_boundary.rs             # What both publishers ask of a release before anything is published
│   │   │   ├── rust_code.rs                    # Rust source with comments and literals blanked, and its top-level items
│   │   │   ├── rust_paths.rs                   # Every path a Rust file names: use trees expanded, a::b chains, visibilities left out
│   │   │   ├── rust_tests.rs                   # The tests inside Rust source: test functions, test-only code, waits on time
│   │   │   ├── rust_versions.rs                # Rust version strings compared the way sort -V compared them
│   │   │   ├── simple_names.rs                 # One validator for every simple-name rule, and hex strings
│   │   │   └── workflow_home.rs                # The home of the reusable workflows, whose ci.yml, Dependabot and hooks are its own
│   │   ├── runner/                             # The runner as the gate sees it: inputs, GITHUB_* files, tools
│   │   │   ├── commands.rs                     # Running a pinned tool: streamed, captured into a report, or both, and the trace
│   │   │   ├── github_actions.rs               # Inputs from env, the four GITHUB_* writers, masking, the job's directories
│   │   │   ├── mod.rs                          # The registry of every step, run and describe, the two doors main.rs calls
│   │   │   ├── outcome.rs                      # How a step ends: complete, or failed with the tool's own status or with one message
│   │   │   └── step_declaration.rs             # A step as data: what it declares, and the refusal of anything undeclared
│   │   ├── steps/                              # One module per step, private to the directory; mod.rs is the one door
│   │   │   ├── architecture/                   # rust-gate architecture: the step and one module per group of source rules
│   │   │   │   ├── cycles.rs                   # ARC-001: no import cycle between the files of a crate
│   │   │   │   ├── doors.rs                    # ARC-002 and ARC-003: doors only declare, and paths go through them
│   │   │   │   ├── layers.rs                   # ARC-004: imports run only to the layers on the right
│   │   │   │   ├── lints.rs                    # LNT-001: the lints denied in the root manifest, clippy.toml no looser
│   │   │   │   ├── mod.rs                      # The step's door: its modules and its declaration
│   │   │   │   ├── names.rs                    # NAME-001 and NAME-002: package names and test names
│   │   │   │   ├── packages.rs                 # LIB-002, TST-003, WSP-001 and WSP-002, read from the manifests
│   │   │   │   ├── roots.rs                    # ARC-006 and ARC-007: thin binary roots, and the module tree is the file tree
│   │   │   │   ├── seams.rs                    # ARC-005: a seam a door offers serves two callers
│   │   │   │   ├── sizes.rs                    # SIZE-002 and SIZE-003: lines of code per file and columns per line
│   │   │   │   ├── sources.rs                  # DOC-001, LIB-001 and TST-001: module comments, library prints, waits in tests
│   │   │   │   └── step.rs                     # The step: module trees, the rules, the exceptions and the report
│   │   │   ├── hygiene/                        # rust-gate hygiene: the step and one module per group of rules over tracked files
│   │   │   │   ├── comments.rs                 # HYG-001: work left for later names its issue
│   │   │   │   ├── files.rs                    # HYG-002 to HYG-005: snapshots, large files, modes, case, symlinks, required files
│   │   │   │   ├── mod.rs                      # The step's door: its modules and its declaration
│   │   │   │   ├── step.rs                     # The step: tracked files, the rules, the exceptions and the report
│   │   │   │   └── widths.rs                   # SIZE-003 for shell scripts and justfiles
│   │   │   ├── managed_files/                  # rust-gate sync, sync --check, init and managed-files: the files every repository holds
│   │   │   │   ├── hooks.rs                    # The commit hooks rendered: prek's checks, each tool through mise, the gate at the release
│   │   │   │   ├── mod.rs                      # The steps' door: their modules and their declaration
│   │   │   │   ├── pin.rs                      # The release a caller pins: a commit and its version
│   │   │   │   ├── render.rs                   # Every managed file rendered, this repository's own among them
│   │   │   │   └── step.rs                     # The steps: write, compare, and refuse by name what differs
│   │   │   ├── quality_scorecard/              # rust-gate scorecard: the step and the value it renders
│   │   │   │   ├── mod.rs                      # The step's door: its two modules and its declaration
│   │   │   │   ├── scorecard.rs                # A run's scorecard as a value: its controls, and the JSON, Markdown and badge of them
│   │   │   │   └── step.rs                     # rust-gate scorecard: what ran, as JSON, Markdown and a self-contained badge
│   │   │   ├── api_compatibility.rs            # rust-gate api: cargo-semver-checks against the base branch unless the title declares a break
│   │   │   ├── attest_binaries.rs              # rust-gate attest-binaries: validate, extract the SBOM, verify, record the outcome
│   │   │   ├── binary_hardening.rs             # rust-gate hardening: reproducible, PIE, RELRO, no executable stack, auditable
│   │   │   ├── changed_coverage.rs             # rust-gate changed-coverage: COV-002, the new lines of a pull request held to 95 or 90 %
│   │   │   ├── commit_hooks.rs                 # rust-gate hooks: the repository's commit hooks over every file, through the pinned prek
│   │   │   ├── configure_cargo_registry.rs       # rust-gate registry: private job-local Cargo home for direct crates.io
│   │   │   ├── declared_msrv.rs                # rust-gate msrv: every member declares a rust-version the compiler under test reaches
│   │   │   ├── dependency_policy.rs            # rust-gate licenses: the consumer's deny.toml, or the generated default policy
│   │   │   ├── feature_combinations.rs         # rust-gate features: cargo hack builds each declared feature, not only the default set
│   │   │   ├── format_lint_test.rs             # rust-gate quality: fmt, Clippy, tests, doc tests, strict rustdoc
│   │   │   ├── fuzz_regression.rs              # rust-gate fuzz: inputs, nightly toolchain with cargo-fuzz, corpus replay and exploration
│   │   │   ├── hygiene_workflow.rs             # rust-gate hygiene prepare: the checkout and reports directory of hygiene.yml
│   │   │   ├── install_toolchain.rs            # rust-gate install-tools: what it refuses, honours, and ci.yml installs
│   │   │   ├── install_tools.rs                # rust-gate install-tools: official release assets, digests verified before extraction
│   │   │   ├── line_coverage.rs                # rust-gate coverage: LCOV line coverage, failing below the threshold
│   │   │   ├── local_runs.rs                   # rust-gate architecture --local and hygiene --local: a step as a commit hook runs it
│   │   │   ├── mod.rs                          # One module per step, the registry among them; run and describe are its doors
│   │   │   ├── mutation_testing.rs             # rust-gate mutants: cargo-mutants scoped to the change, a diff or the last commit
│   │   │   ├── performance.rs                  # rust-gate performance: PRF-001, declared benchmarks base against head under gungraun
│   │   │   ├── publish_binaries.rs             # rust-gate publish-binaries: the publication boundary of the binary publisher
│   │   │   ├── publish_crate.rs                # rust-gate publish-crate: boundary, toolchain, package, semver, publish
│   │   │   ├── publish_evidence.rs             # rust-gate publish-evidence: validate reports and upload release assets
│   │   │   ├── pull_request_rules.rs           # rust-gate pull-request: PRL-001, a feature with its test, and PRL-002, its size
│   │   │   ├── recorded_audits.rs              # rust-gate vet: cargo-vet against the committed ledger
│   │   │   ├── registry.rs                     # Every step's declaration in workflow order, and the two doors main.rs calls
│   │   │   ├── release_build.rs                # rust-gate build: release tests, auditable build, packages, per-member SBOMs
│   │   │   ├── report_duplicates.rs            # rust-gate duplication: pairs of alike functions reported, three alike refused (DUP-001)
│   │   │   ├── report_sizes.rs                 # rust-gate complexity: function and file sizes, reported and never enforced
│   │   │   ├── require_every_check.rs          # rust-gate required: the one status a branch protection can require
│   │   │   ├── secret_scan.rs                  # rust-gate secrets: Gitleaks over the current revision, findings redacted
│   │   │   ├── stage_payload.rs                # rust-gate stage: the immutable payload, its provenance and checksums
│   │   │   ├── unsafe_audit.rs                 # rust-gate unsafe-audit: inputs, nightly toolchain with Miri, the run and its reach
│   │   │   ├── unused_dependencies.rs          # rust-gate unused: cargo-machete on declared-but-unused dependencies
│   │   │   ├── validate_inputs.rs              # rust-gate validate: every ci.yml input checked before any side effect
│   │   │   ├── verify_payload.rs               # rust-gate verify-payload: manifest, checksums, symlinks, revision and provenance
│   │   │   ├── vulnerability_audit.rs          # rust-gate audit: the lockfile against RustSec, yanked and unsound denied
│   │   │   └── write_lints.rs                  # rust-gate lints --write: the organization's lints into the root manifest
│   │   └── main.rs                             # Argument parsing only; every step runs through steps::run, describe prints the steps
│   ├── Cargo.lock                              # Locked resolution for the test crate
│   ├── Cargo.toml                              # Isolated workflow-contract test target
│   └── LICENSE                                 # MIT notice included in the Cargo package
├── scripts/                                    # Provisioning that has to run before the toolbelt exists
│   └── bootstrap.sh                            # Verified pinned Linux x64 toolbelt and hooks
├── supply-chain/                               # The audits the organization publishes for every repository to import
│   └── audits.toml                             # cargo-vet audits recorded by the organization, VET-001's first import
├── tests/                                      # Workflow contract validation
│   ├── ci/                                     # ci.yml, one module per gate it runs: what each step accepts, refuses, builds and reports
│   │   ├── api_compatibility.rs                # ci.yml: an undeclared API break fails a pull request; what has no API is not applicable
│   │   ├── architecture_rules.rs               # ci.yml: ARC-001 to ARC-007, each refused by name, and the exceptions maestro-quality.toml takes
│   │   ├── commit_hooks.rs                     # hooks, the local runs a hook makes, and hygiene.yml's first step
│   │   ├── complexity_report.rs                # ci.yml: function and file sizes, reported and never held against the run
│   │   ├── duplication_report.rs               # ci.yml: pairs reported, three functions of one shape refused unless excused
│   │   ├── feature_combinations.rs             # ci.yml: real per-feature and combined compilation, plus replay coverage
│   │   ├── input_validation.rs                 # unsafe-audit.yml and fuzz.yml: every malformed input refused before a toolchain is touched
│   │   ├── install_tools.rs                    # rust-gate install-tools: what it refuses, honours, and ci.yml installs
│   │   ├── managed_files.rs                    # init, sync, sync --check and managed-files: written, refused by name, written back
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── organization_lints.rs               # LNT-001: written, refused when missing or looser, and read by real Clippy
│   │   ├── performance_budget.rs               # PRF-001: a rise past 5 % refused unless excused, and when nothing is measured
│   │   ├── platform_portability.rs             # ci.yml: named platforms become pinned runners that the required status holds
│   │   ├── pull_request_rules.rs               # COV-002, PRL-001 and PRL-002 over a real change against a base commit
│   │   ├── quality_gates.rs                    # ci.yml: lint, documentation, coverage and analysis gates, each proven to fail
│   │   ├── quality_reports.rs                  # ci.yml: diagnostics survive failing tools without changing their verdict
│   │   ├── release_payload.rs                  # ci.yml: release build, payload, bills of materials, and the example gate
│   │   ├── release_payload_refusals.rs         # The release payload's refusals: lockfile drift, unhardened or irreproducible binaries, malformed staging
│   │   ├── repository_hygiene.rs               # ci.yml: HYG-001 to HYG-005 and shell width, each refused by name
│   │   ├── scorecard_and_required_status.rs    # ci.yml: the scorecard, the required status and mutation testing
│   │   ├── scorecard_states.rs                 # ci.yml: selection, applicability and execution reported separately
│   │   ├── source_rules.rs                     # ci.yml: SIZE, NAME, DOC, LIB, TST and WSP, each refused by name, and the limits a repository tightens
│   │   ├── supply_chain.rs                     # ci.yml: dependency policy, direct crates.io reads and the scanners
│   │   └── workspace_boundary.rs               # ci.yml: a workspace whose manifests or sources reach outside the checkout is refused before any lint
│   ├── gate/                                   # The gate and the tests as structures: layers, no import cycle, the step registry, what holds every step and refusal
│   │   ├── layer_boundaries.rs                 # This crate's own step shape, the checks door, seam unit tests and no whole-harness import
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── step_and_refusal_coverage.rs        # Every declared step is run by a contract test; every refusal the binary composes is asserted by a test
│   │   └── step_registry.rs                    # The step registry: declarations, the generated document, every body registered
│   ├── harness/                                # The one door of the tests: the repository, YAML readers, gate declarations and the fixture
│   │   ├── fixture.rs                          # One temporary checkout, one environment table, a step run against stand-ins, every command traced
│   │   ├── gate_declarations.rs                # The gate built once per test process, and what rust-gate describe declares about its steps
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── repository.rs                       # The repository root, the toolbelt, commands run to completion, temporary directories, stand-in executables, every test file
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
│   │   ├── executable_stubs.rs                 # Stand-in executables written outside the test process, so none is refused as Text file busy
│   │   ├── gate_action.rs                      # The gate action: one pin at every call site, and a commit that ships it
│   │   ├── generated_documents.rs              # Every generated table and the diagram's count are what just docs writes
│   │   ├── metadata_and_inventory.rs           # Repository files, hook, editor and release policies, the Copilot inventory
│   │   ├── mod.rs                              # The repository modules, listed and nothing else
│   │   ├── north_star.rs                       # Promised controls run, every gate names its proof, no lint silenced
│   │   ├── pinned_tool_usage.rs                # Every job installs every pinned tool it invokes before a step reads it
│   │   ├── rendered_hooks_live.rs              # CHECK_NETWORK=1: the rendered hooks in a fresh clone with only prek and rustup
│   │   ├── secret_and_advisory_scans.rs        # Gitleaks over the tree; RustSec audits under CHECK_NETWORK=1
│   │   ├── tool_updates.rs                     # Every install row is what mise locked; update-tools moves a pin everywhere at once
│   │   ├── toolbelt_and_shellcheck.rs          # Toolbelt links to the locked builds; ShellCheck over every Bash line left
│   │   ├── version_pins.rs                     # Tool versions, the toolchain pin and the speed target, one copy each
│   │   └── workflow_policy.rs                  # Permissions, timeouts, runners, trust boundaries, shell policy and the local calls
│   ├── Cargo.lock                              # Locked resolution for the test crate
│   ├── Cargo.toml                              # Isolated workflow-contract test target
│   ├── LICENSE                                 # MIT notice included in the Cargo package
│   ├── native_runtime.rs                       # Real native Windows processes, registry boundary and temporary trees, the native suite's one door
│   ├── native_windows.rs                       # Native gate units, ACLs and real Cargo examples, not Linux ELF replay
│   └── workflows.rs                            # Test crate root: one directory per what the tests prove, and the harness they share
├── .editorconfig                               # UTF-8, LF, final newlines, space indentation
├── .gitattributes                              # Text normalization, Rust-aware diff, binary images
├── .gitignore                                  # Local tools/caches, Cargo build output, Windows markers
├── .pre-commit-config.yaml                     # This repository's own hooks: the organization's set on the pinned toolbelt, and its generated tables
├── .rumdl.toml                                 # Markdown structure: lines wrap where their writer wraps them; rendered by rust-gate sync
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
├── clippy.toml                                 # The organization's thresholds and test allowances; rendered by rust-gate sync
├── deny.toml                                   # DEP-001 and the reviewed licences; rendered by rust-gate sync
├── justfile                                    # Development commands: setup and check
├── maestro-quality.toml                        # The layers this repository's crates declare, its reasoned exceptions and its words
├── mise.lock                                   # Resolved URL and checksum of every toolbelt download
├── mise.toml                                   # The toolbelt: each tool at the version CI pins
├── rust-toolchain.toml                         # The one compiler pin: the gate, the tests and the action build with it
├── rustfmt.toml                                # The 2024 formatting style; rendered by rust-gate sync
├── typos.toml                                  # The words this repository means, from maestro-quality.toml; rendered by rust-gate sync
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
least and a test function in four, and `rust-gate architecture` refuses
anything shorter.

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
