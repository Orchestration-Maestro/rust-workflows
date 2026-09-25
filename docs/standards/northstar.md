# Northstar for `rust-workflows`

> Automate the guardrails to deliver faster, with higher quality, and more
> securely.

`rust-workflows` steers by the organization's
[Northstar](https://github.com/Orchestration-Maestro/.github/blob/864d85597a833864cd8506c3925830503b3c2163/golden-rules/northstar.md):
one KPI per pillar, each with its measurement. Unmeasured is written `not
measured`, never estimated; a value read by hand carries the date it was read.

## The point

One Rust pipeline for every Orchestration-Maestro team instead of one per application: the same
gates, reproducible artifacts and a publication path that cannot skip CI.
Application code stays in consumer repositories; deployment stays in the
platform workflows.

## KPIs

| Pillar | KPI | Current | Target | Measured by |
| --- | --- | --- | --- | --- |
| Speed | `just check` wall time, tools cached | 54 s, read 2026-09-23 | 40 s or less | The SPEED line of `just check` |
| Quality | Line coverage floor on the owned fixtures | 90%, verified on every run | 90% | The example gate, `COVERAGE=90` |
| Maintainability | Undocumented items in the gate crate | 0, verified on every run | 0 | Clippy `clippy::missing_docs_in_private_items`, denied |
| Security | Silenced lints in the gate and the fixtures | 0, verified on every run | 0 | `no_lint_is_silenced_in_the_gate_or_the_fixtures` |
