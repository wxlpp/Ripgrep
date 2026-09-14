#!/usr/bin/env bash
# Usage: scripts/set-release.sh <version> <sha256>
# Points Package.swift's remote binary target at the given release.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:?version}"
CHECKSUM="${2:?sha256}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "ERROR: version must be X.Y.Z, got '$VERSION'" >&2; exit 1; }
[[ "$CHECKSUM" =~ ^[0-9a-f]{64}$ ]] || { echo "ERROR: checksum must be 64 lowercase hex characters" >&2; exit 1; }

perl -0pi -e "s/^let releaseVersion = \"[^\"]*\"\$/let releaseVersion = \"$VERSION\"/m; s/^let releaseChecksum = \"[^\"]*\"\$/let releaseChecksum = \"$CHECKSUM\"/m" Package.swift

grep -q "^let releaseVersion = \"$VERSION\"$" Package.swift || { echo "ERROR: releaseVersion not updated" >&2; exit 1; }
grep -q "^let releaseChecksum = \"$CHECKSUM\"$" Package.swift || { echo "ERROR: releaseChecksum not updated" >&2; exit 1; }
echo "Package.swift now points at release $VERSION"
