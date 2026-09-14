use super::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

/// Build an input of `n` lines each consisting of the pattern repeated to
/// fill at least `line_bytes` bytes, terminated by a newline.
fn make_giant_lines(n: usize, line_bytes: usize) -> Vec<u8> {
    // Each line: "match" repeated to fill line_bytes, then '\n'
    let pattern = b"match";
    let repeats = line_bytes / pattern.len();
    let chunk: Vec<u8> = pattern.repeat(repeats);
    let mut buf = Vec::with_capacity(n * (chunk.len() + 1));
    for _ in 0..n {
        buf.extend_from_slice(&chunk);
        buf.push(b'\n');
    }
    buf
}

/// With a pre-tripped cancel token and 3 giant lines (each ≥ 2 MiB — well
/// above CANCEL_CHECK_BYTES = 1 MiB), the byte accumulator trips on the
/// very first matched line and the search stops early (< 3 matches).
///
/// OLD event-only cadence would NOT poll within 3 events (CANCEL_CHECK_EVERY
/// = 100), so all 3 lines would be processed → 3 matches → test FAILS.
/// Byte-cadence cadence fires at line 1 → 1 match (or 0 if cancel checked
/// before the first match is committed) → test PASSES.
#[test]
fn byte_cadence_stops_on_giant_lines() {
    // 3 lines × 2 MiB each = 6 MiB total; each line alone exceeds the 1 MiB threshold.
    let line_size = 2 * 1024 * 1024; // 2 MiB
    let input = make_giant_lines(3, line_size);

    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let matcher = RegexMatcher::new("match").unwrap();

    // Pre-tripped cancel token.
    let cancel = crate::cancel::CancelToken::new(Some(0));

    let mut sink = ChannelSink::new(
        "synthetic-giant".into(),
        tx,
        cancel,
        Arc::clone(&counter),
        matcher.clone(),
        0,
        0,
    );

    let result = SearcherBuilder::new()
        .line_number(true)
        .build()
        .search_slice(&matcher, &input, &mut sink);

    // Ok(false) cancel path → Ok(()) from search_slice.
    assert!(
        result.is_ok(),
        "Expected Ok(()) from graceful Ok(false) cancel, got: {:?}",
        result
    );

    drop(sink);
    let collected: Vec<_> = rx.iter().collect();

    // KEY ASSERTION: search stopped early — fewer than 3 lines processed.
    // With byte-cadence: first line (2 MiB) exceeds CANCEL_CHECK_BYTES (1 MiB)
    // → cancel checked → stops. Collected count < 3.
    //
    // With OLD event-only cadence: 3 events < CANCEL_CHECK_EVERY (100)
    // → cancel never polled → all 3 lines processed → collected == 3 → FAILS.
    assert!(
        collected.len() < 3,
        "FAIL: byte-cadence did not stop early — got {} matches (expected < 3). \
         Old event-only cadence would process all 3 giant lines without polling cancel.",
        collected.len()
    );
}
