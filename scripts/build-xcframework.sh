#!/usr/bin/env bash
# Builds Frameworks/OhMyGrepCore.xcframework from static libraries (no .framework
# wrappers), so Xcode links the code into the app and embeds nothing.
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE="ohmygrep_core"
LIB_NAME="libohmygrep_core.a"
MODULE_NAME="OhMyGrepCoreFFI"
OUT="Frameworks/OhMyGrepCore.xcframework"
BUILD_DIR="build/xcframework"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-16.0}"

HEADER="Sources/OhMyGrepFFI/${MODULE_NAME}.h"
[[ -f "$HEADER" ]] || { echo "ERROR: $HEADER missing. Run scripts/generate-bindings.sh first." >&2; exit 1; }

rm -rf build "$OUT"
# Headers live in a module-named subdirectory: Xcode copies every static
# xcframework's Headers into one include/ dir, so a root module.modulemap collides
# with other packages that do the same.
mkdir -p "$BUILD_DIR/headers/$MODULE_NAME"
cp "$HEADER" "$BUILD_DIR/headers/$MODULE_NAME/"
cat > "$BUILD_DIR/headers/$MODULE_NAME/module.modulemap" <<EOF
module ${MODULE_NAME} {
    header "${MODULE_NAME}.h"
    export *
}
EOF

for triple in aarch64-apple-darwin x86_64-apple-darwin aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
    echo "Building $CRATE for $triple..."
    # `cargo rustc --crate-type staticlib`: with rlib also in crate-type, cargo build
    # skips LTO and ships bitcode-bearing objects instead.
    cargo rustc -p "$CRATE" --release --target "$triple" --crate-type staticlib
done

lib() { echo "${CARGO_TARGET_DIR}/$1/release/$LIB_NAME"; }

mkdir -p "$BUILD_DIR/macos" "$BUILD_DIR/ios-sim" "$BUILD_DIR/ios"
lipo -create "$(lib aarch64-apple-darwin)" "$(lib x86_64-apple-darwin)" -output "$BUILD_DIR/macos/$LIB_NAME"
lipo -create "$(lib aarch64-apple-ios-sim)" "$(lib x86_64-apple-ios)" -output "$BUILD_DIR/ios-sim/$LIB_NAME"
cp "$(lib aarch64-apple-ios)" "$BUILD_DIR/ios/$LIB_NAME"

xcodebuild -create-xcframework \
    -library "$BUILD_DIR/macos/$LIB_NAME" -headers "$BUILD_DIR/headers" \
    -library "$BUILD_DIR/ios/$LIB_NAME" -headers "$BUILD_DIR/headers" \
    -library "$BUILD_DIR/ios-sim/$LIB_NAME" -headers "$BUILD_DIR/headers" \
    -output "$OUT"

echo "Built: $OUT"
