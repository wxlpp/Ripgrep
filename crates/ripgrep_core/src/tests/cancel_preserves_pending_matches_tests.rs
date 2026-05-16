use super::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

/// Build input with `n` matching lines so the cancel-check cadence (every 100
/// events) is guaranteed to trigger at least once mid-search, leaving some
/// matches already collected in pending_after when the cancel fires.
fn make_input(n: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(n * 8);
    for i in 0..n {
        buf.extend_from_slice(format!("match {i}\n").as_bytes());
    }
    buf
}

/// Primary assertion: with a pre-tripped token the search stops at the first
/// cancel-check (event 100), finish() is still called (Ok(false) contract),
/// flush_pending() runs, and the already-collected matches are delivered on the
/// channel. The result is non-empty and well-formed (not lost).
///
/// Non-vacuity (see task instructions): temporarily changing `return Ok(false)`
/// to `return Err(SinkAbort)` in matched() causes finish() to be skipped,
/// flush_pending() never runs, and this test FAILS (0 matches on channel).
/// That verifies the test actually guards the finish/flush-preserves-pending
/// property and is not vacuously passing.
#[test]
fn cancel_ok_false_preserves_matches_collected_before_cancel() {
    // 150 matching lines — more than CANCEL_CHECK_EVERY (100), so the pre-tripped
    // token is guaranteed to trip on event 100, with 99 matches already pending.
    let input = make_input(150);
    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let matcher = RegexMatcher::new("match").unwrap();

    // Pre-tripped cancel token (deadline = now + 0ms → already past).
    let cancel = crate::cancel::CancelToken::new(Some(0));

    let mut sink = ChannelSink::new(
        "synthetic".into(),
        tx,
        cancel,
        Arc::clone(&counter),
        matcher.clone(),
        0, // before_context
        0, // after_context
    );

    // search_slice returns Ok(()) when the Sink returns Ok(false) (graceful stop);
    // it returns Err(SinkAbort) only if the Sink returns Err(_).
    let search_result = SearcherBuilder::new()
        .line_number(true)
        .build()
        .search_slice(&matcher, &input, &mut sink);

    // With Ok(false): grep_searcher calls finish() → flush_pending() → channel filled.
    // search_result is Ok(()) (graceful stop, not an error propagation).
    assert!(
        search_result.is_ok(),
        "Expected Ok(()) from graceful Ok(false) cancel, got: {:?}",
        search_result
    );

    drop(sink); // close channel sender
    let collected: Vec<_> = rx.iter().collect();

    // KEY ASSERTION: matches collected before the cancel-check (event 100) must
    // be present because finish() flushed them. With Err(SinkAbort) this would
    // be 0 (finish() skipped), proving the test is not vacuous.
    assert!(
        !collected.is_empty(),
        "REGRESSION: cancel path lost pending matches — finish()/flush_pending() did not run. \
         This indicates the cancel return was changed from Ok(false) to Err(SinkAbort)."
    );

    // Sanity: we stopped before processing all 150 lines (cancel tripped at ~100).
    assert!(
        collected.len() < 150,
        "Expected search to stop before all 150 lines, got {} matches (cancel did not trip?)",
        collected.len()
    );

    // Sanity: matches are well-formed (line numbers present, content correct).
    for m in &collected {
        assert!(
            m.line.contains("match"),
            "unexpected line content: {:?}",
            m.line
        );
        assert!(
            m.line_number > 0,
            "line_number should be >0 with line_number(true)"
        );
    }
}

/// Verify the error type distinction: if the cancel path were changed to
/// Err(SinkAbort), grep_searcher would propagate the error and search_slice
/// would return Err(_), not Ok(()). This test documents that the current code
/// (Ok(false)) produces Ok(()) from search_slice — another observable difference.
#[test]
fn cancel_ok_false_search_slice_returns_ok_not_err() {
    let input = make_input(150);
    let (tx, _rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let matcher = RegexMatcher::new("match").unwrap();
    let cancel = crate::cancel::CancelToken::new(Some(0));

    let mut sink = ChannelSink::new(
        "synthetic".into(),
        tx,
        cancel,
        Arc::clone(&counter),
        matcher.clone(),
        0,
        0,
    );

    let result: Result<(), _> = SearcherBuilder::new()
        .line_number(true)
        .build()
        .search_slice(&matcher, &input, &mut sink);

    // Ok(false) from the Sink → grep_searcher stops gracefully → Ok(())
    // Err(SinkAbort) from the Sink → grep_searcher propagates → Err(SinkAbort)
    assert!(
        result.is_ok(),
        "search_slice should return Ok(()) when cancel uses Ok(false), got Err"
    );
}
