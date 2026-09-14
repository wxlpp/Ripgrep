use super::cancel::CancelToken;
use super::search::search_blocking;

fn req(pattern: &str) -> super::options::SearchRequest {
    let mut r = super::search_e2e_tests::req(pattern);
    r.respect_gitignore = false; // ensure we have lots of files to count
    r.include_hidden = true;
    r
}

#[test]
fn max_matches_truncates_and_flags() {
    let mut r = req("TODO");
    r.max_matches = Some(1);
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert!(res.truncated);
}

#[test]
fn max_files_caps_files_searched() {
    let mut r = req("TODO");
    r.max_files = Some(1);
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert!(res.files_searched <= 1, "got {}", res.files_searched);
}

#[test]
fn pre_expired_cancel_marks_result_cancelled() {
    // Timeout is entirely CancelToken-driven; pass a pre-tripped token (deadline=0).
    let r = req("xxxxxxxxxxxxxxxxx_no_match"); // forces full scan
    let res = search_blocking(
        r,
        CancelToken::new(Some(0)), // pre-tripped
    )
    .unwrap();
    assert!(res.cancelled);
}
