use super::cancel::CancelToken;
use super::search::search_blocking;

fn req() -> super::options::SearchRequest {
    let mut r = super::search_e2e_tests::req("match");
    r.paths = vec!["tests/fixtures/mini/context.txt".into()];
    r
}

#[test]
fn before_context_captures_prior_lines() {
    let mut r = req();
    r.before_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    let m = &res.matches[0];
    assert_eq!(m.before_context, vec!["line 2".to_string()]);
}

#[test]
fn after_context_captures_following_lines() {
    let mut r = req();
    r.after_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    let m = &res.matches[0];
    assert_eq!(m.after_context, vec!["line 4".to_string()]);
}

#[test]
fn multiline_pattern_matches_across_lines() {
    let mut r = req();
    r.pattern = r"match[\s\S]*match".into();
    r.multiline = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert!(!res.matches.is_empty());
}
