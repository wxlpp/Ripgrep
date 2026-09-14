use super::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

#[test]
fn sink_emits_match_per_line() {
    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    let m = RegexMatcher::new("foo").unwrap();
    let mut sink = ChannelSink::new(
        "/tmp/test.txt".into(),
        tx,
        crate::cancel::CancelToken::new(None),
        Arc::clone(&counter),
        m.clone(),
        0,
    );
    let body = b"foo\nbar\nfoo\n";
    SearcherBuilder::new()
        .build()
        .search_slice(&m, body, &mut sink)
        .unwrap();
    drop(sink);
    let collected: Vec<_> = rx.iter().collect();
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].line_number, 1);
    assert_eq!(collected[1].line_number, 3);
}
