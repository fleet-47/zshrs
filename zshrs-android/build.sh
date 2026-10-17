#!/usr/bin/env sh
# Bootstrap the patch binary; compilation of zshrs is a separate Cargo command.
set -eu
repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd -- "$repo_dir"
exec cargo run --release --manifest-path zshrs-android/runner/Cargo.toml -- apply --repo "$repo_dir" "$@"
