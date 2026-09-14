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
        StoppableReader::new(std::io::repeat(b'a'), shared, None)
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

#[test]
fn cancel_after_the_walk_ended_reports_cancelled() {
    let dir = many_matches("late_cancel", 1, 100);
    let cancel = CancelToken::new(None);
    let session = SearchSession::start(limited_path(&dir.path()), Arc::clone(&cancel)).unwrap();
    assert_eq!(session.next_batch(1).unwrap().matches.len(), 1);
    wait_until(|| session.worker_finished());
    assert!(session.buffered() > 0);
    cancel.cancel();
    let last = session.next_batch(1024).unwrap();
    assert!(last.matches.is_empty());
    assert!(
        last.summary.unwrap().cancelled,
        "buffered matches were dropped"
    );
}

#[test]
fn reaching_the_limit_keeps_after_context_of_reserved_matches() {
    // Match 50 is sink event 99 and its context line event 100: the periodic stop
    // check (every 100 events) lands exactly between them.
    let dir = many_matches("limit_context", 1, 100);
    let res = search_blocking(limited(&dir, Some(50), 1), CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 50);
    for m in &res.matches {
        assert_eq!(
            m.after_context,
            vec!["filler".to_string()],
            "line {}",
            m.line_number
        );
    }
}

#[test]
fn reserved_matches_are_delivered_through_a_full_channel() {
    let dir = many_matches("full_channel", 1, 400);
    let session =
        SearchSession::start(limited(&dir, Some(257), 0), CancelToken::new(None)).unwrap();
    wait_until(|| session.buffered() == 256);
    // Longer than the send poll interval, so the 257th send has timed out and retried.
    std::thread::sleep(Duration::from_millis(150));
    let (streamed, summary) = drain(&session);
    assert_eq!(streamed.len(), 257);
    assert!(summary.truncated);
}

#[test]
fn submatch_collection_is_bounded_by_max_columns() {
    let dir = TempDir::new("dense");
    let file = dir.write("f.txt", format!("{}\n", "a".repeat(100_000)).as_bytes());
    let mut r = req("a", &file);
    r.max_columns = Some(100);
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    let m = &res.matches[0];
    assert!(m.line_truncated);
    assert!(
        m.submatches.len() <= 101,
        "{} submatches",
        m.submatches.len()
    );
}

#[test]
fn multiline_bom_file_reserves_decoding_budget() {
    let dir = TempDir::new("bom");
    // 350 KiB fits a 1 MiB budget alone, but not with the decoding reserve (3× + slack).
    let body = [b"HIT\n".as_slice(), &vec![b'x'; 350 * 1024]].concat();
    let plain = dir.write("plain.txt", &body);
    let bom = dir.write("bom.txt", &[[0xEF, 0xBB, 0xBF].as_slice(), &body].concat());
    let limits = Limits {
        multiline_budget: MIB,
        ..Limits::default()
    };
    for (path, expect_skip) in [(plain, false), (bom, true)] {
        let mut r = req("HIT", &path);
        r.multiline = true;
        let res = search_with_limits(r, CancelToken::new(None), limits).unwrap();
        assert_eq!(res.matches.is_empty(), expect_skip, "{path}");
        assert_eq!(
            !res.warnings.is_empty(),
            expect_skip,
            "{path}: {:?}",
            res.warnings
        );
    }
}

/// A reader that yields one line, then blocks until released.
struct GatedReader {
    first: Option<Vec<u8>>,
    gate: mpsc::Receiver<()>,
}

impl Read for GatedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(mut line) = self.first.take() {
            let n = line.len().min(buf.len());
            buf[..n].copy_from_slice(&line[..n]);
            let rest = line.split_off(n);
            if !rest.is_empty() {
                self.first = Some(rest);
            }
            return Ok(n);
        }
        let _ = self.gate.recv();
        Ok(0)
    }
}

#[test]
fn match_without_after_context_is_sent_before_the_file_ends() {
    use grep_regex::RegexMatcher;
    use grep_searcher::SearcherBuilder;
    let (tx, rx) = crossbeam_channel::unbounded();
    let (release, gate) = mpsc::channel();
    let searching = std::thread::spawn(move || {
        let matcher = RegexMatcher::new("HIT").unwrap();
        let mut sink =
            super::support::test_sink("gated", tx, CancelToken::new(None), matcher.clone());
        let reader = GatedReader {
            first: Some(b"HIT now\n".to_vec()),
            gate,
        };
        SearcherBuilder::new()
            .line_number(true)
            .build()
            .search_reader(&matcher, reader, &mut sink)
            .unwrap();
    });
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("match held until the file ended");
    assert_eq!(first.line, "HIT now");
    release.send(()).unwrap();
    searching.join().unwrap();
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "condition not reached");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Endless `xxx…\n` lines that count how many bytes were read.
struct CountingLines {
    read: Arc<std::sync::atomic::AtomicUsize>,
}

impl Read for CountingLines {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for (i, b) in buf.iter_mut().enumerate() {
            *b = if i % 100 == 99 { b'\n' } else { b'x' };
        }
        self.read
            .fetch_add(buf.len(), std::sync::atomic::Ordering::Relaxed);
        Ok(buf.len())
    }
}

fn bytes_read_by_search(shared: Arc<Shared>) -> usize {
    use grep_regex::RegexMatcher;
    use grep_searcher::SearcherBuilder;
    let read = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reader = StoppableReader::new(
        CountingLines {
            read: Arc::clone(&read),
        },
        shared,
        None,
    );
    let matcher = RegexMatcher::new("HIT").unwrap();
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut sink =
        super::support::test_sink("endless", tx, CancelToken::new(None), matcher.clone());
    let read_for_thread = Arc::clone(&read);
    within(Duration::from_secs(10), move || {
        SearcherBuilder::new()
            .line_number(true)
            .build()
            .search_reader(&matcher, reader, &mut sink)
            .unwrap();
        read_for_thread.load(std::sync::atomic::Ordering::Relaxed)
    })
}

#[test]
fn stopped_reader_ends_the_search_not_just_one_read() {
    let cancel = CancelToken::new(None);
    cancel.cancel();
    let cancelled = Arc::new(Shared::new(cancel, None, Limits::default()));
    let at_limit = Arc::new(Shared::new(
        CancelToken::new(None),
        Some(0),
        Limits::default(),
    ));
    for (label, shared) in [("cancel", cancelled), ("limit", at_limit)] {
        let read = bytes_read_by_search(shared);
        assert!(
            read <= 4 * MIB,
            "{label}: read {read} bytes before stopping"
        );
    }
}

#[test]
fn multiline_bom_files_waiting_for_budget_do_not_deadlock() {
    let dir = TempDir::new("bom_wait");
    let body = [
        [0xEF, 0xBB, 0xBF].as_slice(),
        b"HIT\n",
        &vec![b'x'; 200 * 1024],
    ]
    .concat();
    for i in 0..8 {
        dir.write(&format!("f{i}.txt"), &body);
    }
    let path = dir.path();
    let res = within(Duration::from_secs(20), move || {
        let mut r = req("HIT", &path);
        r.multiline = true;
        let limits = Limits {
            multiline_budget: MIB,
            threads: 8,
            ..Limits::default()
        };
        search_with_limits(r, CancelToken::new(None), limits).unwrap()
    });
    assert_eq!(res.matches.len(), 8, "{:?}", res.warnings);
}

#[test]
fn multiline_submatches_survive_when_no_row_is_cut() {
    let dir = TempDir::new("ml_subs");
    let file = dir.write("f.txt", "ab ab\n".repeat(50).as_bytes());
    let mut r = req(r"b\s", &file);
    r.multiline = true;
    r.max_columns = Some(10);
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    let total: usize = res.matches.iter().map(|m| m.submatches.len()).sum();
    assert!(res.matches.iter().all(|m| !m.line_truncated));
    assert_eq!(total, 100);
}
