# Documentation

Two audiences, kept apart on purpose.

**If you call these workflows from your own repository**, you need the first three
documents and nothing else.

| Document | Answers |
| --- | --- |
| [ci.md](ci.md) | Every `ci.yml` input, every report the run produces, and what each quality gate actually checks. |
| [steps.md](steps.md) | Every step of every workflow with what it reads, runs and writes, generated from the declarations the gate enforces. |
| [publishing.md](publishing.md) | How the two protected publishers work, what a dry run does, and what a real publication requires. |
| [platform-requirements.md](platform-requirements.md) | What the runner, the registry and the administrator must provide before any of this works. |

**If you maintain this repository**, the standards apply to you as well.

| Document | Answers |
| --- | --- |
| [standards/northstar.md](standards/northstar.md) | The motto, the four axes with the gate that holds each bar and the test that proves it, the KPIs, and what counts as evidence for a green claim. |
| [standards/engineering.md](standards/engineering.md) | The engineering rules and, for each one, the command or test that enforces it here, or an admission that it rests on judgement. |
| [standards/security.md](standards/security.md) | Every security requirement this repository can act on, each with its real status. Two are unproven, and say so. |
| [rust-gate.md](rust-gate.md) | Why every step body is one command of a binary built from the pinned commit, its invariants and its layout. |

## What these documents will not do

They will not tell you a control exists when it does not. Each standards document
separates what a machine checks from what a person is expected to notice, and
names the gaps rather than leaving a reader to assume coverage.

That distinction is the repository's own quality bar applied to its documentation:
a rule nobody checks is an intention, and a document that hides the difference is
worse than one that omits the rule.

## Related

- [CONTEXT.md](../CONTEXT.md): domain vocabulary used throughout these documents.
- [SECURITY.md](../SECURITY.md): trust model and the private reporting path.
- [CONTRIBUTING.md](../CONTRIBUTING.md): how to run the gate and open a change.
