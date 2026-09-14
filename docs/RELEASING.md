# Releasing oh-my-grep

`Package.swift` uses `Frameworks/OhMyGrepCore.xcframework` when it exists (a local
build) and otherwise downloads the XCFramework attached to the GitHub release named
by `releaseVersion`, verified with `releaseChecksum`. Until the first release is
published those values are placeholders (`0.0.0`, all zeros), so depending on the
package by URL fails until then.

## Cutting a release

1. Make sure `main` is green (PR CI) and contains everything for the release.
2. GitHub → Actions → **Release** → *Run workflow*, enter the version as `X.Y.Z`
   (no `v` prefix; SwiftPM resolves plain semantic version tags).
3. The workflow, on `macos-15`:
   - checks the generated UniFFI bindings are up to date;
   - runs `cargo test`, builds and zips the XCFramework (`scripts/package-release.sh`), runs `swift test`;
   - runs `scripts/set-release.sh X.Y.Z <sha256>` and commits `Release X.Y.Z` to `main`;
   - tags that commit `X.Y.Z`, pushes, and creates the GitHub release with the zip and its `.sha256`;
   - builds a throwaway consumer package that depends on `exact: "X.Y.Z"` to prove the
     binary target downloads and the checksum matches.
4. If `main` is protected, allow `github-actions[bot]` to push, or the commit step fails
   before anything is tagged or published.

## If the workflow fails

- Before "Commit, tag and push": nothing was published; fix and rerun with the same version.
- After the tag was pushed but before the release was created: delete the tag
  (`git push origin :refs/tags/X.Y.Z`), revert the `Release X.Y.Z` commit, rerun.
- After the release was created: publish a new patch version instead of replacing
  assets; SwiftPM caches binary targets by URL and checksum.

## Building locally

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
bash scripts/generate-bindings.sh   # after changing the Rust API
bash scripts/build-xcframework.sh   # creates Frameworks/OhMyGrepCore.xcframework
swift test
```

Cargo does not rebuild cached objects when `IPHONEOS_DEPLOYMENT_TARGET` /
`MACOSX_DEPLOYMENT_TARGET` change; run `cargo clean` before a local build that must
carry a new minimum OS version (check with `otool -l <lib> | grep minos`).
