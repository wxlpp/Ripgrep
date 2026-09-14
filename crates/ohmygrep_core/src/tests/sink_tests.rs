use crossbeam_channel::unbounded;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;

#[test]
fn sink_emits_match_per_line() {
    let (tx, rx) = unbounded();
    let m = RegexMatcher::new("foo").unwrap();
    let mut sink = super::support::test_sink(
        "/tmp/test.txt",
        tx,
        crate::cancel::CancelToken::new(None),
        m.clone(),
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
