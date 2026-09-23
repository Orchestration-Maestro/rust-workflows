set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["cmd.exe", "/d", "/s", "/c"]
set export := true

local_bin := justfile_directory() / ".tools/bin"
host_path := env_var("PATH")
export PATH := if os() == "linux" { local_bin + ":" + host_path } else { host_path }

# The North Star speed target: `just check` prints its wall time against it.

export SPEED_TARGET_SECONDS := "40"

# List every recipe and what it does; this is what `just` alone prints.
help:
    @just --list --unsorted

# Install the pinned toolbelt and link it into ignored .tools/bin (Linux x64).
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
      echo 'Pinned tooling supports Linux x64 only' >&2; exit 1;
    }
    # Every tool, its version and the checksum of every download live in
    # mise.toml and mise.lock. mise installs them into its own store and refuses
    # bytes that differ from the lock; mise itself is the one download that
    # scripts/bootstrap.sh verifies by hand, into .tools/bin.
    mise trust mise.toml
    mise install
    # Links keep the gate, the commit hooks and the documentation on the one
    # PATH entry they already use, wherever mise keeps its store.
    mkdir -p .tools/bin
    find .tools/bin -mindepth 1 ! -name mise -delete
    while IFS= read -r directory; do
      for file in "$directory"/*; do
        if [[ -f "$file" && -x "$file" ]]; then
          ln -s -- "$file" ".tools/bin/$(basename -- "$file")"
        fi
      done
    done < <(mise bin-paths)
    # The commit hooks only run if the Git hook is installed; provisioning the
    # tool without wiring it in would leave every contributor to remember this.
    prek install

# Complete local gate; CHECK_NETWORK=1 also requires a live RustSec advisory lookup.
check:
    #!/usr/bin/env bash
    set -euo pipefail
    started=$(date +%s)
    RUSTUP_TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
    export RUSTUP_TOOLCHAIN
    for tool in mise just actionlint zizmor yamlfmt taplo shellcheck prek cargo rustup \
      gitleaks typos jaq readelf similarity-rs cargo-hack cargo-nextest; do
      command -v "$tool" >/dev/null ||
        { echo "Missing $tool; run scripts/bootstrap.sh" >&2; exit 1; }
    done
    just --unstable --fmt --check
    for manifest in gate tests; do
      cargo fmt --manifest-path "$manifest/Cargo.toml" --all --check
      cargo clippy --manifest-path "$manifest/Cargo.toml" --all-targets --locked -- -D warnings
    done
    cargo test --manifest-path gate/Cargo.toml --locked --offline
    RUSTDOCFLAGS='-D warnings -D missing_docs' cargo doc --manifest-path gate/Cargo.toml \
      --no-deps --locked --offline --document-private-items
    cargo test --manifest-path tests/Cargo.toml --locked
    RUSTDOCFLAGS='-D warnings -D missing_docs' cargo doc --manifest-path tests/Cargo.toml \
      --no-deps --locked
    cargo deny --offline --config deny.toml --manifest-path tests/Cargo.toml \
      check licenses bans sources
    actionlint -config-file .github/actionlint.yml .github/workflows/*.yml
    zizmor --offline --persona=pedantic --no-progress --config .github/zizmor.yml .github/
    yamlfmt -no_global_conf -lint
    taplo fmt --check
    # Spelling over the working tree, where the hook below reads the index:
    # typos.toml names the words this repository means.
    typos
    # The same hooks a commit runs, over every tracked file. prek reads the
    # index, so before the first commit it has no file and every hook skips;
    # a skip that looks like a pass is the one thing this gate must not do.
    [[ "$(git ls-files | wc -l)" -gt 0 ]] ||
      echo 'NOT RUN: the commit hooks; git tracks no file yet, so every hook skips'
    prek run --all-files
    # The example gate is a Rust test that replays ci.yml's own step bodies,
    # read by id, against every owned fixture with the pinned toolbelt, so the
    # local gate cannot drift from CI. Plain `cargo test` skips it; this is its
    # one caller.
    cargo test --manifest-path tests/Cargo.toml --locked -- --ignored --nocapture example_gate
    # The toolbelt links, ShellCheck over every Bash line left, Gitleaks over
    # the tree and, under CHECK_NETWORK=1, the RustSec audits are contract tests.
    # The size report: what is growing, named without refusing it.
    cargo test --manifest-path tests/Cargo.toml --locked --offline \
      files_over_three_hundred_lines_are_reported -- --nocapture | grep '^REPORT:'
    if [[ "${CHECK_NETWORK:-0}" != 1 ]]; then
      echo 'NOT RUN: live advisory database check; run CHECK_NETWORK=1 just check'
    fi
    elapsed=$(( $(date +%s) - started ))
    echo "SPEED: $elapsed s wall, target $SPEED_TARGET_SECONDS s"
    echo 'PASS: local gate. GitHub/registry integration is not exercised locally.'

# Regenerate the canonical Linux document about the gate's steps.
[linux]
docs:
    cargo run --manifest-path gate/Cargo.toml --locked --offline --quiet -- describe \
      > docs/steps.md
