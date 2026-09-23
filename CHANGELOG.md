# Changelog

## [1.2.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v1.1.0...v1.2.0) (2026-09-23)


### Features

* test on macOS, Windows and Linux arm64 when asked ([#16](https://github.com/Orchestration-Maestro/rust-workflows/issues/16)) ([6290dc0](https://github.com/Orchestration-Maestro/rust-workflows/commit/6290dc03b70dc67ba024eeef1c23b37e9d927182))

## [1.1.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v1.0.1...v1.1.0) (2026-09-23)


### Features

* check public API compatibility on pull requests ([#14](https://github.com/Orchestration-Maestro/rust-workflows/issues/14)) ([a064dae](https://github.com/Orchestration-Maestro/rust-workflows/commit/a064dae4efbc2826baf0fb5513cd07eaf1361911))
* turn the API compatibility gate on by default ([#15](https://github.com/Orchestration-Maestro/rust-workflows/issues/15)) ([9deeaea](https://github.com/Orchestration-Maestro/rust-workflows/commit/9deeaead8f795fed0c81d4f0d312b380b23652bc))
* upload coverage and test results to Codecov ([#12](https://github.com/Orchestration-Maestro/rust-workflows/issues/12)) ([7528afc](https://github.com/Orchestration-Maestro/rust-workflows/commit/7528afc09e366456b442126803d4619e3d951281))

## [1.0.1](https://github.com/Orchestration-Maestro/rust-workflows/compare/v1.0.0...v1.0.1) (2026-09-23)


### Bug Fixes

* retry a dropped connection before a tool download fails ([#7](https://github.com/Orchestration-Maestro/rust-workflows/issues/7)) ([beda4ef](https://github.com/Orchestration-Maestro/rust-workflows/commit/beda4ef938e575971c99c93d683e7b884402462b))

## 1.0.0 (2026-09-23)

### Features

* reusable, security-gated Rust workflows ([4156864](https://github.com/Orchestration-Maestro/rust-workflows/commit/4156864bd3b60dce57995ce4039d0cd95d5c7dd9))
* turn SARIF on by default and upload it to code scanning ([#4](https://github.com/Orchestration-Maestro/rust-workflows/issues/4)) ([701b4dc](https://github.com/Orchestration-Maestro/rust-workflows/commit/701b4dca765deb4d4bec2b5cda04051eba219118))

### Bug Fixes

* package workspaces on Cargo 1.90 below that compiler ([#2](https://github.com/Orchestration-Maestro/rust-workflows/issues/2)) ([eabfa54](https://github.com/Orchestration-Maestro/rust-workflows/commit/eabfa547d40f60f6ee7cd040bc8ffbbf077aacbc))
