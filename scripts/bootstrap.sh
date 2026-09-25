#!/usr/bin/env bash
# One command to get from a fresh clone to a machine that can run the gate.
#
# mise installs every pinned tool listed in mise.toml, Just included, and refuses
# any download whose checksum differs from mise.lock. Something has to obtain
# mise before that can run. On Linux, fetch its checksum-verified release, then
# hand over to `just setup` when the justfile has one, or link the toolbelt and
# install the commit hooks here when it does not.
#
# rust-gate sync writes this file, mise.toml and mise.lock into every repository
# of the organization from rust-workflows, so the tools a contributor runs are
# the ones CI runs, at the same versions. Move a pin there, never here.
#
# It does not install rustup. Provision that through your approved platform
# channel; no `curl | sh` bootstrap is included here.
set -euo pipefail

readonly MISE_VERSION=v2026.9.9
readonly MISE_SHA256=e4767e4854af5daeff2191b2bbdc94f834742a23efad591dbd33187861d41604

fail() {
  echo "$1" >&2
  exit 1
}

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

[[ "$(uname -s)/$(uname -m)" == Linux/x86_64 ]] ||
  fail 'Pinned tooling requires Linux x64.'
# The commit hooks build the gate with Cargo, so rustup is needed even where
# the repository holds no Rust.
command -v rustup >/dev/null ||
  fail 'rustup is required. Install it through your approved platform channel, then run this again.'
command -v curl >/dev/null || fail 'curl is required.'

# The toolbelt belongs to a machine, never to history: ignored from inside, so
# a repository whose .gitignore does not name .tools/ cannot commit it either.
mkdir -p .tools/bin
[[ -f .tools/.gitignore ]] || echo '*' >.tools/.gitignore

if [[ ! -x .tools/bin/mise || "$(.tools/bin/mise --version)" != "${MISE_VERSION#v} "* ]]; then
  echo "Fetching mise ${MISE_VERSION}"
  archive=$(mktemp)
  trap 'rm -f "$archive"' EXIT
  curl --fail --silent --show-error --location --output "$archive" \
    "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/\
mise-${MISE_VERSION}-linux-x64.tar.gz"
  # An unverified download is how a supply chain is entered, so the digest is
  # checked before the archive is opened rather than after.
  echo "${MISE_SHA256}  ${archive}" | sha256sum --check --strict
  tar -xzf "$archive" -C .tools/bin --strip-components=2 mise/bin/mise
fi

export PATH="$PWD/.tools/bin:$PATH"

# The root rust-toolchain.toml is the one copy of the local toolchain version;
# the justfile reads the same line, so a bump happens in one file. A repository
# without one builds no Rust of its own.
if [[ -f rust-toolchain.toml ]]; then
  TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
  [[ "$TOOLCHAIN" =~ ^1\.[0-9]+\.[0-9]+$ ]] ||
    fail 'rust-toolchain.toml must pin an exact stable version.'
  readonly TOOLCHAIN
  echo "Installing Rust ${TOOLCHAIN}"
  rustup toolchain install "$TOOLCHAIN" --profile minimal \
    --component rustfmt,clippy,llvm-tools-preview --no-self-update
fi

echo 'Installing pinned tools'
mise trust mise.toml
# --locked installs exactly the URLs mise.lock records and never rewrites it.
mise install --locked
if [[ -f justfile ]] && mise exec -- just --justfile justfile --show setup >/dev/null 2>&1; then
  # Just is one of the tools mise just installed. Run it from there once, so
  # the repository's own setup links the toolbelt and wires the commit hooks.
  mise exec -- just setup
else
  # Links keep every tool on the one PATH entry, wherever mise keeps its store.
  find .tools/bin -mindepth 1 ! -name mise -delete
  while IFS= read -r directory; do
    for file in "$directory"/*; do
      if [[ -f "$file" && -x "$file" ]]; then
        ln -s -- "$file" ".tools/bin/$(basename -- "$file")"
      fi
    done
  done < <(mise bin-paths)
  # The commit hooks only run once Git knows them, and their tools are fetched
  # now rather than in the middle of the first commit.
  prek install --prepare-hooks
fi

cat <<EOF

Ready. Add the tools to your PATH for this shell:

  export PATH="\$PWD/.tools/bin:\$PATH"
EOF
if [[ -f gate/Cargo.toml ]]; then
  cat <<EOF

Then run the gate:

  just check                   # everything that does not need the network
  CHECK_NETWORK=1 just check   # adds a live advisory lookup
EOF
elif [[ -f justfile ]]; then
  cat <<'EOF'

Then `just` lists what this repository runs.
EOF
else
  cat <<'EOF'

The commit hooks run on every commit; `prek run --all-files` runs them now.
EOF
fi
