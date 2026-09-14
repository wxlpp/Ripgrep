# Releasing oh-my-grep

On `main`, `Package.swift` has `useLocalBinary = true` and builds against
`Frameworks/OhMyGrepCore.xcframework`, which you create with
`scripts/build-xcframework.sh`. Each release is a tag on a commit that is **not** on
`main`: there `scripts/set-release.sh` sets `useLocalBinary = false` and points the
binary target at the zip attached to that GitHub release (`releaseVersion`,
`releaseChecksum`). Consumers therefore depend on a version (`from:` / `exact:`), not
on `branch: "main"`, and nothing can be resolved by URL until the first release exists.

## Cutting a release

1. Make sure `main` is green (PR CI, `.github/workflows/ci.yml`) and contains everything for the release.
2. GitHub → Actions → **Release** → *Run workflow*, enter the version as `X.Y.Z`
   (no `v` prefix; SwiftPM resolves plain semantic version tags).
3. The workflow, on `macos-15` with a pinned Rust toolchain (`RUST_TOOLCHAIN` in `release.yml`):
   - checks the generated UniFFI bindings are up to date;
   - runs `cargo test`, builds and zips the XCFramework (`scripts/package-release.sh`), runs `swift test`;
   - runs `scripts/set-release.sh X.Y.Z <sha256>` and commits only `Package.swift` as `Release X.Y.Z`;
   - tags that commit `X.Y.Z` and pushes only the tag, then creates the GitHub release with
     the zip and its `.sha256` (for a moment the tag exists before its asset);
   - builds a throwaway consumer package that depends on `exact: "X.Y.Z"` for macOS and iOS
     to prove the binary target downloads and the checksum matches.

## If the workflow fails

- Before "Commit and push the release tag": nothing was published; fix and rerun with the same version.
- After the tag was pushed but before the release was created: delete the tag
  (`git push origin :refs/tags/X.Y.Z`) and rerun.
- In "Verify a consumer can resolve the release": the release is already public. Investigate;
  if it is broken, publish a new patch version instead of replacing assets, because SwiftPM
  caches binary targets by URL and checksum.

## Building locally

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
bash scripts/generate-bindings.sh   # after changing the Rust API
bash scripts/build-xcframework.sh   # creates Frameworks/OhMyGrepCore.xcframework
swift test
```

Without the XCFramework, SwiftPM and Xcode report that the local binary target is missing.

Cargo does not rebuild cached objects when `IPHONEOS_DEPLOYMENT_TARGET` /
`MACOSX_DEPLOYMENT_TARGET` change; run `cargo clean` before a local build that must
carry a new minimum OS version (check with `otool -l <lib> | grep minos`).
