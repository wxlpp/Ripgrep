use super::cancel::CancelToken;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn fresh_token_not_cancelled() {
    let t = CancelToken::new(None);
    assert!(!t.is_cancelled());
}

#[test]
fn explicit_cancel_trips_token() {
    let t = CancelToken::new(None);
    let t2 = Arc::clone(&t);
    thread::spawn(move || t2.cancel());
    thread::sleep(Duration::from_millis(20));
    assert!(t.is_cancelled());
}

#[test]
fn deadline_trips_token_after_elapse() {
    let t = CancelToken::new(Some(10));
    thread::sleep(Duration::from_millis(40));
    assert!(t.is_cancelled());
}

#[test]
fn no_deadline_means_never_auto_cancel() {
    let t = CancelToken::new(None);
    thread::sleep(Duration::from_millis(20));
    assert!(!t.is_cancelled());
}
