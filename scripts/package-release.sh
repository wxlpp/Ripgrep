#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

bash scripts/build-xcframework.sh

OUT="dist"
mkdir -p "$OUT"
ZIP="$OUT/RipgrepCore.xcframework.zip"
rm -f "$ZIP"

# Use ditto so symlinks/permissions survive the round-trip.
ditto -c -k --keepParent Frameworks/RipgrepCore.xcframework "$ZIP"

SHA=$(shasum -a 256 "$ZIP" | awk '{print $1}')
echo "$SHA" > "$ZIP.sha256"
echo "Built $ZIP"
echo "SHA256: $SHA"
