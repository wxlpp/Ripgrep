---
name: oh-my-grep
status: in-progress
created: 2026-09-14T07:12:45Z
updated: 2026-09-14T11:25:26Z
progress: 86%
prd: .claude/prds/oh-my-grep.md
github: https://github.com/wxlpp/oh-my-grep/issues/5
---

# Epic: oh-my-grep

## Overview

Rename RipgrepKit → oh-my-grep across every layer, then fix the assessment findings in dependency order: result correctness, ignore/path/error semantics, bounded memory with a streaming API, App Store-safe packaging with a prepared release pipeline, PR CI with fuzzing, and documentation. See the PRD for measured evidence and acceptance criteria.

## Architecture Decisions

- **Rename first, as one mechanical task.** Every later task touches renamed identifiers; doing it first avoids cross-branch conflicts.
- **Contiguous context model.** A match owns only the contiguous non-match lines adjacent to it. With this rule at most one match is pending after-context, so the v0.3 window-attribution/eviction code is replaced by a single pending slot.
- **Binary detection via `grep-searcher`.** `BinaryDetection::quit(b'\x00')` by default; `searchBinary` switches to `BinaryDetection::none()`.
- **Pull-based streaming over UniFFI.** A `SearchSession` object owns a background walker thread and a bounded `crossbeam_channel`; Swift pulls batches from a detached task. Back-pressure comes from the bounded channel. UniFFI callback interfaces were rejected: no back-pressure and harder Swift 6 isolation.
- **One-shot `search` stays a separate path** (collect + sort) so its ordering contract and performance are unchanged; both paths share walker/searcher/sink construction.
- **Warnings are data, not errors.** Per-file problems accumulate (capped) in the result/summary; only request-level problems throw.
- **Static-library XCFramework** (`-library`, headers + `module.modulemap` per slice) so Xcode never embeds a framework bundle.
- **Manifest-level binary switch.** `Package.swift` checks for the local XCFramework; otherwise uses url+checksum written by the release workflow.
- **Release by `workflow_dispatch`**, which commits the checksum before tagging.

## Technical Approach

### Frontend Components
Swift `OhMyGrep` library: `Options` (+`requireGit`, `searchBinary`, `maxColumns`, defaults `maxMatches = 10_000`), `SearchResult` (+`warnings`), `Match` (+`lineTruncated`, `submatchRanges`), `stream(...)`, formatter `--` separators. `OhMyGrepTool`: new flags, `workingDirectory` resolution, `-A/-B` as `Int?`, tool schema.

### Backend Services
Rust `ohmygrep_core`: `sink.rs` contiguous context + CRLF + column cut; `search.rs` shared builders, binary detection, heap limit, warning collection, require_git; new `session.rs` with `SearchSession`.

### Infrastructure
`scripts/build-xcframework.sh` (static library slices), `scripts/generate-bindings.sh`, `Package.swift` local/remote switch, `Cargo.toml` release profile, `.github/workflows/ci.yml` (PR), `.github/workflows/release.yml` (dispatch), `.github/workflows/fuzz.yml` (nightly), `fuzz/` crate, `PrivacyInfo.xcprivacy`, README.

## Implementation Strategy

Serial for tasks touching `sink.rs`/`search.rs`/`Options` (001 → 002 → 003 → 004). Packaging (005) depends only on 001 and touches different files, but runs after 004 to keep one active branch at a time; CI (006) needs 005's build script; docs (007) last. Each task: private worktree off `epic/oh-my-grep`, plan, implement with focused tests, verification, review per risk (004/005 high risk, others normal, 001 low/mechanical), PR into the epic branch.

## Task Breakdown Preview

1. Rename end to end (mechanical)
2. Result correctness: contiguous context, `--` separators, CRLF, binary detection, `-A/-B 0`
3. Ignore rules, required paths/workingDirectory, warnings, symlink loop test
4. Memory limits and streaming `SearchSession` / `OhMyGrep.stream`
5. Packaging: static-library XCFramework, privacy manifest, release profile, Package.swift switch, dispatch release workflow
6. PR CI (macOS + iOS simulator + bindings drift) and cargo-fuzz nightly
7. Documentation: README, security note, Submatch offsets/ranges, tool schema, device probe re-run

## Dependencies

- PR #4 (CRLF) content folded into task 2, then PR #4 closed.
- iPhone 15 Pro for device probe; GitHub Actions macOS runners.

## Success Criteria (Technical)

- US-2 fixture expectations as Rust + Swift tests; `rg` parity check for `formattedAsText -C`.
- CI green on macOS and iOS simulator; bindings drift check clean.
- Streaming peak RSS on the 40 MB/400k-match log < baseline + 64 MB; one-shot within 10% of 0.14 s.
- Release app: no `Frameworks/OhMyGrepCore*`; privacy manifest in bundle; archive succeeds.
- No `ripgrepkit`/`RipgrepCore` identifiers in tracked source, scripts, workflows, README.

## Estimated Effort

~7 tasks, roughly 2–3 focused sessions; largest are tasks 4 and 5.

## Tasks Created
- [x] #6 - Rename RipgrepKit to oh-my-grep end to end (parallel: false)
- [x] #7 - Result correctness: contiguous context, separators, CRLF, binary detection, -A/-B 0 (parallel: false)
- [x] #8 - Ignore rules, required paths, workingDirectory, warnings, symlink loop (parallel: false)
- [x] #9 - Memory limits and streaming search API (parallel: false)
- [x] #10 - Packaging: static-library XCFramework, privacy manifest, release profile, release pipeline (parallel: false)
- [x] #11 - PR CI with iOS simulator tests and nightly cargo-fuzz (parallel: true)
- [ ] #12 - Documentation, Submatch ranges, tool schema, device re-verification (parallel: false)

Total tasks: 7
Parallel tasks: 1
Sequential tasks: 6
Estimated total effort: 35 hours
