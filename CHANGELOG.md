# Changelog

## [2.3.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v2.2.0...v2.3.0) (2026-09-25)


### Features

* write each repository's copilot guide with rust-gate guide ([#43](https://github.com/Orchestration-Maestro/rust-workflows/issues/43)) ([c3ff7af](https://github.com/Orchestration-Maestro/rust-workflows/commit/c3ff7af08637e862a69e5aa10623ced5515fe390))


### Bug Fixes

* carry the golden rules of .github 864d855 ([#46](https://github.com/Orchestration-Maestro/rust-workflows/issues/46)) ([843ecca](https://github.com/Orchestration-Maestro/rust-workflows/commit/843ecca383ac3b7f585e6516910cee96619a4219))
* say in each guide that the golden rules come first ([#47](https://github.com/Orchestration-Maestro/rust-workflows/issues/47)) ([d11e54c](https://github.com/Orchestration-Maestro/rust-workflows/commit/d11e54c9b99215f3721f0e49dc3c3fb71fc988f1))

## [2.2.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v2.1.0...v2.2.0) (2026-09-25)


### Features

* keep the gate's rules in one list it prints ([#44](https://github.com/Orchestration-Maestro/rust-workflows/issues/44)) ([ca71bbd](https://github.com/Orchestration-Maestro/rust-workflows/commit/ca71bbdda478dd38e8f75168a371fb0039bc54aa))
* write each repository's rule map with rust-gate rules ([#41](https://github.com/Orchestration-Maestro/rust-workflows/issues/41)) ([377cbaa](https://github.com/Orchestration-Maestro/rust-workflows/commit/377cbaa5b8f854032ee25d4f28ff48594daa3681))

## [2.1.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v2.0.1...v2.1.0) (2026-09-24)


### Features

* move every rust-workflows pin with the caller ([#39](https://github.com/Orchestration-Maestro/rust-workflows/issues/39)) ([bd7b23f](https://github.com/Orchestration-Maestro/rust-workflows/commit/bd7b23fac76cee34798e2e0eb9cce44a01a8b32f))

## [2.0.1](https://github.com/Orchestration-Maestro/rust-workflows/compare/v2.0.0...v2.0.1) (2026-09-24)


### Bug Fixes

* give the hooks step the mise bootstrap.sh pins ([#38](https://github.com/Orchestration-Maestro/rust-workflows/issues/38)) ([a88ec51](https://github.com/Orchestration-Maestro/rust-workflows/commit/a88ec510980675b6b9f315765d45911bcb2d70d0))
* skip dependabot auto-merge on runs a person started ([#36](https://github.com/Orchestration-Maestro/rust-workflows/issues/36)) ([57245f6](https://github.com/Orchestration-Maestro/rust-workflows/commit/57245f6e497fd724120fdf1be7f483e037bd21bb))

## [2.0.0](https://github.com/Orchestration-Maestro/rust-workflows/compare/v1.2.1...v2.0.0) (2026-09-24)


### ⚠ BREAKING CHANGES

* the quality-preview input is removed; a caller passing it fails to start.
* finish the organization quality gate behind quality-preview ([#32](https://github.com/Orchestration-Maestro/rust-workflows/issues/32))

### Features

* finish the organization quality gate behind quality-preview ([#32](https://github.com/Orchestration-Maestro/rust-workflows/issues/32)) ([deb5670](https://github.com/Orchestration-Maestro/rust-workflows/commit/deb56707c850e204c96f6a9af093769cdda9e1e5))
* refuse module structure faults with rust-gate architecture ([#30](https://github.com/Orchestration-Maestro/rust-workflows/issues/30)) ([cb65e7c](https://github.com/Orchestration-Maestro/rust-workflows/commit/cb65e7c228b5c6b5c1b58d26e271a1fee0567c7e))
* run every organization rule on every call ([#33](https://github.com/Orchestration-Maestro/rust-workflows/issues/33)) ([207cadb](https://github.com/Orchestration-Maestro/rust-workflows/commit/207cadbb6ee4b129cc8cc7b3e33e5af7bca424af))
* score every rule family and leave the home's hooks to just check ([#35](https://github.com/Orchestration-Maestro/rust-workflows/issues/35)) ([12f8b53](https://github.com/Orchestration-Maestro/rust-workflows/commit/12f8b536457153e7e1285ccee2d77eb3c804b3a6))


### Bug Fixes

* ship the gate's build inputs and authenticate hook installs ([#34](https://github.com/Orchestration-Maestro/rust-workflows/issues/34)) ([cc14131](https://github.com/Orchestration-Maestro/rust-workflows/commit/cc141317193572396c11bb1a130d88634a590314))

## [1.2.1](https://github.com/Orchestration-Maestro/rust-workflows/compare/v1.2.0...v1.2.1) (2026-09-23)


### Bug Fixes

* compare only the libraries the base branch already has ([#25](https://github.com/Orchestration-Maestro/rust-workflows/issues/25)) ([75807b0](https://github.com/Orchestration-Maestro/rust-workflows/commit/75807b0f9d6fc3f3389f43d3890342b23943b3cb))
* keep mise's progress out of the tool moves ([#20](https://github.com/Orchestration-Maestro/rust-workflows/issues/20)) ([8e22793](https://github.com/Orchestration-Maestro/rust-workflows/commit/8e22793345cb248bc2dd1d2408975c26cc148a3d))

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
