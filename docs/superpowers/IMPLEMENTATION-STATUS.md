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

**DONE — Task 14 (UniFFI scaffolding).** Commits `75412c0` (scaffolding) + `13517db` (review fixes). Spec ✅ + code-quality ✅. 33 Rust tests. Plus pre-flight commits `242dffd` (rustfmt sink.rs + Cargo.lock) and `0aafbac` (doc).

**NEXT: Task 15** (Generate Swift bindings) through Task 35. Tasks 36-38 deferred (fuzz/README/symlink-loop). Task 35 is release-time-only (no GitHub release yet).

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
