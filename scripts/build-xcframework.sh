#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE="ripgrep_core"
LIB_NAME="libripgrep_core.a"
FRAMEWORK_NAME="RipgrepCoreFFI"
XCFRAMEWORK_NAME="RipgrepCore"
BUILD_DIR="build/xcframework"
OUT="Frameworks/${XCFRAMEWORK_NAME}.xcframework"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
VERSION="${VERSION:-0.1.0}"

rm -rf "$BUILD_DIR" "$OUT"
mkdir -p "$BUILD_DIR"

build_static() {
    local triple="$1"
    echo "Building $CRATE for $triple..."
    cargo build -p "$CRATE" --release --target "$triple"
}

stage_framework() {
    local lib_path="$1"
    local slice_name="$2"
    local fw_dir="$BUILD_DIR/$slice_name/$FRAMEWORK_NAME.framework"
    mkdir -p "$fw_dir/Headers" "$fw_dir/Modules"

    cp "$lib_path" "$fw_dir/$FRAMEWORK_NAME"
    cp "Sources/RipgrepKitFFI/RipgrepCoreFFI.h" "$fw_dir/Headers/"
    cat > "$fw_dir/Modules/module.modulemap" <<'EOF'
framework module RipgrepCoreFFI {
    umbrella header "RipgrepCoreFFI.h"
    export *
    module * { export * }
}
EOF
    cat > "$fw_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>$FRAMEWORK_NAME</string>
    <key>CFBundleIdentifier</key><string>com.ripgrep.RipgrepCoreFFI</string>
    <key>CFBundleName</key><string>$FRAMEWORK_NAME</string>
    <key>CFBundlePackageType</key><string>FMWK</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>1</string>
</dict>
</plist>
EOF
    echo "$fw_dir"
}

[[ -f "Sources/RipgrepKitFFI/RipgrepCoreFFI.h" ]] || \
  { echo "ERROR: Sources/RipgrepKitFFI/RipgrepCoreFFI.h missing. Run scripts/generate-bindings.sh first." >&2; exit 1; }

# Single slice (smoke test)
build_static "aarch64-apple-darwin"
LIB="${CARGO_TARGET_DIR}/aarch64-apple-darwin/release/$LIB_NAME"
MAC_FW=$(stage_framework "$LIB" "macos-arm64")

xcodebuild -create-xcframework -framework "$MAC_FW" -output "$OUT"
echo "Built: $OUT"
ls -la "$OUT"
