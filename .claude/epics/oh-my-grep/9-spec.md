# Issue #9 spec (rev 2): memory limits and streaming search

Epic: oh-my-grep (#5). Risk: high (concurrency across FFI, new public API). Rev 2 addresses design review round 1 (BLOCK: B1–B4, I1–I11, M1–M7).

Baseline (macOS release, Rust side only): 40 MB log, 400k matches → 227 MB peak RSS one-shot, 0.14 s; no-match baseline 53 MB.

## Goals
1. **Bounded search buffers in every mode.** Total heap used by per-thread line/multiline buffers is capped by a fixed budget, independent of file sizes and line lengths.
2. **Bounded output per line.** Long match/context lines are cut; the cut keeps the matched region visible.
3. **Bounded result size by default.** `maxMatches` defaults to 10 000 on both the Swift API and the tool.
4. **Streaming** whose retained memory does not grow with match count, with back-pressure, prompt cancellation and no hangs on panics.

## Non-goals
Grouped-by-file FFI model; `AsyncSequence` wrapper (can layer on the closure API later); rg per-file `-m` semantics (documented in #12); mmap.

## A. Engine limits (Rust)

### A1. Thread count and buffer budget
- `const BUFFER_BUDGET: usize = 128 MiB`; `threads = min(available_parallelism, 8)` set explicitly on `WalkBuilder::threads`.
- Every `Searcher` gets `heap_limit(Some(BUFFER_BUDGET / threads))` (≥ 16 MiB) in **all** modes, so the line buffer and the multiline buffer are both bounded (`multiline=true` with a non-multiline matcher also uses the line buffer and the same limit). Worst case buffers = 128 MiB total.
- grep-searcher reports the limit before reaching EOF when the buffer is full, so a file/line needing ≥ the per-thread limit is skipped. The error (`configured allocation limit (N) exceeded`) is mapped to a per-file warning: `skipped: line or multiline match needs more than {N} MiB of memory`.
- The limits live in an internal `Limits { buffer_budget, threads }` passed into `prepare`, so tests use small values (M6).

### A2. Panic containment inside the walk (B2)
- Each visitor call (one file) runs under `catch_unwind`. On panic: store `OhMyGrepError::InternalPanic(msg)` in a shared `Mutex<Option<_>>` (first wins), set a `stop` flag, return `WalkState::Quit`. Every worker checks `stop` first.
- `run` returns `Err(InternalPanic)` if the slot is set. A test-only hook (`cfg(test)` static flag) makes the sink panic on a chosen line; the test asserts an error within a timeout for both one-shot and session.
- All `Mutex` locks use `unwrap_or_else(PoisonError::into_inner)`.

### A3. Cancellation latency (I11)
- Files are opened and searched with `search_reader` through a `CancellableReader` that checks the token every 1 MiB read and reports EOF when cancelled, so cancellation does not wait for the end of a large file without matches. Walker workers also check the token before each entry (existing).

### A4. Match limit (I5)
- The sink reserves a slot with a CAS on the shared counter **after** the named-binary early return. If the reservation fails it sets `stop`, marks `truncated`, and returns `Ok(false)`. Walker workers `Quit` as soon as the counter reaches the limit (no search for a 10 001st match).
- `truncated` keeps its current meaning: the limit was reached (it may be that nothing more existed). `max_matches == 0` returns no matches and `truncated == true`. One-shot no longer needs post-hoc truncation.

### A5. Pending slot (I6)
- With `after_context == 0` a match is sent immediately; the pending slot is only used when after-context is requested.

### A6. Column cut (I2, I3, M2, M3)
- `SearchRequest.max_columns: Option<u32>`. Applies to the terminator-free bytes of each match line and context line.
- Single-line match longer than the limit: return a window of `max_columns` bytes. Window start = `max(0, first_submatch.start - max_columns / 4)`, clamped so the window fits, then moved forward to a character start by skipping at most 3 continuation bytes; window end moved back by at most 3 continuation bytes. `SearchMatch.line_offset: u32` = byte offset of the window start within the original line; submatches are rebased to the window, clamped to it, and those entirely outside are dropped. `SearchMatch.line_truncated: bool` set.
- Multiline match: each row cut to a prefix (row count preserved for line arithmetic); `line_offset = 0`; submatches kept only up to the first cut, the one crossing it clamped.
- Context lines: prefix cut, no flag.
- Offsets are byte offsets into the raw (pre-lossy-decode) line bytes; documented, with the invalid-UTF-8 caveat (#12).
- Swift: `Options.maxColumns: Int? = 4096` (nil = unlimited, must be ≥ 1 otherwise `invalidArguments`, and ≤ UInt32.max); tool `-M, --max-columns N` with `0` = unlimited (rg). `Match.lineTruncated` and `Match.lineOffset` with init defaults (`false`, `0`) and tolerant decoding.

## B. Shared core
- `prepare(req, limits) -> Result<Prepared, OhMyGrepError>`: matcher, walker builder, validation (all request errors, synchronously).
- `Prepared::run(self, cancel, tx: Sender<SearchMatch>) -> Result<SearchSummary, OhMyGrepError>`, `SearchSummary { truncated, cancelled, files_searched, elapsed_ms, warnings }` (UniFFI record).
- One-shot `search_blocking`: prepare, run with an unbounded channel, collect, sort (path, line).

## C. Rust `SearchSession` (UniFFI object) (I7, B4, M5, M7)
```rust
#[derive(uniffi::Record)] pub struct SearchBatch { pub matches: Vec<SearchMatch>, pub summary: Option<SearchSummary> }

#[uniffi::export]
impl SearchSession {
    #[uniffi::constructor]
    pub fn start(request: SearchRequest, cancel: Arc<CancelToken>) -> Result<Arc<Self>, OhMyGrepError>;
    pub fn next_batch(&self, max: u32) -> Result<SearchBatch, OhMyGrepError>;
}
```
- `start`: `prepare` synchronously; spawn the walk with `std::thread::Builder::spawn` (spawn failure → `Io` error); the thread runs `run` and stores its `Result` in a slot before dropping its sender.
- Channel `crossbeam_channel::bounded(256)`. The sink sends with `send_timeout(50 ms)` in a loop, re-checking cancel/stop; on cancel or a disconnected receiver it returns `Ok(false)`.
- `next_batch(max)`: `max == 0` → `InvalidArguments`. If the token is cancelled → drain nothing, join the worker (it exits promptly: token + CancellableReader + send loop) and return `{ matches: [], summary }` with `cancelled = true`. Otherwise block for the first match or disconnect, then take up to `max` without blocking. On disconnect: join and return the stored result: `summary: Some(..)` or the run error. After the final batch, further calls return `InvalidArguments("session finished")`. Whole body under `panic_safe`.
- `Drop`: cancel only, never join (Swift may release on a cooperative thread).
- Order: interleaved across files; per-file line order only (I9). `elapsed_ms` includes time blocked on the consumer.

## D. Swift API (B3, B4, I4, I8, M4)
```swift
extension OhMyGrep {
    public struct Summary: Codable, Sendable { truncated, cancelled, filesSearched, elapsed, warnings }

    /// Streams matches as they are found. `onMatch` is awaited before more are fetched, so a slow
    /// consumer slows the search instead of buffering. Matches from different files interleave.
    @discardableResult
    public static func stream(pattern: String, in paths: [String], options: Options = .init(),
                              onMatch: @Sendable (Match) async throws -> Void) async throws -> Summary
}
```
- The `CancelToken` is created before anything else; `withTaskCancellationHandler { … } onCancel: { token.cancel() }` wraps the whole call.
- A private serial `DispatchQueue` per call runs `SearchSession.start` and each blocking `nextBatch(256)`; results are converted to `Match` on that queue before resuming a checked continuation (each continuation resumed exactly once on every path). QoS from `Task.currentPriority`.
- Loop: `if Task.isCancelled` → stop consuming and fetch the final batch (cancelled summary); for each match call `onMatch`, checking `Task.isCancelled` between matches. An error thrown by `onMatch` cancels the token, fetches the final batch on the queue (so the worker exits), then rethrows.
- Tool: text output gains a final line `results truncated at N matches` when `truncated`; tool defaults = API defaults (`maxMatches` 10 000 unless `-m`, `maxColumns` 4096 unless `-M`, `-M 0` = unlimited).

## Tests
- Rust: window cut (match in middle of 1 MB line, multibyte at both edges, invalid UTF-8 run), multiline per-row cut, context cut; limit exactness under parallel walk (many files) and early quit; `max_matches = 0`; heap-limit warning with a small test budget (single-line and multiline); panic hook → error without hang (one-shot + session, with timeout); session equals one-shot set (no limit); `next_batch(0)`; cancel with an idle consumer returns promptly (timeout-guarded); drop without draining; after-context-less immediate send (first batch arrives before the file finishes: 50 MB file, match on line 1, assert first batch within a short time while cancel stops the rest); CancellableReader stops a no-match 200 MB read quickly after cancel.
- Swift: stream equals search set; summary warnings/truncated/cancelled; Task cancel mid-stream stops and returns cancelled summary; `onMatch` throw propagates and later searches still work; defaults and decoding (`maxMatches` missing → 10 000, `null` → nil; `maxColumns`); tool `-M`, truncation notice.

## Measurements (PR body)
- Rust release example: one-shot wall time on the 40 MB log (≤ 0.154 s); session drain peak RSS on the same log with `max_matches: None` (< 53 MB + 64 MB); multiline over 8 × 30 MB files (peak buffers ≤ budget); single 500 MB line file (skipped with warning, RSS bounded).
- Swift (macOS test gated by `OHMYGREP_PERF=1`): `phys_footprint` delta while draining `stream` over the 40 MB log vs one-shot `search` with `maxMatches: nil`.

## Rev 3 amendments (design review round 2: REVISE, no blockers; supersede conflicting text above)

- **A1 → buffers.** Line mode (`multiline == false`): fixed per-thread `heap_limit(16 MiB)` (not core-dependent); worst case `threads × 16 MiB ≤ 128 MiB`. Multiline mode: the visitor reads the file itself and calls `search_slice`. Before reading it reserves `metadata.len()` bytes from a global `AtomicUsize` budget of 128 MiB (CAS); the read is capped at the reservation (`take(reserved + 1)`; growth past it → warning and skip). A file larger than the whole budget is skipped with warning `skipped: file needs more than 128 MiB for multiline search`; when the budget is temporarily exhausted the worker waits in 5 ms steps, polling cancel/abort/limit. The reservation is released after the search. Measurements: multiline over 8 × 30 MB files (all searched, peak buffers ≤ 128 MiB) and one 200 MB file (skipped with warning).
- **A2 panic.** `catch_unwind(AssertUnwindSafe(..))` around the per-file work; the panicking thread's `Searcher`/sink are dropped and never reused. The test hook panics only for a path containing a unique marker (no global flag). A worker `JoinHandle::join` error with an empty result slot maps to `InternalPanic`.
- **A4 flags.** Two shared flags: `abort` (panic or cancel) and `limit_reached` (set by the CAS that makes the counter reach `max`). The send loop gives up only on cancel/abort/disconnect, never on `limit_reached`, so every reserved match is delivered. Walker workers quit on either flag; `CancellableReader` reports EOF on cancel, `abort` or `limit_reached`. `truncated := max.is_some_and(|m| counter >= m)`.
- **Search errors mid-file.** When `search_reader`/`search_slice` returns an error, the visitor flushes the sink's pending match and emits its binary warning (the searcher skips `finish` on errors), then adds the warning `stopped: {message}; earlier matches in this file were reported`. The heap-limit text is recognised by prefix `configured allocation limit` (locked by a test) and rendered as `stopped: a line needs more than 16 MiB of memory`.
- **A6 window.** After boundary adjustment `end = max(end, start)`. With no submatch (context lines, or a match whose submatches lie beyond a multiline cut) the window is the prefix.
- **D Swift cancellation.** On observing `Task.isCancelled` the loop calls `token.cancel()` synchronously before dispatching the final `nextBatch`. If a summary was already received (natural end) the loop stops calling `onMatch` and returns that summary with `cancelled = true`. Any error thrown by `onMatch` (including `CancellationError`) cancels the token, drains the final batch, and is rethrown.
- **Tool.** JSON-lines output appends `{"truncated": {"limit": N}}` when truncated. `-m 0` returns no matches (rg); unlimited matches are only available through the Swift API (`maxMatches: nil`) — documented in #12.
- **Docs (#12).** Cancelled one-shot results may contain a partial last line; `maxColumns: nil` lets a session retain up to 256 very long matches; `elapsed` includes consumer back-pressure; measurement machine core count recorded.
