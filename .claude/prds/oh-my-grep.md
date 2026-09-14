---
name: oh-my-grep
description: Rename RipgrepKit to oh-my-grep and make it correct, memory-bounded, App Store-safe and releasable on iOS/macOS
status: backlog
created: 2026-09-14T07:12:45Z
---

# PRD: oh-my-grep

## Executive Summary

RipgrepKit is a SwiftPM package that wraps ripgrep's reusable Rust crates (`ignore`, `grep-searcher`, `grep-regex`) through UniFFI for iOS 16+/macOS 13+, with a core search API and an rg-style argument parser for LLM tool calling. A 2026-09-14 assessment (macOS + iOS simulator + iPhone 15 Pro on iOS 26.6.1) found it functionally working on device but not shippable: a context-line data bug, no way for external consumers to depend on it, iOS-specific behavior gaps, App Store packaging risks, unbounded memory, and no PR CI.

This epic renames the project end to end to **oh-my-grep** and resolves every finding, adds bounded-memory limits plus a streaming API, and prepares (but does not trigger) the release pipeline.

## Problem Statement

Measured problems (evidence from the assessment):

1. **Wrong context lines.** `before_context` keeps stale lines across matches: file `a b c d HIT e HIT f`, `-B 2` → line 7 gets `["d","e"]` (line 4 is not adjacent); `-C 1` → `["d"]` instead of `["e"]`. `after_context` has the same class: `-A 3` attributes line 8 to the match on line 5 across the line-7 match. `formattedAsText` then prints wrong line numbers.
2. **Not consumable.** `Package.swift` uses `binaryTarget(path: "Frameworks/RipgrepCore.xcframework")` but that directory is gitignored; no release/tag exists; README is 9 bytes.
3. **`.gitignore` is ignored on iOS.** `ignore` requires a `.git` directory by default (same as `rg`). The macOS test passes only because the test bundle sits inside this repo's checkout; on the iOS simulator `testRespectsGitignoreByDefault` fails and on device `ignored.txt` is returned.
4. **App Store risk.** The XCFramework wraps a static archive in `.framework`; Xcode embeds a stub dylib at `App.app/Frameworks/RipgrepCoreFFI.framework` whose Info.plist lacks `MinimumOSVersion`. The linked app uses `stat`/`fstat`/`lstat`/`fstatat` (File Timestamp required-reason APIs) with no privacy manifest.
5. **Behavior gaps.** Binary files are searched (Mach-O content returned as lines); CRLF leaves a trailing `\r` (fix sits unmerged in PR #4); empty `paths` defaults to `"."`, which is `/` on iOS and silently yields 0 files; per-file IO errors are discarded; `-A 0` cannot override `-C`.
6. **Unbounded memory.** 400k matches over a 40 MB log → 227 MB peak RSS on the Rust side alone, before the UniFFI buffer and Swift copies. iOS has no swap; jetsam kills the process (extensions have ~30–120 MB budgets).
7. **Docs/process gaps.** Security note claims catastrophic backtracking (Rust regex is linear-time; on device `(a|aa)+$` over 5000 chars finished in 5 ms); Submatch offsets are undocumented UTF-8 byte offsets; no PR CI; no release profile; Tasks 36–38 (fuzz, README, symlink loop) deferred.

## User Stories

**US-1 — iOS app developer adds the package.**
As an iOS developer, I add `https://github.com/wxlpp/oh-my-grep` in Xcode and search files in my app container.
- Acceptance: after the first release is cut with the prepared workflow, SPM resolves the remote binary target without local build steps; the README documents install, sandbox paths and security-scoped URLs.
- Acceptance: a Release build of an app using the package contains no embedded `OhMyGrepCore*` framework under `App.app/Frameworks/`.
- Acceptance: the package ships a `PrivacyInfo.xcprivacy` declaring the required-reason APIs it links.

**US-2 — Developer relies on structured results.**
As a developer rendering results, I get context lines that are exactly the contiguous non-match lines around each match.
- Acceptance: for `a b c d HIT e HIT f`: `-B 2` → line 5 `["c","d"]`, line 7 `["e"]`; `-C 1` → line 5 before `["d"]` after `["e"]`, line 7 before `[]` after `["f"]`; `-A 3` → line 5 after `["e"]`.
- Acceptance: `formattedAsText` output for `-C` searches matches `rg --no-heading -n -C` output for the same fixture, including `--` group separators.
- Acceptance: CRLF files produce no trailing `\r`; binary files (NUL byte) are skipped unless `searchBinary`/`-a` is set.

**US-3 — Developer searches in a sandbox without a git repo.**
- Acceptance: `requireGit: false` makes `.gitignore` apply outside a git repository; default `true` matches rg.
- Acceptance: the gitignore tests pass identically on macOS, iOS simulator and device.
- Acceptance: `search(in: [])` throws `invalidArguments`; the Tool layer resolves relative and default paths against an explicit `workingDirectory`.
- Acceptance: unreadable paths/files appear in `SearchResult.warnings` instead of being silently dropped.

**US-4 — Developer searches large trees without being killed.**
- Acceptance: default `maxMatches = 10_000` and `maxColumns = 4096` bytes; `nil` disables each; truncation is flagged (`truncated`, per-match `lineTruncated`).
- Acceptance: `OhMyGrep.stream(pattern:in:options:)` returns an `AsyncThrowingStream`; with `maxMatches: nil` over the 40 MB / 400k-match log, peak RSS stays within a fixed bound independent of match count (target: < 64 MB above the no-match baseline) while a consumer drains it.
- Acceptance: cancelling the consuming Task stops the native search.

**US-5 — LLM tool integrator.**
- Acceptance: tool name `oh_my_grep`; schema documents supported flags including `-a/--text`, `--no-require-git`, `-M/--max-columns`; `-A 0` overrides `-C`.

**US-6 — Maintainer ships and keeps it green.**
- Acceptance: every PR runs cargo fmt/clippy/test, XCFramework build, bindings drift check, `swift test` on macOS and `xcodebuild test` on an iOS simulator.
- Acceptance: a `workflow_dispatch(version)` release workflow builds, tests, computes the checksum, commits `Package.swift` url/checksum, tags and publishes — so the tagged `Package.swift` always carries the correct checksum. Not triggered in this epic.

## Functional Requirements

**FR-1 Rename.** GitHub repo `wxlpp/oh-my-grep` (done); SwiftPM package `OhMyGrep`, products `OhMyGrep` and `OhMyGrepTool`; Swift namespace `OhMyGrep`; Rust crate `ohmygrep_core`; clang/FFI module `OhMyGrepCoreFFI`; XCFramework `OhMyGrepCore.xcframework`; tool name `oh_my_grep`; scripts, workflows and docs updated. Historical `docs/superpowers/*` keep their text with a rename note. The local checkout directory is not renamed.

**FR-2 Context correctness.** Before-context holds only lines after the previous match; a new match terminates after-context collection of earlier matches. `formattedAsText` inserts `--` between non-contiguous groups when context is requested.

**FR-3 CRLF.** Strip one trailing `\r` before the final `\n` on match and context lines (from PR #4); PR #4 is closed as superseded.

**FR-4 Binary detection.** Default: quit searching a file on the first NUL byte. `Options.searchBinary` / `-a, --text` searches it as text.

**FR-5 rg flag fidelity.** `-A`/`-B` accept explicit 0 that overrides `-C`.

**FR-6 Ignore rules.** `Options.requireGit` (default `true`) and `--no-require-git`; tests independent of the host checkout; `.ignore` file coverage; self-referencing symlink fixture does not hang or duplicate results.

**FR-7 Paths and errors.** Core API requires ≥1 path. `OhMyGrep.run(_:workingDirectory:)` and `handleToolCall(_:workingDirectory:)` resolve relative/default paths; missing working directory with relative paths → `invalidArguments`. `SearchResult.warnings: [Warning]` (path, message), capped at 100.

**FR-8 Memory limits.** `Options.maxColumns: Int? = 4096` (bytes; match and context lines longer than the limit are cut to a UTF-8-boundary prefix of at most that many bytes; a cut match line sets `lineTruncated = true` and keeps only submatches that end inside the prefix); `maxMatches` default `10_000`; multiline searches use a 64 MiB heap limit, over-limit files become warnings.

**FR-9 Streaming.** Rust `SearchSession` UniFFI object: walker on a background thread feeding a bounded channel; `next_batch(max) -> Option<Vec<SearchMatch>>`, `cancel()`, `finish() -> summary (cancelled, truncated, files_searched, warnings)`. Swift `OhMyGrep.stream` wraps it in `AsyncThrowingStream<Match, Error>` with Task-cancellation propagation. Order: per-file line order; files in discovery order. One-shot `search` keeps sorted-by-path output.

**FR-10 Packaging.** XCFramework built from static libraries (`-library … -headers`); `PrivacyInfo.xcprivacy` in the `OhMyGrep` target (reason codes verified against Apple documentation); Cargo `[profile.release]` with `lto = true`, `codegen-units = 1`, unwinding panics kept.

**FR-11 Release pipeline (prepared only).** `Package.swift` uses the local XCFramework path when present, otherwise `url` + `checksum` (clearly marked placeholders until the first release); `release.yml` becomes `workflow_dispatch(version)`; README runbook.

**FR-12 CI + fuzz.** PR workflow per US-6; cargo-fuzz targets for regex compilation, glob compilation and random `SearchRequest`, run nightly with a time bound.

**FR-13 Docs.** README (install, `search`/`stream`/tool examples, supported/unsupported flags, iOS notes, memory, privacy, build from source, release runbook); corrected security note; Submatch UTF-8 byte offsets documented plus `Match.submatchRanges: [Range<String.Index>]`; code comments in touched files follow the project comment discipline.

## Non-Functional Requirements

- **Platforms:** iOS 16+, macOS 13+, Swift 6 language mode for hand-written targets; toolchain Xcode 26.
- **Memory:** streaming path bounded as in US-4; one-shot path bounded by defaults.
- **Performance:** no regression beyond 10% in wall time for the 40 MB log search (baseline 0.14 s release, macOS) on the one-shot path.
- **Cancellation latency:** unchanged or better than current (event/1 MiB byte cadence).
- **Binary size:** report release-profile before/after for the iOS slice and a linked Release app.
- **Compatibility:** no release exists, so the rename and API changes are free; after this epic, public API changes need deliberate versioning.

## Success Criteria

1. All US-2 fixture expectations pass as automated tests in Rust and Swift.
2. `cargo test`, `swift test` (macOS) and `xcodebuild test` (iOS simulator) all green in the PR CI workflow.
3. iPhone device probe run: gitignore with `requireGit: false`, binary skip, context, stream, cancel, warnings scenarios all report expected values.
4. Streaming memory measurement meets the US-4 bound; one-shot 40 MB log search within the 10% time budget.
5. Release app build shows no embedded framework; `xcodebuild archive` succeeds; privacy manifest present in the built bundle.
6. `grep -ri ripgrepkit` over tracked source, scripts, workflows and README returns no hits (historical docs excepted).

## Constraints & Assumptions

- UniFFI 0.28 proc-macro mode stays; bindings are generated and committed.
- No GitHub release or tag is created in this epic; remote SPM consumption is verifiable only after the maintainer runs the release workflow.
- ripgrep defaults are the reference for tool-layer semantics (`requireGit` default true, binary quit on NUL).
- The iPhone 15 Pro (paired, developer mode) is available for the device probe; App Store Connect upload validation is not available, so archive + bundle inspection is the evidence for packaging.
- Work runs serially across tasks touching `sink.rs`/`search.rs`.

## Out of Scope

- Publishing a release, creating tags, or registering in the Swift Package Index.
- Renaming the local checkout directory.
- rg features beyond those listed (`--pre`, `-z`, `--type-add`, `--sort`, `--vimgrep`, replacement, PCRE2, encoding transcoding other than CRLF).
- Grouped-by-file FFI result model redesign.
- watchOS/tvOS/visionOS/Mac Catalyst slices.

## Dependencies

- GitHub Actions `macos` runners with Xcode 26 and iOS simulator runtimes.
- Crates: `ignore`, `grep-searcher`, `grep-regex`, `grep-matcher`, `uniffi 0.28`, `crossbeam-channel`; `cargo-fuzz` (nightly toolchain) for fuzz targets.
- `swift-argument-parser` ≥ 1.5.0.
- Apple privacy manifest documentation for required-reason codes.
- Open PR #4 (CRLF) content, superseded by this epic.
