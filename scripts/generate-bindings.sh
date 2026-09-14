#!/usr/bin/env bash
set -euo pipefail
# Generate Swift bindings from the compiled ohmygrep_core dylib using uniffi-bindgen.
cd "$(dirname "$0")/.."
cargo build --locked -p ohmygrep_core --release
DYLIB="${CARGO_TARGET_DIR:-target}/release/libohmygrep_core.dylib"
OUT_DIR="Sources/OhMyGrepFFI"
mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/OhMyGrepCore.swift "$OUT_DIR"/OhMyGrepCoreFFI.h "$OUT_DIR"/OhMyGrepCoreFFI.modulemap
cargo run --locked -p uniffi-bindgen -- generate --library "$DYLIB" --language swift --out-dir "$OUT_DIR"
echo "Generated bindings in $OUT_DIR:"
ls -la "$OUT_DIR"
