#!/usr/bin/env bash
set -euo pipefail
# Generate Swift bindings from the compiled ripgrep_core dylib using uniffi-bindgen.
cd "$(dirname "$0")/.."
cargo build -p ripgrep_core --release
DYLIB="${CARGO_TARGET_DIR:-target}/release/libripgrep_core.dylib"
OUT_DIR="Sources/RipgrepKitFFI"
mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/RipgrepCore.swift "$OUT_DIR"/RipgrepCoreFFI.h "$OUT_DIR"/RipgrepCoreFFI.modulemap
cargo run -p uniffi-bindgen -- generate --library "$DYLIB" --language swift --out-dir "$OUT_DIR"
echo "Generated bindings in $OUT_DIR:"
ls -la "$OUT_DIR"
