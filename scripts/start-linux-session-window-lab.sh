#!/usr/bin/env bash
# Builds the fixed first-party held-session child, then opens the Linux Session
# Lab with that exact executable. This development check accepts no application
# path, document, or child argument.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_dir/.." && pwd)"
manifest_path="$repository_root/native/Cargo.toml"
target_root="${CARGO_TARGET_DIR:-$repository_root/native/target}"
if [[ "$target_root" != /* ]]; then
    target_root="$repository_root/$target_root"
fi
child_path="$target_root/release/anodrel-native-linux-session-client"

CARGO_TARGET_DIR="$target_root" cargo build --release --manifest-path "$manifest_path" \
    -p anodrel-native-linux-session-client
exec env CARGO_TARGET_DIR="$target_root" cargo run --release --manifest-path "$manifest_path" \
    -p anodrel-linux-session-window-lab -- "$child_path"
