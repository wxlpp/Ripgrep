#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE="ripgrep_core"
LIB_NAME="libripgrep_core.a"
FRAMEWORK_NAME="RipgrepCoreFFI"
XCFRAMEWORK_NAME="RipgrepCore"
BUILD_DIR="build/xcframework"
OUT="Frameworks/${XCFRAMEWORK_NAME}.xcframework"

rm -rf "$BUILD_DIR" "$OUT"
mkdir -p "$BUILD_DIR"

build_static() {
    local triple="$1"
    cargo build -p "$CRATE" --release --target "$triple"
}

stage_framework() {
    local triple="$1"
    local slice_name="$2"
    local fw_dir="$BUILD_DIR/$slice_name/$FRAMEWORK_NAME.framework"
    mkdir -p "$fw_dir/Headers" "$fw_dir/Modules"

    cp "target/$triple/release/$LIB_NAME" "$fw_dir/$FRAMEWORK_NAME"
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
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key><string>1</string>
</dict>
</plist>
EOF
    echo "$fw_dir"
}

# Single slice (smoke test)
build_static "aarch64-apple-darwin"
MAC_FW=$(stage_framework "aarch64-apple-darwin" "macos-arm64")

xcodebuild -create-xcframework -framework "$MAC_FW" -output "$OUT"
echo "Built: $OUT"
ls -la "$OUT"
