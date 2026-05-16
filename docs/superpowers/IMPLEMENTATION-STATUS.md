# RipgrepKit Implementation — Resume State

**Last updated:** 2026-05-16
**Branch:** `worktree-impl+v0.1.0` (pushed to origin)
**Worktree:** `.claude/worktrees/impl+v0.1.0` (created via EnterWorktree)
**Execution model:** superpowers:subagent-driven-development (fresh implementer + reviewer subagent per task)

## How to Resume

In a new session, from `/Users/evan/Repositories/Ripgrep`:

1. Enter the existing worktree:
   - The branch `worktree-impl+v0.1.0` already exists locally and on origin.
   - Use `EnterWorktree` with `path` pointing at the existing worktree, OR `git worktree list` to find it and `cd` there. The worktree dir is `.claude/worktrees/impl+v0.1.0`.
2. Re-read the plan: `docs/superpowers/plans/2026-05-15-ripgrep-swift-package.md`
3. Re-read the spec: `docs/superpowers/specs/2026-05-15-ripgrep-swift-package-design.md`
4. Resume subagent-driven execution at **Task 14**.

## Progress

> **v0.2 BACKLOG EXECUTION (branch `v0.2-backlog`, off merged-main `2ebec38`; PR #1 merged 2026-05-16).** Baseline green (cargo 33 / swift 48). Items P1–P7 in impact-priority order:
> - **P1 ✅ DONE** (`f5c0dea`,`6c01bda`) — per-match `after_context` window attribution in `ChannelSink` (root cause of the documented `formattedAsText` overlap limitation, now fixed at the data layer). +`saturating_add`, explicit `sb.line_number(true)`, +adjacent-matches fixture/test. cargo 34, swift 48.
> - **P2 ✅ DONE — REFUTED + HARDENED** (`96382ed`) — ccd premise was WRONG (verified vs grep-searcher 0.1.16 `Sink` docs): `Ok(false)` already stops the file immediately AND `finish()` still flushes pending matches; `Err(SinkAbort)` would skip `finish` (lose partial results) for zero latency gain. **No behavior change.** Added contract comments + 2 non-vacuous regression tests. Cancel-latency concern correctly folds into **P6** (`CANCEL_CHECK_EVERY`), not P2. cargo 36, swift 48.
> - **P3 ✅ DONE — REFUTED + DOCUMENTED** (`5dbc7e8`) — ccd premise unsound (atomics model): Acquire/Release does NOT serialize the check-then-act overshoot; these counters publish no data (matches flow via crossbeam channel); single-location coherence already gives `Relaxed` prompt visibility; Errata #7 post-truncation guarantees result-correctness. **No behavior change** (comment-only, 0 `Ordering::` changes). cargo 36, swift 48.
> - **P4 ✅ DONE** (`abdd72e`) — removed the genuinely-dead `SearchRequest.timeout_ms` FFI field (Rust struct + all regenerated-binding sites + `Options.toFFI`); the real timeout path (`CancelToken`/`Search.swift`/`Parse`/`RipgrepArgs`) byte-untouched. Behavior-preserving: cargo 36 / swift 48 UNCHANGED (proves nothing depended on it); idempotent regen. Also cleaned the inert `r.timeout_ms=Some(1)` from the misnamed cancel test (+ CancelToken comment), setting up P5.
> - **P5 ✅ DONE** (`143d9e2`) — rename ONLY (`timeout_marks_result_cancelled`→`pre_expired_cancel_marks_result_cancelled`). YAGNI: deadline-expiry path already covered by existing `cancel_tests::deadline_trips_token_after_elapse` + Swift layer — no redundant test added. cargo 36 unchanged.
> - **P6 ✅ DONE** (`c281889`) — P6a: hybrid event-OR-byte cancel cadence (`poll_cancel(bytes)`; trips on events≥100 OR bytes≥1 MiB, both reset) — bounds cancel/timeout latency on giant/minified-line files (the real lever P2's refuted concern pointed at); non-vacuity independently proven; P2 `Ok(false)` contract + P1 attribution byte-untouched; P1/P2 regression tests green. P6b: `Submatch` u32 documented (NO widen — YAGNI). cargo 37 (3× stable), swift 48.
> - **P7 ✅ DONE** (`4a2cccd`) — extracted all 16 `#[cfg(test)]` modules from lib.rs into `src/tests/*.rs` (`#[cfg(test)] #[path] mod X;` crate-root children → `super::`/`pub(super)` intact). Pure move (742 ins==742 del); test-NAME set proven byte-identical before/after (not just count); Deviation #1/#3 confirmed. cargo 37, swift 48, no production/Swift change.
>
> **ccd v0.2 adversarial review (pre-PR, 2026-05-16) — dispositions** (Critical 0, Major 3, Minor 5, Nit 4; per `receiving-code-review`):
> - **FIXED** (`ec94569`) — Major #2: `Options.toFFI()` narrowed `maxMatches`/`maxFiles`/`before`/`afterContext` via trapping `UInt32.init` → values > `UInt32.max` = uncatchable host-process crash (same class as PR#1 #4's negative-guard, upper bound was missed). Now range-checked → `Ripgrep.Error.invalidArguments`; +regression test (swift 49). Also folded Nit #11 (clearer guards) + Nit #9 (self-contained Errata comment in search.rs).
> - **PUSHED BACK (reasoned, no change):** Major #1 (per-match `path.clone()`) — fix as stated inapplicable: `SearchMatch.path` is a UniFFI `Record String`, owned per-result by FFI contract; `Arc<str>` on the sink can't remove the required owned `String`; a real fix needs an FFI result-model redesign → recorded as a **potential v0.3 architectural item**, not v0.2 cleanup scope. Major #3 (build_walker path-exists TOCTOU) — benign for a search tool (reviewer concedes; `ignore::WalkBuilder` handles vanished paths gracefully; pre-check is a pre-existing UX fast-fail). Minor #5 (`timeout` "dropped") — INCORRECT (ccd lacked `Search.swift`: `Options.timeout`→`CancelToken`; P4 deliberately removed `timeout_ms` from `SearchRequest`). Minor #7 (poll_cancel dual-reset) — intentional P6 design; latency bound holds; independent resets add complexity for zero benefit.
> - **NOTED (pre-existing Phase-1 / negligible, not v0.2 scope):** Minor #4 (mid-file `use` in search.rs), Minor #6 (`before_buf` cap≥1 when before_context=0), Minor #8 (two `cancel_ok_false_*` tests share setup — separate is fine for failure isolation), Nit #10 (`SinkAbort` discards IO-error detail — Phase-1 design), Nit #12 (`SearchRequest` field docs).
>
> **FINISHED 2026-05-16: v0.2 pushed + PR #2 opened → https://github.com/wxlpp/Ripgrep/pull/2** (base `main`, head `v0.2-backlog`, off merged-main `2ebec38`; new origin branch). ccd adversarial review done pre-PR (dispositions above). Worktree preserved for PR iteration.
>
> **PR #2 Copilot review — dispositions (2026-05-17)** (COMMENTED, 3 inline; per `receiving-code-review`):
> - **#2 FIXED** (`474bbed`): `Duration.ffiMilliseconds` was a trapping `UInt64(...)` — a negative/huge public `Options.timeout` (Codable) → uncatchable host crash in `Ripgrep.search` (same class as PR#1 #4 / ccd Major #2). Now `func ffiMilliseconds() throws(Ripgrep.Error)` (negative guard + `multipliedReportingOverflow` + `UInt64(exactly:)`); `Search.swift` rethrows synchronously. +regression test (swift 50). Valid values (incl. `.nanoseconds(1)` used by CancellationTests) unchanged. Completes the "no public-API input crashes the host" invariant.
> - **#1 FALSE POSITIVE (pushed back):** Copilot claimed `SearcherBuilder` "defaults to not computing line numbers" so `sink_emits_match_per_line`'s `line_number==1/3` asserts fail. **Wrong for grep-searcher 0.1.16**: `Config::default` has `line_number: true` (`grep-searcher-0.1.16/src/searcher/mod.rs:195`). The test is correct and green (37/37). No change.
> - **#3 DEFERRED → v0.3 perf (reasoned):** the per-match after-context loop is O(pending_after) per After line. Already evaluated in P1's code-quality review: bounded in practice (pending_after flushed at each `context_break`; degenerate only with unbounded `max_matches` + one huge context group); correctness verified. The eviction optimization is a non-trivial hot-path refactor for a degenerate-case gain — out of v0.2 cleanup scope.
>
> ### v0.3 perf — EXECUTION (branch `v0.3-perf`, off merged-main `7b9348a`; PR #2 merged 2026-05-16)
> - **V3-perf-2 ✅ DONE** (`a4a0433`) — `flush_matured(cur)` drains the matured PREFIX of `pending_after` (strict `m.line + after_context < cur`), called in `matched()` (pre-push, cur=new line) and `context()` After (pre-attribution, cur=abs_line). Bounds the Vec to O(after_context). Behavior-preserving: per-match `after_context` IDENTICAL (matured ⇒ window fully delivered), results sorted in search.rs. Independently proven: disable eviction → bound test FAILS but attribution+P1 still PASS. cargo 39 (3× stable, P1/P2/P6/limits/context green), swift 50.
> - **V3-perf-1 — DESIGN-FIRST (architectural; ROI under question):** `ChannelSink` clones `path: String` per match. `SearchMatch.path` is a UniFFI `Record String` owned per-result by FFI contract — the only real fix is an FFI result-model redesign (group-by-file shape, or a custom interned UniFFI type), which ripples through the public Swift `Ripgrep.SearchResult/Match` API + formatters + Tasks 22/23 tests + consumers, for a micro-allocation the actual LLM-tool workload (IO+regex-bound, max_matches-truncated) almost never feels. Pending controller→user decision (full redesign vs lighter mitigation vs accept+document); do NOT blindly redesign the FFI surface.
>
> **✅ v0.2 BACKLOG COMPLETE (P1–P7).** Final: `cargo test -p ripgrep_core` 37, `swift test` 48, clippy/fmt clean, 0 warnings. Outcome: P1 (genuine after_context correctness fix) + P6a (genuine giant-line cancel-latency fix) were the real fixes; P4 a genuine dead-FFI-field removal; **P2 & P3 were ccd suggestions that proper verification REFUTED** (no behavior change — converted to hardening/docs; not implementing the wrong "fixes" was the correct call: P2's `Err(SinkAbort)` would have regressed partial-result preservation, P3's Acquire/Release was cargo-cult); P5 (rename only, YAGNI), P6b/P7 (proportionate, behavior-preserving). Each item: TDD where applicable + two-stage/independent verification (non-vacuity probes throughout).
>
> Note: P2 & P3 were ccd suggestions that proper verification refuted — neither was a real defect; both converted to durable documentation/hardening with no behavior change. P1 was the one genuine correctness fix so far.
>
> **STATUS 2026-05-16: Tasks 14–34 ALL DONE (spec ✅ + code-quality ✅, two-stage reviewed). Task 35 deferred-by-design (release-time). Tasks 36–38 deferred. Full suite green: Rust 33 tests, Swift 48 tests, clippy/fmt clean, 0 Swift-6 warnings. 15 plan deviations + Errata #7/#13 recorded below.**
>
> **FINAL WHOLE-IMPLEMENTATION REVIEW (opus, holistic): verdict "With fixes — nothing Critical".** Whole stack proven coherent end-to-end from clean (0-warning build, 33 Rust + 48 Swift green, reproducible bindings zero-diff, clean tree, CI-safe trap fixtures via `git archive`, sound Swift-6/5 `@unchecked Sendable` boundary, 15 deviations + 2 errata interlock without contradiction, spec-intent fidelity confirmed). The one Important finding was a DOC inaccuracy ("pinned 1.7.1" vs manifest `from: "1.5.0"`) — corrected in the Task 18 entry; the code is correct/idiomatic as-is. All other open items (precondition-on-negative-JSON, double "invalid regex:" prefix, `-A 0` rg-fidelity, cooperative-pool occupancy, Task 35) confirmed correctly deferred for the v0.1.0 dev-form milestone. Next: `superpowers:finishing-a-development-branch` (per user CLAUDE.md → `/codex:adversarial-review` before any merge).
>
> **FINISHED 2026-05-16: branch pushed + PR opened → https://github.com/wxlpp/Ripgrep/pull/1** (base `main`, head `worktree-impl+v0.1.0`, fast-forward push — no force). External adversarial review (ccd/DeepSeek) done pre-PR: Critical=none; dispositions in the "External adversarial review" section below. Worktree preserved for PR iteration. Implementation phase complete; remaining work is PR review feedback + the deferred Task 35 (release-time) / Tasks 36–38.

### Task 35 — DEFERRED BY DESIGN (release-time only)
`Task 35` (switch `Package.swift` from local `binaryTarget(path: "Frameworks/RipgrepCore.xcframework")` to remote `binaryTarget(url:checksum:)`) CANNOT be done now: it requires a published GitHub Release with a known sha256, and no release/tag exists (publishing is out of implementation scope). The local `binaryTarget(path:)` (Task 18) is correct and intentional for v0.1.0 dev. **Runbook when v0.1.0 is cut:** (1) push a `v0.1.0` tag → `.github/workflows/release.yml` runs on `macos-14` (clean cargo state ⇒ Deviation #10 min-OS stamping correct), builds the xcframework, runs tests, `package-release.sh` produces `dist/RipgrepCore.xcframework.zip` + `.sha256`, and `softprops/action-gh-release@v2` publishes them. (2) Replace the `.binaryTarget(name:"RipgrepCore", path:"Frameworks/RipgrepCore.xcframework")` in `Package.swift` with `.binaryTarget(name:"RipgrepCore", url:"https://github.com/<owner>/Ripgrep/releases/download/v0.1.0/RipgrepCore.xcframework.zip", checksum:"<contents of dist/RipgrepCore.xcframework.zip.sha256>")`. (3) `swift package reset && swift package resolve && swift build && swift test` → green. (4) commit `Package.swift` + `Package.resolved`. NOTE: `binaryTarget(name:"RipgrepCore", ...)` stays — only the framework's internal clang module is `RipgrepCoreFFI` (Deviation #9); do not rename.

**DONE — Phase 1 (Rust core), Tasks 1-13.** 32 Rust tests passing. `cargo test -p ripgrep_core` green; `cargo clippy --tests -- -D warnings` clean; `cargo fmt --check` clean.

Commits (oldest→newest): b509d0d, e18570a, b459f43, 191de86, 1daeb34, f4e82f6, 8a689c7, 1d702eb, 6a2cff5, 2144d15, 703d800, d12bf75, 75702fe, b87d6d1.

| Task | Status |
|---|---|
| 1 Workspace + crate skeleton | ✅ |
| 2 RipgrepError | ✅ |
| 3 Request/Match/Result types | ✅ |
| 4 CancelToken | ✅ |
| 5 ChannelSink (+ dead-state fix) | ✅ |
| 6 build_matcher | ✅ |
| 7 build_walker + fixture | ✅ |
| 8 search_blocking end-to-end | ✅ |
| 9 context lines/multiline | ✅ |
| 10 max_matches/max_files/timeout | ✅ |
| 11 external cancellation | ✅ |
| 12 panic containment | ✅ |
| 13 clippy/fmt closeout | ✅ |

**DONE — Task 14 (UniFFI scaffolding).** Commits `75412c0` + `13517db`. Spec ✅ + code-quality ✅.

**DONE — Task 15 (Generate Swift bindings).** Commits `bd15e9d` + `8b1a5a5`. Spec ✅ (regen byte-identical) + code-quality ✅. crate-type now `["staticlib","cdylib","rlib"]` (Deviation #8). `scripts/generate-bindings.sh` uses `cargo run -p uniffi-bindgen` and respects `CARGO_TARGET_DIR` + purges stale output.

### Generated FFI artifacts (committed; consumed by Tasks 16/18 — DO NOT hand-edit)

Running `bash scripts/generate-bindings.sh` produces, under `Sources/RipgrepKitFFI/`:
- `RipgrepCore.swift` (~1278 lines — the Swift API)
- `RipgrepCoreFFI.h` (~594 lines)
- `RipgrepCoreFFI.modulemap` — declares `module RipgrepCoreFFI { header "RipgrepCoreFFI.h" export * }`

**Critical naming for Task 16/18 reconciliation:** the clang module is **`RipgrepCoreFFI`** (NOT `RipgrepCore`). The plan's Task 16 xcframework script and Task 18 Package.swift were written assuming `RipgrepCore`/`RipgrepCoreFFI.h` — header filename matches, but the binaryTarget/framework/module layering must be reconciled against these ACTUAL names when Task 16/18 run. Exported entrypoint: `public func searchBlocking(request: SearchRequest, cancel: CancelToken) throws -> SearchResult`. `CancelToken` = `open class`, `SearchRequest`/`SearchResult`/`Submatch`/`SearchMatch` = structs, `RipgrepError` = `public enum`. UniFFI auto-converted `search_blocking` → `searchBlocking`.

**DONE — Task 16 (single-slice XCFramework smoke test).** Commits `c9ddf68` + `468ea2a`. Spec ✅ + code-quality ✅. `scripts/build-xcframework.sh` builds `Frameworks/RipgrepCore.xcframework` (gitignored) containing `RipgrepCoreFFI.framework` (Deviation #9). Hardened: `CARGO_TARGET_DIR`, `VERSION` env vars, missing-header guard. **`stage_framework()` was refactored to `stage_framework(lib_path, slice_name)`** — so Task 17 must NOT add the plan's separate `stage_framework_from_lib`; instead reuse the single parameterized `stage_framework` for both per-triple and lipo'd fat-lib slices.

**DONE — Task 17 (5 Apple targets, 3-slice XCFramework). ✅ PHASE 2 COMPLETE.** Commits `62a7fa4` + `f122a59`. Spec ✅ + code-quality ✅. `bash scripts/build-xcframework.sh` produces `Frameworks/RipgrepCore.xcframework` (gitignored) with 3 slices: `macos-arm64_x86_64`, `ios-arm64` (device, no variant), `ios-arm64_x86_64-simulator` (variant=simulator). Each slice's framework = `RipgrepCoreFFI.framework`, binary = static `ar archive`. `build/` fully purged each run (`rm -rf build "$OUT"`).

**DONE — Task 18 (Package.swift + local binaryTarget).** Commit `830b8d1`. Spec ✅ + code-quality ✅ (manifest verbatim per plan). swift-argument-parser: `Package.swift` declares **`from: "1.5.0"`** (plan-verbatim; idiomatic `>=1.5.0,<2.0.0` range — exact-pinning a library dependency is discouraged as it forces downstream resolution conflicts); `Package.resolved` resolves it to **1.7.1** in THIS repo, but downstream consumers ignore our `Package.resolved` and resolve their own within the range (`fullMessage(for:)` and the parse surface used are long-stable, well before 1.5.0). Earlier notes in this doc that said "pinned 1.7.1" were imprecise shorthand for "resolved 1.7.1" — corrected here per final-review (the code is correct/idiomatic as-is; this was a doc wording fix, not a code change). `swift package resolve` works with no source dirs (Task 19 ordering safe). Plus build-script correctness follow-up `7629dcc` (below).

**DONE — build-script min-OS fix (`7629dcc`).** Code-quality review found the cargo-built static libs embedded `LC_VERSION_MIN macOS 10.12 / iOS 10.0`, contradicting Package.swift's `.macOS(.v12)/.iOS(.v15)`. `scripts/build-xcframework.sh` now `export`s `MACOSX_DEPLOYMENT_TARGET=12.0` / `IPHONEOS_DEPLOYMENT_TARGET=15.0`. Verified on clean build: macOS `minos 12.0`, iOS device/sim `minos 15.0`.

10. **CARGO cache caveat for the min-OS stamp (`*_DEPLOYMENT_TARGET` not fingerprinted by cargo).** The min-OS env vars only take effect on a **clean** cargo compile — cargo does NOT invalidate cached `.rlib`/`.o` when `MACOSX/IPHONEOS_DEPLOYMENT_TARGET` change (known cargo limitation). The script's `rm -rf build "$OUT"` does NOT purge `${CARGO_TARGET_DIR}/<triple>/release`. Impact: the **release path is correct** (Task 33/34 CI runs on a fresh macos-14 runner → clean build → correct minos). **Local/manual releases must start from a FULL clean cargo state** — confirmed 2026-05-16 that `cargo clean -p ripgrep_core` is INSUFFICIENT (it clears fingerprints but cached `.rlib`/`.o` still carry the old min-OS); a full `cargo clean` (or fresh checkout) is required to restamp the deployment target. NOT auto-forced in the script (full 5-target clean rebuild every local run is too costly for v0.1.0 DX); documented limitation. Re-verify minos via `otool -l <slice-binary> | grep minos` before any manual release.

### Task 19 carry-forwards (from Task 18 review)
- Task 19 plan Step 1 ALREADY creates `Tests/RipgrepKitCoreTests/Fixtures/.gitkeep` + `Tests/RipgrepKitToolTests/Fixtures/.gitkeep`. This is REQUIRED — without it `swift build` hard-errors on `resources: [.copy("Fixtures")]`. Ensure the Task 19 implementer does that step.
- Task 19 is the FIRST `swift build`/`swift test`. The xcframework must exist locally first (`bash scripts/build-xcframework.sh`). Deviation #9 module-collision watch-out becomes concrete here — resolve empirically.
- Nice-to-have (not blocking): add a one-line "run scripts/build-xcframework.sh before swift build" note to `README.md` (full README is deferred Task 37).

**DONE — Task 19 (stub Swift targets; Phase 2↔3 integration PROVEN).** Commits `10cdebe` + `f9bf39d`. Spec ✅ + code-quality ✅. `swift build` + `swift test` (2 placeholders) green; the `RipgrepCore.xcframework` binaryTarget is genuinely linked (45.9M framework in `.build`).

11. **`RipgrepKitFFI` target needs Swift-5 language mode + sources allowlist.** UniFFI-generated `RipgrepCore.swift` has a module-level lazy global (`private var initializationResult`) that is a hard Swift 6 `MutableGlobalVariable` error. Resolution (in `Package.swift`, scoped ONLY to the `RipgrepKitFFI` target — package stays `swiftLanguageModes: [.v6]`):
    ```swift
    .target(name: "RipgrepKitFFI",
            dependencies: ["RipgrepCore"],
            sources: ["RipgrepCore.swift"],            // allowlist: ignores generated .h/.modulemap (those feed scripts/build-xcframework.sh + the xcframework)
            swiftSettings: [.swiftLanguageMode(.v5)]),
    ```
    `.h`/`.modulemap` stay committed in `Sources/RipgrepKitFFI/` (NOT in the Swift target). The `RipgrepCoreFFI` clang module is vended by the binaryTarget xcframework. This is the standard UniFFI+Swift6 pattern.

### CRITICAL carry-forwards for upcoming tasks (from Task 19 review — bake into implementer prompts)

- **Task 23 (`Ripgrep.search` async wiring) — Sendable boundary.** `RipgrepKitFFI` is Swift-5 mode → UniFFI types carry NO `Sendable`. `RipgrepKitCore` is Swift 6. Task 23's `withTaskCancellationHandler` + detached `Task` passing `CancelToken` (an `open class`) / `SearchRequest` / `SearchResult` across isolation WILL produce Swift 6 errors ("Capture of non-sendable type 'CancelToken'..."). Pre-warn the implementer: add `@preconcurrency import RipgrepKitFFI` in the file doing async work; if `CancelToken` capture still complains, use a thin internal `struct CancelHandle: @unchecked Sendable { let token: CancelToken }` wrapper (semantically honest — the Rust token is cross-thread by design). Tasks 20-22 (Error/Options/SearchResult value types) likely only need plain or `@preconcurrency import`.
- **Task 27 & 31 (RipgrepKitTool).** `Sources/RipgrepKitTool/Stub.swift` is currently `@_exported import RipgrepKitCore` (plan-exact stub). This MUST be deleted/replaced when RipgrepKitTool gets real code, or it silently re-exports all of RipgrepKitCore's API through the Tool product (breaks the product layering). The Task 27/31 implementer must remove this stub file's `@_exported` line.

**DONE — Task 20 (Ripgrep.Error).** Commits `8a2a14c` + `4922e13`. Spec ✅ + code-quality ✅. 5-case `Sendable` enum + internal `from(_:)` mapping the 4 PascalCase FFI cases. Plain `import RipgrepKitFFI` works for sync/value-type use. 4 swift tests.

12. **Spec contradiction: `Duration` public API vs `.iOS(.v15)/.macOS(.v12)` floor → raise floor to iOS 16 / macOS 13.** Spec line 125 declares `.iOS(.v15), .macOS(.v12)` but lines 323/346 use `Duration` (`Options.timeout`, `SearchResult.elapsed`), and `Duration` is only available iOS 16.0+ / macOS 13.0+. Unconditional `Duration` in a public API at iOS15/macOS12 = hard `swift build` error ("'Duration' is only available in macOS 13.0 or newer"). The spec conflated *toolchain* floor (Xcode 16/Swift 6, line 615) with *deployment* floor. The `Duration`-first API is the spec's deliberate design (Tasks 21/22/23/26/29/30 all use it); `@available`-gating the entire public surface or replacing `Duration` would be far more invasive. **Resolution:** `Package.swift` `platforms: [.iOS(.v16), .macOS(.v13)]`. For consistency with Deviation #10 (binary min-OS should match the declared package floor), `scripts/build-xcframework.sh` deployment-target defaults bumped to `IPHONEOS_DEPLOYMENT_TARGET=16.0` / `MACOSX_DEPLOYMENT_TARGET=13.0` + xcframework rebuilt. Applied as a focused infra commit before Task 21.

**DONE — Task 21 (Ripgrep.Options).** Commits `2e78ddd` + `cddc002`. Spec ✅ + code-quality ✅. 14-field Codable/Sendable struct, rg-aligned defaults; `Duration` JSON roundtrip confirmed lossless; Codable forward-compat note added.

- **Task 23 carry-forward (Duration→ms):** `toFFI()` must convert `timeout: Duration?` to ms WITHOUT truncating sub-second — use `UInt64(c.seconds * 1000 + c.attoseconds / 1_000_000_000_000_000)` (the plan's Task 23 code block already does this; spec line ~375 shows an abbreviated form that would truncate — use the full form).

**DONE — Task 22 (SearchResult types).** Commits `6ea127a` + `59ed240`. Spec ✅ + code-quality ✅. Submatch/Match (Codable+Sendable+Equatable), SearchResult (Codable+Sendable, NOT Equatable) + formatters. 9 swift tests.

### Verified generated FFI surface (for Task 23 — DO NOT make implementer re-derive)

- `public func searchBlocking(request: SearchRequest, cancel: CancelToken) throws -> SearchResult`
- `CancelToken`: `open class`; has `public convenience init(timeoutMs: UInt64?)`, `open func cancel()`, `open func isCancelled() -> Bool`. (The plan's `CancelToken(timeoutMs:)` / `token.cancel()` are valid.)
- FFI `SearchRequest` fields EXACTLY match the plan's `toFFI()`: pattern, paths, caseInsensitive, smartCase, multiline, includeGlobs, excludeGlobs, fileTypes, respectGitignore, includeHidden, beforeContext:UInt32, afterContext:UInt32, maxMatches:UInt32?, maxFiles:UInt32?, maxFileSizeBytes:UInt64?, timeoutMs:UInt64?.
- FFI `SearchMatch`: path:String, lineNumber:UInt64, line:String, beforeContext:[String], afterContext:[String], submatches:[Submatch]. FFI `Submatch`: start:UInt32, end:UInt32. FFI `SearchResult`: matches:[SearchMatch], truncated:Bool, cancelled:Bool, filesSearched:UInt64, elapsedMs:UInt64.
- **Name collisions:** FFI `SearchResult` vs `Ripgrep.SearchResult`; FFI `Submatch` vs `Ripgrep.Submatch` (FFI `SearchMatch` does NOT collide with `Ripgrep.Match`). Task 23 conversion inits must qualify FFI types as `RipgrepKitFFI.SearchResult` / `RipgrepKitFFI.SearchMatch` / `RipgrepKitFFI.Submatch`.

### Task 25 watch-outs (from Task 22 review)
- `formattedAsText` does NOT dedup overlapping context between adjacent same-file matches (documented limitation). Task 25 tests should cover: blank-line-between-different-files, empty-results (`formattedAsJSONLines()` on empty → `""`, which `split("\n")` yields 1 empty element not 0), and multi-match-same-file behavior.

### Task 30/31 decision point (record consciously before Task 31)
- `Match` JSON uses synthesized camelCase keys (`lineNumber`, `beforeContext`...), NOT rg-style `line_number`. Current tool contract (`handleToolCall` returns raw String to the LLM) is LLM-opaque so camelCase is acceptable. IF Task 30/31 adds any code/consumer that parses the JSON as rg-JSON, add `CodingKeys` (snake_case) to `Match`. Decide explicitly at Task 31, don't discover post-merge.

**DONE — Task 23 (Ripgrep.search wiring). ✅ PHASE 3 COMPLETE.** Commits `224c3a9` + `6b07b84`. Spec ✅ + code-quality ✅. `@preconcurrency import RipgrepKitFFI` (Options.swift + Search.swift) + `private struct CancelHandle: @unchecked Sendable { let token: CancelToken }` (sound: Rust token is atomic flag + immutable deadline). `Ripgrep.search` async with `withTaskCancellationHandler`+`Task.detached`; conversion inits qualify FFI types `RipgrepKitFFI.*`. Shared `Duration.ffiMilliseconds` helper. All FFI errors → `Ripgrep.Error` (catch-all → `.internalPanic`). 0 Swift6 warnings, 9 swift tests.

### Task 26 (CancellationTests) carry-forwards
- Cancellation NEVER throws `CancellationError` — it surfaces ONLY as `SearchResult.cancelled == true` (or empty matches). Task 26 should cancel the OUTER Swift `Task` (not call token.cancel() directly) and assert the return is a `SearchResult` with `.cancelled == true` (NOT a thrown error). Timeout path: `opts.timeout = .nanoseconds(1)` / pre-tripped → `r.cancelled || r.matches.isEmpty`.
- Post-return `onCancel` race (cancel arrives after searchBlocking returns) is benign (atomic store no-op) — a test cancelling right after `await` resolves must not crash/hang.

### Pre-v0.1.0 decision points (record; not blocking task flow)
- **`Options.toFFI()` uses `precondition` on negative context/limits** (plan's M5). `Options` is a public `Codable` struct; negative values from decoded JSON (CLI path Tasks 28-30) would CRASH the host process. Decide before v0.1.0 ships: keep `precondition` (document "caller must pass validated Options") OR change to throwing `Ripgrep.Error.invalidArguments` (would make `toFFI` throws → ripples to `search()`; an API/spec change). Surfaced, not silently reworked mid-task.
- **Cooperative-thread-pool occupancy:** `Task.detached` + blocking FFI occupies a pool thread per call. Documented via `/// - Important:` on `search()`. A dedicated-executor offload (`withCheckedContinuation` + dedicated queue) is a v0.2 item — fine for CLI/bounded v0.1.0 use.

**DONE — Task 24 (Swift test fixture mini-repo).** Commit `53b1232`. Spec ✅ (`git archive HEAD` proves all 7 files incl. gitignore-trap files committed/CI-safe) + controller code-quality ✅. byte-identical to Rust fixture at `Tests/RipgrepKitCoreTests/Fixtures/mini/`. Trap files (`ignored.txt`, `target/built.txt`) force-added past the fixture's own `.gitignore`.

- **Task 25 watch-out:** plan's `fixturePath()` uses `Bundle.module.url(forResource: "mini", withExtension: nil)`. `resources:[.copy("Fixtures")]` preserves dir structure, so `mini` is at `Fixtures/mini` in the bundle — `forResource:"mini"` may return nil (→ force-unwrap crash). If so, use `Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")`. Adapt the helper empirically; the 5 tests must genuinely pass (not crash on nil).

**DONE — Task 25 (SearchTests).** Commit `970467d`. Spec ✅ (anti-vacuity probe proved gitignore test genuine) + controller code-quality ✅. **Full Rust↔UniFFI↔Swift pipeline proven end-to-end.** `fixturePath()` uses `Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")`. 14 swift tests.

**DONE — Task 26 (CancellationTests). ✅ PHASE 4 COMPLETE.** Commit `029e39a`. Spec ✅ (anti-vacuity proof: genuine cancellation, no escape-hatch pass; no deadlock) + controller code-quality ✅. Cancellation/timeout wiring proven end-to-end. 16 swift tests, cargo 33.

### Task 27 carry-forward (the @_exported landmine — act on it in Task 27)
`Sources/RipgrepKitTool/Stub.swift` = `@_exported import RipgrepKitCore` (Task 19 placeholder, sole purpose: make the then-empty RipgrepKitTool target compile). Task 27 adds the FIRST real RipgrepKitTool file (`Tokenizer.swift`, which has its own `import RipgrepKitCore`). At that point **delete `Sources/RipgrepKitTool/Stub.swift`** — keeping `@_exported` silently re-exports all of RipgrepKitCore's API through the RipgrepKitTool product (breaks the product layering). Plan's later Tool files/tests already `import RipgrepKitCore` explicitly, so removal is safe. Bundle the deletion into Task 27's commit (RipgrepKitTool: stub → real).

**DONE — Task 27 (Tokenizer).** Commits `bcb0d60` + `64b408d`. Spec ✅ (algorithm byte-verbatim plan) + code-quality ✅. Plan Errata #13 applied (corrected testSingleQuotes). `Sources/RipgrepKitTool/Stub.swift` (@_exported) DELETED — RipgrepKitTool = real code now. `+testDanglingBackslashThrows`. 25 swift tests.

14. **Package.swift: `RipgrepKitToolTests` deps now `["RipgrepKitTool", "RipgrepKitCore"]`** (added RipgrepKitCore). Reason: Task 27/29/30/31 tests use `@testable import RipgrepKitCore`; `@testable` needs a DIRECT test-target dependency (local SwiftPM tolerated the omission; stricter CI/toolchains reject it). Already fixed in `64b408d` — Tasks 29/30/31 inherit the correct declaration, no re-fix needed.

**DONE — Task 28 (RipgrepArgs).** Commit `d9780f5`. Spec ✅ (byte-verbatim, behavior-probed flag wiring) + controller code-quality ✅. Internal `RipgrepArgs: ParsableCommand`, ArgumentParser-only. 32 swift tests.

**DONE — Task 29 (Parse → ParsedInvocation).** Commits `cac5b25` + `aa69f5f`. Spec ✅ + code-quality ✅. `parseFilesize` negative-guarded. 42 swift tests.

### Pre-v0.1.0 known limitations / Task 31 carry-forward (from Task 29 review)
- **`-A 0`/`-B 0` rg-fidelity gap:** `RipgrepArgs.afterContext/beforeContext` default `Int = 0`, so explicit `-A 0` is indistinguishable from unset → `rg -C 3 -A 0` yields after=3 (rg gives 0). Proper fix = `Int?` in RipgrepArgs + merge rework (touches plan-dictated Task 28 code + Task 29 merge). Recorded as a known v0.1.0 subset limitation (spec §2 already frames the tool as an rg subset); decide before v0.1.0 ships whether to fix. NOT reworked mid-flow (plan-dictated).
- **Task 31 carry-forward — `--help`/`--version` → `.invalidArguments`:** ArgumentParser throws `CleanExit.helpRequest/versionRequest` for `--help`/`--version`; Parse.swift's catch-all maps these to `Ripgrep.Error.invalidArguments(message: <help text>)`. For the LLM tool (Task 31 `handleToolCall`), an LLM passing `--help` gets a help dump as an "ERROR:". Task 31 must decide: catch `CleanExit` separately (return schema/clean response) OR document in `toolSchema` that `--help`/`--version` aren't valid tool inputs. Surface explicitly at Task 31.

**DONE — Task 30 (Run convenience methods).** Commit `da6df8c`. Spec ✅ + ground-truth investigation ✅ (the spec reviewer's probe evidence was flawed — it accidentally ran the system `rg` binary; a controller-dispatched scratch test PROVED via real `Ripgrep.run` calls: gitignore respected end-to-end, `formattedAsJSONLines()` emits OUR camelCase `Match` JSON `{"path","lineNumber","line","beforeContext","afterContext","submatches"}` — NOT rg-native `type:begin`). RipgrepKitToolTests fixture force-added (CI-safe via `git archive` proof). 44 swift tests.

- **Reinforces the Task 31 JSON-key decision:** the `--json` tool output is our `Match` struct with **camelCase synthesized keys** (`lineNumber`, not rg's `line_number`). For the LLM tool contract (`handleToolCall` returns the raw string to the LLM) this is acceptable/LLM-opaque. Only add `CodingKeys` (snake_case) if Task 31 introduces a consumer that parses it as rg-JSON. Decide explicitly at Task 31.

**DONE — Task 31 (Tool — toolSchema/ToolInput/handleToolCall). ✅ PHASE 5 CODE COMPLETE.** Commit `bcb0a8b`. Spec ✅ (toolSchema independently parsed valid; handleToolCall anti-vacuity proven) + controller code-quality ✅. Verbatim plan. **Decisions:** `--help`/`--version`→`ERROR: <usage>` ACCEPTED for v0.1.0 (informative LLM feedback, not a crash; toolSchema documents the supported subset). camelCase JSON keys ACCEPTED (LLM-opaque). 48 swift tests + 33 Rust tests, 0 warnings.

15. **MINOR v0.1.0 polish (pre-existing, Task 20 area — NOT blocking):** `Ripgrep.Error.invalidPattern` renders as `"invalid regex: \(p)"` (Task 20), but the Rust-supplied `p` for a regex-compile failure already begins `"invalid regex: regex parse error: ..."`, so user-facing messages double-prefix: `ERROR: invalid regex: invalid regex: regex parse error...`. Cosmetic only; functional behavior correct. If polished pre-v0.1.0: either drop the `"invalid regex: "` prefix in `Error.message` for `.invalidPattern`, or strip it from the Rust error string. Surfaced, not reworked (Task 20 plan-verbatim, out of Task 31 scope).

**DONE — Task 32 (Full test suite green-light). ✅ PHASES 1-5 COMPLETE.** Checkpoint commit `1b37509`. Verification gate (controller-run, no implementation). Full suite GREEN: `cargo test -p ripgrep_core` 33; `cargo clippy --tests -D warnings` + `cargo fmt --check` clean; `swift build` 0 warnings; `swift test` 48 tests / 0 failures (clean `.build`). xcframework rebuilt & gitignored.

**DONE — Task 33 (package-release.sh).** Commit `6830e31`. Spec ✅ (clean re-run: 39.5MB zip, SHA-MATCH, valid xcframework, `dist/` gitignored) + controller code-quality ✅.

**DONE — Task 34 (`.github/workflows/release.yml`). ✅ PHASE 6 COMPLETE (33+34).** Commit `7dc5a8f`. Spec ✅ + controller code-quality ✅. Deviation #6 applied (no `cargo install uniffi-bindgen`). Task 35 deferred-by-design (see top). **All implementable tasks (14-34) complete.** NEXT: final whole-implementation code review → `superpowers:finishing-a-development-branch` (per user CLAUDE.md: `/codex:adversarial-review` before merge).

<!-- HISTORICAL pointer below (superseded by STATUS banner at top of Progress) -->
**(superseded) Task 34** (`.github/workflows/release.yml`). **Apply Deviation #6: the plan's workflow has `cargo install uniffi-bindgen --version 0.28.0` — DROP that step entirely** (that crate/version doesn't exist; bindings come from the in-tree `cargo run -p uniffi-bindgen` invoked by `scripts/generate-bindings.sh`). Also: the workflow runs on a fresh `macos-14` runner (clean cargo state → Deviation #10 min-OS stamping works correctly). After Task 34: Task 35 (remote `binaryTarget(url:checksum:)`) is RELEASE-TIME-ONLY / **deferred-by-design** (no GitHub release exists; local `binaryTarget(path:)` stays for v0.1.0). Tasks 36-38 deferred.

### ⚠️ Phase 3 critical watch-out (Task 18/19 — the integration linchpin)

`swift build` is FIRST exercised at Task 19; that is where Phase 2↔3 integration is proven. Known issues to resolve there:
1. **xcframework is gitignored** — `Package.swift` uses local `binaryTarget(path: "Frameworks/RipgrepCore.xcframework")`, but that dir is gitignored and not committed. Task 18/19 implementer MUST run `bash scripts/build-xcframework.sh` first so the xcframework exists locally before `swift build`/`swift test`.
2. **Deviation #9 module collision risk:** `Sources/RipgrepKitFFI/` holds generated `RipgrepCore.swift` + `RipgrepCoreFFI.h` + `RipgrepCoreFFI.modulemap`. The plan's `RipgrepKitFFI` SwiftPM target points there; its `RipgrepCore.swift` does `import RipgrepCoreFFI`, and that clang module is vended by the **binaryTarget xcframework's** `RipgrepCoreFFI.framework`. Having a *second* `RipgrepCoreFFI.h`/`.modulemap` inside the `RipgrepKitFFI` Swift target's source dir will likely make SwiftPM treat it as a mixed/clang target or produce a duplicate-module conflict at `swift build`. Resolution (decide empirically at Task 19): probably the `RipgrepKitFFI` target must contain ONLY `RipgrepCore.swift` (exclude/move the `.h`+`.modulemap`, which belong only inside the xcframework). The plan/spec did not resolve this — treat the plan's Package.swift as a starting point, fix the layering when `swift build` tells you the truth.

**FOUNDATION FLAKE — RESOLVED (commit `057c9be`, 2026-05-16).** `limits_tests::max_matches_truncates_and_flags` flaked ~20-40% under parallel test execution (pre-existing Phase 1 defect, not a Task 14 regression). Root cause: off-by-one in `crates/ripgrep_core/src/search.rs` truncation detection — `if matches.len() > max` should be `>= max`. When the parallel walker stopped at exactly `max` matches, `truncated` was wrongly `false`. Fixed to `>= max`. Verified deterministic: 50× targeted + 30× full-suite (debugger) + 6× full-suite (controller), 0 failures. 33 tests stable.

## Plan Deviations Discovered (apply these going forward)

1. **Fixture path:** Plan's test `req()` helpers use `"crates/ripgrep_core/tests/fixtures/mini"`. The correct path is `"tests/fixtures/mini"` because `cargo test -p ripgrep_core` runs with the crate root as cwd, not the workspace root. Tasks 7-12 already use the short form. Any future Rust test referencing the fixture must use `tests/fixtures/...`.

2. **`grep_matcher::Match` has private fields** (crate resolved to grep-matcher 0.1.8). The plan's `Match { start, end }` destructuring in `sink.rs` was replaced with `mat.start()` / `mat.end()` accessor calls. Already applied in Task 5.

3. **Test helper visibility:** `search_e2e_tests::req()` is `pub(super)` so `context_tests` / `limits_tests` can reuse it. Keep this when touching those modules.

4. **Multiline regex in tests:** Rust regex `.` is not DOTALL by default. Task 9's multiline test uses `r"match[\s\S]*match"` (not `r"match.*\n.*match"`). Use `[\s\S]` for cross-line test patterns.

5. **`#[allow(dead_code)]` on `SearchResult.elapsed_ms`** in options.rs — intentional, commented "Exported to Swift via UniFFI". It becomes live once Task 14 adds UniFFI derives.

6. **uniffi-bindgen install is broken in the plan (resolved 2026-05-16).** Plan Task 15 + Task 34 call `cargo install uniffi-bindgen --version 0.28.0`. That crate/version does NOT exist on crates.io (`error: could not find uniffi-bindgen in registry crates-io with version =0.28.3` either). Resolved uniffi lib version is **0.28.3**. Per official UniFFI 0.28 docs (context7 /mozilla/uniffi-rs), bindgen runs from an in-tree binary. **Final structure (after Task 14 code-review fixes, commit `13517db`): a SEPARATE workspace crate, not a bin inside ripgrep_core** — putting the `cli` feature on `ripgrep_core`'s own deps leaked clap into the shipped iOS static/dylib.
   - `crates/ripgrep_core/Cargo.toml`: `uniffi = { version = "0.28" }` — NO `cli`, NO `build` feature. Proc-macro mode (`setup_scaffolding!()` + derives) needs neither. **`crates/ripgrep_core/build.rs` was DELETED** (the plan's `.ok()` build.rs is a no-op that silently swallows a NotFound forever; proc-macro mode requires no build.rs). Do not recreate it.
   - `crates/uniffi-bindgen/` is a separate workspace member (binary crate `uniffi-bindgen`, `publish=false`, `uniffi = { version = "0.28", features = ["cli"] }`, `src/main.rs` = `fn main() { uniffi::uniffi_bindgen_main() }`). Root `Cargo.toml` `members = ["crates/*"]` glob already covers it.
   - Invoke via `cargo run -p uniffi-bindgen -- generate --library target/release/libripgrep_core.dylib --language swift --out-dir Sources/RipgrepKitFFI`.
   - **Task 15 must use `cargo run -p uniffi-bindgen -- generate ...`** (NOT bare `uniffi-bindgen`, NOT `cargo run -p ripgrep_core --bin ...`). **Task 34 must drop the `cargo install uniffi-bindgen` CI step** (the separate crate builds from source in the workspace).
   - clap is provably absent from `ripgrep_core` runtime closure (`cargo tree -p ripgrep_core -e normal -i clap` → not found).
   - Pre-flight env state (2026-05-16): all 5 Apple Rust targets installed; Xcode 26.4 present; Cargo.lock committed; sink.rs rustfmt fixup committed (242dffd).

7. **PLAN ERRATA — `max_matches` truncation off-by-one (resolved in code, NOT in plan).** The plan at `docs/superpowers/plans/2026-05-15-ripgrep-swift-package.md` ~line 1229 (Task 10 region) shows `if matches.len() > max`. That is a bug — correct is `>= max` (collecting exactly `max` means the limit was hit, so `truncated` must be true). Fixed in `search.rs` at commit `057c9be`. If any future task re-applies that plan snippet verbatim, do NOT reintroduce `> max`. The plan code blocks are reference, not gospel — implementers should prefer the committed source.

8. **`crate-type` lacks `cdylib` — blocks `uniffi-bindgen --library` (Task 15).** Task 1 set `crates/ripgrep_core/Cargo.toml` `crate-type = ["staticlib", "rlib"]`. `cargo build --release` produces only `libripgrep_core.a` + `.rlib`, NO `.dylib`. But `uniffi-bindgen generate --library` (proc-macro mode) needs a cdylib to introspect metadata. The plan/spec never pinned crate-type. **Fix (folded into Task 15 scope, since the bindings artifact depends on it):** change to `crate-type = ["staticlib", "cdylib", "rlib"]` (additive — `staticlib` still feeds the xcframework in Tasks 16/17, `rlib` still feeds `cargo test`, `cdylib` produces `libripgrep_core.dylib` for bindgen). Generated-file naming note: uniffi 0.28 proc-macro mode emits files named by the crate namespace (`ripgrep_core.swift`, `ripgrep_coreFFI.h`, `ripgrep_coreFFI.modulemap`); `module_name="RipgrepCore"` in uniffi.toml only sets the *clang module* name inside the modulemap, NOT the filenames. Tasks 16/18 (which the plan wrote expecting `RipgrepCoreFFI.h`) must be reconciled to the ACTUAL generated filenames — Task 15 implementer reports them; controller reconciles at Task 16.

9. **XCFramework module name MUST be `RipgrepCoreFFI`, not `RipgrepCore` (Task 16/17/18 reconciliation).** Verified: generated `Sources/RipgrepKitFFI/RipgrepCore.swift` does `#if canImport(RipgrepCoreFFI)` and the generated modulemap declares `module RipgrepCoreFFI`. SwiftPM binaryTarget vends the **framework's internal module name** (from the framework's `Modules/module.modulemap`), NOT the binaryTarget `name:` nor the `.xcframework` filename. So the plan's Task 16 `FRAMEWORK_NAME="RipgrepCore"` would make Task 19 `swift build` fail with "no such module 'RipgrepCoreFFI'". **Resolution:**
   - The staged `.framework` inside the xcframework = `RipgrepCoreFFI.framework`; its binary = `RipgrepCoreFFI`; header = `Headers/RipgrepCoreFFI.h`; `Modules/module.modulemap` = `framework module RipgrepCoreFFI { umbrella header "RipgrepCoreFFI.h" export * module * { export * } }`.
   - KEEP the xcframework output filename `Frameworks/RipgrepCore.xcframework` and Package.swift `binaryTarget(name:"RipgrepCore", path:"Frameworks/RipgrepCore.xcframework")` (matches spec, .gitignore line 5 `Frameworks/RipgrepCore.xcframework/`, release zip `RipgrepCore.xcframework.zip`). The binaryTarget `name:` is just a SwiftPM dependency identifier; consumers `import RipgrepCoreFFI` (the framework module), and the generated swift already hardcodes that.
   - Applied in Task 16 (single-slice) + Task 17 (5-slice `stage_framework*` fns).
   - **Phase 3 watch-out (Task 18/19):** `Sources/RipgrepKitFFI/` currently also holds the generated `RipgrepCoreFFI.h` + `RipgrepCoreFFI.modulemap`. The C module actually comes from the binaryTarget framework. Having a second `RipgrepCoreFFI` modulemap/header in the `RipgrepKitFFI` Swift target's source dir may collide with the framework's module at `swift build`. Decide at Task 18/19 whether `RipgrepKitFFI` should contain ONLY `RipgrepCore.swift` (and the .h/.modulemap should be excluded from that target / live only in the xcframework). Not Task 16's problem (Task 16 = build-only smoke test).

13. **PLAN ERRATA — `testSingleQuotes` is self-contradictory with the Tokenizer algorithm (resolved 2026-05-16).** Plan Task 27 test: `tokenize("'don\\'t' x") == ["don\\'t", "x"]` (input chars `'don\'t' x`). The plan's Tokenizer treats `\` inside single quotes as LITERAL (guard `ch == "\\" && !inSingle`), so the middle `'` closes the quote and the trailing `'` reopens it → ends with `inSingle=true` → throws "unbalanced quote". A single-quote nested in single-quotes is POSIX-invalid; throwing is actually correct. The plan's algorithm is SOUND for every real downstream use (Task 29: `-g '*.swift'`→`*.swift`; spec's core `-S 'func\s+\w+'`→regex with backslashes preserved). **Resolution: keep Tokenizer.swift EXACTLY as the plan; replace `testSingleQuotes` with a well-defined test of the design-critical property** (single-quoted content literal incl. backslashes):
    ```swift
    func testSingleQuotes() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"'func\s+\w+' x"#), ["func\\s+\\w+", "x"])
    }
    ```
    (Raw-string input chars `'func\s+\w+' x` → algorithm yields `["func\s+\w+", "x"]`; this is the exact regex-preservation behavior the tool exists for.) Implementer escalated correctly (NEEDS_CONTEXT); controller decided. Other 7 Task-27 tests are consistent with the plan algorithm and unchanged.

## External adversarial review (ccd / DeepSeek) — findings & disposition (2026-05-16, pre-PR)

Run at the finishing-a-development-branch checkpoint (user CLAUDE.md gate; `/codex:adversarial-review` unavailable → used the available `ccd-review`). Reviewed the 34 hand-written Phase 2-6 source files. **Verdict: Critical = none.** Dispositions (per `superpowers:receiving-code-review`):

- **FIXED (in-scope Phase 3):** Nit — stale `// Filled in later tasks.` comment in `Sources/RipgrepKitCore/Ripgrep.swift` replaced with a proper namespace-enum doc comment (the comment was factually wrong post-completion).
- **SECURITY NOTE (v0.1.0 — document in README/Task 37):** user-supplied regex is passed straight to `grep-regex`; a malicious pattern (e.g. `(a+)+b`) can cause catastrophic backtracking. The ONLY mitigation is the `timeout`/`CancelToken`. **Consumers handling untrusted input (esp. the LLM-tool path) MUST set `Options.timeout`.** No code defect (inherent to regex search); must be documented prominently. Path-traversal: none (ignore::WalkBuilder, no symlink follow). Command-injection: none (Tokenizer is in-process, no shell).
- **DISAGREE (no change, reasoning recorded):** ccd flagged `search_blocking` as "duplicated" in `lib.rs` (`#[uniffi::export]`) vs `search.rs` (impl). This is a *deliberate* Task 14 design — a 1-line thin re-export wrapper (no logic duplication) keeping the FFI signature concern out of the core impl; explicitly validated by Task 14's spec + code-quality reviews. Keeping as-is.
- **v0.2 BACKLOG (pre-existing Phase 1 code, OUT of Phase 2-6 PR scope; correct observations, NOT v0.1.0 blockers):**
  1. `sink.rs` context-group merge: adjacent same-file matches' `after_context` all attribute to the last match (extends the already-documented `formattedAsText` overlap limitation — root cause is in `sink.rs`, not just the Swift formatter). v0.2: per-match after-line collection.
  2. `sink.rs` cancel returns `Ok(false)` (skip line) not `Err(SinkAbort)`; large single-file cancel latency bounded only by walker file-boundary checks + `CANCEL_CHECK_EVERY=100`. v0.2: flush + `SinkAbort` on cancel.
  3. `SearchRequest.timeout_ms` is carried over FFI but unread in Rust `search_blocking` (timeout is intentionally CancelToken-driven; Swift `Options.toFFI` sets it, `Ripgrep.search` builds the `CancelToken`). This is the documented dual-timeout design — sharpened here: the Rust-side field is intentionally inert; v0.2 may drop it or comment it in `options.rs`.
  4. `search.rs` match/file counters use `Ordering::Relaxed` → parallel overshoot (wasted work only; RESULT correctness is guaranteed by Errata #7's `>= max` post-truncation, verified deterministic 86×). v0.2: Acquire/Release.
  5. Rust test `timeout_marks_result_cancelled` is misnamed (uses pre-expired `CancelToken::new(Some(0))`, not `timeout_ms`). The real timeout→CancelToken e2e path IS covered by Task 26 Swift `CancellationTests`. v0.2: rename + add a Rust-side e2e.
  6. `CANCEL_CHECK_EVERY=100` may be coarse for pathological single-line giant files; `Submatch.start/end` are `u32` (theoretical >4GB-line truncation — unrealistic). v0.2 notes only.
  7. `lib.rs` flattens many `#[cfg(test)] mod`s in one file — v0.2 cosmetic (extract to test files).

None of the v0.2 items is a correctness defect for the v0.1.0 dev-form milestone (truncation correctness proven; cancellation e2e covered at the Swift layer; security mitigation exists via timeout and is now documented). They are pre-existing Phase-1 characteristics, not regressions from Phases 2-6.

## PR #1 review (GitHub Copilot bot) — dispositions (2026-05-16)

Copilot review: COMMENTED (non-blocking), 6 inline comments. Per `superpowers:receiving-code-review`:

1. **Dead `Tests/RipgrepKitCoreTests/Stub.swift` + `Tests/RipgrepKitToolTests/Stub.swift`** → **FIXED** (deleted; both targets now have real tests; removes 2 no-op `XCTAssertTrue(true)`). Test count 48 → 47 (−2 stubs +1 new pin test).
2. **Empty quoted token `""`/`''` → empty-string token** → algorithm UNCHANGED (pushed back: it's intentional shell-faithful behavior — dropping it would silently swallow `rg -g ''`, masking user error, the *opposite* of the reviewer's concern). Accepted the reviewer's other option: **added `testEmptyQuotedProducesEmptyToken`** pinning the semantics.
3. **`Options.toFFI()` `precondition` traps host on negative Codable-decoded values** → **RESOLVED — user chose "switch to throwing now"** (52142b0→`<follow-up commit>`). `toFFI` is now `throws(Ripgrep.Error)`: the 5 `precondition`s became `guard … else { throw .invalidArguments(...) }`; `Search.swift` calls `try options.toFFI(...)` (search is already `async throws`, throws synchronously before any FFI/walk). New public-contract test `SearchTests.testNegativeOptionsThrowsInvalidArguments`. This supersedes the earlier "pre-v0.1.0 decision point" — the precondition-vs-throwing question is now decided (throwing) and no longer deferred. Done pre-tag so it is NOT a source-breaking change.
4. **Double `invalid regex:` prefix** (Deviation #15) → **FIXED** in `Error.swift`: `.invalidPattern` now `p.hasPrefix("invalid regex:") ? p : "invalid regex: \(p)"` (FFI flat_error supplies the full Display; idempotent guard keeps the bare-pattern path green so `ErrorTests` still passes).
5. **`parseFilesize` silently drops malformed values (`5X`, `-5M`) → silent "no limit"** → **FIXED** in `Parse.swift`: three-way semantics — absent → no limit; valid → bytes; present-but-unparseable → `throw Ripgrep.Error.invalidArguments`. `testMaxFilesizeParsing` updated to assert the error path (the old test had codified the footgun).

Verified after fixes: `swift test` 47/0-fail, 0 warnings; `cargo test -p ripgrep_core` 33; fmt clean. Deviation #15 is now RESOLVED (was deferred → fixed via PR feedback).

## Phase 2 Watch-outs (Task 14+)

- Task 14 adds `uniffi::setup_scaffolding!()`, `#[derive(uniffi::Record)]`, `#[derive(uniffi::Object)]` on CancelToken, `#[uniffi::export]` on a `search_blocking` wrapper. The CancelToken constructor changes to return `Arc<Self>` for UniFFI — adjust all Rust call sites (tests construct `CancelToken::new(None)`; if it becomes `Arc`, tests need `Arc`-aware updates).
- The plan's `build.rs` references a `.udl` file but the design uses proc-macro mode. The plan note says UDL is optional fallback — proc-macro `setup_scaffolding!()` is the primary path. Don't create a UDL unless bindgen demands it.
- `uniffi-bindgen`: do NOT `cargo install` it (broken — see Deviation #6). Use the in-tree bin via `cargo run --bin uniffi-bindgen`.
- Apple Rust targets: already installed (all 5, done 2026-05-16).
- Tasks 16-17 (xcframework) and 18+ (Swift) require Xcode toolchain; these are the highest-risk tasks — budget for fix-and-retry loops; prefer `sonnet` model for implementers there.

## Model Selection Used So Far

- Mechanical Rust tasks (1-4, 6, 9-13): `haiku` implementer + `haiku` reviewer
- Logic-bearing tasks (5, 7, 8, 12): `sonnet` implementer + `sonnet` reviewer
- Task 5 needed one fix-loop (dead tuple element flagged by reviewer, fixed, re-verified)
