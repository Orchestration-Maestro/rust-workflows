## Change

Describe the problem, the smallest complete fix and the affected workflow or tooling.
Link the relevant issue when one exists.

- North Star axis this change moves (Speed, Quality, Security or Maintainability), and what it costs the others:

## Release impact

This repository releases with release-please, which reads the merged commit
message. On a squash merge that message is **this pull request's title**, so the
title decides the next version. Choose the prefix that matches the change:

| Title prefix | Version effect | Use for |
| --- | --- | --- |
| `fix:` | patch | A defect in existing behaviour |
| `feat:` | minor | A new input, workflow or capability |
| `feat!:` or `BREAKING CHANGE:` in the body | major | Any change a caller must react to |
| `docs:` `test:` `chore:` `refactor:` | none | No change to what a caller sees |

- [ ] The title uses one of the prefixes above.
- [ ] A change that alters a documented contract carries `!` or `BREAKING CHANGE:`.

A caller-visible change released as a patch is worse than one released loudly:
consumers pin a major line and will pick it up without review.

## Contract and compatibility

- Public inputs, outputs or required statuses changed:
- Artifact identity, contents or publication behavior changed:
- Rust support/MSRV or dependency pins changed:
- Runner, permission, registry or credential boundaries changed:

State explicitly when each boundary is unchanged. Document migration steps for
breaking changes; do not treat a passing check as approval to change a contract.

## Verification

Record the commands run and their results. Explain any check that was not run.

- [ ] `just check` passes.
- [ ] Behavior changes include an executable regression test and its failure case.
- [ ] The Copilot repository tree and every file explanation match the final files.
- [ ] Affected README/contract docs are updated; the commit title is a conventional `feat:`/`fix:` line.

## Safety review

- [ ] Jobs still use `ubuntu-24.04`, explicit timeouts and least-privilege permissions.
- [ ] External actions and tool updates use reviewed pins and required checksums.
- [ ] Secrets, generated output and local state are excluded from the change.
- [ ] Same-revision CI, dry-run defaults and protected publication gates remain intact.
- [ ] Hosted GitHub checks are reported separately from local evidence; no unverified integration is claimed.
