#!/usr/bin/env bash
# Runs a command in a tooling image with the repository mounted at /workspace.
set -euo pipefail

image="${1:?usage: run-tooling.sh <image> <command> [args...]}"
shift

mkdir -p .cache/cargo/git .cache/cargo/registry .cache/home .cache/npm

run=(
  docker run --rm --init
  --user "$(id -u):$(id -g)"
  --volume "${PWD}:/workspace"
  --workdir /workspace
  --env HOME=/workspace/.cache/home
  # Debugging switches pass through when set. Other host Rust variables, such
  # as CARGO_HOME or RUSTUP_TOOLCHAIN, would point at host paths and toolchains.
  --env RUST_BACKTRACE
  --env RUST_LOG
)
if [[ -t 0 && -t 1 ]]; then
  run+=(--interactive --tty)
fi

"${run[@]}" "${image}" mise exec -- "$@"
