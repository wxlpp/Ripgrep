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

**DONE — Task 18 (Package.swift + local binaryTarget).** Commit `830b8d1`. Spec ✅ + code-quality ✅ (manifest verbatim per plan). swift-argument-parser pinned **1.7.1**; `swift package resolve` works with no source dirs (Task 19 ordering safe). Plus build-script correctness follow-up `7629dcc` (below).

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

**NEXT: Task 23** (Ripgrep.search wiring — async/cancellation/FFI conversion; the Sendable boundary task). Tasks 36-38 deferred. Task 35 release-time-only.

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
