# Platform requirements

Enable unprivileged CI and publisher dry-runs first. The provider repository is
public at `Orchestration-Maestro/rust-workflows`, like every repository of the
organization, which a ruleset keeps public. Public crates.io publication is a
separate, explicitly selected operation.

## Local native prerequisites

The hosted contract is GitHub-hosted Ubuntu 24.04, Linux x64. Local acceptance
runs directly on Linux x64 through `just check`, with the pinned toolbelt from
`scripts/bootstrap.sh`. Docker and Podman are not part of local verification.

The separate native Windows target requires an already-provisioned Windows x64
host, PowerShell, rustup, a native linker, Rust 1.98.1 with rustfmt and Clippy,
Rust 1.85.0 and locked dependencies. See the
[contributor procedure](../CONTRIBUTING.md#native-windows-checks). Native execution
remains unverified. Linux ELF policy stays in Linux acceptance; no Windows hosted
runner or native macOS suite is claimed.

## Administrator-owned setup

| Boundary | Required setup |
| --- | --- |
| GitHub provider | Allow the pinned upstream actions in the organization's allowed-actions policy, `codecov/codecov-action` included. |
| Codecov | Install the Codecov GitHub App on the organization. Uploads log in through OIDC; no Codecov token is stored. |
| Release and update bot | An organization-owned GitHub App with Contents, Issues, Pull requests and Workflows write, its client ID and private key stored for Actions and again as Dependabot secrets; see [CONTRIBUTING](../CONTRIBUTING.md). |
| Runners | GitHub-hosted `ubuntu-24.04`, with the image's Bash, Git, curl, tar, SHA256, rustup, GitHub CLI and native linker. No runner override or self-hosted runner group is used. |
| Supply-chain reads | Direct upstream Rust distributions, sparse crates.io, RustSec advisories and checksum-pinned official GitHub tool release assets. No Cargo read credential is used. |
| Code scanning | Enable CodeQL default setup, which supports Rust, through an organization security configuration. Add a ruleset requiring code scanning results if alerts must block merges. Private and internal repositories need GitHub Code Security. |
| Repository governance | Confirm CODEOWNERS access, require review and required status checks, hold every external contributor's workflow run for approval, and protect release tags with rulesets. |
| Live publishing | A pre-existing `release` environment with required reviewers and exactly one custom deployment policy: type `tag`, name `v*`. Create and review the GitHub Release separately before uploading assets. For crates.io only, configure the existing `CARGO_REGISTRY_TOKEN` secret. |

Every executable job has an explicit timeout and least-privilege permissions.
CI entry jobs refuse `pull_request_target`; publisher entry jobs also refuse fork
pull requests before allocation.
In-file guards are defense in depth: PRs can change YAML, so repository controls
and review remain necessary. No PR job receives a publishing token or restores a
privileged executable cache.

CI and Cargo publication jobs create private job-local Cargo state. Dependencies
come directly from crates.io; pinned tools come from official GitHub Releases.
Rustup uses upstream distribution defaults on the hosted image. These are three
separate integrations, not interchangeable registry endpoints. Consumer source
policy remains enforced; `--locked` is not an offline guarantee.

## Live protection availability

GitHub documents that required environment reviewers on Free, Pro and Team are
available only for public repositories. The organization's repositories are all
public, so GitHub Team meets this live publication contract; a private one would
not. The workflows query actual
protection rules and fail closed; an environment named `release` alone is not
approval. This does not block unprivileged CI or either publisher dry-run.

The preflight and the protected write step both read the environment and its
branch policies. Missing reviewers, broader policies, API errors, insufficient
permissions and malformed JSON fail before writing. GitHub enforces approval at
job entry. Administrators must maintain those protections during the run; the
workflow does not grant reviewers, create environments or change the plan.

Official references checked for this adaptation:

- [Environment availability](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments).
- [Get an environment](https://docs.github.com/en/rest/deployments/environments#get-an-environment): repository `Actions: read`.
- [List deployment branch policies](https://docs.github.com/en/rest/deployments/branch-policies#list-deployment-branch-policies): repository `Actions: read`.

The job's automatic `GITHUB_TOKEN` supplies those reads via `GH_TOKEN`. No PAT,
new approval variable or OIDC exchange is introduced. Binary and evidence upload
jobs additionally need `contents: write`; crate publication needs `contents: read`
and its scoped crates.io token. Callers must grant the static ceiling even for a
skipped nested live job; executed dry-run jobs remain read-only.

## Optional integrations

Artifact attestations are available on the organization's public repositories;
a private one would need GitHub Enterprise Cloud. Missing entitlement is never
treated as a successful or skipped signature. See
[publishing.md](publishing.md#provenance-attestation).

Rustfmt, Clippy, tests, coverage, RustSec and Gitleaks remain native enforced gates.
The licence gate uses the consumer policy or the existing `LICENSE_ALLOWLIST`
variable when configured; no organization licence policy is invented here.

## Verification boundary

Local gates exercise Cargo, scanners, workflow lint and actual extracted step
commands. They do not exercise GitHub's scheduler, private reusable access,
environment reviewers, release uploads or crates.io authentication. A hosted run
must independently confirm action resolution, source SHA, all fifteen matrix
cases, both publisher dry-runs, artifacts and final required statuses.

No live crate, binary or evidence publication is authorized by local acceptance.
No hosted or native Windows result is claimed until it has actually run.
