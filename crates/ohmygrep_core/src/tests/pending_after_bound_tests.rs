/// TDD for v0.3-perf-2: bound `pending_after` by evicting matured matches.
///
/// A match `m` is "matured" once `cur > m.line_number + after_context` — it can
/// receive no more After lines. Because matches are pushed in ascending line order
/// with constant `after_context`, matured ones form a prefix of `pending_after`.
/// `flush_matured(cur)` drains that prefix and sends each match (same action as
/// `flush_pending`'s per-match send) — called in `matched()` and `context()` After.
///
/// ## Behavior-preservation invariants tested
///
/// 1. **Attribution identical**: a matured match has already received every
///    after-context line in its window before eviction; eviction does not add or
///    remove any after-context line from any match.
/// 2. **Bound**: `pending_after.len()` never exceeds `after_context + 1` during the
///    run.  Asserted via `ChannelSink::max_pending_seen()` (test-only accessor).
/// 3. **Non-vacuity**: disabling `flush_matured` calls causes the bound assertion to
///    FAIL (Vec grows to N matches); attribution assertions still pass either way,
///    proving behavior-preservation while the bound is the perf property.
use super::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Build input: `n` matching lines (pattern "MATCH") followed by `after` non-matching
/// "tail_k" lines.  All match lines are back-to-back so pending_after accumulates
/// maximally without eviction, and the tail lines let the last match's window fill.
fn make_dense_input(n: usize, after: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    for i in 1..=(n as u64) {
        buf.extend_from_slice(format!("MATCH line {i}\n").as_bytes());
    }
    for j in 1..=after {
        buf.extend_from_slice(format!("tail_{j}\n").as_bytes());
    }
    buf
}

/// Full behavior-preservation test: 10 dense MATCH lines + 3 tail lines with
/// after_context=3.  Every match's after_context Vec must contain exactly the
/// non-match lines within its -A window.
///
/// Match lines: 1..=10 (all "MATCH line N").
/// Non-match lines: 11="tail_1", 12="tail_2", 13="tail_3".
///
/// Expected after_context for match at line k:
///   - Window covers lines k+1 ..= k+3.
///   - Match lines (1..=10) are emitted via matched(), NOT as after-context events,
///     so they do NOT appear in after_context.
///   - Only non-match lines (≥11) appear in after_context.
///   - k=1..7: k+1..k+3 are all within 1..=10 → after_context = []
///   - k=8: k+1=9 (match), k+2=10 (match), k+3=11 "tail_1" → after=["tail_1"]
///   - k=9: k+1=10 (match), k+2=11 "tail_1", k+3=12 "tail_2" → after=["tail_1","tail_2"]
///   - k=10: k+1=11 "tail_1", k+2=12 "tail_2", k+3=13 "tail_3"
///     → after=["tail_1","tail_2","tail_3"]
#[test]
fn dense_matches_attribution_is_identical_with_eviction() {
    const N: usize = 10;
    const AFTER: usize = 3;

    let input = make_dense_input(N, AFTER);
    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let matcher = RegexMatcher::new("MATCH").unwrap();
    let cancel = crate::cancel::CancelToken::new(None);

    let mut sink = ChannelSink::new(
        "synthetic-dense".into(),
        tx,
        cancel,
        Arc::clone(&counter),
        matcher.clone(),
        0,
        AFTER,
    );

    SearcherBuilder::new()
        .line_number(true)
        .after_context(AFTER)
        .build()
        .search_slice(&matcher, &input, &mut sink)
        .unwrap();

    assert_eq!(
        sink.pending_len(),
        0,
        "All matches must be flushed after search completes"
    );
    drop(sink);

    let mut results: Vec<_> = rx.iter().collect();
    results.sort_by_key(|m| m.line_number);

    assert_eq!(results.len(), N, "Expected {N} matches");
    assert_eq!(
        counter.load(Ordering::Relaxed),
        N,
        "match_counter must equal N"
    );

    for m in &results {
        // k = 1..=10; non-match lines (> N) are the expected after_context
        let k = m.line_number;
        let expected: Vec<String> = ((k + 1)..=(k + AFTER as u64))
            .filter(|&l| l > N as u64)
            .map(|l| format!("tail_{}", l - N as u64))
            .collect();
        assert_eq!(
            m.after_context, expected,
            "Match at line {k}: got {:?}, expected {:?}",
            m.after_context, expected
        );
    }
}

/// Bound test: verify `pending_after` never exceeds `after_context + 1` during the
/// search, using `ChannelSink::max_pending_seen()` (test-only accessor that tracks the
/// high-water mark of `pending_after.len()` at each push).
///
/// N=20 dense matches; without flush_matured the Vec would grow to 20.
/// With flush_matured it is bounded to at most AFTER+1=4 (one match can be pushed
/// before flush_matured fires for the now-matured front entries).
///
/// NON-VACUITY: temporarily comment out the two `flush_matured(...)` call sites in
/// sink.rs and rerun.  `max_pending_seen()` will return 20 (= N), failing this
/// assertion.  Restore the calls → passes.  The attribution test above passes BOTH
/// ways, proving eviction is behavior-preserving and the bound is the sole perf diff.
#[test]
fn pending_after_bounded_during_dense_search() {
    const N: usize = 20;
    const AFTER: usize = 3;

    let input = make_dense_input(N, AFTER);
    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let matcher = RegexMatcher::new("MATCH").unwrap();
    let cancel = crate::cancel::CancelToken::new(None);

    let mut sink = ChannelSink::new(
        "synthetic-bound".into(),
        tx,
        cancel,
        Arc::clone(&counter),
        matcher.clone(),
        0,
        AFTER,
    );

    SearcherBuilder::new()
        .line_number(true)
        .after_context(AFTER)
        .build()
        .search_slice(&matcher, &input, &mut sink)
        .unwrap();

    let max_seen = sink.max_pending_seen();

    // Without eviction: max_seen == N (== 20). With eviction: max_seen ≤ AFTER+1 (== 4).
    assert!(
        max_seen <= AFTER + 1,
        "BOUND VIOLATED: pending_after high-water mark = {} (expected ≤ {}). \
         Disable flush_matured() calls to see this fail at N={}.",
        max_seen,
        AFTER + 1,
        N
    );

    drop(sink);
    let results: Vec<_> = rx.iter().collect();
    assert_eq!(results.len(), N, "All {N} matches must be collected");
    assert_eq!(counter.load(Ordering::Relaxed), N);
}
