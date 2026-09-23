# Contributing

Run `just check` before requesting review. Repository tooling uses Rust, Just,
Bash on Linux, and PowerShell on Windows.

## Local setup

```bash
./scripts/bootstrap.sh
```

That is the whole setup. It fetches one checksum-verified copy of mise, installs
Rust 1.98.1, then has mise install every other pinned tool, Just included, from
`mise.toml` against the checksums in `mise.lock`. About fifteen seconds on a warm
machine; it prints the next command when it finishes.

You need Linux x64, `curl`, and `rustup` already installed through your approved
platform channel. The script deliberately does not install rustup: no
`curl | sh` bootstrap is included here.

Everything on `PATH` goes through ignored `.tools/bin`: mise itself and one link
per tool into mise's store. Cargo keeps its normal rebuildable registry cache. Verification copies consumer fixtures to a temporary directory, so
packaging, SBOM generation and build output never modify their source.

Every tool is pinned to an exact version in [mise.toml](mise.toml) and verified
against the checksum recorded in [mise.lock](mise.lock) before it is unpacked;
mise refuses bytes that differ. Those two files are where a reviewer should read
the pins: a list repeated here would be one more place to go stale. A test fails
when a tool CI installs is missing from `mise.toml` or pinned to another version,
so the local gate and CI cannot drift apart. Rust itself stays with rustup.

The isolated Rust test crate in `tests/` has one direct test dependency,
`serde_json = "=1.0.151"`, and a committed lockfile. It invokes pinned **jaq's real
YAML parser**, consumes its JSON output and runs the extracted step commands,
`rust-gate` built once per test run, against controlled stand-ins. There is no hand-written YAML parser, root Cargo package,
production Rust test framework or additional language runtime. The examples remain std-only.
The separate native Windows test target uses real processes rather than Bash
stand-ins.

## Native Windows checks

The local Docker and Podman integration has been removed, including
`just check-local`, its images and the container-host bootstraps. Run `just check`
directly on Linux x64 for full acceptance, including ELF hardening and all three
example replays. The bootstrap script provisions only that Linux toolbelt.

The native Windows suite remains a separate Cargo target. It requires Windows
x64, PowerShell, a native linker, rustup, Rust 1.98.1 with rustfmt and Clippy, and
Rust 1.85.0 already provisioned through an approved platform channel. Dependencies
for the test crate and all three examples must be available before the suite's
offline Cargo checks. No Windows provisioning script is provided.

On that Windows host, run:

```text
cargo +1.98.1 test --manifest-path tests/Cargo.toml --locked --features native-windows --test native_windows
```

It refuses a non-Windows host. It runs all gate unit tests, strict gate formatting,
Clippy and rustdoc, native registry input/configuration/ACL contracts, MSRV
compilation and real debug/release tests and Clippy for every example. It also
formats the test crate and lints the native test target. The Linux Bash-stub
workflow contracts remain Linux checks, not claimed Windows execution. No PE
equivalent of ELF hardening is invented.

The native suite has not been validated on a Windows host here. Native PowerShell
ACL probes and cross-compilation are not a native Rust suite result. These checks
do not introduce a Windows hosted runner contract or establish GitHub, advisory
database or registry readiness. No native macOS suite is provided.

## One complete gate

`just check` verifies tool presence; formats, lints, tests and documents the
gate crate and the isolated Rust test crate; runs actionlint, zizmor,
`yamlfmt -lint` and `taplo fmt --check`. The contract tests hold the rest of the repository's own
hygiene: every toolbelt link points at the locked build, every line of Bash
left (the Just recipes, `bootstrap.sh`, the gate action's build step) passes
ShellCheck, and Gitleaks scans the tree. It then runs real redacted source
scanning and each fixture's formatting, Clippy, unit/integration/doc/release tests,
80% line-coverage gate, release build, packaging, SBOM validation and the actual
workflow artifact-staging and selected-package verification commands.

`CHECK_NETWORK=1` also fetches one isolated RustSec database snapshot and
audits the development and all consumer lockfiles, failing on findings or lookup/
parser errors. Without it the advisory check is explicitly **NOT RUN**;
GitHub CI always requires it. Other Cargo commands may still access the network.
Missing prerequisites or incorrect pinned versions fail instead of being skipped.

For focused checks (full acceptance still requires `just check`):

```bash
cargo +1.98.1 test --manifest-path tests/Cargo.toml --locked
actionlint -config-file .github/actionlint.yml .github/workflows/*.yml
zizmor --offline --persona=pedantic --config .github/zizmor.yml .github/
yamlfmt -lint
taplo fmt --check
```

Give actionlint explicit paths because this directory initially has no Git
metadata. YAML formatting is defined in `.yamlfmt.yml`; run `yamlfmt` to fix it.

## The gate

Every step body is a subcommand of `rust-gate`, a standard-library-only crate
in `gate/` that the `gate` action builds from the pinned commit: `ci.yml` steps
are `rust-gate <id>`, the other workflows `rust-gate <workflow> <step>`, and
the two steps several workflows share are `rust-gate install-tools` and
`rust-gate verify-payload`. [docs/rust-gate.md](docs/rust-gate.md) holds the
invariants and the layout. `just check` formats, lints, tests and documents
that crate like the test crate. A change to a step is a change to its module
under `gate/src/steps/`, covered by the contract test that runs the command
against a fixture; what several steps share lives under `gate/src/checks/`.
Imports flow one way, steps to checks to runner, and no crate holds an import
cycle, the tests included: `every_crate_has_an_acyclic_import_graph` names the
cycle it finds.

Five more tests hold the crate and its proof to the same bar. The binary never
panics: Clippy's `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`,
`unimplemented` and `dbg_macro` lints are denied in its manifest, and
`the_binary_refuses_every_way_to_panic` keeps them there; a refusal is a
message and an exit status, never a stack trace. Every step it declares is
run by a contract test (`a_contract_test_runs_every_registered_step`), and every refusal it
composes is asserted by a test, in its own words
(`a_test_asserts_every_refusal_the_gate_can_print`); a message that relays an
operating system error is the system's wording and is left out. Every module
under `gate/src/checks/`, and every internal seam beside a step, that defines
a function carries unit tests
(`every_check_and_step_seam_with_a_function_has_unit_tests`), and no crate holds
an import cycle (`every_crate_has_an_acyclic_import_graph`).

The tests are laid out by what they prove: `tests/ci/`, `tests/publishers/`,
`tests/nightly/`, `tests/gate/` and `tests/repository/`, with the harness they
share under `tests/harness/`. A module names what it proves in two words at
least (`every_test_module_names_what_it_proves_in_two_words_at_least`), a test
function in four (`every_test_function_names_what_it_proves_in_four_words_at_least`),
and never behind a `test_` prefix or a `_works`, `_ok` or `_test` suffix
(`no_test_function_is_named_by_a_test_prefix_or_a_works_ok_or_test_suffix`).
A step declares its inputs, tools and reports in its `STEPS` constant and the
gate refuses anything else at run time; `just docs` regenerates
[docs/steps.md](docs/steps.md) from those declarations, and `just check`
refuses a stale copy. A contract test never reads the gate's source: it runs
the step in a fixture and reads what it wrote, what it declares and the trace
of every command it ran.

## The gate action and its pin

`.github/actions/gate` builds `rust-gate` from this repository at the commit
the workflows pin, as the first step of every job that runs a step body. The
workflows call it as `Orchestration-Maestro/rust-workflows/.github/actions/gate@<sha>`,
and every call site pins the same commit of this repository; a test refuses
two different pins, and a pinned commit that does not contain the action file.

A gate change ships in two pull requests, because the default branch takes only
squash merges and a pin must name a commit on it:

1. Merge the gate change. Its pins still name the previous gate commit, so its
   own CI runs the previous gate.
2. Open a second pull request that replaces every pin with the first one's
   squash commit, then run `just check`: the pin test requires that commit to
   contain the action. Never invent a SHA or substitute a branch or tag.

Consumers pin a release, so cut one after the second merge: its `ci.yml` is the
one that calls the new gate.

## Strict coding standard

1. Keep workflows thin and directly discoverable. Production runs in the consumer
   checkout: no assumed helper files, floating self-checkout, generalized workflow
   engine or arbitrary shell-command inputs. Use Just for repository automation.
   Step logic is a `rust-gate` subcommand, one per step, in the
   standard-library-only crate under `gate/`: a reusable workflow cannot read
   this repository's files at run time, so the `gate` action builds the binary
   from the pinned commit first, and the contract tests execute the exact
   command. A `run:` body is that one command and nothing else; inputs reach
   it through `env`.
2. Use rustfmt, Rust 2024, declared MSRV, forbidden unsafe code and Clippy
   `-D warnings`, with the size limits the root `clippy.toml` declares
   (cognitive complexity 15, 100 lines and 5 parameters per function), at
   most 500 lines of code per Rust file, doc comments not counted, with the
   files over 300 reported rather than refused, and 100
   columns per line of Rust and shell. Use standard-library examples with behavior-asserting tests,
   including binary process output. Commit generated Cargo lockfiles. Do not
   enable all features implicitly.
3. Pin external actions to verified 40-character SHAs with version comments;
   this repository's `gate` action is pinned the same way, as "The gate action
   and its pin" describes. Pin tools/download checksums, keep the Just recipes and the action's
   build step under Bash `set -euo pipefail`, and pass expressions through
   `env`. Validate paths/names/registry/ref boundaries;
   never use `eval`, unchecked downloads, scanner error suppression or broad
   secrets inheritance. Every owned executable job needs an explicit timeout
   and least-privilege permissions.
4. Write failing contract/extracted-command regression tests before nontrivial
   changes. Preserve real parser and command execution coverage at trust
   boundaries; string checks alone are insufficient. Run the complete Just gate.
5. Treat public inputs/outputs, artifact shape and required statuses as an API.
   Require independent review; retain compatibility or document an intentional
   breaking change. Describe the change in a conventional commit (`feat:`, `fix:`,
   `refactor:`); release-please writes `CHANGELOG.md` from those titles; the commit-msg hook
   prek installs refuses a first line that is not such a header and any line
   over 80 columns. Validate
   pin-update PRs against the exact upstream workflow contracts.

Examples declare MSRV 1.85, while their normal gate pins 1.98.1. `ci.yml`
accepts any exact stable version from the MSRV up; the tested set is 1.85.0,
1.95.0, 1.96.1, 1.97.1 and 1.98.1, and `ci-internal.yml` runs every example
on every one of them in CI, with no case silently skipped. When the tested set
changes, keep the MSRV floor, the GitHub matrix, the examples, the tool pins,
the tests and the README consistent; the README describes the procedure.

Cargo packaging warns about
absent remote documentation/repository URLs: do not invent URLs to suppress it.

Administrators must set an actual CODEOWNER and keep the organization's branch,
tag and environment protections in place.

Releases run through `.github/workflows/release-please.yml`, which stays skipped
until an owner sets it up once:

1. Create a GitHub App owned by the organization, with no webhook and the
   repository permissions Contents, Issues, Pull requests and Workflows set to
   read and write. Install it on the organization's repositories.
2. Store its client ID as the organization variable `RELEASE_APP_CLIENT_ID` and
   a private key as the organization secret `RELEASE_APP_PRIVATE_KEY`. Store both
   again, under the same names, as organization Dependabot secrets: a run
   Dependabot started reads no other secret.

The workflow then keeps a release pull request open from the conventional
titles merged to `main`; merging it tags `vMAJOR.MINOR.PATCH` and creates the
GitHub Release. An App token, not `GITHUB_TOKEN`, opens that pull request: the
organization forbids `GITHUB_TOKEN` from opening one, and a pull request it
opened would trigger none of the checks a merge requires.

`.github/workflows/dependabot-auto-merge.yml` uses the same App to queue a
squash merge of each Dependabot pull request, which groups an ecosystem's
patch and minor updates of the week; the required checks still
decide whether it merges. A major update, or one whose type Dependabot did not
record, waits for a person. The App's Workflows permission is what lets an
action update, which edits workflow files, merge.

Dependabot does not read the tools the workflows download. `mise.lock` is their
one record, and a test holds every workflow install row to the asset it locked.
`just update-tools` moves every pin to its latest release: `mise.toml`, then
`mise.lock` through mise, then each install row with the digest of the bytes the
new asset serves, and the Codecov CLI version. `tool-updates.yml` runs it every
Monday and opens, or refreshes, one pull request on the same App; a person
merges it, because a new scanner or test runner can change what the gate
refuses. A move marked `(major)` needs its release notes read first.
