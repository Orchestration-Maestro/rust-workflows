# Glossary

The words every repository of the organization uses for the organization's own
concepts. Each repository's `CONTEXT.md` holds the words of its own domain; this
page holds the words they share.

- `_Avoid_` names a synonym a reader should not meet for that concept. Whether a
  use means the concept depends on the sentence, so review judges it.
- `_Never_` names a word no repository uses at all, in code or prose. The gate
  refuses it everywhere but in a glossary and in records (HYG-007).

**Repository**:
A GitHub repository of the organization, with its own CI and rule map.
_Avoid_: project (when meaning the repository itself)

**Organization**:
Orchestration-Maestro on GitHub: its settings, rulesets and repositories.

**Golden rules**:
The rules every repository follows: the foundations, the named principles, the
hard mandates and the security rules.

**Gate**:
A machine check that refuses a change and gives the same answer every time.
_Avoid_: guard (when meaning a gate)

**Gate rule**:
A rule the shared CI refuses, named by its ID, such as ARC-001.

**Finding**:
One thing a gate refuses or reports, naming its rule, its file and what to do.
_Avoid_: violation

**Exception**:
An authorized, scoped and recorded departure from a rule, with its reason.
_Never_: waiver, grandfathered

**Rule map**:
A repository's `docs/standards/` pages: what holds each golden rule there, or
why it does not apply.
_Avoid_: compliance matrix

**Managed file**:
A file every repository holds as `rust-gate sync` renders it, byte for byte.
_Avoid_: template file, synced file

**Sync pull request**:
The pull request the organization's bot opens from `maestro/sync`.

**Drift**:
A difference between what a repository or the organization holds and what its
source says.

**Allowlist**:
A list of what is allowed, each entry with its reason and a check that fails
once it stops being true.
_Never_: whitelist

**Denylist**:
A list of what is refused.
_Never_: blacklist

**Placeholder**:
A stand-in value in an example, a fixture or a test.
_Never_: dummy

**Coherence check**:
A quick check that a result is plausible before it is trusted.
_Never_: sanity check
