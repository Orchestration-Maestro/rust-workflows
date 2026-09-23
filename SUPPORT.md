# Support

## Where your problem goes

| Symptom | Owner |
| --- | --- |
| A gate fails and you disagree with it | Repository maintainers, through an issue |
| A gate passes and should not have | Repository maintainers, through an issue |
| `ubuntu-24.04` never picks up the job | Repository administrators: Actions access and hosted runner availability |
| Release upload fails | Repository administrators: existing Release, matching protected tag, asset conflicts and scoped job permissions |
| Live approval is refused | Repository administrators: actual reviewers and only the `v*` tag policy on `release` |
| CodeQL code scanning is missing or not required | Platform administrators: CodeQL default setup in the organization security configuration, and the code scanning ruleset |
| An attestation step reports the capability is unavailable | Platform administrators: artifact attestations must be enabled for the repository |
| A suspected vulnerability, or an exposed credential | [SECURITY.md](SECURITY.md), privately, never in a public issue |

No ownership team, support address or service-level commitment is assumed here.

## What to include

Run the gate and report what it printed:

```bash
just check
```

Then give the first failing command, its exit code, the workflow revision you
called, and a minimal Cargo project that reproduces it. The reports artifact from
the failing run carries the machine-readable results; attach it rather than
pasting a screenshot.

Redact tokens and unredacted scan findings before sharing anything.

## Before opening an issue

Two failures look like defects and are not:

- The licence report says "licences NOT APPLIED". The default source and
  version policy still ran; no licence list applied because no `deny.toml` is
  committed and administrators have not set the `LICENSE_ALLOWLIST`
  organization variable. Commit a policy, or ask for the variable.
- The scorecard shows fewer active gates than available. The optional gates
  stay off until your caller turns them on, so upgrading never fails a consumer
  who did not ask for them.

[docs/ci.md](docs/ci.md) documents every input, every report and what each gate
checks.
