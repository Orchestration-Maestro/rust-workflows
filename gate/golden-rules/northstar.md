# Northstar

> Automate the guardrails to deliver faster, with higher quality, and more
> securely.

Speed, quality, maintainability and security are not a trade-off. Automation is
what lets one repository have all four at once. A Northstar is never reached: it
is the direction every repository in the organization steers by, and the
[engineering](engineering.md) and [security](security.md) rules are how it
holds that course.

## The point

Each repository states the outcome it exists for: whose problem it solves, and
what changes for them when it works.

## Four pillars

One KPI per pillar, not a list. Each pillar names what it protects; each
repository chooses its own KPI, never copies another's.

| Pillar | What it protects | Where a KPI usually comes from |
| --- | --- | --- |
| Speed | The edit-run loop and the path from commit to release | Fast-tier duration, lead time, cost per test |
| Quality | It does what it claims, and keeps doing it | Coverage of the behaviour that matters, surviving mutants, escaped defects |
| Maintainability | The next reader can change it safely | Warning-free gates, documented items, module and complexity ceilings |
| Security | Nothing reaches a machine without passing the gates | Open findings, secrets in history, time to patch, verified release assets |

## KPIs

A KPI has a current value, a target, and the measurement that produces it,
automated wherever possible. A KPI nobody measures is decoration. Every
repository keeps this table:

| Pillar | KPI | Current | Target | Measured by |
| --- | --- | --- | --- | --- |
| Speed | One KPI | not measured | Its target | The CI job, script or command |
| Quality | One KPI | not measured | Its target | The test or coverage gate |
| Maintainability | One KPI | not measured | Its target | The lint or documentation gate |
| Security | One KPI | not measured | Its target | The scanner or audit in CI |

- **Unmeasured is written `not measured`**, never estimated. The first completed
  run sets the baseline.
- **A value a gate verifies on every run is written plain:** it cannot drift
  without turning something red. A value read by hand carries the date it was
  read, because nothing keeps it current afterwards.
- **Targets come from the first baseline**, not from this file. A target that is
  always green is too easy; one that is always red is fantasy. Tighten the
  first, fix or drop the second.

## How we hold ourselves to it

- **Shift left, automated.** If a rule matters, it is a gate in the hooks and
  CI. If it cannot be automated, it is written down as a check someone runs.
- **Guardrails over gatekeepers.** A linter rule, a schema or a policy check
  beats a review comment that will be forgotten.
- **A gate that cannot fail is not a gate.** Prove each one by injecting the
  fault it exists to catch and watching it fail.
- **A decision is recorded where it is enforced:** an architecture decision
  record beside the thing it governs.
- **Plans name the pillar they move.** Work that degrades a pillar without a
  stated trade-off is flagged, not merged quietly.

## What this is not

Not a claim that any target is met. Direction lives in each repository's
roadmap; its Northstar records what is measured and how.
