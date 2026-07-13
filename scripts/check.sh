#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

find_cargo() {
  if command -v cargo >/dev/null 2>&1 && cargo --version >/dev/null 2>&1; then
    command -v cargo
    return
  fi

  for candidate in "${CARGO_HOME:-$HOME/.cargo}/bin/cargo" "$HOME"/.rustup/toolchains/stable-*/bin/cargo; do
    if [ -x "$candidate" ] && "$candidate" --version >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return
    fi
  done

  printf 'cargo was not found; install a stable Rust toolchain\n' >&2
  exit 1
}

cargo_bin=$(find_cargo)
PATH=$(dirname "$cargo_bin"):$PATH
export PATH

scripts/check-agent-harness-interface.sh
"$cargo_bin" fmt --all --check
"$cargo_bin" test --all-targets --locked
"$cargo_bin" clippy --all-targets --locked -- -D warnings
"$cargo_bin" build --release --locked
"$cargo_bin" run --quiet --locked -- --plain data/folder1 data/folder2 >/dev/null

printf 'fdiff checks passed\n'
