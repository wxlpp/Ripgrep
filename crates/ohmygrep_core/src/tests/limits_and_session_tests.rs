use super::cancel::CancelToken;
use super::error::OhMyGrepError;
use super::options::{SearchMatch, SearchRequest};
use super::search::{search_blocking, search_with_limits};
use super::session::SearchSession;
use super::shared::{Limits, Shared, StoppableReader};
use super::sink::TEST_PANIC_MARKER;
use super::support::{req, TempDir};
use std::io::Read;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MIB: usize = 1 << 20;

/// `files` files with `per_file` matching lines each.
fn many_matches(tag: &str, files: usize, per_file: usize) -> TempDir {
    let dir = TempDir::new(tag);
    let body: String = (0..per_file)
        .map(|i| format!("HIT {i}\nfiller\n"))
        .collect();
    for f in 0..files {
        dir.write(&format!("f{f:03}.txt"), body.as_bytes());
    }
    dir
}

/// Runs `f` on a thread and fails the test instead of hanging.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).expect("timed out (hang?)")
}

fn sorted_keys(matches: &[SearchMatch]) -> Vec<(String, u64)> {
    let mut keys: Vec<_> = matches
        .iter()
        .map(|m| (m.path.clone(), m.line_number))
        .collect();
    keys.sort();
    keys
}

fn limited(dir: &TempDir, max: Option<u32>, after: u32) -> SearchRequest {
    let mut r = req("HIT", &dir.path());
    r.max_matches = max;
    r.after_context = after;
    r
}

#[test]
fn match_limit_is_exact_under_parallel_walk() {
    let dir = many_matches("limit", 50, 10);
    for after in [0, 1] {
        let res = search_blocking(limited(&dir, Some(7), after), CancelToken::new(None)).unwrap();
        assert_eq!(res.matches.len(), 7, "after_context={after}");
        assert!(res.truncated);
    }
    let all = search_blocking(limited(&dir, Some(1000), 0), CancelToken::new(None)).unwrap();
    assert_eq!((all.matches.len(), all.truncated), (500, false));
    let exact = search_blocking(limited(&dir, Some(500), 0), CancelToken::new(None)).unwrap();
    assert_eq!((exact.matches.len(), exact.truncated), (500, true));
}

#[test]
fn zero_match_limit_returns_nothing() {
    let dir = many_matches("zero", 2, 2);
    let res = search_blocking(limited(&dir, Some(0), 0), CancelToken::new(None)).unwrap();
    assert!(res.matches.is_empty());
    assert!(res.truncated);
}

#[test]
fn long_line_stops_file_with_warning_after_reporting_earlier_matches() {
    let dir = TempDir::new("heap");
    let mut content = b"HIT early\n".to_vec();
    content.extend(vec![b'x'; 3 * MIB]);
    content.extend_from_slice(b"\nHIT late\n");
    let file = dir.write("long.txt", &content);
    let limits = Limits {
        line_heap: MIB,
        ..Limits::default()
    };
    let res = search_with_limits(req("HIT", &dir.path()), CancelToken::new(None), limits).unwrap();
    let lines: Vec<_> = res.matches.iter().map(|m| m.line.as_str()).collect();
    assert_eq!(lines, vec!["HIT early"]);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert_eq!(res.warnings[0].path, file);
    assert_eq!(
        res.warnings[0].message,
        "stopped: a line needs more than 1 MiB of memory; earlier matches in this file were reported"
    );
}

#[test]
fn multiline_file_over_budget_is_skipped() {
    let dir = TempDir::new("mlbudget");
    dir.write("small.txt", b"HIT\n");
    let big = dir.write(
        "big.txt",
        &[b"HIT\n".as_slice(), &vec![b'x'; 2 * MIB]].concat(),
    );
    let mut r = req("HIT", &dir.path());
    r.multiline = true;
    let limits = Limits {
        multiline_budget: MIB,
        ..Limits::default()
    };
    let res = search_with_limits(r, CancelToken::new(None), limits).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert_eq!(res.warnings[0].path, big);
    assert!(res.warnings[0]
        .message
        .starts_with("skipped: file needs more than 1 MiB"));
}

#[test]
fn multiline_files_share_the_budget_without_hanging() {
    let dir = TempDir::new("mlshare");
    for i in 0..8 {
        dir.write(
            &format!("f{i}.txt"),
            &[b"HIT\n".as_slice(), &vec![b'x'; 600 * 1024]].concat(),
        );
    }
    let path = dir.path();
    let res = within(Duration::from_secs(20), move || {
        let mut r = req("HIT", &path);
        r.multiline = true;
        let limits = Limits {
            multiline_budget: MIB,
            ..Limits::default()
        };
        search_with_limits(r, CancelToken::new(None), limits).unwrap()
    });
    assert_eq!(res.matches.len(), 8);
    assert!(res.warnings.is_empty(), "{:?}", res.warnings);
}

#[test]
fn panic_in_one_file_fails_the_search_instead_of_hanging() {
    let dir = many_matches("panic", 20, 5);
    dir.write(&format!("{TEST_PANIC_MARKER}.txt"), b"HIT\n");
    let path = dir.path();
    let err = within(Duration::from_secs(10), move || {
        search_blocking(req("HIT", &path), CancelToken::new(None)).unwrap_err()
    });
    assert!(matches!(err, OhMyGrepError::InternalPanic(_)), "{err:?}");

    let path = dir.path();
    let err = within(Duration::from_secs(10), move || {
        let session = SearchSession::start(req("HIT", &path), CancelToken::new(None)).unwrap();
        loop {
            match session.next_batch(64) {
                Ok(batch) if batch.summary.is_none() => continue,
                other => return other,
            }
        }
    });
    assert!(
        matches!(err, Err(OhMyGrepError::InternalPanic(_))),
        "{err:?}"
    );
}

fn drain(session: &SearchSession) -> (Vec<SearchMatch>, super::options::SearchSummary) {
    let mut all = Vec::new();
    loop {
        let batch = session.next_batch(32).unwrap();
        all.extend(batch.matches);
        if let Some(summary) = batch.summary {
            return (all, summary);
        }
    }
}

#[test]
fn session_returns_the_same_matches_as_one_shot() {
    let dir = many_matches("session", 30, 20);
    let one_shot = search_blocking(limited(&dir, None, 1), CancelToken::new(None)).unwrap();
    let session = SearchSession::start(limited(&dir, None, 1), CancelToken::new(None)).unwrap();
    let (streamed, summary) = drain(&session);
    assert_eq!(sorted_keys(&streamed), sorted_keys(&one_shot.matches));
    assert_eq!(summary.files_searched, 30);
    assert!(!summary.truncated && !summary.cancelled);
}

#[test]
fn session_honors_match_limit() {
    let dir = many_matches("session_limit", 30, 20);
    let session = SearchSession::start(limited(&dir, Some(25), 0), CancelToken::new(None)).unwrap();
    let (streamed, summary) = drain(&session);
    assert_eq!(streamed.len(), 25);
    assert!(summary.truncated);
}

#[test]
fn session_rejects_zero_batch_and_calls_after_finish() {
    let dir = many_matches("session_misuse", 1, 1);
    let session = SearchSession::start(limited(&dir, None, 0), CancelToken::new(None)).unwrap();
    assert!(matches!(
        session.next_batch(0),
        Err(OhMyGrepError::InvalidArguments(_))
    ));
    drain(&session);
    assert!(matches!(
        session.next_batch(1),
        Err(OhMyGrepError::InvalidArguments(_))
    ));
}

#[test]
fn cancelling_an_idle_consumer_returns_promptly_without_draining() {
    let dir = many_matches("session_cancel", 200, 200);
    let path = dir.path();
    let (buffered_after_cancel, summary, elapsed) = within(Duration::from_secs(10), move || {
        let cancel = CancelToken::new(None);
        let session = SearchSession::start(limited_path(&path), Arc::clone(&cancel)).unwrap();
        let first = session.next_batch(8).unwrap();
        assert!(first.summary.is_none());
        std::thread::sleep(Duration::from_millis(200)); // let the channel fill
        cancel.cancel();
        let started = Instant::now();
        let last = session.next_batch(1024).unwrap();
        (last.matches.len(), last.summary.unwrap(), started.elapsed())
    });
    assert_eq!(buffered_after_cancel, 0);
    assert!(summary.cancelled);
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
}

fn limited_path(path: &str) -> SearchRequest {
    let mut r = req("HIT", path);
    r.max_matches = None;
    r
}

#[test]
fn dropping_an_undrained_session_does_not_block() {
    let dir = many_matches("session_drop", 100, 200);
    let path = dir.path();
    within(Duration::from_secs(5), move || {
        let session = SearchSession::start(limited_path(&path), CancelToken::new(None)).unwrap();
        let _ = session.next_batch(1).unwrap();
        drop(session);
    });
}

#[test]
fn stoppable_reader_ends_an_endless_read_once_cancelled() {
    let cancel = CancelToken::new(None);
    let shared = Arc::new(Shared::new(Arc::clone(&cancel), None, Limits::default()));
    cancel.cancel();
    let read = within(Duration::from_secs(5), move || {
        let mut buf = Vec::new();
        StoppableReader::new(std::io::repeat(b'a'), shared)
            .read_to_end(&mut buf)
            .map(|_| buf.len())
            .unwrap()
    });
    assert!(read <= 2 * MIB, "read {read} bytes");
}

#[test]
fn max_columns_applies_to_match_and_context_lines() {
    let dir = TempDir::new("columns");
    let long = "y".repeat(50);
    let file = dir.write(
        "f.txt",
        format!("{long}\nxx HIT {long}\n{long}\n").as_bytes(),
    );
    let mut r = req("HIT", &file);
    r.max_columns = Some(20);
    r.before_context = 1;
    r.after_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    let m = &res.matches[0];
    assert!(m.line_truncated);
    assert_eq!(m.line.len(), 20);
    assert_eq!(
        &m.line[m.submatches[0].start as usize..m.submatches[0].end as usize],
        "HIT"
    );
    assert_eq!(m.before_context, vec!["y".repeat(20)]);
    assert_eq!(m.after_context, vec!["y".repeat(20)]);
}
