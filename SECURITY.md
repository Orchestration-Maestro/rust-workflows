# Security

## Reporting a vulnerability

Report privately through the repository's GitHub private vulnerability reporting
feature **once administrators enable it**. If it is unavailable, use your
organization's approved private security channel. Never open a public issue
containing credentials, exploit details or confidential logs. This repository has
no dedicated security mailbox or team.

Redact secret values from reports. Revoke exposed credentials through the owning
platform; deleting a report is not remediation. Scan findings and scanner failures
are blocking.

## Trust model

Live publication requires reviewed source, protected release tags, protected
environments and explicit administrative opt-in. Only reviewed commit SHAs are
suitable workflow dependencies until actual releases and a support policy exist.

This repository does not claim a published supported release or complete
GitHub/registry integration. See [platform requirements](docs/platform-requirements.md)
for the remaining boundaries, and [docs/standards/security.md](docs/standards/security.md)
with [docs/standards/controls.md](docs/standards/controls.md) for the full control
status, including the requirements that are **not** met.

The `release` environment must have real required reviewers and only the `v*`
tag deployment policy. Live preflight and write steps verify that configuration
through GitHub's API; no environment is created as approval. GitHub Team offers
these reviewers because the organization's repositories are public. Crates
publish explicitly to public crates.io; review package contents for confidential
material before approving.

## Verifying release assets

These steps were run, as written, against the assets of
[release-canary v0.1.0](https://github.com/Orchestration-Maestro/release-canary/releases/tag/v0.1.0),
the organization's canary consumer, on 2026-09-23: every step passed. This
repository's own releases are source only.

A release run attaches `payload.tar.gz`, `provenance.json`, `SHA256SUMS` and, with
`publish-evidence.yml`, `evidence.tar.gz`. The payload contains the built binaries,
each crate's `.crate` package and the bills of materials.

### 1. Checksums

```bash
sha256sum --check --strict SHA256SUMS
```

This proves the archive was not altered after it was written. It proves nothing
about who wrote it. The next step does that.

### 2. Build provenance

Requires the [GitHub CLI](https://cli.github.com/) and network access to the
GitHub attestation API.

```bash
gh attestation verify payload.tar.gz \
  --repo <owner>/<repo> \
  --signer-workflow Orchestration-Maestro/rust-workflows/.github/workflows/attest-binaries.yml
```

Check the reported subject digest against the digest in `SHA256SUMS`. A successful
exit code alone is not verification: it must be the right subject, signed by the
expected reusable workflow. `--repo` identifies the consumer repository that
holds the attestation; `--signer-workflow` identifies this repository, not the
consumer's workflow.

A missing or inaccessible attestation is not verified provenance. The default
`on-unavailable: skip` tolerates only Enterprise Server identified by a successful
GitHub metadata response. Missing Cloud entitlement, OIDC, API, signing and
verification errors always block; none is inferred to mean platform unavailability.
Use `on-unavailable: fail` and require `attested == 'true'` when provenance is
mandatory. Permission validation may reject a call before these checks can run.

The gate action rebuilds in a fresh directory for every invocation; it never
restores executable bytes or Cargo build fingerprints from a consumer cache.
This closes that cache boundary, not arbitrary compromise of a persistent runner:
isolated, ephemeral runners remain required. Opt-in command traces redact explicit
environment values by default except reviewed non-sensitive settings. Command
arguments and subprocess output are not secret-safe logging channels.

### 3. Bills of materials

They are inside the payload, next to the binaries:

```bash
tar -xzf payload.tar.gz
ls ./*.cdx.json ./*.spdx.json
```

- `payload.cdx.json` is CycloneDX 1.5, hierarchical: one root component with each
  workspace member nested under it.
- `<binary>.spdx.json` is SPDX 2.3, one per released binary, written natively by
  cargo-sbom: converting the CycloneDX document instead loses packages and drops
  the relationship graph.
- `<binary>.cdx.json` is the same binary's CycloneDX document.

Use whichever your tooling reads.

### 4. Dependencies embedded in the binary

Each released binary carries its resolved dependency list in a `.dep-v0` ELF
section, so it can be audited without the documents above:

```bash
readelf -S <binary> | grep dep-v0      # the section is present
cargo audit bin <binary>               # read it and check it against advisories
```

This is the check that still works after a binary has been copied somewhere its
SBOM did not follow.

### What you need

`sha256sum`, `tar` and `readelf` (binutils) are enough for steps 1, 3 and 4.
Step 2 also needs `gh` and network access; step 4's advisory check needs
`cargo-audit` and the RustSec database.
