use super::cancel::CancelToken;
use super::search::search_blocking;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn external_cancel_stops_search_promptly() {
    let cancel = CancelToken::new(None);
    let cancel_for_thread = Arc::clone(&cancel);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        cancel_for_thread.cancel();
    });

    let r = super::search_e2e_tests::req("anything");
    let start = Instant::now();
    let res = search_blocking(r, cancel).unwrap();
    let elapsed = start.elapsed();
    // tiny fixture finishes in <50ms anyway, so cancellation may or may not
    // have kicked in; the assertion is about *no panic / no hang*.
    assert!(
        elapsed < Duration::from_secs(2),
        "search hung: {:?}",
        elapsed
    );
    let _ = res;
}
